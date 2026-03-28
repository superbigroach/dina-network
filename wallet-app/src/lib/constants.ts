import { Wallet, Currency, Transaction } from './types';

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

export const MOCK_CURRENCIES: Currency[] = [
  { symbol: 'USDC', name: 'US Dollar', balance: 12_947_530_000, yieldRateBps: 450, ratePerUsdc: 1_000_000, icon: '🇺🇸' },
  { symbol: 'EURC', name: 'Euro', balance: 465_000_000, yieldRateBps: 400, ratePerUsdc: 930_000, icon: '🇪🇺' },
  { symbol: 'GBPC', name: 'British Pound', balance: 200_000_000, yieldRateBps: 420, ratePerUsdc: 790_000, icon: '🇬🇧' },
  { symbol: 'CADC', name: 'Canadian Dollar', balance: 1_500_000_000, yieldRateBps: 430, ratePerUsdc: 1_370_000, icon: '🇨🇦' },
  { symbol: 'JPYC', name: 'Japanese Yen', balance: 150_000_000_000, yieldRateBps: 100, ratePerUsdc: 149_000_000, icon: '🇯🇵' },
];

export const MOCK_TRANSACTIONS: Transaction[] = [
  { id: 'tx-1', type: 'receive', amount: 500_000_000, currency: 'USDC', counterparty: 'alice.dina', timestamp: ONE_HOUR_AGO, status: 'confirmed', wallet: 'Main Wallet' },
  { id: 'tx-2', type: 'send', amount: 25_000_000, currency: 'USDC', counterparty: 'bob@email.com', timestamp: ONE_DAY_AGO, status: 'confirmed', wallet: 'Main Wallet' },
  { id: 'tx-3', type: 'yield', amount: 1_232_000, currency: 'USDC', timestamp: ONE_DAY_AGO, status: 'confirmed', wallet: 'Savings' },
  { id: 'tx-4', type: 'convert', amount: 200_000_000, currency: 'EURC', counterparty: 'USDC -> EURC', timestamp: THREE_DAYS_AGO, status: 'confirmed', wallet: 'Main Wallet' },
  { id: 'tx-5', type: 'send', amount: 75_000_000, currency: 'USDC', counterparty: 'carol.dina', timestamp: THREE_DAYS_AGO, status: 'confirmed', wallet: 'Shopping' },
];

export const MOCK_WALLET_ADDRESS = '0xd1nA...7f3E';
export const MOCK_WALLET_ADDRESS_FULL = '0xd1nA4b2c8E9f0A1B3d5C7e2F4a6D8b0E9c1F3a5D7f3E';
