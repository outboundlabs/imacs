//! Predicate extraction from CEL expressions
//!
//! Converts CEL conditions into atomic predicates that can be analyzed
//! using boolean logic techniques (Quine-McCluskey).
//!
//! A predicate is an atomic boolean-valued expression like:
//! - `amount > 1000` (comparison)
//! - `region == "EU"` (equality)
//! - `rate_exceeded` (boolean variable)
//! - `status in ["active", "pending"]` (membership)

use crate::cel::CelCompiler;
use crate::error::Result;
use cel_parser::ast::{operators, CallExpr, Expr};
use cel_parser::reference::Val;
pub use cel_parser::Expression as CelExpr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// An atomic predicate extracted from CEL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Predicate {
    /// Direct boolean variable: `rate_exceeded`
    BoolVar(String),

    /// Comparison: `amount > 1000`, `count <= 5`
    Comparison {
        var: String,
        op: ComparisonOp,
        value: LiteralValue,
    },

    /// Equality: `status == "active"`
    Equality {
        var: String,
        value: LiteralValue,
        negated: bool,
    },

    /// Membership: `region in ["US", "EU"]`
    Membership {
        var: String,
        values: Vec<LiteralValue>,
        negated: bool,
    },

    /// String operation: `name.startsWith("test")`
    StringOp {
        var: String,
        op: StringOpKind,
        arg: String,
        negated: bool,
    },
}

impl PartialEq for Predicate {
    fn eq(&self, other: &Self) -> bool {
        self.to_cel_string() == other.to_cel_string()
    }
}

impl Eq for Predicate {}

impl Hash for Predicate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_cel_string().hash(state);
    }
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComparisonOp {
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
}

impl std::fmt::Display for ComparisonOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparisonOp::Lt => write!(f, "<"),
            ComparisonOp::Le => write!(f, "<="),
            ComparisonOp::Gt => write!(f, ">"),
            ComparisonOp::Ge => write!(f, ">="),
        }
    }
}

/// String operation kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StringOpKind {
    StartsWith,
    EndsWith,
    Contains,
    Matches,
}

impl std::fmt::Display for StringOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringOpKind::StartsWith => write!(f, "startsWith"),
            StringOpKind::EndsWith => write!(f, "endsWith"),
            StringOpKind::Contains => write!(f, "contains"),
            StringOpKind::Matches => write!(f, "matches"),
        }
    }
}

/// Literal values in predicates
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl std::fmt::Display for LiteralValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteralValue::Bool(b) => write!(f, "{}", b),
            LiteralValue::Int(i) => write!(f, "{}", i),
            LiteralValue::Float(fl) => write!(f, "{}", fl),
            LiteralValue::String(s) => write!(f, "\"{}\"", s),
        }
    }
}

impl Predicate {
    /// Convert predicate back to CEL string representation
    pub fn to_cel_string(&self) -> String {
        match self {
            Predicate::BoolVar(name) => name.clone(),

            Predicate::Comparison { var, op, value } => {
                format!("{} {} {}", var, op, value)
            }

            Predicate::Equality {
                var,
                value,
                negated,
            } => {
                if *negated {
                    format!("{} != {}", var, value)
                } else {
                    format!("{} == {}", var, value)
                }
            }

            Predicate::Membership {
                var,
                values,
                negated,
            } => {
                let vals: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                if *negated {
                    format!("!({} in [{}])", var, vals.join(", "))
                } else {
                    format!("{} in [{}]", var, vals.join(", "))
                }
            }

            Predicate::StringOp {
                var,
                op,
                arg,
                negated,
            } => {
                let expr = format!("{}.{}(\"{}\")", var, op, arg);
                if *negated {
                    format!("!{}", expr)
                } else {
                    expr
                }
            }
        }
    }

    /// Get the negated form of this predicate
    pub fn negated(&self) -> Predicate {
        match self {
            Predicate::BoolVar(name) => Predicate::BoolVar(format!("!{}", name)),

            Predicate::Comparison { var, op, value } => {
                // Negate comparison by flipping operator
                let neg_op = match op {
                    ComparisonOp::Lt => ComparisonOp::Ge,
                    ComparisonOp::Le => ComparisonOp::Gt,
                    ComparisonOp::Gt => ComparisonOp::Le,
                    ComparisonOp::Ge => ComparisonOp::Lt,
                };
                Predicate::Comparison {
                    var: var.clone(),
                    op: neg_op,
                    value: value.clone(),
                }
            }

            Predicate::Equality {
                var,
                value,
                negated,
            } => Predicate::Equality {
                var: var.clone(),
                value: value.clone(),
                negated: !negated,
            },

            Predicate::Membership {
                var,
                values,
                negated,
            } => Predicate::Membership {
                var: var.clone(),
                values: values.clone(),
                negated: !negated,
            },

            Predicate::StringOp {
                var,
                op,
                arg,
                negated,
            } => Predicate::StringOp {
                var: var.clone(),
                op: *op,
                arg: arg.clone(),
                negated: !negated,
            },
        }
    }
}

/// A set of unique predicates with index mapping
#[derive(Debug, Clone)]
pub struct PredicateSet {
    /// All unique predicates
    pub predicates: Vec<Predicate>,
    /// Map from predicate to index
    index_map: HashMap<String, usize>,
}

impl PredicateSet {
    pub fn new() -> Self {
        Self {
            predicates: Vec::new(),
            index_map: HashMap::new(),
        }
    }

    /// Add a predicate, returning its index
    pub fn add(&mut self, pred: Predicate) -> usize {
        let key = pred.to_cel_string();
        if let Some(&idx) = self.index_map.get(&key) {
            idx
        } else {
            let idx = self.predicates.len();
            self.index_map.insert(key, idx);
            self.predicates.push(pred);
            idx
        }
    }

    /// Get predicate by index
    pub fn get(&self, idx: usize) -> Option<&Predicate> {
        self.predicates.get(idx)
    }

    /// Get index of predicate
    pub fn index_of(&self, pred: &Predicate) -> Option<usize> {
        self.index_map.get(&pred.to_cel_string()).copied()
    }

    /// Number of predicates
    pub fn len(&self) -> usize {
        self.predicates.len()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.predicates.is_empty()
    }
}

impl Default for PredicateSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract all atomic predicates from a CEL expression string
pub fn extract_predicates(cel_expr: &str) -> Result<Vec<Predicate>> {
    let ast = CelCompiler::parse(cel_expr)?;
    let mut predicates = Vec::new();
    extract_from_ast(&ast, &mut predicates, false);
    Ok(predicates)
}

/// Recursively extract predicates from CEL AST
fn extract_from_ast(expr: &CelExpr, predicates: &mut Vec<Predicate>, negated: bool) {
    // In cel-parser 0.10, Expression is IdedExpr with expr field
    match &expr.expr {
        // Simple identifier - treat as boolean variable
        Expr::Ident(name) => {
            let name_str = name.to_string();
            if name_str != "true" && name_str != "false" {
                // Always store positive form of boolean variable
                predicates.push(Predicate::BoolVar(name_str));
            }
        }

        // Call expressions - operators and function calls
        Expr::Call(call) => {
            // Check for logical operators
            if call.func_name == operators::LOGICAL_AND {
                // AND: recurse into both sides
                if call.args.len() == 2 {
                    extract_from_ast(&call.args[0], predicates, negated);
                    extract_from_ast(&call.args[1], predicates, negated);
                }
            } else if call.func_name == operators::LOGICAL_OR {
                // OR: recurse into both sides
                if call.args.len() == 2 {
                    extract_from_ast(&call.args[0], predicates, negated);
                    extract_from_ast(&call.args[1], predicates, negated);
                }
            } else if call.func_name == operators::LOGICAL_NOT {
                // NOT: flip negation and recurse
                if let Some(inner) = call.args.first() {
                    extract_from_ast(inner, predicates, !negated);
                }
            } else if is_relation_op(&call.func_name) {
                // Relation operator (==, !=, <, >, <=, >=)
                if call.args.len() == 2 {
                    if let Some(pred) = extract_relation_from_call(call, negated) {
                        predicates.push(pred);
                    }
                }
            } else {
                // Other function calls - extract from arguments
                for arg in &call.args {
                    extract_from_ast(arg, predicates, negated);
                }
            }
        }

        // Select expressions - member access
        Expr::Select(select) => {
            // Extract from base operand
            extract_from_ast(&select.operand, predicates, negated);
        }

        // List literals - extract from elements
        Expr::List(list) => {
            for item in &list.elements {
                extract_from_ast(item, predicates, negated);
            }
        }

        // Map literals - no predicates (for now)
        Expr::Map(_) => {}

        // Literals - no predicates
        Expr::Literal(_) => {}

        // Other expression types
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
        || func_name == operators::IN
}

/// Extract predicate from a relation Call expression
fn extract_relation_from_call(call: &CallExpr, negated: bool) -> Option<Predicate> {
    if call.args.len() != 2 {
        return None;
    }

    let left = &call.args[0];
    let right = &call.args[1];

    // Get variable name from left side
    let var = match &left.expr {
        Expr::Ident(name) => name.to_string(),
        Expr::Select(_select) => {
            // Handle nested member access like user.status
            format_member_path(left)
        }
        _ => return None,
    };

    // Handle `in` operator first (RHS is a list, not a literal)
    if call.func_name == operators::IN {
        if let Expr::List(list) = &right.expr {
            let values: Vec<LiteralValue> =
                list.elements.iter().filter_map(extract_literal).collect();
            if !values.is_empty() {
                return Some(Predicate::Membership {
                    var,
                    values,
                    negated,
                });
            }
        }
        return None;
    }

    // Get literal value from right side (for non-list operators)
    let value = extract_literal(right)?;

    match call.func_name.as_str() {
        operators::EQUALS => Some(Predicate::Equality {
            var,
            value,
            negated,
        }),

        operators::NOT_EQUALS => Some(Predicate::Equality {
            var,
            value,
            negated: !negated,
        }),

        operators::LESS => {
            let op = if negated {
                ComparisonOp::Ge
            } else {
                ComparisonOp::Lt
            };
            Some(Predicate::Comparison { var, op, value })
        }

        operators::LESS_EQUALS => {
            let op = if negated {
                ComparisonOp::Gt
            } else {
                ComparisonOp::Le
            };
            Some(Predicate::Comparison { var, op, value })
        }

        operators::GREATER => {
            let op = if negated {
                ComparisonOp::Le
            } else {
                ComparisonOp::Gt
            };
            Some(Predicate::Comparison { var, op, value })
        }

        operators::GREATER_EQUALS => {
            let op = if negated {
                ComparisonOp::Lt
            } else {
                ComparisonOp::Ge
            };
            Some(Predicate::Comparison { var, op, value })
        }

        _ => None,
    }
}

/// Extract literal value from CEL expression
fn extract_literal(expr: &CelExpr) -> Option<LiteralValue> {
    match &expr.expr {
        Expr::Literal(val) => match val {
            Val::Int(i) => Some(LiteralValue::Int(*i)),
            Val::UInt(u) => Some(LiteralValue::Int(*u as i64)),
            Val::Double(f) => Some(LiteralValue::Float(*f)),
            Val::String(s) => Some(LiteralValue::String(s.to_string())),
            Val::Boolean(b) => Some(LiteralValue::Bool(*b)),
            Val::Null => None,
            _ => None,
        },
        Expr::Ident(name) => {
            let s = name.to_string();
            if s == "true" {
                Some(LiteralValue::Bool(true))
            } else if s == "false" {
                Some(LiteralValue::Bool(false))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Format member path like user.profile.status
fn format_member_path(expr: &CelExpr) -> String {
    match &expr.expr {
        Expr::Ident(name) => name.to_string(),
        Expr::Select(select) => {
            let base_str = format_member_path(&select.operand);
            if !select.field.is_empty() {
                format!("{}.{}", base_str, select.field)
            } else {
                base_str
            }
        }
        _ => "?".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bool_var() {
        let preds = extract_predicates("rate_exceeded").unwrap();
        assert_eq!(preds.len(), 1);
        assert!(matches!(&preds[0], Predicate::BoolVar(name) if name == "rate_exceeded"));
    }

    #[test]
    fn test_extract_comparison() {
        let preds = extract_predicates("amount > 1000").unwrap();
        assert_eq!(preds.len(), 1);
        assert!(matches!(
            &preds[0],
            Predicate::Comparison { var, op: ComparisonOp::Gt, value: LiteralValue::Int(1000) }
            if var == "amount"
        ));
    }

    #[test]
    fn test_extract_equality() {
        let preds = extract_predicates("status == \"active\"").unwrap();
        assert_eq!(preds.len(), 1);
        assert!(matches!(
            &preds[0],
            Predicate::Equality { var, value: LiteralValue::String(s), negated: false }
            if var == "status" && s == "active"
        ));
    }

    #[test]
    fn test_extract_and() {
        let preds = extract_predicates("amount > 1000 && rate_exceeded").unwrap();
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn test_extract_negation() {
        // Negated booleans are normalized to positive form
        // The negation context is handled during analysis
        let preds = extract_predicates("!rate_exceeded").unwrap();
        assert_eq!(preds.len(), 1);
        assert!(matches!(&preds[0], Predicate::BoolVar(name) if name == "rate_exceeded"));
    }

    #[test]
    fn test_extract_membership() {
        let preds = extract_predicates("region in [\"US\", \"EU\"]").unwrap();
        assert_eq!(preds.len(), 1);
        assert!(matches!(
            &preds[0],
            Predicate::Membership { var, values, negated: false }
            if var == "region" && values.len() == 2
        ));
    }

    #[test]
    fn test_predicate_to_cel() {
        let pred = Predicate::Comparison {
            var: "amount".into(),
            op: ComparisonOp::Gt,
            value: LiteralValue::Int(1000),
        };
        assert_eq!(pred.to_cel_string(), "amount > 1000");
    }

    #[test]
    fn test_predicate_set() {
        let mut set = PredicateSet::new();
        let pred = Predicate::BoolVar("test".into());

        let idx1 = set.add(pred.clone());
        let idx2 = set.add(pred.clone());

        assert_eq!(idx1, idx2);
        assert_eq!(set.len(), 1);
    }
}
