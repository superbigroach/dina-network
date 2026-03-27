import Link from "next/link";

export const metadata = {
  title: "HD Wallets & Mnemonics — Dina Network Developer Portal",
  description:
    "BIP-39 mnemonic phrases, HD derivation paths, and deterministic wallet generation on Dina Network.",
};

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

export default function HDWalletsPage() {
  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Wallets
      </p>
      <h1 className="text-4xl font-bold tracking-tight mb-4">
        HD Wallets &amp; Mnemonics
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-12">
        Dina Network supports BIP-39 mnemonic phrases for deterministic wallet
        generation. A single 12- or 24-word phrase can derive an unlimited
        number of wallets, making backup and recovery simple.
      </p>

      {/* What is a mnemonic */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          What is a BIP-39 Mnemonic?
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          A BIP-39 mnemonic is a human-readable representation of cryptographic
          entropy. Instead of backing up a 64-character hex string, you back up
          12 or 24 English words. The mnemonic deterministically generates a
          seed, from which all key pairs are derived.
        </p>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6 mb-6">
          <pre className="text-sm text-slate-300 leading-relaxed overflow-x-auto font-mono">
            {`  Entropy (128 bits)
       |
       v
  BIP-39 Encoding
       |
       v
  12-word mnemonic phrase
  "abandon abandon abandon abandon abandon
   abandon abandon abandon abandon abandon
   abandon about"
       |
       v
  PBKDF2 (2048 rounds, optional passphrase)
       |
       v
  512-bit Seed
       |
       v
  HD Derivation Tree
       |
  +----+----+----+----+
  |    |    |    |    |
  m/0  m/1  m/2  m/3  ...    Unlimited child wallets`}
          </pre>
        </div>
      </div>

      {/* 12 vs 24 words */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          12-Word vs 24-Word Phrases
        </h2>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Property
                </th>
                <th className="px-4 py-3 text-left font-medium text-blue-400">
                  12 Words
                </th>
                <th className="px-4 py-3 text-left font-medium text-purple-400">
                  24 Words
                </th>
              </tr>
            </thead>
            <tbody>
              {[
                {
                  prop: "Entropy",
                  twelve: "128 bits",
                  twentyfour: "256 bits",
                },
                {
                  prop: "Security level",
                  twelve: "2^128 combinations",
                  twentyfour: "2^256 combinations",
                },
                {
                  prop: "Brute force time",
                  twelve: "Billions of years",
                  twentyfour: "Heat death of universe",
                },
                {
                  prop: "Ease of backup",
                  twelve: "Easier to write down",
                  twentyfour: "More words to manage",
                },
                {
                  prop: "Recommendation",
                  twelve: "Personal wallets",
                  twentyfour: "High-value / institutional",
                },
              ].map((row, i) => (
                <tr
                  key={row.prop}
                  className={
                    i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"
                  }
                >
                  <td className="px-4 py-3 font-medium text-slate-300">
                    {row.prop}
                  </td>
                  <td className="px-4 py-3 text-slate-400">{row.twelve}</td>
                  <td className="px-4 py-3 text-slate-400">
                    {row.twentyfour}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="text-xs text-slate-500 mt-3">
          Both 12-word and 24-word phrases are considered cryptographically
          secure. 12 words is sufficient for most use cases.
        </p>
      </div>

      {/* Code examples */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Generate and Restore Wallets
        </h2>
        <div className="space-y-4">
          <CodeBlock language="typescript" title="Generate a wallet with 12-word mnemonic">
            {`import { DinaWallet } from "@dina-network/sdk";

// Generate a new wallet — mnemonic is created automatically
const wallet = DinaWallet.generate();

console.log("Mnemonic:", wallet.mnemonic);
// "abandon abandon abandon abandon abandon abandon
//  abandon abandon abandon abandon abandon about"

console.log("Address:", wallet.address);
// "0x7a3b1f4e8c9d2a5b6f0e1d3c4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b"

// IMPORTANT: Write down the mnemonic and store it securely.
// It is the ONLY way to recover this wallet.`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Generate with 24-word mnemonic">
            {`import { DinaWallet } from "@dina-network/sdk";

// Specify 256-bit entropy for a 24-word phrase
const wallet = DinaWallet.generate({ strength: 256 });

console.log("Mnemonic:", wallet.mnemonic);
// 24 words
console.log("Word count:", wallet.mnemonic.split(" ").length); // 24`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Restore from mnemonic">
            {`import { DinaWallet } from "@dina-network/sdk";

const mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

// Deterministic — always produces the same wallet
const wallet = DinaWallet.fromMnemonic(mnemonic);
console.log("Address:", wallet.address);

// Optional: use a passphrase for additional security (BIP-39 passphrase)
const walletWithPassphrase = DinaWallet.fromMnemonic(mnemonic, {
  passphrase: "my-secret-passphrase",
});
// Different passphrase = different wallet (same mnemonic)`}
          </CodeBlock>

          <CodeBlock language="python" title="Restore from mnemonic (Python)">
            {`from dina_network import DinaWallet

mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

wallet = DinaWallet.from_mnemonic(mnemonic)
print(f"Address: {wallet.address}")

# With passphrase
wallet_secure = DinaWallet.from_mnemonic(mnemonic, passphrase="my-secret")
print(f"Address (with passphrase): {wallet_secure.address}")`}
          </CodeBlock>

          <CodeBlock language="bash" title="Restore from mnemonic (CLI)">
            {`dina wallet import --name recovered \\
  --mnemonic "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

# With passphrase
dina wallet import --name recovered-secure \\
  --mnemonic "abandon abandon ..." \\
  --passphrase "my-secret-passphrase"`}
          </CodeBlock>
        </div>
      </div>

      {/* Derivation Paths */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Derivation Paths
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          HD (Hierarchical Deterministic) wallets use a tree structure to derive
          child keys from a master seed. Each level in the tree is separated by
          a <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">/</code>.
          Dina Network follows a SLIP-0010 compatible derivation scheme for Ed25519.
        </p>

        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6 mb-6">
          <h3 className="text-sm font-semibold text-slate-200 mb-3">
            Default Derivation Path
          </h3>
          <pre className="text-sm text-slate-300 leading-relaxed font-mono">
            {`m / purpose' / coin_type' / account' / change / index

m / 44' / 8108' / 0' / 0 / 0     ← Default (first wallet)
m / 44' / 8108' / 0' / 0 / 1     ← Second wallet
m / 44' / 8108' / 0' / 0 / 2     ← Third wallet
m / 44' / 8108' / 1' / 0 / 0     ← Second account, first wallet

Where:
  44'    = BIP-44 standard
  8108'  = Dina Network coin type (registered)
  0'     = Account index
  0      = External chain (0) vs internal/change (1)
  0      = Address index`}
          </pre>
        </div>

        <CodeBlock language="typescript" title="Derive multiple wallets from one mnemonic">
          {`import { DinaWallet } from "@dina-network/sdk";

const mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

// Derive wallets at different indices
const wallet0 = DinaWallet.fromMnemonic(mnemonic, { index: 0 });
const wallet1 = DinaWallet.fromMnemonic(mnemonic, { index: 1 });
const wallet2 = DinaWallet.fromMnemonic(mnemonic, { index: 2 });

console.log("Wallet 0:", wallet0.address); // m/44'/8108'/0'/0/0
console.log("Wallet 1:", wallet1.address); // m/44'/8108'/0'/0/1
console.log("Wallet 2:", wallet2.address); // m/44'/8108'/0'/0/2

// All three are different wallets, all recoverable from the same mnemonic.`}
        </CodeBlock>

        <CodeBlock language="typescript" title="Custom derivation path">
          {`import { DinaWallet } from "@dina-network/sdk";

const mnemonic = "abandon abandon abandon ...";

// Use a custom derivation path
const wallet = DinaWallet.fromMnemonic(mnemonic, {
  path: "m/44'/8108'/5'/0/0", // Account 5, first address
});

console.log("Custom path wallet:", wallet.address);`}
        </CodeBlock>
      </div>

      {/* Multiple accounts pattern */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Multi-Account Pattern
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          A common pattern is to use one mnemonic to derive multiple accounts
          for different purposes: a hot wallet, a savings wallet, a testing
          wallet, and so on.
        </p>
        <CodeBlock language="typescript" title="Multi-account setup">
          {`import { DinaWallet } from "@dina-network/sdk";

const mnemonic = process.env.MASTER_MNEMONIC!;

// Derive purpose-specific wallets
const hotWallet = DinaWallet.fromMnemonic(mnemonic, {
  account: 0,  // m/44'/8108'/0'/0/0
});

const savingsWallet = DinaWallet.fromMnemonic(mnemonic, {
  account: 1,  // m/44'/8108'/1'/0/0
});

const testWallet = DinaWallet.fromMnemonic(mnemonic, {
  account: 2,  // m/44'/8108'/2'/0/0
});

console.log("Hot wallet:    ", hotWallet.address);
console.log("Savings wallet:", savingsWallet.address);
console.log("Test wallet:   ", testWallet.address);

// All recoverable from the same mnemonic phrase.
// No need to back up multiple private keys.`}
        </CodeBlock>
      </div>

      {/* Security warnings */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Mnemonic Security
        </h2>
        <div className="space-y-4">
          {[
            {
              title: "Write it down on paper",
              desc: "Do not store mnemonics digitally (screenshots, notes apps, cloud storage). Write the words on paper or stamp them on metal for fire/water resistance.",
            },
            {
              title: "Never share your mnemonic",
              desc: "Anyone with your mnemonic can derive all your wallets and drain all funds. No legitimate service will ever ask for your mnemonic phrase.",
            },
            {
              title: "Use a passphrase for high-value wallets",
              desc: "BIP-39 supports an optional passphrase that acts as a 13th (or 25th) word. Even if someone finds your mnemonic, they cannot access your wallet without the passphrase.",
            },
            {
              title: "Test recovery before funding",
              desc: "After generating a new wallet, immediately test recovery by restoring from the mnemonic and verifying the address matches. Only then send funds to it.",
            },
            {
              title: "Consider splitting the mnemonic",
              desc: "For very high-value wallets, split the 12 words into 2-3 parts stored in separate physical locations. No single location reveals the full phrase.",
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

      {/* Wordlist */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          BIP-39 Wordlist
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Dina Network uses the standard BIP-39 English wordlist containing
          exactly <strong className="text-slate-200">2,048 words</strong>. Each
          word maps to 11 bits of entropy. The last word includes a checksum to
          detect transcription errors.
        </p>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
          <p className="text-sm text-slate-300 leading-relaxed">
            The wordlist is intentionally designed so that the first 4 characters
            of each word are unique. This means you only need to write the first
            4 letters of each word for an unambiguous backup:{" "}
            <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
              aban aban aban ... abou
            </code>
          </p>
        </div>
      </div>

      {/* Navigation */}
      <div className="flex items-center justify-between pt-8 border-t border-slate-800">
        <Link
          href="/docs/wallets/keys"
          className="text-sm text-slate-400 hover:text-blue-400 transition-colors"
        >
          &larr; Key Management
        </Link>
        <Link
          href="/docs/wallets"
          className="text-sm text-slate-400 hover:text-blue-400 transition-colors"
        >
          Wallets Overview &rarr;
        </Link>
      </div>
    </div>
  );
}
