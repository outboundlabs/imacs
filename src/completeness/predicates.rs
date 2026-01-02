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
use crate::error::{Error, Result};
use cel_parser::{Atom, Expression as CelExpr, Member, RelationOp};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    Lt,  // <
    Le,  // <=
    Gt,  // >
    Ge,  // >=
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

            Predicate::Equality { var, value, negated } => {
                if *negated {
                    format!("{} != {}", var, value)
                } else {
                    format!("{} == {}", var, value)
                }
            }

            Predicate::Membership { var, values, negated } => {
                let vals: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                if *negated {
                    format!("!({} in [{}])", var, vals.join(", "))
                } else {
                    format!("{} in [{}]", var, vals.join(", "))
                }
            }

            Predicate::StringOp { var, op, arg, negated } => {
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

            Predicate::Equality { var, value, negated } => {
                Predicate::Equality {
                    var: var.clone(),
                    value: value.clone(),
                    negated: !negated,
                }
            }

            Predicate::Membership { var, values, negated } => {
                Predicate::Membership {
                    var: var.clone(),
                    values: values.clone(),
                    negated: !negated,
                }
            }

            Predicate::StringOp { var, op, arg, negated } => {
                Predicate::StringOp {
                    var: var.clone(),
                    op: *op,
                    arg: arg.clone(),
                    negated: !negated,
                }
            }
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
    match expr {
        // Simple identifier - treat as boolean variable
        // Always use positive form - negation is tracked by the caller
        CelExpr::Ident(name) => {
            let name_str = name.to_string();
            if name_str != "true" && name_str != "false" {
                // Always store positive form of boolean variable
                predicates.push(Predicate::BoolVar(name_str));
            }
        }

        // Relation: amount > 1000, status == "active"
        CelExpr::Relation(left, op, right) => {
            if let Some(pred) = extract_relation(left, op, right, negated) {
                predicates.push(pred);
            }
        }

        // AND: recurse into both sides
        CelExpr::And(left, right) => {
            extract_from_ast(left, predicates, negated);
            extract_from_ast(right, predicates, negated);
        }

        // OR: recurse into both sides
        CelExpr::Or(left, right) => {
            extract_from_ast(left, predicates, negated);
            extract_from_ast(right, predicates, negated);
        }

        // NOT: flip negation and recurse
        CelExpr::Unary(cel_parser::UnaryOp::Not, inner) => {
            extract_from_ast(inner, predicates, !negated);
        }

        // Negation (minus)
        CelExpr::Unary(cel_parser::UnaryOp::Minus, inner) => {
            extract_from_ast(inner, predicates, negated);
        }

        // Double not
        CelExpr::Unary(cel_parser::UnaryOp::DoubleNot, inner) => {
            extract_from_ast(inner, predicates, negated);
        }

        // Double minus
        CelExpr::Unary(cel_parser::UnaryOp::DoubleMinus, inner) => {
            extract_from_ast(inner, predicates, negated);
        }

        // Ternary: extract from all branches
        CelExpr::Ternary(cond, true_branch, false_branch) => {
            extract_from_ast(cond, predicates, negated);
            extract_from_ast(true_branch, predicates, negated);
            extract_from_ast(false_branch, predicates, negated);
        }

        // Member access (function calls like size(), has(), etc.)
        CelExpr::Member(base, member) => {
            extract_from_member(base, member, predicates, negated);
        }

        // List literals - no predicates
        CelExpr::List(_) => {}

        // Map literals - no predicates
        CelExpr::Map(_) => {}

        // Atoms (literals) - no predicates
        CelExpr::Atom(_) => {}

        // Arithmetic - recurse to find any embedded predicates
        CelExpr::Arithmetic(left, _, right) => {
            extract_from_ast(left, predicates, negated);
            extract_from_ast(right, predicates, negated);
        }
    }
}

/// Extract predicate from a relation expression
fn extract_relation(
    left: &CelExpr,
    op: &RelationOp,
    right: &CelExpr,
    negated: bool,
) -> Option<Predicate> {
    // Get variable name from left side
    let var = match left {
        CelExpr::Ident(name) => name.to_string(),
        CelExpr::Member(_base, _) => {
            // Handle nested member access like user.status
            format_member_path(left)
        }
        _ => return None,
    };

    // Handle `in` operator first (RHS is a list, not a literal)
    if *op == RelationOp::In {
        if let CelExpr::List(items) = right {
            let values: Vec<LiteralValue> = items
                .iter()
                .filter_map(extract_literal)
                .collect();
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

    match op {
        RelationOp::Equals => Some(Predicate::Equality {
            var,
            value,
            negated,
        }),

        RelationOp::NotEquals => Some(Predicate::Equality {
            var,
            value,
            negated: !negated,
        }),

        RelationOp::LessThan => {
            let op = if negated { ComparisonOp::Ge } else { ComparisonOp::Lt };
            Some(Predicate::Comparison { var, op, value })
        }

        RelationOp::LessThanEq => {
            let op = if negated { ComparisonOp::Gt } else { ComparisonOp::Le };
            Some(Predicate::Comparison { var, op, value })
        }

        RelationOp::GreaterThan => {
            let op = if negated { ComparisonOp::Le } else { ComparisonOp::Gt };
            Some(Predicate::Comparison { var, op, value })
        }

        RelationOp::GreaterThanEq => {
            let op = if negated { ComparisonOp::Lt } else { ComparisonOp::Ge };
            Some(Predicate::Comparison { var, op, value })
        }

        RelationOp::In => unreachable!(), // Already handled above
    }
}

/// Extract literal value from CEL expression
fn extract_literal(expr: &CelExpr) -> Option<LiteralValue> {
    match expr {
        CelExpr::Atom(atom) => match atom {
            Atom::Int(i) => Some(LiteralValue::Int(*i)),
            Atom::UInt(u) => Some(LiteralValue::Int(*u as i64)),
            Atom::Float(f) => Some(LiteralValue::Float(*f)),
            Atom::String(s) => Some(LiteralValue::String(s.to_string())),
            Atom::Bool(b) => Some(LiteralValue::Bool(*b)),
            Atom::Null => None,
            Atom::Bytes(_) => None,
        },
        CelExpr::Ident(name) => {
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
    match expr {
        CelExpr::Ident(name) => name.to_string(),
        CelExpr::Member(base, member) => {
            let base_str = format_member_path(base);
            match member.as_ref() {
                Member::Attribute(attr) => format!("{}.{}", base_str, attr),
                Member::Index(idx) => format!("{}[{}]", base_str, format_member_path(idx)),
                Member::Fields(_) => base_str,
                Member::FunctionCall(_) => base_str,
            }
        }
        _ => "?".to_string(),
    }
}

/// Extract predicates from member access (function calls)
fn extract_from_member(
    base: &CelExpr,
    member: &Member,
    predicates: &mut Vec<Predicate>,
    negated: bool,
) {
    match member {
        // Method call like str.startsWith("prefix")
        Member::FunctionCall(args) => {
            // Check if base is an identifier (top-level function call)
            if let CelExpr::Ident(func_name) = base {
                let func = func_name.to_string();
                // Functions like has(), size() - extract from args
                for arg in args {
                    extract_from_ast(arg, predicates, negated);
                }
                return;
            }

            // Method call on a variable
            if let CelExpr::Member(obj, boxed_member) = base {
                if let Member::Attribute(method) = boxed_member.as_ref() {
                    let var = format_member_path(obj);
                    let method_str = method.to_string();

                    if let Some(arg_expr) = args.first() {
                        if let Some(LiteralValue::String(arg)) = extract_literal(arg_expr) {
                            let op = match method_str.as_str() {
                                "startsWith" => Some(StringOpKind::StartsWith),
                                "endsWith" => Some(StringOpKind::EndsWith),
                                "contains" => Some(StringOpKind::Contains),
                                "matches" => Some(StringOpKind::Matches),
                                _ => None,
                            };

                            if let Some(op) = op {
                                predicates.push(Predicate::StringOp {
                                    var,
                                    op,
                                    arg,
                                    negated,
                                });
                                return;
                            }
                        }
                    }
                }
            }

            // Fallback: extract from base and args
            extract_from_ast(base, predicates, negated);
            for arg in args {
                extract_from_ast(arg, predicates, negated);
            }
        }

        // Attribute access - recurse on base
        Member::Attribute(_) | Member::Index(_) | Member::Fields(_) => {
            extract_from_ast(base, predicates, negated);
        }
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
