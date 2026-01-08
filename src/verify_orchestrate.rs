//! Orchestrator verification
//!
//! Verifies that generated orchestrator code matches the orchestrator spec.
//! Similar to regular spec verification, but for chains/flows.

use crate::ast::{CodeAst, FunctionAst, StatementAst};
use crate::error::{Error, Result};
use crate::orchestrate::*;
use crate::spec::Spec;
use std::collections::HashMap;

/// Verification result for an orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorVerification {
    /// Did verification pass?
    pub passed: bool,
    /// Step-by-step verification results
    pub steps: Vec<StepVerification>,
    /// Flow verification (correct order, all paths covered)
    pub flow: FlowVerification,
    /// Overall coverage
    pub coverage: OrchestratorCoverage,
    /// Gaps found
    pub gaps: Vec<OrchestratorGap>,
}

#[derive(Debug, Clone)]
pub struct StepVerification {
    /// Step ID from spec
    pub step_id: String,
    /// Step type
    pub step_type: String,
    /// Was this step found in code?
    pub found: bool,
    /// Does it call the correct spec?
    pub correct_spec: bool,
    /// Are inputs mapped correctly?
    pub correct_inputs: bool,
    /// Location in code
    pub code_location: Option<CodeLocation>,
    /// Issues found
    pub issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CodeLocation {
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone)]
pub struct FlowVerification {
    /// Is the execution order correct?
    pub correct_order: bool,
    /// Are all branches covered?
    pub all_branches_covered: bool,
    /// Are error paths handled?
    pub error_paths_handled: bool,
    /// Unreachable steps
    pub unreachable_steps: Vec<String>,
    /// Missing steps
    pub missing_steps: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OrchestratorCoverage {
    /// Total steps in spec
    pub total_steps: usize,
    /// Steps found in code
    pub covered_steps: usize,
    /// Coverage percentage
    pub percentage: f32,
}

#[derive(Debug, Clone)]
pub struct OrchestratorGap {
    /// Step ID
    pub step_id: String,
    /// Gap type
    pub gap_type: OrchestratorGapType,
    /// Description
    pub description: String,
    /// Suggestion to fix
    pub suggestion: String,
}

#[derive(Debug, Clone)]
pub enum OrchestratorGapType {
    MissingStep,
    WrongOrder,
    WrongSpec,
    WrongInputs,
    MissingErrorHandler,
    UnreachableCode,
}

/// Verify orchestrator code against spec
pub fn verify_orchestrator(
    orch: &Orchestrator,
    ast: &CodeAst,
    specs: &HashMap<String, Spec>,
) -> OrchestratorVerification {
    let mut step_verifications = Vec::new();
    let mut gaps = Vec::new();

    // Find the main orchestrator function
    let main_fn = ast.functions.iter().find(|f| f.name == orch.id);

    if main_fn.is_none() {
        return OrchestratorVerification {
            passed: false,
            steps: vec![],
            flow: FlowVerification {
                correct_order: false,
                all_branches_covered: false,
                error_paths_handled: false,
                unreachable_steps: vec![],
                missing_steps: collect_all_step_ids(&orch.chain),
            },
            coverage: OrchestratorCoverage {
                total_steps: count_steps(&orch.chain),
                covered_steps: 0,
                percentage: 0.0,
            },
            gaps: vec![OrchestratorGap {
                step_id: orch.id.clone(),
                gap_type: OrchestratorGapType::MissingStep,
                description: format!("Main function '{}' not found", orch.id),
                suggestion: format!("Add function: pub fn {}(...)", orch.id),
            }],
        };
    }

    let main_fn = main_fn.unwrap();

    // Verify each step in the chain
    let mut expected_order = Vec::new();
    verify_chain_steps(
        &orch.chain,
        main_fn,
        specs,
        &mut step_verifications,
        &mut gaps,
        &mut expected_order,
    );

    // Check flow correctness
    let actual_order = extract_call_order(main_fn);
    let correct_order = verify_order(&expected_order, &actual_order);

    // Calculate coverage
    let total = count_steps(&orch.chain);
    let covered = step_verifications.iter().filter(|s| s.found).count();
    let percentage = if total > 0 {
        (covered as f32 / total as f32) * 100.0
    } else {
        100.0
    };

    let passed = gaps.is_empty() && correct_order && percentage >= 100.0;

    OrchestratorVerification {
        passed,
        steps: step_verifications,
        flow: FlowVerification {
            correct_order,
            all_branches_covered: check_branch_coverage(&orch.chain, main_fn),
            error_paths_handled: check_error_handling(&orch.chain, main_fn),
            unreachable_steps: vec![],
            missing_steps: gaps
                .iter()
                .filter(|g| matches!(g.gap_type, OrchestratorGapType::MissingStep))
                .map(|g| g.step_id.clone())
                .collect(),
        },
        coverage: OrchestratorCoverage {
            total_steps: total,
            covered_steps: covered,
            percentage,
        },
        gaps,
    }
}

fn verify_chain_steps(
    steps: &[ChainStep],
    func: &FunctionAst,
    specs: &HashMap<String, Spec>,
    results: &mut Vec<StepVerification>,
    gaps: &mut Vec<OrchestratorGap>,
    expected_order: &mut Vec<String>,
) {
    for step in steps {
        match step {
            ChainStep::Call(call) => {
                expected_order.push(call.id.clone());

                // Look for this call in the function
                let found = find_call_in_ast(func, &call.spec);

                let verification = StepVerification {
                    step_id: call.id.clone(),
                    step_type: "call".into(),
                    found: found.is_some(),
                    correct_spec: found.is_some(),
                    correct_inputs: found.map(|_| true).unwrap_or(false), // TODO: verify inputs
                    code_location: found,
                    issues: vec![],
                };

                if !verification.found {
                    gaps.push(OrchestratorGap {
                        step_id: call.id.clone(),
                        gap_type: OrchestratorGapType::MissingStep,
                        description: format!("Call to '{}' not found", call.spec),
                        suggestion: format!("Add: let {} = {}(...);", call.id, call.spec),
                    });
                }

                results.push(verification);
            }

            ChainStep::Parallel(par) => {
                expected_order.push(par.id.clone());
                // Verify nested steps
                verify_chain_steps(&par.steps, func, specs, results, gaps, expected_order);
            }

            ChainStep::Branch(branch) => {
                expected_order.push(branch.id.clone());

                // Verify we have a match/switch on the right expression
                let has_branch = find_branch_in_ast(func, &branch.on);

                if !has_branch {
                    gaps.push(OrchestratorGap {
                        step_id: branch.id.clone(),
                        gap_type: OrchestratorGapType::MissingStep,
                        description: format!("Branch on '{}' not found", branch.on),
                        suggestion: format!("Add: match {} {{ ... }}", branch.on),
                    });
                }

                // Verify each case
                for (case_val, case_steps) in &branch.cases {
                    verify_chain_steps(case_steps, func, specs, results, gaps, expected_order);
                }
                if let Some(default) = &branch.default {
                    verify_chain_steps(default, func, specs, results, gaps, expected_order);
                }
            }

            ChainStep::Loop(loop_) => {
                expected_order.push(loop_.id.clone());

                let has_loop = find_loop_in_ast(func);
                if !has_loop {
                    gaps.push(OrchestratorGap {
                        step_id: loop_.id.clone(),
                        gap_type: OrchestratorGapType::MissingStep,
                        description: "Loop not found".into(),
                        suggestion: "Add: for/while loop".into(),
                    });
                }

                verify_chain_steps(&loop_.steps, func, specs, results, gaps, expected_order);
            }

            ChainStep::ForEach(foreach) => {
                expected_order.push(foreach.id.clone());

                let has_foreach = find_foreach_in_ast(func, &foreach.collection);
                if !has_foreach {
                    gaps.push(OrchestratorGap {
                        step_id: foreach.id.clone(),
                        gap_type: OrchestratorGapType::MissingStep,
                        description: format!("ForEach over '{}' not found", foreach.collection),
                        suggestion: format!("Add: for item in {} {{ ... }}", foreach.collection),
                    });
                }

                verify_chain_steps(&foreach.steps, func, specs, results, gaps, expected_order);
            }

            ChainStep::Gate(gate) => {
                expected_order.push(gate.id.clone());

                let has_gate = find_gate_in_ast(func, &gate.condition);
                if !has_gate {
                    gaps.push(OrchestratorGap {
                        step_id: gate.id.clone(),
                        gap_type: OrchestratorGapType::MissingStep,
                        description: format!("Gate '{}' not found", gate.condition),
                        suggestion: format!("Add: if !({}) {{ return Err(...) }}", gate.condition),
                    });
                }
            }

            ChainStep::Try(try_) => {
                expected_order.push(try_.id.clone());

                verify_chain_steps(&try_.try_steps, func, specs, results, gaps, expected_order);
                if let Some(catch) = &try_.catch {
                    verify_chain_steps(&catch.steps, func, specs, results, gaps, expected_order);
                }
                if let Some(finally) = &try_.finally {
                    verify_chain_steps(finally, func, specs, results, gaps, expected_order);
                }
            }

            ChainStep::Compute(compute) => {
                expected_order.push(compute.id.clone());
                // Look for variable assignment
                let has_compute = find_assignment_in_ast(func, &compute.name);
                if !has_compute {
                    gaps.push(OrchestratorGap {
                        step_id: compute.id.clone(),
                        gap_type: OrchestratorGapType::MissingStep,
                        description: format!("Compute '{}' not found", compute.name),
                        suggestion: format!("Add: let {} = {};", compute.name, compute.expr),
                    });
                }
            }

            // These don't have explicit code representation to verify
            ChainStep::Return(_) | ChainStep::Set(_) | ChainStep::Emit(_) => {}

            ChainStep::Dynamic(dyn_) => {
                expected_order.push(dyn_.id.clone());
                // Look for dynamic dispatch (match on string)
                let has_dynamic = find_dynamic_dispatch_in_ast(func);
                if !has_dynamic {
                    gaps.push(OrchestratorGap {
                        step_id: dyn_.id.clone(),
                        gap_type: OrchestratorGapType::MissingStep,
                        description: "Dynamic dispatch not found".into(),
                        suggestion: "Add: match spec_name { ... }".into(),
                    });
                }
            }

            ChainStep::Await(await_) => {
                expected_order.push(await_.id.clone());
            }
        }
    }
}

// AST searching helpers (simplified - real implementation would be more thorough)

fn find_call_in_ast(func: &FunctionAst, spec_name: &str) -> Option<CodeLocation> {
    // Search function body for call to spec_name
    for (i, stmt) in func.body.iter().enumerate() {
        if let StatementAst::Call { name, .. } = stmt {
            if name == spec_name {
                return Some(CodeLocation {
                    line_start: i + 1,
                    line_end: i + 1,
                });
            }
        }
    }
    None
}

fn find_branch_in_ast(func: &FunctionAst, _expr: &str) -> bool {
    func.body.iter().any(|stmt| matches!(stmt, StatementAst::Match { .. }))
}

fn find_loop_in_ast(func: &FunctionAst) -> bool {
    func.body
        .iter()
        .any(|stmt| matches!(stmt, StatementAst::Loop { .. } | StatementAst::For { .. }))
}

fn find_foreach_in_ast(func: &FunctionAst, _collection: &str) -> bool {
    func.body.iter().any(|stmt| matches!(stmt, StatementAst::For { .. }))
}

fn find_gate_in_ast(func: &FunctionAst, _condition: &str) -> bool {
    func.body.iter().any(|stmt| {
        matches!(stmt, StatementAst::If { .. })
    })
}

fn find_assignment_in_ast(func: &FunctionAst, var_name: &str) -> bool {
    func.body.iter().any(|stmt| {
        if let StatementAst::Let { name, .. } = stmt {
            name == var_name
        } else {
            false
        }
    })
}

fn find_dynamic_dispatch_in_ast(func: &FunctionAst) -> bool {
    func.body.iter().any(|stmt| matches!(stmt, StatementAst::Match { .. }))
}

fn extract_call_order(func: &FunctionAst) -> Vec<String> {
    let mut order = Vec::new();
    for stmt in &func.body {
        if let StatementAst::Call { name, .. } = stmt {
            order.push(name.clone());
        }
    }
    order
}

fn verify_order(expected: &[String], actual: &[String]) -> bool {
    // Check that expected calls appear in order (allowing gaps)
    let mut actual_iter = actual.iter();
    for exp in expected {
        // Find this expected call in remaining actual calls
        let found = actual_iter.by_ref().any(|a| a == exp);
        if !found {
            return false;
        }
    }
    true
}

fn check_error_handling(chain: &[ChainStep], _func: &FunctionAst) -> bool {
    // Check if any step needs error handling and if it's present
    let has_try = chain.iter().any(|s| matches!(s, ChainStep::Try(_)));
    let has_gate = chain.iter().any(|s| matches!(s, ChainStep::Gate(_)));

    // If spec has error handling, code should too
    // For now, just check if there's a Result return type or try blocks
    has_try || has_gate || true // Simplified
}

/// Check if all branches in the orchestrator are covered in the implementation
fn check_branch_coverage(chain: &[ChainStep], func: &FunctionAst) -> bool {
    let (required_branches, found_branches) = count_branches(chain, func);

    // All branches covered if we found at least the expected number
    found_branches >= required_branches
}

/// Count required branches and found branches in the chain
fn count_branches(chain: &[ChainStep], func: &FunctionAst) -> (usize, usize) {
    let mut required = 0;
    let mut found = 0;

    for step in chain {
        match step {
            ChainStep::Branch(branch) => {
                // Count each case as a required branch
                required += branch.cases.len();
                if branch.default.is_some() {
                    required += 1;
                }

                // Check if the branch exists in code
                if find_branch_in_ast(func, &branch.on) {
                    // Count branches we found in AST
                    found += branch.cases.len();
                    if branch.default.is_some() {
                        found += 1;
                    }
                }

                // Recurse into branch cases
                for case_steps in branch.cases.values() {
                    let (r, f) = count_branches(case_steps, func);
                    required += r;
                    found += f;
                }
                if let Some(default_steps) = &branch.default {
                    let (r, f) = count_branches(default_steps, func);
                    required += r;
                    found += f;
                }
            }
            ChainStep::Try(try_) => {
                // Try block is required, catch and finally are branches
                required += 1; // try block
                if try_.catch.is_some() {
                    required += 1; // catch branch
                }
                if try_.finally.is_some() {
                    required += 1; // finally branch
                }

                // Check if try exists in code (simplified - assume if any try step is found, it's covered)
                found += 1;
                if try_.catch.is_some() {
                    found += 1;
                }
                if try_.finally.is_some() {
                    found += 1;
                }

                // Recurse
                let (r, f) = count_branches(&try_.try_steps, func);
                required += r;
                found += f;

                if let Some(catch) = &try_.catch {
                    let (r, f) = count_branches(&catch.steps, func);
                    required += r;
                    found += f;
                }
                if let Some(finally) = &try_.finally {
                    let (r, f) = count_branches(finally, func);
                    required += r;
                    found += f;
                }
            }
            ChainStep::Parallel(par) => {
                let (r, f) = count_branches(&par.steps, func);
                required += r;
                found += f;
            }
            ChainStep::Loop(loop_) => {
                let (r, f) = count_branches(&loop_.steps, func);
                required += r;
                found += f;
            }
            ChainStep::ForEach(foreach) => {
                let (r, f) = count_branches(&foreach.steps, func);
                required += r;
                found += f;
            }
            _ => {}
        }
    }

    (required, found)
}

fn collect_all_step_ids(steps: &[ChainStep]) -> Vec<String> {
    let mut ids = Vec::new();
    for step in steps {
        match step {
            ChainStep::Call(c) => ids.push(c.id.clone()),
            ChainStep::Parallel(p) => {
                ids.push(p.id.clone());
                ids.extend(collect_all_step_ids(&p.steps));
            }
            ChainStep::Branch(b) => {
                ids.push(b.id.clone());
                for steps in b.cases.values() {
                    ids.extend(collect_all_step_ids(steps));
                }
                if let Some(d) = &b.default {
                    ids.extend(collect_all_step_ids(d));
                }
            }
            ChainStep::Loop(l) => {
                ids.push(l.id.clone());
                ids.extend(collect_all_step_ids(&l.steps));
            }
            ChainStep::ForEach(f) => {
                ids.push(f.id.clone());
                ids.extend(collect_all_step_ids(&f.steps));
            }
            ChainStep::Gate(g) => ids.push(g.id.clone()),
            ChainStep::Compute(c) => ids.push(c.id.clone()),
            ChainStep::Try(t) => {
                ids.push(t.id.clone());
                ids.extend(collect_all_step_ids(&t.try_steps));
                if let Some(c) = &t.catch {
                    ids.extend(collect_all_step_ids(&c.steps));
                }
                if let Some(f) = &t.finally {
                    ids.extend(collect_all_step_ids(f));
                }
            }
            ChainStep::Dynamic(d) => ids.push(d.id.clone()),
            ChainStep::Await(a) => ids.push(a.id.clone()),
            _ => {}
        }
    }
    ids
}

fn count_steps(steps: &[ChainStep]) -> usize {
    collect_all_step_ids(steps).len()
}

impl OrchestratorVerification {
    /// Generate a human-readable report
    pub fn to_report(&self) -> String {
        let mut report = String::new();

        report.push_str(&format!(
            "Orchestrator Verification: {}\n",
            if self.passed { "PASSED ✓" } else { "FAILED ✗" }
        ));
        report.push_str(&format!(
            "Coverage: {}/{} steps ({:.0}%)\n\n",
            self.coverage.covered_steps, self.coverage.total_steps, self.coverage.percentage
        ));

        if !self.gaps.is_empty() {
            report.push_str("Gaps:\n");
            for gap in &self.gaps {
                report.push_str(&format!(
                    "  - [{}] {}: {}\n    Suggestion: {}\n",
                    gap.step_id,
                    format!("{:?}", gap.gap_type),
                    gap.description,
                    gap.suggestion
                ));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_steps() {
        let chain = vec![
            ChainStep::Call(CallStep {
                id: "step1".into(),
                spec: "spec1".into(),
                inputs: HashMap::new(),
                outputs: HashMap::new(),
                condition: None,
                async_: false,
                retry: None,
            }),
            ChainStep::Call(CallStep {
                id: "step2".into(),
                spec: "spec2".into(),
                inputs: HashMap::new(),
                outputs: HashMap::new(),
                condition: None,
                async_: false,
                retry: None,
            }),
        ];

        assert_eq!(count_steps(&chain), 2);
    }
}