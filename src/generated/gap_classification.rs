// GENERATED FROM: gap_classification.yaml
// SPEC HASH: sha256:876759ce8c965ce4
// GENERATED: 2026-01-06T05:05:49.151871631+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
pub fn gap_classification(
    rule_found: bool,
    condition_matches: bool,
    output_matches: bool,
    priority_correct: bool,
) -> (String, String) {
    if ((rule_found && condition_matches) && output_matches) {
        // no_gap
        ("None".to_string(), "None".to_string())
    } else if (!rule_found) {
        // missing
        ("Missing".to_string(), "Error".to_string())
    } else if (rule_found && (!condition_matches)) {
        // condition
        ("ConditionMismatch".to_string(), "Error".to_string())
    } else if ((rule_found && condition_matches) && (!output_matches)) {
        // output
        ("OutputMismatch".to_string(), "Error".to_string())
    } else if (((rule_found && condition_matches) && output_matches) && (!priority_correct)) {
        // priority
        ("WrongPriority".to_string(), "Warning".to_string())
    } else {
        unreachable!("No rule matched")
    }
}
