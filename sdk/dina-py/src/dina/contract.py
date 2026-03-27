"""Smart contract interaction helpers for the Dina Network."""

from __future__ import annotations

from typing import Any, Optional

from .client import DinaClient, DinaError
from .types import Address, Hash, TransactionReceipt
from .wallet import DinaWallet


class DinaContract:
    """Wrapper for interacting with a deployed Dina smart contract.

    Provides convenience methods for calling state-mutating and
    read-only (view) methods on a contract.

    Args:
        address: The on-chain address of the deployed contract.
        client: A connected DinaClient instance.
    """

    def __init__(self, address: Address, client: DinaClient) -> None:
        self.address = address
        self._client = client

    def call(
        self,
        method: str,
        args: Optional[dict[str, Any]] = None,
        wallet: Optional[DinaWallet] = None,
        usdc: int = 0,
        wait: bool = False,
        timeout: float = 30.0,
    ) -> Hash | TransactionReceipt:
        """Call a state-mutating method on the contract.

        Args:
            method: The contract method name.
            args: Keyword arguments to pass to the method.
            wallet: The wallet to sign the transaction. Required for
                state-mutating calls.
            usdc: Micro-USDC to send with the call (default 0).
            wait: If True, wait for the transaction to be confirmed and
                return the receipt instead of the hash.
            timeout: Seconds to wait if ``wait=True`` (default 30).

        Returns:
            The transaction hash (str) or a TransactionReceipt if
            ``wait=True``.

        Raises:
            ValueError: If no wallet is provided.
        """
        if wallet is None:
            raise ValueError("A wallet is required for state-mutating calls")

        tx_hash = self._client.call_contract(
            wallet=wallet,
            contract=self.address,
            method=method,
            args=args or {},
            usdc=usdc,
        )

        if wait:
            return self._client.wait_for_transaction(tx_hash, timeout=timeout)
        return tx_hash

    def view(self, method: str, args: Optional[dict[str, Any]] = None) -> Any:
        """Call a read-only (view) method on the contract.

        View calls do not require a wallet or transaction -- they query
        the current contract state via the RPC node.

        Args:
            method: The contract method name.
            args: Keyword arguments to pass to the method.

        Returns:
            The deserialized return value from the contract.
        """
        result = self._client._rpc(
            "contract.view",
            {
                "contract": self.address,
                "method": method,
                "args": args or {},
            },
        )
        return result

    def __repr__(self) -> str:
        return f"DinaContract(address={self.address!r})"


class TokenContract(DinaContract):
    """Helper for DRC-1 token contracts (fungible tokens like USDC on Dina).

    Provides typed wrappers around standard DRC-1 methods:
    name, symbol, decimals, total_supply, balance_of, transfer, approve,
    allowance.
    """

    def name(self) -> str:
        """Get the token name."""
        return str(self.view("name"))

    def symbol(self) -> str:
        """Get the token symbol."""
        return str(self.view("symbol"))

    def decimals(self) -> int:
        """Get the number of decimal places."""
        return int(self.view("decimals"))

    def total_supply(self) -> int:
        """Get the total supply in smallest units."""
        return int(self.view("total_supply"))

    def balance_of(self, owner: Address) -> int:
        """Get the token balance for an address.

        Args:
            owner: The address to query.

        Returns:
            Token balance in smallest units.
        """
        return int(self.view("balance_of", {"owner": owner}))

    def transfer(
        self,
        wallet: DinaWallet,
        to: Address,
        amount: int,
        wait: bool = False,
    ) -> Hash | TransactionReceipt:
        """Transfer tokens to another address.

        Args:
            wallet: Sender's wallet.
            to: Recipient address.
            amount: Amount in smallest units.
            wait: Wait for confirmation.

        Returns:
            Transaction hash or receipt.
        """
        return self.call("transfer", {"to": to, "amount": amount}, wallet=wallet, wait=wait)

    def approve(
        self,
        wallet: DinaWallet,
        spender: Address,
        amount: int,
        wait: bool = False,
    ) -> Hash | TransactionReceipt:
        """Approve a spender to transfer tokens on behalf of the caller.

        Args:
            wallet: Token owner's wallet.
            spender: Address to approve.
            amount: Maximum amount the spender can transfer.
            wait: Wait for confirmation.

        Returns:
            Transaction hash or receipt.
        """
        return self.call(
            "approve", {"spender": spender, "amount": amount}, wallet=wallet, wait=wait
        )

    def allowance(self, owner: Address, spender: Address) -> int:
        """Get the remaining allowance for a spender.

        Args:
            owner: Token owner address.
            spender: Approved spender address.

        Returns:
            Remaining allowance in smallest units.
        """
        return int(self.view("allowance", {"owner": owner, "spender": spender}))


class AgentWalletContract(DinaContract):
    """Helper for DRC-101 agent wallet contracts.

    DRC-101 contracts hold funds on behalf of an AI agent and expose
    methods for deposits, withdrawals, task execution, and
    operator management.
    """

    def owner(self) -> Address:
        """Get the wallet owner address."""
        return str(self.view("owner"))

    def agent_id(self) -> str:
        """Get the agent identifier."""
        return str(self.view("agent_id"))

    def balance(self) -> int:
        """Get the USDC balance held by the agent wallet."""
        return int(self.view("balance"))

    def is_operator(self, address: Address) -> bool:
        """Check if an address is an authorized operator.

        Args:
            address: The address to check.

        Returns:
            True if the address is an operator.
        """
        return bool(self.view("is_operator", {"address": address}))

    def deposit(
        self,
        wallet: DinaWallet,
        amount: int,
        wait: bool = False,
    ) -> Hash | TransactionReceipt:
        """Deposit USDC into the agent wallet.

        Args:
            wallet: Depositor's wallet.
            amount: Micro-USDC to deposit.
            wait: Wait for confirmation.

        Returns:
            Transaction hash or receipt.
        """
        return self.call("deposit", {}, wallet=wallet, usdc=amount, wait=wait)

    def withdraw(
        self,
        wallet: DinaWallet,
        amount: int,
        to: Optional[Address] = None,
        wait: bool = False,
    ) -> Hash | TransactionReceipt:
        """Withdraw USDC from the agent wallet.

        Args:
            wallet: Must be the owner or an operator.
            amount: Micro-USDC to withdraw.
            to: Destination address (defaults to wallet address).
            wait: Wait for confirmation.

        Returns:
            Transaction hash or receipt.
        """
        args: dict[str, Any] = {"amount": amount}
        if to is not None:
            args["to"] = to
        return self.call("withdraw", args, wallet=wallet, wait=wait)

    def execute_task(
        self,
        wallet: DinaWallet,
        task_id: str,
        payload: dict[str, Any],
        max_spend: int = 0,
        wait: bool = False,
    ) -> Hash | TransactionReceipt:
        """Execute an agent task, optionally spending USDC.

        Args:
            wallet: Must be the owner or an operator.
            task_id: Unique identifier for the task.
            payload: Task execution parameters.
            max_spend: Maximum micro-USDC the task can spend.
            wait: Wait for confirmation.

        Returns:
            Transaction hash or receipt.
        """
        args = {
            "task_id": task_id,
            "payload": payload,
            "max_spend": max_spend,
        }
        return self.call("execute_task", args, wallet=wallet, wait=wait)

    def add_operator(
        self,
        wallet: DinaWallet,
        operator: Address,
        wait: bool = False,
    ) -> Hash | TransactionReceipt:
        """Add an authorized operator to the agent wallet.

        Args:
            wallet: Must be the owner.
            operator: Address to authorize.
            wait: Wait for confirmation.

        Returns:
            Transaction hash or receipt.
        """
        return self.call("add_operator", {"operator": operator}, wallet=wallet, wait=wait)

    def remove_operator(
        self,
        wallet: DinaWallet,
        operator: Address,
        wait: bool = False,
    ) -> Hash | TransactionReceipt:
        """Remove an operator from the agent wallet.

        Args:
            wallet: Must be the owner.
            operator: Address to remove.
            wait: Wait for confirmation.

        Returns:
            Transaction hash or receipt.
        """
        return self.call("remove_operator", {"operator": operator}, wallet=wallet, wait=wait)
