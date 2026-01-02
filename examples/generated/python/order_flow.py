# Generated orchestrator: order_flow # DO NOT EDIT from dataclasses import dataclass from typing import Any, Optional import asyncio from .access_level import access_level

from .shipping_rate import shipping_rate

 @dataclass class OrderFlowInput: role: str

verified: bool

weight_kg: float

zone: str

priority: bool

member_tier: str

 @dataclass class OrderFlowOutput: can_order: bool

shipping_cost: float

 async def order_flow(input: OrderFlowInput) -> OrderFlowOutput: ctx = {} # check_access ctx["check_access"] = await access_level( role=input.role,

verified=input.verified, )

if not (ctx['check_access']['level >= 50']): raise ValueError("Gate require_access failed")

# calc_shipping ctx["calc_shipping"] = await shipping_rate( zone=input.zone,

priority=input.priority,

member_tier=input.member_tier,

weight_kg=input.weight_kg, ) return OrderFlowOutput( can_order=None, # TODO

shipping_cost=None, # TODO )
