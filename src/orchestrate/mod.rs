//! Orchestration — chain multiple specs into workflows
//!
//! Orchestrators compose specs into pipelines with:
//! - Sequential execution (call spec A, then B, then C)
//! - Parallel execution (call A, B, C simultaneously)
//! - Conditional branching (call A if X, else B)
//! - Loops and iteration (foreach item in collection)
//! - Gates and guards (fail fast if condition not met)
//! - Error handling (try/catch/finally)
//!
//! This module uses genco for code generation across languages.

mod rust;
mod typescript;
mod python;
mod csharp;
mod java;
mod go;

use crate::cel::Target;
use crate::spec::{Spec, VarType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export renderers
pub use rust::render as render_rust;
pub use typescript::render as render_typescript;
pub use python::render as render_python;
pub use csharp::render as render_csharp;
pub use java::render as render_java;
pub use go::render as render_go;

/// Render an orchestrator to target language
pub fn render_orchestrator(
    orch: &Orchestrator,
    specs: &HashMap<String, Spec>,
    target: Target,
) -> String {
    match target {
        Target::Rust => rust::render(orch, specs),
        Target::TypeScript => typescript::render(orch, specs),
        Target::Python => python::render(orch, specs),
        Target::CSharp => csharp::render(orch, specs),
        Target::Go => go::render(orch, specs),
        Target::Java => java::render(orch, specs),
    }
}

/// An orchestrator composes multiple specs into a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orchestrator {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    #[serde(default)]
    pub name: Option<String>,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Inputs to the orchestrator
    #[serde(default)]
    pub inputs: Vec<OrchestratorInput>,
    /// Outputs from the orchestrator
    #[serde(default)]
    pub outputs: Vec<OrchestratorOutput>,
    /// Specs this orchestrator uses
    #[serde(default)]
    pub uses: Vec<String>,
    /// The chain of steps
    #[serde(default)]
    pub chain: Vec<ChainStep>,
}

impl Orchestrator {
    /// Parse orchestrator from YAML
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Get all specs referenced by this orchestrator
    pub fn referenced_specs(&self) -> Vec<String> {
        let mut specs = self.uses.clone();
        self.collect_specs_from_chain(&self.chain, &mut specs);
        specs.sort();
        specs.dedup();
        specs
    }

    fn collect_specs_from_chain(&self, steps: &[ChainStep], specs: &mut Vec<String>) {
        for step in steps {
            match step {
                ChainStep::Call(call) => specs.push(call.spec.clone()),
                ChainStep::Parallel(par) => self.collect_specs_from_chain(&par.steps, specs),
                ChainStep::Branch(branch) => {
                    for steps in branch.cases.values() {
                        self.collect_specs_from_chain(steps, specs);
                    }
                    if let Some(default) = &branch.default {
                        self.collect_specs_from_chain(default, specs);
                    }
                }
                ChainStep::Loop(loop_) => self.collect_specs_from_chain(&loop_.steps, specs),
                ChainStep::ForEach(foreach) => self.collect_specs_from_chain(&foreach.steps, specs),
                ChainStep::Try(try_) => {
                    self.collect_specs_from_chain(&try_.try_steps, specs);
                    if let Some(catch) = &try_.catch {
                        self.collect_specs_from_chain(&catch.steps, specs);
                    }
                    if let Some(finally) = &try_.finally {
                        self.collect_specs_from_chain(finally, specs);
                    }
                }
                ChainStep::Dynamic(dyn_) => specs.extend(dyn_.allowed.clone()),
                _ => {}
            }
        }
    }

    /// Validate the orchestrator against the specs
    pub fn validate(&self, specs: &HashMap<String, Spec>) -> Vec<String> {
        let mut errors = Vec::new();

        // PY-3: Check for duplicate step IDs
        let ids = collect_step_ids(&self.chain);
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        if ids.len() != unique.len() {
            // Find the duplicates
            let mut seen = std::collections::HashSet::new();
            for id in &ids {
                if !seen.insert(id) {
                    errors.push(format!("Duplicate step ID: {}", id));
                }
            }
        }

        // Check all referenced specs exist
        for spec_id in self.referenced_specs() {
            if !specs.contains_key(&spec_id) {
                errors.push(format!("Missing spec: {}", spec_id));
            }
        }

        // Check inputs are provided for all call steps
        self.validate_chain(&self.chain, specs, &mut errors);

        errors
    }

    fn validate_chain(&self, steps: &[ChainStep], specs: &HashMap<String, Spec>, errors: &mut Vec<String>) {
        for step in steps {
            match step {
                ChainStep::Call(call) => {
                    if let Some(spec) = specs.get(&call.spec) {
                        for input in &spec.inputs {
                            if !call.inputs.contains_key(&input.name) {
                                errors.push(format!(
                                    "Step '{}' missing required input '{}' for spec '{}'",
                                    call.id, input.name, call.spec
                                ));
                            }
                        }
                    }
                }
                ChainStep::Parallel(par) => self.validate_chain(&par.steps, specs, errors),
                ChainStep::Branch(branch) => {
                    for steps in branch.cases.values() {
                        self.validate_chain(steps, specs, errors);
                    }
                    if let Some(default) = &branch.default {
                        self.validate_chain(default, specs, errors);
                    }
                }
                ChainStep::Loop(loop_) => self.validate_chain(&loop_.steps, specs, errors),
                ChainStep::ForEach(foreach) => self.validate_chain(&foreach.steps, specs, errors),
                ChainStep::Try(try_) => {
                    self.validate_chain(&try_.try_steps, specs, errors);
                    if let Some(catch) = &try_.catch {
                        self.validate_chain(&catch.steps, specs, errors);
                    }
                    if let Some(finally) = &try_.finally {
                        self.validate_chain(finally, specs, errors);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Input to an orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorInput {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: VarType,
    #[serde(default)]
    pub description: Option<String>,
}

/// Output from an orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorOutput {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: VarType,
    #[serde(default)]
    pub description: Option<String>,
}

/// A step in the orchestration chain
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "lowercase")]
pub enum ChainStep {
    /// Call a spec
    Call(CallStep),
    /// Execute steps in parallel
    Parallel(ParallelStep),
    /// Branch based on condition
    Branch(BranchStep),
    /// Loop with counter
    Loop(LoopStep),
    /// Iterate over collection
    ForEach(ForEachStep),
    /// Guard/gate - fail if condition not met
    Gate(GateStep),
    /// Early return
    Return(ReturnStep),
    /// Compute a value
    Compute(ComputeStep),
    /// Set a context value
    Set(SetStep),
    /// Try/catch/finally
    Try(TryStep),
    /// Dynamic spec dispatch
    Dynamic(DynamicStep),
    /// Await async result
    Await(AwaitStep),
    /// Emit an event
    Emit(EmitStep),
}

/// Call a spec with mapped inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallStep {
    /// Step identifier
    pub id: String,
    /// Spec to call
    pub spec: String,
    /// Input mappings: spec_input -> expression
    #[serde(default)]
    pub inputs: HashMap<String, String>,
    /// Output mappings: local_name -> spec_output
    #[serde(default)]
    pub outputs: HashMap<String, String>,
    /// Optional condition for execution
    #[serde(default)]
    pub condition: Option<String>,
    /// Timeout in milliseconds
    #[serde(default)]
    pub timeout: Option<u64>,
    /// Retry configuration
    #[serde(default)]
    pub retry: Option<RetryConfig>,
}

/// Execute steps in parallel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelStep {
    pub id: String,
    pub steps: Vec<ChainStep>,
    /// Wait strategy: all, any, first_success
    #[serde(default = "default_wait_all")]
    pub wait: WaitStrategy,
}

fn default_wait_all() -> WaitStrategy {
    WaitStrategy::All
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WaitStrategy {
    #[default]
    All,
    Any,
    FirstSuccess,
}

/// Branch based on expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchStep {
    pub id: String,
    /// Expression to match on
    pub on: String,
    /// Case value -> steps
    pub cases: HashMap<String, Vec<ChainStep>>,
    /// Default case
    #[serde(default)]
    pub default: Option<Vec<ChainStep>>,
}

/// Loop with counter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStep {
    pub id: String,
    /// Counter variable name
    #[serde(default = "default_counter")]
    pub counter: String,
    /// Maximum iterations
    #[serde(default = "default_max_iter")]
    pub max_iterations: u64,
    /// Loop body
    pub steps: Vec<ChainStep>,
    /// Break condition
    #[serde(default)]
    pub until: Option<String>,
}

fn default_counter() -> String {
    "i".to_string()
}

fn default_max_iter() -> u64 {
    1000
}

/// Iterate over collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForEachStep {
    pub id: String,
    /// Collection expression
    pub collection: String,
    /// Item variable name
    #[serde(default = "default_item")]
    pub item: String,
    /// Index variable name
    #[serde(default = "default_index")]
    pub index: String,
    /// Loop body
    pub steps: Vec<ChainStep>,
}

fn default_item() -> String {
    "item".to_string()
}

fn default_index() -> String {
    "idx".to_string()
}

/// Guard/gate step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateStep {
    pub id: String,
    /// Condition that must be true
    pub condition: String,
    /// Error message if condition fails
    #[serde(default)]
    pub error: Option<String>,
}

/// Early return
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnStep {
    /// Value expression
    pub value: String,
    /// Optional condition
    #[serde(default)]
    pub condition: Option<String>,
}

/// Compute a value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeStep {
    /// Variable name
    pub id: String,
    pub name: String,
    /// Expression
    pub expr: String,
}

/// Set context value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetStep {
    pub name: String,
    pub value: String,
}

/// Try/catch/finally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TryStep {
    pub id: String,
    #[serde(rename = "try")]
    pub try_steps: Vec<ChainStep>,
    #[serde(default)]
    pub catch: Option<CatchBlock>,
    #[serde(default)]
    pub finally: Option<Vec<ChainStep>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchBlock {
    /// Error variable name
    #[serde(default = "default_error")]
    pub error: String,
    pub steps: Vec<ChainStep>,
}

fn default_error() -> String {
    "err".to_string()
}

/// Dynamic spec dispatch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicStep {
    pub id: String,
    /// Expression that evaluates to spec name
    pub spec: String,
    /// Allowed specs (for type safety)
    #[serde(default)]
    pub allowed: Vec<String>,
    /// Input mappings
    #[serde(default)]
    pub inputs: HashMap<String, String>,
}

/// Await async result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitStep {
    pub id: String,
    /// Expression to await
    pub expr: String,
    /// Timeout
    #[serde(default)]
    pub timeout: Option<u64>,
}

/// Emit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitStep {
    /// Event name
    pub event: String,
    /// Event data expression
    pub data: String,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Max attempts
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// Delay between attempts (ms)
    #[serde(default = "default_delay")]
    pub delay_ms: u64,
    /// Exponential backoff
    #[serde(default)]
    pub exponential: bool,
}

fn default_max_attempts() -> u32 {
    3
}

fn default_delay() -> u64 {
    1000
}

// ============================================================================
// Utility functions
// ============================================================================

/// Convert snake_case to PascalCase
pub fn to_pascal(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Convert snake_case to camelCase
pub fn to_camel(s: &str) -> String {
    let pascal = to_pascal(s);
    let mut chars = pascal.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_lowercase().chain(chars).collect(),
    }
}

/// Collect all step IDs from a chain (for context struct generation)
pub fn collect_step_ids(steps: &[ChainStep]) -> Vec<String> {
    let mut ids = Vec::new();
    for step in steps {
        match step {
            ChainStep::Call(c) => ids.push(c.id.clone()),
            ChainStep::Parallel(p) => {
                ids.push(p.id.clone());
                ids.extend(collect_step_ids(&p.steps));
            }
            ChainStep::Branch(b) => {
                ids.push(b.id.clone());
                for steps in b.cases.values() {
                    ids.extend(collect_step_ids(steps));
                }
                if let Some(d) = &b.default {
                    ids.extend(collect_step_ids(d));
                }
            }
            ChainStep::Loop(l) => {
                ids.push(l.id.clone());
                ids.extend(collect_step_ids(&l.steps));
            }
            ChainStep::ForEach(f) => {
                ids.push(f.id.clone());
                ids.extend(collect_step_ids(&f.steps));
            }
            ChainStep::Gate(g) => ids.push(g.id.clone()),
            ChainStep::Compute(c) => ids.push(c.id.clone()),
            ChainStep::Try(t) => {
                ids.push(t.id.clone());
                ids.extend(collect_step_ids(&t.try_steps));
                if let Some(c) = &t.catch {
                    ids.extend(collect_step_ids(&c.steps));
                }
                if let Some(f) = &t.finally {
                    ids.extend(collect_step_ids(f));
                }
            }
            ChainStep::Dynamic(d) => ids.push(d.id.clone()),
            ChainStep::Await(a) => ids.push(a.id.clone()),
            _ => {}
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_orchestrator() {
        let yaml = r#"
id: order_flow
name: Order Processing Flow
inputs:
  - name: user_id
    type: string
  - name: cart_items
    type: object
outputs:
  - name: order_id
    type: string
  - name: total
    type: float
uses:
  - validate_cart
  - calculate_total
  - create_order
chain:
  - step: call
    id: validate
    spec: validate_cart
    inputs:
      items: "cart_items"
  - step: gate
    id: check_valid
    condition: "validate.is_valid"
    error: "Cart validation failed"
  - step: call
    id: totals
    spec: calculate_total
    inputs:
      items: "cart_items"
"#;
        let orch = Orchestrator::from_yaml(yaml).unwrap();
        assert_eq!(orch.id, "order_flow");
        assert_eq!(orch.inputs.len(), 2);
        assert_eq!(orch.uses.len(), 3);
        assert_eq!(orch.chain.len(), 3);
    }

    #[test]
    fn test_to_pascal() {
        assert_eq!(to_pascal("hello_world"), "HelloWorld");
        assert_eq!(to_pascal("order_flow"), "OrderFlow");
        assert_eq!(to_pascal("simple"), "Simple");
    }

    #[test]
    fn test_to_camel() {
        assert_eq!(to_camel("hello_world"), "helloWorld");
        assert_eq!(to_camel("order_flow"), "orderFlow");
    }
}
