# Dina Network Bridge Guide

This guide covers all bridging options available on Dina Network for bringing USDC and other assets cross-chain.

## Overview

Dina Network supports 5 bridge integrations, each optimized for different use cases:

| Bridge | Token Standard | Trust Model | Speed | Best For |
|--------|---------------|-------------|-------|----------|
| **CCTP** | Bridged USDC (USDC.e) | Circle attestation | ~15 min | USDC transfers (official) |
| **Base Bridge** | Bridged USDC (USDC.e) | Trusted relayer | ~5 min | Base <-> Dina direct |
| **Wormhole** | Bridged USDC (USDC.e) | Guardian network (19) | ~15 min | Multi-chain, large transfers |
| **LayerZero** | OFT (USDC.e) | DVN network | ~3 min | Fast, omnichain |
| **Axelar** | ITS (USDC.e) | Validator network | ~5 min | GMP + token transfers |

## Architecture

All bridges share the same **Bridged USDC token contract** (`bridge-usdc`) on Dina, which follows Circle's official Bridged USDC Standard. Only the authorized bridge contract can mint and burn USDC.e.

```
                           Dina Network
                    +-----------------------+
                    |                       |
                    |   Bridged USDC Token  |  <-- Single token, multiple bridges
                    |      (USDC.e)         |
                    |                       |
                    +---+---+---+---+---+---+
                        |   |   |   |   |
                   CCTP | B | W | L | A |
                        | a | o | a | x |
                        | s | r | y | e |
                        | e | m | e | l |
                        |   | h | r | a |
                        |   | o | Z | r |
                        |   | l | e |   |
                        |   | e | r |   |
                        |   |   | o |   |
                    +---+---+---+---+---+---+
                    |                       |
                    |   External Chains     |
                    | Ethereum, Base, Sol,  |
                    | Arbitrum, etc.        |
                    +-----------------------+
```

## 1. Circle CCTP (Cross-Chain Transfer Protocol)

**Contract:** `bridge-cctp`

### How It Works

CCTP is Circle's official cross-chain transfer protocol. It uses a burn-and-mint model with Circle's attestation service as the trust anchor.

```
Source Chain                    Circle                     Dina Network
+------------+          +----------------+          +------------------+
| 1. User    |          | 3. Attestation |          | 5. Mint USDC.e   |
|    calls   |   burn   |    service     |  signed  |    to recipient  |
| deposit_   |--------->|    observes    |--------->|                  |
| for_burn() |          |    & signs     |          | receive_message()|
+------------+          +----------------+          +------------------+
      |                                                      |
      | 2. USDC burned                          6. USDC.e minted
      |    on source                               on Dina
```

### Domain IDs

| Chain | Domain ID |
|-------|-----------|
| Ethereum | 0 |
| Arbitrum | 3 |
| Solana | 5 |
| Base | 6 |
| **Dina** | **99** |

### When to Use

- Official Circle-supported USDC transfers
- Highest security (Circle is the trust anchor)
- When you need regulatory compliance (Circle KYC/AML)
- Large institutional transfers

### Fees

- No protocol fee (Circle does not charge)
- Only gas costs on source and destination chains
- Dina gas is negligible (<$0.001)

### Security Model

- **Trust anchor:** Circle's attestation service
- **Attestation:** Circle's attester set signs each burn message
- **Replay protection:** Nonce tracking per source domain
- **Pause capability:** Owner can pause in emergencies

---

## 2. Base Bridge (Direct Lock/Mint)

**Contract:** `bridge-base`

### How It Works

A direct lock/mint bridge between Base and Dina, optimized for the most common bridging path (since Lucilla's USDC is on Base via Circle Modular Wallets).

```
Base Chain                   Relayer                    Dina Network
+------------+          +----------------+          +------------------+
| 1. Lock    |  event   | 2. Observe     |  proof   | 3. Verify proof  |
|    USDC in |--------->|    lock event  |--------->|    & mint USDC.e |
|    bridge  |          |    on Base     |          |    to recipient  |
+------------+          +----------------+          +------------------+

Dina Network                 Relayer                    Base Chain
+------------+          +----------------+          +------------------+
| 1. Burn    |  event   | 2. Observe     |  release | 3. Release       |
|    USDC.e  |--------->|    burn event  |--------->|    locked USDC   |
|    on Dina |          |    on Dina     |          |    to recipient  |
+------------+          +----------------+          +------------------+
```

### When to Use

- Direct Base <-> Dina transfers
- Lucilla app wallet top-ups
- Fastest path for Base-native USDC
- Lower value transfers where relayer trust is acceptable

### Fees

- 0.1% bridge fee (configurable, max 5%)
- Minimum: 0.001 USDC
- Maximum: 1,000,000 USDC per transaction

### Security Model

- **Trust anchor:** Trusted relayer (upgradeable to decentralized set)
- **Proof verification:** SHA-256 proof of Base transaction
- **Replay protection:** Processed deposit tracking
- **Limits:** Configurable min/max per transaction
- **Pause capability:** Owner can pause

---

## 3. Wormhole

**Contract:** `bridge-wormhole`

### How It Works

Wormhole uses a guardian network of 19 validators who observe and attest to cross-chain messages via Verified Action Approvals (VAAs).

```
Source Chain              Guardian Network              Dina Network
+------------+          +----------------+          +------------------+
| 1. Burn    |  observe | 2. 19 guardians|   VAA    | 4. Verify 13/19  |
|    tokens  |--------->|    observe &   |--------->|    guardian sigs  |
|            |          |    sign VAA    |          |    & mint tokens  |
+------------+          | 3. 13/19 sigs  |          +------------------+
                        |    required    |
                        +----------------+
```

### Wormhole Chain IDs

| Chain | Chain ID |
|-------|----------|
| Solana | 1 |
| Ethereum | 2 |
| Arbitrum | 23 |
| Base | 30 |
| **Dina** | **99** |

### When to Use

- Multi-chain transfers (broadest chain support)
- Large value transfers (strong security)
- When you need Solana connectivity
- When decentralized verification is important

### Fees

- Wormhole protocol: no fee
- Gas costs on source and destination
- Relayer tip (if using automatic relaying)

### Security Model

- **Trust anchor:** 19-guardian network (2/3 + 1 quorum = 13 signatures)
- **VAA verification:** Each transfer requires 13/19 guardian signatures
- **Replay protection:** Consumed VAA hash tracking
- **Guardian set updates:** Versioned guardian sets with owner control

---

## 4. LayerZero

**Contract:** `bridge-layerzero`

### How It Works

LayerZero uses an Ultra Light Node (ULN) architecture with Decentralized Verifier Networks (DVNs) for fast, configurable cross-chain messaging. The OFT (Omnichain Fungible Token) standard enables seamless multi-chain tokens.

```
Dina Network             LayerZero DVNs              Destination Chain
+------------+          +----------------+          +------------------+
| 1. Burn    |   ULN    | 2. DVNs verify |  relay   | 4. lz_receive()  |
|    tokens  |--------->|    the message |--------->|    mints tokens  |
| send()     |          | 3. Executor    |          |    to recipient  |
+------------+          |    relays msg  |          +------------------+
                        +----------------+
```

### LayerZero Endpoint v2 Chain IDs

| Chain | Endpoint ID |
|-------|-------------|
| Ethereum | 30101 |
| Arbitrum | 30110 |
| Solana | 30168 |
| Base | 30184 |
| **Dina** | **30099** |

### When to Use

- Fastest cross-chain transfers
- When you need configurable security (choose your DVNs)
- Omnichain applications (not just token transfers)
- When gas optimization matters (adapter params)

### Fees

- LayerZero messaging fee (varies by DVN config)
- Destination gas (configurable via adapter params)
- No protocol fee on Dina side

### Security Model

- **Trust anchor:** Configurable DVN set (choose your verifiers)
- **Trusted remotes:** Explicit per-chain trust configuration
- **Nonce ordering:** Sequential nonces per path, out-of-order stored as failed
- **Failed message retry:** Owner can retry failed messages
- **Adapter params:** Per-message gas configuration

---

## 5. Axelar

**Contract:** `bridge-axelar`

### How It Works

Axelar provides both token bridging (ITS) and general message passing (GMP) through a decentralized validator network and the Axelar Gateway contract.

```
Source Chain              Axelar Network              Dina Network
+------------+          +----------------+          +------------------+
| 1. Call    |  GMP     | 2. Axelar      | command  | 4. Gateway calls |
|    gateway |--------->|    validators  |--------->|    execute() or  |
|    .send() |          |    confirm msg |          |    execute_with_ |
+------------+          | 3. Generate    |          |    token()       |
                        |    command_id  |          +------------------+
                        +----------------+
```

### When to Use

- When you need GMP (General Message Passing) beyond just tokens
- Complex cross-chain workflows (e.g., cross-chain contract calls)
- When you want both token transfers AND arbitrary message execution
- Cosmos ecosystem connectivity

### Fees

- Axelar network fee (paid in AXL)
- Gas costs on source and destination
- No additional protocol fee on Dina side

### Security Model

- **Trust anchor:** Axelar validator network (PoS with slashing)
- **Gateway verification:** Only the gateway can call execute functions
- **Command ID tracking:** Unique per-command replay protection
- **Trusted sources:** Explicit per-chain source address verification
- **Token registry:** Only registered tokens can be received

---

## Circle Upgrade Path: Bridged USDC to Native USDC

The Bridged USDC token (`bridge-usdc`) follows Circle's official standard for eventual upgrade to native USDC.

### How the Upgrade Works

```
Phase 1: Bridged USDC (Current)
+------------------+     +------------------+
| Dina Team owns   |     | Bridge contract  |
| USDC.e contract  |---->| can mint/burn    |
| (owner)          |     | USDC.e           |
+------------------+     +------------------+

Phase 2: Ownership Transfer (When Circle approves)
+------------------+     +------------------+
| Circle becomes   |     | Circle sets      |
| new owner of     |---->| their own minter |
| USDC.e contract  |     | contract         |
+------------------+     +------------------+

Phase 3: Native USDC
+------------------+     +------------------+
| Circle controls  |     | Native USDC on   |
| minting/burning  |---->| Dina (no bridge  |
| directly         |     | needed)          |
+------------------+     +------------------+
```

### Steps for Circle Upgrade

1. **Prove the chain:** Demonstrate sufficient volume, security, and demand
2. **Contact Circle:** Apply for native USDC support
3. **Transfer ownership:** Call `transfer_ownership(circle_address)` on USDC.e
4. **Circle sets minter:** Circle calls `set_bridge_address(circle_minter)`
5. **Lock bridge:** Circle calls `lock_bridge_address()` to prevent changes
6. **Symbol change:** USDC.e becomes USDC (same contract, rebranded)

### Requirements for Upgrade

- Consistent bridging volume (>$10M monthly)
- Clean security audit of the token contract
- Chain stability and uptime track record
- DeFi ecosystem adoption on Dina
- Compliance with Circle's regulatory requirements

---

## Fee Comparison

| Bridge | Protocol Fee | Gas (Dina side) | Total Typical Cost (100 USDC) |
|--------|-------------|-----------------|-------------------------------|
| CCTP | 0% | <$0.001 | ~$0.50 (source gas only) |
| Base Bridge | 0.1% | <$0.001 | ~$0.10 + $0.01 gas |
| Wormhole | 0% | <$0.001 | ~$0.50 (source gas only) |
| LayerZero | ~$0.10 msg fee | <$0.001 | ~$0.20 |
| Axelar | ~$0.50 AXL fee | <$0.001 | ~$0.60 |

## Choosing the Right Bridge

### Decision Tree

```
Need USDC specifically?
  |
  +-- Yes --> Is it from Base?
  |             |
  |             +-- Yes --> Base Bridge (fastest, cheapest)
  |             |
  |             +-- No --> Is Circle-grade security needed?
  |                          |
  |                          +-- Yes --> CCTP
  |                          |
  |                          +-- No --> What chain?
  |                                       |
  |                                       +-- Solana --> Wormhole
  |                                       +-- EVM chains --> LayerZero (fastest)
  |                                       +-- Cosmos --> Axelar
  |
  +-- No --> Need GMP (General Message Passing)?
               |
               +-- Yes --> Axelar (GMP + tokens)
               +-- No --> LayerZero (fastest, most configurable)
```

### Quick Recommendations

- **Lucilla app top-ups:** Base Bridge (direct, fast, cheap)
- **Large institutional transfers:** CCTP (Circle-backed security)
- **Multi-chain DeFi:** Wormhole (broadest chain support)
- **Speed-critical:** LayerZero (fastest finality)
- **Cross-chain dApps:** Axelar (GMP for complex workflows)

## Contract Addresses

| Contract | Dina Address | Description |
|----------|-------------|-------------|
| `bridge-usdc` | TBD | Bridged USDC (USDC.e) token |
| `bridge-cctp` | TBD | CCTP MessageTransmitter |
| `bridge-base` | TBD | Base <-> Dina direct bridge |
| `bridge-wormhole` | TBD | Wormhole token bridge |
| `bridge-layerzero` | TBD | LayerZero OFT endpoint |
| `bridge-axelar` | TBD | Axelar ITS + GMP receiver |

## Security Considerations

1. **All bridges share one USDC.e token** -- only the active bridge address can mint/burn
2. **Bridge address is changeable** by owner until locked (for upgrade flexibility)
3. **Blacklist/pause** capabilities match Circle's USDC compliance requirements
4. **Each bridge has independent replay protection** (nonces, command IDs, VAA hashes)
5. **Rate limits and min/max amounts** prevent dust attacks and limit exposure
6. **Owner can pause any bridge** independently in case of exploit
