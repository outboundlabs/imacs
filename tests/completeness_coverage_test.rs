//! Comprehensive coverage tests for completeness module
//!
//! Tests all public APIs and edge cases to ensure 100% coverage

use imacs::completeness::*;
use imacs::spec::{ConditionValue, Output, Rule, Spec, VarType, Variable};

// ============================================================================
// analyze_completeness() Edge Cases
// ============================================================================

#[test]
fn test_analyze_empty_spec() {
    let spec = Spec {
        id: "empty".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "a".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: vec![],
        default: None,
        meta: Default::default(),
    };

    let report = analyze_completeness(&spec);
    assert!(!report.is_complete);
    // With no rules, no predicates are extracted, so total_combinations = 2^0 = 1
    assert_eq!(report.total_combinations, 1);
    assert_eq!(report.covered_combinations, 0);
    assert_eq!(report.coverage_ratio, 0.0);
}

#[test]
fn test_analyze_single_rule() {
    let spec = Spec {
        id: "single".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "a".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: vec![Rule {
            id: "R1".into(),
            when: Some("a".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(1)),
            priority: 0,
            description: None,
        }],
        default: None,
        meta: Default::default(),
    };

    let report = analyze_completeness(&spec);
    assert!(!report.is_complete); // Missing !a case
    assert_eq!(report.total_combinations, 2);
    assert_eq!(report.covered_combinations, 1);
}

#[test]
fn test_analyze_no_predicates() {
    let spec = Spec {
        id: "no_preds".into(),
        name: None,
        description: None,
        inputs: vec![],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: vec![Rule {
            id: "R1".into(),
            when: None, // No condition
            conditions: None,
            then: Output::Single(ConditionValue::Int(1)),
            priority: 0,
            description: None,
        }],
        default: None,
        meta: Default::default(),
    };

    let report = analyze_completeness(&spec);
    // Should handle gracefully - verify we get a valid report
    let _ = report.total_combinations;
}

#[test]
fn test_analyze_invalid_cel() {
    let spec = Spec {
        id: "invalid".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "a".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: vec![Rule {
            id: "R1".into(),
            when: Some("invalid!!!".into()), // Invalid CEL
            conditions: None,
            then: Output::Single(ConditionValue::Int(1)),
            priority: 0,
            description: None,
        }],
        default: None,
        meta: Default::default(),
    };

    let report = analyze_completeness(&spec);
    // Should handle gracefully, not panic - verify we get a valid report
    let _ = report.total_combinations;
}

// ============================================================================
// Predicate Extraction Edge Cases
// ============================================================================

#[test]
fn test_extract_predicates_or_expression() {
    let preds = extract_predicates("a || b").unwrap();
    assert_eq!(preds.len(), 2);
}

#[test]
fn test_extract_predicates_ternary() {
    let preds = extract_predicates("a ? b : c").unwrap();
    assert!(preds.len() >= 1);
}

#[test]
fn test_extract_predicates_nested() {
    let preds = extract_predicates("(a && b) || (c && d)").unwrap();
    assert_eq!(preds.len(), 4);
}

#[test]
fn test_predicate_negated_comparison() {
    let pred = Predicate::Comparison {
        var: "amount".into(),
        op: ComparisonOp::Gt,
        value: LiteralValue::Int(1000),
    };
    let neg = pred.negated();
    assert!(matches!(
        neg,
        Predicate::Comparison {
            op: ComparisonOp::Le,
            ..
        }
    ));
}

#[test]
fn test_predicate_negated_equality() {
    let pred = Predicate::Equality {
        var: "status".into(),
        value: LiteralValue::String("active".into()),
        negated: false,
    };
    let neg = pred.negated();
    assert!(matches!(neg, Predicate::Equality { negated: true, .. }));
}

#[test]
fn test_predicate_set_operations() {
    let mut set = PredicateSet::new();
    let pred1 = Predicate::BoolVar("a".into());
    let pred2 = Predicate::BoolVar("b".into());

    let idx1 = set.add(pred1.clone());
    let idx2 = set.add(pred1.clone()); // Duplicate
    let idx3 = set.add(pred2.clone());

    assert_eq!(idx1, idx2); // Should return same index
    assert_ne!(idx1, idx3);
    assert_eq!(set.len(), 2);
    assert_eq!(set.index_of(&pred1), Some(0));
    assert_eq!(set.index_of(&pred2), Some(1));
}

// ============================================================================
// Adapter Edge Cases
// ============================================================================

#[test]
fn test_cube_to_cel_all_values() {
    let mut pset = PredicateSet::new();
    pset.add(Predicate::BoolVar("a".into()));
    pset.add(Predicate::BoolVar("b".into()));

    // Test One
    let mut cube1 = Cube::new(2, 1);
    cube1.set_input(0, CubeValue::One);
    cube1.set_input(1, CubeValue::DontCare);
    let cel1 = cube_to_cel(&cube1, &pset).unwrap();
    assert!(cel1.contains("a"));

    // Test Zero
    let mut cube2 = Cube::new(2, 1);
    cube2.set_input(0, CubeValue::Zero);
    let cel2 = cube_to_cel(&cube2, &pset).unwrap();
    assert!(cel2.contains("!a") || cel2.contains("!"));

    // Test DontCare (should be empty or tautology)
    let mut cube3 = Cube::new(2, 1);
    cube3.set_input(0, CubeValue::DontCare);
    cube3.set_input(1, CubeValue::DontCare);
    let cel3 = cube_to_cel(&cube3, &pset).unwrap();
    assert_eq!(cel3, "true");
}

#[test]
fn test_rules_to_cover_multiple() {
    let mut pset = PredicateSet::new();
    pset.add(Predicate::BoolVar("a".into()));

    let rules = vec![
        Rule {
            id: "R1".into(),
            when: Some("a".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(1)),
            priority: 0,
            description: None,
        },
        Rule {
            id: "R2".into(),
            when: Some("!a".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(2)),
            priority: 0,
            description: None,
        },
    ];

    let cover = rules_to_cover(&rules, &pset);
    assert_eq!(cover.len(), 2);
}

// ============================================================================
// Suite Analysis Edge Cases
// ============================================================================

#[test]
fn test_analyze_suite_empty() {
    let specs = vec![];
    let result = analyze_suite(&specs, false);
    assert_eq!(result.individual_results.len(), 0);
    assert_eq!(result.collisions.len(), 0);
}

#[test]
fn test_analyze_suite_single() {
    let spec = Spec {
        id: "single".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "a".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![],
        rules: vec![],
        default: None,
        meta: Default::default(),
    };

    let specs = vec![("single".into(), spec)];
    let result = analyze_suite(&specs, false);
    assert_eq!(result.individual_results.len(), 1);
}

#[test]
fn test_analyze_suite_full_mode() {
    let spec = Spec {
        id: "test".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "a".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![],
        rules: vec![],
        default: None,
        meta: Default::default(),
    };

    let specs = vec![("test".into(), spec)];
    let result = analyze_suite(&specs, true);
    assert!(matches!(
        result.complexity.analysis_mode,
        AnalysisMode::Full
    ));
}

// ============================================================================
// Collision Detection Edge Cases
// ============================================================================

#[test]
fn test_detect_collisions_none() {
    let specs = vec![
        (
            "spec_a".into(),
            vec![Variable {
                name: "a".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            }],
        ),
        (
            "spec_b".into(),
            vec![Variable {
                name: "b".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            }],
        ),
    ];

    let collisions = detect_collisions(&specs);
    assert_eq!(collisions.len(), 0);
}

#[test]
fn test_detect_collisions_ambiguous() {
    let specs = vec![
        (
            "spec_a".into(),
            vec![Variable {
                name: "status".into(),
                typ: VarType::String,
                description: None,
                values: None, // No values = ambiguous
            }],
        ),
        (
            "spec_b".into(),
            vec![Variable {
                name: "status".into(),
                typ: VarType::String,
                description: None,
                values: None,
            }],
        ),
    ];

    let collisions = detect_collisions(&specs);
    assert!(!collisions.is_empty());
    assert!(collisions
        .iter()
        .any(|c| matches!(c.collision_type, CollisionType::AmbiguousSemantics)));
}

// ============================================================================
// Duplicate Detection Edge Cases
// ============================================================================

#[test]
fn test_detect_duplicates_none() {
    let spec_a = Spec {
        id: "spec_a".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "a".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![],
        rules: vec![Rule {
            id: "R1".into(),
            when: Some("a".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(1)),
            priority: 0,
            description: None,
        }],
        default: None,
        meta: Default::default(),
    };

    let spec_b = Spec {
        id: "spec_b".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "b".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![],
        rules: vec![Rule {
            id: "R1".into(),
            when: Some("b".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(1)),
            priority: 0,
            description: None,
        }],
        default: None,
        meta: Default::default(),
    };

    let specs = vec![("spec_a".into(), &spec_a), ("spec_b".into(), &spec_b)];

    let duplicates = detect_duplicates(&specs);
    // Different variables, so no duplicates
    assert_eq!(duplicates.len(), 0);
}

// ============================================================================
// Relationship Detection Edge Cases
// ============================================================================

#[test]
fn test_detect_relationships_none() {
    let spec_a = Spec {
        id: "spec_a".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "a".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![],
        rules: vec![],
        default: None,
        meta: Default::default(),
    };

    let spec_b = Spec {
        id: "spec_b".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "b".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![],
        rules: vec![],
        default: None,
        meta: Default::default(),
    };

    let specs = vec![("spec_a".into(), &spec_a), ("spec_b".into(), &spec_b)];

    let relationships = detect_relationships(&specs);
    // No overlap, no relationships
    assert_eq!(relationships.len(), 0);
}

// ============================================================================
// Variable Matching Edge Cases
// ============================================================================

#[test]
fn test_match_variables_no_matches() {
    let specs = vec![
        (
            "spec_a".into(),
            vec![Variable {
                name: "a".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            }],
        ),
        (
            "spec_b".into(),
            vec![Variable {
                name: "b".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            }],
        ),
    ];

    let result = match_variables(&specs);
    assert_eq!(result.matches.len(), 0);
}

// ============================================================================
// Suggestions Edge Cases
// ============================================================================

#[test]
fn test_generate_suggestions_empty() {
    let suggestions = generate_suggestions(&[], &[], &[]);
    assert_eq!(suggestions.len(), 0);
}

// ============================================================================
// Orchestrator Suite Edge Cases
// ============================================================================

#[test]
fn test_analyze_orchestrator_suite_missing_specs() {
    use imacs::orchestrate::{CallStep, ChainStep, Orchestrator};
    use std::collections::HashMap;

    let orch = Orchestrator {
        id: "test".into(),
        name: None,
        description: None,
        inputs: vec![],
        outputs: vec![],
        uses: vec!["missing_spec".into()],
        chain: vec![ChainStep::Call(CallStep {
            id: "step1".into(),
            spec: "missing_spec".into(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            condition: None,
            timeout: None,
            retry: None,
        })],
    };

    let specs = HashMap::new();
    let result = analyze_orchestrator_suite(&orch, &specs, false);

    assert_eq!(result.missing_specs.len(), 1);
    assert_eq!(result.found_specs.len(), 0);
}
