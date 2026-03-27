export default function ArchitecturePage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Architecture
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Dina Network is a Layer 1 blockchain designed from scratch for
        high-throughput, low-latency transaction processing. It combines
        TurboBFT consensus, parallel execution lanes, and a WASM smart
        contract runtime into a single cohesive stack.
      </p>

      {/* Architecture diagram */}
      <h2 className="mt-12 text-2xl font-semibold text-white">
        System overview
      </h2>
      <div className="mt-6 overflow-x-auto rounded-lg border border-slate-800 bg-slate-800 p-6">
        <pre className="!m-0 !border-0 !bg-transparent !p-0">
          <code className="text-xs leading-relaxed text-slate-200 sm:text-sm">
{`                        +---------------------+
                        |    Client / SDK     |
                        |  (dina-js, Python)  |
                        +----------+----------+
                                   |
                          JSON-RPC / REST / WS
                                   |
                        +----------v----------+
                        |    Validator Node    |
                        |   (Ed25519 signer)  |
                        +----------+----------+
                                   |
                  +----------------+----------------+
                  |                                 |
        +---------v---------+           +----------v----------+
        |   TurboBFT        |           |   Mempool           |
        |   Consensus       |           |   (tx ordering)     |
        |  (3-7 validators) |           +----------+----------+
        +---------+---------+                      |
                  |                                |
                  +----------------+---------------+
                                   |
                        +----------v----------+
                        |  Parallel Executor  |
                        +----------+----------+
                        |  Lane 0  |  Lane 1  |
                        |  Lane 2  |  Lane N  |
                        +----------+----------+
                                   |
                        +----------v----------+
                        |   State Database    |
                        |  (accounts, WASM    |
                        |   contract storage) |
                        +---------------------+`}
          </code>
        </pre>
      </div>

      {/* Core components */}
      <h2 className="mt-14 text-2xl font-semibold text-white">
        Core components
      </h2>

      <div className="mt-6 space-y-6">
        {/* Ed25519 Keys */}
        <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
          <h3 className="text-base font-semibold text-blue-400">
            Ed25519 cryptography
          </h3>
          <p className="mt-2 text-sm leading-relaxed text-slate-300">
            All addresses, validator identities, and transaction signatures
            use Ed25519 elliptic-curve keys. Ed25519 provides fast signing
            (~62,000 signatures/second on commodity hardware), compact 64-byte
            signatures, and resistance to timing side-channel attacks. Wallet
            addresses are derived by hashing the 32-byte public key with
            BLAKE2b and encoding with the{" "}
            <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
              dina1
            </code>{" "}
            prefix.
          </p>
        </div>

        {/* USDC-Native */}
        <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
          <h3 className="text-base font-semibold text-blue-400">
            USDC-native fees
          </h3>
          <p className="mt-2 text-sm leading-relaxed text-slate-300">
            Unlike chains with volatile gas tokens, Dina denominates all
            balances and fees in USDC with 6-decimal precision. A simple
            transfer costs approximately 0.000100 USDC. This removes the need
            to hold a separate token for gas, simplifying the developer and
            user experience. Fee revenue is distributed to validators
            proportionally each epoch.
          </p>
        </div>

        {/* TurboBFT */}
        <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
          <h3 className="text-base font-semibold text-blue-400">
            TurboBFT consensus
          </h3>
          <p className="mt-2 text-sm leading-relaxed text-slate-300">
            TurboBFT is a pipelined BFT protocol that achieves single-slot
            finality in ~100ms. The protocol tolerates up to{" "}
            <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
              f = (n-1)/3
            </code>{" "}
            Byzantine validators. With the current 3-validator testnet, every
            block is final once 2 of 3 validators attest. Block production
            rotates in round-robin order, and the pipeline allows the next
            proposer to begin building while the current block is still being
            attested.
          </p>
        </div>

        {/* Parallel Execution */}
        <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
          <h3 className="text-base font-semibold text-blue-400">
            Parallel execution lanes
          </h3>
          <p className="mt-2 text-sm leading-relaxed text-slate-300">
            Transactions are assigned to independent execution lanes based on
            their read/write sets. Transactions that touch disjoint accounts
            or contracts execute simultaneously across multiple CPU cores.
            Conflict detection happens at the account level: if two
            transactions write to the same account, they are placed in the
            same lane and executed sequentially. This approach scales linearly
            with core count, enabling 100,000+ TPS on modern hardware.
          </p>
        </div>

        {/* WASM Runtime */}
        <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
          <h3 className="text-base font-semibold text-blue-400">
            WASM smart contracts
          </h3>
          <p className="mt-2 text-sm leading-relaxed text-slate-300">
            Smart contracts compile to WebAssembly and run inside a sandboxed
            WASM runtime with deterministic execution. Contracts can be
            written in Rust, AssemblyScript, or any language that targets
            WASM. The runtime enforces metered gas and memory limits per call.
            Dina provides 82 DRC (Dina Request for Comment) standards that
            cover fungible tokens, NFTs, DeFi primitives, identity,
            governance, IoT device attestation, and AI agent interactions.
          </p>
        </div>
      </div>

      {/* Comparison table */}
      <h2 className="mt-14 text-2xl font-semibold text-white">
        How Dina compares
      </h2>
      <p className="mt-3 text-sm text-slate-400">
        Key differences between Dina and other Layer 1 chains.
      </p>

      <div className="mt-6 overflow-x-auto rounded-xl border border-slate-800">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-800 bg-slate-900/60">
              <th className="px-4 py-3 text-left font-semibold text-slate-300">
                Feature
              </th>
              <th className="px-4 py-3 text-left font-semibold text-blue-400">
                Dina
              </th>
              <th className="px-4 py-3 text-left font-semibold text-slate-300">
                Ethereum
              </th>
              <th className="px-4 py-3 text-left font-semibold text-slate-300">
                Solana
              </th>
              <th className="px-4 py-3 text-left font-semibold text-slate-300">
                Sui
              </th>
            </tr>
          </thead>
          <tbody className="text-slate-300">
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Finality</td>
              <td className="px-4 py-3 text-blue-400">~100ms</td>
              <td className="px-4 py-3">~12 min</td>
              <td className="px-4 py-3">~400ms</td>
              <td className="px-4 py-3">~500ms</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Throughput</td>
              <td className="px-4 py-3 text-blue-400">100k+ TPS</td>
              <td className="px-4 py-3">~30 TPS</td>
              <td className="px-4 py-3">~65k TPS</td>
              <td className="px-4 py-3">~120k TPS</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Gas token</td>
              <td className="px-4 py-3 text-blue-400">USDC (stable)</td>
              <td className="px-4 py-3">ETH (volatile)</td>
              <td className="px-4 py-3">SOL (volatile)</td>
              <td className="px-4 py-3">SUI (volatile)</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Execution</td>
              <td className="px-4 py-3 text-blue-400">Parallel lanes</td>
              <td className="px-4 py-3">Sequential</td>
              <td className="px-4 py-3">Parallel (Sealevel)</td>
              <td className="px-4 py-3">Parallel (object model)</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">VM</td>
              <td className="px-4 py-3 text-blue-400">WASM</td>
              <td className="px-4 py-3">EVM</td>
              <td className="px-4 py-3">SBF (eBPF)</td>
              <td className="px-4 py-3">Move VM</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Consensus</td>
              <td className="px-4 py-3 text-blue-400">TurboBFT</td>
              <td className="px-4 py-3">Gasper (PoS)</td>
              <td className="px-4 py-3">Tower BFT</td>
              <td className="px-4 py-3">Narwhal/Bullshark</td>
            </tr>
            <tr>
              <td className="px-4 py-3 font-medium text-slate-200">AI agent support</td>
              <td className="px-4 py-3 text-blue-400">Native (DRC-63, DRC-101)</td>
              <td className="px-4 py-3">None</td>
              <td className="px-4 py-3">None</td>
              <td className="px-4 py-3">Limited</td>
            </tr>
          </tbody>
        </table>
      </div>

      {/* Block structure */}
      <h2 className="mt-14 text-2xl font-semibold text-white">
        Block structure
      </h2>
      <p className="mt-3 text-sm text-slate-300">
        Each block contains a header with the following fields:
      </p>
      <div className="mt-4 overflow-hidden rounded-lg border border-slate-800">
        <pre className="!m-0 !rounded-none bg-slate-800 p-4">
          <code className="text-sm leading-relaxed text-slate-200">
{`{
  "number": 14523,
  "hash": "0xa1b2c3...f4e5",
  "parentHash": "0xd6e7f8...a9b0",
  "timestamp": 1711497600100,
  "proposer": "dina1val0qxy2kgdygjrs...",
  "txCount": 847,
  "stateRoot": "0x1234ab...cdef",
  "txRoot": "0x5678ef...0123",
  "attestations": [
    { "validator": "dina1val0...", "signature": "0x..." },
    { "validator": "dina1val1...", "signature": "0x..." }
  ]
}`}
          </code>
        </pre>
      </div>

      {/* Network parameters */}
      <h2 className="mt-14 text-2xl font-semibold text-white">
        Network parameters
      </h2>
      <div className="mt-6 overflow-x-auto rounded-xl border border-slate-800">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-800 bg-slate-900/60">
              <th className="px-4 py-3 text-left font-semibold text-slate-300">
                Parameter
              </th>
              <th className="px-4 py-3 text-left font-semibold text-slate-300">
                Value
              </th>
            </tr>
          </thead>
          <tbody className="text-slate-300">
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Block time</td>
              <td className="px-4 py-3">100ms</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Max transactions per block</td>
              <td className="px-4 py-3">10,000</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Validators (testnet)</td>
              <td className="px-4 py-3">3 (supports 3-7)</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Byzantine fault tolerance</td>
              <td className="px-4 py-3">f = (n-1)/3</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Native currency</td>
              <td className="px-4 py-3">USDC (6 decimals)</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">Signature scheme</td>
              <td className="px-4 py-3">Ed25519</td>
            </tr>
            <tr>
              <td className="px-4 py-3 font-medium text-slate-200">Smart contract VM</td>
              <td className="px-4 py-3">WASM (WebAssembly)</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}
