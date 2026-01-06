//! Rust code generation
//!
//! DOGFOODING: Uses generated::type_mapping for type conversions

use crate::cel::{CelCompiler, Target};
use crate::spec::*;
use chrono::Utc;
use std::collections::HashMap;

use super::scoping::ResolvedNamespace;
use super::{is_expression, RenderConfig};

/// Render spec to Rust code
pub fn render(spec: &Spec, config: &RenderConfig) -> String {
    RustRenderer { config }.render(spec)
}

struct RustRenderer<'a> {
    config: &'a RenderConfig,
}

impl<'a> RustRenderer<'a> {
    fn render(&self, spec: &Spec) -> String {
        let mut out = String::new();

        // Module path comment if configured
        let module_path = self.config.namespace.as_ref().and_then(|ns| {
            if let ResolvedNamespace::Rust(rust_mod) = ns {
                Some(rust_mod.render())
            } else {
                None
            }
        });

        if let Some(ref path) = module_path {
            out.push_str(&format!("//! Module: {}\n\n", path));
        }

        // Header
        if self.config.provenance {
            out.push_str(&format!("// GENERATED FROM: {}.yaml\n", spec.id));
            out.push_str(&format!("// SPEC HASH: {}\n", spec.hash()));
            out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
            out.push_str("// DO NOT EDIT — regenerate from spec\n\n");
        }

        // Function signature
        let params: Vec<String> = spec
            .inputs
            .iter()
            .map(|v| format!("{}: {}", v.name, self.render_type(&v.typ)))
            .collect();

        let return_type = if spec.outputs.len() > 1 {
            // Multiple outputs -> tuple
            let types: Vec<_> = spec
                .outputs
                .iter()
                .map(|v| self.render_type(&v.typ))
                .collect();
            format!("({})", types.join(", "))
        } else {
            spec.outputs
                .first()
                .map(|v| self.render_type(&v.typ))
                .unwrap_or_else(|| "()".into())
        };

        out.push_str(&format!(
            "pub fn {}({}) -> {} {{\n",
            spec.id,
            params.join(", "),
            return_type
        ));

        // Render as match or if-else based on structure
        if self.should_use_match(spec) {
            self.render_match(spec, &mut out);
        } else {
            self.render_if_else(spec, &mut out);
        }

        out.push_str("}\n");
        out
    }

    /// DOGFOODING: Use generated type_mapping for type conversions
    fn render_type(&self, typ: &VarType) -> String {
        // Use the generated type_mapping function
        use crate::generated::type_mapping::type_mapping;

        match typ {
            VarType::Bool => type_mapping("Bool".into(), "Rust".into()),
            VarType::Int => type_mapping("Int".into(), "Rust".into()),
            VarType::Float => type_mapping("Float".into(), "Rust".into()),
            VarType::String => type_mapping("String".into(), "Rust".into()),
            VarType::Enum(_) => "String".into(), // Enums render as strings
            VarType::List(inner) => format!("Vec<{}>", self.render_type(inner)),
            VarType::Object => type_mapping("Object".into(), "Rust".into()),
        }
    }

    fn should_use_match(&self, spec: &Spec) -> bool {
        // Use match if all rules use equality conditions on same variables
        spec.rules.iter().all(|r| {
            r.conditions
                .as_ref()
                .map(|c| c.iter().all(|cond| cond.op == ConditionOp::Eq))
                .unwrap_or(false)
        })
    }

    fn render_match(&self, spec: &Spec, out: &mut String) {
        let ind = &self.config.indent;

        // Build match expression
        if spec.inputs.len() == 1 {
            out.push_str(&format!("{}match {} {{\n", ind, spec.inputs[0].name));
        } else {
            let tuple: Vec<_> = spec.inputs.iter().map(|v| v.name.as_str()).collect();
            out.push_str(&format!("{}match ({}) {{\n", ind, tuple.join(", ")));
        }

        // Match arms
        for rule in &spec.rules {
            if self.config.comments {
                out.push_str(&format!("{}{}// {}\n", ind, ind, rule.id));
            }

            let pattern = self.render_pattern(spec, rule);
            let output = self.render_output(&rule.then);
            out.push_str(&format!("{}{}{} => {},\n", ind, ind, pattern, output));
        }

        // Default if specified
        if let Some(default) = &spec.default {
            out.push_str(&format!(
                "{}{}_ => {},\n",
                ind,
                ind,
                self.render_output(default)
            ));
        }

        out.push_str(&format!("{}}}\n", ind));
    }

    fn render_if_else(&self, spec: &Spec, out: &mut String) {
        let ind = &self.config.indent;

        for (i, rule) in spec.rules.iter().enumerate() {
            let condition = rule
                .as_cel()
                .map(|cel| CelCompiler::compile(&cel, Target::Rust).unwrap_or_else(|_| cel.clone()))
                .unwrap_or_else(|| "true".into());

            if i == 0 {
                out.push_str(&format!("{}if {} {{\n", ind, condition));
            } else {
                out.push_str(&format!("{}}} else if {} {{\n", ind, condition));
            }

            if self.config.comments {
                out.push_str(&format!("{}{}// {}\n", ind, ind, rule.id));
            }

            out.push_str(&format!(
                "{}{}{}\n",
                ind,
                ind,
                self.render_output_for_spec(&rule.then, spec)
            ));
        }

        // Default/else - always add one
        out.push_str(&format!("{}}} else {{\n", ind));
        if let Some(default) = &spec.default {
            out.push_str(&format!(
                "{}{}{}\n",
                ind,
                ind,
                self.render_output_for_spec(default, spec)
            ));
        } else {
            out.push_str(&format!(
                "{}{}unreachable!(\"No rule matched\")\n",
                ind, ind
            ));
        }

        out.push_str(&format!("{}}}\n", ind));
    }

    fn render_pattern(&self, spec: &Spec, rule: &Rule) -> String {
        let conditions = rule.conditions.as_ref();

        if spec.inputs.len() == 1 {
            conditions
                .and_then(|c| c.first())
                .map(|c| self.render_value(&c.value))
                .unwrap_or_else(|| "_".into())
        } else {
            let patterns: Vec<String> = spec
                .inputs
                .iter()
                .map(|input| {
                    conditions
                        .and_then(|c| c.iter().find(|cond| cond.var == input.name))
                        .map(|c| self.render_value(&c.value))
                        .unwrap_or_else(|| "_".into())
                })
                .collect();
            format!("({})", patterns.join(", "))
        }
    }

    fn render_value(&self, val: &ConditionValue) -> String {
        match val {
            ConditionValue::Bool(b) => self.render_bool(*b),
            ConditionValue::Int(i) => i.to_string(),
            ConditionValue::Float(f) => format!("{:?}", f),
            ConditionValue::String(s) => format!("\"{}\"", s),
            ConditionValue::Null => self.render_null(),
            _ => "_".into(),
        }
    }

    /// DOGFOODING: Use generated bool_literal for boolean rendering
    fn render_bool(&self, value: bool) -> String {
        use crate::generated::bool_literal::bool_literal;
        bool_literal(value, "Rust".into())
    }

    /// DOGFOODING: Use generated null_literal for null rendering
    fn render_null(&self) -> String {
        use crate::generated::null_literal::null_literal;
        null_literal("Rust".into())
    }

    fn render_output(&self, output: &Output) -> String {
        match output {
            Output::Single(v) => self.render_condition_value(v),
            Output::Named(map) => {
                let fields: Vec<_> = map
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.render_condition_value(v)))
                    .collect();
                format!("{{ {} }}", fields.join(", "))
            }
        }
    }

    fn render_condition_value(&self, v: &ConditionValue) -> String {
        match v {
            ConditionValue::Bool(b) => self.render_bool(*b),
            ConditionValue::Int(i) => i.to_string(),
            ConditionValue::Float(f) => format!("{:?}", f),
            ConditionValue::String(s) => {
                // Check if this is a CEL expression or a literal string
                if is_expression(s) {
                    // Compile as CEL expression
                    CelCompiler::compile(s, Target::Rust).unwrap_or_else(|_| {
                        // Fallback to literal if parsing fails
                        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                        format!("\"{}\".to_string()", escaped)
                    })
                } else {
                    // Literal string
                    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                    format!("\"{}\".to_string()", escaped)
                }
            }
            ConditionValue::Null => self.render_null(),
            _ => "()".into(),
        }
    }

    /// Render output for a spec, handling named outputs as tuples
    fn render_output_for_spec(&self, output: &Output, spec: &Spec) -> String {
        // Handle both Output::Named and Output::Single(ConditionValue::Map)
        // The latter occurs due to serde untagged parsing order
        let map: Option<&HashMap<String, ConditionValue>> = match output {
            Output::Named(m) => Some(m),
            Output::Single(ConditionValue::Map(m)) => Some(m),
            Output::Single(v) => return self.render_condition_value(v),
        };

        let map = map.unwrap();
        if spec.outputs.len() > 1 {
            // Multiple outputs -> tuple in spec output order
            let values: Vec<_> = spec
                .outputs
                .iter()
                .map(|out_var| {
                    map.get(&out_var.name)
                        .map(|v| self.render_condition_value(v))
                        .unwrap_or_else(|| "Default::default()".into())
                })
                .collect();
            format!("({})", values.join(", "))
        } else if let Some(first_output) = spec.outputs.first() {
            // Single output - get its value from the map
            map.get(&first_output.name)
                .map(|v| self.render_condition_value(v))
                .unwrap_or_else(|| "Default::default()".into())
        } else {
            "()".into()
        }
    }
}
