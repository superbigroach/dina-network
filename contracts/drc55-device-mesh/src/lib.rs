use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

// ---------------------------------------------------------------------------
// DRC-55  Device Mesh Network Registry
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TopologyType {
    Star,
    Mesh,
    Ring,
    Tree,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeInfo {
    pub address: Address,
    pub name: String,
    pub device_type: String,
    pub registered_at: u64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionInfo {
    pub signal_strength: i32, // dBm, e.g. -70
    pub latency_ms: u32,
    pub bandwidth_kbps: u64,
    pub last_seen: u64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MeshNetwork {
    pub id: u64,
    pub name: String,
    pub owner: Address,
    pub nodes: BTreeSet<Address>,
    pub topology_type: TopologyType,
}

/// Edge key: sorted pair to avoid duplicates for undirected connections.
fn edge_key(a: Address, b: Address) -> (Address, Address) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MeshState {
    pub owner: Address,
    pub nodes: BTreeMap<Address, NodeInfo>,
    pub edges: BTreeMap<(Address, Address), ConnectionInfo>,
    pub networks: BTreeMap<u64, MeshNetwork>,
    pub next_network_id: u64,
}

impl MeshState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            networks: BTreeMap::new(),
            next_network_id: 1,
        }
    }

    pub fn register_node(
        &mut self,
        caller: Address,
        name: String,
        device_type: String,
        registered_at: u64,
    ) {
        assert!(
            !self.nodes.contains_key(&caller),
            "DRC55: node already registered"
        );
        self.nodes.insert(
            caller,
            NodeInfo {
                address: caller,
                name,
                device_type,
                registered_at,
                active: true,
            },
        );
    }

    pub fn connect_nodes(
        &mut self,
        caller: Address,
        node_a: Address,
        node_b: Address,
        signal_strength: i32,
        latency_ms: u32,
        bandwidth_kbps: u64,
        timestamp: u64,
    ) {
        assert!(node_a != node_b, "DRC55: cannot connect node to itself");
        assert!(
            self.nodes.contains_key(&node_a),
            "DRC55: node_a not registered"
        );
        assert!(
            self.nodes.contains_key(&node_b),
            "DRC55: node_b not registered"
        );
        assert!(
            caller == node_a || caller == node_b || caller == self.owner,
            "DRC55: only involved nodes or owner can connect"
        );
        let key = edge_key(node_a, node_b);
        self.edges.insert(
            key,
            ConnectionInfo {
                signal_strength,
                latency_ms,
                bandwidth_kbps,
                last_seen: timestamp,
                active: true,
            },
        );
    }

    pub fn disconnect(&mut self, caller: Address, node_a: Address, node_b: Address) {
        assert!(
            caller == node_a || caller == node_b || caller == self.owner,
            "DRC55: not authorised"
        );
        let key = edge_key(node_a, node_b);
        let conn = self
            .edges
            .get_mut(&key)
            .expect("DRC55: connection not found");
        conn.active = false;
    }

    pub fn update_connection(
        &mut self,
        caller: Address,
        node_a: Address,
        node_b: Address,
        signal_strength: i32,
        latency_ms: u32,
        bandwidth_kbps: u64,
        timestamp: u64,
    ) {
        assert!(
            caller == node_a || caller == node_b || caller == self.owner,
            "DRC55: not authorised"
        );
        let key = edge_key(node_a, node_b);
        let conn = self
            .edges
            .get_mut(&key)
            .expect("DRC55: connection not found");
        conn.signal_strength = signal_strength;
        conn.latency_ms = latency_ms;
        conn.bandwidth_kbps = bandwidth_kbps;
        conn.last_seen = timestamp;
        conn.active = true;
    }

    /// BFS shortest path between two nodes (active edges only).
    pub fn find_path(&self, from: Address, to: Address) -> Option<Vec<Address>> {
        if from == to {
            return Some(vec![from]);
        }
        let mut visited = BTreeSet::new();
        let mut queue: VecDeque<(Address, Vec<Address>)> = VecDeque::new();
        visited.insert(from);
        queue.push_back((from, vec![from]));

        while let Some((current, path)) = queue.pop_front() {
            for (&(a, b), conn) in &self.edges {
                if !conn.active {
                    continue;
                }
                let neighbor = if a == current {
                    Some(b)
                } else if b == current {
                    Some(a)
                } else {
                    None
                };
                if let Some(n) = neighbor {
                    if n == to {
                        let mut result = path.clone();
                        result.push(n);
                        return Some(result);
                    }
                    if !visited.contains(&n) {
                        visited.insert(n);
                        let mut new_path = path.clone();
                        new_path.push(n);
                        queue.push_back((n, new_path));
                    }
                }
            }
        }
        None
    }

    pub fn create_network(
        &mut self,
        caller: Address,
        name: String,
        topology_type: TopologyType,
    ) -> u64 {
        let id = self.next_network_id;
        self.next_network_id += 1;
        self.networks.insert(
            id,
            MeshNetwork {
                id,
                name,
                owner: caller,
                nodes: BTreeSet::new(),
                topology_type,
            },
        );
        id
    }

    pub fn add_node_to_network(&mut self, caller: Address, network_id: u64, node: Address) {
        let net = self
            .networks
            .get_mut(&network_id)
            .expect("DRC55: network not found");
        assert!(
            caller == net.owner || caller == self.owner,
            "DRC55: not network owner"
        );
        assert!(self.nodes.contains_key(&node), "DRC55: node not registered");
        net.nodes.insert(node);
    }

    /// Return all nodes reachable within max_hops from addr.
    pub fn nearby_nodes(&self, addr: Address, max_hops: u32) -> Vec<Address> {
        let mut visited = BTreeSet::new();
        let mut queue: VecDeque<(Address, u32)> = VecDeque::new();
        visited.insert(addr);
        queue.push_back((addr, 0));

        while let Some((current, hops)) = queue.pop_front() {
            if hops >= max_hops {
                continue;
            }
            for (&(a, b), conn) in &self.edges {
                if !conn.active {
                    continue;
                }
                let neighbor = if a == current {
                    Some(b)
                } else if b == current {
                    Some(a)
                } else {
                    None
                };
                if let Some(n) = neighbor {
                    if !visited.contains(&n) {
                        visited.insert(n);
                        queue.push_back((n, hops + 1));
                    }
                }
            }
        }
        visited.remove(&addr);
        visited.into_iter().collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterNodeArgs {
    name: String,
    device_type: String,
    registered_at: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConnectArgs {
    node_a: Address,
    node_b: Address,
    signal_strength: i32,
    latency_ms: u32,
    bandwidth_kbps: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DisconnectArgs {
    node_a: Address,
    node_b: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct FindPathArgs {
    from: Address,
    to: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateNetworkArgs {
    name: String,
    topology_type: TopologyType,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddNodeNetworkArgs {
    network_id: u64,
    node: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct NearbyArgs {
    addr: Address,
    max_hops: u32,
}

pub fn dispatch(
    state: &mut Option<MeshState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC55: already initialised");
            *state = Some(MeshState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "register_node" => {
            let s = state.as_mut().expect("DRC55: not initialised");
            let a: RegisterNodeArgs = serde_json::from_slice(args).expect("DRC55: bad args");
            s.register_node(caller, a.name, a.device_type, a.registered_at);
            serde_json::to_vec("ok").unwrap()
        }
        "connect_nodes" => {
            let s = state.as_mut().expect("DRC55: not initialised");
            let a: ConnectArgs = serde_json::from_slice(args).expect("DRC55: bad args");
            s.connect_nodes(
                caller,
                a.node_a,
                a.node_b,
                a.signal_strength,
                a.latency_ms,
                a.bandwidth_kbps,
                a.timestamp,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "disconnect" => {
            let s = state.as_mut().expect("DRC55: not initialised");
            let a: DisconnectArgs = serde_json::from_slice(args).expect("DRC55: bad args");
            s.disconnect(caller, a.node_a, a.node_b);
            serde_json::to_vec("ok").unwrap()
        }
        "update_connection" => {
            let s = state.as_mut().expect("DRC55: not initialised");
            let a: ConnectArgs = serde_json::from_slice(args).expect("DRC55: bad args");
            s.update_connection(
                caller,
                a.node_a,
                a.node_b,
                a.signal_strength,
                a.latency_ms,
                a.bandwidth_kbps,
                a.timestamp,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "find_path" => {
            let s = state.as_ref().expect("DRC55: not initialised");
            let a: FindPathArgs = serde_json::from_slice(args).expect("DRC55: bad args");
            serde_json::to_vec(&s.find_path(a.from, a.to)).unwrap()
        }
        "create_network" => {
            let s = state.as_mut().expect("DRC55: not initialised");
            let a: CreateNetworkArgs = serde_json::from_slice(args).expect("DRC55: bad args");
            let id = s.create_network(caller, a.name, a.topology_type);
            serde_json::to_vec(&id).unwrap()
        }
        "add_node_to_network" => {
            let s = state.as_mut().expect("DRC55: not initialised");
            let a: AddNodeNetworkArgs = serde_json::from_slice(args).expect("DRC55: bad args");
            s.add_node_to_network(caller, a.network_id, a.node);
            serde_json::to_vec("ok").unwrap()
        }
        "nearby_nodes" => {
            let s = state.as_ref().expect("DRC55: not initialised");
            let a: NearbyArgs = serde_json::from_slice(args).expect("DRC55: bad args");
            serde_json::to_vec(&s.nearby_nodes(a.addr, a.max_hops)).unwrap()
        }
        _ => panic!("DRC55: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const NODE_A: Address = [1u8; 32];
    const NODE_B: Address = [2u8; 32];
    const NODE_C: Address = [3u8; 32];
    const NODE_D: Address = [4u8; 32];

    fn setup() -> MeshState {
        let mut s = MeshState::new(OWNER);
        s.register_node(NODE_A, "Sensor A".into(), "temperature".into(), 100);
        s.register_node(NODE_B, "Sensor B".into(), "humidity".into(), 100);
        s.register_node(NODE_C, "Gateway".into(), "gateway".into(), 100);
        s
    }

    #[test]
    fn test_register_and_connect() {
        let mut s = setup();
        s.connect_nodes(NODE_A, NODE_A, NODE_B, -65, 10, 1000, 200);
        let key = edge_key(NODE_A, NODE_B);
        assert!(s.edges.contains_key(&key));
        assert_eq!(s.edges[&key].latency_ms, 10);
    }

    #[test]
    fn test_find_path_direct() {
        let mut s = setup();
        s.connect_nodes(NODE_A, NODE_A, NODE_B, -65, 10, 1000, 200);
        let path = s.find_path(NODE_A, NODE_B).unwrap();
        assert_eq!(path, vec![NODE_A, NODE_B]);
    }

    #[test]
    fn test_find_path_multihop() {
        let mut s = setup();
        // A--B--C (no direct A--C)
        s.connect_nodes(NODE_A, NODE_A, NODE_B, -65, 10, 1000, 200);
        s.connect_nodes(NODE_B, NODE_B, NODE_C, -70, 15, 800, 200);
        let path = s.find_path(NODE_A, NODE_C).unwrap();
        assert_eq!(path, vec![NODE_A, NODE_B, NODE_C]);
    }

    #[test]
    fn test_find_path_no_route() {
        let s = setup();
        // No edges
        assert!(s.find_path(NODE_A, NODE_C).is_none());
    }

    #[test]
    fn test_disconnect_breaks_path() {
        let mut s = setup();
        s.connect_nodes(NODE_A, NODE_A, NODE_B, -65, 10, 1000, 200);
        s.disconnect(NODE_A, NODE_A, NODE_B);
        assert!(s.find_path(NODE_A, NODE_B).is_none());
    }

    #[test]
    fn test_nearby_nodes() {
        let mut s = setup();
        s.register_node(NODE_D, "Relay".into(), "relay".into(), 100);
        s.connect_nodes(NODE_A, NODE_A, NODE_B, -65, 10, 1000, 200);
        s.connect_nodes(NODE_B, NODE_B, NODE_C, -70, 15, 800, 200);
        s.connect_nodes(NODE_C, NODE_C, NODE_D, -75, 20, 500, 200);

        let one_hop = s.nearby_nodes(NODE_A, 1);
        assert_eq!(one_hop, vec![NODE_B]);

        let two_hop = s.nearby_nodes(NODE_A, 2);
        assert!(two_hop.contains(&NODE_B));
        assert!(two_hop.contains(&NODE_C));
        assert!(!two_hop.contains(&NODE_D));
    }

    #[test]
    fn test_create_network_and_add_node() {
        let mut s = setup();
        let net_id = s.create_network(OWNER, "Sensor Net".into(), TopologyType::Star);
        s.add_node_to_network(OWNER, net_id, NODE_A);
        s.add_node_to_network(OWNER, net_id, NODE_B);
        let net = s.networks.get(&net_id).unwrap();
        assert_eq!(net.nodes.len(), 2);
        assert_eq!(net.topology_type, TopologyType::Star);
    }
}
