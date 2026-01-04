//! Integration tests for suite analysis

use imacs::completeness::analyze_suite;
use imacs::spec::Spec;
use std::fs;
use std::path::PathBuf;

fn load_spec_fixture(name: &str) -> Spec {
    let path = PathBuf::from("tests/fixtures/suite").join(name);
    let content = fs::read_to_string(&path).expect("Failed to read fixture");
    Spec::from_yaml(&content).expect("Failed to parse fixture")
}

#[test]
fn test_suite_analysis_basic() {
    let specs = vec![
        ("pricing".into(), load_spec_fixture("pricing.yaml")),
        ("discounts".into(), load_spec_fixture("discounts.yaml")),
    ];

    let result = analyze_suite(&specs, false);

    assert_eq!(result.individual_results.len(), 2);
    assert!(!result.collisions.is_empty()); // Should detect customer_type collision
}

#[test]
fn test_detect_collisions() {
    let specs = vec![
        ("pricing".into(), load_spec_fixture("pricing.yaml")),
        ("discounts".into(), load_spec_fixture("discounts.yaml")),
        ("billing".into(), load_spec_fixture("billing.yaml")),
    ];

    let result = analyze_suite(&specs, false);

    // Should detect customer_type collision across all three specs
    let customer_type_collisions: Vec<_> = result
        .collisions
        .iter()
        .filter(|c| c.variable_name == "customer_type")
        .collect();

    assert!(!customer_type_collisions.is_empty());
}

#[test]
fn test_detect_relationships() {
    let specs = vec![
        ("discounts".into(), load_spec_fixture("discounts.yaml")),
        ("billing".into(), load_spec_fixture("billing.yaml")),
    ];

    let result = analyze_suite(&specs, false);

    // Should detect chain: discounts.discount_rate → billing.discount_rate
    let chains: Vec<_> = result
        .relationships
        .iter()
        .filter(|r| {
            matches!(
                r.relationship_type,
                imacs::completeness::RelationshipType::Chain
            )
        })
        .collect();

    assert!(!chains.is_empty());
}

#[test]
fn test_complexity_report() {
    let specs = vec![
        ("pricing".into(), load_spec_fixture("pricing.yaml")),
        ("discounts".into(), load_spec_fixture("discounts.yaml")),
        ("shipping".into(), load_spec_fixture("shipping.yaml")),
    ];

    let result = analyze_suite(&specs, false);

    assert!(result.complexity.total_unique_predicates > 0);
    assert!(matches!(
        result.complexity.analysis_mode,
        imacs::completeness::AnalysisMode::Incremental
    ));
}

#[test]
fn test_suggestions_generated() {
    let specs = vec![
        ("pricing".into(), load_spec_fixture("pricing.yaml")),
        ("discounts".into(), load_spec_fixture("discounts.yaml")),
    ];

    let result = analyze_suite(&specs, false);

    // Should generate suggestions for collisions
    if !result.collisions.is_empty() {
        assert!(!result.suggestions.is_empty());
    }
}
