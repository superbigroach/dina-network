# Dina Network Smart Contract Starter

A minimal but complete smart contract template for the Dina Network. Implements a counter with greetings to demonstrate state management, access control, and the dispatch pattern.

## Quick Start

1. Clone this template:

```bash
cp -r templates/contract-starter my-contract
cd my-contract
```

2. Edit `src/lib.rs` -- add your contract logic

3. Run tests:

```bash
cargo test
```

4. Build for deployment:

```bash
cargo build --target wasm32-unknown-unknown --release
```

5. (Optional) Optimize binary size:

```bash
wasm-opt -Oz target/wasm32-unknown-unknown/release/my_dina_contract.wasm -o optimized.wasm
```

6. Deploy to testnet:

```bash
dina deploy --wasm target/wasm32-unknown-unknown/release/my_dina_contract.wasm --network testnet
```

7. Interact via CLI:

```bash
dina call --contract 0x... --method increment --network testnet
dina call --contract 0x... --method get_counter --network testnet
```

8. Interact via SDK:

```typescript
import { DinaClient, Wallet } from '@dina-network/sdk';

const client = new DinaClient('http://35.184.213.248:8545');
const wallet = Wallet.fromMnemonic('your mnemonic...');

// Increment
await client.callContract(wallet, {
  contract: '0x...',
  method: 'increment',
  args: {},
});

// Read counter (free, no gas)
const counter = await client.viewContract({
  contract: '0x...',
  method: 'get_counter',
  args: {},
});
console.log('Counter:', counter);
```

## Project Structure

```
my-dina-contract/
  Cargo.toml       -- Dependencies and release profile
  src/
    lib.rs         -- Contract logic, dispatch, and tests
```

## What This Template Demonstrates

- **State struct**: `MyContract` holds all on-chain data (owner, counter, greetings)
- **Mutation methods**: `increment()`, `set_greeting()`, `reset()` modify state
- **View methods**: `get_counter()`, `get_greeting()` read state without gas cost
- **Access control**: `reset()` checks `caller == owner` before executing
- **Dispatch function**: Routes method calls from the Dina runtime to your logic
- **Unit tests**: Full lifecycle test and access control test with `cargo test`

## Next Steps

- Read the [Smart Contract Developer Guide](https://docs.dina.network/docs/contracts/guide)
- Browse [DRC Standards](https://docs.dina.network/docs/contracts/standards) for token, NFT, and DeFi patterns
- See [WASM Runtime](https://docs.dina.network/docs/contracts/wasm) for host functions and gas costs
