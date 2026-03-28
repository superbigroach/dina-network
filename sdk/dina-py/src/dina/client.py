"""HTTP client for interacting with the Dina Network RPC endpoint."""

from __future__ import annotations

import hashlib
import json
import time
from typing import Any, Optional

import httpx

from .types import (
    Account,
    Address,
    Block,
    Hash,
    NetworkInfo,
    TransactionReceipt,
)
from .wallet import DinaWallet


class DinaError(Exception):
    """Raised when an RPC call returns an error."""

    def __init__(self, message: str, code: Optional[int] = None) -> None:
        super().__init__(message)
        self.code = code


class DinaClient:
    """Client for the Dina Network JSON-RPC API.

    Args:
        rpc_url: Base URL of the Dina Network RPC node
            (e.g. "https://rpc.dina.network").
        timeout: Request timeout in seconds (default 30).
    """

    def __init__(self, rpc_url: str, *, timeout: float = 30.0) -> None:
        self._rpc_url = rpc_url.rstrip("/")
        self._client = httpx.Client(
            base_url=self._rpc_url,
            timeout=timeout,
            headers={"Content-Type": "application/json"},
        )
        self._request_id = 0

    # ------------------------------------------------------------------ #
    # Low-level RPC
    # ------------------------------------------------------------------ #

    def _next_id(self) -> int:
        self._request_id += 1
        return self._request_id

    def _rpc(self, method: str, params: Optional[list[Any]] = None) -> Any:
        """Send a JSON-RPC 2.0 request and return the result.

        Args:
            method: The RPC method name (e.g. ``"dina_getBalance"``).
            params: Positional parameters as a list (JSON array), matching
                the server's expected positional args.

        Raises:
            DinaError: If the RPC response contains an error field.
            httpx.HTTPStatusError: On non-2xx HTTP status.
        """
        payload = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": method,
            "params": params or [],
        }
        response = self._client.post("/rpc", json=payload)
        response.raise_for_status()
        body = response.json()

        if "error" in body and body["error"] is not None:
            err = body["error"]
            msg = err.get("message", str(err)) if isinstance(err, dict) else str(err)
            code = err.get("code") if isinstance(err, dict) else None
            raise DinaError(msg, code=code)

        return body.get("result")

    # ------------------------------------------------------------------ #
    # Account queries
    # ------------------------------------------------------------------ #

    def get_balance(self, address: Address) -> int:
        """Get the balance of an address in micro-USDC.

        Args:
            address: The Dina address to query.

        Returns:
            The balance as an integer (micro-USDC).
        """
        result = self._rpc("dina_getBalance", [address])
        return int(result)

    def get_account(self, address: Address) -> Account:
        """Get full account information.

        Args:
            address: The Dina address to query.

        Returns:
            An Account dataclass with balance, nonce, etc.
        """
        result = self._rpc("dina_getAccount", [address])
        return Account.from_dict(result)

    # ------------------------------------------------------------------ #
    # Block queries
    # ------------------------------------------------------------------ #

    def get_block(self, height: int) -> Block:
        """Get a block by height.

        Args:
            height: The block number.

        Returns:
            A Block dataclass.
        """
        result = self._rpc("dina_getBlock", [height])
        return Block.from_dict(result)

    def get_latest_block(self) -> Block:
        """Get the most recent block.

        Returns:
            A Block dataclass for the latest block.
        """
        result = self._rpc("dina_getLatestBlock", [])
        return Block.from_dict(result)

    # ------------------------------------------------------------------ #
    # Transactions
    # ------------------------------------------------------------------ #

    def _build_and_sign_tx(
        self,
        wallet: DinaWallet,
        tx_type: str,
        body: dict[str, Any],
    ) -> dict[str, Any]:
        """Build a transaction envelope, sign it, and return the signed payload."""
        # Fetch the sender's current nonce
        account = self.get_account(wallet.address)
        nonce = account.nonce

        tx = {
            "type": tx_type,
            "sender": wallet.address,
            "nonce": nonce,
            "body": body,
        }

        # Canonical JSON encoding for signing
        tx_bytes = json.dumps(tx, sort_keys=True, separators=(",", ":")).encode("utf-8")
        signature = wallet.sign(tx_bytes)

        return {
            "tx": tx,
            "signature": signature.hex(),
            "public_key": wallet.public_key_hex,
        }

    def transfer(
        self,
        wallet: DinaWallet,
        to: Address,
        amount: int,
        memo: Optional[str] = None,
    ) -> Hash:
        """Send USDC to another address.

        Args:
            wallet: The sender's wallet (used for signing).
            to: Destination address.
            amount: Amount in micro-USDC.
            memo: Optional memo string attached to the transaction.

        Returns:
            The transaction hash.
        """
        body: dict[str, Any] = {"to": to, "amount": amount}
        if memo is not None:
            body["memo"] = memo

        signed = self._build_and_sign_tx(wallet, "transfer", body)
        tx_hex = json.dumps(signed, sort_keys=True, separators=(",", ":"))
        result = self._rpc("dina_sendTransaction", [tx_hex])
        return str(result)

    def deploy_contract(
        self,
        wallet: DinaWallet,
        wasm_bytes: bytes,
        init_args: dict[str, Any],
    ) -> Hash:
        """Deploy a WASM smart contract.

        Args:
            wallet: The deployer's wallet.
            wasm_bytes: Compiled WASM binary.
            init_args: Arguments passed to the contract's init function.

        Returns:
            The transaction hash (the contract address is in the receipt).
        """
        body = {
            "code": wasm_bytes.hex(),
            "init_args": init_args,
        }
        signed = self._build_and_sign_tx(wallet, "deploy_contract", body)
        tx_hex = json.dumps(signed, sort_keys=True, separators=(",", ":"))
        result = self._rpc("dina_sendTransaction", [tx_hex])
        return str(result)

    def call_contract(
        self,
        wallet: DinaWallet,
        contract: Address,
        method: str,
        args: dict[str, Any],
        usdc: int = 0,
    ) -> Hash:
        """Call a method on a deployed smart contract.

        Args:
            wallet: The caller's wallet.
            contract: The contract address.
            method: The method name to invoke.
            args: Arguments for the method call.
            usdc: Amount of micro-USDC to send with the call (default 0).

        Returns:
            The transaction hash.
        """
        body: dict[str, Any] = {
            "contract": contract,
            "method": method,
            "args": args,
        }
        if usdc > 0:
            body["usdc"] = usdc

        signed = self._build_and_sign_tx(wallet, "call_contract", body)
        tx_hex = json.dumps(signed, sort_keys=True, separators=(",", ":"))
        result = self._rpc("dina_sendTransaction", [tx_hex])
        return str(result)

    # ------------------------------------------------------------------ #
    # Transaction receipt
    # ------------------------------------------------------------------ #

    def get_transaction_receipt(self, tx_hash: Hash) -> Optional[TransactionReceipt]:
        """Get the receipt for a transaction.

        Args:
            tx_hash: The transaction hash.

        Returns:
            A TransactionReceipt if found, or None if pending/unknown.
        """
        try:
            result = self._rpc("dina_getTransaction", [tx_hash])
        except DinaError:
            return None
        if result is None:
            return None
        return TransactionReceipt.from_dict(result)

    def wait_for_transaction(
        self,
        tx_hash: Hash,
        timeout: float = 30.0,
        poll_interval: float = 1.0,
    ) -> TransactionReceipt:
        """Poll until a transaction is confirmed or the timeout is reached.

        Args:
            tx_hash: The transaction hash to wait for.
            timeout: Maximum seconds to wait (default 30).
            poll_interval: Seconds between polls (default 1).

        Returns:
            The TransactionReceipt once confirmed.

        Raises:
            TimeoutError: If the transaction is not confirmed within the timeout.
        """
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            receipt = self.get_transaction_receipt(tx_hash)
            if receipt is not None:
                return receipt
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                break
            time.sleep(min(poll_interval, remaining))

        raise TimeoutError(
            f"Transaction {tx_hash} not confirmed within {timeout}s"
        )

    # ------------------------------------------------------------------ #
    # Network info
    # ------------------------------------------------------------------ #

    def get_network_info(self) -> NetworkInfo:
        """Get general information about the Dina Network.

        Returns:
            A NetworkInfo dataclass.
        """
        result = self._rpc("dina_networkInfo", [])
        return NetworkInfo.from_dict(result)

    # ------------------------------------------------------------------ #
    # Lifecycle
    # ------------------------------------------------------------------ #

    def close(self) -> None:
        """Close the underlying HTTP client."""
        self._client.close()

    def __enter__(self) -> DinaClient:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()

    def __repr__(self) -> str:
        return f"DinaClient(rpc_url={self._rpc_url!r})"
