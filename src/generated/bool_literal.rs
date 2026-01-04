// GENERATED FROM: bool_literal.yaml
// SPEC HASH: sha256:20bb55a7928f1721
// GENERATED: 2026-01-02T13:32:39.649037939+00:00
// DO NOT EDIT — regenerate from spec

pub fn bool_literal(value: bool, target: String) -> String {
    if (value) && (target == "Python") {
        // true_py
        "True".to_string()
    } else if (!value) && (target == "Python") {
        // false_py
        "False".to_string()
    } else if value {
        // true_default
        "true".to_string()
    } else if !value {
        // false_default
        "false".to_string()
    } else {
        unreachable!("No rule matched")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // GENERATED TESTS FROM: bool_literal.yaml
    // SPEC HASH: sha256:20bb55a7928f1721
    // GENERATED: 2026-01-02T13:32:39.741393553+00:00
    // DO NOT EDIT — regenerate from spec

    #[cfg(test)]
    mod bool_literal_tests {
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
                fn prop_always_valid_output(value in any::<bool>(), target in any::<bool>()) {
                    let result = bool_literal(value, target);
                    prop_assert!(["False".to_string(), "True".to_string(), "false".to_string(), "true".to_string()].contains(&result));
                }
            }
        }
    }
}
