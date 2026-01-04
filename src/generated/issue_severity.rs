// GENERATED FROM: issue_severity.yaml
// SPEC HASH: sha256:a234932ea9db0d8a
// GENERATED: 2026-01-02T13:32:40.485937166+00:00
// DO NOT EDIT — regenerate from spec

pub fn issue_severity(issue_kind: String, threshold_exceeded_by: String) -> String {
    if (issue_kind == "HighComplexity") && (threshold_exceeded_by == "Large") {
        // complexity_error
        "Error".to_string()
    } else if issue_kind == "HighComplexity" {
        // complexity_warn
        "Warning".to_string()
    } else if issue_kind == "DeepNesting" {
        // nesting
        "Warning".to_string()
    } else if issue_kind == "LongFunction" {
        // long_func
        "Warning".to_string()
    } else if issue_kind == "MagicNumber" {
        // magic
        "Info".to_string()
    } else if issue_kind == "TooManyParams" {
        // params
        "Warning".to_string()
    } else if issue_kind == "MissingDefault" {
        // default
        "Warning".to_string()
    } else {
        unreachable!("No rule matched")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // GENERATED TESTS FROM: issue_severity.yaml
    // SPEC HASH: sha256:a234932ea9db0d8a
    // GENERATED: 2026-01-02T13:32:40.557581389+00:00
    // DO NOT EDIT — regenerate from spec

    #[cfg(test)]
    mod issue_severity_tests {
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
                fn prop_always_valid_output(issue_kind in any::<bool>(), threshold_exceeded_by in any::<bool>()) {
                    let result = issue_severity(issue_kind, threshold_exceeded_by);
                    prop_assert!(["Error".to_string(), "Info".to_string(), "Warning".to_string()].contains(&result));
                }
            }
        }
    }
}
