


# GENERATED FROM: access_level.yaml
# SPEC HASH: sha256:61f180f99fb26ed2
# GENERATED: 2026-01-05T17:28:46.656091041+00:00
# DO NOT EDIT - regenerate from spec


from dataclasses import dataclass
from typing import Any


@dataclass
class AccessLevelInput:

    role: str

    verified: bool




def access_level(input: AccessLevelInput) -> int:

    role = input.role

    verified = input.verified




    if (role == "admin"):

        # R1
        return 100


    elif ((role == "member") and verified):

        # R2
        return 50


    elif ((role == "member") and (not verified)):

        # R3
        return 25


    elif (role == "guest"):

        # R4
        return 10

    else:

        raise ValueError("No rule matched")
