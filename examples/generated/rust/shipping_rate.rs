// GENERATED FROM: shipping_rate.yaml
// SPEC HASH: sha256:bfd80b5a15c6208e
// GENERATED: 2026-01-02T15:13:53.569037982+00:00
// DO NOT EDIT — regenerate from spec

pub fn shipping_rate(weight_kg: f64, zone: String, priority: bool, member_tier: String) -> f64 {
    if ((member_tier == "gold") && (zone == "domestic")) {
        // R1
        0.0
    } else if (priority && (zone == "international")) {
        // R2
        ((weight_kg * 25.0) + 50.0)
    } else if (priority && (zone == "north_america")) {
        // R3
        ((weight_kg * 15.0) + 20.0)
    } else if (priority && (zone == "domestic")) {
        // R4
        ((weight_kg * 8.0) + 10.0)
    } else if ((member_tier == "silver") && (zone == "international")) {
        // R5
        ((weight_kg * 16.0) + 30.0)
    } else if ((member_tier == "silver") && (zone == "north_america")) {
        // R6
        ((weight_kg * 8.0) + 12.0)
    } else if ((member_tier == "silver") && (zone == "domestic")) {
        // R7
        ((weight_kg * 4.0) + 5.0)
    } else if (zone == "international") {
        // R8
        ((weight_kg * 20.0) + 40.0)
    } else if (zone == "north_america") {
        // R9
        ((weight_kg * 10.0) + 15.0)
    } else if (zone == "domestic") {
        // R10
        ((weight_kg * 5.0) + 7.0)
    } else {
        unreachable!("No rule matched")
    }
}

