//! Python orchestrator code generation using genco

use crate::spec::Spec;
use genco::prelude::*;
use std::collections::HashMap;

use super::{
    to_pascal, BranchStep, CallStep, ChainStep, ForEachStep, GateStep, Orchestrator, ParallelStep,
};

/// Render orchestrator to Python code
pub fn render(orch: &Orchestrator, _specs: &HashMap<String, Spec>) -> String {
    let tokens = PyOrchRenderer::new(orch).render();
    tokens.to_file_string().unwrap_or_default()
}

struct PyOrchRenderer<'a> {
    orch: &'a Orchestrator,
}

impl<'a> PyOrchRenderer<'a> {
    fn new(orch: &'a Orchestrator) -> Self {
        Self { orch }
    }

    fn render(&self) -> python::Tokens {
        let orch_name = to_pascal(&self.orch.id);

        quote! {
            # Generated orchestrator: $(&self.orch.id)
            # DO NOT EDIT

            from dataclasses import dataclass
            from typing import Any, Optional
            import asyncio

            $(for spec_id in &self.orch.uses join ($['\n']) =>
                from .$(spec_id) import $(spec_id)
            )
            $['\n']

            @dataclass
            class $(&orch_name)Input:
                $(for input in &self.orch.inputs join ($['\n']) =>
                    $(&input.name): $(py_type(&input.var_type))
                )
            $['\n']

            @dataclass
            class $(&orch_name)Output:
                $(for output in &self.orch.outputs join ($['\n']) =>
                    $(&output.name): $(py_type(&output.var_type))
                )
            $['\n']

            async def $(&self.orch.id)(input: $(&orch_name)Input) -> $(&orch_name)Output:
                ctx = {}

                $(for step in &self.orch.chain join ($['\n']) =>
                    $(self.render_step(step))
                )

                return $(&orch_name)Output(
                    $(for output in &self.orch.outputs join ($['\n']) =>
                        $(&output.name)=None,  # TODO
                    )
                )
        }
    }

    fn render_step(&self, step: &ChainStep) -> python::Tokens {
        match step {
            ChainStep::Call(call) => self.render_call(call),
            ChainStep::Parallel(par) => self.render_parallel(par),
            ChainStep::Branch(branch) => self.render_branch(branch),
            ChainStep::ForEach(foreach) => self.render_foreach(foreach),
            ChainStep::Gate(gate) => self.render_gate(gate),
            ChainStep::Compute(compute) => {
                let expr = py_expr(&compute.expr);
                quote! {
                    $(&compute.name) = $expr
                }
            }
            ChainStep::Set(set) => {
                let expr = py_expr(&set.value);
                let key = quoted(&set.name);
                quote! {
                    ctx[$key] = $expr
                }
            }
            ChainStep::Return(ret) => {
                let value = py_expr(&ret.value);
                if let Some(cond) = &ret.condition {
                    let cond_expr = py_expr(cond);
                    quote! {
                        if $cond_expr:
                            return $value
                    }
                } else {
                    quote! {
                        return $value
                    }
                }
            }
            ChainStep::Loop(loop_) => {
                let until_check = if let Some(until) = &loop_.until {
                    let until_expr = py_expr(until);
                    quote! {
                        if $until_expr:
                            break
                    }
                } else {
                    quote! {}
                };

                quote! {
                    # Loop: $(&loop_.id)
                    for $(&loop_.counter) in range($(loop_.max_iterations)):
                        $until_check
                        $(for step in &loop_.steps join ($['\n']) =>
                            $(self.render_step(step))
                        )
                }
            }
            ChainStep::Try(try_) => {
                let except_block = if let Some(catch) = &try_.catch {
                    quote! {
                        except Exception as $(&catch.error):
                            $(for step in &catch.steps join ($['\n']) =>
                                $(self.render_step(step))
                            )
                    }
                } else {
                    quote! {}
                };

                let finally_block = if let Some(finally) = &try_.finally {
                    quote! {
                        finally:
                            $(for step in finally join ($['\n']) =>
                                $(self.render_step(step))
                            )
                    }
                } else {
                    quote! {}
                };

                quote! {
                    # Try: $(&try_.id)
                    try:
                        $(for step in &try_.try_steps join ($['\n']) =>
                            $(self.render_step(step))
                        )
                    $except_block
                    $finally_block
                }
            }
            ChainStep::Dynamic(dyn_) => {
                let spec_expr = py_expr(&dyn_.spec);
                quote! {
                    # Dynamic dispatch: $(&dyn_.id)
                    spec_map = {$(for allowed in &dyn_.allowed join (, ) => $(quoted(allowed)): $(allowed))}
                    if $(&spec_expr) in spec_map:
                        await spec_map[$(&spec_expr)]()
                }
            }
            ChainStep::Await(await_) => {
                let expr = py_expr(&await_.expr);
                let key = quoted(&await_.id);
                quote! {
                    ctx[$key] = await $expr
                }
            }
            ChainStep::Emit(emit) => {
                let data = py_expr(&emit.data);
                let event_name = quoted(&emit.event);
                quote! {
                    # Emit event: $(&emit.event)
                    # emitter.emit($event_name, $data)
                }
            }
        }
    }

    fn render_call(&self, call: &CallStep) -> python::Tokens {
        let key = quoted(&call.id);
        quote! {
            # $(&call.id)
            ctx[$key] = await $(&call.spec)(
                $(for (k, v) in &call.inputs join ($['\n']) =>
                    $(k)=$(py_expr(v)),
                )
            )
        }
    }

    fn render_parallel(&self, par: &ParallelStep) -> python::Tokens {
        quote! {
            # Parallel: $(&par.id)
            results = await asyncio.gather(
                $(for step in &par.steps join ($['\n']) =>
                    $(if let ChainStep::Call(call) = step {
                        $(&call.spec)(),
                    })
                )
            )
        }
    }

    fn render_branch(&self, branch: &BranchStep) -> python::Tokens {
        let on_expr = py_expr(&branch.on);
        quote! {
            match $on_expr:
                $(for (val, steps) in &branch.cases join ($['\n']) =>
                    case $(val):
                        $(for s in steps join ($['\n']) =>
                            $(self.render_step(s))
                        )
                )
                $(if branch.default.is_some() {
                    case _:
                        $(for s in branch.default.as_ref().unwrap() join ($['\n']) =>
                            $(self.render_step(s))
                        )
                })
        }
    }

    fn render_foreach(&self, foreach: &ForEachStep) -> python::Tokens {
        let collection = py_expr(&foreach.collection);
        quote! {
            for $(&foreach.index), $(&foreach.item) in enumerate($collection):
                $(for s in &foreach.steps join ($['\n']) =>
                    $(self.render_step(s))
                )
        }
    }

    fn render_gate(&self, gate: &GateStep) -> python::Tokens {
        let cond = py_expr(&gate.condition);
        let error_msg = format!("Gate {} failed", gate.id);
        quote! {
            if not ($cond):
                raise ValueError($(quoted(error_msg)))
        }
    }
}

/// Convert expression to Python syntax
fn py_expr(expr: &str) -> String {
    if expr.contains('.') {
        let parts: Vec<&str> = expr.split('.').collect();
        format!("ctx['{}']", parts.join("']['"))
    } else {
        format!("input.{}", expr)
    }
}

/// Map VarType to Python type string
fn py_type(var_type: &crate::spec::VarType) -> &'static str {
    match var_type {
        crate::spec::VarType::Bool => "bool",
        crate::spec::VarType::Int => "int",
        crate::spec::VarType::Float => "float",
        crate::spec::VarType::String => "str",
        _ => "Any",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_python_orchestrator() {
        let orch = Orchestrator::from_yaml(
            r#"
id: payment_flow
inputs:
  - name: amount
    type: float
outputs:
  - name: success
    type: bool
uses:
  - validate_payment
chain:
  - step: call
    id: validate
    spec: validate_payment
    inputs:
      amt: "amount"
"#,
        )
        .unwrap();

        let specs = HashMap::new();
        let py = render(&orch, &specs);

        assert!(py.contains("async def payment_flow"));
        assert!(py.contains("PaymentFlowInput"));
        assert!(py.contains("validate_payment"));
    }
}
