// GENERATED FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-02T15:13:53.879200899+00:00
// DO NOT EDIT — regenerate from spec

pub fn access_level(role: String, verified: bool) -> i64 {
    if (role == "admin") {
        // R1
        100
    } else if ((role == "member") && verified) {
        // R2
        50
    } else if ((role == "member") && (!verified)) {
        // R3
        25
    } else if (role == "guest") {
        // R4
        10
    } else {
        unreachable!("No rule matched")
    }
}

