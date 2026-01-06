// GENERATED FROM: extraction_confidence.yaml
// SPEC HASH: sha256:2a233b220bb043ae
// GENERATED: 2026-01-06T05:05:49.109582751+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
pub fn extraction_confidence(pattern_type: String, has_guard: bool, output_type: String) -> f64 {
    if (((pattern_type == "Literal") && (output_type == "Literal")) && (!has_guard)) {
        // literal_literal
        1.0f64
    } else if (((pattern_type == "Tuple") && (output_type == "Literal")) && (!has_guard)) {
        // tuple_literal
        0.95f64
    } else if ((pattern_type == "Wildcard") && (output_type == "Literal")) {
        // wildcard
        0.85f64
    } else if has_guard {
        // guarded
        0.7f64
    } else if (pattern_type == "Constructor") {
        // constructor
        0.75f64
    } else if (output_type == "FunctionCall") {
        // complex_output
        0.6f64
    } else if ((pattern_type == "Complex") || (output_type == "Complex")) {
        // complex
        0.4f64
    } else {
        unreachable!("No rule matched")
    }
}
