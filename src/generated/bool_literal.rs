// GENERATED FROM: bool_literal.yaml
// SPEC HASH: sha256:36872e0f7f0af7d1
// GENERATED: 2026-01-06T05:05:48.704114893+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
pub fn bool_literal(value: bool, target: String) -> String {
    if ((value == true) && (target == "Python")) {
        // true_py
        "True".to_string()
    } else if ((value == false) && (target == "Python")) {
        // false_py
        "False".to_string()
    } else if (value == true) {
        // true_default
        "true".to_string()
    } else if (value == false) {
        // false_default
        "false".to_string()
    } else {
        unreachable!("No rule matched")
    }
}
