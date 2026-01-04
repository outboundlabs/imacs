//! Go code generation using genco
#![allow(for_loops_over_fallibles)]

use crate::cel::{CelCompiler, Target};
use crate::spec::*;
use chrono::Utc;
use genco::prelude::*;

use super::{is_expression, to_pascal_case, RenderConfig};

use super::{translate_vars, VarTranslation};

/// Render spec to Go code
pub fn render(spec: &Spec, config: &RenderConfig) -> String {
    let input_names: Vec<String> = spec.inputs.iter().map(|i| i.name.clone()).collect();
    let tokens = GoRenderer {
        config,
        input_names,
    }
    .render(spec);
    tokens.to_file_string().unwrap_or_default()
}

struct GoRenderer<'a> {
    config: &'a RenderConfig,
    input_names: Vec<String>,
}

impl<'a> GoRenderer<'a> {
    fn render(&self, spec: &Spec) -> go::Tokens {
        let func_name = to_pascal_case(&spec.id);
        let return_type = self.render_type(spec.outputs.first().map(|v| &v.typ));

        quote! {
            $(if self.config.provenance {
                $(format!("// GENERATED FROM: {}.yaml", spec.id))
                $(format!("// SPEC HASH: {}", spec.hash()))
                $(format!("// GENERATED: {}", Utc::now().to_rfc3339()))
                // DO NOT EDIT — regenerate from spec
                $['\n']
            })

            package $(to_snake_case(&spec.id))
            $['\n']

            $(self.render_input_struct(spec, &func_name))
            $['\n']

            func $(&func_name)(input $(&func_name)Input) $return_type {
                $(self.render_body(spec))
            }
        }
    }

    fn render_input_struct(&self, spec: &Spec, func_name: &str) -> go::Tokens {
        quote! {
            type $(func_name)Input struct {
                $(for input in &spec.inputs join ($['\n']) =>
                    $(to_pascal_case(&input.name)) $(self.render_type(Some(&input.typ)))
                )
            }
        }
    }

    fn render_body(&self, spec: &Spec) -> go::Tokens {
        let mut tokens = go::Tokens::new();

        for (i, rule) in spec.rules.iter().enumerate() {
            let condition = rule
                .as_cel()
                .map(|cel| {
                    let compiled =
                        CelCompiler::compile(&cel, Target::Go).unwrap_or_else(|_| cel.clone());
                    translate_vars(&compiled, &self.input_names, VarTranslation::InputPascal)
                })
                .unwrap_or_else(|| "true".into());

            let output = self.render_output(&rule.then);
            let comment = if self.config.comments {
                Some(format!("// {}", rule.id))
            } else {
                None
            };

            if i == 0 {
                tokens.append(quote! {
                    if $(&condition) {
                        $(for c in &comment => $c$['\n'])
                        return $output
                    }
                });
            } else {
                tokens.append(quote! {
                    else if $(&condition) {
                        $(for c in &comment => $c$['\n'])
                        return $output
                    }
                });
            }
        }

        if let Some(default) = &spec.default {
            let output = self.render_output(default);
            tokens.append(quote! {
                else {
                    return $output
                }
            });
        }

        tokens
    }

    fn render_type(&self, typ: Option<&VarType>) -> &'static str {
        match typ {
            Some(VarType::Bool) => "bool",
            Some(VarType::Int) => "int64",
            Some(VarType::Float) => "float64",
            Some(VarType::String) => "string",
            Some(VarType::Object) => "interface{}",
            Some(VarType::List(_)) => "[]interface{}",
            Some(VarType::Enum(_)) => "string",
            None => "",
        }
    }

    fn render_output(&self, output: &Output) -> go::Tokens {
        match output {
            Output::Single(v) => self.render_value(v),
            Output::Named(map) => {
                let fields: Vec<_> = map
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.render_value_string(v)))
                    .collect();
                quote!(map[string]interface{}{$(fields.join(", "))})
            }
        }
    }

    fn render_value(&self, v: &ConditionValue) -> go::Tokens {
        match v {
            ConditionValue::Bool(b) => quote!($(b.to_string())),
            ConditionValue::Int(i) => quote!($(i.to_string())),
            ConditionValue::Float(f) => quote!($(f.to_string())),
            ConditionValue::String(s) => {
                // Check if this is a CEL expression or a literal string
                if is_expression(s) {
                    // Compile as CEL expression and translate variable names
                    let compiled = CelCompiler::compile(s, Target::Go)
                        .map(|c| translate_vars(&c, &self.input_names, VarTranslation::InputPascal))
                        .unwrap_or_else(|_| format!("\"{}\"", s));
                    quote!($compiled)
                } else {
                    quote!($(quoted(s)))
                }
            }
            ConditionValue::Null => quote!(nil),
            _ => quote!(nil),
        }
    }

    fn render_value_string(&self, v: &ConditionValue) -> String {
        match v {
            ConditionValue::Bool(b) => b.to_string(),
            ConditionValue::Int(i) => i.to_string(),
            ConditionValue::Float(f) => f.to_string(),
            ConditionValue::String(s) => {
                // Check if this is a CEL expression or a literal string
                if is_expression(s) {
                    // Compile as CEL expression and translate variable names
                    CelCompiler::compile(s, Target::Go)
                        .map(|c| translate_vars(&c, &self.input_names, VarTranslation::InputPascal))
                        .unwrap_or_else(|_| format!("\"{}\"", s))
                } else {
                    format!("\"{}\"", s)
                }
            }
            ConditionValue::Null => "nil".into(),
            _ => "nil".into(),
        }
    }
}

fn to_snake_case(s: &str) -> String {
    s.to_lowercase()
}
