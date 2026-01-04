// GENERATED FROM: operator_mapping.yaml
// SPEC HASH: sha256:0839ac716c87f4ad
// GENERATED: 2026-01-02T13:32:40.631527392+00:00
// DO NOT EDIT — regenerate from spec

pub fn operator_mapping(op: String, target: String) -> String {
    if (op == "Eq") && (target == "TypeScript") {
        // eq_ts
        "===".to_string()
    } else if op == "Eq" {
        // eq_default
        "==".to_string()
    } else if (op == "Ne") && (target == "TypeScript") {
        // ne_ts
        "!==".to_string()
    } else if op == "Ne" {
        // ne_default
        "!=".to_string()
    } else if op == "Lt" {
        // lt
        "<".to_string()
    } else if op == "Le" {
        // le
        "<=".to_string()
    } else if op == "Gt" {
        // gt
        ">".to_string()
    } else if op == "Ge" {
        // ge
        ">=".to_string()
    } else if (op == "And") && (target == "Python") {
        // and_py
        "and".to_string()
    } else if op == "And" {
        // and_default
        "&&".to_string()
    } else if (op == "Or") && (target == "Python") {
        // or_py
        "or".to_string()
    } else if op == "Or" {
        // or_default
        "||".to_string()
    } else if (op == "Not") && (target == "Python") {
        // not_py
        "not ".to_string()
    } else if op == "Not" {
        // not_default
        "!".to_string()
    } else if (op == "In") && (target == "Rust") {
        // in_rust
        ".contains(&{})".to_string()
    } else if (op == "In") && (target == "TypeScript") {
        // in_ts
        ".includes({})".to_string()
    } else if (op == "In") && (target == "Python") {
        // in_py
        " in ".to_string()
    } else if (op == "In") && (target == "CSharp") {
        // in_csharp
        ".Contains({})".to_string()
    } else if (op == "In") && (target == "Java") {
        // in_java
        ".contains({})".to_string()
    } else if (op == "In") && (target == "Go") {
        // in_go
        "contains({}, {})".to_string()
    } else {
        unreachable!("No rule matched")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // GENERATED TESTS FROM: operator_mapping.yaml
    // SPEC HASH: sha256:0839ac716c87f4ad
    // GENERATED: 2026-01-02T13:32:40.822752505+00:00
    // DO NOT EDIT — regenerate from spec

    #[cfg(test)]
    mod operator_mapping_tests {
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
                fn prop_always_valid_output(op in any::<bool>(), target in any::<bool>()) {
                    let result = operator_mapping(op, target);
                    prop_assert!([" in ".to_string(), "!".to_string(), "!=".to_string(), "!==".to_string(), "&&".to_string(), ".Contains({})".to_string(), ".contains(&{})".to_string(), ".contains({})".to_string(), ".includes({})".to_string(), "<".to_string(), "<=".to_string(), "==".to_string(), "===".to_string(), ">".to_string(), ">=".to_string(), "and".to_string(), "contains({}, {})".to_string(), "not ".to_string(), "or".to_string(), "||".to_string()].contains(&result));
                }
            }
        }
    }
}
