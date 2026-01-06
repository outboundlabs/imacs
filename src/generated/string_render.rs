// GENERATED FROM: string_render.yaml
// SPEC HASH: sha256:879a9c7c59ff173a
// GENERATED: 2026-01-06T05:05:49.445200847+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
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
