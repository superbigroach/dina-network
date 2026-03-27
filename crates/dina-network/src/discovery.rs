//! Peer discovery combining mDNS (local/LAN) and Kademlia DHT (internet).
//!
//! mDNS is used for Cognitum Seeds -- local devices that discover each other
//! on the same network segment without any bootstrap node.
//!
//! Kademlia provides internet-scale peer discovery using a distributed hash
//! table with configurable bootstrap peers.

use std::collections::HashSet;
use std::time::Duration;

use libp2p::kad::store::MemoryStore;
use libp2p::kad::{self, Mode};
use libp2p::mdns;
use libp2p::{Multiaddr, PeerId};
use tracing::{debug, info, warn};

/// Configuration for the discovery subsystem.
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Enable mDNS for local/LAN peer discovery. Useful for Cognitum Seeds
    /// running on the same local network.
    pub enable_mdns: bool,
    /// Enable Kademlia DHT for internet-scale peer discovery.
    pub enable_kademlia: bool,
    /// Bootstrap peer addresses for Kademlia. At least one is needed to join
    /// the DHT.
    pub bootstrap_peers: Vec<Multiaddr>,
    /// How often Kademlia runs a random walk to discover new peers.
    pub kademlia_query_interval: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_mdns: true,
            enable_kademlia: true,
            bootstrap_peers: Vec::new(),
            kademlia_query_interval: Duration::from_secs(60),
        }
    }
}

/// Build an mDNS behaviour for local peer discovery.
///
/// Returns `None` if mDNS is disabled in the config.
pub fn build_mdns(local_peer_id: PeerId, config: &DiscoveryConfig) -> Option<mdns::tokio::Behaviour> {
    if !config.enable_mdns {
        return None;
    }

    let mdns_config = mdns::Config {
        ttl: Duration::from_secs(300),
        query_interval: Duration::from_secs(30),
        enable_ipv6: false,
    };

    match mdns::tokio::Behaviour::new(mdns_config, local_peer_id) {
        Ok(behaviour) => {
            info!("mDNS discovery enabled");
            Some(behaviour)
        }
        Err(e) => {
            warn!(%e, "failed to initialize mDNS, continuing without it");
            None
        }
    }
}

/// Build a Kademlia DHT behaviour for internet peer discovery.
///
/// Returns `None` if Kademlia is disabled in the config.
pub fn build_kademlia(
    local_peer_id: PeerId,
    config: &DiscoveryConfig,
) -> Option<kad::Behaviour<MemoryStore>> {
    if !config.enable_kademlia {
        return None;
    }

    let store = MemoryStore::new(local_peer_id);
    let mut behaviour = kad::Behaviour::new(local_peer_id, store);

    // Start in server mode so other peers can find us in the DHT.
    behaviour.set_mode(Some(Mode::Server));

    info!(
        bootstrap_count = config.bootstrap_peers.len(),
        "Kademlia discovery enabled"
    );

    Some(behaviour)
}

/// Manages the set of discovered peers from all discovery mechanisms.
pub struct DiscoveryState {
    /// Peers discovered via mDNS (local).
    pub mdns_peers: HashSet<PeerId>,
    /// Peers discovered via Kademlia (internet).
    pub kademlia_peers: HashSet<PeerId>,
}

impl DiscoveryState {
    pub fn new() -> Self {
        Self {
            mdns_peers: HashSet::new(),
            kademlia_peers: HashSet::new(),
        }
    }

    /// Handle an mDNS discovered event: new peers found on the local network.
    pub fn on_mdns_discovered(&mut self, peers: Vec<(PeerId, Multiaddr)>) {
        for (peer_id, addr) in &peers {
            if self.mdns_peers.insert(*peer_id) {
                info!(%peer_id, %addr, "mDNS: discovered local peer");
            }
        }
    }

    /// Handle an mDNS expired event: peers are no longer visible on the LAN.
    pub fn on_mdns_expired(&mut self, peers: Vec<(PeerId, Multiaddr)>) {
        for (peer_id, addr) in &peers {
            if self.mdns_peers.remove(peer_id) {
                debug!(%peer_id, %addr, "mDNS: peer expired");
            }
        }
    }

    /// Handle a Kademlia routing table update: a new peer was added to the DHT.
    pub fn on_kademlia_peer_found(&mut self, peer_id: PeerId) {
        if self.kademlia_peers.insert(peer_id) {
            debug!(%peer_id, "Kademlia: new peer in routing table");
        }
    }

    /// Return all unique discovered peers from both mechanisms.
    pub fn all_discovered(&self) -> HashSet<PeerId> {
        self.mdns_peers
            .union(&self.kademlia_peers)
            .copied()
            .collect()
    }

    /// Total number of unique discovered peers.
    pub fn discovered_count(&self) -> usize {
        self.all_discovered().len()
    }
}

impl Default for DiscoveryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Add bootstrap peers to the Kademlia routing table.
///
/// Each multiaddr must contain a `/p2p/<peer_id>` component so the DHT knows
/// who lives at that address.
pub fn add_bootstrap_peers(
    kademlia: &mut kad::Behaviour<MemoryStore>,
    peers: &[Multiaddr],
) {
    for addr in peers {
        // Extract the PeerId from the multiaddr's last `/p2p/...` component.
        if let Some(libp2p::multiaddr::Protocol::P2p(peer_id)) = addr.iter().last() {
            let mut addr_without_p2p = addr.clone();
            // Remove the /p2p/ suffix for the address portion
            let protocols: Vec<_> = addr_without_p2p.iter().collect();
            let base_addr: Multiaddr = protocols
                .into_iter()
                .filter(|p| !matches!(p, libp2p::multiaddr::Protocol::P2p(_)))
                .collect();

            kademlia.add_address(&peer_id, base_addr);
            info!(%peer_id, %addr, "added Kademlia bootstrap peer");
        } else {
            warn!(%addr, "bootstrap address missing /p2p/ component, skipping");
        }
    }

    // Kick off a bootstrap query to populate the routing table.
    if let Err(e) = kademlia.bootstrap() {
        warn!(%e, "Kademlia bootstrap query failed (no known peers?)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_state_tracks_mdns_peers() {
        let mut state = DiscoveryState::new();
        let peer = PeerId::random();
        let addr: Multiaddr = "/ip4/192.168.1.1/tcp/4001".parse().unwrap();

        state.on_mdns_discovered(vec![(peer, addr.clone())]);
        assert!(state.mdns_peers.contains(&peer));
        assert_eq!(state.discovered_count(), 1);

        state.on_mdns_expired(vec![(peer, addr)]);
        assert!(!state.mdns_peers.contains(&peer));
        assert_eq!(state.discovered_count(), 0);
    }

    #[test]
    fn discovery_state_deduplicates() {
        let mut state = DiscoveryState::new();
        let peer = PeerId::random();
        let addr: Multiaddr = "/ip4/10.0.0.1/tcp/4001".parse().unwrap();

        // Same peer found via both mDNS and Kademlia
        state.on_mdns_discovered(vec![(peer, addr)]);
        state.on_kademlia_peer_found(peer);

        assert_eq!(state.mdns_peers.len(), 1);
        assert_eq!(state.kademlia_peers.len(), 1);
        // Union deduplicates
        assert_eq!(state.discovered_count(), 1);
    }

    #[test]
    fn default_config_enables_both() {
        let config = DiscoveryConfig::default();
        assert!(config.enable_mdns);
        assert!(config.enable_kademlia);
    }
}
