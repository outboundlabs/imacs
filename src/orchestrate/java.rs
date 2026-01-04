//! Java orchestrator code generation using genco

use crate::spec::Spec;
use genco::prelude::*;
use std::collections::HashMap;

use super::{
    collect_step_ids, to_pascal, BranchStep, CallStep, ChainStep, ForEachStep, GateStep, LoopStep,
    Orchestrator, ParallelStep, TryStep,
};

/// Render orchestrator to Java code
pub fn render(orch: &Orchestrator, _specs: &HashMap<String, Spec>) -> String {
    let tokens = JavaOrchRenderer::new(orch).render();
    tokens.to_file_string().unwrap_or_default()
}

struct JavaOrchRenderer<'a> {
    orch: &'a Orchestrator,
}

impl<'a> JavaOrchRenderer<'a> {
    fn new(orch: &'a Orchestrator) -> Self {
        Self { orch }
    }

    fn render(&self) -> java::Tokens {
        let orch_name = to_pascal(&self.orch.id);
        let step_ids = collect_step_ids(&self.orch.chain);

        quote! {
            // Generated orchestrator: $(&self.orch.id)
            // DO NOT EDIT - regenerate with: imacs render $(&self.orch.id)

            package generated;

            import java.util.Map;
            import java.util.HashMap;
            import java.util.List;
            import java.util.concurrent.CompletableFuture;
            import com.fasterxml.jackson.databind.JsonNode;
            import com.fasterxml.jackson.databind.ObjectMapper;

            public class $(&orch_name) {
                private static final ObjectMapper mapper = new ObjectMapper();

                public static class Input {
                    $(for input in &self.orch.inputs join ($['\n']) =>
                        public $(java_type(&input.var_type)) $(&input.name);
                    )

                    public Input() {}
                }

                public static class Output {
                    $(for output in &self.orch.outputs join ($['\n']) =>
                        public $(java_type(&output.var_type)) $(&output.name);
                    )

                    public Output() {}
                }

                public static class Context {
                    $(for id in &step_ids join ($['\n']) =>
                        public JsonNode $(id);
                    )
                }

                public static class OrchestrationException extends RuntimeException {
                    public final String step;
                    public final String reason;

                    public OrchestrationException(String step, String reason) {
                        super("Gate " + step + " failed: " + reason);
                        this.step = step;
                        this.reason = reason;
                    }
                }

                public static CompletableFuture<Output> executeAsync(Input input) {
                    return CompletableFuture.supplyAsync(() -> {
                        try {
                            return execute(input);
                        } catch (Exception e) {
                            throw new RuntimeException(e);
                        }
                    });
                }

                public static Output execute(Input input) throws Exception {
                    Context ctx = new Context();

                    $(for step in &self.orch.chain join ($['\n']) =>
                        $(self.render_step(step, &orch_name))
                    )

                    // Build output
                    Output output = new Output();
                    $(for out in &self.orch.outputs join ($['\n']) =>
                        output.$(&out.name) = null; // TODO: map output
                    )
                    return output;
                }
            }
        }
    }

    fn render_step(&self, step: &ChainStep, orch_name: &str) -> java::Tokens {
        match step {
            ChainStep::Call(call) => self.render_call(call),
            ChainStep::Parallel(par) => self.render_parallel(par, orch_name),
            ChainStep::Branch(branch) => self.render_branch(branch, orch_name),
            ChainStep::Loop(loop_) => self.render_loop(loop_, orch_name),
            ChainStep::ForEach(foreach) => self.render_foreach(foreach, orch_name),
            ChainStep::Gate(gate) => self.render_gate(gate, orch_name),
            ChainStep::Try(try_) => self.render_try(try_, orch_name),
            ChainStep::Compute(compute) => {
                let expr = java_expr(&compute.expr);
                quote! {
                    var $(&compute.name) = $expr;
                }
            }
            ChainStep::Set(set) => {
                let expr = java_expr(&set.value);
                quote! {
                    ctx.$(&set.name) = $expr;
                }
            }
            ChainStep::Return(ret) => {
                let value = java_expr(&ret.value);
                if let Some(cond) = &ret.condition {
                    let cond_expr = java_expr(cond);
                    quote! {
                        if ($cond_expr) {
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
                let spec_expr = java_expr(&dyn_.spec);
                quote! {
                    // Dynamic dispatch: $(&dyn_.id)
                    switch ($spec_expr) {
                        $(for allowed in &dyn_.allowed join ($['\n']) =>
                            case $(quoted(allowed)):
                                $(to_pascal(allowed)).execute(/* inputs */);
                                break;
                        )
                        default:
                            throw new IllegalArgumentException("Unknown spec");
                    }
                }
            }
            ChainStep::Await(await_) => {
                let expr = java_expr(&await_.expr);
                quote! {
                    ctx.$(&await_.id) = $expr.get();
                }
            }
            ChainStep::Emit(emit) => {
                let _data = java_expr(&emit.data);
                quote! {
                    // Emit event: $(&emit.event)
                    // emitter.emit($(quoted(&emit.event)), $data);
                }
            }
        }
    }

    fn render_call(&self, call: &CallStep) -> java::Tokens {
        let spec_pascal = to_pascal(&call.spec);
        let call_id = call.id.clone();

        let core_logic = quote! {
            var $(&call_id)Input = new $(&spec_pascal).Input();
            $(for (spec_input, expr) in &call.inputs join ($['\n']) =>
                $(&call_id)Input.$(spec_input) = $(java_expr(expr));
            )
            var $(&call_id)Result = $(&spec_pascal).execute($(&call_id)Input);
            ctx.$(&call_id) = mapper.valueToTree($(&call_id)Result);
        };

        if let Some(cond) = &call.condition {
            let cond_expr = java_expr(cond);
            quote! {
                // Step: $(&call_id)
                if ($cond_expr) {
                    $core_logic
                }
            }
        } else {
            quote! {
                // Step: $(&call_id)
                $core_logic
            }
        }
    }

    fn render_parallel(&self, par: &ParallelStep, _orch_name: &str) -> java::Tokens {
        quote! {
            // Parallel: $(&par.id)
            CompletableFuture.allOf(
                $(for step in &par.steps join (, ) =>
                    $(if let ChainStep::Call(call) = step {
                        CompletableFuture.runAsync(() -> {
                            try {
                                $(self.render_call(call))
                            } catch (Exception e) {
                                throw new RuntimeException(e);
                            }
                        })
                    })
                )
            ).join();
        }
    }

    fn render_branch(&self, branch: &BranchStep, orch_name: &str) -> java::Tokens {
        let on_expr = java_expr(&branch.on);
        quote! {
            // Branch: $(&branch.id)
            switch ($on_expr) {
                $(for (case_val, steps) in &branch.cases join ($['\n']) =>
                    case $(case_val): {
                        $(for step in steps join ($['\n']) =>
                            $(self.render_step(step, orch_name))
                        )
                        break;
                    }
                )
                $(if let Some(default) = &branch.default {
                    default: {
                        $(for step in default join ($['\n']) =>
                            $(self.render_step(step, orch_name))
                        )
                        break;
                    }
                })
            }
        }
    }

    fn render_loop(&self, loop_: &LoopStep, orch_name: &str) -> java::Tokens {
        let until_check = if let Some(until) = &loop_.until {
            let until_expr = java_expr(until);
            quote! { if ($until_expr) break; }
        } else {
            quote! {}
        };

        quote! {
            // Loop: $(&loop_.id)
            for (var $(&loop_.counter) = 0; $(&loop_.counter) < $(loop_.max_iterations); $(&loop_.counter)++) {
                $until_check
                $(for step in &loop_.steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
            }
        }
    }

    fn render_foreach(&self, foreach: &ForEachStep, orch_name: &str) -> java::Tokens {
        let collection = java_expr(&foreach.collection);
        quote! {
            // ForEach: $(&foreach.id)
            var $(&foreach.index) = 0;
            for (var $(&foreach.item) : $collection) {
                $(for step in &foreach.steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
                $(&foreach.index)++;
            }
        }
    }

    fn render_gate(&self, gate: &GateStep, _orch_name: &str) -> java::Tokens {
        let cond = java_expr(&gate.condition);
        quote! {
            // Gate: $(&gate.id)
            if (!($cond)) {
                throw new OrchestrationException($(quoted(&gate.id)), $(quoted(&gate.condition)));
            }
        }
    }

    fn render_try(&self, try_: &TryStep, orch_name: &str) -> java::Tokens {
        let catch_block = if let Some(catch) = &try_.catch {
            quote! {
                catch (Exception $(&catch.error)) {
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
                finally {
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
            try {
                $(for step in &try_.try_steps join ($['\n']) =>
                    $(self.render_step(step, orch_name))
                )
            }
            $catch_block
            $finally_block
        }
    }
}

/// Convert expression to Java syntax
fn java_expr(expr: &str) -> String {
    if expr.contains('.') {
        let parts: Vec<&str> = expr.split('.').collect();
        let mut result = format!("ctx.{}", parts[0]);
        for part in &parts[1..] {
            result = format!("{}.get(\"{}\")", result, part);
        }
        result
    } else {
        format!("input.{}", expr)
    }
}

/// Map VarType to Java type string
fn java_type(var_type: &crate::spec::VarType) -> &'static str {
    match var_type {
        crate::spec::VarType::Bool => "Boolean",
        crate::spec::VarType::Int => "Long",
        crate::spec::VarType::Float => "Double",
        crate::spec::VarType::String => "String",
        crate::spec::VarType::Object => "JsonNode",
        _ => "JsonNode",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_java_orchestrator() {
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
        let java = render(&orch, &specs);

        assert!(java.contains("public class UserFlow"));
        assert!(java.contains("public static class Input"));
        assert!(java.contains("execute"));
    }
}
