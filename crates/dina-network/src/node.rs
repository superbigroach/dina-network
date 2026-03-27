//! DinaNode: the top-level P2P networking component.
//!
//! Sets up a libp2p Swarm with:
//! - TCP transport with Noise encryption and Yamux multiplexing
//! - GossipSub for transaction, block, and consensus message propagation
//! - mDNS for local/LAN peer discovery (Cognitum Seeds)
//! - Kademlia DHT for internet-scale peer discovery
//! - Identify protocol for peer metadata exchange

use std::time::Duration;

use libp2p::futures::StreamExt as _;
use libp2p::identity::Keypair;
use libp2p::kad::store::MemoryStore;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{gossipsub, identify, kad, mdns, noise, tcp, yamux, Multiaddr, PeerId, Swarm};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::discovery::{self, DiscoveryConfig, DiscoveryState, add_bootstrap_peers};
use crate::gossip::{self, DinaGossip};
use crate::message::{BlockPayload, NetworkMessage, TransactionPayload, Vote};
use crate::peer::{PeerManager, PeerManagerConfig};

/// The composed libp2p behaviour for a Dina network node.
#[derive(NetworkBehaviour)]
pub struct DinaBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
}

/// Commands that can be sent to the node's event loop from application code.
#[derive(Debug)]
pub enum NodeCommand {
    BroadcastTransaction(TransactionPayload),
    BroadcastBlock(BlockPayload),
    BroadcastVote(Vote),
    BroadcastConsensus(NetworkMessage),
    GetPeerCount(tokio::sync::oneshot::Sender<usize>),
    GetConnectedPeers(tokio::sync::oneshot::Sender<Vec<PeerId>>),
}

/// Events emitted by the node to application code.
#[derive(Debug)]
pub enum NodeEvent {
    /// A validated message was received from the network.
    MessageReceived {
        source: PeerId,
        message: NetworkMessage,
    },
    /// A new peer connected.
    PeerConnected(PeerId),
    /// A peer disconnected.
    PeerDisconnected(PeerId),
}

/// The main Dina network node.
pub struct DinaNode {
    swarm: Swarm<DinaBehaviour>,
    peer_manager: PeerManager,
    discovery_state: DiscoveryState,
    listen_addr: Multiaddr,
    command_rx: mpsc::Receiver<NodeCommand>,
    event_tx: mpsc::Sender<NodeEvent>,
    /// Handle for sending commands to the node.
    #[allow(dead_code)]
    command_tx: mpsc::Sender<NodeCommand>,
}

/// A handle for interacting with a running DinaNode from application code.
///
/// The command sender is cloneable, but the event receiver is not -- use
/// `take_event_rx()` once and then clone the handle for command-only use.
pub struct DinaNodeHandle {
    command_tx: mpsc::Sender<NodeCommand>,
    event_rx: Option<mpsc::Receiver<NodeEvent>>,
}

impl DinaNodeHandle {
    /// Create a command-only clone of this handle (no event receiver).
    pub fn command_handle(&self) -> CommandHandle {
        CommandHandle {
            command_tx: self.command_tx.clone(),
        }
    }
}

/// A lightweight, cloneable handle for sending commands to the node.
#[derive(Clone)]
pub struct CommandHandle {
    command_tx: mpsc::Sender<NodeCommand>,
}

impl CommandHandle {
    /// Send a transaction to be broadcast to the network.
    pub async fn broadcast_transaction(&self, tx: TransactionPayload) -> Result<(), anyhow::Error> {
        self.command_tx
            .send(NodeCommand::BroadcastTransaction(tx))
            .await
            .map_err(|_| anyhow::anyhow!("node event loop has shut down"))
    }

    /// Send a block to be broadcast to the network.
    pub async fn broadcast_block(&self, block: BlockPayload) -> Result<(), anyhow::Error> {
        self.command_tx
            .send(NodeCommand::BroadcastBlock(block))
            .await
            .map_err(|_| anyhow::anyhow!("node event loop has shut down"))
    }

    /// Query the current peer count.
    pub async fn peer_count(&self) -> Result<usize, anyhow::Error> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(NodeCommand::GetPeerCount(tx))
            .await
            .map_err(|_| anyhow::anyhow!("node event loop has shut down"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("node event loop dropped response"))
    }
}

impl DinaNode {
    /// Create a new DinaNode.
    ///
    /// # Arguments
    /// * `keypair` -- Ed25519 keypair for this node's identity
    /// * `listen_addr` -- Multiaddr to listen on (e.g. `/ip4/0.0.0.0/tcp/9000`)
    /// * `bootstrap_peers` -- Bootstrap peer addresses for Kademlia DHT
    pub fn new(
        keypair: Keypair,
        listen_addr: Multiaddr,
        bootstrap_peers: Vec<Multiaddr>,
    ) -> Result<(Self, DinaNodeHandle), anyhow::Error> {
        let local_peer_id = keypair.public().to_peer_id();
        info!(%local_peer_id, %listen_addr, "initializing DinaNode");

        // Build the GossipSub behaviour
        let mut gossipsub_behaviour = gossip::build_gossipsub(&keypair)
            .map_err(|e| anyhow::anyhow!("gossipsub config error: {e}"))?;

        // Subscribe to all Dina topics
        for topic in gossip::topics() {
            gossipsub_behaviour.subscribe(&topic)?;
        }

        // Build Kademlia
        let discovery_config = DiscoveryConfig {
            enable_mdns: true,
            enable_kademlia: true,
            bootstrap_peers: bootstrap_peers.clone(),
            ..Default::default()
        };
        let mut kademlia = discovery::build_kademlia(local_peer_id, &discovery_config)
            .expect("kademlia enabled in config");

        // Add bootstrap peers to Kademlia
        if !bootstrap_peers.is_empty() {
            add_bootstrap_peers(&mut kademlia, &bootstrap_peers);
        }

        // Build mDNS
        let mdns_config = mdns::Config {
            ttl: Duration::from_secs(300),
            query_interval: Duration::from_secs(30),
            enable_ipv6: false,
        };
        let mdns_behaviour = mdns::tokio::Behaviour::new(mdns_config, local_peer_id)?;

        // Build Identify
        let identify_behaviour = identify::Behaviour::new(identify::Config::new(
            "/dina/id/1.0.0".to_string(),
            keypair.public(),
        ));

        // Compose the behaviour
        let behaviour = DinaBehaviour {
            gossipsub: gossipsub_behaviour,
            kademlia,
            mdns: mdns_behaviour,
            identify: identify_behaviour,
        };

        // Build the swarm with TCP + Noise + Yamux
        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|_key| Ok::<_, Box<dyn std::error::Error + Send + Sync>>(behaviour))?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(120))
            })
            .build();

        let (command_tx, command_rx) = mpsc::channel(256);
        let (event_tx, event_rx) = mpsc::channel(256);

        let node = DinaNode {
            swarm,
            peer_manager: PeerManager::new(PeerManagerConfig::default()),
            discovery_state: DiscoveryState::new(),
            listen_addr,
            command_rx,
            event_tx,
            command_tx: command_tx.clone(),
        };

        let handle = DinaNodeHandle {
            command_tx,
            event_rx: Some(event_rx),
        };

        Ok((node, handle))
    }

    /// Start the node's event loop. This listens on the configured address and
    /// processes swarm events and commands until the process exits.
    pub async fn start(mut self) -> Result<(), anyhow::Error> {
        // Start listening on the configured address.
        self.swarm.listen_on(self.listen_addr.clone())?;

        // Periodic maintenance timer
        let mut maintenance_interval = tokio::time::interval(Duration::from_secs(30));

        info!("DinaNode event loop started");

        loop {
            tokio::select! {
                // Handle swarm events
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                }

                // Handle incoming commands from application code
                Some(cmd) = self.command_rx.recv() => {
                    self.handle_command(cmd);
                }

                // Periodic maintenance
                _ = maintenance_interval.tick() => {
                    self.run_maintenance();
                }
            }
        }
    }

    /// Start the event loop listening on a specific address.
    pub async fn start_with_addr(mut self, listen_addr: Multiaddr) -> Result<(), anyhow::Error> {
        self.swarm.listen_on(listen_addr)?;

        let mut maintenance_interval = tokio::time::interval(Duration::from_secs(30));

        info!("DinaNode event loop started");

        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                }

                Some(cmd) = self.command_rx.recv() => {
                    self.handle_command(cmd);
                }

                _ = maintenance_interval.tick() => {
                    self.run_maintenance();
                }
            }
        }
    }

    async fn handle_swarm_event(&mut self, event: SwarmEvent<DinaBehaviourEvent>) {
        match event {
            // --- Connection events ---
            SwarmEvent::ConnectionEstablished {
                peer_id,
                ..
            } => {
                info!(%peer_id, "connection established");
                self.peer_manager.on_connected(peer_id);
                let _ = self.event_tx.try_send(NodeEvent::PeerConnected(peer_id));
            }

            SwarmEvent::ConnectionClosed {
                peer_id,
                ..
            } => {
                debug!(%peer_id, "connection closed");
                self.peer_manager.on_disconnected(&peer_id);
                let _ = self.event_tx.try_send(NodeEvent::PeerDisconnected(peer_id));
            }

            // --- GossipSub events ---
            SwarmEvent::Behaviour(DinaBehaviourEvent::Gossipsub(
                gossipsub::Event::Message {
                    propagation_source,
                    message,
                    ..
                },
            )) => {
                match DinaGossip::validate_message(&message.data) {
                    Ok(net_msg) => {
                        debug!(
                            source = %propagation_source,
                            label = net_msg.label(),
                            "received valid gossip message"
                        );
                        self.peer_manager.record_good_message(&propagation_source);
                        let _ = self.event_tx.try_send(NodeEvent::MessageReceived {
                            source: propagation_source,
                            message: net_msg,
                        });
                    }
                    Err(e) => {
                        warn!(
                            source = %propagation_source,
                            error = %e,
                            "invalid gossip message"
                        );
                        self.peer_manager.record_bad_message(&propagation_source);
                    }
                }
            }

            SwarmEvent::Behaviour(DinaBehaviourEvent::Gossipsub(
                gossipsub::Event::Subscribed { peer_id, topic },
            )) => {
                debug!(%peer_id, %topic, "peer subscribed to topic");
            }

            SwarmEvent::Behaviour(DinaBehaviourEvent::Gossipsub(
                gossipsub::Event::Unsubscribed { peer_id, topic },
            )) => {
                debug!(%peer_id, %topic, "peer unsubscribed from topic");
            }

            // --- mDNS events ---
            SwarmEvent::Behaviour(DinaBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                let peer_addrs: Vec<_> = peers.into_iter().collect();
                for (peer_id, addr) in &peer_addrs {
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(peer_id, addr.clone());
                    if let Err(e) = self.swarm.dial(addr.clone()) {
                        debug!(%peer_id, %e, "failed to dial mDNS peer");
                    }
                }
                self.discovery_state.on_mdns_discovered(peer_addrs);
            }

            SwarmEvent::Behaviour(DinaBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                let peer_addrs: Vec<_> = peers.into_iter().collect();
                self.discovery_state.on_mdns_expired(peer_addrs);
            }

            // --- Kademlia events ---
            SwarmEvent::Behaviour(DinaBehaviourEvent::Kademlia(
                kad::Event::RoutingUpdated { peer, .. },
            )) => {
                debug!(%peer, "Kademlia routing table updated");
                self.discovery_state.on_kademlia_peer_found(peer);
            }

            SwarmEvent::Behaviour(DinaBehaviourEvent::Kademlia(
                kad::Event::OutboundQueryProgressed {
                    result: kad::QueryResult::Bootstrap(Ok(kad::BootstrapOk { num_remaining, .. })),
                    ..
                },
            )) => {
                debug!(num_remaining, "Kademlia bootstrap progress");
            }

            // --- Identify events ---
            SwarmEvent::Behaviour(DinaBehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info: identify_info,
                ..
            })) => {
                debug!(
                    %peer_id,
                    protocol_version = %identify_info.protocol_version,
                    agent_version = %identify_info.agent_version,
                    "received identify info"
                );
                // Add the peer's listen addresses to Kademlia so they can be
                // discovered by other peers.
                for addr in &identify_info.listen_addrs {
                    self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                    self.peer_manager.add_address(&peer_id, addr.clone());
                }
            }

            // --- Listener events ---
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(%address, "listening on new address");
            }

            SwarmEvent::ListenerError { error, .. } => {
                error!(%error, "listener error");
            }

            // Catch-all for events we don't need to handle individually.
            _ => {}
        }
    }

    fn handle_command(&mut self, cmd: NodeCommand) {
        match cmd {
            NodeCommand::BroadcastTransaction(tx) => {
                if let Err(e) =
                    DinaGossip::publish_transaction(
                        &mut self.swarm.behaviour_mut().gossipsub,
                        tx,
                    )
                {
                    warn!(%e, "failed to broadcast transaction");
                }
            }
            NodeCommand::BroadcastBlock(block) => {
                if let Err(e) =
                    DinaGossip::publish_block(&mut self.swarm.behaviour_mut().gossipsub, block)
                {
                    warn!(%e, "failed to broadcast block");
                }
            }
            NodeCommand::BroadcastVote(vote) => {
                let msg = NetworkMessage::Vote(vote);
                if let Err(e) = DinaGossip::publish_consensus_message(
                    &mut self.swarm.behaviour_mut().gossipsub,
                    msg,
                ) {
                    warn!(%e, "failed to broadcast vote");
                }
            }
            NodeCommand::BroadcastConsensus(msg) => {
                if let Err(e) = DinaGossip::publish_consensus_message(
                    &mut self.swarm.behaviour_mut().gossipsub,
                    msg,
                ) {
                    warn!(%e, "failed to broadcast consensus message");
                }
            }
            NodeCommand::GetPeerCount(reply) => {
                let _ = reply.send(self.peer_manager.peer_count());
            }
            NodeCommand::GetConnectedPeers(reply) => {
                let _ = reply.send(self.peer_manager.connected_peers());
            }
        }
    }

    fn run_maintenance(&mut self) {
        let stale = self.peer_manager.prune_stale();
        if !stale.is_empty() {
            debug!(count = stale.len(), "pruned stale peers");
        }
    }

    /// Get the number of connected peers (synchronous, for use before start).
    pub fn peer_count(&self) -> usize {
        self.peer_manager.peer_count()
    }

    /// Get the list of connected peer IDs (synchronous, for use before start).
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.peer_manager.connected_peers()
    }
}

// Additional DinaNodeHandle methods (full handle with event receiver).
impl DinaNodeHandle {
    /// Send a transaction to be broadcast to the network.
    pub async fn broadcast_transaction(&self, tx: TransactionPayload) -> Result<(), anyhow::Error> {
        self.command_tx
            .send(NodeCommand::BroadcastTransaction(tx))
            .await
            .map_err(|_| anyhow::anyhow!("node event loop has shut down"))
    }

    /// Send a block to be broadcast to the network.
    pub async fn broadcast_block(&self, block: BlockPayload) -> Result<(), anyhow::Error> {
        self.command_tx
            .send(NodeCommand::BroadcastBlock(block))
            .await
            .map_err(|_| anyhow::anyhow!("node event loop has shut down"))
    }

    /// Send a vote to be broadcast to the network.
    pub async fn broadcast_vote(&self, vote: Vote) -> Result<(), anyhow::Error> {
        self.command_tx
            .send(NodeCommand::BroadcastVote(vote))
            .await
            .map_err(|_| anyhow::anyhow!("node event loop has shut down"))
    }

    /// Query the current peer count.
    pub async fn peer_count(&self) -> Result<usize, anyhow::Error> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(NodeCommand::GetPeerCount(tx))
            .await
            .map_err(|_| anyhow::anyhow!("node event loop has shut down"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("node event loop dropped response"))
    }

    /// Query the list of connected peers.
    pub async fn connected_peers(&self) -> Result<Vec<PeerId>, anyhow::Error> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(NodeCommand::GetConnectedPeers(tx))
            .await
            .map_err(|_| anyhow::anyhow!("node event loop has shut down"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("node event loop dropped response"))
    }

    /// Take the event receiver. Can only be called once; subsequent calls
    /// return `None`.
    pub fn take_event_rx(&mut self) -> Option<mpsc::Receiver<NodeEvent>> {
        self.event_rx.take()
    }
}
