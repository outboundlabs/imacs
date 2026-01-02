// GENERATED FROM: string_render.yaml
// SPEC HASH: sha256:11916c4d1817b96f
// GENERATED: 2026-01-02T13:32:40.889493999+00:00
// DO NOT EDIT — regenerate from spec

pub fn string_render(target: String, needs_owned: bool) -> (String, String) {
    if ((target == "Rust") && needs_owned) {
        // rust_owned
        ("\"".to_string(), ".to_string()".to_string())
    } else if ((target == "Rust") && (!needs_owned)) {
        // rust_borrowed
        ("\"".to_string(), "".to_string())
    } else if (target == "TypeScript") {
        // ts
        ("\"".to_string(), "".to_string())
    } else if (target == "Python") {
        // py
        ("\"".to_string(), "".to_string())
    } else {
        unreachable!("No rule matched")
    }
}


#[cfg(test)]
mod tests {
    use super::*;

// GENERATED TESTS FROM: string_render.yaml
// SPEC HASH: sha256:11916c4d1817b96f
// GENERATED: 2026-01-02T13:32:40.925982342+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod string_render_tests {
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
            fn prop_always_valid_output(target in any::<bool>(), needs_owned in any::<bool>()) {
                let result = string_render(target, needs_owned);
                prop_assert!([("\"".to_string(), "".to_string()), ("\"".to_string(), ".to_string()".to_string())].contains(&result));
            }
        }
    }
}

}
