//! Adapter between CEL predicates and Espresso cubes
//!
//! This module provides conversion functions to bridge the gap between
//! the predicate-based completeness analysis and Espresso's cube-based
//! logic minimization.

use super::espresso::{Cover, Cube, CubeValue};
use super::predicates::{Predicate, PredicateSet};
use crate::spec::Rule;
use cel_parser::{
    ast::operators,
    ast::{CallExpr, Expr},
    reference::Val,
    Parser,
};

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
    if let Ok(ast) = Parser::new().parse(expr) {
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
    // In cel-parser 0.10, Expression is IdedExpr with expr field
    match &expr.expr {
        Expr::Ident(name) => {
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

        Expr::Call(call) => {
            // Check if this is a logical AND
            if call.func_name == operators::LOGICAL_AND {
                if call.args.len() == 2 {
                    set_cube_from_ast(&call.args[0], cube, predicate_set, negated);
                    set_cube_from_ast(&call.args[1], cube, predicate_set, negated);
                }
            } else if call.func_name == operators::LOGICAL_OR {
                // For OR expressions, we can't represent them in a single cube
                // They need multiple cubes (handled at higher level)
                // Skip - OR requires multiple cubes
            } else if call.func_name == operators::LOGICAL_NOT {
                // NOT operator
                if let Some(inner) = call.args.first() {
                    set_cube_from_ast(inner, cube, predicate_set, !negated);
                }
            } else if is_relation_op(&call.func_name) {
                // Relation operator (==, !=, <, >, <=, >=)
                if call.args.len() == 2 {
                    if let Some(pred) = relation_to_predicate_from_call(call) {
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
            }
        }

        _ => {}
    }
}

/// Check if a function name is a relation operator
fn is_relation_op(func_name: &str) -> bool {
    func_name == operators::EQUALS
        || func_name == operators::NOT_EQUALS
        || func_name == operators::GREATER
        || func_name == operators::LESS
        || func_name == operators::GREATER_EQUALS
        || func_name == operators::LESS_EQUALS
}

/// Convert a relation Call expression to a Predicate
fn relation_to_predicate_from_call(call: &CallExpr) -> Option<Predicate> {
    use super::predicates::{ComparisonOp, LiteralValue};

    if call.args.len() != 2 {
        return None;
    }

    let left = &call.args[0];
    let right = &call.args[1];

    // Get variable name from left side
    let var = match &left.expr {
        Expr::Ident(name) => name.to_string(),
        _ => return None,
    };

    // Extract literal value from right side
    let value = match &right.expr {
        Expr::Literal(val) => match val {
            Val::Int(i) => LiteralValue::Int(*i),
            Val::UInt(u) => LiteralValue::Int(*u as i64),
            Val::Double(f) => LiteralValue::Float(*f),
            Val::String(s) => LiteralValue::String(s.to_string()),
            Val::Boolean(b) => LiteralValue::Bool(*b),
            _ => return None,
        },
        _ => return None,
    };

    match call.func_name.as_str() {
        operators::GREATER => {
            if let LiteralValue::Int(i) = value {
                Some(Predicate::Comparison {
                    var,
                    op: ComparisonOp::Gt,
                    value: LiteralValue::Int(i),
                })
            } else {
                None
            }
        }
        operators::GREATER_EQUALS => {
            if let LiteralValue::Int(i) = value {
                Some(Predicate::Comparison {
                    var,
                    op: ComparisonOp::Ge,
                    value: LiteralValue::Int(i),
                })
            } else {
                None
            }
        }
        operators::LESS => {
            if let LiteralValue::Int(i) = value {
                Some(Predicate::Comparison {
                    var,
                    op: ComparisonOp::Lt,
                    value: LiteralValue::Int(i),
                })
            } else {
                None
            }
        }
        operators::LESS_EQUALS => {
            if let LiteralValue::Int(i) = value {
                Some(Predicate::Comparison {
                    var,
                    op: ComparisonOp::Le,
                    value: LiteralValue::Int(i),
                })
            } else {
                None
            }
        }
        operators::EQUALS => {
            if let LiteralValue::String(s) = value {
                Some(Predicate::Equality {
                    var,
                    value: LiteralValue::String(s),
                    negated: false,
                })
            } else {
                None
            }
        }
        operators::NOT_EQUALS => {
            if let LiteralValue::String(s) = value {
                Some(Predicate::Equality {
                    var,
                    value: LiteralValue::String(s),
                    negated: true,
                })
            } else {
                None
            }
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
