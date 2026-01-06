// GENERATED TESTS FROM: extraction_confidence.yaml
// SPEC HASH: sha256:2a233b220bb043ae
// GENERATED: 2026-01-06T05:05:49.109684982+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod extraction_confidence_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_literal_literal() {
        // literal_literal: (pattern_type == 'Literal') && (output_type == 'Literal') && (!has_guard) → 1
        assert_eq!(extraction_confidence("Literal".to_string(), false, "Literal".to_string()), 1.0);
    }

    #[test]
    fn test_tuple_literal() {
        // tuple_literal: (pattern_type == 'Tuple') && (output_type == 'Literal') && (!has_guard) → 0.95
        assert_eq!(extraction_confidence("Tuple".to_string(), false, "Literal".to_string()), 0.95);
    }

    #[test]
    fn test_wildcard() {
        // wildcard: (pattern_type == 'Wildcard') && (output_type == 'Literal') → 0.85
        assert_eq!(extraction_confidence("Wildcard".to_string(), false, "Literal".to_string()), 0.85);
    }

    #[test]
    fn test_guarded() {
        // guarded: has_guard → 0.7
        assert_eq!(extraction_confidence("Literal".to_string(), true, "Literal".to_string()), 0.7);
    }

    #[test]
    fn test_constructor() {
        // constructor: pattern_type == 'Constructor' → 0.75
        assert_eq!(extraction_confidence("Constructor".to_string(), false, "Literal".to_string()), 0.75);
    }

    #[test]
    fn test_complex_output() {
        // complex_output: output_type == 'FunctionCall' → 0.6
        assert_eq!(extraction_confidence("Literal".to_string(), false, "FunctionCall".to_string()), 0.6);
    }

    #[test]
    fn test_complex() {
        // complex: pattern_type == 'Complex' || output_type == 'Complex' → 0.4
        assert_eq!(extraction_confidence("Complex".to_string(), false, "Complex".to_string()), 0.4);
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
            fn prop_always_valid_output(pattern_type in any::<String>(), has_guard in any::<bool>(), output_type in any::<String>()) {
                let result = extraction_confidence(pattern_type, has_guard, output_type);
                let valid_outputs = vec![0.4, 0.6, 0.7, 0.75, 0.85, 0.95, 1.0];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
