//! C# orchestrator code generation using genco

use crate::spec::Spec;
use genco::prelude::*;
use std::collections::HashMap;

use super::{
    collect_step_ids, to_pascal, BranchStep, CallStep, ChainStep, ForEachStep, GateStep, LoopStep,
    Orchestrator, ParallelStep, TryStep,
};

/// Render orchestrator to C# code
pub fn render(orch: &Orchestrator, _specs: &HashMap<String, Spec>) -> String {
    let tokens = CSharpOrchRenderer::new(orch).render();
    tokens.to_file_string().unwrap_or_default()
}

struct CSharpOrchRenderer<'a> {
    orch: &'a Orchestrator,
}

impl<'a> CSharpOrchRenderer<'a> {
    fn new(orch: &'a Orchestrator) -> Self {
        Self { orch }
    }

    fn render(&self) -> csharp::Tokens {
        let orch_name = to_pascal(&self.orch.id);
        let step_ids = collect_step_ids(&self.orch.chain);

        quote! {
            // Generated orchestrator: $(&self.orch.id)
            // DO NOT EDIT - regenerate with: imacs render $(&self.orch.id)

            using System;
            using System.Collections.Generic;
            using System.Threading.Tasks;
            using Newtonsoft.Json.Linq;

            namespace Generated
            {
                public class $(&orch_name)Input
                {
                    $(for input in &self.orch.inputs join ($['\n']) =>
                        public $(csharp_type(&input.var_type)) $(&input.name) { get; set; }
                    )
                }

                public class $(&orch_name)Output
                {
                    $(for output in &self.orch.outputs join ($['\n']) =>
                        public $(csharp_type(&output.var_type)) $(&output.name) { get; set; }
                    )
                }

                public class $(&orch_name)Context
                {
                    $(for id in &step_ids join ($['\n']) =>
                        public JToken $(id) { get; set; }
                    )
                }

                public class $(&orch_name)Exception : Exception
                {
                    public string Step { get; }
                    public string Reason { get; }

                    public $(&orch_name)Exception(string step, string reason)
                        : base($[str](Gate {step} failed: {reason}))
                    {
                        Step = step;
                        Reason = reason;
                    }
                }

                public static class $(&orch_name)
                {
                    public static async Task<$(&orch_name)Output> ExecuteAsync($(&orch_name)Input input)
                    {
                        var ctx = new $(&orch_name)Context();

                        $(for step in &self.orch.chain join ($['\n']) =>
                            $(self.render_step(step, &orch_name))
                        )

                        // Build output
                        return new $(&orch_name)Output
                        {
                            $(for output in &self.orch.outputs join ($['\n']) =>
                                $(&output.name) = default, // TODO: map output
                            )
                        };
                    }
                }
            }
        }
    }

    fn render_step(&self, step: &ChainStep, orch_name: &str) -> csharp::Tokens {
        match step {
            ChainStep::Call(call) => self.render_call(call),
            ChainStep::Parallel(par) => self.render_parallel(par, orch_name),
            ChainStep::Branch(branch) => self.render_branch(branch, orch_name),
            ChainStep::Loop(loop_) => self.render_loop(loop_, orch_name),
            ChainStep::ForEach(foreach) => self.render_foreach(foreach, orch_name),
            ChainStep::Gate(gate) => self.render_gate(gate, orch_name),
            ChainStep::Try(try_) => self.render_try(try_, orch_name),
            ChainStep::Compute(compute) => {
                let expr = csharp_expr(&compute.expr);
                quote! {
                    var $(&compute.name) = $expr;
                }
            }
            ChainStep::Set(set) => {
                let expr = csharp_expr(&set.value);
                quote! {
                    ctx.$(&set.name) = $expr;
                }
            }
            ChainStep::Return(ret) => {
                let value = csharp_expr(&ret.value);
                if let Some(cond) = &ret.condition {
                    let cond_expr = csharp_expr(cond);
                    quote! {
                        if ($cond_expr)
                        {
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
                let spec_expr = csharp_expr(&dyn_.spec);
                quote! {
                    // Dynamic dispatch: $(&dyn_.id)
                    switch ($spec_expr)
                    {
                        $(for allowed in &dyn_.allowed join ($['\n']) =>
                            case $(quoted(allowed)):
                                await $(to_pascal(allowed)).ExecuteAsync(/* inputs */);
                                break;
                        )
                        default:
                            throw new InvalidOperationException("Unknown spec");
                    }
                }
            }
            ChainStep::Await(await_) => {
                let expr = csharp_expr(&await_.expr);
                quote! {
                    ctx.$(&await_.id) = await $expr;
                }
            }
            ChainStep::Emit(emit) => {
                let data = csharp_expr(&emit.data);
                quote! {
                    // Emit event: $(&emit.event)
                    // await emitter.EmitAsync($(quoted(&emit.event)), $data);
                }
            }
        }
    }

    fn render_call(&self, call: &CallStep) -> csharp::Tokens {
        let spec_pascal = to_pascal(&call.spec);
        let call_id = &call.id;

        let core_logic = quote! {
            var $(call_id)Input = new $(&spec_pascal)Input
            {
                $(for (spec_input, expr) in &call.inputs join ($['\n']) =>
                    $(to_pascal(spec_input)) = $(csharp_expr(expr)),
                )
            };
            var $(call_id)Result = await $(&spec_pascal).ExecuteAsync($(call_id)Input);
            ctx.$(call_id) = JToken.FromObject($(call_id)Result);
        };

        if let Some(cond) = &call.condition {
            let cond_expr = csharp_expr(cond);
            quote! {
                // Step: $(call_id)
                if ($cond_expr)
                {
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

    fn render_parallel(&self, par: &ParallelStep, orch_name: &str) -> csharp::Tokens {
        quote! {
            // Parallel: $(&par.id)
            await Task.WhenAll(
                $(for step in &par.steps join (, ) =>
                    $(if let ChainStep::Call(call) = step {
                        Task.Run(async () => {
                            $(self.render_call(call))
                        })
                    })
                )
            );
        }
    }

    fn render_branch(&self, branch: &BranchStep, orch_name: &str) -> csharp::Tokens {
        let on_expr = csharp_expr(&branch.on);
        quote! {
            // Branch: $(&branch.id)
            switch ($on_expr)
            {
                $(for (case_val, steps) in &branch.cases join ($['\n']) =>
                    case $(case_val):
                        $(for step in steps join ($['\n']) =>
                            $(self.render_step(step, orch_name))
                        )
                        break;
                )
                $(if let Some(default) = &branch.default {
                    default:
                        $(for step in default join ($['\n']) =>
                            $(self.render_step(step, orch_name))
                        )
                        break;
                })
            }
        }
    }

    fn render_loop(&self, loop_: &LoopStep, orch_name: &str) -> csharp::Tokens {
        let until_check = if let Some(until) = &loop_.until {
            let until_expr = csharp_expr(until);
            quote! { if ($until_expr) break; }
        } else {
            quote! {}
        };

        quote! {
            // Loop: $(&loop_.id)
            for (var $(&loop_.counter) = 0; $(&loop_.counter) < $(loop_.max_iterations); $(&loop_.counter)++)
            {
                $until_check
                $(for step in &loop_.steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
            }
        }
    }

    fn render_foreach(&self, foreach: &ForEachStep, orch_name: &str) -> csharp::Tokens {
        let collection = csharp_expr(&foreach.collection);
        quote! {
            // ForEach: $(&foreach.id)
            var $(&foreach.index) = 0;
            foreach (var $(&foreach.item) in $collection)
            {
                $(for step in &foreach.steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
                $(&foreach.index)++;
            }
        }
    }

    fn render_gate(&self, gate: &GateStep, orch_name: &str) -> csharp::Tokens {
        let cond = csharp_expr(&gate.condition);
        quote! {
            // Gate: $(&gate.id)
            if (!($cond))
            {
                throw new $(orch_name)Exception($(quoted(&gate.id)), $(quoted(&gate.condition)));
            }
        }
    }

    fn render_try(&self, try_: &TryStep, orch_name: &str) -> csharp::Tokens {
        let catch_block = if let Some(catch) = &try_.catch {
            quote! {
                catch (Exception $(&catch.error))
                {
                    $(for step in &catch.steps join ($['\n']) =>
                        $(self.render_step(step, orch_name))
                    )
                }
            }
        } else {
            quote! {}
        };

        let finally_block = if let Some(finally) = &try_.finally {
            quote! {
                finally
                {
                    $(for step in finally join ($['\n']) =>
                        $(self.render_step(step, orch_name))
                    )
                }
            }
        } else {
            quote! {}
        };

        quote! {
            // Try: $(&try_.id)
            try
            {
                $(for step in &try_.try_steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
            }
            $catch_block
            $finally_block
        }
    }
}

/// Convert expression to C# syntax
fn csharp_expr(expr: &str) -> String {
    if expr.contains('.') {
        let parts: Vec<&str> = expr.split('.').collect();
        let mut result = format!("ctx.{}", parts[0]);
        for part in &parts[1..] {
            result = format!("{}[\"{}\"]", result, part);
        }
        result
    } else {
        format!("input.{}", expr)
    }
}

/// Map VarType to C# type string
fn csharp_type(var_type: &crate::spec::VarType) -> &'static str {
    match var_type {
        crate::spec::VarType::Bool => "bool",
        crate::spec::VarType::Int => "long",
        crate::spec::VarType::Float => "double",
        crate::spec::VarType::String => "string",
        crate::spec::VarType::Object => "JToken",
        _ => "JToken",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_csharp_orchestrator() {
        let orch = Orchestrator::from_yaml(
            r#"
id: user_flow
inputs:
  - name: userId
    type: string
outputs:
  - name: profile
    type: object
uses:
  - get_user
chain:
  - step: call
    id: fetch
    spec: get_user
    inputs:
      id: "userId"
"#,
        )
        .unwrap();

        let specs = HashMap::new();
        let cs = render(&orch, &specs);

        assert!(cs.contains("public static class UserFlow"));
        assert!(cs.contains("UserFlowInput"));
        assert!(cs.contains("ExecuteAsync"));
    }
}
