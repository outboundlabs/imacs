//! Java code generation using genco
#![allow(for_loops_over_fallibles)]

use crate::cel::{CelCompiler, Target};
use crate::spec::*;
use chrono::Utc;
use genco::prelude::*;

use super::{is_expression, to_camel_case, to_pascal_case, RenderConfig};

use super::{translate_vars, VarTranslation};

/// Render spec to Java code
pub fn render(spec: &Spec, config: &RenderConfig) -> String {
    let input_names: Vec<String> = spec.inputs.iter().map(|i| i.name.clone()).collect();
    let tokens = JavaRenderer {
        config,
        input_names,
    }
    .render(spec);
    tokens.to_file_string().unwrap_or_default()
}

struct JavaRenderer<'a> {
    config: &'a RenderConfig,
    input_names: Vec<String>,
}

impl<'a> JavaRenderer<'a> {
    fn render(&self, spec: &Spec) -> java::Tokens {
        let class_name = to_pascal_case(&spec.id);
        let return_type = self.render_type(spec.outputs.first().map(|v| &v.typ));

        quote! {
            $(if self.config.provenance {
                $(format!("// GENERATED FROM: {}.yaml", spec.id))
                $(format!("// SPEC HASH: {}", spec.hash()))
                $(format!("// GENERATED: {}", Utc::now().to_rfc3339()))
                // DO NOT EDIT — regenerate from spec
                $['\n']
            })

            public class $(&class_name) {
                $['\n']
                $(self.render_input_class(spec, &class_name))
                $['\n']
                public static $return_type evaluate($(&class_name)Input input) {
                    $(self.render_body(spec))
                }
            }
        }
    }

    fn render_input_class(&self, spec: &Spec, class_name: &str) -> java::Tokens {
        quote! {
            public static class $(class_name)Input {
                $(for input in &spec.inputs join ($['\n']) =>
                    public $(self.render_type(Some(&input.typ))) $(to_camel_case(&input.name));
                )
            }
        }
    }

    fn render_body(&self, spec: &Spec) -> java::Tokens {
        let mut tokens = java::Tokens::new();

        for (i, rule) in spec.rules.iter().enumerate() {
            let condition = rule
                .as_cel()
                .map(|cel| {
                    let compiled =
                        CelCompiler::compile(&cel, Target::Java).unwrap_or_else(|_| cel.clone());
                    translate_vars(&compiled, &self.input_names, VarTranslation::InputCamel)
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
                    if ($(&condition)) {
                        $(for c in &comment => $c$['\n'])
                        return $output;
                    }
                });
            } else {
                tokens.append(quote! {
                    else if ($(&condition)) {
                        $(for c in &comment => $c$['\n'])
                        return $output;
                    }
                });
            }
        }

        if let Some(default) = &spec.default {
            let output = self.render_output(default);
            tokens.append(quote! {
                else {
                    return $output;
                }
            });
        }

        tokens
    }

    fn render_type(&self, typ: Option<&VarType>) -> &'static str {
        match typ {
            Some(VarType::Bool) => "boolean",
            Some(VarType::Int) => "long",
            Some(VarType::Float) => "double",
            Some(VarType::String) => "String",
            Some(VarType::Object) => "Object",
            Some(VarType::List(_)) => "List<Object>",
            Some(VarType::Enum(_)) => "String",
            None => "void",
        }
    }

    fn render_output(&self, output: &Output) -> java::Tokens {
        match output {
            Output::Single(v) => self.render_value(v),
            Output::Named(map) => {
                let fields: Vec<_> = map
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.render_value_string(v)))
                    .collect();
                quote!(new Object() {{ $(fields.join(", ")) }})
            }
        }
    }

    fn render_value(&self, v: &ConditionValue) -> java::Tokens {
        match v {
            ConditionValue::Bool(b) => quote!($(b.to_string())),
            ConditionValue::Int(i) => quote!($(format!("{}L", i))),
            ConditionValue::Float(f) => quote!($(format!("{}d", f))),
            ConditionValue::String(s) => {
                // Check if this is a CEL expression or a literal string
                if is_expression(s) {
                    // Compile as CEL expression and translate variable names
                    let compiled = CelCompiler::compile(s, Target::Java)
                        .map(|c| translate_vars(&c, &self.input_names, VarTranslation::InputCamel))
                        .unwrap_or_else(|_| format!("\"{}\"", s));
                    quote!($compiled)
                } else {
                    quote!($(quoted(s)))
                }
            }
            ConditionValue::Null => quote!(null),
            _ => quote!(null),
        }
    }

    fn render_value_string(&self, v: &ConditionValue) -> String {
        match v {
            ConditionValue::Bool(b) => b.to_string(),
            ConditionValue::Int(i) => format!("{}L", i),
            ConditionValue::Float(f) => format!("{}d", f),
            ConditionValue::String(s) => {
                // Check if this is a CEL expression or a literal string
                if is_expression(s) {
                    // Compile as CEL expression and translate variable names
                    CelCompiler::compile(s, Target::Java)
                        .map(|c| translate_vars(&c, &self.input_names, VarTranslation::InputCamel))
                        .unwrap_or_else(|_| format!("\"{}\"", s))
                } else {
                    format!("\"{}\"", s)
                }
            }
            ConditionValue::Null => "null".into(),
            _ => "null".into(),
        }
    }
}
