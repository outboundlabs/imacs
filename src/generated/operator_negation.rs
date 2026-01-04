// GENERATED FROM: operator_negation.yaml
// SPEC HASH: sha256:dfd97b9916af3acb
// GENERATED: 2026-01-02T13:32:40.829376348+00:00
// DO NOT EDIT — regenerate from spec

pub fn operator_negation(op: String) -> String {
    if op == "Eq" {
        // eq_to_ne
        "Ne".to_string()
    } else if op == "Ne" {
        // ne_to_eq
        "Eq".to_string()
    } else if op == "Lt" {
        // lt_to_ge
        "Ge".to_string()
    } else if op == "Le" {
        // le_to_gt
        "Gt".to_string()
    } else if op == "Gt" {
        // gt_to_le
        "Le".to_string()
    } else if op == "Ge" {
        // ge_to_lt
        "Lt".to_string()
    } else {
        unreachable!("No rule matched")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // GENERATED TESTS FROM: operator_negation.yaml
    // SPEC HASH: sha256:dfd97b9916af3acb
    // GENERATED: 2026-01-02T13:32:40.882474255+00:00
    // DO NOT EDIT — regenerate from spec

    #[cfg(test)]
    mod operator_negation_tests {
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
                fn prop_always_valid_output(op in any::<bool>()) {
                    let result = operator_negation(op);
                    prop_assert!(["Eq".to_string(), "Ge".to_string(), "Gt".to_string(), "Le".to_string(), "Lt".to_string(), "Ne".to_string()].contains(&result));
                }
            }
        }
    }
}
