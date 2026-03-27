use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-113  Relay Protocol  (no ERC equivalent)
// ---------------------------------------------------------------------------

type Address = [u8; 32];
type ChannelId = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SettlementBlob {
    pub channel_id: ChannelId,
    pub balance_a: u64,
    pub balance_b: u64,
    pub sequence: u64,
    pub party_a: Address,
    pub party_b: Address,
    pub signature_a: Vec<u8>,
    pub signature_b: Vec<u8>,
    pub relay_fee: u64,
    pub submitted_by: Address,
    pub submitted_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelayInfo {
    pub address: Address,
    pub total_relays: u64,
    pub total_fees_earned: u64,
    pub registered_at: u64,
    pub reputation: u64, // 0-10000
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelayRecord {
    pub channel_id: ChannelId,
    pub settlement_hash: [u8; 32],
    pub fee_earned: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelayRegistry {
    pub admin: Address,
    pub pending_settlements: BTreeMap<ChannelId, SettlementBlob>,
    pub relays: BTreeMap<Address, RelayInfo>,
    pub relay_history: BTreeMap<Address, Vec<RelayRecord>>,
    pub finalized_count: u64,
    /// Challenge period in seconds (e.g. 3600 = 1 hour)
    pub challenge_period: u64,
    pub current_time: u64,
}

impl RelayRegistry {
    pub fn new(admin: Address, challenge_period: u64) -> Self {
        Self {
            admin,
            pending_settlements: BTreeMap::new(),
            relays: BTreeMap::new(),
            relay_history: BTreeMap::new(),
            finalized_count: 0,
            challenge_period,
            current_time: 0,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn relay_info(&self, addr: &Address) -> Option<&RelayInfo> {
        self.relays.get(addr)
    }

    pub fn relay_history(&self, addr: &Address) -> Vec<RelayRecord> {
        self.relay_history.get(addr).cloned().unwrap_or_default()
    }

    pub fn pending_for_channel(&self, channel_id: &ChannelId) -> Option<&SettlementBlob> {
        self.pending_settlements.get(channel_id)
    }

    // -- Mutations -----------------------------------------------------------

    pub fn register_relay(&mut self, addr: Address, timestamp: u64) {
        assert!(
            !self.relays.contains_key(&addr),
            "DRC113: relay already registered"
        );
        self.relays.insert(
            addr,
            RelayInfo {
                address: addr,
                total_relays: 0,
                total_fees_earned: 0,
                registered_at: timestamp,
                reputation: 5000, // start at 50%
            },
        );
    }

    pub fn submit_settlement(&mut self, caller: Address, mut blob: SettlementBlob) {
        // Validate both signatures exist (non-empty)
        assert!(
            !blob.signature_a.is_empty(),
            "DRC113: signature_a is missing"
        );
        assert!(
            !blob.signature_b.is_empty(),
            "DRC113: signature_b is missing"
        );
        assert!(
            blob.party_a != blob.party_b,
            "DRC113: parties must be different"
        );

        blob.submitted_by = caller;

        // If there is already a pending settlement, only accept if sequence is higher
        if let Some(existing) = self.pending_settlements.get(&blob.channel_id) {
            assert!(
                blob.sequence > existing.sequence,
                "DRC113: settlement sequence must be higher than existing ({} <= {})",
                blob.sequence,
                existing.sequence,
            );
        }

        self.pending_settlements.insert(blob.channel_id, blob);
    }

    pub fn challenge_settlement(&mut self, channel_id: ChannelId, mut newer_blob: SettlementBlob) {
        let existing = self
            .pending_settlements
            .get(&channel_id)
            .expect("DRC113: no pending settlement for channel");

        assert!(
            newer_blob.sequence > existing.sequence,
            "DRC113: challenger sequence must be higher than existing ({} <= {})",
            newer_blob.sequence,
            existing.sequence,
        );
        assert!(
            !newer_blob.signature_a.is_empty(),
            "DRC113: signature_a is missing"
        );
        assert!(
            !newer_blob.signature_b.is_empty(),
            "DRC113: signature_b is missing"
        );
        assert!(
            newer_blob.party_a == existing.party_a && newer_blob.party_b == existing.party_b,
            "DRC113: parties must match existing settlement"
        );

        newer_blob.channel_id = channel_id;
        self.pending_settlements.insert(channel_id, newer_blob);
    }

    pub fn finalize_settlement(&mut self, channel_id: ChannelId, current_time: u64) {
        let blob = self
            .pending_settlements
            .get(&channel_id)
            .expect("DRC113: no pending settlement for channel");

        // Check challenge period has elapsed
        assert!(
            current_time >= blob.submitted_at + self.challenge_period,
            "DRC113: challenge period has not elapsed"
        );

        let relay_fee = blob.relay_fee;
        let submitter = blob.submitted_by;

        // Derive a simple settlement hash from channel_id + sequence
        let mut settlement_hash = [0u8; 32];
        settlement_hash[..32].copy_from_slice(&channel_id);
        let seq_bytes = blob.sequence.to_le_bytes();
        for i in 0..8 {
            settlement_hash[i] ^= seq_bytes[i];
        }

        let record = RelayRecord {
            channel_id,
            settlement_hash,
            fee_earned: relay_fee,
            timestamp: current_time,
        };

        // Update relay stats if submitter is a registered relay
        if let Some(relay) = self.relays.get_mut(&submitter) {
            relay.total_relays += 1;
            relay.total_fees_earned += relay_fee;
            // Increase reputation (cap at 10000)
            relay.reputation = (relay.reputation + 100).min(10000);
        }

        self.relay_history
            .entry(submitter)
            .or_default()
            .push(record);

        // Remove from pending
        self.pending_settlements.remove(&channel_id);
        self.finalized_count += 1;
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    challenge_period: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RegisterRelayArgs {
    addr: Address,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SubmitSettlementArgs {
    blob: SettlementBlob,
}

#[derive(Serialize, Deserialize, Debug)]
struct RelayInfoArgs {
    addr: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct RelayHistoryArgs {
    addr: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct PendingForChannelArgs {
    channel_id: ChannelId,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChallengeSettlementArgs {
    channel_id: ChannelId,
    newer_blob: SettlementBlob,
}

#[derive(Serialize, Deserialize, Debug)]
struct FinalizeSettlementArgs {
    channel_id: ChannelId,
    current_time: u64,
}

pub fn dispatch(
    state: &mut Option<RelayRegistry>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC113: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC113: bad init args");
            *state = Some(RelayRegistry::new(caller, a.challenge_period));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "relay_info" => {
            let s = state.as_ref().expect("DRC113: not initialised");
            let a: RelayInfoArgs =
                serde_json::from_slice(args).expect("DRC113: bad relay_info args");
            serde_json::to_vec(&s.relay_info(&a.addr)).unwrap()
        }
        "relay_history" => {
            let s = state.as_ref().expect("DRC113: not initialised");
            let a: RelayHistoryArgs =
                serde_json::from_slice(args).expect("DRC113: bad relay_history args");
            serde_json::to_vec(&s.relay_history(&a.addr)).unwrap()
        }
        "pending_for_channel" => {
            let s = state.as_ref().expect("DRC113: not initialised");
            let a: PendingForChannelArgs =
                serde_json::from_slice(args).expect("DRC113: bad pending_for_channel args");
            serde_json::to_vec(&s.pending_for_channel(&a.channel_id)).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "submit_settlement" => {
            let s = state.as_mut().expect("DRC113: not initialised");
            let a: SubmitSettlementArgs =
                serde_json::from_slice(args).expect("DRC113: bad submit_settlement args");
            s.submit_settlement(caller, a.blob);
            serde_json::to_vec("ok").unwrap()
        }
        "register_relay" => {
            let s = state.as_mut().expect("DRC113: not initialised");
            let a: RegisterRelayArgs =
                serde_json::from_slice(args).expect("DRC113: bad register_relay args");
            s.register_relay(a.addr, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "challenge_settlement" => {
            let s = state.as_mut().expect("DRC113: not initialised");
            let a: ChallengeSettlementArgs =
                serde_json::from_slice(args).expect("DRC113: bad challenge_settlement args");
            s.challenge_settlement(a.channel_id, a.newer_blob);
            serde_json::to_vec("ok").unwrap()
        }
        "finalize_settlement" => {
            let s = state.as_mut().expect("DRC113: not initialised");
            let a: FinalizeSettlementArgs =
                serde_json::from_slice(args).expect("DRC113: bad finalize_settlement args");
            s.finalize_settlement(a.channel_id, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC113: unknown method '{method}'"),
    }
}
