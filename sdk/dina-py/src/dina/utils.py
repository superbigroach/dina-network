"""Utility functions for the Dina Network SDK."""

from __future__ import annotations

import re

# Dina addresses: "dina1" prefix followed by 38 hex characters (total 43 chars)
_ADDRESS_PATTERN = re.compile(r"^dina1[0-9a-f]{38}$")

# USDC on Dina uses 6 decimal places (micro-USDC)
_USDC_DECIMALS = 6


def format_usdc(micro_usdc: int) -> str:
    """Convert micro-USDC (integer) to a human-readable USDC string.

    Args:
        micro_usdc: Amount in micro-USDC (1 USDC = 1_000_000 micro-USDC).

    Returns:
        Formatted string like "1.50" or "0.001234".

    Examples:
        >>> format_usdc(1_500_000)
        '1.500000'
        >>> format_usdc(0)
        '0.000000'
        >>> format_usdc(-250_000)
        '-0.250000'
    """
    if not isinstance(micro_usdc, int):
        raise TypeError(f"Expected int, got {type(micro_usdc).__name__}")

    negative = micro_usdc < 0
    abs_val = abs(micro_usdc)
    whole = abs_val // 10**_USDC_DECIMALS
    frac = abs_val % 10**_USDC_DECIMALS
    sign = "-" if negative else ""
    return f"{sign}{whole}.{frac:0{_USDC_DECIMALS}d}"


def parse_usdc(usdc_str: str) -> int:
    """Convert a USDC string to micro-USDC (integer).

    Args:
        usdc_str: A decimal string like "1.50" or "100".

    Returns:
        Amount in micro-USDC.

    Raises:
        ValueError: If the string is not a valid decimal number or has
            more than 6 decimal places.

    Examples:
        >>> parse_usdc("1.50")
        1500000
        >>> parse_usdc("100")
        100000000
    """
    usdc_str = usdc_str.strip()
    if not usdc_str:
        raise ValueError("Empty USDC string")

    negative = usdc_str.startswith("-")
    if negative:
        usdc_str = usdc_str[1:]

    parts = usdc_str.split(".")
    if len(parts) > 2:
        raise ValueError(f"Invalid USDC value: {usdc_str}")

    whole_str = parts[0]
    frac_str = parts[1] if len(parts) == 2 else ""

    if len(frac_str) > _USDC_DECIMALS:
        raise ValueError(
            f"Too many decimal places (max {_USDC_DECIMALS}): {usdc_str}"
        )

    # Validate digits
    if not whole_str.isdigit():
        raise ValueError(f"Invalid whole part: {whole_str}")
    if frac_str and not frac_str.isdigit():
        raise ValueError(f"Invalid fractional part: {frac_str}")

    # Pad fractional part to 6 digits
    frac_str = frac_str.ljust(_USDC_DECIMALS, "0")

    result = int(whole_str) * 10**_USDC_DECIMALS + int(frac_str)
    return -result if negative else result


def is_valid_address(addr: str) -> bool:
    """Check if a string is a valid Dina Network address.

    Valid addresses start with 'dina1' followed by 38 lowercase hex characters.

    Args:
        addr: The address string to validate.

    Returns:
        True if the address is valid.

    Examples:
        >>> is_valid_address("dina1" + "a" * 38)
        True
        >>> is_valid_address("invalid")
        False
    """
    if not isinstance(addr, str):
        return False
    return bool(_ADDRESS_PATTERN.match(addr))
