// GENERATED FROM: extraction_confidence.yaml
// SPEC HASH: sha256:01e8fcbe09b45d77
// GENERATED: 2026-01-02T13:32:40.358346504+00:00
// DO NOT EDIT — regenerate from spec

pub fn extraction_confidence(pattern_type: String, has_guard: bool, output_type: String) -> f64 {
    if ((pattern_type == "Literal") && (output_type == "Literal")) && (!has_guard) {
        // literal_literal
        1.0
    } else if ((pattern_type == "Tuple") && (output_type == "Literal")) && (!has_guard) {
        // tuple_literal
        0.95
    } else if (pattern_type == "Wildcard") && (output_type == "Literal") {
        // wildcard
        0.85
    } else if has_guard {
        // guarded
        0.7
    } else if pattern_type == "Constructor" {
        // constructor
        0.75
    } else if output_type == "FunctionCall" {
        // complex_output
        0.6
    } else if (pattern_type == "Complex") || (output_type == "Complex") {
        // complex
        0.4
    } else {
        unreachable!("No rule matched")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // GENERATED TESTS FROM: extraction_confidence.yaml
    // SPEC HASH: sha256:01e8fcbe09b45d77
    // GENERATED: 2026-01-02T13:32:40.418793953+00:00
    // DO NOT EDIT — regenerate from spec

    #[cfg(test)]
    mod extraction_confidence_tests {
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
                fn prop_always_valid_output(pattern_type in any::<bool>(), has_guard in any::<bool>(), output_type in any::<bool>()) {
                    let result = extraction_confidence(pattern_type, has_guard, output_type);
                    prop_assert!([0.4, 0.6, 0.7, 0.75, 0.85, 0.95, 1.0].contains(&result));
                }
            }
        }
    }
}
