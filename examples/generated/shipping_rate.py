


# GENERATED FROM: shipping_rate.yaml
# SPEC HASH: sha256:bfd80b5a15c6208e
# GENERATED: 2026-01-05T17:28:47.304968899+00:00
# DO NOT EDIT - regenerate from spec


from dataclasses import dataclass
from typing import Any


@dataclass
class ShippingRateInput:

    weight_kg: float

    zone: str

    priority: bool

    member_tier: str




def shipping_rate(input: ShippingRateInput) -> float:

    weight_kg = input.weight_kg

    zone = input.zone

    priority = input.priority

    member_tier = input.member_tier




    if ((member_tier == "gold") and (zone == "domestic")):

        # R1
        return 0.0


    elif (priority and (zone == "international")):

        # R2
        return ((weight_kg * 25.0) + 50.0)


    elif (priority and (zone == "north_america")):

        # R3
        return ((weight_kg * 15.0) + 20.0)


    elif (priority and (zone == "domestic")):

        # R4
        return ((weight_kg * 8.0) + 10.0)


    elif ((member_tier == "silver") and (zone == "international")):

        # R5
        return ((weight_kg * 16.0) + 30.0)


    elif ((member_tier == "silver") and (zone == "north_america")):

        # R6
        return ((weight_kg * 8.0) + 12.0)


    elif ((member_tier == "silver") and (zone == "domestic")):

        # R7
        return ((weight_kg * 4.0) + 5.0)


    elif (zone == "international"):

        # R8
        return ((weight_kg * 20.0) + 40.0)


    elif (zone == "north_america"):

        # R9
        return ((weight_kg * 10.0) + 15.0)


    elif (zone == "domestic"):

        # R10
        return ((weight_kg * 5.0) + 7.0)

    else:

        raise ValueError("No rule matched")
