//! Specification types — the core data model
//!
//! A `Spec` defines decision logic as a set of rules. Each rule has:
//! - An ID for reference
//! - A condition (CEL expression or structured)
//! - An output value
//!
//! ## Example Spec
//!
//! ```yaml
//! id: login_attempt
//! name: "Login Attempt Validation"
//! inputs:
//!   - name: rate_exceeded
//!     type: bool
//!   - name: locked
//!     type: bool
//!   - name: valid_creds
//!     type: bool
//! outputs:
//!   - name: status
//!     type: int
//! rules:
//!   - id: R1
//!     when: "rate_exceeded"
//!     then: 429
//!   - id: R2
//!     when: "!rate_exceeded && locked"
//!     then: 423
//!   - id: R3
//!     when: "!rate_exceeded && !locked && !valid_creds"
//!     then: 401
//!   - id: R4
//!     when: "!rate_exceeded && !locked && valid_creds"
//!     then: 200
//! ```

use crate::error::{Error, Result};
use crate::render::ScopingConfig;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete specification
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "IMACS Spec", description = "Decision table specification")]
pub struct Spec {
    /// Unique identifier (used as function name)
    pub id: String,

    /// Human-readable name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Input variables
    #[serde(default)]
    pub inputs: Vec<Variable>,

    /// Output variables
    #[serde(default)]
    pub outputs: Vec<Variable>,

    /// Decision rules
    #[serde(default)]
    pub rules: Vec<Rule>,

    /// Default output if no rules match
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Output>,

    /// Metadata
    #[serde(default, skip_serializing_if = "SpecMeta::is_empty")]
    pub meta: SpecMeta,

    /// Namespace/scoping configuration for code generation
    /// Required for rendering - validates at render time
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scoping: Option<ScopingConfig>,
}

/// A variable (input or output)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Variable {
    /// Variable name
    pub name: String,

    /// Variable type
    #[serde(rename = "type")]
    pub typ: VarType,

    /// Description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// For enums: valid values
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

/// Variable types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum VarType {
    Bool,
    Int,
    Float,
    #[default]
    String,
    #[serde(rename = "enum")]
    Enum(Vec<String>),
    List(Box<VarType>),
    Object,
}

/// Condition clause - can be a single CEL expression or an array (AND'd together)
///
/// # Examples
///
/// Single expression:
/// ```yaml
/// when: "role == 'admin'"
/// ```
///
/// Array of expressions (AND'd):
/// ```yaml
/// when:
///   - role == 'member'
///   - verified
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum WhenClause {
    /// Single CEL expression
    Single(String),
    /// Multiple CEL expressions (AND'd together)
    Multiple(Vec<String>),
}

impl WhenClause {
    /// Convert to a single CEL expression (joining array items with &&)
    pub fn to_cel(&self) -> String {
        match self {
            WhenClause::Single(s) => s.clone(),
            WhenClause::Multiple(v) => {
                if v.is_empty() {
                    "true".to_string()
                } else if v.len() == 1 {
                    v[0].clone()
                } else {
                    // Wrap each condition in parens for safety, join with &&
                    v.iter()
                        .map(|s| format!("({})", s))
                        .collect::<Vec<_>>()
                        .join(" && ")
                }
            }
        }
    }
}

impl From<&str> for WhenClause {
    fn from(s: &str) -> Self {
        WhenClause::Single(s.to_string())
    }
}

impl From<String> for WhenClause {
    fn from(s: String) -> Self {
        WhenClause::Single(s)
    }
}

/// A decision rule
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Rule {
    /// Rule identifier
    pub id: String,

    /// CEL condition expression(s) - can be a single string or array of strings
    #[serde(rename = "when", default, skip_serializing_if = "Option::is_none")]
    pub when: Option<WhenClause>,

    /// Structured conditions (alternative to CEL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<Condition>>,

    /// Output value(s)
    #[serde(rename = "then")]
    pub then: Output,

    /// Priority (lower = higher priority)
    #[serde(default)]
    pub priority: i32,

    /// Description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A structured condition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct Condition {
    /// Variable name
    pub var: String,

    /// Comparison operator
    #[serde(default)]
    pub op: ConditionOp,

    /// Value to compare against
    pub value: ConditionValue,
}

/// Comparison operators
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ConditionOp {
    #[default]
    #[serde(rename = "==", alias = "eq")]
    Eq,
    #[serde(rename = "!=", alias = "ne")]
    Ne,
    #[serde(rename = "<", alias = "lt")]
    Lt,
    #[serde(rename = "<=", alias = "le")]
    Le,
    #[serde(rename = ">", alias = "gt")]
    Gt,
    #[serde(rename = ">=", alias = "ge")]
    Ge,
    #[serde(rename = "in")]
    In,
    Contains,
    StartsWith,
    EndsWith,
    Matches,
}

impl std::fmt::Display for ConditionOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConditionOp::Eq => write!(f, "=="),
            ConditionOp::Ne => write!(f, "!="),
            ConditionOp::Lt => write!(f, "<"),
            ConditionOp::Le => write!(f, "<="),
            ConditionOp::Gt => write!(f, ">"),
            ConditionOp::Ge => write!(f, ">="),
            ConditionOp::In => write!(f, "in"),
            ConditionOp::Contains => write!(f, "contains"),
            ConditionOp::StartsWith => write!(f, "startsWith"),
            ConditionOp::EndsWith => write!(f, "endsWith"),
            ConditionOp::Matches => write!(f, "matches"),
        }
    }
}

/// A condition value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum ConditionValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<ConditionValue>),
    Map(HashMap<String, ConditionValue>),
    Null,
}

impl std::fmt::Display for ConditionValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConditionValue::Bool(b) => write!(f, "{}", b),
            ConditionValue::Int(i) => write!(f, "{}", i),
            ConditionValue::Float(fl) => write!(f, "{}", fl),
            ConditionValue::String(s) => write!(f, "\"{}\"", s),
            ConditionValue::List(items) => {
                let strs: Vec<_> = items.iter().map(|i| i.to_string()).collect();
                write!(f, "[{}]", strs.join(", "))
            }
            ConditionValue::Map(m) => write!(f, "{:?}", m),
            ConditionValue::Null => write!(f, "null"),
        }
    }
}

/// Rule output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum Output {
    /// Single value
    Single(ConditionValue),
    /// Named fields
    Named(HashMap<String, ConditionValue>),
}

impl std::fmt::Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Output::Single(v) => write!(f, "{}", v),
            Output::Named(m) => {
                let pairs: Vec<_> = m.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{ {} }}", pairs.join(", "))
            }
        }
    }
}

/// Spec metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SpecMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl SpecMeta {
    pub fn is_empty(&self) -> bool {
        self.version.is_none()
            && self.author.is_none()
            && self.created.is_none()
            && self.updated.is_none()
            && self.tags.is_empty()
    }
}

impl Spec {
    /// Parse spec from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_norway::from_str(yaml).map_err(|e| Error::SpecParse(e.to_string()))
    }

    /// Serialize spec to YAML string
    pub fn to_yaml(&self) -> Result<String> {
        serde_norway::to_string(self).map_err(|e| Error::SpecParse(e.to_string()))
    }

    /// Parse spec from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| Error::SpecParse(e.to_string()))
    }

    /// Serialize spec to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| Error::SpecParse(e.to_string()))
    }

    /// Get a rule by ID
    pub fn get_rule(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// Compute hash of spec for change detection
    pub fn hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let content = self.to_yaml().unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("sha256:{}", hex::encode(&hasher.finalize()[..8]))
    }

    /// Validate spec for completeness
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.id.is_empty() {
            errors.push("Spec ID is required".into());
        }

        if self.inputs.is_empty() {
            errors.push("At least one input is required".into());
        }

        if self.rules.is_empty() {
            errors.push("At least one rule is required".into());
        }

        // Check for duplicate rule IDs
        let mut seen_ids = std::collections::HashSet::new();
        for rule in &self.rules {
            if !seen_ids.insert(&rule.id) {
                errors.push(format!("Duplicate rule ID: {}", rule.id));
            }
        }

        // Check that rule conditions reference valid inputs
        let input_names: std::collections::HashSet<_> =
            self.inputs.iter().map(|i| i.name.as_str()).collect();

        for rule in &self.rules {
            if let Some(conditions) = &rule.conditions {
                for cond in conditions {
                    if !input_names.contains(cond.var.as_str()) {
                        errors.push(format!(
                            "Rule {} references unknown input: {}",
                            rule.id, cond.var
                        ));
                    }
                }
            }
        }

        // PY-2: Warn if no default rule (exhaustiveness not guaranteed)
        if self.default.is_none() && !self.rules.is_empty() {
            errors.push("Warning: No default rule - exhaustiveness not guaranteed".into());
        }

        errors
    }
}

impl Rule {
    /// Get condition as CEL expression
    pub fn as_cel(&self) -> Option<String> {
        if let Some(when_clause) = &self.when {
            return Some(when_clause.to_cel());
        }

        // Convert structured conditions to CEL
        if let Some(conditions) = &self.conditions {
            if conditions.is_empty() {
                return None;
            }
            let parts: Vec<String> = conditions.iter().map(|c| c.to_cel()).collect();
            return Some(parts.join(" && "));
        }

        None
    }
}

impl Condition {
    /// Convert to CEL expression fragment
    pub fn to_cel(&self) -> String {
        let val_str = match &self.value {
            ConditionValue::Bool(b) => b.to_string(),
            ConditionValue::Int(i) => i.to_string(),
            ConditionValue::Float(f) => f.to_string(),
            ConditionValue::String(s) => format!("\"{}\"", s),
            ConditionValue::List(items) => {
                let inner: Vec<_> = items
                    .iter()
                    .map(|v| match v {
                        ConditionValue::String(s) => format!("\"{}\"", s),
                        ConditionValue::Int(i) => i.to_string(),
                        _ => "?".to_string(),
                    })
                    .collect();
                format!("[{}]", inner.join(", "))
            }
            _ => "?".to_string(),
        };

        match self.op {
            ConditionOp::Eq => format!("{} == {}", self.var, val_str),
            ConditionOp::Ne => format!("{} != {}", self.var, val_str),
            ConditionOp::Lt => format!("{} < {}", self.var, val_str),
            ConditionOp::Le => format!("{} <= {}", self.var, val_str),
            ConditionOp::Gt => format!("{} > {}", self.var, val_str),
            ConditionOp::Ge => format!("{} >= {}", self.var, val_str),
            ConditionOp::In => format!("{} in {}", self.var, val_str),
            ConditionOp::Contains => format!("{}.contains({})", self.var, val_str),
            ConditionOp::StartsWith => format!("{}.startsWith({})", self.var, val_str),
            ConditionOp::EndsWith => format!("{}.endsWith({})", self.var, val_str),
            ConditionOp::Matches => format!("{}.matches({})", self.var, val_str),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml() {
        let yaml = r#"
id: test_spec
inputs:
  - name: x
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    when: "x"
    then: 1
  - id: R2
    when: "!x"
    then: 0
"#;
        let spec = Spec::from_yaml(yaml).unwrap();
        assert_eq!(spec.id, "test_spec");
        assert_eq!(spec.inputs.len(), 1);
        assert_eq!(spec.rules.len(), 2);
    }

    #[test]
    fn test_validate() {
        let spec = Spec {
            id: "".into(),
            name: None,
            description: None,
            inputs: vec![],
            outputs: vec![],
            rules: vec![],
            default: None,
            meta: SpecMeta::default(),
            scoping: None,
        };

        let errors = spec.validate();
        assert!(errors.iter().any(|e| e.contains("ID")));
        assert!(errors.iter().any(|e| e.contains("input")));
        assert!(errors.iter().any(|e| e.contains("rule")));
    }

    #[test]
    fn test_condition_to_cel() {
        let cond = Condition {
            var: "amount".into(),
            op: ConditionOp::Gt,
            value: ConditionValue::Int(1000),
        };
        assert_eq!(cond.to_cel(), "amount > 1000");

        let cond2 = Condition {
            var: "status".into(),
            op: ConditionOp::In,
            value: ConditionValue::List(vec![
                ConditionValue::String("active".into()),
                ConditionValue::String("pending".into()),
            ]),
        };
        assert_eq!(cond2.to_cel(), "status in [\"active\", \"pending\"]");
    }

    #[test]
    fn test_when_clause_single() {
        let clause = WhenClause::Single("role == 'admin'".into());
        assert_eq!(clause.to_cel(), "role == 'admin'");
    }

    #[test]
    fn test_when_clause_multiple() {
        let clause = WhenClause::Multiple(vec!["role == 'member'".into(), "verified".into()]);
        assert_eq!(clause.to_cel(), "(role == 'member') && (verified)");
    }

    #[test]
    fn test_when_clause_multiple_single_item() {
        let clause = WhenClause::Multiple(vec!["role == 'admin'".into()]);
        assert_eq!(clause.to_cel(), "role == 'admin'");
    }

    #[test]
    fn test_when_clause_multiple_empty() {
        let clause = WhenClause::Multiple(vec![]);
        assert_eq!(clause.to_cel(), "true");
    }

    #[test]
    fn test_parse_yaml_with_array_when() {
        let yaml = r#"
id: test_spec
inputs:
  - name: role
    type: string
  - name: verified
    type: bool
outputs:
  - name: level
    type: int
rules:
  - id: R1
    when:
      - role == 'admin'
    then: 100
  - id: R2
    when:
      - role == 'member'
      - verified
    then: 50
"#;
        let spec = Spec::from_yaml(yaml).unwrap();
        assert_eq!(spec.id, "test_spec");
        assert_eq!(spec.rules.len(), 2);

        // R1 has single condition in array
        assert_eq!(spec.rules[0].as_cel(), Some("role == 'admin'".into()));

        // R2 has multiple conditions
        assert_eq!(
            spec.rules[1].as_cel(),
            Some("(role == 'member') && (verified)".into())
        );
    }

    #[test]
    fn test_parse_yaml_with_string_when() {
        // Ensure backward compatibility with string when
        let yaml = r#"
id: test_spec
inputs:
  - name: x
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    when: "x && y"
    then: 1
"#;
        let spec = Spec::from_yaml(yaml).unwrap();
        assert_eq!(spec.rules[0].as_cel(), Some("x && y".into()));
    }
}
