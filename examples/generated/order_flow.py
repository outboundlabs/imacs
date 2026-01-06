
# GENERATED FROM: order_flow.yaml
# GENERATED: 2026-01-05T17:28:46.875040450+00:00
# DO NOT EDIT - regenerate from spec

from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class OrderFlowInput:
    role: str
    verified: bool
    weight_kg: float
    zone: str
    priority: bool
    member_tier: str


@dataclass
class OrderFlowOutput:
    can_order: bool
    shipping_cost: float


@dataclass
class OrderFlowContext:
    check_access: Optional[Any] = None
    calc_shipping: Optional[Any] = None


class OrderFlowError(Exception):
    def __init__(self, step: str, error_type: str, message: str):
        self.step = step
        self.error_type = error_type
        super().__init__(message)


def order_flow(input: OrderFlowInput) -> OrderFlowOutput:
    ctx = OrderFlowContext()

    # Step: check_access (call access_level)
    check_access_input = AccessLevelInput(
        verified=input.verified,
        role=input.role
    )
    check_access_result = access_level(check_access_input)
    ctx.check_access = check_access_result

    # Gate: require_access
    if not (ctx.check_access.get('level >= 50')):
        raise OrderFlowError(
            "require_access",
            "gate_failed",
            "Gate condition failed: check_access.level >= 50"
        )

    # Step: calc_shipping (call shipping_rate)
    calc_shipping_input = ShippingRateInput(
        priority=input.priority,
        member_tier=input.member_tier,
        weight_kg=input.weight_kg,
        zone=input.zone
    )
    calc_shipping_result = shipping_rate(calc_shipping_input)
    ctx.calc_shipping = calc_shipping_result

    return OrderFlowOutput(
        can_order=None,  # TODO: map output from context
        shipping_cost=None,  # TODO: map output from context
    )