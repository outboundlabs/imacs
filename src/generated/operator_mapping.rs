// GENERATED FROM: operator_mapping.yaml
// SPEC HASH: sha256:a20f57d199b9c1a4
// GENERATED: 2026-01-06T05:05:49.352836896+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
pub fn operator_mapping(op: String, target: String) -> String {
    if ((op == "Eq") && (target == "TypeScript")) {
        // eq_ts
        "===".to_string()
    } else if (op == "Eq") {
        // eq_default
        "==".to_string()
    } else if ((op == "Ne") && (target == "TypeScript")) {
        // ne_ts
        "!==".to_string()
    } else if (op == "Ne") {
        // ne_default
        "!=".to_string()
    } else if (op == "Lt") {
        // lt
        "<".to_string()
    } else if (op == "Le") {
        // le
        "<=".to_string()
    } else if (op == "Gt") {
        // gt
        ">".to_string()
    } else if (op == "Ge") {
        // ge
        ">=".to_string()
    } else if ((op == "And") && (target == "Python")) {
        // and_py
        "and".to_string()
    } else if (op == "And") {
        // and_default
        "&&".to_string()
    } else if ((op == "Or") && (target == "Python")) {
        // or_py
        "or".to_string()
    } else if (op == "Or") {
        // or_default
        "||".to_string()
    } else if ((op == "Not") && (target == "Python")) {
        // not_py
        "not ".to_string()
    } else if (op == "Not") {
        // not_default
        "!".to_string()
    } else if ((op == "In") && (target == "Rust")) {
        // in_rust
        ".contains(&{})".to_string()
    } else if ((op == "In") && (target == "TypeScript")) {
        // in_ts
        ".includes({})".to_string()
    } else if ((op == "In") && (target == "Python")) {
        // in_py
        " in ".to_string()
    } else if ((op == "In") && (target == "CSharp")) {
        // in_csharp
        ".Contains({})".to_string()
    } else if ((op == "In") && (target == "Java")) {
        // in_java
        ".contains({})".to_string()
    } else if ((op == "In") && (target == "Go")) {
        // in_go
        "contains({}, {})".to_string()
    } else {
        unreachable!("No rule matched")
    }
}
