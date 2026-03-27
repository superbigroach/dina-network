//! Peer management: tracking, scoring, and banning connected peers.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use libp2p::{Multiaddr, PeerId};
use tracing::{debug, info, warn};

/// Configuration for peer management limits and thresholds.
#[derive(Debug, Clone)]
pub struct PeerManagerConfig {
    /// Maximum number of connected peers.
    pub max_peers: usize,
    /// Duration after which a peer is considered stale if no messages received.
    pub stale_timeout: Duration,
    /// Score below which a peer gets disconnected.
    pub min_score: f64,
    /// Score below which a peer gets banned.
    pub ban_threshold: f64,
    /// How long a ban lasts.
    pub ban_duration: Duration,
}

impl Default for PeerManagerConfig {
    fn default() -> Self {
        Self {
            max_peers: 50,
            stale_timeout: Duration::from_secs(300),
            min_score: -50.0,
            ban_threshold: -100.0,
            ban_duration: Duration::from_secs(3600),
        }
    }
}

/// Tracked information about a connected peer.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// The peer's libp2p identity.
    pub peer_id: PeerId,
    /// Known multiaddresses for this peer.
    pub addresses: Vec<Multiaddr>,
    /// When we first connected.
    pub connected_at: Instant,
    /// Timestamp of the last message received from this peer.
    pub last_seen: Instant,
    /// Whether this peer is a known validator.
    pub is_validator: bool,
    /// Reputation score. Starts at 0, goes up for good behavior, down for bad.
    pub score: f64,
    /// Number of valid messages received.
    pub good_messages: u64,
    /// Number of invalid or duplicate messages received.
    pub bad_messages: u64,
    /// Number of active connections to this peer.
    pub active_connections: usize,
}

impl PeerInfo {
    pub fn new(peer_id: PeerId) -> Self {
        let now = Instant::now();
        Self {
            peer_id,
            addresses: Vec::new(),
            connected_at: now,
            last_seen: now,
            is_validator: false,
            score: 0.0,
            good_messages: 0,
            bad_messages: 0,
            active_connections: 0,
        }
    }
}

/// Record of a banned peer.
#[derive(Debug, Clone)]
struct BanRecord {
    banned_at: Instant,
    duration: Duration,
    reason: String,
}

impl BanRecord {
    fn is_expired(&self) -> bool {
        self.banned_at.elapsed() >= self.duration
    }
}

/// Manages the set of connected peers, their scores, and ban lists.
pub struct PeerManager {
    config: PeerManagerConfig,
    peers: HashMap<PeerId, PeerInfo>,
    banned: HashMap<PeerId, BanRecord>,
}

impl PeerManager {
    pub fn new(config: PeerManagerConfig) -> Self {
        Self {
            config,
            peers: HashMap::new(),
            banned: HashMap::new(),
        }
    }

    /// Register a new peer connection. Returns false if the peer is banned or
    /// we are at capacity.
    pub fn on_connected(&mut self, peer_id: PeerId) -> bool {
        // Check bans (and expire old ones)
        if let Some(ban) = self.banned.get(&peer_id) {
            if !ban.is_expired() {
                warn!(%peer_id, reason = %ban.reason, "rejecting banned peer");
                return false;
            }
            self.banned.remove(&peer_id);
        }

        // Check capacity (validators always allowed)
        let info = self.peers.get(&peer_id);
        let is_new = info.is_none();
        let is_validator = info.map_or(false, |i| i.is_validator);

        if is_new && !is_validator && self.peers.len() >= self.config.max_peers {
            debug!(%peer_id, max = self.config.max_peers, "at peer capacity, rejecting");
            return false;
        }

        let entry = self.peers.entry(peer_id).or_insert_with(|| {
            info!(%peer_id, "new peer connected");
            PeerInfo::new(peer_id)
        });
        entry.active_connections += 1;
        entry.last_seen = Instant::now();
        true
    }

    /// Remove a connection for a peer. If no connections remain, remove the peer entirely.
    pub fn on_disconnected(&mut self, peer_id: &PeerId) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            info.active_connections = info.active_connections.saturating_sub(1);
            if info.active_connections == 0 {
                info!(peer = %peer_id, "peer fully disconnected");
                self.peers.remove(peer_id);
            }
        }
    }

    /// Record that we received a valid, useful message from a peer.
    pub fn record_good_message(&mut self, peer_id: &PeerId) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            info.good_messages += 1;
            info.score += 1.0;
            info.last_seen = Instant::now();
        }
    }

    /// Record that we received an invalid or spammy message from a peer.
    pub fn record_bad_message(&mut self, peer_id: &PeerId) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            info.bad_messages += 1;
            info.score -= 10.0;
            info.last_seen = Instant::now();

            if info.score <= self.config.ban_threshold {
                warn!(%peer_id, score = info.score, "auto-banning peer due to low score");
                self.ban(peer_id, "score below ban threshold".into());
            }
        }
    }

    /// Mark a peer as a known validator.
    pub fn set_validator(&mut self, peer_id: &PeerId, is_validator: bool) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            info.is_validator = is_validator;
        }
    }

    /// Add a known address for a peer.
    pub fn add_address(&mut self, peer_id: &PeerId, addr: Multiaddr) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            if !info.addresses.contains(&addr) {
                info.addresses.push(addr);
            }
        }
    }

    /// Manually ban a peer.
    pub fn ban(&mut self, peer_id: &PeerId, reason: String) {
        info!(%peer_id, %reason, "banning peer");
        self.banned.insert(
            *peer_id,
            BanRecord {
                banned_at: Instant::now(),
                duration: self.config.ban_duration,
                reason,
            },
        );
        self.peers.remove(peer_id);
    }

    /// Manually unban a peer.
    pub fn unban(&mut self, peer_id: &PeerId) {
        if self.banned.remove(peer_id).is_some() {
            info!(%peer_id, "unbanned peer");
        }
    }

    /// Check whether a peer is currently banned.
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        self.banned
            .get(peer_id)
            .map_or(false, |b| !b.is_expired())
    }

    /// Return the number of connected peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Return a list of all connected peer IDs.
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.peers.keys().copied().collect()
    }

    /// Return info about a specific peer, if connected.
    pub fn peer_info(&self, peer_id: &PeerId) -> Option<&PeerInfo> {
        self.peers.get(peer_id)
    }

    /// Evict the lowest-scoring non-validator peers to make room.
    /// Returns the list of peer IDs that should be disconnected.
    pub fn evict_lowest_scoring(&mut self, count: usize) -> Vec<PeerId> {
        let mut candidates: Vec<_> = self
            .peers
            .iter()
            .filter(|(_, info)| !info.is_validator)
            .map(|(id, info)| (*id, info.score))
            .collect();

        // Sort ascending by score so worst peers come first.
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let to_evict: Vec<PeerId> = candidates.into_iter().take(count).map(|(id, _)| id).collect();

        for peer_id in &to_evict {
            debug!(%peer_id, "evicting low-score peer");
            self.peers.remove(peer_id);
        }

        to_evict
    }

    /// Prune peers that haven't sent any message in longer than `stale_timeout`.
    /// Returns the list of stale peer IDs removed.
    pub fn prune_stale(&mut self) -> Vec<PeerId> {
        let timeout = self.config.stale_timeout;
        let stale: Vec<PeerId> = self
            .peers
            .iter()
            .filter(|(_, info)| info.last_seen.elapsed() > timeout && !info.is_validator)
            .map(|(id, _)| *id)
            .collect();

        for peer_id in &stale {
            debug!(%peer_id, "pruning stale peer");
            self.peers.remove(peer_id);
        }

        // Also expire old bans.
        self.banned.retain(|_, ban| !ban.is_expired());

        stale
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;

    fn random_peer_id() -> PeerId {
        let kp = Keypair::generate_ed25519();
        kp.public().to_peer_id()
    }

    #[test]
    fn connect_and_count() {
        let mut mgr = PeerManager::new(PeerManagerConfig::default());
        let p1 = random_peer_id();
        assert!(mgr.on_connected(p1));
        assert_eq!(mgr.peer_count(), 1);
        assert!(mgr.connected_peers().contains(&p1));
    }

    #[test]
    fn disconnect_removes_peer() {
        let mut mgr = PeerManager::new(PeerManagerConfig::default());
        let p1 = random_peer_id();
        mgr.on_connected(p1);
        mgr.on_disconnected(&p1);
        assert_eq!(mgr.peer_count(), 0);
    }

    #[test]
    fn multiple_connections_tracked() {
        let mut mgr = PeerManager::new(PeerManagerConfig::default());
        let p1 = random_peer_id();
        mgr.on_connected(p1);
        mgr.on_connected(p1); // second connection

        assert_eq!(mgr.peer_count(), 1);
        assert_eq!(mgr.peer_info(&p1).unwrap().active_connections, 2);

        mgr.on_disconnected(&p1); // close one connection
        assert_eq!(mgr.peer_count(), 1); // still connected
        mgr.on_disconnected(&p1); // close last connection
        assert_eq!(mgr.peer_count(), 0); // now gone
    }

    #[test]
    fn ban_rejects_connection() {
        let mut mgr = PeerManager::new(PeerManagerConfig::default());
        let p1 = random_peer_id();
        mgr.banned.insert(
            p1,
            BanRecord {
                banned_at: Instant::now(),
                duration: Duration::from_secs(3600),
                reason: "test".into(),
            },
        );
        assert!(!mgr.on_connected(p1));
        assert!(mgr.is_banned(&p1));
    }

    #[test]
    fn scoring_and_auto_ban() {
        let mut mgr = PeerManager::new(PeerManagerConfig {
            ban_threshold: -20.0,
            ..PeerManagerConfig::default()
        });
        let p1 = random_peer_id();
        mgr.on_connected(p1);

        // 3 bad messages = -30 score, below -20 threshold
        mgr.record_bad_message(&p1);
        mgr.record_bad_message(&p1);
        mgr.record_bad_message(&p1);

        assert!(mgr.is_banned(&p1));
        assert_eq!(mgr.peer_count(), 0);
    }

    #[test]
    fn capacity_limit() {
        let mut mgr = PeerManager::new(PeerManagerConfig {
            max_peers: 2,
            ..PeerManagerConfig::default()
        });
        let p1 = random_peer_id();
        let p2 = random_peer_id();
        let p3 = random_peer_id();

        assert!(mgr.on_connected(p1));
        assert!(mgr.on_connected(p2));
        assert!(!mgr.on_connected(p3));
        assert_eq!(mgr.peer_count(), 2);
    }

    #[test]
    fn evict_lowest_scoring() {
        let mut mgr = PeerManager::new(PeerManagerConfig::default());
        let p1 = random_peer_id();
        let p2 = random_peer_id();
        let p3 = random_peer_id();

        mgr.on_connected(p1);
        mgr.on_connected(p2);
        mgr.on_connected(p3);

        // Give p1 a bad score
        mgr.record_bad_message(&p1);
        // Give p3 good score
        mgr.record_good_message(&p3);

        let evicted = mgr.evict_lowest_scoring(1);
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0], p1);
        assert_eq!(mgr.peer_count(), 2);
    }

    #[test]
    fn unban_allows_reconnect() {
        let mut mgr = PeerManager::new(PeerManagerConfig::default());
        let p1 = random_peer_id();
        mgr.ban(&p1, "test ban".into());
        assert!(mgr.is_banned(&p1));

        mgr.unban(&p1);
        assert!(!mgr.is_banned(&p1));
        assert!(mgr.on_connected(p1));
    }
}
