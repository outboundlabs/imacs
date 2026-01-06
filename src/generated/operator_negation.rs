// GENERATED FROM: operator_negation.yaml
// SPEC HASH: sha256:719d81f5844361a1
// GENERATED: 2026-01-06T05:05:49.407065271+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
pub fn operator_negation(op: String) -> String {
    if (op == "Eq") {
        // eq_to_ne
        "Ne".to_string()
    } else if (op == "Ne") {
        // ne_to_eq
        "Eq".to_string()
    } else if (op == "Lt") {
        // lt_to_ge
        "Ge".to_string()
    } else if (op == "Le") {
        // le_to_gt
        "Gt".to_string()
    } else if (op == "Gt") {
        // gt_to_le
        "Le".to_string()
    } else if (op == "Ge") {
        // ge_to_lt
        "Lt".to_string()
    } else {
        unreachable!("No rule matched")
    }
}
