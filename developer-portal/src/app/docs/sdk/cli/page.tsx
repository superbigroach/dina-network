"use client";

import { CodeBlock, LanguageTabs } from "@/components/code-block";

function H2({ id, children }: { id: string; children: React.ReactNode }) {
  return (
    <h2
      id={id}
      className="mb-4 mt-12 scroll-mt-24 border-b border-slate-800/60 pb-2 text-2xl font-bold tracking-tight"
    >
      {children}
    </h2>
  );
}

function H3({ id, children }: { id: string; children: React.ReactNode }) {
  return (
    <h3
      id={id}
      className="mb-3 mt-8 scroll-mt-24 text-xl font-semibold tracking-tight"
    >
      {children}
    </h3>
  );
}

function Badge({ children }: { children: React.ReactNode }) {
  return (
    <span className="ml-2 rounded-full bg-purple-600/15 px-2 py-0.5 text-xs font-medium text-purple-400">
      {children}
    </span>
  );
}

function CommandCard({
  command,
  description,
  flags,
  example,
  output,
}: {
  command: string;
  description: string;
  flags?: { flag: string; desc: string }[];
  example: string;
  output?: string;
}) {
  return (
    <div className="my-5 overflow-hidden rounded-xl border border-slate-800/60 bg-slate-900/40">
      <div className="border-b border-slate-800/60 px-5 py-3">
        <code className="text-sm font-semibold text-purple-400">{command}</code>
        <p className="mt-1 text-sm text-slate-400">{description}</p>
      </div>

      {flags && flags.length > 0 && (
        <div className="border-b border-slate-800/40 px-5 py-3">
          <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-500">
            Flags
          </p>
          <div className="space-y-1">
            {flags.map((f) => (
              <div key={f.flag} className="flex gap-3 text-sm">
                <code className="shrink-0 text-slate-300">{f.flag}</code>
                <span className="text-slate-500">{f.desc}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="px-5 py-3">
        <p className="mb-1.5 text-xs font-semibold uppercase tracking-wider text-slate-500">
          Example
        </p>
        <pre className="!m-0 !rounded-lg !border-0 !bg-slate-950/60 !p-3 text-sm leading-relaxed">
          <code className="text-slate-300">{example}</code>
        </pre>
      </div>

      {output && (
        <div className="border-t border-slate-800/40 px-5 py-3">
          <p className="mb-1.5 text-xs font-semibold uppercase tracking-wider text-slate-500">
            Output
          </p>
          <pre className="!m-0 !rounded-lg !border-0 !bg-slate-950/60 !p-3 text-sm leading-relaxed">
            <code className="text-green-400/80">{output}</code>
          </pre>
        </div>
      )}
    </div>
  );
}

export default function CliReferencePage() {
  return (
    <>
      {/* Header */}
      <div className="mb-2 flex items-center gap-2 text-sm text-slate-500">
        SDKs
        <span className="text-slate-700">/</span>
        CLI Reference
      </div>
      <h1 className="text-4xl font-extrabold tracking-tight">
        CLI Reference
        <Badge>dina-cli</Badge>
      </h1>
      <p className="mt-4 text-lg leading-relaxed text-slate-400">
        Command-line interface for interacting with the Dina Network. Manage
        wallets, send transactions, deploy contracts, and run validator nodes.
      </p>

      {/* ---- Installation ---- */}
      <H2 id="installation">Installation</H2>

      <LanguageTabs
        tabs={[
          {
            label: "Cargo",
            language: "bash",
            code: "cargo install dina-cli",
          },
          {
            label: "macOS",
            language: "bash",
            code: `curl -fsSL https://get.dina.network/cli | sh

# Or via Homebrew:
brew install dina-network/tap/dina-cli`,
          },
          {
            label: "Linux",
            language: "bash",
            code: `curl -fsSL https://get.dina.network/cli | sh

# Or download the binary directly:
wget https://github.com/superbigroach/dina-network/releases/latest/download/dina-cli-linux-amd64
chmod +x dina-cli-linux-amd64
sudo mv dina-cli-linux-amd64 /usr/local/bin/dina`,
          },
          {
            label: "Windows",
            language: "powershell",
            code: `# Download from GitHub Releases:
# https://github.com/superbigroach/dina-network/releases/latest

# Or via cargo:
cargo install dina-cli`,
          },
        ]}
      />

      <p className="mt-4 text-sm text-slate-400">
        Verify the installation:
      </p>
      <CodeBlock
        language="bash"
        filename="terminal"
        code={`dina --version
# dina-cli 0.1.0 (dina-network)`}
      />

      {/* ---- Configuration ---- */}
      <H2 id="configuration">Configuration</H2>

      <CommandCard
        command="dina config set rpc-url URL"
        description="Set the default RPC endpoint. All subsequent commands will use this URL unless overridden with --rpc."
        flags={[
          { flag: "URL", desc: "The RPC endpoint URL" },
        ]}
        example={`dina config set rpc-url https://rpc.dina.network`}
        output={`Config updated: rpc-url = https://rpc.dina.network
Saved to ~/.dina/config.toml`}
      />

      <CodeBlock
        language="toml"
        filename="~/.dina/config.toml"
        code={`# Dina CLI configuration

rpc-url = "https://rpc.dina.network"
default-wallet = "my-wallet"
output-format = "table"  # table | json | minimal`}
      />

      {/* ---- Wallet Commands ---- */}
      <H2 id="wallet-commands">Wallet Commands</H2>

      <CommandCard
        command="dina wallet create --name NAME"
        description="Create a new encrypted wallet. You will be prompted for a passphrase to encrypt the private key at rest."
        flags={[
          { flag: "--name NAME", desc: "Human-readable wallet name (required)" },
          { flag: "--mnemonic", desc: "Also generate and display a BIP-39 mnemonic" },
        ]}
        example="dina wallet create --name my-wallet"
        output={`Created wallet "my-wallet"
Address: dina1a8f3k2x9p7q4w...
Encrypted key saved to ~/.dina/wallets/my-wallet.key

Mnemonic (write this down):
  abandon abandon abandon abandon abandon abandon
  abandon abandon abandon abandon abandon about`}
      />

      <CommandCard
        command="dina wallet list"
        description="List all wallets stored locally with their addresses and balances."
        flags={[
          { flag: "--json", desc: "Output as JSON" },
        ]}
        example="dina wallet list"
        output={`NAME          ADDRESS                          BALANCE
my-wallet     dina1a8f3k2x9p7q4w...             1,250.00 USDC
validator     dina1v7m2n5b8c3d9f...            50,000.00 USDC
agent-1       dina1x4k9p2w7m6n3r...               100.50 USDC`}
      />

      <CommandCard
        command="dina wallet import --key FILE"
        description="Import a wallet from a private key file. The key will be encrypted with a passphrase you provide."
        flags={[
          { flag: "--key FILE", desc: "Path to the raw private key file" },
          { flag: "--name NAME", desc: "Name for the imported wallet" },
          { flag: "--hex KEY", desc: "Import from a hex-encoded private key string instead of file" },
        ]}
        example="dina wallet import --key ./my-key.pem --name imported-wallet"
        output={`Imported wallet "imported-wallet"
Address: dina1q9w8e7r6t5y4u...
Encrypted key saved to ~/.dina/wallets/imported-wallet.key`}
      />

      <CommandCard
        command="dina wallet export --name NAME"
        description="Export the raw private key for a wallet. You will be prompted for the wallet passphrase."
        flags={[
          { flag: "--name NAME", desc: "Wallet name to export (required)" },
          { flag: "--format hex|pem", desc: "Output format (default: hex)" },
        ]}
        example="dina wallet export --name my-wallet"
        output={`Passphrase: ********
Private key (hex):
  a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2`}
      />

      {/* ---- Balance & Transfer ---- */}
      <H2 id="balance-transfer">Balance &amp; Transfer</H2>

      <CommandCard
        command="dina balance ADDRESS"
        description="Check the USDC balance for any address on the network."
        flags={[
          { flag: "ADDRESS", desc: "The Dina address to query" },
          { flag: "--rpc URL", desc: "Override the default RPC endpoint" },
        ]}
        example="dina balance dina1a8f3k2x9p7q4w..."
        output={`Address: dina1a8f3k2x9p7q4w...
Balance: 1,250.00 USDC
Nonce:   42`}
      />

      <CommandCard
        command="dina transfer --from WALLET --to ADDRESS --amount USDC"
        description="Send USDC from a local wallet to a recipient address. You will be prompted for the wallet passphrase."
        flags={[
          { flag: "--from WALLET", desc: "Name of the sending wallet (required)" },
          { flag: "--to ADDRESS", desc: "Recipient address (required)" },
          { flag: "--amount USDC", desc: "Amount in USDC, e.g. '100.00' (required)" },
          { flag: "--memo TEXT", desc: "Optional memo attached to the transaction" },
          { flag: "--dry-run", desc: "Estimate fees without sending" },
        ]}
        example="dina transfer --from my-wallet --to dina1recipient... --amount 100.00"
        output={`Passphrase: ********

Transaction submitted:
  Hash:   0xa1b2c3d4e5f6...
  From:   dina1a8f3k2x9p7q4w...
  To:     dina1recipient...
  Amount: 100.00 USDC
  Fee:    0.001 USDC

Waiting for confirmation... done (102ms)
Status: CONFIRMED (block #1,234,567)`}
      />

      {/* ---- Transaction & Block ---- */}
      <H2 id="transaction-block">Transaction &amp; Block</H2>

      <CommandCard
        command="dina tx status HASH"
        description="Check the status and details of a transaction by its hash."
        flags={[
          { flag: "HASH", desc: "Transaction hash (required)" },
          { flag: "--json", desc: "Output as JSON" },
          { flag: "--wait", desc: "Wait for confirmation if still pending" },
        ]}
        example="dina tx status 0xa1b2c3d4e5f6..."
        output={`Transaction: 0xa1b2c3d4e5f6...
Status:      CONFIRMED
Block:       #1,234,567
From:        dina1a8f3k2x9p7q4w...
To:          dina1recipient...
Amount:      100.00 USDC
Fee:         0.001 USDC
Gas Used:    21,000
Timestamp:   2026-03-27T14:30:00Z`}
      />

      <CommandCard
        command="dina block NUMBER"
        description="View the details and transactions in a specific block."
        flags={[
          { flag: "NUMBER", desc: "Block number (or 'latest')" },
          { flag: "--json", desc: "Output as JSON" },
          { flag: "--txs", desc: "Show full transaction details (not just hashes)" },
        ]}
        example="dina block latest"
        output={`Block #1,234,567
  Hash:         0xf1e2d3c4b5a6...
  Parent:       0xe9d8c7b6a5f4...
  Timestamp:    2026-03-27T14:30:00Z
  Validator:    dina1validator...
  Transactions: 847
  Gas Used:     12,450,000
  State Root:   0x1a2b3c4d5e6f...`}
      />

      {/* ---- Contract Commands ---- */}
      <H2 id="contract-commands">Contract Commands</H2>

      <CommandCard
        command="dina deploy --from WALLET --wasm FILE"
        description="Deploy a compiled WASM smart contract to the network."
        flags={[
          { flag: "--from WALLET", desc: "Wallet to pay deployment fees (required)" },
          { flag: "--wasm FILE", desc: "Path to the compiled .wasm file (required)" },
          { flag: "--args JSON", desc: "Constructor arguments as JSON array" },
          { flag: "--dry-run", desc: "Estimate deployment cost without deploying" },
        ]}
        example={`dina deploy --from my-wallet --wasm ./target/wasm32-unknown-unknown/release/counter.wasm --args '["My Counter"]'`}
        output={`Passphrase: ********

Deploying contract...
  WASM size:  14.2 KB
  Gas estimate: 850,000

Transaction submitted: 0xdeploy123...
Waiting for confirmation... done (98ms)

Contract deployed:
  Address: dina1contract7x9m2k...
  Block:   #1,234,568
  Fee:     0.085 USDC`}
      />

      <CommandCard
        command="dina call --contract ADDR --method NAME --args JSON"
        description="Call a smart contract method. Read-only calls are free; state-changing calls require a wallet."
        flags={[
          { flag: "--contract ADDR", desc: "Contract address (required)" },
          { flag: "--method NAME", desc: "Method name to call (required)" },
          { flag: "--args JSON", desc: "Method arguments as JSON array" },
          { flag: "--from WALLET", desc: "Wallet for state-changing calls" },
          { flag: "--json", desc: "Output raw JSON response" },
        ]}
        example={`# Read-only call (free)
dina call --contract dina1contract7x9m2k... --method get_count

# State-changing call (requires wallet)
dina call --contract dina1contract7x9m2k... --method increment --from my-wallet`}
        output={`Result: 42`}
      />

      {/* ---- Node Commands ---- */}
      <H2 id="node-commands">Node Commands</H2>

      <CommandCard
        command="dina node --validator --key FILE"
        description="Run a Dina Network node. Add --validator to participate in consensus."
        flags={[
          { flag: "--validator", desc: "Run as a validator node" },
          { flag: "--key FILE", desc: "Path to the validator key file" },
          { flag: "--data-dir DIR", desc: "State storage directory (default: ~/.dina/data)" },
          { flag: "--rpc-port PORT", desc: "JSON-RPC port (default: 8545)" },
          { flag: "--p2p-port PORT", desc: "P2P networking port (default: 26656)" },
          { flag: "--peers ADDR,...", desc: "Comma-separated list of bootstrap peer addresses" },
          { flag: "--log-level LEVEL", desc: "Log level: debug, info, warn, error (default: info)" },
        ]}
        example={`# Run a full node (non-validator)
dina node --data-dir ./dina-data --log-level info

# Run a validator
dina node --validator --key ./validator.key --data-dir ./dina-data`}
        output={`Dina Network Node v0.1.0
  Chain ID:   dina-mainnet-1
  Data dir:   ./dina-data
  RPC:        http://0.0.0.0:8545
  P2P:        0.0.0.0:26656
  Validator:  yes
  Address:    dina1validator7x9m...

Connecting to peers...
  Connected: validator-0.dina.network:26656 (Validator 0)
  Connected: validator-1.dina.network:26656 (Validator 1)

Syncing blocks... 1,234,500 / 1,234,567 (99.99%)
Sync complete. Producing blocks.

Block #1,234,568 produced (847 txs, 98ms)`}
      />

      {/* ---- All Commands Reference ---- */}
      <H2 id="all-commands">Command Summary</H2>

      <div className="overflow-x-auto">
        <table className="mt-4 w-full text-sm">
          <thead>
            <tr className="border-b border-slate-800/60 text-left">
              <th className="py-3 pr-6 font-semibold text-slate-300">Command</th>
              <th className="py-3 font-semibold text-slate-300">Description</th>
            </tr>
          </thead>
          <tbody className="text-slate-400">
            {[
              ["dina wallet create --name NAME", "Create a new encrypted wallet"],
              ["dina wallet list", "List all local wallets with balances"],
              ["dina wallet import --key FILE", "Import wallet from a private key file"],
              ["dina wallet export --name NAME", "Export the private key for a wallet"],
              ["dina balance ADDRESS", "Check USDC balance for an address"],
              ["dina transfer --from WALLET --to ADDR --amount USDC", "Send USDC to a recipient"],
              ["dina tx status HASH", "Check transaction status and details"],
              ["dina block NUMBER", "View block details and transactions"],
              ["dina deploy --from WALLET --wasm FILE", "Deploy a WASM smart contract"],
              ["dina call --contract ADDR --method NAME --args JSON", "Call a contract method"],
              ["dina node --validator --key FILE", "Run a validator node"],
              ["dina config set rpc-url URL", "Set the default RPC endpoint"],
            ].map(([cmd, desc]) => (
              <tr key={cmd} className="border-b border-slate-800/40">
                <td className="py-3 pr-6">
                  <code className="text-purple-400 text-xs">{cmd}</code>
                </td>
                <td className="py-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* ---- Environment Variables ---- */}
      <H2 id="environment-variables">Environment Variables</H2>

      <div className="overflow-x-auto">
        <table className="mt-4 w-full text-sm">
          <thead>
            <tr className="border-b border-slate-800/60 text-left">
              <th className="py-3 pr-6 font-semibold text-slate-300">Variable</th>
              <th className="py-3 pr-6 font-semibold text-slate-300">Default</th>
              <th className="py-3 font-semibold text-slate-300">Description</th>
            </tr>
          </thead>
          <tbody className="text-slate-400">
            {[
              ["DINA_RPC_URL", "https://rpc.dina.network", "Default RPC endpoint"],
              ["DINA_HOME", "~/.dina", "Config and wallet storage directory"],
              ["DINA_LOG", "info", "Log level (debug, info, warn, error)"],
              ["DINA_WALLET", "(none)", "Default wallet name for commands"],
              ["DINA_PASSPHRASE", "(none)", "Wallet passphrase (avoid in production)"],
            ].map(([v, def, desc]) => (
              <tr key={v} className="border-b border-slate-800/40">
                <td className="py-3 pr-6">
                  <code className="text-purple-400">{v}</code>
                </td>
                <td className="py-3 pr-6 text-slate-500">{def}</td>
                <td className="py-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* ---- Shell Completion ---- */}
      <H2 id="shell-completion">Shell Completion</H2>

      <LanguageTabs
        tabs={[
          {
            label: "Bash",
            language: "bash",
            code: `# Add to ~/.bashrc
eval "$(dina completions bash)"`,
          },
          {
            label: "Zsh",
            language: "bash",
            code: `# Add to ~/.zshrc
eval "$(dina completions zsh)"`,
          },
          {
            label: "Fish",
            language: "bash",
            code: `# Add to ~/.config/fish/config.fish
dina completions fish | source`,
          },
          {
            label: "PowerShell",
            language: "powershell",
            code: `# Add to $PROFILE
dina completions powershell | Out-String | Invoke-Expression`,
          },
        ]}
      />
    </>
  );
}
