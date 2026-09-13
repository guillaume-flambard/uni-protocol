def discounted_price(cents: int, pct: int) -> int:
    """Price after a percentage discount, floor rounding."""
    return cents * (100 - pct) // 100
