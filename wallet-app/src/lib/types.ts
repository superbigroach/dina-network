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
