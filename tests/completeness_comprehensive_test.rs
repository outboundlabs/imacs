//! Comprehensive data-driven tests for completeness analysis
//!
//! Tests every use case scenario to demonstrate declarative behavior
//! and ensure the tool works correctly for all edge cases.

use imacs::completeness::analyze_completeness;
use imacs::spec::{ConditionValue, Output, Rule, Spec, VarType, Variable};
use rstest::rstest;

// ============================================================================
// Single Spec Test Cases (Data-Driven)
// ============================================================================

#[rstest]
#[case("complete_2_bool", true, 4, 4, 0, 0, false)]
#[case("incomplete_2_bool", false, 4, 3, 1, 0, false)]
#[case("complete_3_bool", true, 8, 8, 0, 0, false)]
// overlapping_rules: 1 bool var (2 combinations), rules only cover a=true (1 combination)
#[case("overlapping_rules", false, 2, 1, 1, 1, false)]
// minimizable_spec: 2 bool vars (4 combinations), rules cover a=true cases only (2 combinations)
#[case("minimizable_spec", false, 4, 2, 2, 0, true)]
// empty_spec: No rules means no predicates extracted, total=1, covered=0, no specific missing cases reported
#[case("empty_spec", false, 1, 0, 0, 0, false)]
// TODO: Non-boolean predicates need enhanced support
// For now, comparison/equality/membership predicates are treated as independent atoms
// comparison_predicates: 2 predicates = 4 combos, 3 covered, 1 overlap (both rules match some combos)
#[case("comparison_predicates", false, 4, 3, 1, 1, false)]
// equality_predicates: 2 predicates = 4 combos, 3 covered, 1 overlap
#[case("equality_predicates", false, 4, 3, 1, 1, false)]
// membership_predicates: 2 predicates from in[] = 4 combos, all covered, 2 overlaps
#[case("membership_predicates", true, 4, 4, 0, 2, false)]
fn test_completeness_scenarios(
    #[case] name: &str,
    #[case] expected_complete: bool,
    #[case] expected_total: u64,
    #[case] expected_covered: u64,
    #[case] expected_missing: usize,
    #[case] expected_overlap: usize,
    #[case] expected_minimize: bool,
) {
    let spec = match name {
        "complete_2_bool" => make_2_bool_spec(true),
        "incomplete_2_bool" => make_2_bool_spec(false),
        "complete_3_bool" => make_3_bool_spec(),
        "overlapping_rules" => make_overlapping_spec(),
        "minimizable_spec" => make_minimizable_spec(),
        "empty_spec" => make_empty_spec(),
        "comparison_predicates" => make_comparison_spec(),
        "equality_predicates" => make_equality_spec(),
        "membership_predicates" => make_membership_spec(),
        _ => panic!("Unknown test case: {}", name),
    };

    let report = analyze_completeness(&spec);

    assert_eq!(
        report.is_complete, expected_complete,
        "{}: completeness mismatch",
        name
    );
    assert_eq!(
        report.total_combinations, expected_total,
        "{}: total combinations mismatch",
        name
    );
    assert_eq!(
        report.covered_combinations, expected_covered,
        "{}: covered combinations mismatch",
        name
    );
    assert_eq!(
        report.missing_cases.len(),
        expected_missing,
        "{}: missing cases count mismatch",
        name
    );
    assert_eq!(
        report.overlaps.len(),
        expected_overlap,
        "{}: overlaps count mismatch",
        name
    );
    assert_eq!(
        report.can_minimize, expected_minimize,
        "{}: minimization opportunity mismatch",
        name
    );
}

// ============================================================================
// Cross-Spec Test Cases (Data-Driven)
// ============================================================================

#[rstest]
// collision_different_values: same var name with different enum values = 1 collision, also detected as merge opportunity
#[case("collision_different_values", 1, 0, 0, 1)]
#[case("chain_relationship", 0, 0, 1, 0)]
// duplicate_rules: duplicate logic detected, var overlap triggers collision and merge
#[case("duplicate_rules", 1, 1, 0, 1)]
// merge_opportunity: shared vars trigger both collisions and merge detection
#[case("merge_opportunity", 2, 0, 0, 1)]
fn test_cross_spec_scenarios(
    #[case] name: &str,
    #[case] expected_collisions: usize,
    #[case] expected_duplicates: usize,
    #[case] expected_chains: usize,
    #[case] expected_merges: usize,
) {
    use imacs::completeness::{detect_collisions, detect_duplicates, detect_relationships};

    let specs = match name {
        "collision_different_values" => vec![
            (
                "spec_a".into(),
                make_spec_with_var("customer_type", vec!["standard".into(), "premium".into()]),
            ),
            (
                "spec_b".into(),
                make_spec_with_var("customer_type", vec!["new".into(), "returning".into()]),
            ),
        ],
        "chain_relationship" => vec![
            (
                "discounts".into(),
                make_spec_with_output("discount_rate", VarType::Float),
            ),
            (
                "pricing".into(),
                make_spec_with_input("discount_rate", VarType::Float),
            ),
        ],
        "duplicate_rules" => vec![
            ("spec_a".into(), make_spec_with_rule(r#"region == "US""#)),
            ("spec_b".into(), make_spec_with_rule(r#"region == "US""#)),
        ],
        "merge_opportunity" => vec![
            (
                "spec_a".into(),
                make_spec_with_vars(vec!["a".into(), "b".into(), "c".into()]),
            ),
            (
                "spec_b".into(),
                make_spec_with_vars(vec!["a".into(), "b".into(), "d".into()]),
            ),
        ],
        _ => panic!("Unknown test case: {}", name),
    };

    // Extract variable lists for collision detection
    let spec_vars: Vec<_> = specs
        .iter()
        .map(|(id, spec): &(String, Spec)| (id.clone(), spec.inputs.clone()))
        .collect();

    let collisions = detect_collisions(&spec_vars);
    let spec_refs: Vec<_> = specs
        .iter()
        .map(|(id, spec): &(String, Spec)| (id.clone(), spec))
        .collect();
    let duplicates = detect_duplicates(&spec_refs);
    let relationships = detect_relationships(&spec_refs);

    let chains: Vec<_> = relationships
        .iter()
        .filter(|r| {
            matches!(
                r.relationship_type,
                imacs::completeness::RelationshipType::Chain
            )
        })
        .collect();
    let merges: Vec<_> = relationships
        .iter()
        .filter(|r| {
            matches!(
                r.relationship_type,
                imacs::completeness::RelationshipType::MergeOpportunity
            )
        })
        .collect();

    assert_eq!(
        collisions.len(),
        expected_collisions,
        "{}: collisions count mismatch",
        name
    );
    assert_eq!(
        duplicates.len(),
        expected_duplicates,
        "{}: duplicates count mismatch",
        name
    );
    assert_eq!(
        chains.len(),
        expected_chains,
        "{}: chains count mismatch",
        name
    );
    assert_eq!(
        merges.len(),
        expected_merges,
        "{}: merges count mismatch",
        name
    );
}

// ============================================================================
// Helper Functions for Test Data
// ============================================================================

fn make_2_bool_spec(complete: bool) -> Spec {
    let mut rules = vec![
        Rule {
            id: "R1".into(),
            when: Some("a && b".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(1)),
            priority: 0,
            description: None,
        },
        Rule {
            id: "R2".into(),
            when: Some("a && !b".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(2)),
            priority: 0,
            description: None,
        },
        Rule {
            id: "R3".into(),
            when: Some("!a && b".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(3)),
            priority: 0,
            description: None,
        },
    ];

    if complete {
        rules.push(Rule {
            id: "R4".into(),
            when: Some("!a && !b".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(4)),
            priority: 0,
            description: None,
        });
    }

    Spec {
        id: "test_2_bool".into(),
        name: None,
        description: None,
        inputs: vec![
            Variable {
                name: "a".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            },
            Variable {
                name: "b".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            },
        ],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules,
        default: None,
        meta: Default::default(),
    }
}

fn make_3_bool_spec() -> Spec {
    Spec {
        id: "test_3_bool".into(),
        name: None,
        description: None,
        inputs: vec![
            Variable {
                name: "a".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            },
            Variable {
                name: "b".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            },
            Variable {
                name: "c".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            },
        ],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: (0..8)
            .map(|i| {
                let a_val = (i & 1) != 0;
                let b_val = (i & 2) != 0;
                let c_val = (i & 4) != 0;

                let mut conditions = Vec::new();
                if a_val {
                    conditions.push("a".to_string());
                } else {
                    conditions.push("!a".to_string());
                }
                if b_val {
                    conditions.push("b".to_string());
                } else {
                    conditions.push("!b".to_string());
                }
                if c_val {
                    conditions.push("c".to_string());
                } else {
                    conditions.push("!c".to_string());
                }

                Rule {
                    id: format!("R{}", i + 1),
                    when: Some(conditions.join(" && ")),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(i as i64)),
                    priority: 0,
                    description: None,
                }
            })
            .collect(),
        default: None,
        meta: Default::default(),
    }
}

fn make_overlapping_spec() -> Spec {
    Spec {
        id: "overlap_test".into(),
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
        rules: vec![
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
                when: Some("a".into()), // Same condition!
                conditions: None,
                then: Output::Single(ConditionValue::Int(2)),
                priority: 0,
                description: None,
            },
        ],
        default: None,
        meta: Default::default(),
    }
}

fn make_minimizable_spec() -> Spec {
    // (a && b) || (a && !b) → a
    Spec {
        id: "minimize_test".into(),
        name: None,
        description: None,
        inputs: vec![
            Variable {
                name: "a".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            },
            Variable {
                name: "b".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            },
        ],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: vec![
            Rule {
                id: "R1".into(),
                when: Some("a && b".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            },
            Rule {
                id: "R2".into(),
                when: Some("a && !b".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            },
        ],
        default: None,
        meta: Default::default(),
    }
}

fn make_empty_spec() -> Spec {
    Spec {
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
    }
}

fn make_comparison_spec() -> Spec {
    Spec {
        id: "comparison_test".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "amount".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: vec![
            Rule {
                id: "R1".into(),
                when: Some("amount > 1000".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            },
            Rule {
                id: "R2".into(),
                when: Some("amount <= 1000".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(0)),
                priority: 0,
                description: None,
            },
        ],
        default: None,
        meta: Default::default(),
    }
}

fn make_equality_spec() -> Spec {
    Spec {
        id: "equality_test".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "status".into(),
            typ: VarType::String,
            description: None,
            values: Some(vec!["active".into(), "inactive".into()]),
        }],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: vec![
            Rule {
                id: "R1".into(),
                when: Some(r#"status == "active""#.into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            },
            Rule {
                id: "R2".into(),
                when: Some(r#"status == "inactive""#.into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(0)),
                priority: 0,
                description: None,
            },
        ],
        default: None,
        meta: Default::default(),
    }
}

fn make_membership_spec() -> Spec {
    Spec {
        id: "membership_test".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "region".into(),
            typ: VarType::String,
            description: None,
            values: Some(vec!["US".into(), "EU".into(), "APAC".into()]),
        }],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: vec![
            Rule {
                id: "R1".into(),
                when: Some(r#"region in ["US", "EU"]"#.into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            },
            Rule {
                id: "R2".into(),
                when: Some(r#"region == "APAC""#.into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(0)),
                priority: 0,
                description: None,
            },
        ],
        default: None,
        meta: Default::default(),
    }
}

fn make_spec_with_var(name: &str, values: Vec<String>) -> Spec {
    Spec {
        id: format!("spec_{}", name),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: name.into(),
            typ: VarType::String,
            description: None,
            values: Some(values),
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
    }
}

fn make_spec_with_input(name: &str, typ: VarType) -> Spec {
    Spec {
        id: format!("spec_{}", name),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: name.into(),
            typ,
            description: None,
            values: None,
        }],
        outputs: vec![],
        rules: vec![],
        default: None,
        meta: Default::default(),
    }
}

fn make_spec_with_output(name: &str, typ: VarType) -> Spec {
    Spec {
        id: format!("spec_{}", name),
        name: None,
        description: None,
        inputs: vec![],
        outputs: vec![Variable {
            name: name.into(),
            typ,
            description: None,
            values: None,
        }],
        rules: vec![],
        default: None,
        meta: Default::default(),
    }
}

fn make_spec_with_rule(when: &str) -> Spec {
    Spec {
        id: "spec_with_rule".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "region".into(),
            typ: VarType::String,
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
            when: Some(when.into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(1)),
            priority: 0,
            description: None,
        }],
        default: None,
        meta: Default::default(),
    }
}

fn make_spec_with_vars(names: Vec<String>) -> Spec {
    Spec {
        id: "spec_with_vars".into(),
        name: None,
        description: None,
        inputs: names
            .into_iter()
            .map(|name| Variable {
                name,
                typ: VarType::Bool,
                description: None,
                values: None,
            })
            .collect(),
        outputs: vec![],
        rules: vec![],
        default: None,
        meta: Default::default(),
    }
}
