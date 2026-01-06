# GENERATED TESTS FROM: order_flow.yaml
# GENERATED: 2026-01-05T17:28:46.876919843+00:00
# DO NOT EDIT — regenerate from spec

import pytest
from order_flow import order_flow, OrderFlowInput, OrderFlowError

class TestOrderFlow:
    """Tests for orchestrator happy path and gate failures"""

    def test_happy_path(self):
        """All gates pass, orchestrator completes"""
        input_data = OrderFlowInput(
            role="test",
            verified=True,
            weight_kg=10.0,
            zone="test",
            priority=True,
            member_tier="test",
        )

        result = order_flow(input_data)
        assert result is not None

    def test_gate_require_access_fails(self):
        """Gate 'require_access' rejects invalid inputs"""
        input_data = OrderFlowInput(
            role="",
            verified=False,
            weight_kg=0.0,
            zone="",
            priority=False,
            member_tier="",
        )

        with pytest.raises(OrderFlowError) as exc:
            order_flow(input_data)
        assert exc.value.step == "require_access"
        assert exc.value.error_type == "gate_failed"

