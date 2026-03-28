import { Wallet, Currency, Transaction } from './types';

// ---------------------------------------------------------------------------
// Testnet Configuration
// ---------------------------------------------------------------------------
export const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL || 'http://35.184.213.248:8545';
export const REST_URL = process.env.NEXT_PUBLIC_REST_URL || 'http://35.184.213.248:8080';
export const CHAIN_ID = 'dina-testnet-1';

// ---------------------------------------------------------------------------
// Mock Wallets
// ---------------------------------------------------------------------------
const NOW = Math.floor(Date.now() / 1000);
const ONE_HOUR_AGO = NOW - 3600;
const ONE_DAY_AGO = NOW - 86400;
const THREE_DAYS_AGO = NOW - 259200;

export const MOCK_WALLETS: Wallet[] = [
  {
    id: 'main-1',
    name: 'Main Wallet',
    type: 'main',
    icon: '🏦',
    balance: 2_347_530_000,
    yieldRateBps: 450,
    lastYieldUpdate: ONE_HOUR_AGO,
    isDefault: true,
    isSetUp: true,
  },
  {
    id: 'savings-1',
    name: 'Savings',
    type: 'savings',
    icon: '🐷',
    balance: 10_000_000_000,
    yieldRateBps: 500,
    lastYieldUpdate: ONE_HOUR_AGO,
    isSetUp: true,
  },
  {
    id: 'backup-1',
    name: 'Backup',
    type: 'backup',
    icon: '🔒',
    balance: 500_000_000,
    yieldRateBps: 450,
    lastYieldUpdate: ONE_HOUR_AGO,
    isSetUp: true,
  },
  {
    id: 'agent-1',
    name: 'Shopping',
    type: 'agent',
    icon: '🛒',
    balance: 150_000_000,
    yieldRateBps: 350,
    lastYieldUpdate: ONE_HOUR_AGO,
    dailyLimit: 500_000_000,
    isSetUp: true,
  },
  {
    id: 'agent-2',
    name: 'Bills',
    type: 'agent',
    icon: '📄',
    balance: 50_000_000,
    yieldRateBps: 350,
    lastYieldUpdate: ONE_HOUR_AGO,
    dailyLimit: 200_000_000,
    isSetUp: true,
  },
  {
    id: 'agent-3',
    name: 'Agent 3',
    type: 'agent',
    icon: '🤖',
    balance: 0,
    yieldRateBps: 350,
    lastYieldUpdate: NOW,
    isSetUp: false,
  },
  {
    id: 'speed-1',
    name: 'Business',
    type: 'speed',
    icon: '⚡',
    balance: 100_000_000,
    yieldRateBps: 300,
    lastYieldUpdate: ONE_HOUR_AGO,
    isSetUp: true,
  },
  {
    id: 'speed-2',
    name: 'Speed 2',
    type: 'speed',
    icon: '⚡',
    balance: 0,
    yieldRateBps: 300,
    lastYieldUpdate: NOW,
    isSetUp: false,
  },
  {
    id: 'speed-3',
    name: 'Speed 3',
    type: 'speed',
    icon: '⚡',
    balance: 0,
    yieldRateBps: 300,
    lastYieldUpdate: NOW,
    isSetUp: false,
  },
];

// ---------------------------------------------------------------------------
// All 40 Deployed Currencies
// Rates are micro-units per 1 USDC (6 decimals). e.g. EUR at 0.93 = 930_000
// Yield rates in basis points (450 = 4.50% APY)
// ---------------------------------------------------------------------------
export const MOCK_CURRENCIES: Currency[] = [
  // ── Major ──────────────────────────────────────────────────────────────
  { symbol: 'USDC',  name: 'US Dollar',              balance: 12_947_530_000,   yieldRateBps: 450, ratePerUsdc: 1_000_000,   icon: '🇺🇸', region: 'Major' },
  { symbol: 'EURC',  name: 'Euro',                   balance: 465_000_000,      yieldRateBps: 400, ratePerUsdc: 930_000,     icon: '🇪🇺', region: 'Major' },
  { symbol: 'GBPC',  name: 'British Pound',           balance: 200_000_000,      yieldRateBps: 420, ratePerUsdc: 790_000,     icon: '🇬🇧', region: 'Major' },
  { symbol: 'JPYC',  name: 'Japanese Yen',            balance: 150_000_000_000,  yieldRateBps: 100, ratePerUsdc: 149_000_000, icon: '🇯🇵', region: 'Major' },
  { symbol: 'CHFC',  name: 'Swiss Franc',             balance: 0,                yieldRateBps: 350, ratePerUsdc: 880_000,     icon: '🇨🇭', region: 'Major' },
  { symbol: 'AUDC',  name: 'Australian Dollar',       balance: 0,                yieldRateBps: 430, ratePerUsdc: 1_540_000,   icon: '🇦🇺', region: 'Major' },
  { symbol: 'CADC',  name: 'Canadian Dollar',          balance: 1_500_000_000,    yieldRateBps: 430, ratePerUsdc: 1_370_000,   icon: '🇨🇦', region: 'Major' },
  { symbol: 'CNHC',  name: 'Chinese Yuan (Offshore)',  balance: 0,                yieldRateBps: 200, ratePerUsdc: 7_250_000,   icon: '🇨🇳', region: 'Major' },

  // ── European ───────────────────────────────────────────────────────────
  { symbol: 'SEKC',  name: 'Swedish Krona',           balance: 0, yieldRateBps: 380, ratePerUsdc: 10_500_000,  icon: '🇸🇪', region: 'European' },
  { symbol: 'NOKC',  name: 'Norwegian Krone',         balance: 0, yieldRateBps: 400, ratePerUsdc: 10_800_000,  icon: '🇳🇴', region: 'European' },
  { symbol: 'DKKC',  name: 'Danish Krone',            balance: 0, yieldRateBps: 390, ratePerUsdc: 6_940_000,   icon: '🇩🇰', region: 'European' },
  { symbol: 'PLNC',  name: 'Polish Zloty',            balance: 0, yieldRateBps: 500, ratePerUsdc: 4_020_000,   icon: '🇵🇱', region: 'European' },
  { symbol: 'CZKC',  name: 'Czech Koruna',            balance: 0, yieldRateBps: 470, ratePerUsdc: 23_400_000,  icon: '🇨🇿', region: 'European' },
  { symbol: 'HUFC',  name: 'Hungarian Forint',        balance: 0, yieldRateBps: 550, ratePerUsdc: 370_000_000, icon: '🇭🇺', region: 'European' },
  { symbol: 'RONC',  name: 'Romanian Leu',            balance: 0, yieldRateBps: 480, ratePerUsdc: 4_650_000,   icon: '🇷🇴', region: 'European' },
  { symbol: 'TRYC',  name: 'Turkish Lira',            balance: 0, yieldRateBps: 800, ratePerUsdc: 32_500_000,  icon: '🇹🇷', region: 'European' },

  // ── Americas ───────────────────────────────────────────────────────────
  { symbol: 'MXNC',  name: 'Mexican Peso',            balance: 0, yieldRateBps: 600, ratePerUsdc: 17_200_000,  icon: '🇲🇽', region: 'Americas' },
  { symbol: 'BRLC',  name: 'Brazilian Real',           balance: 0, yieldRateBps: 700, ratePerUsdc: 5_100_000,   icon: '🇧🇷', region: 'Americas' },
  { symbol: 'ARSC',  name: 'Argentine Peso',           balance: 0, yieldRateBps: 900, ratePerUsdc: 900_000_000, icon: '🇦🇷', region: 'Americas' },
  { symbol: 'CLPC',  name: 'Chilean Peso',             balance: 0, yieldRateBps: 550, ratePerUsdc: 950_000_000, icon: '🇨🇱', region: 'Americas' },
  { symbol: 'COPC',  name: 'Colombian Peso',           balance: 0, yieldRateBps: 600, ratePerUsdc: 4_100_000_000, icon: '🇨🇴', region: 'Americas' },
  { symbol: 'PENC',  name: 'Peruvian Sol',             balance: 0, yieldRateBps: 500, ratePerUsdc: 3_750_000,   icon: '🇵🇪', region: 'Americas' },

  // ── Asia-Pacific ───────────────────────────────────────────────────────
  { symbol: 'KRWC',  name: 'South Korean Won',         balance: 0, yieldRateBps: 300, ratePerUsdc: 1_350_000_000, icon: '🇰🇷', region: 'Asia-Pacific' },
  { symbol: 'INRC',  name: 'Indian Rupee',             balance: 0, yieldRateBps: 500, ratePerUsdc: 83_500_000,    icon: '🇮🇳', region: 'Asia-Pacific' },
  { symbol: 'SGDC',  name: 'Singapore Dollar',         balance: 0, yieldRateBps: 380, ratePerUsdc: 1_340_000,     icon: '🇸🇬', region: 'Asia-Pacific' },
  { symbol: 'HKDC',  name: 'Hong Kong Dollar',         balance: 0, yieldRateBps: 400, ratePerUsdc: 7_810_000,     icon: '🇭🇰', region: 'Asia-Pacific' },
  { symbol: 'TWDC',  name: 'Taiwan Dollar',            balance: 0, yieldRateBps: 350, ratePerUsdc: 32_200_000,    icon: '🇹🇼', region: 'Asia-Pacific' },
  { symbol: 'THBC',  name: 'Thai Baht',                balance: 0, yieldRateBps: 350, ratePerUsdc: 35_500_000,    icon: '🇹🇭', region: 'Asia-Pacific' },
  { symbol: 'IDRC',  name: 'Indonesian Rupiah',        balance: 0, yieldRateBps: 450, ratePerUsdc: 15_800_000_000, icon: '🇮🇩', region: 'Asia-Pacific' },
  { symbol: 'MYRC',  name: 'Malaysian Ringgit',        balance: 0, yieldRateBps: 380, ratePerUsdc: 4_470_000,     icon: '🇲🇾', region: 'Asia-Pacific' },
  { symbol: 'PHPC',  name: 'Philippine Peso',          balance: 0, yieldRateBps: 420, ratePerUsdc: 56_200_000,    icon: '🇵🇭', region: 'Asia-Pacific' },
  { symbol: 'VNDC',  name: 'Vietnamese Dong',          balance: 0, yieldRateBps: 400, ratePerUsdc: 25_200_000_000, icon: '🇻🇳', region: 'Asia-Pacific' },
  { symbol: 'NZDC',  name: 'New Zealand Dollar',       balance: 0, yieldRateBps: 420, ratePerUsdc: 1_680_000,     icon: '🇳🇿', region: 'Asia-Pacific' },

  // ── Middle East & Africa ───────────────────────────────────────────────
  { symbol: 'AEDC',  name: 'UAE Dirham',               balance: 0, yieldRateBps: 350, ratePerUsdc: 3_670_000,     icon: '🇦🇪', region: 'Middle East & Africa' },
  { symbol: 'SARC',  name: 'Saudi Riyal',              balance: 0, yieldRateBps: 350, ratePerUsdc: 3_750_000,     icon: '🇸🇦', region: 'Middle East & Africa' },
  { symbol: 'ILSC',  name: 'Israeli Shekel',           balance: 0, yieldRateBps: 400, ratePerUsdc: 3_650_000,     icon: '🇮🇱', region: 'Middle East & Africa' },
  { symbol: 'ZARC',  name: 'South African Rand',       balance: 0, yieldRateBps: 600, ratePerUsdc: 18_200_000,    icon: '🇿🇦', region: 'Middle East & Africa' },
  { symbol: 'NGnc',  name: 'Nigerian Naira',           balance: 0, yieldRateBps: 800, ratePerUsdc: 1_550_000_000, icon: '🇳🇬', region: 'Middle East & Africa' },
  { symbol: 'KESC',  name: 'Kenyan Shilling',          balance: 0, yieldRateBps: 650, ratePerUsdc: 129_000_000,   icon: '🇰🇪', region: 'Middle East & Africa' },
  { symbol: 'EGPC',  name: 'Egyptian Pound',           balance: 0, yieldRateBps: 750, ratePerUsdc: 49_500_000,    icon: '🇪🇬', region: 'Middle East & Africa' },
];

// ---------------------------------------------------------------------------
// Currency region order for display grouping
// ---------------------------------------------------------------------------
export const CURRENCY_REGIONS = ['Major', 'European', 'Americas', 'Asia-Pacific', 'Middle East & Africa'] as const;

// ---------------------------------------------------------------------------
// Mock Transactions
// ---------------------------------------------------------------------------
export const MOCK_TRANSACTIONS: Transaction[] = [
  { id: 'tx-1', type: 'receive', amount: 500_000_000, currency: 'USDC', counterparty: 'alice.dina', timestamp: ONE_HOUR_AGO, status: 'confirmed', wallet: 'Main Wallet' },
  { id: 'tx-2', type: 'send', amount: 25_000_000, currency: 'USDC', counterparty: 'bob@email.com', timestamp: ONE_DAY_AGO, status: 'confirmed', wallet: 'Main Wallet' },
  { id: 'tx-3', type: 'yield', amount: 1_232_000, currency: 'USDC', timestamp: ONE_DAY_AGO, status: 'confirmed', wallet: 'Savings' },
  { id: 'tx-4', type: 'convert', amount: 200_000_000, currency: 'EURC', counterparty: 'USDC -> EURC', timestamp: THREE_DAYS_AGO, status: 'confirmed', wallet: 'Main Wallet' },
  { id: 'tx-5', type: 'send', amount: 75_000_000, currency: 'USDC', counterparty: 'carol.dina', timestamp: THREE_DAYS_AGO, status: 'confirmed', wallet: 'Shopping' },
];

export const MOCK_WALLET_ADDRESS = '0xd1nA...7f3E';
export const MOCK_WALLET_ADDRESS_FULL = '0xd1nA4b2c8E9f0A1B3d5C7e2F4a6D8b0E9c1F3a5D7f3E';
