//! TypeScript orchestrator code generation using genco

use crate::spec::Spec;
use genco::prelude::*;
use std::collections::HashMap;

use super::{
    to_pascal, BranchStep, CallStep, ChainStep, ForEachStep, GateStep, Orchestrator, ParallelStep,
};

/// Render orchestrator to TypeScript code
pub fn render(orch: &Orchestrator, _specs: &HashMap<String, Spec>) -> String {
    let tokens = TsOrchRenderer::new(orch).render();
    tokens.to_file_string().unwrap_or_default()
}

struct TsOrchRenderer<'a> {
    orch: &'a Orchestrator,
}

impl<'a> TsOrchRenderer<'a> {
    fn new(orch: &'a Orchestrator) -> Self {
        Self { orch }
    }

    fn render(&self) -> js::Tokens {
        let orch_name = to_pascal(&self.orch.id);

        quote! {
            // Generated orchestrator: $(&self.orch.id)
            // DO NOT EDIT

            $(for spec_id in &self.orch.uses join ($['\n']) =>
                import { $(spec_id) } from $(quoted(format!("./{}", spec_id)))
            )
            $['\n']

            export interface $(&orch_name)Input {
                $(for input in &self.orch.inputs join ($['\n']) =>
                    $(&input.name): $(ts_type(&input.var_type))
                )
            }

            export interface $(&orch_name)Output {
                $(for output in &self.orch.outputs join ($['\n']) =>
                    $(&output.name): $(ts_type(&output.var_type))
                )
            }

            export async function $(&self.orch.id)(input: $(&orch_name)Input): Promise<$(&orch_name)Output> {
                const ctx: Record<string, any> = {}

                $(for step in &self.orch.chain join ($['\n']) =>
                    $(self.render_step(step))
                )

                return {
                    $(for output in &self.orch.outputs join ($['\n']) =>
                        $(&output.name): undefined as any, // TODO
                    )
                }
            }
        }
    }

    fn render_step(&self, step: &ChainStep) -> js::Tokens {
        match step {
            ChainStep::Call(call) => self.render_call(call),
            ChainStep::Parallel(par) => self.render_parallel(par),
            ChainStep::Branch(branch) => self.render_branch(branch),
            ChainStep::ForEach(foreach) => self.render_foreach(foreach),
            ChainStep::Gate(gate) => self.render_gate(gate),
            ChainStep::Compute(compute) => {
                let expr = ts_expr(&compute.expr);
                quote! {
                    const $(&compute.name) = $expr
                }
            }
            ChainStep::Set(set) => {
                let expr = ts_expr(&set.value);
                quote! {
                    ctx.$(&set.name) = $expr
                }
            }
            ChainStep::Return(ret) => {
                let value = ts_expr(&ret.value);
                if let Some(cond) = &ret.condition {
                    let cond_expr = ts_expr(cond);
                    quote! {
                        if ($cond_expr) {
                            return $value
                        }
                    }
                } else {
                    quote! {
                        return $value
                    }
                }
            }
            ChainStep::Loop(loop_) => {
                let until_check = if let Some(until) = &loop_.until {
                    let until_expr = ts_expr(until);
                    quote! { if ($until_expr) break }
                } else {
                    quote! {}
                };

                quote! {
                    // Loop: $(&loop_.id)
                    for (let $(&loop_.counter) = 0; $(&loop_.counter) < $(loop_.max_iterations); $(&loop_.counter)++) {
                        $until_check
                        $(for step in &loop_.steps join ($['\n']) =>
                            $(self.render_step(step))
                        )
                    }
                }
            }
            ChainStep::Try(try_) => {
                let catch_block = if let Some(catch) = &try_.catch {
                    quote! {
                        catch ($(&catch.error)) {
                            $(for step in &catch.steps join ($['\n']) =>
                                $(self.render_step(step))
                            )
                        }
                    }
                } else {
                    quote! {}
                };

                let finally_block = if let Some(finally) = &try_.finally {
                    quote! {
                        finally {
                            $(for step in finally join ($['\n']) =>
                                $(self.render_step(step))
                            )
                        }
                    }
                } else {
                    quote! {}
                };

                quote! {
                    // Try: $(&try_.id)
                    try {
                        $(for step in &try_.try_steps join ($['\n']) =>
                            $(self.render_step(step))
                        )
                    } $catch_block $finally_block
                }
            }
            ChainStep::Dynamic(dyn_) => {
                let spec_expr = ts_expr(&dyn_.spec);
                quote! {
                    // Dynamic dispatch: $(&dyn_.id)
                    const specFn = { $(for allowed in &dyn_.allowed join (, ) => $(quoted(allowed)): $(allowed)) }[$spec_expr]
                    if (specFn) await specFn({ /* inputs */ })
                }
            }
            ChainStep::Await(await_) => {
                let expr = ts_expr(&await_.expr);
                quote! {
                    ctx.$(&await_.id) = await $expr
                }
            }
            ChainStep::Emit(emit) => {
                let data = ts_expr(&emit.data);
                quote! {
                    // Emit event: $(&emit.event)
                    // emitter.emit($(quoted(&emit.event)), $data)
                }
            }
        }
    }

    fn render_call(&self, call: &CallStep) -> js::Tokens {
        quote! {
            // $(&call.id)
            ctx.$(&call.id) = await $(&call.spec)({
                $(for (k, v) in &call.inputs join ($['\n']) =>
                    $(k): $(ts_expr(v)),
                )
            })
        }
    }

    fn render_parallel(&self, par: &ParallelStep) -> js::Tokens {
        let ids: Vec<_> = par
            .steps
            .iter()
            .filter_map(|s| match s {
                ChainStep::Call(c) => Some(c.id.clone()),
                _ => None,
            })
            .collect();

        quote! {
            // Parallel: $(&par.id)
            const [$(for id in &ids join (, ) => $(id))] = await Promise.all([
                $(for step in &par.steps join ($['\n']) =>
                    $(if let ChainStep::Call(call) = step {
                        $(&call.spec)({ /* inputs */ }),
                    })
                )
            ])
            $(for id in &ids join ($['\n']) =>
                ctx.$(id) = $(id)
            )
        }
    }

    fn render_branch(&self, branch: &BranchStep) -> js::Tokens {
        let on_expr = ts_expr(&branch.on);
        quote! {
            switch ($on_expr) {
                $(for (val, steps) in &branch.cases join ($['\n']) =>
                    case $(val):
                        $(for s in steps join ($['\n']) =>
                            $(self.render_step(s))
                        )
                        break
                )
                $(if branch.default.is_some() {
                    default:
                        $(for s in branch.default.as_ref().unwrap() join ($['\n']) =>
                            $(self.render_step(s))
                        )
                })
            }
        }
    }

    fn render_foreach(&self, foreach: &ForEachStep) -> js::Tokens {
        let collection = ts_expr(&foreach.collection);
        quote! {
            for (const [$(&foreach.index), $(&foreach.item)] of $collection.entries()) {
                $(for s in &foreach.steps join ($['\n']) =>
                    $(self.render_step(s))
                )
            }
        }
    }

    fn render_gate(&self, gate: &GateStep) -> js::Tokens {
        let cond = ts_expr(&gate.condition);
        let error_msg = format!("Gate {} failed", gate.id);
        quote! {
            if (!($cond)) throw new Error($(quoted(error_msg)))
        }
    }
}

/// Convert expression to TypeScript syntax
fn ts_expr(expr: &str) -> String {
    if expr.contains('.') {
        let parts: Vec<&str> = expr.split('.').collect();
        format!("ctx.{}", parts.join("?."))
    } else {
        format!("input.{}", expr)
    }
}

/// Map VarType to TypeScript type string
fn ts_type(var_type: &crate::spec::VarType) -> &'static str {
    match var_type {
        crate::spec::VarType::Bool => "boolean",
        crate::spec::VarType::Int | crate::spec::VarType::Float => "number",
        crate::spec::VarType::String => "string",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_typescript_orchestrator() {
        let orch = Orchestrator::from_yaml(
            r#"
id: order_flow
inputs:
  - name: user_id
    type: string
outputs:
  - name: order_id
    type: string
uses:
  - validate_cart
chain:
  - step: call
    id: validate
    spec: validate_cart
    inputs:
      id: "user_id"
"#,
        )
        .unwrap();

        let specs = HashMap::new();
        let ts = render(&orch, &specs);

        assert!(ts.contains("export async function order_flow"));
        assert!(ts.contains("OrderFlowInput"));
        assert!(ts.contains("validate_cart"));
    }
}
