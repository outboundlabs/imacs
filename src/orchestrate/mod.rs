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
//! Code generation uses MiniJinja templates for properly formatted output.

use crate::cel::Target;
use crate::spec::{Spec, VarType};
use crate::templates;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Render an orchestrator to target language using templates
///
/// This function uses MiniJinja templates for code generation,
/// producing properly formatted output without needing external formatters.
pub fn render_orchestrator(
    orch: &Orchestrator,
    specs: &HashMap<String, Spec>,
    target: Target,
) -> String {
    templates::render_orchestrator(orch, specs, target, true)
        .unwrap_or_else(|e| panic!("Template rendering failed: {}", e))
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
    /// Namespace/scoping configuration for code generation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scoping: Option<crate::render::ScopingConfig>,
}

impl Orchestrator {
    /// Parse orchestrator from YAML
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_norway::Error> {
        serde_norway::from_str(yaml)
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

    fn validate_chain(
        &self,
        steps: &[ChainStep],
        specs: &HashMap<String, Spec>,
        errors: &mut Vec<String>,
    ) {
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

/// Count total steps in an orchestrator chain (recursive)
pub fn count_steps(steps: &[ChainStep]) -> usize {
    let mut count = 0;
    for step in steps {
        count += 1;
        match step {
            ChainStep::Parallel(p) => count += count_steps(&p.steps),
            ChainStep::Branch(b) => {
                for case_steps in b.cases.values() {
                    count += count_steps(case_steps);
                }
                if let Some(d) = &b.default {
                    count += count_steps(d);
                }
            }
            ChainStep::Loop(l) => count += count_steps(&l.steps),
            ChainStep::ForEach(f) => count += count_steps(&f.steps),
            ChainStep::Try(t) => {
                count += count_steps(&t.try_steps);
                if let Some(c) = &t.catch {
                    count += count_steps(&c.steps);
                }
                if let Some(f) = &t.finally {
                    count += count_steps(f);
                }
            }
            _ => {}
        }
    }
    count
}

/// Check if a chain of steps contains any spec calls
fn contains_spec_call(steps: &[ChainStep]) -> bool {
    for step in steps {
        match step {
            ChainStep::Call(_) | ChainStep::Dynamic(_) => return true,
            ChainStep::Parallel(p) => {
                if contains_spec_call(&p.steps) {
                    return true;
                }
            }
            ChainStep::Branch(b) => {
                for case_steps in b.cases.values() {
                    if contains_spec_call(case_steps) {
                        return true;
                    }
                }
                if let Some(d) = &b.default {
                    if contains_spec_call(d) {
                        return true;
                    }
                }
            }
            ChainStep::Loop(l) => {
                if contains_spec_call(&l.steps) {
                    return true;
                }
            }
            ChainStep::ForEach(f) => {
                if contains_spec_call(&f.steps) {
                    return true;
                }
            }
            ChainStep::Try(t) => {
                if contains_spec_call(&t.try_steps) {
                    return true;
                }
                if let Some(c) = &t.catch {
                    if contains_spec_call(&c.steps) {
                        return true;
                    }
                }
                if let Some(f) = &t.finally {
                    if contains_spec_call(f) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// Calculate cyclomatic complexity of an orchestrator chain
/// Each decision point (Branch, Loop, ForEach, Gate, Try) adds 1 to complexity
pub fn calculate_complexity(steps: &[ChainStep]) -> usize {
    let mut complexity = 1; // Base complexity
    for step in steps {
        match step {
            ChainStep::Branch(b) => {
                // Each case adds a path
                complexity += b.cases.len();
                for case_steps in b.cases.values() {
                    complexity += calculate_complexity(case_steps) - 1;
                }
                if let Some(d) = &b.default {
                    complexity += calculate_complexity(d) - 1;
                }
            }
            ChainStep::Loop(l) => {
                complexity += 1; // Loop adds one decision
                complexity += calculate_complexity(&l.steps) - 1;
            }
            ChainStep::ForEach(f) => {
                complexity += 1; // ForEach adds one decision
                complexity += calculate_complexity(&f.steps) - 1;
            }
            ChainStep::Gate(_) => {
                complexity += 1; // Gate is a decision point
            }
            ChainStep::Try(t) => {
                complexity += 1; // Try/catch is a decision
                complexity += calculate_complexity(&t.try_steps) - 1;
                if let Some(c) = &t.catch {
                    complexity += calculate_complexity(&c.steps) - 1;
                }
                if let Some(f) = &t.finally {
                    complexity += calculate_complexity(f) - 1;
                }
            }
            ChainStep::Parallel(p) => {
                complexity += calculate_complexity(&p.steps) - 1;
            }
            _ => {}
        }
    }
    complexity
}

/// Complexity analysis result for an orchestrator
#[derive(Debug, Clone)]
pub struct ComplexityReport {
    /// Total number of steps
    pub step_count: usize,
    /// Cyclomatic complexity
    pub cyclomatic_complexity: usize,
    /// Warnings about potential issues
    pub warnings: Vec<String>,
}

impl Orchestrator {
    /// Analyze orchestrator complexity and return warnings
    /// FM-2: Orchestrator Complexity Escape Hatch mitigation
    pub fn analyze_complexity(&self) -> ComplexityReport {
        let mut warnings = Vec::new();
        let step_count = count_steps(&self.chain);
        let cyclomatic_complexity = calculate_complexity(&self.chain);

        // FM-2 Lint: Flag orchestrators with >10 steps
        if step_count > 10 {
            warnings.push(format!(
                "Orchestrator '{}' has {} steps (>10). Consider decomposing into smaller orchestrators.",
                self.id, step_count
            ));
        }

        // FM-2 Lint: High cyclomatic complexity
        if cyclomatic_complexity > 10 {
            warnings.push(format!(
                "Orchestrator '{}' has cyclomatic complexity {} (>10). Consider simplifying control flow.",
                self.id, cyclomatic_complexity
            ));
        }

        // FM-2 Lint: Check Branch/Loop without spec calls
        self.check_control_flow_warnings(&self.chain, &mut warnings);

        ComplexityReport {
            step_count,
            cyclomatic_complexity,
            warnings,
        }
    }

    fn check_control_flow_warnings(&self, steps: &[ChainStep], warnings: &mut Vec<String>) {
        for step in steps {
            match step {
                ChainStep::Branch(b) => {
                    // Check if branch has any spec calls
                    let mut has_spec = false;
                    for case_steps in b.cases.values() {
                        if contains_spec_call(case_steps) {
                            has_spec = true;
                        }
                        self.check_control_flow_warnings(case_steps, warnings);
                    }
                    if let Some(d) = &b.default {
                        if contains_spec_call(d) {
                            has_spec = true;
                        }
                        self.check_control_flow_warnings(d, warnings);
                    }
                    if !has_spec {
                        warnings.push(format!(
                            "Branch step '{}' contains no spec calls. Consider extracting logic to a Spec for better verification.",
                            b.id
                        ));
                    }
                }
                ChainStep::Loop(l) => {
                    if !contains_spec_call(&l.steps) {
                        warnings.push(format!(
                            "Loop step '{}' contains no spec calls. Consider extracting logic to a Spec for better verification.",
                            l.id
                        ));
                    }
                    self.check_control_flow_warnings(&l.steps, warnings);
                }
                ChainStep::ForEach(f) => {
                    if !contains_spec_call(&f.steps) {
                        warnings.push(format!(
                            "ForEach step '{}' contains no spec calls. Consider extracting logic to a Spec for better verification.",
                            f.id
                        ));
                    }
                    self.check_control_flow_warnings(&f.steps, warnings);
                }
                ChainStep::Parallel(p) => {
                    self.check_control_flow_warnings(&p.steps, warnings);
                }
                ChainStep::Try(t) => {
                    self.check_control_flow_warnings(&t.try_steps, warnings);
                    if let Some(c) = &t.catch {
                        self.check_control_flow_warnings(&c.steps, warnings);
                    }
                    if let Some(f) = &t.finally {
                        self.check_control_flow_warnings(f, warnings);
                    }
                }
                _ => {}
            }
        }
    }
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

    #[test]
    fn test_count_steps() {
        let yaml = r#"
id: simple
chain:
  - step: call
    id: step1
    spec: spec_a
    inputs: {}
  - step: call
    id: step2
    spec: spec_b
    inputs: {}
"#;
        let orch = Orchestrator::from_yaml(yaml).unwrap();
        assert_eq!(count_steps(&orch.chain), 2);
    }

    #[test]
    fn test_count_steps_nested() {
        let yaml = r#"
id: nested
chain:
  - step: branch
    id: brancher
    on: "condition"
    cases:
      "true":
        - step: call
          id: inner1
          spec: spec_a
          inputs: {}
        - step: call
          id: inner2
          spec: spec_b
          inputs: {}
"#;
        let orch = Orchestrator::from_yaml(yaml).unwrap();
        // 1 branch + 2 calls = 3 steps
        assert_eq!(count_steps(&orch.chain), 3);
    }

    #[test]
    fn test_calculate_complexity() {
        let yaml = r#"
id: simple
chain:
  - step: call
    id: step1
    spec: spec_a
    inputs: {}
  - step: call
    id: step2
    spec: spec_b
    inputs: {}
"#;
        let orch = Orchestrator::from_yaml(yaml).unwrap();
        // No decisions, base complexity = 1
        assert_eq!(calculate_complexity(&orch.chain), 1);
    }

    #[test]
    fn test_calculate_complexity_with_gate() {
        let yaml = r#"
id: gated
chain:
  - step: call
    id: step1
    spec: spec_a
    inputs: {}
  - step: gate
    id: check
    condition: "step1.valid"
"#;
        let orch = Orchestrator::from_yaml(yaml).unwrap();
        // Base 1 + 1 gate = 2
        assert_eq!(calculate_complexity(&orch.chain), 2);
    }

    #[test]
    fn test_analyze_complexity_too_many_steps() {
        // Create an orchestrator with >10 steps
        let yaml = r#"
id: big_orch
chain:
  - step: call
    id: s1
    spec: a
    inputs: {}
  - step: call
    id: s2
    spec: a
    inputs: {}
  - step: call
    id: s3
    spec: a
    inputs: {}
  - step: call
    id: s4
    spec: a
    inputs: {}
  - step: call
    id: s5
    spec: a
    inputs: {}
  - step: call
    id: s6
    spec: a
    inputs: {}
  - step: call
    id: s7
    spec: a
    inputs: {}
  - step: call
    id: s8
    spec: a
    inputs: {}
  - step: call
    id: s9
    spec: a
    inputs: {}
  - step: call
    id: s10
    spec: a
    inputs: {}
  - step: call
    id: s11
    spec: a
    inputs: {}
"#;
        let orch = Orchestrator::from_yaml(yaml).unwrap();
        let report = orch.analyze_complexity();
        assert_eq!(report.step_count, 11);
        assert!(report.warnings.iter().any(|w| w.contains(">10")));
    }

    #[test]
    fn test_analyze_complexity_branch_without_spec() {
        let yaml = r#"
id: branch_only
chain:
  - step: branch
    id: my_branch
    on: "condition"
    cases:
      "a":
        - step: compute
          id: compute1
          name: result1
          expr: "x + 1"
      "b":
        - step: compute
          id: compute2
          name: result2
          expr: "x + 2"
"#;
        let orch = Orchestrator::from_yaml(yaml).unwrap();
        let report = orch.analyze_complexity();
        // Should warn about branch without spec calls
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("my_branch") && w.contains("no spec calls")));
    }

    #[test]
    fn test_analyze_complexity_branch_with_spec_no_warning() {
        let yaml = r#"
id: branch_with_spec
chain:
  - step: branch
    id: my_branch
    on: "condition"
    cases:
      "a":
        - step: call
          id: call1
          spec: spec_a
          inputs: {}
"#;
        let orch = Orchestrator::from_yaml(yaml).unwrap();
        let report = orch.analyze_complexity();
        // Should NOT warn about this branch since it has a spec call
        assert!(!report.warnings.iter().any(|w| w.contains("my_branch")));
    }
}
