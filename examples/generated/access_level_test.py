# GENERATED TESTS FROM: access_level.yaml
# SPEC HASH: sha256:61f180f99fb26ed2
# GENERATED: 2026-01-05T17:28:46.656201213+00:00
# DO NOT EDIT — regenerate from spec

import pytest
from access_level import access_level

class TestAccessLevelRules:
    """One test per rule"""

    def test_r1(self):
        # R1: role == 'admin' → 100
        assert access_level("admin", False) == 100

    def test_r2(self):
        # R2: role == 'member' && verified → 50
        assert access_level("member", True) == 50

    def test_r3(self):
        # R3: role == 'member' && !verified → 25
        assert access_level("member", False) == 25

    def test_r4(self):
        # R4: role == 'guest' → 10
        assert access_level("guest", False) == 10

