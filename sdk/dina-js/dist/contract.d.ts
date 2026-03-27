import type { Address, Hash, SpendingStats } from './types';
import { DinaClient } from './client';
import { DinaWallet } from './wallet';
/**
 * Generic smart contract interaction wrapper.
 *
 * Provides `call` (state-changing) and `view` (read-only) methods
 * that map to the Dina WASM contract ABI.
 */
export declare class DinaContract {
    readonly address: Address;
    protected readonly client: DinaClient;
    constructor(address: Address, client: DinaClient);
    /**
     * Execute a state-changing contract method.
     * Requires a wallet for signing. Returns the transaction hash.
     */
    call(method: string, args: unknown, wallet?: DinaWallet, usdcAttached?: bigint): Promise<Hash>;
    /**
     * Execute a read-only view method. No wallet or gas needed.
     */
    view(method: string, args: unknown): Promise<unknown>;
    /** Create a DRC-1 token contract helper. */
    static token(address: Address, client: DinaClient): TokenContract;
    /** Create a DRC-101 agent wallet contract helper. */
    static agentWallet(address: Address, client: DinaClient): AgentWalletContract;
}
/**
 * DRC-1 Token standard contract — fungible token interface.
 * Similar to ERC-20 but for Dina Network.
 */
export declare class TokenContract extends DinaContract {
    /** Get token balance for an address. */
    balanceOf(owner: Address): Promise<bigint>;
    /** Transfer tokens to another address. */
    transfer(wallet: DinaWallet, to: Address, amount: bigint): Promise<Hash>;
    /** Approve a spender to transfer tokens on your behalf. */
    approve(wallet: DinaWallet, spender: Address, amount: bigint): Promise<Hash>;
    /** Check how much a spender is allowed to spend on behalf of an owner. */
    allowance(owner: Address, spender: Address): Promise<bigint>;
    /** Get the total token supply. */
    totalSupply(): Promise<bigint>;
}
/**
 * DRC-101 Agent Wallet contract — AI agent spending wallet.
 * Provides spending limits, emergency controls, and stats.
 */
export declare class AgentWalletContract extends DinaContract {
    /** Execute a transfer from the agent wallet. */
    executeTransfer(wallet: DinaWallet, to: Address, amount: bigint): Promise<Hash>;
    /** Get spending statistics for the agent wallet. */
    spendingStats(): Promise<SpendingStats>;
    /** Halt all agent wallet activity immediately. */
    emergencyStop(wallet: DinaWallet): Promise<Hash>;
    /** Resume agent wallet activity after an emergency stop. */
    resume(wallet: DinaWallet): Promise<Hash>;
}
//# sourceMappingURL=contract.d.ts.map