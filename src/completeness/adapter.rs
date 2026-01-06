//! Adapter between CEL predicates and Espresso cubes
//!
//! This module provides conversion functions to bridge the gap between
//! the predicate-based completeness analysis and Espresso's cube-based
//! logic minimization.

use super::espresso::{Cover, Cube, CubeValue};
use super::predicates::{Predicate, PredicateSet};
use crate::spec::Rule;

/// Convert a set of rules with predicates into an Espresso Cover
///
/// Each rule becomes a cube where:
/// - Predicate index i being true -> input\[i\] = 1
/// - Predicate index i being false -> input\[i\] = 0
/// - Predicate not mentioned -> input\[i\] = - (don't care)
pub fn rules_to_cover(rules: &[Rule], predicate_set: &PredicateSet) -> Cover {
    let num_inputs = predicate_set.predicates.len();
    let num_outputs = 1; // Single output for now

    let mut cover = Cover::new(num_inputs, num_outputs);

    for rule in rules {
        if let Some(cel_expr) = rule.as_cel() {
            if let Ok(cube) = expression_to_cube(&cel_expr, predicate_set) {
                cover.add(cube);
            }
        }
    }

    cover
}

/// Convert a single CEL expression to an Espresso Cube
pub fn expression_to_cube(expr: &str, predicate_set: &PredicateSet) -> Result<Cube, String> {
    let num_inputs = predicate_set.predicates.len();
    let mut cube = Cube::new(num_inputs, 1);

    // Set output to 1 (this cube is active)
    cube.set_output(0, CubeValue::One);

    // Parse the expression and set cube values
    if let Ok(ast) = cel_parser::parse(expr) {
        set_cube_from_ast(&ast, &mut cube, predicate_set, false);
    }

    Ok(cube)
}

/// Recursively process AST to set cube values
fn set_cube_from_ast(
    expr: &cel_parser::Expression,
    cube: &mut Cube,
    predicate_set: &PredicateSet,
    negated: bool,
) {
    use cel_parser::Expression as E;

    match expr {
        E::Ident(name) => {
            let name_str = name.to_string();
            // Find this predicate in our set
            let pred = Predicate::BoolVar(name_str.clone());
            if let Some(idx) = predicate_set.index_of(&pred) {
                cube.set_input(
                    idx,
                    if negated {
                        CubeValue::Zero
                    } else {
                        CubeValue::One
                    },
                );
            }
        }

        E::And(left, right) => {
            set_cube_from_ast(left, cube, predicate_set, negated);
            set_cube_from_ast(right, cube, predicate_set, negated);
        }

        E::Unary(cel_parser::UnaryOp::Not, inner) => {
            set_cube_from_ast(inner, cube, predicate_set, !negated);
        }

        E::Relation(left, op, right) => {
            // Build predicate from relation and find its index
            if let Some(pred) = relation_to_predicate(left, op, right) {
                if let Some(idx) = predicate_set.index_of(&pred) {
                    cube.set_input(
                        idx,
                        if negated {
                            CubeValue::Zero
                        } else {
                            CubeValue::One
                        },
                    );
                } else {
                    // Try negated form
                    let neg_pred = pred.negated();
                    if let Some(idx) = predicate_set.index_of(&neg_pred) {
                        cube.set_input(
                            idx,
                            if negated {
                                CubeValue::One
                            } else {
                                CubeValue::Zero
                            },
                        );
                    }
                }
            }
        }

        // For OR expressions, we can't represent them in a single cube
        // They need multiple cubes (handled at higher level)
        E::Or(_, _) => {
            // Skip - OR requires multiple cubes
        }

        _ => {}
    }
}

/// Convert a relation expression to a Predicate
fn relation_to_predicate(
    left: &cel_parser::Expression,
    op: &cel_parser::RelationOp,
    right: &cel_parser::Expression,
) -> Option<Predicate> {
    use super::predicates::{ComparisonOp, LiteralValue};
    use cel_parser::Expression as E;
    use cel_parser::RelationOp;

    let var = match left {
        E::Ident(name) => name.to_string(),
        _ => return None,
    };

    match (op, right) {
        (RelationOp::GreaterThan, E::Atom(cel_parser::Atom::Int(i))) => {
            Some(Predicate::Comparison {
                var,
                op: ComparisonOp::Gt,
                value: LiteralValue::Int(*i),
            })
        }
        (RelationOp::GreaterThanEq, E::Atom(cel_parser::Atom::Int(i))) => {
            Some(Predicate::Comparison {
                var,
                op: ComparisonOp::Ge,
                value: LiteralValue::Int(*i),
            })
        }
        (RelationOp::LessThan, E::Atom(cel_parser::Atom::Int(i))) => Some(Predicate::Comparison {
            var,
            op: ComparisonOp::Lt,
            value: LiteralValue::Int(*i),
        }),
        (RelationOp::LessThanEq, E::Atom(cel_parser::Atom::Int(i))) => {
            Some(Predicate::Comparison {
                var,
                op: ComparisonOp::Le,
                value: LiteralValue::Int(*i),
            })
        }
        (RelationOp::Equals, E::Atom(cel_parser::Atom::String(s))) => Some(Predicate::Equality {
            var,
            value: LiteralValue::String(s.to_string()),
            negated: false,
        }),
        (RelationOp::NotEquals, E::Atom(cel_parser::Atom::String(s))) => {
            Some(Predicate::Equality {
                var,
                value: LiteralValue::String(s.to_string()),
                negated: true,
            })
        }
        _ => None,
    }
}

/// Convert an Espresso Cover back to CEL expressions
///
/// Each cube in the cover becomes a CEL expression.
/// Returns a vector of CEL expression strings.
pub fn cover_to_cel(cover: &Cover, predicate_set: &PredicateSet) -> Vec<String> {
    let mut expressions = Vec::new();

    for cube in cover.iter() {
        if let Some(expr) = cube_to_cel(cube, predicate_set) {
            expressions.push(expr);
        }
    }

    expressions
}

/// Convert a single Espresso Cube to a CEL expression
pub fn cube_to_cel(cube: &Cube, predicate_set: &PredicateSet) -> Option<String> {
    let mut conditions = Vec::new();

    for (idx, pred) in predicate_set.predicates.iter().enumerate() {
        match cube.input(idx) {
            CubeValue::One => {
                conditions.push(pred.to_cel_string());
            }
            CubeValue::Zero => {
                // Negate the predicate
                let neg = pred.negated();
                conditions.push(neg.to_cel_string());
            }
            CubeValue::DontCare => {
                // Don't include in expression
            }
        }
    }

    if conditions.is_empty() {
        Some("true".to_string()) // Tautology
    } else if conditions.len() == 1 {
        Some(conditions.into_iter().next().unwrap())
    } else {
        Some(conditions.join(" && "))
    }
}

/// Minimize a set of rules using Espresso
///
/// Returns a simplified set of CEL expressions that are logically equivalent.
pub fn minimize_rules(rules: &[Rule], predicate_set: &PredicateSet) -> Vec<String> {
    use super::espresso::espresso;

    let on_set = rules_to_cover(rules, predicate_set);
    let dc_set = Cover::new(predicate_set.predicates.len(), 1);

    let minimized = espresso(&on_set, &dc_set);

    cover_to_cel(&minimized, predicate_set)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{ConditionValue, Output, Rule};

    #[test]
    fn test_expression_to_cube() {
        let mut pset = PredicateSet::new();
        pset.add(Predicate::BoolVar("rate_exceeded".into()));
        pset.add(Predicate::Comparison {
            var: "amount".into(),
            op: super::super::predicates::ComparisonOp::Gt,
            value: super::super::predicates::LiteralValue::Int(1000),
        });

        let cube = expression_to_cube("rate_exceeded", &pset).unwrap();
        assert_eq!(cube.input(0), CubeValue::One);
        assert_eq!(cube.input(1), CubeValue::DontCare);
    }

    #[test]
    fn test_cube_to_cel() {
        let mut pset = PredicateSet::new();
        pset.add(Predicate::BoolVar("rate_exceeded".into()));
        pset.add(Predicate::BoolVar("is_premium".into()));

        let mut cube = Cube::new(2, 1);
        cube.set_input(0, CubeValue::One);
        cube.set_input(1, CubeValue::Zero);
        cube.set_output(0, CubeValue::One);

        let cel = cube_to_cel(&cube, &pset).unwrap();
        assert!(cel.contains("rate_exceeded"));
        assert!(cel.contains("!is_premium"));
    }

    #[test]
    fn test_rules_to_cover() {
        let mut pset = PredicateSet::new();
        pset.add(Predicate::BoolVar("flag".into()));

        let rules = vec![Rule {
            id: "R1".into(),
            when: Some("flag".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(1)),
            priority: 0,
            description: None,
        }];

        let cover = rules_to_cover(&rules, &pset);
        assert_eq!(cover.len(), 1);
    }
}
