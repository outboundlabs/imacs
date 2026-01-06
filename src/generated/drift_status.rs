// GENERATED FROM: drift_status.yaml
// SPEC HASH: sha256:238924e827da9055
// GENERATED: 2026-01-06T05:05:49.060510221+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
pub fn drift_status(error_count: i64, warning_count: i64, comparable: bool) -> String {
    if (!comparable) {
        // incomparable
        "Incomparable".to_string()
    } else if (comparable && (error_count > 0)) {
        // major
        "MajorDrift".to_string()
    } else if ((comparable && (error_count == 0)) && (warning_count > 0)) {
        // minor
        "MinorDrift".to_string()
    } else if ((comparable && (error_count == 0)) && (warning_count == 0)) {
        // synced
        "Synced".to_string()
    } else {
        unreachable!("No rule matched")
    }
}
