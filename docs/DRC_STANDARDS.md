# DRC Standards Reference

## Overview

DRC (Dina Request for Comments) standards define the interfaces, behaviors, and conventions for smart contracts on the Dina Network. There are 30 DRC standards organized into two categories:

1. **DRC 1-18**: Ports of Ethereum ERC standards adapted for Dina's WASM runtime and USDC-denominated economy (skipping DRC-3 which is reserved)
2. **DRC 101-113**: Novel standards designed specifically for AI agent, IoT/robotics, and privacy use cases

All DRC reference implementations are in the `contracts/` directory. Each compiles to a standalone WASM module using the `dina-sdk` and `dina-sdk-macros` crates.

## Complete Standards Table

### ERC Ports (DRC 1-18)

| DRC | Name | ERC Equivalent | Status | Category |
|-----|------|----------------|--------|----------|
| DRC-1 | Fungible Token | ERC-20 | Implemented | Token |
| DRC-2 | Device Identity | -- (novel) | Implemented | Identity |
| DRC-4 | Permit (Gasless Approval) | ERC-2612 | Implemented | Token |
| DRC-5 | Soulbound Token | ERC-5192 | Implemented | Token |
| DRC-6 | NFT | ERC-721 | Implemented | Token |
| DRC-7 | Multi-Token | ERC-1155 | Implemented | Token |
| DRC-8 | Token-Bound Account | ERC-6551 | Implemented | Account |
| DRC-9 | Rental / Lending | ERC-4907 | Implemented | Token |
| DRC-10 | Royalties | ERC-2981 | Implemented | Token |
| DRC-11 | Semi-Fungible Token | ERC-3525 | Implemented | Token |
| DRC-12 | Vault (Yield) | ERC-4626 | Implemented | DeFi |
| DRC-13 | Compliant Token | ERC-3643 | Implemented | Compliance |
| DRC-14 | Contract Signature | ERC-1271 | Implemented | Auth |
| DRC-15 | Meta-Transactions | ERC-2771 | Implemented | Transaction |
| DRC-16 | Proxy (Upgradeable) | ERC-1967 | Implemented | Infrastructure |
| DRC-17 | Hooks | ERC-777 hooks | Implemented | Token |
| DRC-18 | Scriptable | ERC-5169 | Implemented | Token |

### Novel Agent / Robot / Privacy Standards (DRC 101-113)

| DRC | Name | Purpose | Status | Category |
|-----|------|---------|--------|----------|
| DRC-101 | Agent Wallet | AI agent-owned wallets with spending policies | Implemented | Agent |
| DRC-102 | Capability | Delegated permission tokens for agents | Implemented | Agent |
| DRC-103 | Service Agreement | Machine-to-machine SLA contracts | Implemented | Agent |
| DRC-104 | Swarm | Multi-agent coordination and task distribution | Implemented | Agent |
| DRC-105 | Sensor Attestation | IoT sensor data authenticity proofs | Implemented | IoT |
| DRC-106 | Data Market | Buy/sell sensor and AI training data | Implemented | IoT |
| DRC-107 | Reputation | On-chain agent/device reputation scoring | Implemented | Agent |
| DRC-108 | Resource | Compute/bandwidth/storage resource tokens | Implemented | IoT |
| DRC-109 | Emergency Stop | Circuit breaker for autonomous systems | Implemented | Safety |
| DRC-110 | Firmware | On-chain firmware registry and verification | Implemented | IoT |
| DRC-111 | Smart Wallet | Programmable wallet with session keys | Implemented | Account |
| DRC-112 | View Keys | Selective disclosure for privacy compliance | Implemented | Privacy |
| DRC-113 | Relay | Mesh relay incentive and routing protocol | Implemented | Network |

## Standard Specifications

### DRC-1: Fungible Token

**Purpose:** Standard interface for fungible tokens on Dina, equivalent to ERC-20. Used for representing any divisible asset -- stablecoins, utility tokens, governance tokens.

**Interface:**
```rust
fn name() -> String;
fn symbol() -> String;
fn decimals() -> u8;
fn total_supply() -> u64;
fn balance_of(owner: Address) -> u64;
fn transfer(to: Address, amount: u64) -> bool;
fn approve(spender: Address, amount: u64) -> bool;
fn allowance(owner: Address, spender: Address) -> u64;
fn transfer_from(from: Address, to: Address, amount: u64) -> bool;
```

**Events:**
- `Transfer { from: Address, to: Address, amount: u64 }`
- `Approval { owner: Address, spender: Address, amount: u64 }`

**Usage:**
```rust
// Deploy a DRC-1 token
let args = json!({"name": "My Token", "symbol": "MTK", "decimals": 6, "initial_supply": 1000000});
deploy_contract(drc1_wasm, &args);

// Transfer tokens
call_contract(token_addr, "transfer", &json!({"to": recipient, "amount": 100}));
```

---

### DRC-2: Device Identity

**Purpose:** On-chain registry for hardware device identities. Devices register their Ed25519 public key, firmware hash, and attestation. This is a novel standard with no ERC equivalent.

**Interface:**
```rust
fn register(device_pubkey: [u8; 32], owner: Address, attestation: DeviceAttestation) -> Address;
fn verify(device_id: Address) -> bool;
fn get_device(device_id: Address) -> Option<DeviceRecord>;
fn update_firmware(device_id: Address, new_firmware_hash: Hash) -> bool;
fn deactivate(device_id: Address) -> bool;
fn transfer_ownership(device_id: Address, new_owner: Address) -> bool;
```

**Events:**
- `DeviceRegistered { device_id: Address, owner: Address, firmware_hash: Hash }`
- `FirmwareUpdated { device_id: Address, old_hash: Hash, new_hash: Hash }`
- `DeviceDeactivated { device_id: Address }`
- `OwnershipTransferred { device_id: Address, from: Address, to: Address }`

---

### DRC-4: Permit (Gasless Approval)

**Purpose:** Gasless token approvals using Ed25519 signatures, equivalent to ERC-2612. Allows a spender to submit an off-chain-signed approval, paying the gas on behalf of the token owner.

**Interface:**
```rust
fn permit(owner: Address, spender: Address, value: u64, deadline: u64, signature: [u8; 64]);
fn nonces(owner: Address) -> u64;
fn domain_separator() -> Hash;
```

---

### DRC-5: Soulbound Token

**Purpose:** Non-transferable tokens bound to a single address, equivalent to ERC-5192. Used for credentials, certifications, and reputation badges that should not change hands.

**Interface:**
```rust
fn locked(token_id: u64) -> bool;
fn mint(to: Address, token_id: u64) -> bool;
fn revoke(token_id: u64) -> bool;
```

**Events:**
- `Locked { token_id: u64 }`
- `Minted { to: Address, token_id: u64 }`
- `Revoked { token_id: u64 }`

---

### DRC-6: NFT

**Purpose:** Non-fungible token standard, equivalent to ERC-721.

**Interface:**
```rust
fn balance_of(owner: Address) -> u64;
fn owner_of(token_id: u64) -> Address;
fn transfer_from(from: Address, to: Address, token_id: u64);
fn approve(to: Address, token_id: u64);
fn get_approved(token_id: u64) -> Address;
fn set_approval_for_all(operator: Address, approved: bool);
fn is_approved_for_all(owner: Address, operator: Address) -> bool;
```

---

### DRC-7: Multi-Token

**Purpose:** Multi-token standard supporting both fungible and non-fungible tokens in a single contract, equivalent to ERC-1155.

**Interface:**
```rust
fn balance_of(account: Address, id: u64) -> u64;
fn balance_of_batch(accounts: Vec<Address>, ids: Vec<u64>) -> Vec<u64>;
fn safe_transfer_from(from: Address, to: Address, id: u64, amount: u64, data: Vec<u8>);
fn safe_batch_transfer_from(from: Address, to: Address, ids: Vec<u64>, amounts: Vec<u64>, data: Vec<u8>);
fn set_approval_for_all(operator: Address, approved: bool);
fn is_approved_for_all(account: Address, operator: Address) -> bool;
```

---

### DRC-8: Token-Bound Account

**Purpose:** Accounts owned by NFTs, equivalent to ERC-6551. Allows an NFT to own assets and interact with contracts.

**Interface:**
```rust
fn create_account(token_contract: Address, token_id: u64) -> Address;
fn account(token_contract: Address, token_id: u64) -> Address;
fn execute_call(to: Address, value: u64, data: Vec<u8>) -> Vec<u8>;
fn token() -> (Address, u64);
fn owner() -> Address;
```

---

### DRC-9: Rental / Lending

**Purpose:** NFT rental standard, equivalent to ERC-4907. Allows NFT owners to rent out their tokens for a time period without transferring ownership.

**Interface:**
```rust
fn set_user(token_id: u64, user: Address, expires: u64);
fn user_of(token_id: u64) -> Address;
fn user_expires(token_id: u64) -> u64;
```

---

### DRC-10: Royalties

**Purpose:** Royalty payment standard, equivalent to ERC-2981. Returns royalty information for secondary sales.

**Interface:**
```rust
fn royalty_info(token_id: u64, sale_price: u64) -> (Address, u64);
fn set_default_royalty(receiver: Address, fee_numerator: u64);
fn set_token_royalty(token_id: u64, receiver: Address, fee_numerator: u64);
```

---

### DRC-11: Semi-Fungible Token

**Purpose:** Semi-fungible token with value and slot attributes, equivalent to ERC-3525.

**Interface:**
```rust
fn value_of(token_id: u64) -> u64;
fn slot_of(token_id: u64) -> u64;
fn transfer_value(from_token_id: u64, to_token_id: u64, value: u64);
fn approve_value(token_id: u64, operator: Address, value: u64);
fn allowance_value(token_id: u64, operator: Address) -> u64;
```

---

### DRC-12: Vault (Yield)

**Purpose:** Tokenized vault standard, equivalent to ERC-4626. Used for yield-bearing deposits and lending pools.

**Interface:**
```rust
fn asset() -> Address;
fn total_assets() -> u64;
fn deposit(assets: u64, receiver: Address) -> u64;
fn withdraw(assets: u64, receiver: Address, owner: Address) -> u64;
fn preview_deposit(assets: u64) -> u64;
fn preview_withdraw(assets: u64) -> u64;
fn max_deposit(receiver: Address) -> u64;
fn max_withdraw(owner: Address) -> u64;
```

---

### DRC-13: Compliant Token

**Purpose:** Compliance-ready token with transfer restrictions, equivalent to ERC-3643. Used for regulated securities and KYC-gated tokens.

**Interface:**
```rust
fn is_verified(addr: Address) -> bool;
fn add_agent(agent: Address);
fn remove_agent(agent: Address);
fn freeze(addr: Address);
fn unfreeze(addr: Address);
fn is_frozen(addr: Address) -> bool;
fn forced_transfer(from: Address, to: Address, amount: u64);
```

---

### DRC-14: Contract Signature

**Purpose:** Standard for contracts to validate signatures, equivalent to ERC-1271.

**Interface:**
```rust
fn is_valid_signature(hash: Hash, signature: [u8; 64]) -> [u8; 4];
```

Returns `0x1626ba7e` if valid, `0xffffffff` if invalid.

---

### DRC-15: Meta-Transactions

**Purpose:** Gasless transactions via a trusted forwarder, equivalent to ERC-2771.

**Interface:**
```rust
fn is_trusted_forwarder(forwarder: Address) -> bool;
fn execute(from: Address, to: Address, data: Vec<u8>, gas: u64, nonce: u64, signature: [u8; 64]) -> Vec<u8>;
```

---

### DRC-16: Proxy (Upgradeable)

**Purpose:** Upgradeable proxy contract pattern, equivalent to ERC-1967.

**Interface:**
```rust
fn implementation() -> Address;
fn upgrade_to(new_implementation: Address);
fn admin() -> Address;
fn change_admin(new_admin: Address);
```

---

### DRC-17: Hooks

**Purpose:** Hooks for token send/receive operations, inspired by ERC-777.

**Interface:**
```rust
fn tokens_to_send(operator: Address, from: Address, to: Address, amount: u64, data: Vec<u8>);
fn tokens_received(operator: Address, from: Address, to: Address, amount: u64, data: Vec<u8>);
fn register_hook(interface_hash: Hash, implementer: Address);
```

---

### DRC-18: Scriptable

**Purpose:** Attach off-chain scripts to tokens, equivalent to ERC-5169.

**Interface:**
```rust
fn script_uri(token_id: u64) -> Vec<String>;
fn set_script_uri(token_id: u64, uris: Vec<String>);
```

---

### DRC-101: Agent Wallet

**Purpose:** AI agent-owned wallets with spending policies. Allows autonomous agents to hold funds and transact within defined limits, with human oversight via spending caps and allowlists.

**Interface:**
```rust
fn create_wallet(agent_id: Address, owner: Address) -> Address;
fn set_spending_limit(wallet: Address, daily_limit: u64);
fn set_allowed_recipients(wallet: Address, recipients: Vec<Address>);
fn set_allowed_contracts(wallet: Address, contracts: Vec<Address>);
fn execute(wallet: Address, to: Address, amount: u64, data: Vec<u8>) -> Vec<u8>;
fn get_spending_today(wallet: Address) -> u64;
fn pause(wallet: Address);
fn resume(wallet: Address);
```

**Events:**
- `WalletCreated { wallet: Address, agent: Address, owner: Address }`
- `SpendingLimitSet { wallet: Address, daily_limit: u64 }`
- `ExecutionPerformed { wallet: Address, to: Address, amount: u64 }`
- `WalletPaused { wallet: Address }`

---

### DRC-102: Capability

**Purpose:** Delegated capability tokens that grant specific permissions to agents. Think of them as revocable, expirable permission slips.

**Interface:**
```rust
fn grant(grantee: Address, capability: String, expiry: u64) -> u64;
fn revoke(capability_id: u64);
fn check(grantee: Address, capability: String) -> bool;
fn delegate(capability_id: u64, delegatee: Address) -> u64;
fn list_capabilities(holder: Address) -> Vec<CapabilityInfo>;
```

---

### DRC-103: Service Agreement

**Purpose:** Machine-to-machine service level agreements. Two agents or devices agree on service terms, pricing, and penalties on-chain.

**Interface:**
```rust
fn propose(provider: Address, consumer: Address, terms: ServiceTerms) -> u64;
fn accept(agreement_id: u64);
fn report_delivery(agreement_id: u64, proof: Vec<u8>);
fn dispute(agreement_id: u64, reason: String);
fn settle(agreement_id: u64);
fn get_agreement(agreement_id: u64) -> ServiceAgreement;
```

---

### DRC-104: Swarm

**Purpose:** Multi-agent coordination and task distribution. A swarm leader can assign tasks to member agents, track completion, and distribute rewards.

**Interface:**
```rust
fn create_swarm(leader: Address, members: Vec<Address>) -> u64;
fn assign_task(swarm_id: u64, member: Address, task: TaskSpec);
fn report_completion(swarm_id: u64, task_id: u64, result: Vec<u8>);
fn distribute_rewards(swarm_id: u64);
fn join_swarm(swarm_id: u64);
fn leave_swarm(swarm_id: u64);
```

---

### DRC-105: Sensor Attestation

**Purpose:** IoT sensor data authenticity proofs. Sensors sign their readings with Ed25519 keys, and the attestation is verifiable on-chain.

**Interface:**
```rust
fn attest(sensor_id: Address, reading: SensorReading, signature: [u8; 64]) -> u64;
fn verify_attestation(attestation_id: u64) -> bool;
fn get_readings(sensor_id: Address, from_time: u64, to_time: u64) -> Vec<SensorReading>;
fn register_sensor(sensor_id: Address, metadata: SensorMetadata);
```

---

### DRC-106: Data Market

**Purpose:** Buy and sell sensor data and AI training datasets. Data providers list datasets with pricing, and consumers purchase access rights.

**Interface:**
```rust
fn list_dataset(provider: Address, metadata: DatasetMetadata, price: u64) -> u64;
fn purchase(dataset_id: u64, buyer: Address) -> AccessToken;
fn rate_dataset(dataset_id: u64, rating: u8);
fn get_listings(category: String, limit: u64) -> Vec<DatasetListing>;
fn revoke_access(dataset_id: u64, buyer: Address);
```

---

### DRC-107: Reputation

**Purpose:** On-chain reputation scoring for agents and devices. Reputation is accumulated through successful transactions and service delivery.

**Interface:**
```rust
fn get_score(entity: Address) -> u64;
fn record_positive(entity: Address, context: String, weight: u64);
fn record_negative(entity: Address, context: String, weight: u64);
fn get_history(entity: Address, limit: u64) -> Vec<ReputationEvent>;
fn calculate_trust_score(entity: Address) -> f64;
```

---

### DRC-108: Resource

**Purpose:** Tokenized compute, bandwidth, and storage resources. Devices can offer and consume resources with on-chain accounting.

**Interface:**
```rust
fn register_resource(provider: Address, resource_type: ResourceType, capacity: u64) -> u64;
fn reserve(resource_id: u64, consumer: Address, amount: u64, duration: u64);
fn release(reservation_id: u64);
fn get_available(resource_type: ResourceType) -> Vec<ResourceInfo>;
fn report_usage(reservation_id: u64, actual_usage: u64);
```

---

### DRC-109: Emergency Stop

**Purpose:** Circuit breaker for autonomous systems. Authorized parties can immediately halt agent operations.

**Interface:**
```rust
fn register_system(system_id: Address, operators: Vec<Address>);
fn emergency_stop(system_id: Address, reason: String);
fn resume(system_id: Address);
fn is_stopped(system_id: Address) -> bool;
fn add_operator(system_id: Address, operator: Address);
fn remove_operator(system_id: Address, operator: Address);
```

---

### DRC-110: Firmware

**Purpose:** On-chain firmware registry and integrity verification. Manufacturers register firmware hashes, and devices verify updates against the registry.

**Interface:**
```rust
fn register_firmware(manufacturer: Address, version: String, hash: Hash, signature: [u8; 64]) -> u64;
fn verify_firmware(firmware_id: u64, device_hash: Hash) -> bool;
fn get_latest(manufacturer: Address) -> FirmwareInfo;
fn get_version(manufacturer: Address, version: String) -> Option<FirmwareInfo>;
fn revoke_firmware(firmware_id: u64);
```

---

### DRC-111: Smart Wallet

**Purpose:** Programmable wallet with session keys, social recovery, and batched transactions. Provides the account abstraction layer for Dina.

**Interface:**
```rust
fn add_session_key(key: [u8; 32], permissions: KeyPermission, expiry: u64);
fn remove_session_key(key: [u8; 32]);
fn execute_batch(calls: Vec<Call>) -> Vec<Vec<u8>>;
fn add_guardian(guardian: Address);
fn remove_guardian(guardian: Address);
fn recover(new_owner: Address, guardian_signatures: Vec<[u8; 64]>);
fn get_session_keys() -> Vec<SessionKeyInfo>;
```

---

### DRC-112: View Keys

**Purpose:** Selective disclosure for privacy compliance. Account holders can grant view-only access to auditors or regulators without exposing spending keys.

**Interface:**
```rust
fn grant_view_access(viewer: Address, scope: ViewScope);
fn revoke_view_access(viewer: Address);
fn can_view(viewer: Address, resource: Address) -> bool;
fn get_viewers() -> Vec<ViewerInfo>;
fn generate_view_key(scope: ViewScope) -> [u8; 32];
```

---

### DRC-113: Relay

**Purpose:** Mesh relay incentive and routing protocol. Defines how relay nodes earn fees for forwarding settlement blobs over BLE.

**Interface:**
```rust
fn register_relay(node: Address, capacity: u64);
fn submit_relay_proof(blob_hash: Hash, relayer: Address, hop_count: u8);
fn claim_relay_fees(relayer: Address) -> u64;
fn get_relay_stats(relayer: Address) -> RelayStats;
fn set_fee_schedule(min_fee: u64, fee_per_hop: u64);
```

---

## Required vs Optional Standards

### Required for All Tokens

Any token contract on Dina **must** implement DRC-1 (for fungible) or DRC-6 (for NFT) as a base standard.

### Required for Compliance

Regulated token issuers must additionally implement:
- DRC-13 (Compliant Token) for transfer restrictions
- DRC-112 (View Keys) for auditor access

### Required for Device Registration

Hardware devices registering on-chain must implement:
- DRC-2 (Device Identity)
- DRC-105 (Sensor Attestation) if producing sensor data

### Optional Extensions

All other DRC standards are optional and can be composed freely. For example, a token contract could implement DRC-1 + DRC-4 + DRC-10 + DRC-17 for a fungible token with gasless approvals, royalties, and send/receive hooks.

## DRC Interaction Map

```
DRC-1 (Token) ----+---- DRC-4 (Permit) --- gasless approvals
                   |
                   +---- DRC-10 (Royalties) --- secondary sale fees
                   |
                   +---- DRC-13 (Compliant) --- transfer restrictions
                   |         |
                   |         +---- DRC-112 (View Keys) --- auditor access
                   |
                   +---- DRC-17 (Hooks) --- send/receive callbacks

DRC-6 (NFT) ------+---- DRC-8 (Token-Bound Account) --- NFT-owned accounts
                   |
                   +---- DRC-9 (Rental) --- time-limited usage rights
                   |
                   +---- DRC-10 (Royalties) --- creator royalties
                   |
                   +---- DRC-18 (Scriptable) --- attached scripts

DRC-101 (Agent Wallet) --+---- DRC-102 (Capability) --- delegated permissions
                          |
                          +---- DRC-103 (Service Agreement) --- SLAs
                          |
                          +---- DRC-107 (Reputation) --- trust scoring
                          |
                          +---- DRC-109 (Emergency Stop) --- kill switch

DRC-2 (Device Identity) -+---- DRC-105 (Sensor Attestation) --- data proofs
                          |
                          +---- DRC-106 (Data Market) --- sell sensor data
                          |
                          +---- DRC-108 (Resource) --- offer compute
                          |
                          +---- DRC-110 (Firmware) --- update verification

DRC-111 (Smart Wallet) --+---- DRC-112 (View Keys) --- privacy
                          |
                          +---- DRC-16 (Proxy) --- upgradeable logic

DRC-104 (Swarm) -------------- DRC-107 (Reputation) --- member scoring
                          |
                          +---- DRC-103 (Service Agreement) --- task SLAs

DRC-113 (Relay) -------------- DRC-107 (Reputation) --- relay reputation
```

## How to Propose a New DRC

1. **Discussion**: Open a GitHub issue titled `DRC-XXX: [Standard Name]` with a description of the use case, motivation, and rough interface.

2. **Specification**: Create a formal specification document with:
   - Abstract (100 words)
   - Motivation and rationale
   - Interface (method signatures with types)
   - Events
   - Interactions with other DRCs
   - Security considerations
   - Reference implementation

3. **Reference Implementation**: Create a new contract crate at `contracts/drc{N}-{name}/` with:
   - `Cargo.toml` with `crate-type = ["cdylib"]` and `dina-sdk` dependency
   - `src/lib.rs` implementing `dispatch(state, method, args, caller) -> Result<Vec<u8>>`
   - Unit tests using `dina-sdk::TestRuntime`

4. **Add to Workspace**: Add the crate to the `members` list in the root `Cargo.toml`.

5. **Review**: Submit a pull request. The DRC committee will review for:
   - Interface consistency with existing DRCs
   - No overlap with existing standards
   - Complete test coverage
   - Security analysis

6. **Numbering**:
   - DRC 1-99: Ports of established ERC standards
   - DRC 100-199: Novel agent/robot/privacy standards
   - DRC 200+: Community-proposed standards
