//! TypeScript code generation
//!
//! DOGFOODING: Uses generated code for type conversions and literals

use crate::cel::{CelCompiler, Target};
use crate::spec::*;
use chrono::Utc;

use super::scoping::ResolvedNamespace;
use super::{is_expression, to_camel_case, to_pascal_case, RenderConfig};
use super::{translate_vars, VarTranslation};

/// Render spec to TypeScript code
pub fn render(spec: &Spec, config: &RenderConfig) -> String {
    let input_names: Vec<String> = spec.inputs.iter().map(|i| i.name.clone()).collect();
    TypeScriptRenderer {
        config,
        input_names,
    }
    .render(spec)
}

struct TypeScriptRenderer<'a> {
    config: &'a RenderConfig,
    input_names: Vec<String>,
}

impl<'a> TypeScriptRenderer<'a> {
    fn render(&self, spec: &Spec) -> String {
        let mut out = String::new();
        let ind = &self.config.indent;

        // Module path comment if configured
        let module_path = self.config.namespace.as_ref().and_then(|ns| {
            if let ResolvedNamespace::TypeScript(ts_mod) = ns {
                Some(ts_mod.render())
            } else {
                None
            }
        });

        if let Some(ref path) = module_path {
            out.push_str(&format!("/**\n * Module: {}\n */\n\n", path));
        }

        // Header
        if self.config.provenance {
            out.push_str(&format!("// GENERATED FROM: {}.yaml\n", spec.id));
            out.push_str(&format!("// SPEC HASH: {}\n", spec.hash()));
            out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
            out.push_str("// DO NOT EDIT — regenerate from spec\n\n");
        }

        // Input type
        let type_name = to_pascal_case(&spec.id);
        out.push_str(&format!("export interface {}Input {{\n", type_name));
        for input in &spec.inputs {
            out.push_str(&format!(
                "{}{}: {};\n",
                ind,
                to_camel_case(&input.name),
                self.render_type(&input.typ)
            ));
        }
        out.push_str("}\n\n");

        // Function
        let return_type = spec
            .outputs
            .first()
            .map(|v| self.render_type(&v.typ))
            .unwrap_or_else(|| "void".into());

        out.push_str(&format!(
            "export function {}(input: {}Input): {} {{\n",
            to_camel_case(&spec.id),
            type_name,
            return_type
        ));

        // Destructure
        let vars: Vec<_> = spec.inputs.iter().map(|v| to_camel_case(&v.name)).collect();
        out.push_str(&format!(
            "{}const {{ {} }} = input;\n\n",
            ind,
            vars.join(", ")
        ));

        // If-else chain
        for (i, rule) in spec.rules.iter().enumerate() {
            let condition = rule
                .as_cel()
                .map(|cel| {
                    let compiled = CelCompiler::compile(&cel, Target::TypeScript)
                        .unwrap_or_else(|_| cel.clone());
                    translate_vars(&compiled, &self.input_names, VarTranslation::CamelCase)
                })
                .unwrap_or_else(|| "true".into());

            if i == 0 {
                out.push_str(&format!("{}if ({}) {{\n", ind, condition));
            } else {
                out.push_str(&format!("{}}} else if ({}) {{\n", ind, condition));
            }

            if self.config.comments {
                out.push_str(&format!("{}{}// {}\n", ind, ind, rule.id));
            }

            out.push_str(&format!(
                "{}{}return {};\n",
                ind,
                ind,
                self.render_output(&rule.then)
            ));
        }

        if let Some(default) = &spec.default {
            out.push_str(&format!("{}}} else {{\n", ind));
            out.push_str(&format!(
                "{}{}return {};\n",
                ind,
                ind,
                self.render_output(default)
            ));
        }

        out.push_str(&format!("{}}}\n", ind));
        out.push_str("}\n");

        out
    }

    /// DOGFOODING: Use generated type_mapping for TypeScript types
    fn render_type(&self, typ: &VarType) -> String {
        use crate::generated::type_mapping::type_mapping;

        match typ {
            VarType::Bool => type_mapping("Bool".into(), "TypeScript".into()),
            VarType::Int | VarType::Float => type_mapping("Int".into(), "TypeScript".into()), // Both are "number"
            VarType::String => type_mapping("String".into(), "TypeScript".into()),
            VarType::Enum(variants) => variants
                .iter()
                .map(|v| format!("\"{}\"", v))
                .collect::<Vec<_>>()
                .join(" | "),
            VarType::List(inner) => format!("{}[]", self.render_type(inner)),
            VarType::Object => type_mapping("Object".into(), "TypeScript".into()),
        }
    }

    fn render_output(&self, output: &Output) -> String {
        match output {
            Output::Single(v) => self.render_value(v),
            Output::Named(map) => {
                let fields: Vec<_> = map
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.render_value(v)))
                    .collect();
                format!("{{ {} }}", fields.join(", "))
            }
        }
    }

    fn render_value(&self, v: &ConditionValue) -> String {
        match v {
            ConditionValue::Bool(b) => self.render_bool(*b),
            ConditionValue::Int(i) => i.to_string(),
            ConditionValue::Float(f) => f.to_string(),
            ConditionValue::String(s) => {
                // Check if this is a CEL expression or a literal string
                if is_expression(s) {
                    // Compile as CEL expression and translate variable names
                    CelCompiler::compile(s, Target::TypeScript)
                        .map(|compiled| {
                            translate_vars(&compiled, &self.input_names, VarTranslation::CamelCase)
                        })
                        .unwrap_or_else(|_| format!("\"{}\"", s))
                } else {
                    // Literal string
                    format!("\"{}\"", s)
                }
            }
            ConditionValue::Null => self.render_null(),
            _ => "undefined".into(),
        }
    }

    /// DOGFOODING: Use generated bool_literal for boolean rendering
    fn render_bool(&self, value: bool) -> String {
        use crate::generated::bool_literal::bool_literal;
        bool_literal(value, "TypeScript".into())
    }

    /// DOGFOODING: Use generated null_literal for null rendering
    fn render_null(&self) -> String {
        use crate::generated::null_literal::null_literal;
        null_literal("TypeScript".into())
    }
}
