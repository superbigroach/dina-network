use drc37_agent_communication::{dispatch, CommState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_comm(admin: [u8; 32]) -> Option<CommState> {
    let mut state: Option<CommState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

#[test]
fn send_and_read_message() {
    let alice = addr(1);
    let bob = addr(2);
    let mut state = init_comm(alice);

    let send_args = serde_json::to_vec(&serde_json::json!({
        "to": bob,
        "topic": "coordination",
        "payload_hash": "hash_abc",
        "timestamp": 1000u64
    }))
    .unwrap();
    let result = dispatch(&mut state, "send_message", &send_args, alice);
    let msg_id: u64 = serde_json::from_slice(&result).unwrap();

    // Check unread
    let s = state.as_ref().unwrap();
    assert_eq!(s.unread_count(&bob), 1);

    // Read message
    let read_args = serde_json::to_vec(&serde_json::json!({ "message_id": msg_id })).unwrap();
    dispatch(&mut state, "read_message", &read_args, bob);

    let s = state.as_ref().unwrap();
    assert_eq!(s.unread_count(&bob), 0);
    assert!(s.get_message(msg_id).unwrap().read);
}

#[test]
fn create_channel_and_send_message() {
    let alice = addr(1);
    let bob = addr(2);
    let mut state = init_comm(alice);

    let create_args = serde_json::to_vec(&serde_json::json!({
        "topic": "swarm-coordination",
        "initial_members": [bob]
    }))
    .unwrap();
    let result = dispatch(&mut state, "create_channel", &create_args, alice);
    let channel_id: u64 = serde_json::from_slice(&result).unwrap();

    let s = state.as_ref().unwrap();
    let channel = s.get_channel(channel_id).unwrap();
    assert_eq!(channel.members.len(), 2); // alice + bob
    assert_eq!(channel.topic, "swarm-coordination");

    // Send channel message
    let msg_args = serde_json::to_vec(&serde_json::json!({
        "channel_id": channel_id,
        "payload_hash": "broadcast_hash",
        "timestamp": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "send_channel_message", &msg_args, alice);

    let s = state.as_ref().unwrap();
    assert_eq!(s.channel_messages(channel_id).len(), 1);
}

#[test]
fn join_channel() {
    let alice = addr(1);
    let charlie = addr(3);
    let mut state = init_comm(alice);

    let create_args = serde_json::to_vec(&serde_json::json!({
        "topic": "open-channel",
        "initial_members": []
    }))
    .unwrap();
    let result = dispatch(&mut state, "create_channel", &create_args, alice);
    let channel_id: u64 = serde_json::from_slice(&result).unwrap();

    let join_args = serde_json::to_vec(&serde_json::json!({ "channel_id": channel_id })).unwrap();
    dispatch(&mut state, "join_channel", &join_args, charlie);

    let s = state.as_ref().unwrap();
    assert_eq!(s.get_channel(channel_id).unwrap().members.len(), 2);
}

#[test]
#[should_panic(expected = "already a member")]
fn cannot_join_twice() {
    let alice = addr(1);
    let mut state = init_comm(alice);

    let create_args = serde_json::to_vec(&serde_json::json!({
        "topic": "test",
        "initial_members": []
    }))
    .unwrap();
    let result = dispatch(&mut state, "create_channel", &create_args, alice);
    let channel_id: u64 = serde_json::from_slice(&result).unwrap();

    let join_args = serde_json::to_vec(&serde_json::json!({ "channel_id": channel_id })).unwrap();
    dispatch(&mut state, "join_channel", &join_args, alice);
}

#[test]
#[should_panic(expected = "not a channel member")]
fn non_member_cannot_send_to_channel() {
    let alice = addr(1);
    let outsider = addr(99);
    let mut state = init_comm(alice);

    let create_args = serde_json::to_vec(&serde_json::json!({
        "topic": "private",
        "initial_members": []
    }))
    .unwrap();
    let result = dispatch(&mut state, "create_channel", &create_args, alice);
    let channel_id: u64 = serde_json::from_slice(&result).unwrap();

    let msg_args = serde_json::to_vec(&serde_json::json!({
        "channel_id": channel_id,
        "payload_hash": "hack",
        "timestamp": 1000u64
    }))
    .unwrap();
    dispatch(&mut state, "send_channel_message", &msg_args, outsider);
}

#[test]
fn unread_count_tracks_correctly() {
    let alice = addr(1);
    let bob = addr(2);
    let mut state = init_comm(alice);

    for i in 0..3 {
        let args = serde_json::to_vec(&serde_json::json!({
            "to": bob,
            "topic": "ping",
            "payload_hash": format!("hash_{i}"),
            "timestamp": (1000 + i) as u64
        }))
        .unwrap();
        dispatch(&mut state, "send_message", &args, alice);
    }

    let s = state.as_ref().unwrap();
    assert_eq!(s.unread_count(&bob), 3);
    assert_eq!(s.unread_count(&alice), 0);
}
