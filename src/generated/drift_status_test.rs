// GENERATED TESTS FROM: drift_status.yaml
// SPEC HASH: sha256:238924e827da9055
// GENERATED: 2026-01-06T05:05:49.060615792+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod drift_status_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_incomparable() {
        // incomparable: !comparable → "Incomparable"
        assert_eq!(drift_status(0, 0, false), "Incomparable".to_string());
    }

    #[test]
    fn test_major() {
        // major: (comparable) && (error_count > 0) → "MajorDrift"
        assert_eq!(drift_status(0, 0, true), "MajorDrift".to_string());
    }

    #[test]
    fn test_minor() {
        // minor: (comparable) && (error_count == 0) && (warning_count > 0) → "MinorDrift"
        assert_eq!(drift_status(0, 0, true), "MinorDrift".to_string());
    }

    #[test]
    fn test_synced() {
        // synced: (comparable) && (error_count == 0) && (warning_count == 0) → "Synced"
        assert_eq!(drift_status(0, 0, true), "Synced".to_string());
    }

    // ═══════════════════════════════════════════════════════════════
    // Property tests
    // ═══════════════════════════════════════════════════════════════

    #[cfg(feature = "proptest")]
    mod property_tests {
        #[allow(unused_imports)]
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_always_valid_output(error_count in any::<i64>(), warning_count in any::<i64>(), comparable in any::<bool>()) {
                let result = drift_status(error_count, warning_count, comparable);
                let valid_outputs = vec!["Incomparable".to_string(), "MajorDrift".to_string(), "MinorDrift".to_string(), "Synced".to_string()];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
