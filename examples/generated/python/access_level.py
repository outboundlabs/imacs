# GENERATED FROM: access_level.yaml
# SPEC HASH: sha256:61f180f99fb26ed2
# GENERATED: 2026-01-02T15:13:54.938002203+00:00
# DO NOT EDIT — regenerate from spec

def access_level(role: str, verified: bool) -> int:
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

