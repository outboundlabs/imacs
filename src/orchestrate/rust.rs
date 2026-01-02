//! Rust orchestrator code generation using genco

use crate::spec::Spec;
use genco::prelude::*;
use std::collections::HashMap;

use super::{
    collect_step_ids, to_pascal, BranchStep, CallStep, ChainStep, ForEachStep, GateStep, LoopStep,
    Orchestrator, ParallelStep, TryStep,
};

/// Render orchestrator to Rust code
pub fn render(orch: &Orchestrator, _specs: &HashMap<String, Spec>) -> String {
    let tokens = RustOrchRenderer::new(orch).render();
    tokens.to_file_string().unwrap_or_default()
}

struct RustOrchRenderer<'a> {
    orch: &'a Orchestrator,
}

impl<'a> RustOrchRenderer<'a> {
    fn new(orch: &'a Orchestrator) -> Self {
        Self { orch }
    }

    fn render(&self) -> rust::Tokens {
        let orch_name = to_pascal(&self.orch.id);
        let step_ids = collect_step_ids(&self.orch.chain);

        quote! {
            // Generated orchestrator: $(&self.orch.id)
            // DO NOT EDIT - regenerate with: imacs render $(&self.orch.id)

            use serde::{Deserialize, Serialize};
            use serde_json::Value;

            $(for spec_id in &self.orch.uses join ($['\n']) =>
                use crate::$(spec_id)::$(spec_id);
            )
            $['\n']

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct $(&orch_name)Input {
                $(for input in &self.orch.inputs join ($['\n']) =>
                    pub $(&input.name): $(rust_type(&input.var_type)),
                )
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct $(&orch_name)Output {
                $(for output in &self.orch.outputs join ($['\n']) =>
                    pub $(&output.name): $(rust_type(&output.var_type)),
                )
            }

            #[derive(Debug, Clone, Default)]
            struct $(&orch_name)Context {
                $(for id in &step_ids join ($['\n']) =>
                    $(id): Option<Value>,
                )
            }

            #[derive(Debug, Clone)]
            pub enum $(&orch_name)Error {
                StepFailed { step: String, message: String },
                GateFailed { gate: String, condition: String },
                Timeout { step: String },
            }

            pub fn $(&self.orch.id)(input: $(&orch_name)Input) -> Result<$(&orch_name)Output, $(&orch_name)Error> {
                let mut ctx = $(&orch_name)Context::default();

                $(for step in &self.orch.chain join ($['\n']) =>
                    $(self.render_step(step, &orch_name))
                )

                // Build output
                Ok($(&orch_name)Output {
                    $(for output in &self.orch.outputs join ($['\n']) =>
                        $(&output.name): todo!("map output"),
                    )
                })
            }
        }
    }

    fn render_step(&self, step: &ChainStep, orch_name: &str) -> rust::Tokens {
        match step {
            ChainStep::Call(call) => self.render_call(call),
            ChainStep::Parallel(par) => self.render_parallel(par, orch_name),
            ChainStep::Branch(branch) => self.render_branch(branch, orch_name),
            ChainStep::Loop(loop_) => self.render_loop(loop_, orch_name),
            ChainStep::ForEach(foreach) => self.render_foreach(foreach, orch_name),
            ChainStep::Gate(gate) => self.render_gate(gate, orch_name),
            ChainStep::Try(try_) => self.render_try(try_, orch_name),
            ChainStep::Compute(compute) => {
                let expr = rust_expr(&compute.expr);
                quote! {
                    let $(&compute.name) = $expr;
                }
            }
            ChainStep::Set(set) => {
                let expr = rust_expr(&set.value);
                quote! {
                    ctx.$(&set.name) = $expr;
                }
            }
            ChainStep::Return(ret) => {
                let value = rust_expr(&ret.value);
                if let Some(cond) = &ret.condition {
                    let cond_expr = rust_expr(cond);
                    quote! {
                        if $cond_expr {
                            return $value;
                        }
                    }
                } else {
                    quote! {
                        return $value;
                    }
                }
            }
            ChainStep::Dynamic(dyn_) => {
                let spec_expr = rust_expr(&dyn_.spec);
                quote! {
                    // Dynamic dispatch: $(&dyn_.id)
                    match $spec_expr.as_str() {
                        $(for allowed in &dyn_.allowed join ($['\n']) =>
                            $(quoted(allowed)) => { $(allowed)(/* inputs */); }
                        )
                        _ => panic!("Unknown spec"),
                    }
                }
            }
            ChainStep::Await(await_) => {
                quote! {
                    // Await: $(&await_.id) (TODO: async support)
                }
            }
            ChainStep::Emit(emit) => {
                let data = rust_expr(&emit.data);
                quote! {
                    // Emit event: $(&emit.event)
                    // events.push(($(quoted(&emit.event)), $data));
                }
            }
        }
    }

    fn render_call(&self, call: &CallStep) -> rust::Tokens {
        let spec_pascal = to_pascal(&call.spec);
        let call_id = &call.id;
        let spec_name = &call.spec;

        let core_logic = quote! {
            let $(call_id)_input = $(&spec_pascal)Input {
                $(for (spec_input, expr) in &call.inputs join ($['\n']) =>
                    $(spec_input): $(rust_expr(expr)),
                )
            };
            let $(call_id)_result = $(spec_name)($(call_id)_input);
            ctx.$(call_id) = Some(serde_json::to_value(&$(call_id)_result).unwrap());
        };

        if let Some(cond) = &call.condition {
            let cond_expr = rust_expr(cond);
            quote! {
                // Step: $(call_id)
                if $cond_expr {
                    $core_logic
                }
            }
        } else {
            quote! {
                // Step: $(call_id)
                $core_logic
            }
        }
    }

    fn render_parallel(&self, par: &ParallelStep, orch_name: &str) -> rust::Tokens {
        quote! {
            // Parallel: $(&par.id)
            // TODO: Use rayon or tokio::join!
            $(for step in &par.steps join ($['\n']) =>
                $(self.render_step(step, orch_name))
            )
        }
    }

    fn render_branch(&self, branch: &BranchStep, orch_name: &str) -> rust::Tokens {
        let on_expr = rust_expr(&branch.on);
        quote! {
            // Branch: $(&branch.id)
            match $on_expr {
                $(for (case_val, steps) in &branch.cases join ($['\n']) =>
                    $(case_val) => {
                        $(for step in steps join ($['\n']) =>
                            $(self.render_step(step, orch_name))
                        )
                    }
                )
                $(if let Some(default) = &branch.default {
                    _ => {
                        $(for step in default join ($['\n']) =>
                            $(self.render_step(step, orch_name))
                        )
                    }
                })
            }
        }
    }

    fn render_loop(&self, loop_: &LoopStep, orch_name: &str) -> rust::Tokens {
        let until_check = if let Some(until) = &loop_.until {
            let until_expr = rust_expr(until);
            quote! { if $until_expr { break; } }
        } else {
            quote! {}
        };

        quote! {
            // Loop: $(&loop_.id)
            for $(&loop_.counter) in 0..$(loop_.max_iterations) {
                $until_check
                $(for step in &loop_.steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
            }
        }
    }

    fn render_foreach(&self, foreach: &ForEachStep, orch_name: &str) -> rust::Tokens {
        let collection = rust_expr(&foreach.collection);
        quote! {
            // ForEach: $(&foreach.id)
            for ($(&foreach.index), $(&foreach.item)) in $collection.iter().enumerate() {
                $(for step in &foreach.steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
            }
        }
    }

    fn render_gate(&self, gate: &GateStep, orch_name: &str) -> rust::Tokens {
        let cond = rust_expr(&gate.condition);
        quote! {
            // Gate: $(&gate.id)
            if !($cond) {
                return Err($(orch_name)Error::GateFailed {
                    gate: $(&gate.id).into(),
                    condition: $(quoted(&gate.condition)),
                });
            }
        }
    }

    fn render_try(&self, try_: &TryStep, orch_name: &str) -> rust::Tokens {
        let catch_block = if let Some(catch) = &try_.catch {
            let error_var = &catch.error;
            quote! {
                .or_else(|$error_var| {
                    $(for step in &catch.steps join ($['\n']) =>
                        $(self.render_step(step, orch_name))
                    )
                    Ok(())
                })
            }
        } else {
            quote! {}
        };

        let finally_block = if let Some(finally) = &try_.finally {
            quote! {
                // Finally
                $(for step in finally join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
            }
        } else {
            quote! {}
        };

        quote! {
            // Try: $(&try_.id)
            let $(&try_.id)_result = (|| {
                $(for step in &try_.try_steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
                Ok(())
            })()$catch_block;
            $finally_block
        }
    }
}

/// Convert expression to Rust syntax
fn rust_expr(expr: &str) -> String {
    if expr.contains('.') {
        let parts: Vec<&str> = expr.split('.').collect();
        let mut result = format!("ctx.{}.as_ref().unwrap()", parts[0]);
        for part in &parts[1..] {
            result = format!("{}[\"{}\"]", result, part);
        }
        result
    } else {
        expr.to_string()
    }
}

/// Map VarType to Rust type string
fn rust_type(var_type: &crate::spec::VarType) -> &'static str {
    match var_type {
        crate::spec::VarType::Bool => "bool",
        crate::spec::VarType::Int => "i64",
        crate::spec::VarType::Float => "f64",
        crate::spec::VarType::String => "String",
        crate::spec::VarType::Object => "serde_json::Value",
        _ => "serde_json::Value",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_orchestrator() {
        let orch = Orchestrator::from_yaml(
            r#"
id: simple_flow
inputs:
  - name: value
    type: int
outputs:
  - name: result
    type: int
uses:
  - step_one
  - step_two
chain:
  - step: call
    id: first
    spec: step_one
    inputs:
      x: "value"
  - step: call
    id: second
    spec: step_two
    inputs:
      y: "first.output"
"#,
        )
        .unwrap();

        let specs = HashMap::new();
        let rust = render(&orch, &specs);

        assert!(rust.contains("pub fn simple_flow"));
        assert!(rust.contains("SimpleFlowInput"));
        assert!(rust.contains("step_one"));
    }
}
