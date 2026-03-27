"""Dina Network Python SDK.

Provides wallet management, RPC client, and smart contract helpers
for interacting with the Dina Network.

Quick start::

    from dina import DinaWallet, DinaClient

    wallet = DinaWallet.generate()
    client = DinaClient("https://rpc.dina.network")

    balance = client.get_balance(wallet.address)
"""

from .client import DinaClient, DinaError
from .contract import AgentWalletContract, DinaContract, TokenContract
from .types import (
    Account,
    Address,
    Block,
    Hash,
    NetworkInfo,
    TransactionReceipt,
)
from .utils import format_usdc, is_valid_address, parse_usdc
from .wallet import DinaWallet

__all__ = [
    # Client
    "DinaClient",
    "DinaError",
    # Wallet
    "DinaWallet",
    # Contracts
    "DinaContract",
    "TokenContract",
    "AgentWalletContract",
    # Types
    "Account",
    "Address",
    "Block",
    "Hash",
    "NetworkInfo",
    "TransactionReceipt",
    # Utilities
    "format_usdc",
    "parse_usdc",
    "is_valid_address",
]

__version__ = "0.1.0"
