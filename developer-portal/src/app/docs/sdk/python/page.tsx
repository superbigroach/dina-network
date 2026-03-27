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
    <span className="ml-2 rounded-full bg-green-600/15 px-2 py-0.5 text-xs font-medium text-green-400">
      {children}
    </span>
  );
}

function MethodSignature({ sig, returns, description }: { sig: string; returns: string; description: string }) {
  return (
    <div className="my-3 rounded-lg border border-slate-800/60 bg-slate-900/40 p-4">
      <code className="text-sm text-green-400">{sig}</code>
      <span className="ml-2 text-sm text-slate-500">-&gt; {returns}</span>
      <p className="mt-1.5 text-sm text-slate-400">{description}</p>
    </div>
  );
}

export default function PythonSdkPage() {
  return (
    <>
      {/* Header */}
      <div className="mb-2 flex items-center gap-2 text-sm text-slate-500">
        SDKs
        <span className="text-slate-700">/</span>
        Python
      </div>
      <h1 className="text-4xl font-extrabold tracking-tight">
        Python SDK
        <Badge>dina-py</Badge>
      </h1>
      <p className="mt-4 text-lg leading-relaxed text-slate-400">
        The official Python SDK for the Dina Network. Supports Python 3.9+ with
        full async/await support via asyncio.
      </p>

      {/* ---- Installation ---- */}
      <H2 id="installation">Installation</H2>

      <LanguageTabs
        tabs={[
          { label: "pip", language: "bash", code: "pip install dina-py" },
          { label: "poetry", language: "bash", code: "poetry add dina-py" },
          { label: "pipx (CLI)", language: "bash", code: "pipx install dina-py" },
        ]}
      />

      {/* ---- Quick Start ---- */}
      <H2 id="quick-start">Quick Start</H2>

      <CodeBlock
        language="python"
        filename="quickstart.py"
        code={`from dina import DinaWallet, DinaClient

# Generate a new wallet
wallet = DinaWallet.generate()
print(f"Address: {wallet.address}")

# Connect to testnet
client = DinaClient("https://rpc.dina.network")

# Check balance
balance = client.get_balance(wallet.address)
print(f"Balance: {balance} USDC")

# Send USDC
tx = client.send_transaction(
    from_wallet=wallet,
    to="dina1recipient...",
    amount="10.00",
)
print(f"TX Hash: {tx.hash}")

# Wait for confirmation
receipt = client.wait_for_transaction(tx.hash)
print(f"Status: {receipt.status}")  # "confirmed"`}
      />

      {/* ---- API Reference ---- */}
      <H2 id="api-reference">API Reference</H2>

      {/* DinaWallet */}
      <H3 id="dina-wallet">DinaWallet</H3>
      <p className="mb-4 text-sm text-slate-400">
        Key management and transaction signing. Private keys are held in memory
        only and never serialized.
      </p>

      <MethodSignature
        sig="DinaWallet.generate()"
        returns="DinaWallet"
        description="Generate a new random wallet with a fresh Ed25519 keypair."
      />
      <MethodSignature
        sig="DinaWallet.from_private_key(key: str)"
        returns="DinaWallet"
        description="Restore a wallet from a hex-encoded private key string."
      />
      <MethodSignature
        sig="DinaWallet.from_mnemonic(mnemonic: str, index: int = 0)"
        returns="DinaWallet"
        description="Derive a wallet from a BIP-39 mnemonic phrase with optional HD index."
      />
      <MethodSignature
        sig="wallet.sign(message: bytes)"
        returns="bytes"
        description="Sign arbitrary bytes with the wallet's private key."
      />
      <MethodSignature
        sig="wallet.verify(message: bytes, signature: bytes)"
        returns="bool"
        description="Verify a signature against the wallet's public key."
      />
      <MethodSignature
        sig="wallet.address"
        returns="str"
        description="The wallet's Dina address (read-only property)."
      />
      <MethodSignature
        sig="wallet.public_key"
        returns="str"
        description="Hex-encoded public key (read-only property)."
      />

      <CodeBlock
        language="python"
        filename="wallet_example.py"
        code={`from dina import DinaWallet

# Generate a new wallet
wallet = DinaWallet.generate()
print(f"Address:    {wallet.address}")
print(f"Public Key: {wallet.public_key}")

# Sign and verify
message = b"Hello Dina"
signature = wallet.sign(message)
assert wallet.verify(message, signature)

# Restore from private key
restored = DinaWallet.from_private_key("a1b2c3d4...")

# Restore from mnemonic
hd_wallet = DinaWallet.from_mnemonic(
    "abandon abandon abandon abandon abandon abandon "
    "abandon abandon abandon abandon abandon about",
    index=0,
)`}
      />

      {/* DinaClient */}
      <H3 id="dina-client">DinaClient</H3>
      <p className="mb-4 text-sm text-slate-400">
        Synchronous RPC client. For async usage, see the{" "}
        <a href="#async-support" className="text-blue-400 hover:underline">
          asyncio section
        </a>{" "}
        below.
      </p>

      <MethodSignature
        sig="DinaClient(rpc_url: str)"
        returns="DinaClient"
        description="Create a client connected to a Dina Network RPC endpoint."
      />
      <MethodSignature
        sig="client.get_balance(address: str)"
        returns="str"
        description="Get the USDC balance as a human-readable string (e.g. '100.50')."
      />
      <MethodSignature
        sig="client.get_account(address: str)"
        returns="Account"
        description="Get full account details: balance, nonce, code hash, and storage root."
      />
      <MethodSignature
        sig="client.get_block(number: int | str)"
        returns="Block"
        description="Fetch a block by number or 'latest'."
      />
      <MethodSignature
        sig="client.send_transaction(from_wallet, to, amount, data=None)"
        returns="TxResult"
        description="Sign and broadcast a transaction. Returns the tx hash and nonce."
      />
      <MethodSignature
        sig="client.estimate_gas(from_wallet, to, amount, data=None)"
        returns="str"
        description="Estimate gas cost in USDC micro-units."
      />
      <MethodSignature
        sig="client.get_transaction_receipt(tx_hash: str)"
        returns="Receipt | None"
        description="Get the receipt for a confirmed transaction. Returns None if pending."
      />
      <MethodSignature
        sig="client.wait_for_transaction(tx_hash: str, timeout: float = 30.0)"
        returns="Receipt"
        description="Block until a transaction is confirmed or the timeout is reached."
      />

      <CodeBlock
        language="python"
        filename="client_example.py"
        code={`from dina import DinaClient

client = DinaClient("https://rpc.dina.network")

# Latest block
block = client.get_block("latest")
print(f"Block #{block.number}, {len(block.transactions)} txs")

# Account info
account = client.get_account("dina1abc...")
print(f"Balance: {account.balance} USDC")
print(f"Nonce:   {account.nonce}")

# Estimate gas
fee = client.estimate_gas(
    from_wallet=wallet,
    to="dina1xyz...",
    amount="50.00",
)
print(f"Fee: {fee} USDC")`}
      />

      {/* DinaContract */}
      <H3 id="dina-contract">DinaContract</H3>
      <p className="mb-4 text-sm text-slate-400">
        Interact with and deploy smart contracts.
      </p>

      <MethodSignature
        sig="DinaContract(client, address, abi)"
        returns="DinaContract"
        description="Bind to an existing deployed contract."
      />
      <MethodSignature
        sig="contract.call(method: str, *args)"
        returns="Any"
        description="Call a read-only method (no gas cost)."
      />
      <MethodSignature
        sig="DinaContract.deploy(client, wallet, bytecode, abi, *args)"
        returns="DinaContract"
        description="Deploy a new contract and return the bound instance."
      />

      <CodeBlock
        language="python"
        filename="contract_example.py"
        code={`from dina import DinaClient, DinaContract, DinaWallet

client = DinaClient("https://rpc.dina.network")
wallet = DinaWallet.from_private_key("a1b2c3d4...")

# Read from a contract
contract = DinaContract(client, "dina1contract...", abi=MY_ABI)
score = contract.call("get_score", wallet.address)
print(f"Score: {score}")

# Deploy a contract
with open("my_contract.wasm", "rb") as f:
    bytecode = f.read().hex()

deployed = DinaContract.deploy(
    client, wallet, bytecode, MY_ABI,
    "Constructor Arg",
    42,
)
print(f"Deployed at: {deployed.address}")`}
      />

      {/* ---- Async Support ---- */}
      <H2 id="async-support">Async Support</H2>
      <p className="mb-4 text-sm text-slate-400">
        Every method on <code className="text-slate-300">DinaClient</code> has an
        async counterpart via <code className="text-slate-300">AsyncDinaClient</code>.
        Use it with <code className="text-slate-300">asyncio</code> for non-blocking
        I/O in web servers and high-throughput applications.
      </p>

      <CodeBlock
        language="python"
        filename="async_example.py"
        code={`import asyncio
from dina import DinaWallet, AsyncDinaClient

async def main():
    wallet = DinaWallet.generate()
    client = AsyncDinaClient("https://rpc.dina.network")

    # All methods are awaitable
    balance = await client.get_balance(wallet.address)
    print(f"Balance: {balance} USDC")

    # Send transaction
    tx = await client.send_transaction(
        from_wallet=wallet,
        to="dina1recipient...",
        amount="10.00",
    )

    # Wait for confirmation
    receipt = await client.wait_for_transaction(tx.hash)
    print(f"Confirmed in block {receipt.block_number}")

    # Parallel queries
    balances = await asyncio.gather(
        client.get_balance("dina1addr1..."),
        client.get_balance("dina1addr2..."),
        client.get_balance("dina1addr3..."),
    )
    print("Balances:", balances)

    await client.close()

asyncio.run(main())`}
      />

      {/* ---- Batch Operations ---- */}
      <H2 id="batch-operations">Batch Operations</H2>
      <CodeBlock
        language="python"
        filename="batch_example.py"
        code={`from dina import DinaClient, DinaWallet, BatchTransfer

client = DinaClient("https://rpc.dina.network")
wallet = DinaWallet.from_private_key("a1b2c3d4...")

# Send to multiple recipients in a single transaction (DRC-19)
batch = BatchTransfer(client, wallet)
batch.add("dina1alice...", "10.00")
batch.add("dina1bob...",   "25.50")
batch.add("dina1carol...", "5.00")

receipt = batch.execute()
print(f"Batch TX: {receipt.tx_hash}")
print(f"Total sent: 40.50 USDC in one tx")`}
      />

      {/* ---- Error Handling ---- */}
      <H2 id="error-handling">Error Handling</H2>

      <CodeBlock
        language="python"
        filename="error_handling.py"
        code={`from dina import DinaClient, DinaWallet
from dina.errors import (
    InsufficientFundsError,
    TransactionTimeoutError,
    InvalidAddressError,
    RpcError,
)

client = DinaClient("https://rpc.dina.network")
wallet = DinaWallet.from_private_key("a1b2c3d4...")

try:
    tx = client.send_transaction(
        from_wallet=wallet,
        to="dina1recipient...",
        amount="1000000.00",
    )
    receipt = client.wait_for_transaction(tx.hash, timeout=10.0)

except InsufficientFundsError as e:
    print(f"Not enough USDC. Have: {e.balance}, Need: {e.required}")

except TransactionTimeoutError as e:
    print(f"TX not confirmed in time. Hash: {e.tx_hash}")

except InvalidAddressError as e:
    print(f"Bad address: {e.address}")

except RpcError as e:
    print(f"RPC error {e.code}: {e.message}")`}
      />

      {/* ---- Data Classes ---- */}
      <H2 id="types">Data Classes</H2>
      <CodeBlock
        language="python"
        filename="types.py"
        code={`from dataclasses import dataclass
from typing import Optional

@dataclass
class Account:
    address: str
    balance: str        # USDC as human-readable string
    nonce: int
    code_hash: Optional[str] = None
    storage_root: Optional[str] = None

@dataclass
class Block:
    number: int
    hash: str
    parent_hash: str
    timestamp: int
    transactions: list[str]   # tx hashes
    validator: str
    gas_used: str

@dataclass
class TxResult:
    hash: str
    nonce: int

@dataclass
class Receipt:
    tx_hash: str
    status: str          # "confirmed" or "failed"
    block_number: int
    gas_used: str
    events: list[dict]

@dataclass
class AgentLimits:
    max_per_transaction: str   # USDC
    max_daily: str             # USDC
    allowed_recipients: list[str]
    expires_at: Optional[int] = None`}
      />
    </>
  );
}
