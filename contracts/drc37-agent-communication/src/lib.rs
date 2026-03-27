use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-37  Agent Messaging / Communication
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub id: u64,
    pub from: Address,
    pub to: Address,
    pub topic: String,
    pub payload_hash: String,
    pub timestamp: u64,
    pub read: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Channel {
    pub id: u64,
    pub creator: Address,
    pub members: Vec<Address>,
    pub topic: String,
    pub message_ids: Vec<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommState {
    pub admin: Address,
    pub messages: BTreeMap<u64, Message>,
    pub channels: BTreeMap<u64, Channel>,
    pub next_msg_id: u64,
    pub next_channel_id: u64,
}

impl CommState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            messages: BTreeMap::new(),
            channels: BTreeMap::new(),
            next_msg_id: 1,
            next_channel_id: 1,
        }
    }

    pub fn send_message(
        &mut self,
        from: Address,
        to: Address,
        topic: String,
        payload_hash: String,
        timestamp: u64,
    ) -> u64 {
        let id = self.next_msg_id;
        self.next_msg_id += 1;
        let msg = Message {
            id,
            from,
            to,
            topic,
            payload_hash,
            timestamp,
            read: false,
        };
        self.messages.insert(id, msg);
        id
    }

    pub fn read_message(&mut self, caller: Address, message_id: u64) -> &Message {
        let msg = self
            .messages
            .get_mut(&message_id)
            .expect("DRC37: message not found");
        assert!(
            msg.to == caller || msg.from == caller,
            "DRC37: not authorized to read"
        );
        if msg.to == caller {
            msg.read = true;
        }
        // Return immutable ref after mutation
        self.messages.get(&message_id).unwrap()
    }

    pub fn create_channel(
        &mut self,
        caller: Address,
        topic: String,
        initial_members: Vec<Address>,
    ) -> u64 {
        let id = self.next_channel_id;
        self.next_channel_id += 1;
        let mut members = initial_members;
        if !members.contains(&caller) {
            members.insert(0, caller);
        }
        let channel = Channel {
            id,
            creator: caller,
            members,
            topic,
            message_ids: Vec::new(),
        };
        self.channels.insert(id, channel);
        id
    }

    pub fn join_channel(&mut self, caller: Address, channel_id: u64) {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .expect("DRC37: channel not found");
        assert!(
            !channel.members.contains(&caller),
            "DRC37: already a member"
        );
        channel.members.push(caller);
    }

    pub fn send_channel_message(
        &mut self,
        caller: Address,
        channel_id: u64,
        payload_hash: String,
        timestamp: u64,
    ) -> u64 {
        let channel = self
            .channels
            .get(&channel_id)
            .expect("DRC37: channel not found");
        assert!(
            channel.members.contains(&caller),
            "DRC37: not a channel member"
        );
        let topic = channel.topic.clone();
        // Broadcast: store a message with `to` = [0;32] (broadcast address)
        let id = self.next_msg_id;
        self.next_msg_id += 1;
        let msg = Message {
            id,
            from: caller,
            to: [0u8; 32], // broadcast
            topic,
            payload_hash,
            timestamp,
            read: false,
        };
        self.messages.insert(id, msg);

        let channel_mut = self.channels.get_mut(&channel_id).unwrap();
        channel_mut.message_ids.push(id);
        id
    }

    pub fn channel_messages(&self, channel_id: u64) -> Vec<&Message> {
        let channel = self
            .channels
            .get(&channel_id)
            .expect("DRC37: channel not found");
        channel
            .message_ids
            .iter()
            .filter_map(|id| self.messages.get(id))
            .collect()
    }

    pub fn unread_count(&self, address: &Address) -> usize {
        self.messages
            .values()
            .filter(|m| m.to == *address && !m.read)
            .count()
    }

    pub fn get_message(&self, id: u64) -> Option<&Message> {
        self.messages.get(&id)
    }

    pub fn get_channel(&self, id: u64) -> Option<&Channel> {
        self.channels.get(&id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct SendMessageArgs {
    to: Address,
    topic: String,
    payload_hash: String,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReadMessageArgs {
    message_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateChannelArgs {
    topic: String,
    initial_members: Vec<Address>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JoinChannelArgs {
    channel_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SendChannelMessageArgs {
    channel_id: u64,
    payload_hash: String,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChannelMessagesArgs {
    channel_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UnreadCountArgs {
    address: Address,
}

pub fn dispatch(
    state: &mut Option<CommState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC37: already initialised");
            *state = Some(CommState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "send_message" => {
            let s = state.as_mut().expect("DRC37: not initialised");
            let a: SendMessageArgs =
                serde_json::from_slice(args).expect("DRC37: bad send_message args");
            let id = s.send_message(caller, a.to, a.topic, a.payload_hash, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        "read_message" => {
            let s = state.as_mut().expect("DRC37: not initialised");
            let a: ReadMessageArgs =
                serde_json::from_slice(args).expect("DRC37: bad read_message args");
            s.read_message(caller, a.message_id);
            // Return the message after marking read
            let msg = s.get_message(a.message_id);
            serde_json::to_vec(&msg).unwrap()
        }
        "create_channel" => {
            let s = state.as_mut().expect("DRC37: not initialised");
            let a: CreateChannelArgs =
                serde_json::from_slice(args).expect("DRC37: bad create_channel args");
            let id = s.create_channel(caller, a.topic, a.initial_members);
            serde_json::to_vec(&id).unwrap()
        }
        "join_channel" => {
            let s = state.as_mut().expect("DRC37: not initialised");
            let a: JoinChannelArgs =
                serde_json::from_slice(args).expect("DRC37: bad join_channel args");
            s.join_channel(caller, a.channel_id);
            serde_json::to_vec("ok").unwrap()
        }
        "send_channel_message" => {
            let s = state.as_mut().expect("DRC37: not initialised");
            let a: SendChannelMessageArgs =
                serde_json::from_slice(args).expect("DRC37: bad send_channel_message args");
            let id = s.send_channel_message(caller, a.channel_id, a.payload_hash, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        "channel_messages" => {
            let s = state.as_ref().expect("DRC37: not initialised");
            let a: ChannelMessagesArgs =
                serde_json::from_slice(args).expect("DRC37: bad channel_messages args");
            serde_json::to_vec(&s.channel_messages(a.channel_id)).unwrap()
        }
        "unread_count" => {
            let s = state.as_ref().expect("DRC37: not initialised");
            let a: UnreadCountArgs =
                serde_json::from_slice(args).expect("DRC37: bad unread_count args");
            serde_json::to_vec(&s.unread_count(&a.address)).unwrap()
        }
        _ => panic!("DRC37: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: Address = [1u8; 32];
    const ALICE: Address = [2u8; 32];
    const BOB: Address = [3u8; 32];
    const CAROL: Address = [4u8; 32];

    fn init_state() -> Option<CommState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", ADMIN);
        state
    }

    #[test]
    fn test_send_and_read_message() {
        let mut state = init_state();

        let send_args = serde_json::to_vec(&serde_json::json!({
            "to": BOB,
            "topic": "coordination",
            "payload_hash": "hash_abc",
            "timestamp": 1700000000u64
        }))
        .unwrap();
        let result = dispatch(&mut state, "send_message", &send_args, ALICE);
        let msg_id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(msg_id, 1);

        let s = state.as_ref().unwrap();
        let msg = s.get_message(1).unwrap();
        assert_eq!(msg.from, ALICE);
        assert_eq!(msg.to, BOB);
        assert!(!msg.read);
    }

    #[test]
    fn test_unread_count() {
        let mut state = init_state();

        // Send 3 messages to BOB
        for i in 0..3 {
            let args = serde_json::to_vec(&serde_json::json!({
                "to": BOB,
                "topic": "alert",
                "payload_hash": format!("hash_{}", i),
                "timestamp": 1700000000u64 + i
            }))
            .unwrap();
            dispatch(&mut state, "send_message", &args, ALICE);
        }

        let unread_args = serde_json::to_vec(&serde_json::json!({"address": BOB})).unwrap();
        let result = dispatch(&mut state, "unread_count", &unread_args, ADMIN);
        let count: usize = serde_json::from_slice(&result).unwrap();
        assert_eq!(count, 3);

        // Read one message
        let read_args = serde_json::to_vec(&serde_json::json!({"message_id": 1})).unwrap();
        dispatch(&mut state, "read_message", &read_args, BOB);

        let result2 = dispatch(&mut state, "unread_count", &unread_args, ADMIN);
        let count2: usize = serde_json::from_slice(&result2).unwrap();
        assert_eq!(count2, 2);
    }

    #[test]
    fn test_create_channel_and_join() {
        let mut state = init_state();

        let create_args = serde_json::to_vec(&serde_json::json!({
            "topic": "swarm-coordination",
            "initial_members": [ALICE, BOB]
        }))
        .unwrap();
        let result = dispatch(&mut state, "create_channel", &create_args, ALICE);
        let channel_id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(channel_id, 1);

        // Carol joins
        let join_args = serde_json::to_vec(&serde_json::json!({"channel_id": 1})).unwrap();
        dispatch(&mut state, "join_channel", &join_args, CAROL);

        let s = state.as_ref().unwrap();
        let ch = s.get_channel(1).unwrap();
        assert_eq!(ch.members.len(), 3);
        assert!(ch.members.contains(&CAROL));
    }

    #[test]
    fn test_channel_messages() {
        let mut state = init_state();

        let create_args = serde_json::to_vec(&serde_json::json!({
            "topic": "ops",
            "initial_members": [ALICE, BOB]
        }))
        .unwrap();
        dispatch(&mut state, "create_channel", &create_args, ALICE);

        // Send two channel messages
        for i in 0..2 {
            let args = serde_json::to_vec(&serde_json::json!({
                "channel_id": 1,
                "payload_hash": format!("ch_hash_{}", i),
                "timestamp": 1700000000u64 + i
            }))
            .unwrap();
            dispatch(&mut state, "send_channel_message", &args, ALICE);
        }

        let ch_args = serde_json::to_vec(&serde_json::json!({"channel_id": 1})).unwrap();
        let result = dispatch(&mut state, "channel_messages", &ch_args, ADMIN);
        let msgs: Vec<Message> = serde_json::from_slice(&result).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].topic, "ops");
    }

    #[test]
    #[should_panic(expected = "DRC37: already a member")]
    fn test_cannot_join_channel_twice() {
        let mut state = init_state();

        let create_args = serde_json::to_vec(&serde_json::json!({
            "topic": "test",
            "initial_members": [ALICE]
        }))
        .unwrap();
        dispatch(&mut state, "create_channel", &create_args, ALICE);

        let join_args = serde_json::to_vec(&serde_json::json!({"channel_id": 1})).unwrap();
        dispatch(&mut state, "join_channel", &join_args, ALICE);
    }

    #[test]
    #[should_panic(expected = "DRC37: not a channel member")]
    fn test_non_member_cannot_send_channel_message() {
        let mut state = init_state();

        let create_args = serde_json::to_vec(&serde_json::json!({
            "topic": "private",
            "initial_members": [ALICE]
        }))
        .unwrap();
        dispatch(&mut state, "create_channel", &create_args, ALICE);

        let send_args = serde_json::to_vec(&serde_json::json!({
            "channel_id": 1,
            "payload_hash": "sneaky",
            "timestamp": 999u64
        }))
        .unwrap();
        dispatch(&mut state, "send_channel_message", &send_args, CAROL);
    }

    #[test]
    fn test_creator_auto_added_to_channel() {
        let mut state = init_state();

        let create_args = serde_json::to_vec(&serde_json::json!({
            "topic": "solo",
            "initial_members": []
        }))
        .unwrap();
        dispatch(&mut state, "create_channel", &create_args, ALICE);

        let s = state.as_ref().unwrap();
        let ch = s.get_channel(1).unwrap();
        assert_eq!(ch.members.len(), 1);
        assert_eq!(ch.members[0], ALICE);
    }
}
