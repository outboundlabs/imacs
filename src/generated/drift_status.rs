// GENERATED FROM: drift_status.yaml
// SPEC HASH: sha256:a2bcc289cfaf6a16
// GENERATED: 2026-01-02T13:32:40.305408696+00:00
// DO NOT EDIT — regenerate from spec

pub fn drift_status(error_count: i64, warning_count: i64, comparable: bool) -> String {
    if !comparable {
        // incomparable
        "Incomparable".to_string()
    } else if comparable && (error_count > 0) {
        // major
        "MajorDrift".to_string()
    } else if (comparable && (error_count == 0)) && (warning_count > 0) {
        // minor
        "MinorDrift".to_string()
    } else if (comparable && (error_count == 0)) && (warning_count == 0) {
        // synced
        "Synced".to_string()
    } else {
        unreachable!("No rule matched")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // GENERATED TESTS FROM: drift_status.yaml
    // SPEC HASH: sha256:a2bcc289cfaf6a16
    // GENERATED: 2026-01-02T13:32:40.342580661+00:00
    // DO NOT EDIT — regenerate from spec

    #[cfg(test)]
    mod drift_status_tests {
        use super::*;

        // ═══════════════════════════════════════════════════════════════
        // Rule tests (one per rule)
        // ═══════════════════════════════════════════════════════════════

        // ═══════════════════════════════════════════════════════════════
        // Property tests
        // ═══════════════════════════════════════════════════════════════

        #[cfg(feature = "proptest")]
        mod property_tests {
            use super::*;
            use proptest::prelude::*;

            proptest! {
                #[test]
                fn prop_always_valid_output(error_count in any::<i64>(), warning_count in any::<i64>(), comparable in any::<bool>()) {
                    let result = drift_status(error_count, warning_count, comparable);
                    prop_assert!(["Incomparable".to_string(), "MajorDrift".to_string(), "MinorDrift".to_string(), "Synced".to_string()].contains(&result));
                }
            }
        }
    }
}
