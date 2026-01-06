


// GENERATED FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-05T17:28:46.558337059+00:00
// DO NOT EDIT - regenerate from spec


pub fn access_level(role: String, verified: bool) -> i64 {



    if (role == "admin") {

        // R1
        100i64


    } else if ((role == "member") && verified) {

        // R2
        50i64


    } else if ((role == "member") && (!verified)) {

        // R3
        25i64


    } else if (role == "guest") {

        // R4
        10i64

    } else {

        unreachable!("No rule matched")

    }

}