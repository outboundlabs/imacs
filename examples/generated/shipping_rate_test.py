# GENERATED TESTS FROM: shipping_rate.yaml
# SPEC HASH: sha256:bfd80b5a15c6208e
# GENERATED: 2026-01-05T17:28:47.305099468+00:00
# DO NOT EDIT — regenerate from spec

import pytest
from shipping_rate import shipping_rate

class TestShippingRateRules:
    """One test per rule"""

    def test_r1(self):
        # R1: member_tier == 'gold' && zone == 'domestic' → 0
        assert shipping_rate(0.0, "domestic", False, "gold") == 0

    def test_r2(self):
        # R2: priority && zone == 'international' → "weight_kg * 25.0 + 50.0"
        assert shipping_rate(0.0, "international", True, "") == "weight_kg * 25.0 + 50.0"

    def test_r3(self):
        # R3: priority && zone == 'north_america' → "weight_kg * 15.0 + 20.0"
        assert shipping_rate(0.0, "north_america", True, "") == "weight_kg * 15.0 + 20.0"

    def test_r4(self):
        # R4: priority && zone == 'domestic' → "weight_kg * 8.0 + 10.0"
        assert shipping_rate(0.0, "domestic", True, "") == "weight_kg * 8.0 + 10.0"

    def test_r5(self):
        # R5: member_tier == 'silver' && zone == 'international' → "weight_kg * 16.0 + 30.0"
        assert shipping_rate(0.0, "international", False, "silver") == "weight_kg * 16.0 + 30.0"

    def test_r6(self):
        # R6: member_tier == 'silver' && zone == 'north_america' → "weight_kg * 8.0 + 12.0"
        assert shipping_rate(0.0, "north_america", False, "silver") == "weight_kg * 8.0 + 12.0"

    def test_r7(self):
        # R7: member_tier == 'silver' && zone == 'domestic' → "weight_kg * 4.0 + 5.0"
        assert shipping_rate(0.0, "domestic", False, "silver") == "weight_kg * 4.0 + 5.0"

    def test_r8(self):
        # R8: zone == 'international' → "weight_kg * 20.0 + 40.0"
        assert shipping_rate(0.0, "international", False, "") == "weight_kg * 20.0 + 40.0"

    def test_r9(self):
        # R9: zone == 'north_america' → "weight_kg * 10.0 + 15.0"
        assert shipping_rate(0.0, "north_america", False, "") == "weight_kg * 10.0 + 15.0"

    def test_r10(self):
        # R10: zone == 'domestic' → "weight_kg * 5.0 + 7.0"
        assert shipping_rate(0.0, "domestic", False, "") == "weight_kg * 5.0 + 7.0"

