export type WalletType = 'main' | 'savings' | 'backup' | 'agent' | 'speed';

export interface Wallet {
  id: string;
  name: string;
  type: WalletType;
  icon: string;
  balance: number;
  yieldRateBps: number;
  lastYieldUpdate: number;
  dailyLimit?: number;
  isDefault?: boolean;
  isSetUp: boolean;
}

export type CurrencyRegion =
  | 'Major'
  | 'North America'
  | 'South America'
  | 'Western Europe'
  | 'Eastern Europe'
  | 'East Asia'
  | 'Southeast Asia'
  | 'South Asia'
  | 'Middle East'
  | 'Central Asia'
  | 'North Africa'
  | 'West Africa'
  | 'East Africa'
  | 'Central Africa'
  | 'Southern Africa'
  | 'Oceania'
  // Legacy aliases kept for backward compatibility
  | 'European'
  | 'Americas'
  | 'Asia-Pacific'
  | 'Middle East & Africa';

export interface Currency {
  symbol: string;
  name: string;
  balance: number;
  yieldRateBps: number;
  ratePerUsdc: number;
  icon: string;
  region: CurrencyRegion;
}

export interface Transaction {
  id: string;
  type: 'send' | 'receive' | 'convert' | 'yield';
  amount: number;
  currency: string;
  counterparty?: string;
  timestamp: number;
  status: 'confirmed' | 'pending';
  wallet: string;
}

export interface RateStats {
  current: number;       // micro-units per USDC
  high24h: number;
  low24h: number;
  avg24h: number;
  high7d: number;
  low7d: number;
  avg7d: number;
  high30d: number;
  low30d: number;
  avg30d: number;
  change24hBps: number;  // positive = local currency weakened vs USD
  change7dBps: number;
  change30dBps: number;
}

export interface YieldInfo {
  currency: string;
  currencySymbol: string;
  flag: string;
  localBalance: number;       // balance in local currency (micro-units)
  usdcBacking: number;        // USDC locked (micro-USDC)
  pendingYieldUsdc: number;   // unclaimed USDC yield
  pendingYieldLocal: number;  // unclaimed yield in local currency
  totalClaimedUsdc: number;   // all-time claimed USDC
  totalClaimedLocal: number;  // all-time claimed in local currency
  depositRate: number;        // rate when they deposited
  currentRate: number;        // current rate
  depositTime: number;        // timestamp
}
