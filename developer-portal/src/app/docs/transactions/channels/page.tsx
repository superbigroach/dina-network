import Link from "next/link";

export const metadata = {
  title: "Payment Channels — Dina Network Developer Portal",
  description:
    "Off-chain bilateral payment channels for instant 5ms micropayments on Dina Network.",
};

export default function PaymentChannelsPage() {
  return (
    <div>
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Transactions
      </p>
      <h1 className="text-4xl font-bold tracking-tight text-white mb-4">
        Payment Channels
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-10">
        Off-chain bilateral payment channels that enable unlimited instant
        payments between two parties at 5ms latency. Open a channel, exchange
        thousands of payments without touching the blockchain, then close and
        settle on-chain. Perfect for IoT micropayments, streaming payments,
        and high-frequency machine-to-machine commerce.
      </p>

      {/* How It Works */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          How It Works
        </h2>
        <div className="space-y-6">
          <div className="flex gap-4 items-start">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-blue-600/20 text-sm font-bold text-blue-400">
              1
            </div>
            <div>
              <h3 className="text-sm font-semibold text-white">Open Channel (on-chain)</h3>
              <p className="text-sm text-slate-400 mt-0.5">
                Party A deposits USDC into a payment channel contract, specifying
                Party B as the counterparty and a funding amount. This is the
                only on-chain transaction required to start. Costs 30,000 gas.
              </p>
            </div>
          </div>
          <div className="flex gap-4 items-start">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-blue-600/20 text-sm font-bold text-blue-400">
              2
            </div>
            <div>
              <h3 className="text-sm font-semibold text-white">Exchange Payments (off-chain)</h3>
              <p className="text-sm text-slate-400 mt-0.5">
                Both parties exchange signed balance updates directly (peer-to-peer).
                Each update contains the latest balance allocation and a monotonically
                increasing sequence number. No blockchain interaction. Latency is
                approximately 5ms on a local network.
              </p>
            </div>
          </div>
          <div className="flex gap-4 items-start">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-blue-600/20 text-sm font-bold text-blue-400">
              3
            </div>
            <div>
              <h3 className="text-sm font-semibold text-white">Close Channel (on-chain)</h3>
              <p className="text-sm text-slate-400 mt-0.5">
                Either party submits the latest signed balance update to the channel
                contract. After a challenge period of 1,000 blocks (~100 seconds),
                the funds are distributed according to the final balances.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Comparison */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          On-Chain vs Payment Channel
        </h2>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Property</th>
                <th className="px-4 py-3 text-left font-medium text-blue-400">On-Chain Transfer</th>
                <th className="px-4 py-3 text-left font-medium text-green-400">Payment Channel</th>
              </tr>
            </thead>
            <tbody>
              {[
                { prop: "Latency", onchain: "100ms (1 block)", channel: "~5ms (local)" },
                { prop: "Cost per payment", onchain: "0.001 USDC", channel: "Free (after open/close)" },
                { prop: "Throughput", onchain: "10,000 TPS (network)", channel: "Unlimited (bilateral)" },
                { prop: "Finality", onchain: "Immediate (1 block)", channel: "On close + challenge period" },
                { prop: "Internet required", onchain: "Yes", channel: "No (peer-to-peer)" },
                { prop: "Counterparty", onchain: "Any address", channel: "Pre-established partner" },
                { prop: "Setup cost", onchain: "None", channel: "30,000 gas to open" },
              ].map((row, i) => (
                <tr
                  key={row.prop}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 font-medium text-slate-300">{row.prop}</td>
                  <td className="px-4 py-3 text-slate-400">{row.onchain}</td>
                  <td className="px-4 py-3 text-slate-400">{row.channel}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Code Example: Open */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Open a Payment Channel
        </h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`import { DinaClient, DinaWallet, PaymentChannel, parseUSDC } from '@dina-network/sdk';

const client = new DinaClient('https://rpc-testnet.dina.network');
const wallet = DinaWallet.fromKeyFile('./my-wallet.json');

// Open a channel with 100 USDC deposit
const channel = await PaymentChannel.open({
  client,
  wallet,
  counterparty: '0xccc...counterparty',
  deposit: parseUSDC('100'),        // lock 100 USDC
  challengePeriod: 1000,            // 1,000 blocks (~100 seconds)
});

console.log('Channel ID:', channel.id);
console.log('Deposit:', channel.deposit);       // 100_000_000
console.log('Status:', channel.status);         // "open"
console.log('My balance:', channel.myBalance);  // 100_000_000
console.log('Their balance:', channel.theirBalance); // 0`}</code>
        </pre>
      </section>

      {/* Code Example: Use */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Send Payments Through Channel
        </h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`// Send micropayments at 5ms speed -- no blockchain involved
// Each payment is a signed balance update exchanged peer-to-peer

// Pay 0.01 USDC for a sensor reading
await channel.pay(parseUSDC('0.01'));
console.log('Balance after pay:', channel.myBalance); // 99_990_000

// Pay again -- unlimited payments at near-zero latency
await channel.pay(parseUSDC('0.01'));
await channel.pay(parseUSDC('0.01'));
await channel.pay(parseUSDC('0.05'));
// Total sent: 0.08 USDC across 4 payments, zero gas cost

// Receive payments from counterparty
channel.onPayment((amount, newBalance) => {
  console.log(\`Received \${amount / 1_000_000} USDC\`);
  console.log(\`My new balance: \${newBalance / 1_000_000} USDC\`);
});

// Check current state at any time
console.log('Sequence number:', channel.sequence);      // 4
console.log('My balance:', channel.myBalance);          // 99_920_000
console.log('Their balance:', channel.theirBalance);    // 80_000`}</code>
        </pre>
      </section>

      {/* Code Example: Close */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Close and Settle
        </h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`// Close the channel -- submits final balance to chain
const closeReceipt = await channel.close();
console.log('Close TX:', closeReceipt.hash);
console.log('Challenge period: 1,000 blocks (~100 seconds)');

// After the challenge period, finalize settlement
const settleReceipt = await channel.finalize();
console.log('Settled! Funds released.');
console.log('My withdrawal:', settleReceipt.myWithdrawal);     // 99_920_000
console.log('Their withdrawal:', settleReceipt.theirWithdrawal); // 80_000`}</code>
        </pre>
      </section>

      {/* Challenge Period */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Challenge Period
        </h2>
        <div className="rounded-xl border border-amber-500/30 bg-amber-500/5 p-6">
          <h3 className="text-sm font-semibold text-amber-400 mb-2">
            Dispute Resolution: 1,000 Blocks (~100 Seconds)
          </h3>
          <p className="text-sm text-slate-300 leading-relaxed mb-4">
            When a channel is closed, there is a mandatory challenge period of
            1,000 blocks (approximately 100 seconds at 100ms block time). During
            this window, either party can submit a more recent signed balance
            update if they believe the submitted state is outdated.
          </p>
          <p className="text-sm text-slate-300 leading-relaxed mb-4">
            This prevents a malicious party from closing the channel with an
            old state that favors them. The contract always accepts the balance
            update with the highest sequence number.
          </p>
          <pre className="overflow-x-auto rounded-lg bg-slate-800 p-4 text-sm leading-relaxed">
            <code className="text-slate-200">{`// If you detect a stale close attempt, challenge it
channel.onDisputeDetected(async (staleState, latestState) => {
  console.log('Dispute! They submitted sequence', staleState.sequence);
  console.log('We have sequence', latestState.sequence);

  // Submit the latest state as a challenge
  const challengeReceipt = await channel.challenge(latestState);
  console.log('Challenge submitted:', challengeReceipt.hash);
});`}</code>
          </pre>
        </div>
      </section>

      {/* IoT Use Case */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          IoT Micropayment Example
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Payment channels are ideal for IoT devices that need to make
          frequent, small payments. A sensor might sell data readings at
          $0.001 each, making thousands of payments per hour.
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`import { DinaClient, DinaWallet, PaymentChannel, parseUSDC } from '@dina-network/sdk';

// Temperature sensor sells readings at $0.001 each
const sensor = DinaWallet.fromKeyFile('./sensor-wallet.json');
const client = new DinaClient('https://rpc-testnet.dina.network');

// Open channel with the data consumer
const channel = await PaymentChannel.open({
  client,
  wallet: sensor,
  counterparty: '0xbbb...data-buyer',
  deposit: parseUSDC('10'),  // fund with 10 USDC
});

// Sell sensor readings -- each payment takes ~5ms
async function sellReading(temperature: number) {
  // Consumer pays the sensor through the channel
  // (In practice, the consumer initiates the pay)
  const reading = { temperature, timestamp: Date.now() };

  // Deliver data + receive payment atomically
  await channel.receivePayment(parseUSDC('0.001'));
  return reading;
}

// 1,000 readings per hour = 1 USDC/hour, zero gas
// Channel stays open for days, only 2 on-chain txs total`}</code>
        </pre>
      </section>

      {/* CLI */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">CLI</h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`# Open a payment channel
dina channel open \\
  --counterparty 0xccc...counterparty \\
  --deposit 100.0 \\
  --challenge-period 1000 \\
  --network testnet

# List open channels
dina channel list --network testnet

# Check channel status
dina channel status <channel-id> --network testnet

# Close a channel
dina channel close <channel-id> --network testnet

# Finalize after challenge period
dina channel finalize <channel-id> --network testnet`}</code>
        </pre>
      </section>

      {/* Next Steps */}
      <div className="mt-10 flex flex-wrap gap-4">
        <Link
          href="/docs/transactions/fees"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          &larr; Gas & Fees
        </Link>
        <Link
          href="/docs/transactions/lifecycle"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Transaction Lifecycle &rarr;
        </Link>
      </div>
    </div>
  );
}
