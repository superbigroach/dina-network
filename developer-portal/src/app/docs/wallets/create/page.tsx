export const metadata = {
  title: "Create a Wallet — Dina Network Developer Portal",
  description:
    "Generate, import, and manage wallets on Dina Network using JavaScript, Python, CLI, or REST API.",
};

/* ------------------------------------------------------------------ */
/*  Reusable code block component                                      */
/* ------------------------------------------------------------------ */
function CodeBlock({
  language,
  title,
  children,
}: {
  language: string;
  title?: string;
  children: string;
}) {
  return (
    <div className="rounded-xl border border-slate-800 overflow-hidden">
      {title && (
        <div className="flex items-center justify-between border-b border-slate-800 bg-slate-900/80 px-4 py-2">
          <span className="text-xs font-medium text-slate-400">{title}</span>
          <span className="rounded bg-slate-800 px-2 py-0.5 text-[10px] font-mono text-slate-500 uppercase">
            {language}
          </span>
        </div>
      )}
      <pre className="bg-slate-900/50 p-4 overflow-x-auto text-sm leading-relaxed">
        <code className="text-slate-300">{children}</code>
      </pre>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Tab-like section for each language                                 */
/* ------------------------------------------------------------------ */
function LangSection({
  id,
  label,
  color,
  children,
}: {
  id: string;
  label: string;
  color: string;
  children: React.ReactNode;
}) {
  return (
    <div id={id} className="scroll-mt-24">
      <div className="flex items-center gap-3 mb-4">
        <h3 className="text-xl font-semibold">{label}</h3>
        <span
          className={`rounded-full px-2.5 py-0.5 text-xs font-medium ${color}`}
        >
          {id.toUpperCase()}
        </span>
      </div>
      <div className="space-y-4">{children}</div>
    </div>
  );
}

export default function CreateWalletPage() {
  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Wallets
      </p>
      <h1 className="text-4xl font-bold tracking-tight mb-4">
        Create a Wallet
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-12">
        Generate a new Dina Network wallet or import an existing one using a
        mnemonic phrase or raw private key. Examples in JavaScript, Python, the
        CLI, and the REST API.
      </p>

      {/* Address format */}
      <div className="rounded-xl border border-blue-500/30 bg-blue-500/5 p-6 mb-12">
        <h3 className="text-sm font-semibold text-blue-400 mb-2">
          Address Format
        </h3>
        <p className="text-sm text-slate-300 leading-relaxed mb-3">
          A Dina address is{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            0x + SHA-256(Ed25519_public_key)
          </code>
          , producing a <strong>66-character</strong> hex string (0x + 64 hex chars).
        </p>
        <CodeBlock language="text" title="Example address">
          {`0x7a3b1f4e8c9d2a5b6f0e1d3c4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b`}
        </CodeBlock>
      </div>

      {/* Jump links */}
      <div className="flex flex-wrap gap-2 mb-12">
        {["javascript", "python", "cli", "rest"].map((lang) => (
          <a
            key={lang}
            href={`#${lang}`}
            className="rounded-lg border border-slate-800 bg-slate-900/50 px-4 py-2 text-sm font-medium text-slate-300 transition-colors hover:border-blue-500/40 hover:text-blue-400"
          >
            {lang === "cli"
              ? "CLI"
              : lang === "rest"
                ? "REST API"
                : lang.charAt(0).toUpperCase() + lang.slice(1)}
          </a>
        ))}
      </div>

      <div className="space-y-16">
        {/* ---- JavaScript ---- */}
        <LangSection
          id="javascript"
          label="JavaScript / TypeScript"
          color="bg-yellow-500/20 text-yellow-400"
        >
          <p className="text-sm text-slate-400 leading-relaxed">
            Install the SDK:
          </p>
          <CodeBlock language="bash" title="Install">
            {`npm install @dina-network/sdk`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Generate a new wallet">
            {`import { DinaWallet } from "@dina-network/sdk";

// Generate a brand-new wallet with a random key pair
const wallet = DinaWallet.generate();

console.log("Address:    ", wallet.address);
console.log("Public key: ", wallet.publicKey);
console.log("Mnemonic:   ", wallet.mnemonic);   // 12-word BIP-39 phrase
// IMPORTANT: Store the mnemonic securely — it is the ONLY way to recover this wallet.`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Import from mnemonic">
            {`import { DinaWallet } from "@dina-network/sdk";

const mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

const wallet = DinaWallet.fromMnemonic(mnemonic);
console.log("Address:", wallet.address);
// Deterministic — the same mnemonic always produces the same key pair.`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Import from private key">
            {`import { DinaWallet } from "@dina-network/sdk";

const privateKeyHex = "3a5b9c..."; // 64-char hex Ed25519 private key

const wallet = DinaWallet.fromPrivateKey(privateKeyHex);
console.log("Address:", wallet.address);`}
          </CodeBlock>
        </LangSection>

        {/* ---- Python ---- */}
        <LangSection
          id="python"
          label="Python"
          color="bg-green-500/20 text-green-400"
        >
          <p className="text-sm text-slate-400 leading-relaxed">
            Install the SDK:
          </p>
          <CodeBlock language="bash" title="Install">
            {`pip install dina-network`}
          </CodeBlock>

          <CodeBlock language="python" title="Generate a new wallet">
            {`from dina_network import DinaWallet

# Generate a brand-new wallet
wallet = DinaWallet.generate()

print(f"Address:    {wallet.address}")
print(f"Public key: {wallet.public_key}")
print(f"Mnemonic:   {wallet.mnemonic}")
# Store the mnemonic securely — it is the ONLY way to recover this wallet.`}
          </CodeBlock>

          <CodeBlock language="python" title="Import from mnemonic">
            {`from dina_network import DinaWallet

mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

wallet = DinaWallet.from_mnemonic(mnemonic)
print(f"Address: {wallet.address}")`}
          </CodeBlock>

          <CodeBlock language="python" title="Import from private key">
            {`from dina_network import DinaWallet

private_key = "3a5b9c..."  # 64-char hex Ed25519 private key

wallet = DinaWallet.from_private_key(private_key)
print(f"Address: {wallet.address}")`}
          </CodeBlock>
        </LangSection>

        {/* ---- CLI ---- */}
        <LangSection
          id="cli"
          label="Dina CLI"
          color="bg-cyan-500/20 text-cyan-400"
        >
          <p className="text-sm text-slate-400 leading-relaxed">
            Install the CLI:
          </p>
          <CodeBlock language="bash" title="Install">
            {`# macOS / Linux
curl -sSf https://install.dina.network | sh

# Or via npm
npm install -g @dina-network/cli`}
          </CodeBlock>

          <CodeBlock language="bash" title="Generate a new wallet">
            {`# Create a named wallet — keys are stored in ~/.dina/wallets/
dina wallet create --name my-wallet

# Output:
#   Address:    0x7a3b1f4e8c9d...
#   Mnemonic:   abandon abandon ...
#   Saved to:   ~/.dina/wallets/my-wallet.json`}
          </CodeBlock>

          <CodeBlock language="bash" title="Import from mnemonic">
            {`dina wallet import --name recovered-wallet \\
  --mnemonic "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"`}
          </CodeBlock>

          <CodeBlock language="bash" title="List wallets">
            {`dina wallet list

# NAME              ADDRESS                                                            BALANCE
# my-wallet         0x7a3b1f4e8c9d2a5b6f0e1d3c4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b   100.00 DINA
# recovered-wallet  0x1234abcd...                                                       0.00 DINA`}
          </CodeBlock>
        </LangSection>

        {/* ---- REST API ---- */}
        <LangSection
          id="rest"
          label="REST API"
          color="bg-orange-500/20 text-orange-400"
        >
          <p className="text-sm text-slate-400 leading-relaxed">
            For server-side integrations. Requires an API key from the{" "}
            <a href="/dashboard" className="text-blue-400 hover:underline">
              Dina Dashboard
            </a>
            .
          </p>

          <CodeBlock language="bash" title="Create a wallet via REST">
            {`curl -X POST https://api.dina.network/v1/wallets \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "name": "payment-wallet",
    "type": "standard"
  }'

# Response:
# {
#   "address": "0x7a3b1f4e8c9d2a5b...",
#   "publicKey": "ed25519:abcdef1234...",
#   "type": "standard",
#   "createdAt": "2026-03-27T12:00:00Z"
# }
#
# NOTE: The private key is returned ONCE in the response.
# Store it securely — Dina Network does NOT retain it.`}
          </CodeBlock>

          <CodeBlock language="bash" title="Get wallet balance">
            {`curl https://api.dina.network/v1/wallets/0x7a3b.../balance \\
  -H "Authorization: Bearer YOUR_API_KEY"

# { "dina": "1000.00", "usdc": "250.00" }`}
          </CodeBlock>
        </LangSection>
      </div>

      {/* Security best practices */}
      <div className="mt-16">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Security Best Practices
        </h2>
        <div className="space-y-4">
          {[
            {
              title: "Never log or print private keys in production",
              desc: "The examples above use console.log for illustration. In production, write keys directly to encrypted storage.",
            },
            {
              title: "Store mnemonics offline",
              desc: "Write the 12- or 24-word phrase on paper and store it in a secure physical location. Do not store plain-text mnemonics on internet-connected devices.",
            },
            {
              title: "Encrypt keys at rest",
              desc: "Use AES-256-GCM to encrypt private keys before writing them to disk. See the Key Management guide for details.",
            },
            {
              title: "Use environment variables for API keys",
              desc: "Never hardcode API keys in source code. Use environment variables or a secrets manager like Google Cloud Secret Manager or AWS Secrets Manager.",
            },
            {
              title: "Rotate keys periodically",
              desc: "Create a new wallet, transfer assets, and decommission the old key. Dina wallets are free to create — there is no reason to reuse a compromised key.",
            },
          ].map((item) => (
            <div
              key={item.title}
              className="rounded-xl border border-slate-800 bg-slate-900/50 p-5"
            >
              <h3 className="text-sm font-semibold text-slate-200 mb-1">
                {item.title}
              </h3>
              <p className="text-sm text-slate-400 leading-relaxed">
                {item.desc}
              </p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
