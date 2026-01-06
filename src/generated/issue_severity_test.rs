// GENERATED TESTS FROM: issue_severity.yaml
// SPEC HASH: sha256:b3c0ee9e0a3bf438
// GENERATED: 2026-01-06T05:05:49.207555948+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod issue_severity_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_complexity_error() {
        // complexity_error: (issue_kind == 'HighComplexity') && (threshold_exceeded_by == 'Large') → "Error"
        assert_eq!(issue_severity("HighComplexity".to_string(), "Large".to_string()), "Error".to_string());
    }

    #[test]
    fn test_complexity_warn() {
        // complexity_warn: issue_kind == 'HighComplexity' → "Warning"
        assert_eq!(issue_severity("HighComplexity".to_string(), "None".to_string()), "Warning".to_string());
    }

    #[test]
    fn test_nesting() {
        // nesting: issue_kind == 'DeepNesting' → "Warning"
        assert_eq!(issue_severity("DeepNesting".to_string(), "None".to_string()), "Warning".to_string());
    }

    #[test]
    fn test_long_func() {
        // long_func: issue_kind == 'LongFunction' → "Warning"
        assert_eq!(issue_severity("LongFunction".to_string(), "None".to_string()), "Warning".to_string());
    }

    #[test]
    fn test_magic() {
        // magic: issue_kind == 'MagicNumber' → "Info"
        assert_eq!(issue_severity("MagicNumber".to_string(), "None".to_string()), "Info".to_string());
    }

    #[test]
    fn test_params() {
        // params: issue_kind == 'TooManyParams' → "Warning"
        assert_eq!(issue_severity("TooManyParams".to_string(), "None".to_string()), "Warning".to_string());
    }

    #[test]
    fn test_default() {
        // default: issue_kind == 'MissingDefault' → "Warning"
        assert_eq!(issue_severity("MissingDefault".to_string(), "None".to_string()), "Warning".to_string());
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
            fn prop_always_valid_output(issue_kind in any::<String>(), threshold_exceeded_by in any::<String>()) {
                let result = issue_severity(issue_kind, threshold_exceeded_by);
                let valid_outputs = vec!["Error".to_string(), "Info".to_string(), "Warning".to_string()];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
