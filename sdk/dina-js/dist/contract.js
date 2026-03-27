"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.AgentWalletContract = exports.TokenContract = exports.DinaContract = void 0;
/**
 * Generic smart contract interaction wrapper.
 *
 * Provides `call` (state-changing) and `view` (read-only) methods
 * that map to the Dina WASM contract ABI.
 */
class DinaContract {
    constructor(address, client) {
        this.address = address;
        this.client = client;
    }
    /**
     * Execute a state-changing contract method.
     * Requires a wallet for signing. Returns the transaction hash.
     */
    async call(method, args, wallet, usdcAttached) {
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
    async view(method, args) {
        return this.client['rpc']('dina_viewContract', [
            this.address,
            method,
            args,
        ]);
    }
    /** Create a DRC-1 token contract helper. */
    static token(address, client) {
        return new TokenContract(address, client);
    }
    /** Create a DRC-101 agent wallet contract helper. */
    static agentWallet(address, client) {
        return new AgentWalletContract(address, client);
    }
}
exports.DinaContract = DinaContract;
/**
 * DRC-1 Token standard contract — fungible token interface.
 * Similar to ERC-20 but for Dina Network.
 */
class TokenContract extends DinaContract {
    /** Get token balance for an address. */
    async balanceOf(owner) {
        const result = await this.view('balance_of', { account: owner });
        return BigInt(result);
    }
    /** Transfer tokens to another address. */
    async transfer(wallet, to, amount) {
        return this.call('transfer', { to, amount: amount.toString() }, wallet);
    }
    /** Approve a spender to transfer tokens on your behalf. */
    async approve(wallet, spender, amount) {
        return this.call('approve', { spender, amount: amount.toString() }, wallet);
    }
    /** Check how much a spender is allowed to spend on behalf of an owner. */
    async allowance(owner, spender) {
        const result = await this.view('allowance', { owner, spender });
        return BigInt(result);
    }
    /** Get the total token supply. */
    async totalSupply() {
        const result = await this.view('total_supply', {});
        return BigInt(result);
    }
}
exports.TokenContract = TokenContract;
/**
 * DRC-101 Agent Wallet contract — AI agent spending wallet.
 * Provides spending limits, emergency controls, and stats.
 */
class AgentWalletContract extends DinaContract {
    /** Execute a transfer from the agent wallet. */
    async executeTransfer(wallet, to, amount) {
        return this.call('execute_transfer', { to, amount: amount.toString() }, wallet);
    }
    /** Get spending statistics for the agent wallet. */
    async spendingStats() {
        const raw = (await this.view('spending_stats', {}));
        return {
            totalSpent: BigInt(raw.totalSpent),
            transactionCount: raw.transactionCount,
            lastTransaction: raw.lastTransaction,
            dailyLimit: BigInt(raw.dailyLimit),
            dailySpent: BigInt(raw.dailySpent),
        };
    }
    /** Halt all agent wallet activity immediately. */
    async emergencyStop(wallet) {
        return this.call('emergency_stop', {}, wallet);
    }
    /** Resume agent wallet activity after an emergency stop. */
    async resume(wallet) {
        return this.call('resume', {}, wallet);
    }
}
exports.AgentWalletContract = AgentWalletContract;
//# sourceMappingURL=contract.js.map