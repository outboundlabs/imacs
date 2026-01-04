//! Smart variable matching across specs
//!
//! Detects when variables in different specs likely refer to the same concept,
//! even if they have different names or slightly different definitions.

use crate::spec::Variable;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of matching variables across specs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VariableMatchResult {
    pub matches: Vec<VariableMatch>,
}

/// A match between two variables from different specs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VariableMatch {
    pub var_a: Variable,
    pub var_b: Variable,
    pub spec_a: String,
    pub spec_b: String,
    pub confidence: f64,
    pub match_type: MatchType,
}

/// Type of match detected
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum MatchType {
    /// Exact match: same name, type, and values
    Exact,
    /// Same name, different values (potential collision)
    SameNameDifferentValues,
    /// Same name, different types (definite collision)
    SameNameDifferentTypes,
    /// Similar semantics based on name similarity and type
    SemanticSimilarity,
}

/// Match variables across a set of specs
pub fn match_variables(specs: &[(String, Vec<Variable>)]) -> VariableMatchResult {
    let mut matches = Vec::new();
    const MATCH_THRESHOLD: f64 = 0.3;

    for (i, (spec_a_id, vars_a)) in specs.iter().enumerate() {
        for (spec_b_id, vars_b) in specs.iter().skip(i + 1) {
            for var_a in vars_a {
                for var_b in vars_b {
                    let score = compute_match_score(var_a, var_b);
                    if score > MATCH_THRESHOLD {
                        matches.push(VariableMatch {
                            var_a: var_a.clone(),
                            var_b: var_b.clone(),
                            spec_a: spec_a_id.clone(),
                            spec_b: spec_b_id.clone(),
                            confidence: score,
                            match_type: classify_match(var_a, var_b),
                        });
                    }
                }
            }
        }
    }

    VariableMatchResult { matches }
}

/// Compute a match score between two variables (0.0 to 1.0)
fn compute_match_score(a: &Variable, b: &Variable) -> f64 {
    let mut score = 0.0;

    // Exact name match
    if a.name == b.name {
        score += 0.5;
    } else {
        // Similar name (simple heuristic)
        let similarity = name_similarity(&a.name, &b.name);
        score += 0.3 * similarity;
    }

    // Same type
    if a.typ == b.typ {
        score += 0.2;
    }

    // Similar values (for enums)
    if let (Some(vals_a), Some(vals_b)) = (&a.values, &b.values) {
        if !vals_a.is_empty() && !vals_b.is_empty() {
            let overlap = vals_a.iter().filter(|v| vals_b.contains(v)).count();
            let max_len = vals_a.len().max(vals_b.len());
            score += 0.3 * (overlap as f64 / max_len as f64);
        }
    }

    score.min(1.0)
}

/// Simple name similarity (0.0 to 1.0)
fn name_similarity(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }

    // Check if one contains the other
    if a.contains(b) || b.contains(a) {
        return 0.6;
    }

    // Check common prefix/suffix
    let common_prefix = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    let common_suffix = a
        .chars()
        .rev()
        .zip(b.chars().rev())
        .take_while(|(x, y)| x == y)
        .count();

    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 0.0;
    }

    ((common_prefix + common_suffix) as f64 / max_len as f64).min(1.0)
}

/// Classify the type of match
fn classify_match(a: &Variable, b: &Variable) -> MatchType {
    if a.name == b.name {
        if a.typ != b.typ {
            return MatchType::SameNameDifferentTypes;
        }
        if let (Some(vals_a), Some(vals_b)) = (&a.values, &b.values) {
            if vals_a != vals_b {
                return MatchType::SameNameDifferentValues;
            }
        }
        return MatchType::Exact;
    }

    MatchType::SemanticSimilarity
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::VarType;

    #[test]
    fn test_exact_match() {
        let var_a = Variable {
            name: "customer_type".into(),
            typ: VarType::String,
            description: None,
            values: Some(vec!["standard".into(), "premium".into()]),
        };
        let var_b = Variable {
            name: "customer_type".into(),
            typ: VarType::String,
            description: None,
            values: Some(vec!["standard".into(), "premium".into()]),
        };

        let score = compute_match_score(&var_a, &var_b);
        assert!(score > 0.9);
    }

    #[test]
    fn test_same_name_different_values() {
        let var_a = Variable {
            name: "customer_type".into(),
            typ: VarType::String,
            description: None,
            values: Some(vec!["standard".into(), "premium".into()]),
        };
        let var_b = Variable {
            name: "customer_type".into(),
            typ: VarType::String,
            description: None,
            values: Some(vec!["new".into(), "returning".into()]),
        };

        let match_type = classify_match(&var_a, &var_b);
        assert!(matches!(match_type, MatchType::SameNameDifferentValues));
    }

    #[test]
    fn test_name_similarity() {
        assert!(name_similarity("customer_type", "customer_type") > 0.9);
        assert!(name_similarity("customer_type", "customer_tier") > 0.5);
        assert!(name_similarity("customer_type", "region") < 0.3);
    }
}
