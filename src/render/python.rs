//! Python code generation
//!
//! DOGFOODING: Uses generated code for type conversions and literals

use crate::cel::{CelCompiler, Target};
use crate::spec::*;
use chrono::Utc;

use super::scoping::ResolvedNamespace;
use super::{is_expression, RenderConfig};

/// Render spec to Python code
pub fn render(spec: &Spec, config: &RenderConfig) -> String {
    PythonRenderer { config }.render(spec)
}

struct PythonRenderer<'a> {
    config: &'a RenderConfig,
}

impl<'a> PythonRenderer<'a> {
    fn render(&self, spec: &Spec) -> String {
        let mut out = String::new();
        let _ind = &self.config.indent;

        // Module docstring with namespace if configured
        let module_path = self.config.namespace.as_ref().and_then(|ns| {
            if let ResolvedNamespace::Python(python_mod) = ns {
                Some(python_mod.render())
            } else {
                None
            }
        });

        if let Some(ref path) = module_path {
            out.push_str(&format!("\"\"\"\nModule: {}\n\"\"\"\n\n", path));
        }

        // Header
        if self.config.provenance {
            out.push_str(&format!("# GENERATED FROM: {}.yaml\n", spec.id));
            out.push_str(&format!("# SPEC HASH: {}\n", spec.hash()));
            out.push_str(&format!("# GENERATED: {}\n", Utc::now().to_rfc3339()));
            out.push_str("# DO NOT EDIT — regenerate from spec\n\n");
        }

        // Function signature
        let params: Vec<String> = spec
            .inputs
            .iter()
            .map(|v| format!("{}: {}", v.name, self.render_type(&v.typ)))
            .collect();

        let return_type = spec
            .outputs
            .first()
            .map(|v| self.render_type(&v.typ))
            .unwrap_or_else(|| "None".into());

        out.push_str(&format!(
            "def {}({}) -> {}:\n",
            spec.id,
            params.join(", "),
            return_type
        ));

        // Check if we should use match (structured conditions) or if/else (CEL)
        let uses_cel = spec.rules.iter().any(|r| r.when.is_some());

        if uses_cel {
            // Use if/elif/else for CEL conditions
            self.render_if_else(spec, &mut out);
        } else {
            // Use match for structured conditions
            self.render_match(spec, &mut out);
        }

        out
    }

    fn render_if_else(&self, spec: &Spec, out: &mut String) {
        let ind = &self.config.indent;

        for (i, rule) in spec.rules.iter().enumerate() {
            let condition = rule
                .as_cel()
                .map(|cel| {
                    CelCompiler::compile(&cel, Target::Python).unwrap_or_else(|_| cel.clone())
                })
                .unwrap_or_else(|| "True".into());

            if i == 0 {
                out.push_str(&format!("{}if {}:\n", ind, condition));
            } else {
                out.push_str(&format!("{}elif {}:\n", ind, condition));
            }

            if self.config.comments {
                out.push_str(&format!("{}{}# {}\n", ind, ind, rule.id));
            }

            out.push_str(&format!(
                "{}{}return {}\n",
                ind,
                ind,
                self.render_output(&rule.then)
            ));
        }

        // Default/else
        out.push_str(&format!("{}else:\n", ind));
        if let Some(default) = &spec.default {
            out.push_str(&format!(
                "{}{}return {}\n",
                ind,
                ind,
                self.render_output(default)
            ));
        } else {
            out.push_str(&format!(
                "{}{}raise ValueError(\"No rule matched\")\n",
                ind, ind
            ));
        }
    }

    fn render_match(&self, spec: &Spec, out: &mut String) {
        let ind = &self.config.indent;

        // Match statement
        if spec.inputs.len() == 1 {
            out.push_str(&format!("{}match {}:\n", ind, spec.inputs[0].name));
        } else {
            let tuple: Vec<_> = spec.inputs.iter().map(|v| v.name.as_str()).collect();
            out.push_str(&format!("{}match ({}):\n", ind, tuple.join(", ")));
        }

        for rule in &spec.rules {
            let pattern = self.render_pattern(spec, rule);
            out.push_str(&format!("{}{}case {}:\n", ind, ind, pattern));

            if self.config.comments {
                out.push_str(&format!("{}{}{}# {}\n", ind, ind, ind, rule.id));
            }

            out.push_str(&format!(
                "{}{}{}return {}\n",
                ind,
                ind,
                ind,
                self.render_output(&rule.then)
            ));
        }

        if let Some(default) = &spec.default {
            out.push_str(&format!("{}{}case _:\n", ind, ind));
            out.push_str(&format!(
                "{}{}{}return {}\n",
                ind,
                ind,
                ind,
                self.render_output(default)
            ));
        }
    }

    /// DOGFOODING: Use generated type_mapping for Python types
    fn render_type(&self, typ: &VarType) -> String {
        use crate::generated::type_mapping::type_mapping;

        match typ {
            VarType::Bool => type_mapping("Bool".into(), "Python".into()),
            VarType::Int => type_mapping("Int".into(), "Python".into()),
            VarType::Float => type_mapping("Float".into(), "Python".into()),
            VarType::String => type_mapping("String".into(), "Python".into()),
            VarType::Enum(_) => "str".into(), // Enums render as str
            VarType::List(inner) => format!("list[{}]", self.render_type(inner)),
            VarType::Object => type_mapping("Object".into(), "Python".into()),
        }
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

    fn render_output(&self, output: &Output) -> String {
        match output {
            Output::Single(v) => self.render_value(v),
            Output::Named(map) => {
                let fields: Vec<_> = map
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.render_value(v)))
                    .collect();
                format!("{{ {} }}", fields.join(", "))
            }
        }
    }

    fn render_value(&self, val: &ConditionValue) -> String {
        match val {
            ConditionValue::Bool(b) => self.render_bool(*b),
            ConditionValue::Int(i) => i.to_string(),
            ConditionValue::Float(f) => f.to_string(),
            ConditionValue::String(s) => {
                // Check if this is a CEL expression or a literal string
                if is_expression(s) {
                    // Compile as CEL expression
                    CelCompiler::compile(s, Target::Python).unwrap_or_else(|_| format!("\"{}\"", s))
                } else {
                    // Literal string
                    format!("\"{}\"", s)
                }
            }
            ConditionValue::Null => self.render_null(),
            _ => "_".into(),
        }
    }

    /// DOGFOODING: Use generated bool_literal for Python-specific booleans (True/False)
    fn render_bool(&self, value: bool) -> String {
        use crate::generated::bool_literal::bool_literal;
        bool_literal(value, "Python".into())
    }

    /// DOGFOODING: Use generated null_literal for Python null (None)
    fn render_null(&self) -> String {
        use crate::generated::null_literal::null_literal;
        null_literal("Python".into())
    }
}
