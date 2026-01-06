// GENERATED FROM: null_literal.yaml
// SPEC HASH: sha256:89a8d6dc562fefab
// GENERATED: 2026-01-06T05:05:49.253036529+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
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
