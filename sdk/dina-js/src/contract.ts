import type { Address, Hash, SpendingStats } from './types';
import { DinaClient } from './client';
import { DinaWallet } from './wallet';

/**
 * Generic smart contract interaction wrapper.
 *
 * Provides `call` (state-changing) and `view` (read-only) methods
 * that map to the Dina WASM contract ABI.
 */
export class DinaContract {
  constructor(
    public readonly address: Address,
    protected readonly client: DinaClient
  ) {}

  /**
   * Execute a state-changing contract method.
   * Requires a wallet for signing. Returns the transaction hash.
   */
  async call(
    method: string,
    args: unknown,
    wallet?: DinaWallet,
    usdcAttached?: bigint
  ): Promise<Hash> {
    if (!wallet) {
      throw new Error('Wallet required for state-changing calls');
    }
    return this.client.callContract(wallet, {
      contract: this.address,
      method,
      args,
      usdcAttached,
    });
  }

  /**
   * Execute a read-only view method. No wallet or gas needed.
   */
  async view(method: string, args: unknown): Promise<unknown> {
    return this.client['rpc']('dina_viewContract', [
      this.address,
      method,
      args,
    ]);
  }

  /** Create a DRC-1 token contract helper. */
  static token(address: Address, client: DinaClient): TokenContract {
    return new TokenContract(address, client);
  }

  /** Create a DRC-101 agent wallet contract helper. */
  static agentWallet(
    address: Address,
    client: DinaClient
  ): AgentWalletContract {
    return new AgentWalletContract(address, client);
  }
}

/**
 * DRC-1 Token standard contract — fungible token interface.
 * Similar to ERC-20 but for Dina Network.
 */
export class TokenContract extends DinaContract {
  /** Get token balance for an address. */
  async balanceOf(owner: Address): Promise<bigint> {
    const result = await this.view('balance_of', { account: owner });
    return BigInt(result as string);
  }

  /** Transfer tokens to another address. */
  async transfer(
    wallet: DinaWallet,
    to: Address,
    amount: bigint
  ): Promise<Hash> {
    return this.call(
      'transfer',
      { to, amount: amount.toString() },
      wallet
    );
  }

  /** Approve a spender to transfer tokens on your behalf. */
  async approve(
    wallet: DinaWallet,
    spender: Address,
    amount: bigint
  ): Promise<Hash> {
    return this.call(
      'approve',
      { spender, amount: amount.toString() },
      wallet
    );
  }

  /** Check how much a spender is allowed to spend on behalf of an owner. */
  async allowance(owner: Address, spender: Address): Promise<bigint> {
    const result = await this.view('allowance', { owner, spender });
    return BigInt(result as string);
  }

  /** Get the total token supply. */
  async totalSupply(): Promise<bigint> {
    const result = await this.view('total_supply', {});
    return BigInt(result as string);
  }
}

/**
 * DRC-101 Agent Wallet contract — AI agent spending wallet.
 * Provides spending limits, emergency controls, and stats.
 */
export class AgentWalletContract extends DinaContract {
  /** Execute a transfer from the agent wallet. */
  async executeTransfer(
    wallet: DinaWallet,
    to: Address,
    amount: bigint
  ): Promise<Hash> {
    return this.call(
      'execute_transfer',
      { to, amount: amount.toString() },
      wallet
    );
  }

  /** Get spending statistics for the agent wallet. */
  async spendingStats(): Promise<SpendingStats> {
    const raw = (await this.view('spending_stats', {})) as {
      totalSpent: string;
      transactionCount: number;
      lastTransaction: number;
      dailyLimit: string;
      dailySpent: string;
    };
    return {
      totalSpent: BigInt(raw.totalSpent),
      transactionCount: raw.transactionCount,
      lastTransaction: raw.lastTransaction,
      dailyLimit: BigInt(raw.dailyLimit),
      dailySpent: BigInt(raw.dailySpent),
    };
  }

  /** Halt all agent wallet activity immediately. */
  async emergencyStop(wallet: DinaWallet): Promise<Hash> {
    return this.call('emergency_stop', {}, wallet);
  }

  /** Resume agent wallet activity after an emergency stop. */
  async resume(wallet: DinaWallet): Promise<Hash> {
    return this.call('resume', {}, wallet);
  }
}
