"""Wallet implementation for the Dina Network using Ed25519 signatures."""

from __future__ import annotations

import hashlib
import os
from typing import Optional

from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)
from cryptography.hazmat.primitives.serialization import (
    Encoding,
    NoEncryption,
    PrivateFormat,
    PublicFormat,
)

from .types import Address


class DinaWallet:
    """Ed25519-based wallet for signing Dina Network transactions.

    Create wallets via the class methods:
        wallet = DinaWallet.generate()
        wallet = DinaWallet.from_private_key("abcd1234...")
    """

    def __init__(self, private_key: Ed25519PrivateKey) -> None:
        self._private_key = private_key
        self._public_key = private_key.public_key()
        self._address: Optional[Address] = None

    # ------------------------------------------------------------------ #
    # Factory methods
    # ------------------------------------------------------------------ #

    @classmethod
    def generate(cls) -> DinaWallet:
        """Generate a new random wallet.

        Returns:
            A new DinaWallet with a freshly generated Ed25519 keypair.
        """
        private_key = Ed25519PrivateKey.generate()
        return cls(private_key)

    @classmethod
    def from_private_key(cls, hex_key: str) -> DinaWallet:
        """Restore a wallet from a hex-encoded 32-byte private seed.

        Args:
            hex_key: 64-character hex string representing the 32-byte seed.

        Returns:
            A DinaWallet derived from the given private key.

        Raises:
            ValueError: If the hex string is invalid or the wrong length.
        """
        hex_key = hex_key.strip().lower()
        if len(hex_key) != 64:
            raise ValueError(
                f"Private key must be 64 hex characters (32 bytes), got {len(hex_key)}"
            )
        try:
            seed_bytes = bytes.fromhex(hex_key)
        except ValueError as exc:
            raise ValueError(f"Invalid hex in private key: {exc}") from exc

        private_key = Ed25519PrivateKey.from_private_bytes(seed_bytes)
        return cls(private_key)

    # ------------------------------------------------------------------ #
    # Properties
    # ------------------------------------------------------------------ #

    @property
    def address(self) -> Address:
        """Derive the Dina address from the public key.

        The address is 'dina1' followed by the first 38 hex characters of
        SHA-256(public_key_bytes).
        """
        if self._address is None:
            pub_bytes = self._public_key.public_bytes(
                Encoding.Raw, PublicFormat.Raw
            )
            digest = hashlib.sha256(pub_bytes).hexdigest()
            self._address = f"dina1{digest[:38]}"
        return self._address

    @property
    def public_key_hex(self) -> str:
        """Return the 32-byte public key as a hex string."""
        pub_bytes = self._public_key.public_bytes(
            Encoding.Raw, PublicFormat.Raw
        )
        return pub_bytes.hex()

    @property
    def private_key_hex(self) -> str:
        """Return the 32-byte private seed as a hex string."""
        raw = self._private_key.private_bytes(
            Encoding.Raw, PrivateFormat.Raw, NoEncryption()
        )
        return raw.hex()

    # ------------------------------------------------------------------ #
    # Signing / verification
    # ------------------------------------------------------------------ #

    def sign(self, message: bytes) -> bytes:
        """Sign a message using Ed25519.

        Args:
            message: The raw bytes to sign.

        Returns:
            64-byte Ed25519 signature.
        """
        if not isinstance(message, bytes):
            raise TypeError(f"message must be bytes, got {type(message).__name__}")
        return self._private_key.sign(message)

    def verify(self, message: bytes, signature: bytes) -> bool:
        """Verify an Ed25519 signature against this wallet's public key.

        Args:
            message: The original message bytes.
            signature: The 64-byte signature to verify.

        Returns:
            True if the signature is valid, False otherwise.
        """
        if not isinstance(message, bytes):
            raise TypeError(f"message must be bytes, got {type(message).__name__}")
        if not isinstance(signature, bytes):
            raise TypeError(f"signature must be bytes, got {type(signature).__name__}")
        try:
            self._public_key.verify(signature, message)
            return True
        except Exception:
            return False

    # ------------------------------------------------------------------ #
    # Dunder methods
    # ------------------------------------------------------------------ #

    def __repr__(self) -> str:
        return f"DinaWallet(address={self.address!r})"

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, DinaWallet):
            return NotImplemented
        return self.private_key_hex == other.private_key_hex
