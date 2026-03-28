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

export interface Currency {
  symbol: string;
  name: string;
  balance: number;
  yieldRateBps: number;
  ratePerUsdc: number;
  icon: string;
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
