//! Go orchestrator code generation using genco

use crate::spec::Spec;
use genco::prelude::*;
use std::collections::HashMap;

use super::{
    collect_step_ids, to_pascal, BranchStep, CallStep, ChainStep, ForEachStep, GateStep, LoopStep,
    Orchestrator, ParallelStep, TryStep,
};

/// Render orchestrator to Go code
pub fn render(orch: &Orchestrator, _specs: &HashMap<String, Spec>) -> String {
    let tokens = GoOrchRenderer::new(orch).render();
    tokens.to_file_string().unwrap_or_default()
}

struct GoOrchRenderer<'a> {
    orch: &'a Orchestrator,
}

impl<'a> GoOrchRenderer<'a> {
    fn new(orch: &'a Orchestrator) -> Self {
        Self { orch }
    }

    fn render(&self) -> go::Tokens {
        let orch_name = to_pascal(&self.orch.id);
        let step_ids = collect_step_ids(&self.orch.chain);

        quote! {
            // Generated orchestrator: $(&self.orch.id)
            // DO NOT EDIT - regenerate with: imacs render $(&self.orch.id)

            package generated

            import (
                "fmt"
                "sync"
            )

            type $(&orch_name)Input struct {
                $(for input in &self.orch.inputs join ($['\n']) =>
                    $(to_pascal(&input.name)) $(go_type(&input.var_type))
                )
            }

            type $(&orch_name)Output struct {
                $(for output in &self.orch.outputs join ($['\n']) =>
                    $(to_pascal(&output.name)) $(go_type(&output.var_type))
                )
            }

            type $(&orch_name)Context struct {
                $(for id in &step_ids join ($['\n']) =>
                    $(to_pascal(id)) interface{}
                )
            }

            type $(&orch_name)Error struct {
                Step   string
                Reason string
            }

            func (e *$(&orch_name)Error) Error() string {
                return fmt.Sprintf("gate %s failed: %s", e.Step, e.Reason)
            }

            func $(&orch_name)Execute(input *$(&orch_name)Input) (*$(&orch_name)Output, error) {
                ctx := &$(&orch_name)Context{}

                $(for step in &self.orch.chain join ($['\n']) =>
                    $(self.render_step(step, &orch_name))
                )

                // Build output
                output := &$(&orch_name)Output{
                    $(for out in &self.orch.outputs join ($['\n']) =>
                        $(to_pascal(&out.name)): nil, // TODO: map output
                    )
                }
                return output, nil
            }
        }
    }

    fn render_step(&self, step: &ChainStep, orch_name: &str) -> go::Tokens {
        match step {
            ChainStep::Call(call) => self.render_call(call, orch_name),
            ChainStep::Parallel(par) => self.render_parallel(par, orch_name),
            ChainStep::Branch(branch) => self.render_branch(branch, orch_name),
            ChainStep::Loop(loop_) => self.render_loop(loop_, orch_name),
            ChainStep::ForEach(foreach) => self.render_foreach(foreach, orch_name),
            ChainStep::Gate(gate) => self.render_gate(gate, orch_name),
            ChainStep::Try(try_) => self.render_try(try_, orch_name),
            ChainStep::Compute(compute) => {
                let expr = go_expr(&compute.expr);
                quote! {
                    $(&compute.name) := $expr
                }
            }
            ChainStep::Set(set) => {
                let expr = go_expr(&set.value);
                quote! {
                    ctx.$(to_pascal(&set.name)) = $expr
                }
            }
            ChainStep::Return(ret) => {
                let value = go_expr(&ret.value);
                if let Some(cond) = &ret.condition {
                    let cond_expr = go_expr(cond);
                    quote! {
                        if $cond_expr {
                            return $value, nil
                        }
                    }
                } else {
                    quote! {
                        return $value, nil
                    }
                }
            }
            ChainStep::Dynamic(dyn_) => {
                let spec_expr = go_expr(&dyn_.spec);
                quote! {
                    // Dynamic dispatch: $(&dyn_.id)
                    switch $(&spec_expr) {
                    $(for allowed in &dyn_.allowed join ($['\n']) =>
                        case $(quoted(allowed)):
                            $(to_pascal(allowed))Execute(/* inputs */)
                    )
                    default:
                        return nil, fmt.Errorf("unknown spec: %s", $(&spec_expr))
                    }
                }
            }
            ChainStep::Await(await_) => {
                let expr = go_expr(&await_.expr);
                quote! {
                    ctx.$(to_pascal(&await_.id)) = <-$expr
                }
            }
            ChainStep::Emit(emit) => {
                let data = go_expr(&emit.data);
                quote! {
                    // Emit event: $(&emit.event)
                    // emitter.Emit($(quoted(&emit.event)), $data)
                }
            }
        }
    }

    fn render_call(&self, call: &CallStep, orch_name: &str) -> go::Tokens {
        let spec_pascal = to_pascal(&call.spec);
        let call_id = &call.id;
        let call_id_pascal = to_pascal(call_id);

        let core_logic = quote! {
            $(call_id)Input := &$(&spec_pascal)Input{
                $(for (spec_input, expr) in &call.inputs join ($['\n']) =>
                    $(to_pascal(spec_input)): $(go_expr(expr)),
                )
            }
            $(call_id)Result, err := $(&spec_pascal)Execute($(call_id)Input)
            if err != nil {
                return nil, fmt.Errorf("step $(call_id) failed: %w", err)
            }
            ctx.$(&call_id_pascal) = $(call_id)Result
        };

        if let Some(cond) = &call.condition {
            let cond_expr = go_expr(cond);
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

    fn render_parallel(&self, par: &ParallelStep, orch_name: &str) -> go::Tokens {
        quote! {
            // Parallel: $(&par.id)
            {
                var wg sync.WaitGroup
                var mu sync.Mutex
                var errs []error

                $(for step in &par.steps join ($['\n']) =>
                    $(if let ChainStep::Call(call) = step {
                        wg.Add(1)
                        go func() {
                            defer wg.Done()
                            $(self.render_call(call, orch_name))
                        }()
                    })
                )

                wg.Wait()
                if len(errs) > 0 {
                    return nil, errs[0]
                }
            }
        }
    }

    fn render_branch(&self, branch: &BranchStep, orch_name: &str) -> go::Tokens {
        let on_expr = go_expr(&branch.on);
        quote! {
            // Branch: $(&branch.id)
            switch $on_expr {
            $(for (case_val, steps) in &branch.cases join ($['\n']) =>
                case $(case_val):
                    $(for step in steps join ($['\n']) =>
                        $(self.render_step(step, orch_name))
                    )
            )
            $(if let Some(default) = &branch.default {
                default:
                    $(for step in default join ($['\n']) =>
                        $(self.render_step(step, orch_name))
                    )
            })
            }
        }
    }

    fn render_loop(&self, loop_: &LoopStep, orch_name: &str) -> go::Tokens {
        let until_check = if let Some(until) = &loop_.until {
            let until_expr = go_expr(until);
            quote! { if $until_expr { break } }
        } else {
            quote! {}
        };

        quote! {
            // Loop: $(&loop_.id)
            for $(&loop_.counter) := 0; $(&loop_.counter) < $(loop_.max_iterations); $(&loop_.counter)++ {
                $until_check
                $(for step in &loop_.steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
            }
        }
    }

    fn render_foreach(&self, foreach: &ForEachStep, orch_name: &str) -> go::Tokens {
        let collection = go_expr(&foreach.collection);
        quote! {
            // ForEach: $(&foreach.id)
            for $(&foreach.index), $(&foreach.item) := range $collection {
                $(for step in &foreach.steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
            }
        }
    }

    fn render_gate(&self, gate: &GateStep, orch_name: &str) -> go::Tokens {
        let cond = go_expr(&gate.condition);
        quote! {
            // Gate: $(&gate.id)
            if !($cond) {
                return nil, &$(orch_name)Error{Step: $(quoted(&gate.id)), Reason: $(quoted(&gate.condition))}
            }
        }
    }

    fn render_try(&self, try_: &TryStep, orch_name: &str) -> go::Tokens {
        // Go doesn't have try/catch - use defer/recover pattern
        quote! {
            // Try: $(&try_.id)
            func() {
                $(if try_.finally.is_some() {
                    defer func() {
                        $(if let Some(finally) = &try_.finally {
                            $(for step in finally join ($['\n']) =>
                                $(self.render_step(step, orch_name))
                            )
                        })
                    }()
                })
                $(if try_.catch.is_some() {
                    defer func() {
                        if r := recover(); r != nil {
                            $(if let Some(catch) = &try_.catch {
                                $(&catch.error) := r
                                _ = $(&catch.error)
                                $(for step in &catch.steps join ($['\n']) =>
                                    $(self.render_step(step, orch_name))
                                )
                            })
                        }
                    }()
                })
                $(for step in &try_.try_steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
            }()
        }
    }
}

/// Convert expression to Go syntax
fn go_expr(expr: &str) -> String {
    if expr.contains('.') {
        let parts: Vec<&str> = expr.split('.').collect();
        let mut result = format!("ctx.{}", to_pascal(parts[0]));
        for part in &parts[1..] {
            // In Go, we'd use type assertion or map access
            result = format!("{}[\"{}\"]", result, part);
        }
        result
    } else {
        format!("input.{}", to_pascal(expr))
    }
}

/// Map VarType to Go type string
fn go_type(var_type: &crate::spec::VarType) -> &'static str {
    match var_type {
        crate::spec::VarType::Bool => "bool",
        crate::spec::VarType::Int => "int64",
        crate::spec::VarType::Float => "float64",
        crate::spec::VarType::String => "string",
        crate::spec::VarType::Object => "interface{}",
        _ => "interface{}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_go_orchestrator() {
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
        let go = render(&orch, &specs);

        assert!(go.contains("type UserFlowInput struct"));
        assert!(go.contains("func UserFlowExecute"));
    }
}
