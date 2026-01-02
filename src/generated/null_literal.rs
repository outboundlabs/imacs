// GENERATED FROM: null_literal.yaml
// SPEC HASH: sha256:886ecd42f78a4fff
// GENERATED: 2026-01-02T13:32:40.564102391+00:00
// DO NOT EDIT — regenerate from spec

pub fn null_literal(target: String) -> String {
    if (target == "Rust") {
        // null_rust
        "None".to_string()
    } else if (target == "TypeScript") {
        // null_ts
        "null".to_string()
    } else if (target == "Python") {
        // null_py
        "None".to_string()
    } else if (target == "CSharp") {
        // null_csharp
        "null".to_string()
    } else if (target == "Java") {
        // null_java
        "null".to_string()
    } else if (target == "Go") {
        // null_go
        "nil".to_string()
    } else {
        unreachable!("No rule matched")
    }
}


#[cfg(test)]
mod tests {
    use super::*;

// GENERATED TESTS FROM: null_literal.yaml
// SPEC HASH: sha256:886ecd42f78a4fff
// GENERATED: 2026-01-02T13:32:40.623876069+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod null_literal_tests {
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
            fn prop_always_valid_output(target in any::<bool>()) {
                let result = null_literal(target);
                prop_assert!(["None".to_string(), "nil".to_string(), "null".to_string()].contains(&result));
            }
        }
    }
}

}
