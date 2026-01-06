// GENERATED FROM: issue_severity.yaml
// SPEC HASH: sha256:b3c0ee9e0a3bf438
// GENERATED: 2026-01-06T05:05:49.207459300+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
pub fn issue_severity(issue_kind: String, threshold_exceeded_by: String) -> String {
    if ((issue_kind == "HighComplexity") && (threshold_exceeded_by == "Large")) {
        // complexity_error
        "Error".to_string()
    } else if (issue_kind == "HighComplexity") {
        // complexity_warn
        "Warning".to_string()
    } else if (issue_kind == "DeepNesting") {
        // nesting
        "Warning".to_string()
    } else if (issue_kind == "LongFunction") {
        // long_func
        "Warning".to_string()
    } else if (issue_kind == "MagicNumber") {
        // magic
        "Info".to_string()
    } else if (issue_kind == "TooManyParams") {
        // params
        "Warning".to_string()
    } else if (issue_kind == "MissingDefault") {
        // default
        "Warning".to_string()
    } else {
        unreachable!("No rule matched")
    }
}
