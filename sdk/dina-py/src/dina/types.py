"""Core types for the Dina Network SDK."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional


# Type aliases
Address = str
Hash = str


@dataclass(frozen=True)
class Account:
    """Represents an account on the Dina Network."""

    address: Address
    balance: int
    nonce: int
    is_contract: bool = False
    code_hash: Optional[Hash] = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Account:
        return cls(
            address=data["address"],
            balance=int(data["balance"]),
            nonce=int(data["nonce"]),
            is_contract=data.get("is_contract", False),
            code_hash=data.get("code_hash"),
        )


@dataclass(frozen=True)
class Block:
    """Represents a block on the Dina Network."""

    height: int
    hash: Hash
    previous_hash: Hash
    timestamp: int
    validator: Address
    transaction_count: int
    transactions: list[Hash] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Block:
        return cls(
            height=int(data["height"]),
            hash=data["hash"],
            previous_hash=data["previous_hash"],
            timestamp=int(data["timestamp"]),
            validator=data["validator"],
            transaction_count=int(data["transaction_count"]),
            transactions=data.get("transactions", []),
        )


@dataclass(frozen=True)
class TransactionReceipt:
    """Receipt returned after a transaction is confirmed."""

    tx_hash: Hash
    block_height: int
    status: str  # "success" or "failed"
    gas_used: int
    sender: Address
    receiver: Address
    amount: int
    error: Optional[str] = None
    logs: list[dict[str, Any]] = field(default_factory=list)

    @property
    def succeeded(self) -> bool:
        return self.status == "success"

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> TransactionReceipt:
        return cls(
            tx_hash=data["tx_hash"],
            block_height=int(data["block_height"]),
            status=data["status"],
            gas_used=int(data["gas_used"]),
            sender=data["sender"],
            receiver=data["receiver"],
            amount=int(data.get("amount", 0)),
            error=data.get("error"),
            logs=data.get("logs", []),
        )


@dataclass(frozen=True)
class NetworkInfo:
    """Information about the Dina Network."""

    chain_id: str
    latest_block_height: int
    node_count: int
    version: str
    epoch: int
    total_supply: int
    circulating_supply: int

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> NetworkInfo:
        return cls(
            chain_id=data["chain_id"],
            latest_block_height=int(data["latest_block_height"]),
            node_count=int(data["node_count"]),
            version=data["version"],
            epoch=int(data["epoch"]),
            total_supply=int(data["total_supply"]),
            circulating_supply=int(data["circulating_supply"]),
        )
