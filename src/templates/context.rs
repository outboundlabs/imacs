//! Template context structures
//!
//! Converts Spec and Orchestrator into template-friendly data structures.

use crate::cel::{CelCompiler, Target};
use crate::spec::{ConditionOp, ConditionValue, Output, Rule, Spec, VarType, Variable};
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;

/// Context for spec template rendering
#[derive(Debug, Clone, Serialize)]
pub struct SpecContext {
    /// Spec ID
    pub id: String,
    /// PascalCase version of ID (for class/struct names)
    pub id_pascal: String,
    /// camelCase version of ID (for function names)
    pub id_camel: String,
    /// Spec hash for provenance
    pub spec_hash: String,
    /// Whether to include provenance header
    pub provenance: bool,
    /// Generation timestamp
    pub generated_at: String,
    /// Input variables
    pub inputs: Vec<InputView>,
    /// Output variables
    pub outputs: Vec<OutputView>,
    /// Rules
    pub rules: Vec<RuleView>,
    /// Default output (if specified)
    pub default: Option<OutputValueView>,
    /// Whether to use match/switch vs if-else
    pub use_match: bool,
    /// Whether HashMap import is needed (for Rust)
    pub needs_hashmap: bool,
    /// Whether outputs are named (Output::Named) - affects return type
    pub has_named_outputs: bool,
    /// Target language
    pub target: String,
    /// Description (if any)
    pub description: Option<String>,
    // Namespace fields for scoping
    /// C# namespace (e.g., "Company.Rules.Auth")
    pub namespace: Option<String>,
    /// Java package (e.g., "com.company.rules.auth")
    pub package: Option<String>,
    /// Go module path (e.g., "github.com/company/rules/auth")
    pub module_path: Option<String>,
    /// Python/Rust/TypeScript module (e.g., "company.rules.auth")
    pub module: Option<String>,
}

/// View of an input variable
#[derive(Debug, Clone, Serialize)]
pub struct InputView {
    /// Variable name (snake_case)
    pub name: String,
    /// PascalCase name
    pub name_pascal: String,
    /// camelCase name
    pub name_camel: String,
    /// Type as string
    pub var_type: String,
    /// Rust type
    pub rust_type: String,
    /// TypeScript type
    pub ts_type: String,
    /// Python type
    pub py_type: String,
    /// Go type
    pub go_type: String,
    /// Java type
    pub java_type: String,
    /// C# type
    pub csharp_type: String,
}

/// View of an output variable
#[derive(Debug, Clone, Serialize)]
pub struct OutputView {
    /// Variable name
    pub name: String,
    /// PascalCase name
    pub name_pascal: String,
    /// camelCase name
    pub name_camel: String,
    /// Type as string
    pub var_type: String,
    /// Rust type
    pub rust_type: String,
    /// TypeScript type
    pub ts_type: String,
    /// Python type
    pub py_type: String,
    /// Go type
    pub go_type: String,
    /// Java type
    pub java_type: String,
    /// C# type
    pub csharp_type: String,
}

/// View of a rule
#[derive(Debug, Clone, Serialize)]
pub struct RuleView {
    /// Rule ID
    pub id: String,
    /// Condition as Rust code
    pub condition_rust: String,
    /// Condition as TypeScript code
    pub condition_ts: String,
    /// Condition as Python code
    pub condition_py: String,
    /// Condition as Go code
    pub condition_go: String,
    /// Condition as Java code
    pub condition_java: String,
    /// Condition as C# code
    pub condition_csharp: String,
    /// Pattern for match statements (Rust)
    pub pattern_rust: String,
    /// Pattern for match statements (Python)
    pub pattern_py: String,
    /// Output value
    pub output: OutputValueView,
    /// Whether this rule uses CEL (vs simple conditions)
    pub is_cel: bool,
    /// Raw CEL expression (if any)
    pub cel_expr: Option<String>,
}

/// View of an output value
#[derive(Debug, Clone, Serialize)]
pub struct OutputValueView {
    /// Is this a single value or named map?
    pub is_single: bool,
    /// Single value rendered for Rust
    pub rust: String,
    /// Single value rendered for TypeScript
    pub ts: String,
    /// Single value rendered for Python
    pub py: String,
    /// Single value rendered for Go
    pub go: String,
    /// Single value rendered for Java
    pub java: String,
    /// Single value rendered for C#
    pub csharp: String,
    /// Named values (if Output::Named)
    pub named: Option<HashMap<String, NamedValueView>>,
}

/// View of a named output value
#[derive(Debug, Clone, Serialize)]
pub struct NamedValueView {
    pub rust: String,
    pub ts: String,
    pub py: String,
    pub go: String,
    pub java: String,
    pub csharp: String,
}

impl SpecContext {
    /// Create a SpecContext from a Spec
    pub fn from_spec(spec: &Spec, target: Target, provenance: bool) -> Self {
        let inputs: Vec<InputView> = spec.inputs.iter().map(InputView::from_var).collect();
        let input_names: Vec<String> = inputs.iter().map(|i| i.name.clone()).collect();

        let outputs: Vec<OutputView> = spec.outputs.iter().map(OutputView::from_var).collect();

        let rules: Vec<RuleView> = spec
            .rules
            .iter()
            .map(|r| RuleView::from_rule(r, &input_names, &spec.inputs))
            .collect();

        let default = spec
            .default
            .as_ref()
            .map(|d| OutputValueView::from_output(d, &input_names));

        // Check if return type should be HashMap (only when no outputs are defined in spec)
        // When spec.outputs is defined, we always use tuple/single return type
        let has_named_outputs = spec.outputs.is_empty()
            && (rules.iter().any(|r| r.output.named.is_some())
                || default.as_ref().is_some_and(|d| d.named.is_some()));

        // Determine if we should use match (all rules have simple equality conditions)
        let use_match = spec.rules.iter().all(|r| {
            r.conditions
                .as_ref()
                .map(|c| c.iter().all(|cond| cond.op == ConditionOp::Eq))
                .unwrap_or(false)
        });

        // Check if HashMap is needed (for Rust) - only when outputs are dynamic (not defined in spec)
        let needs_hashmap = has_named_outputs;

        // Extract namespace values from scoping config
        let (namespace, package, module_path, module) = extract_namespace_fields(spec, target);

        Self {
            id: spec.id.clone(),
            id_pascal: to_pascal_case(&spec.id),
            id_camel: to_camel_case(&spec.id),
            spec_hash: spec.hash(),
            provenance,
            generated_at: Utc::now().to_rfc3339(),
            inputs,
            outputs,
            rules,
            default,
            use_match,
            needs_hashmap,
            has_named_outputs,
            target: format!("{:?}", target),
            description: spec.description.clone(),
            namespace,
            package,
            module_path,
            module,
        }
    }
}

/// Extract namespace fields from spec scoping config based on target language
fn extract_namespace_fields(
    spec: &Spec,
    target: Target,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let scoping = match &spec.scoping {
        Some(s) => s,
        None => return (None, None, None, None),
    };

    match target {
        Target::CSharp => {
            let ns = scoping.languages.csharp.as_ref().map(|n| n.render());
            (ns, None, None, None)
        }
        Target::Java => {
            let pkg = scoping.languages.java.as_ref().map(|p| p.render());
            (None, pkg, None, None)
        }
        Target::Go => {
            let (pkg, module) = scoping
                .languages
                .go
                .as_ref()
                .map(|g| (Some(g.render()), g.module_path.clone()))
                .unwrap_or((None, None));
            (None, pkg, module, None)
        }
        Target::Python => {
            let module = scoping.languages.python.as_ref().map(|p| p.render());
            (None, None, None, module)
        }
        Target::Rust => {
            let module = scoping.languages.rust.as_ref().map(|r| r.render());
            (None, None, None, module)
        }
        Target::TypeScript => {
            let module = scoping.languages.typescript.as_ref().map(|t| t.render());
            (None, None, None, module)
        }
    }
}

impl InputView {
    fn from_var(var: &Variable) -> Self {
        let var_type = format_var_type(&var.typ);
        Self {
            name: var.name.clone(),
            name_pascal: to_pascal_case(&var.name),
            name_camel: to_camel_case(&var.name),
            var_type: var_type.clone(),
            rust_type: map_type_rust(&var.typ),
            ts_type: map_type_ts(&var.typ),
            py_type: map_type_python(&var.typ),
            go_type: map_type_go(&var.typ),
            java_type: map_type_java(&var.typ),
            csharp_type: map_type_csharp(&var.typ),
        }
    }
}

impl OutputView {
    fn from_var(var: &Variable) -> Self {
        let var_type = format_var_type(&var.typ);
        Self {
            name: var.name.clone(),
            name_pascal: to_pascal_case(&var.name),
            name_camel: to_camel_case(&var.name),
            var_type: var_type.clone(),
            rust_type: map_type_rust(&var.typ),
            ts_type: map_type_ts(&var.typ),
            py_type: map_type_python(&var.typ),
            go_type: map_type_go(&var.typ),
            java_type: map_type_java(&var.typ),
            csharp_type: map_type_csharp(&var.typ),
        }
    }
}

impl RuleView {
    fn from_rule(rule: &Rule, input_names: &[String], inputs: &[Variable]) -> Self {
        let cel_expr = rule.as_cel();
        let is_cel = cel_expr.is_some();

        // Compile conditions to each language
        let (
            condition_rust,
            condition_ts,
            condition_py,
            condition_go,
            condition_java,
            condition_csharp,
        ) = if let Some(cel) = &cel_expr {
            (
                CelCompiler::compile(cel, Target::Rust).unwrap_or_else(|_| "true".into()),
                compile_ts_condition(cel, input_names),
                CelCompiler::compile(cel, Target::Python).unwrap_or_else(|_| "True".into()),
                compile_go_condition(cel, input_names),
                compile_java_condition(cel, input_names),
                compile_csharp_condition(cel, input_names),
            )
        } else {
            (
                "true".into(),
                "true".into(),
                "True".into(),
                "true".into(),
                "true".into(),
                "true".into(),
            )
        };

        // Generate match patterns
        let pattern_rust = generate_rust_pattern(rule, inputs);
        let pattern_py = generate_python_pattern(rule, inputs);

        let output = OutputValueView::from_output(&rule.then, input_names);

        Self {
            id: rule.id.clone(),
            condition_rust,
            condition_ts,
            condition_py,
            condition_go,
            condition_java,
            condition_csharp,
            pattern_rust,
            pattern_py,
            output,
            is_cel,
            cel_expr,
        }
    }
}

impl OutputValueView {
    fn from_output(output: &Output, input_names: &[String]) -> Self {
        // Helper to build named output view from a map
        let build_named = |map: &HashMap<String, ConditionValue>| -> Self {
            let named: HashMap<String, NamedValueView> = map
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        NamedValueView {
                            rust: render_value_rust(v, input_names),
                            ts: render_value_ts(v, input_names),
                            py: render_value_python(v, input_names),
                            go: render_value_go(v, input_names),
                            java: render_value_java(v, input_names),
                            csharp: render_value_csharp(v, input_names),
                        },
                    )
                })
                .collect();
            Self {
                is_single: false,
                rust: String::new(),
                ts: String::new(),
                py: String::new(),
                go: String::new(),
                java: String::new(),
                csharp: String::new(),
                named: Some(named),
            }
        };

        match output {
            // Handle ConditionValue::Map as named output (serde untagged may parse it this way)
            Output::Single(ConditionValue::Map(map)) => build_named(map),
            Output::Single(val) => Self {
                is_single: true,
                rust: render_value_rust(val, input_names),
                ts: render_value_ts(val, input_names),
                py: render_value_python(val, input_names),
                go: render_value_go(val, input_names),
                java: render_value_java(val, input_names),
                csharp: render_value_csharp(val, input_names),
                named: None,
            },
            Output::Named(map) => build_named(map),
        }
    }
}

// Re-export from shared util module
use crate::util::{to_camel_case, to_pascal_case};

// ============================================================================
// Type mapping helpers
// ============================================================================

fn format_var_type(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "bool".to_string(),
        VarType::Int => "int".to_string(),
        VarType::Float => "float".to_string(),
        VarType::String => "string".to_string(),
        VarType::Object => "object".to_string(),
        VarType::Enum(variants) => format!("enum({})", variants.join("|")),
        VarType::List(inner) => format!("List<{}>", format_var_type(inner)),
    }
}

fn map_type_rust(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "bool".to_string(),
        VarType::Int => "i64".to_string(),
        VarType::Float => "f64".to_string(),
        VarType::String => "String".to_string(),
        VarType::Object => "serde_json::Value".to_string(),
        VarType::Enum(_) => "String".to_string(),
        VarType::List(inner) => format!("Vec<{}>", map_type_rust(inner)),
    }
}

fn map_type_ts(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "boolean".to_string(),
        VarType::Int | VarType::Float => "number".to_string(),
        VarType::String => "string".to_string(),
        VarType::Object => "Record<string, unknown>".to_string(),
        VarType::Enum(variants) => {
            let quoted: Vec<_> = variants.iter().map(|v| format!("\"{}\"", v)).collect();
            quoted.join(" | ")
        }
        VarType::List(inner) => format!("{}[]", map_type_ts(inner)),
    }
}

fn map_type_python(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "bool".to_string(),
        VarType::Int => "int".to_string(),
        VarType::Float => "float".to_string(),
        VarType::String => "str".to_string(),
        VarType::Object => "dict[str, Any]".to_string(),
        VarType::Enum(_) => "str".to_string(),
        VarType::List(inner) => format!("list[{}]", map_type_python(inner)),
    }
}

fn map_type_go(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "bool".to_string(),
        VarType::Int => "int64".to_string(),
        VarType::Float => "float64".to_string(),
        VarType::String => "string".to_string(),
        VarType::Object => "map[string]interface{}".to_string(),
        VarType::Enum(_) => "string".to_string(),
        VarType::List(inner) => format!("[]{}", map_type_go(inner)),
    }
}

fn map_type_java(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "boolean".to_string(),
        VarType::Int => "long".to_string(),
        VarType::Float => "double".to_string(),
        VarType::String => "String".to_string(),
        VarType::Object => "Map<String, Object>".to_string(),
        VarType::Enum(_) => "String".to_string(),
        VarType::List(inner) => format!("List<{}>", map_type_java_boxed(inner)),
    }
}

fn map_type_java_boxed(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "Boolean".to_string(),
        VarType::Int => "Long".to_string(),
        VarType::Float => "Double".to_string(),
        _ => map_type_java(typ),
    }
}

fn map_type_csharp(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "bool".to_string(),
        VarType::Int => "long".to_string(),
        VarType::Float => "double".to_string(),
        VarType::String => "string".to_string(),
        VarType::Object => "Dictionary<string, object>".to_string(),
        VarType::Enum(_) => "string".to_string(),
        VarType::List(inner) => format!("List<{}>", map_type_csharp(inner)),
    }
}

// ============================================================================
// Value rendering helpers
// ============================================================================

#[allow(clippy::only_used_in_recursion)]
fn render_value_rust(val: &ConditionValue, input_names: &[String]) -> String {
    match val {
        ConditionValue::Bool(b) => b.to_string(),
        ConditionValue::Int(i) => format!("{}i64", i),
        ConditionValue::Float(f) => format!("{:?}f64", f),
        ConditionValue::String(s) => {
            if is_expression(s) {
                CelCompiler::compile(s, Target::Rust)
                    .unwrap_or_else(|_| format!("\"{}\".to_string()", escape_string(s)))
            } else {
                format!("\"{}\".to_string()", escape_string(s))
            }
        }
        ConditionValue::Null => "None".to_string(),
        ConditionValue::List(items) => {
            let rendered: Vec<_> = items
                .iter()
                .map(|i| render_value_rust(i, input_names))
                .collect();
            format!("vec![{}]", rendered.join(", "))
        }
        ConditionValue::Map(map) => {
            let pairs: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("(\"{}\", {})", k, render_value_rust(v, input_names)))
                .collect();
            format!("HashMap::from([{}])", pairs.join(", "))
        }
    }
}

#[allow(clippy::only_used_in_recursion)]
fn render_value_ts(val: &ConditionValue, input_names: &[String]) -> String {
    match val {
        ConditionValue::Bool(b) => b.to_string(),
        ConditionValue::Int(i) => i.to_string(),
        ConditionValue::Float(f) => format!("{:?}", f),
        ConditionValue::String(s) => {
            if is_expression(s) {
                compile_ts_expression(s, input_names)
            } else {
                format!("\"{}\"", escape_string(s))
            }
        }
        ConditionValue::Null => "null".to_string(),
        ConditionValue::List(items) => {
            let rendered: Vec<_> = items
                .iter()
                .map(|i| render_value_ts(i, input_names))
                .collect();
            format!("[{}]", rendered.join(", "))
        }
        ConditionValue::Map(map) => {
            let pairs: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, render_value_ts(v, input_names)))
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }
    }
}

#[allow(clippy::only_used_in_recursion)]
fn render_value_python(val: &ConditionValue, input_names: &[String]) -> String {
    match val {
        ConditionValue::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        ConditionValue::Int(i) => i.to_string(),
        ConditionValue::Float(f) => format!("{:?}", f),
        ConditionValue::String(s) => {
            if is_expression(s) {
                CelCompiler::compile(s, Target::Python)
                    .unwrap_or_else(|_| format!("\"{}\"", escape_string(s)))
            } else {
                format!("\"{}\"", escape_string(s))
            }
        }
        ConditionValue::Null => "None".to_string(),
        ConditionValue::List(items) => {
            let rendered: Vec<_> = items
                .iter()
                .map(|i| render_value_python(i, input_names))
                .collect();
            format!("[{}]", rendered.join(", "))
        }
        ConditionValue::Map(map) => {
            let pairs: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, render_value_python(v, input_names)))
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }
    }
}

fn render_value_go(val: &ConditionValue, input_names: &[String]) -> String {
    match val {
        ConditionValue::Bool(b) => b.to_string(),
        ConditionValue::Int(i) => format!("int64({})", i),
        ConditionValue::Float(f) => format!("float64({:?})", f),
        ConditionValue::String(s) => {
            if is_expression(s) {
                compile_go_expression(s, input_names)
            } else {
                format!("\"{}\"", escape_string(s))
            }
        }
        ConditionValue::Null => "nil".to_string(),
        ConditionValue::List(items) => {
            let rendered: Vec<_> = items
                .iter()
                .map(|i| render_value_go(i, input_names))
                .collect();
            format!("[]interface{{}}{{{}}}", rendered.join(", "))
        }
        ConditionValue::Map(map) => {
            let pairs: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, render_value_go(v, input_names)))
                .collect();
            format!("map[string]interface{{}}{{{}}}", pairs.join(", "))
        }
    }
}

fn render_value_java(val: &ConditionValue, input_names: &[String]) -> String {
    match val {
        ConditionValue::Bool(b) => b.to_string(),
        ConditionValue::Int(i) => format!("{}L", i),
        ConditionValue::Float(f) => format!("{:?}", f),
        ConditionValue::String(s) => {
            if is_expression(s) {
                compile_java_expression(s, input_names)
            } else {
                format!("\"{}\"", escape_string(s))
            }
        }
        ConditionValue::Null => "null".to_string(),
        ConditionValue::List(items) => {
            let rendered: Vec<_> = items
                .iter()
                .map(|i| render_value_java(i, input_names))
                .collect();
            format!("Arrays.asList({})", rendered.join(", "))
        }
        ConditionValue::Map(map) => {
            let pairs: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("entry(\"{}\", {})", k, render_value_java(v, input_names)))
                .collect();
            format!("Map.ofEntries({})", pairs.join(", "))
        }
    }
}

fn render_value_csharp(val: &ConditionValue, input_names: &[String]) -> String {
    match val {
        ConditionValue::Bool(b) => b.to_string(),
        ConditionValue::Int(i) => format!("{}L", i),
        ConditionValue::Float(f) => format!("{:?}", f),
        ConditionValue::String(s) => {
            if is_expression(s) {
                compile_csharp_expression(s, input_names)
            } else {
                format!("\"{}\"", escape_string(s))
            }
        }
        ConditionValue::Null => "null".to_string(),
        ConditionValue::List(items) => {
            let rendered: Vec<_> = items
                .iter()
                .map(|i| render_value_csharp(i, input_names))
                .collect();
            format!("new List<object> {{ {} }}", rendered.join(", "))
        }
        ConditionValue::Map(map) => {
            let pairs: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("{{ \"{}\", {} }}", k, render_value_csharp(v, input_names)))
                .collect();
            format!("new Dictionary<string, object> {{ {} }}", pairs.join(", "))
        }
    }
}

// ============================================================================
// Expression and pattern helpers
// ============================================================================

fn is_expression(s: &str) -> bool {
    let has_operator = s.contains(" + ")
        || s.contains(" - ")
        || s.contains(" * ")
        || s.contains(" / ")
        || s.contains(" % ")
        || s.contains(" == ")
        || s.contains(" != ")
        || s.contains(" < ")
        || s.contains(" > ")
        || s.contains(" <= ")
        || s.contains(" >= ")
        || s.contains(" && ")
        || s.contains(" || ")
        || s.contains(" ? ");

    if has_operator {
        return true;
    }

    // Check if it looks like a variable reference
    if !s.contains(' ')
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        && (s.contains('_') || s.contains('.'))
    {
        return true;
    }

    false
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn compile_ts_condition(cel: &str, input_names: &[String]) -> String {
    let mut result =
        CelCompiler::compile(cel, Target::TypeScript).unwrap_or_else(|_| "true".into());
    // TypeScript uses camelCase local variables (destructured from input)
    for name in input_names {
        if name.contains('_') {
            let camel = to_camel_case(name);
            // Replace whole words only (use word boundary logic)
            result = replace_var_name(&result, name, &camel);
        }
    }
    result
}

fn compile_csharp_condition(cel: &str, input_names: &[String]) -> String {
    let mut result = CelCompiler::compile(cel, Target::CSharp).unwrap_or_else(|_| "true".into());
    // C# uses camelCase local variables (extracted from input)
    for name in input_names {
        if name.contains('_') {
            let camel = to_camel_case(name);
            result = replace_var_name(&result, name, &camel);
        }
    }
    result
}

fn compile_go_condition(cel: &str, input_names: &[String]) -> String {
    let mut result = CelCompiler::compile(cel, Target::Go).unwrap_or_else(|_| "true".into());
    // Go uses input.FieldName pattern
    for name in input_names {
        let pascal = to_pascal_case(name);
        result = replace_var_name(&result, name, &format!("input.{}", pascal));
    }
    result
}

fn compile_java_condition(cel: &str, input_names: &[String]) -> String {
    let mut result = CelCompiler::compile(cel, Target::Java).unwrap_or_else(|_| "true".into());
    // Java uses input.fieldName pattern
    for name in input_names {
        let camel = to_camel_case(name);
        result = replace_var_name(&result, name, &format!("input.{}", camel));
    }
    result
}

/// Replace variable name with word boundary awareness
/// This prevents replacing "member_tier" inside "non_member_tier"
fn replace_var_name(source: &str, from: &str, to: &str) -> String {
    let mut result = String::new();
    let mut remaining = source;

    while let Some(pos) = remaining.find(from) {
        // Check character before match
        let before_ok = if pos == 0 {
            true
        } else {
            let before = remaining.as_bytes()[pos - 1];
            !before.is_ascii_alphanumeric() && before != b'_'
        };

        // Check character after match
        let after_pos = pos + from.len();
        let after_ok = if after_pos >= remaining.len() {
            true
        } else {
            let after = remaining.as_bytes()[after_pos];
            !after.is_ascii_alphanumeric() && after != b'_'
        };

        if before_ok && after_ok {
            result.push_str(&remaining[..pos]);
            result.push_str(to);
            remaining = &remaining[after_pos..];
        } else {
            // Not a word boundary match, keep original
            result.push_str(&remaining[..pos + from.len()]);
            remaining = &remaining[after_pos..];
        }
    }
    result.push_str(remaining);
    result
}

fn compile_ts_expression(expr: &str, input_names: &[String]) -> String {
    let mut result =
        CelCompiler::compile(expr, Target::TypeScript).unwrap_or_else(|_| expr.to_string());
    // TypeScript uses camelCase
    for name in input_names {
        if name.contains('_') {
            let camel = to_camel_case(name);
            result = replace_var_name(&result, name, &camel);
        }
    }
    result
}

fn compile_go_expression(expr: &str, input_names: &[String]) -> String {
    let mut result = CelCompiler::compile(expr, Target::Go).unwrap_or_else(|_| expr.to_string());
    for name in input_names {
        let pascal = to_pascal_case(name);
        result = replace_var_name(&result, name, &format!("input.{}", pascal));
    }
    result
}

fn compile_java_expression(expr: &str, input_names: &[String]) -> String {
    let mut result = CelCompiler::compile(expr, Target::Java).unwrap_or_else(|_| expr.to_string());
    for name in input_names {
        let camel = to_camel_case(name);
        result = replace_var_name(&result, name, &format!("input.{}", camel));
    }
    result
}

fn compile_csharp_expression(expr: &str, input_names: &[String]) -> String {
    let mut result =
        CelCompiler::compile(expr, Target::CSharp).unwrap_or_else(|_| expr.to_string());
    // C# uses camelCase local variables (extracted from input)
    for name in input_names {
        if name.contains('_') {
            let camel = to_camel_case(name);
            result = replace_var_name(&result, name, &camel);
        }
    }
    result
}

fn generate_rust_pattern(rule: &Rule, inputs: &[Variable]) -> String {
    let conditions = rule.conditions.as_ref();

    if inputs.len() == 1 {
        conditions
            .and_then(|c| c.first())
            .map(|c| render_pattern_value_rust(&c.value))
            .unwrap_or_else(|| "_".into())
    } else {
        let patterns: Vec<String> = inputs
            .iter()
            .map(|input| {
                conditions
                    .and_then(|c| c.iter().find(|cond| cond.var == input.name))
                    .map(|c| render_pattern_value_rust(&c.value))
                    .unwrap_or_else(|| "_".into())
            })
            .collect();
        format!("({})", patterns.join(", "))
    }
}

fn generate_python_pattern(rule: &Rule, inputs: &[Variable]) -> String {
    let conditions = rule.conditions.as_ref();

    if inputs.len() == 1 {
        conditions
            .and_then(|c| c.first())
            .map(|c| render_pattern_value_python(&c.value))
            .unwrap_or_else(|| "_".into())
    } else {
        let patterns: Vec<String> = inputs
            .iter()
            .map(|input| {
                conditions
                    .and_then(|c| c.iter().find(|cond| cond.var == input.name))
                    .map(|c| render_pattern_value_python(&c.value))
                    .unwrap_or_else(|| "_".into())
            })
            .collect();
        format!("({})", patterns.join(", "))
    }
}

fn render_pattern_value_rust(val: &ConditionValue) -> String {
    match val {
        ConditionValue::Bool(b) => b.to_string(),
        ConditionValue::Int(i) => i.to_string(),
        ConditionValue::Float(f) => format!("{:?}", f),
        ConditionValue::String(s) => format!("\"{}\"", escape_string(s)),
        ConditionValue::Null => "None".to_string(),
        _ => "_".to_string(),
    }
}

fn render_pattern_value_python(val: &ConditionValue) -> String {
    match val {
        ConditionValue::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        ConditionValue::Int(i) => i.to_string(),
        ConditionValue::Float(f) => format!("{:?}", f),
        ConditionValue::String(s) => format!("\"{}\"", escape_string(s)),
        ConditionValue::Null => "None".to_string(),
        _ => "_".to_string(),
    }
}

// ============================================================================
// Orchestrator context
// ============================================================================

/// Context for orchestrator template rendering
#[derive(Debug, Clone, Serialize)]
pub struct OrchestratorContext {
    /// Orchestrator ID
    pub id: String,
    /// PascalCase version
    pub id_pascal: String,
    /// camelCase version
    pub id_camel: String,
    /// Whether to include provenance header
    pub provenance: bool,
    /// Generation timestamp
    pub generated_at: String,
    /// Inputs
    pub inputs: Vec<InputView>,
    /// Outputs
    pub outputs: Vec<OutputView>,
    /// Steps
    pub steps: Vec<StepView>,
    /// Target language
    pub target: String,
    // Namespace fields for scoping
    /// C# namespace (e.g., "Company.Rules.Auth")
    pub namespace: Option<String>,
    /// Java package (e.g., "com.company.rules.auth")
    pub package: Option<String>,
    /// Go module path (e.g., "github.com/company/rules/auth")
    pub module_path: Option<String>,
    /// Python/Rust/TypeScript module (e.g., "company.rules.auth")
    pub module: Option<String>,
}

/// View of an orchestrator step
#[derive(Debug, Clone, Serialize)]
pub struct StepView {
    /// Step ID
    pub id: String,
    /// Step type
    pub step_type: String,
    /// Spec ID (for Call steps)
    pub spec_id: Option<String>,
    /// Is this a gate step?
    pub is_gate: bool,
    /// Is this a call step?
    pub is_call: bool,
    /// Is this a compute step?
    pub is_compute: bool,
    /// Is this a branch step?
    pub is_branch: bool,
    /// Is this a loop step?
    pub is_loop: bool,
    /// Condition (for gate/branch) - raw expression
    pub condition: Option<String>,
    /// Compiled condition per language (for gates/branches)
    pub condition_rust: Option<String>,
    pub condition_ts: Option<String>,
    pub condition_py: Option<String>,
    pub condition_go: Option<String>,
    pub condition_java: Option<String>,
    pub condition_csharp: Option<String>,
    /// Input mappings for Call steps: spec_input_name -> compiled expression
    pub input_mappings: Vec<InputMapping>,
    /// Output mappings for Call steps: local_name -> spec_output_name
    pub output_mappings: Vec<OutputMapping>,
}

/// Input mapping for a Call step
#[derive(Debug, Clone, Serialize)]
pub struct InputMapping {
    /// Spec input name (PascalCase for C#/Java/Go, camelCase for TS, snake_case for Python/Rust)
    pub spec_input_name: String,
    /// Compiled expression in Rust
    pub expr_rust: String,
    /// Compiled expression in TypeScript
    pub expr_ts: String,
    /// Compiled expression in Python
    pub expr_py: String,
    /// Compiled expression in Go
    pub expr_go: String,
    /// Compiled expression in Java
    pub expr_java: String,
    /// Compiled expression in C#
    pub expr_csharp: String,
}

/// Output mapping for a Call step
#[derive(Debug, Clone, Serialize)]
pub struct OutputMapping {
    /// Local variable name
    pub local_name: String,
    /// Spec output name
    pub spec_output_name: String,
}

impl OrchestratorContext {
    pub fn from_orchestrator(
        orch: &crate::orchestrate::Orchestrator,
        _specs: &HashMap<String, Spec>,
        target: Target,
        provenance: bool,
    ) -> Self {
        use crate::orchestrate::ChainStep;

        let inputs: Vec<InputView> = orch.inputs.iter().map(InputView::from_orch_var).collect();
        let outputs: Vec<OutputView> = orch.outputs.iter().map(OutputView::from_orch_var).collect();
        let input_names: Vec<String> = inputs.iter().map(|i| i.name.clone()).collect();

        let steps: Vec<StepView> = orch
            .chain
            .iter()
            .map(|s| {
                match s {
                    ChainStep::Call(call) => {
                        // Compile input mappings
                        let input_mappings: Vec<InputMapping> = call
                            .inputs
                            .iter()
                            .map(|(spec_input, expr)| {
                                let _spec_input_pascal = to_pascal_case(spec_input);
                                let _spec_input_camel = to_camel_case(spec_input);
                                InputMapping {
                                    spec_input_name: spec_input.clone(),
                                    expr_rust: compile_orch_expr_rust(expr, &input_names),
                                    expr_ts: compile_orch_expr_ts(expr, &input_names),
                                    expr_py: compile_orch_expr_py(expr, &input_names),
                                    expr_go: compile_orch_expr_go(expr, &input_names),
                                    expr_java: compile_orch_expr_java(expr, &input_names),
                                    expr_csharp: compile_orch_expr_csharp(expr, &input_names),
                                }
                            })
                            .collect();

                        // Compile output mappings
                        let output_mappings: Vec<OutputMapping> = call
                            .outputs
                            .iter()
                            .map(|(local_name, spec_output)| OutputMapping {
                                local_name: local_name.clone(),
                                spec_output_name: spec_output.clone(),
                            })
                            .collect();

                        // Compile condition if present
                        let (
                            condition_rust,
                            condition_ts,
                            condition_py,
                            condition_go,
                            condition_java,
                            condition_csharp,
                        ) = if let Some(cond) = &call.condition {
                            (
                                Some(compile_orch_expr_rust(cond, &input_names)),
                                Some(compile_orch_expr_ts(cond, &input_names)),
                                Some(compile_orch_expr_py(cond, &input_names)),
                                Some(compile_orch_expr_go(cond, &input_names)),
                                Some(compile_orch_expr_java(cond, &input_names)),
                                Some(compile_orch_expr_csharp(cond, &input_names)),
                            )
                        } else {
                            (None, None, None, None, None, None)
                        };

                        StepView {
                            id: call.id.clone(),
                            step_type: "Call".to_string(),
                            spec_id: Some(call.spec.clone()),
                            is_gate: false,
                            is_call: true,
                            is_compute: false,
                            is_branch: false,
                            is_loop: false,
                            condition: call.condition.clone(),
                            condition_rust,
                            condition_ts,
                            condition_py,
                            condition_go,
                            condition_java,
                            condition_csharp,
                            input_mappings,
                            output_mappings,
                        }
                    }
                    ChainStep::Gate(gate) => {
                        let cond = gate.condition.clone();
                        StepView {
                            id: gate.id.clone(),
                            step_type: "Gate".to_string(),
                            spec_id: None,
                            is_gate: true,
                            is_call: false,
                            is_compute: false,
                            is_branch: false,
                            is_loop: false,
                            condition: Some(cond.clone()),
                            condition_rust: Some(compile_orch_expr_rust(&cond, &input_names)),
                            condition_ts: Some(compile_orch_expr_ts(&cond, &input_names)),
                            condition_py: Some(compile_orch_expr_py(&cond, &input_names)),
                            condition_go: Some(compile_orch_expr_go(&cond, &input_names)),
                            condition_java: Some(compile_orch_expr_java(&cond, &input_names)),
                            condition_csharp: Some(compile_orch_expr_csharp(&cond, &input_names)),
                            input_mappings: Vec::new(),
                            output_mappings: Vec::new(),
                        }
                    }
                    ChainStep::Compute(compute) => StepView {
                        id: compute.id.clone(),
                        step_type: "Compute".to_string(),
                        spec_id: None,
                        is_gate: false,
                        is_call: false,
                        is_compute: true,
                        is_branch: false,
                        is_loop: false,
                        condition: None,
                        condition_rust: None,
                        condition_ts: None,
                        condition_py: None,
                        condition_go: None,
                        condition_java: None,
                        condition_csharp: None,
                        input_mappings: Vec::new(),
                        output_mappings: Vec::new(),
                    },
                    ChainStep::Branch(branch) => {
                        let cond = branch.on.clone();
                        StepView {
                            id: branch.id.clone(),
                            step_type: "Branch".to_string(),
                            spec_id: None,
                            is_gate: false,
                            is_call: false,
                            is_compute: false,
                            is_branch: true,
                            is_loop: false,
                            condition: Some(cond.clone()),
                            condition_rust: Some(compile_orch_expr_rust(&cond, &input_names)),
                            condition_ts: Some(compile_orch_expr_ts(&cond, &input_names)),
                            condition_py: Some(compile_orch_expr_py(&cond, &input_names)),
                            condition_go: Some(compile_orch_expr_go(&cond, &input_names)),
                            condition_java: Some(compile_orch_expr_java(&cond, &input_names)),
                            condition_csharp: Some(compile_orch_expr_csharp(&cond, &input_names)),
                            input_mappings: Vec::new(),
                            output_mappings: Vec::new(),
                        }
                    }
                    ChainStep::Loop(loop_step) => {
                        let cond = loop_step.until.clone();
                        StepView {
                            id: loop_step.id.clone(),
                            step_type: "Loop".to_string(),
                            spec_id: None,
                            is_gate: false,
                            is_call: false,
                            is_compute: false,
                            is_branch: false,
                            is_loop: true,
                            condition: cond.clone(),
                            condition_rust: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_rust(c, &input_names)),
                            condition_ts: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_ts(c, &input_names)),
                            condition_py: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_py(c, &input_names)),
                            condition_go: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_go(c, &input_names)),
                            condition_java: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_java(c, &input_names)),
                            condition_csharp: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_csharp(c, &input_names)),
                            input_mappings: Vec::new(),
                            output_mappings: Vec::new(),
                        }
                    }
                    ChainStep::ForEach(foreach) => StepView {
                        id: foreach.id.clone(),
                        step_type: "ForEach".to_string(),
                        spec_id: None,
                        is_gate: false,
                        is_call: false,
                        is_compute: false,
                        is_branch: false,
                        is_loop: true,
                        condition: None,
                        condition_rust: None,
                        condition_ts: None,
                        condition_py: None,
                        condition_go: None,
                        condition_java: None,
                        condition_csharp: None,
                        input_mappings: Vec::new(),
                        output_mappings: Vec::new(),
                    },
                    ChainStep::Parallel(par) => StepView {
                        id: par.id.clone(),
                        step_type: "Parallel".to_string(),
                        spec_id: None,
                        is_gate: false,
                        is_call: false,
                        is_compute: false,
                        is_branch: false,
                        is_loop: false,
                        condition: None,
                        condition_rust: None,
                        condition_ts: None,
                        condition_py: None,
                        condition_go: None,
                        condition_java: None,
                        condition_csharp: None,
                        input_mappings: Vec::new(),
                        output_mappings: Vec::new(),
                    },
                    ChainStep::Return(ret) => {
                        let cond = ret.condition.clone();
                        StepView {
                            id: "return".to_string(),
                            step_type: "Return".to_string(),
                            spec_id: None,
                            is_gate: false,
                            is_call: false,
                            is_compute: false,
                            is_branch: false,
                            is_loop: false,
                            condition: cond.clone(),
                            condition_rust: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_rust(c, &input_names)),
                            condition_ts: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_ts(c, &input_names)),
                            condition_py: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_py(c, &input_names)),
                            condition_go: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_go(c, &input_names)),
                            condition_java: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_java(c, &input_names)),
                            condition_csharp: cond
                                .as_ref()
                                .map(|c| compile_orch_expr_csharp(c, &input_names)),
                            input_mappings: Vec::new(),
                            output_mappings: Vec::new(),
                        }
                    }
                    ChainStep::Set(set) => StepView {
                        id: format!("set_{}", set.name),
                        step_type: "Set".to_string(),
                        spec_id: None,
                        is_gate: false,
                        is_call: false,
                        is_compute: true,
                        is_branch: false,
                        is_loop: false,
                        condition: None,
                        condition_rust: None,
                        condition_ts: None,
                        condition_py: None,
                        condition_go: None,
                        condition_java: None,
                        condition_csharp: None,
                        input_mappings: Vec::new(),
                        output_mappings: Vec::new(),
                    },
                    ChainStep::Try(try_step) => StepView {
                        id: try_step.id.clone(),
                        step_type: "Try".to_string(),
                        spec_id: None,
                        is_gate: false,
                        is_call: false,
                        is_compute: false,
                        is_branch: false,
                        is_loop: false,
                        condition: None,
                        condition_rust: None,
                        condition_ts: None,
                        condition_py: None,
                        condition_go: None,
                        condition_java: None,
                        condition_csharp: None,
                        input_mappings: Vec::new(),
                        output_mappings: Vec::new(),
                    },
                    ChainStep::Dynamic(dyn_step) => {
                        // Similar to Call step but with dynamic spec selection
                        let input_mappings: Vec<InputMapping> = dyn_step
                            .inputs
                            .iter()
                            .map(|(spec_input, expr)| InputMapping {
                                spec_input_name: spec_input.clone(),
                                expr_rust: compile_orch_expr_rust(expr, &input_names),
                                expr_ts: compile_orch_expr_ts(expr, &input_names),
                                expr_py: compile_orch_expr_py(expr, &input_names),
                                expr_go: compile_orch_expr_go(expr, &input_names),
                                expr_java: compile_orch_expr_java(expr, &input_names),
                                expr_csharp: compile_orch_expr_csharp(expr, &input_names),
                            })
                            .collect();
                        StepView {
                            id: dyn_step.id.clone(),
                            step_type: "Dynamic".to_string(),
                            spec_id: Some(dyn_step.spec.clone()),
                            is_gate: false,
                            is_call: true,
                            is_compute: false,
                            is_branch: false,
                            is_loop: false,
                            condition: None,
                            condition_rust: None,
                            condition_ts: None,
                            condition_py: None,
                            condition_go: None,
                            condition_java: None,
                            condition_csharp: None,
                            input_mappings,
                            output_mappings: Vec::new(),
                        }
                    }
                    ChainStep::Await(await_step) => StepView {
                        id: await_step.id.clone(),
                        step_type: "Await".to_string(),
                        spec_id: None,
                        is_gate: false,
                        is_call: false,
                        is_compute: false,
                        is_branch: false,
                        is_loop: false,
                        condition: None,
                        condition_rust: None,
                        condition_ts: None,
                        condition_py: None,
                        condition_go: None,
                        condition_java: None,
                        condition_csharp: None,
                        input_mappings: Vec::new(),
                        output_mappings: Vec::new(),
                    },
                    ChainStep::Emit(emit) => StepView {
                        id: format!("emit_{}", emit.event),
                        step_type: "Emit".to_string(),
                        spec_id: None,
                        is_gate: false,
                        is_call: false,
                        is_compute: false,
                        is_branch: false,
                        is_loop: false,
                        condition: None,
                        condition_rust: None,
                        condition_ts: None,
                        condition_py: None,
                        condition_go: None,
                        condition_java: None,
                        condition_csharp: None,
                        input_mappings: Vec::new(),
                        output_mappings: Vec::new(),
                    },
                }
            })
            .collect();

        // Extract namespace from orchestrator's scoping config if present
        let (namespace, package, module_path, module) = extract_orch_namespace_fields(orch, target);

        Self {
            id: orch.id.clone(),
            id_pascal: to_pascal_case(&orch.id),
            id_camel: to_camel_case(&orch.id),
            provenance,
            generated_at: Utc::now().to_rfc3339(),
            inputs,
            outputs,
            steps,
            target: format!("{:?}", target),
            namespace,
            package,
            module_path,
            module,
        }
    }
}

/// Extract namespace fields from orchestrator scoping config based on target language
fn extract_orch_namespace_fields(
    orch: &crate::orchestrate::Orchestrator,
    target: Target,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let scoping = match &orch.scoping {
        Some(s) => s,
        None => return (None, None, None, None),
    };

    match target {
        Target::CSharp => {
            let ns = scoping.languages.csharp.as_ref().map(|n| n.render());
            (ns, None, None, None)
        }
        Target::Java => {
            let pkg = scoping.languages.java.as_ref().map(|p| p.render());
            (None, pkg, None, None)
        }
        Target::Go => {
            let (pkg, module) = scoping
                .languages
                .go
                .as_ref()
                .map(|g| (Some(g.render()), g.module_path.clone()))
                .unwrap_or((None, None));
            (None, pkg, module, None)
        }
        Target::Python => {
            let module = scoping.languages.python.as_ref().map(|p| p.render());
            (None, None, None, module)
        }
        Target::Rust => {
            let module = scoping.languages.rust.as_ref().map(|r| r.render());
            (None, None, None, module)
        }
        Target::TypeScript => {
            let module = scoping.languages.typescript.as_ref().map(|t| t.render());
            (None, None, None, module)
        }
    }
}

impl InputView {
    fn from_orch_var(var: &crate::orchestrate::OrchestratorInput) -> Self {
        let var_type = format_var_type(&var.var_type);
        Self {
            name: var.name.clone(),
            name_pascal: to_pascal_case(&var.name),
            name_camel: to_camel_case(&var.name),
            var_type: var_type.clone(),
            rust_type: map_type_rust(&var.var_type),
            ts_type: map_type_ts(&var.var_type),
            py_type: map_type_python(&var.var_type),
            go_type: map_type_go(&var.var_type),
            java_type: map_type_java(&var.var_type),
            csharp_type: map_type_csharp(&var.var_type),
        }
    }
}

impl OutputView {
    fn from_orch_var(var: &crate::orchestrate::OrchestratorOutput) -> Self {
        let var_type = format_var_type(&var.var_type);
        Self {
            name: var.name.clone(),
            name_pascal: to_pascal_case(&var.name),
            name_camel: to_camel_case(&var.name),
            var_type: var_type.clone(),
            rust_type: map_type_rust(&var.var_type),
            ts_type: map_type_ts(&var.var_type),
            py_type: map_type_python(&var.var_type),
            go_type: map_type_go(&var.var_type),
            java_type: map_type_java(&var.var_type),
            csharp_type: map_type_csharp(&var.var_type),
        }
    }
}

// ============================================================================
// Orchestrator expression compilation helpers
// ============================================================================

/// Compile an orchestrator expression to Rust syntax
/// Expressions can reference:
/// - Input fields: "field_name" -> "input.field_name"
/// - Context fields: "step_id.field" -> "ctx.step_id[\"field\"]"
fn compile_orch_expr_rust(expr: &str, input_names: &[String]) -> String {
    if expr.contains('.') {
        // Context reference: "check_access.level" -> "ctx.check_access[\"level\"]"
        let parts: Vec<&str> = expr.split('.').collect();
        let mut result = format!("ctx.{}", parts[0]);
        for part in &parts[1..] {
            result = format!("{}[\"{}\"]", result, part);
        }
        result
    } else {
        // Input reference: check if it's a known input
        if input_names.contains(&expr.to_string()) {
            format!("input.{}", expr)
        } else {
            // Literal or unknown - return as-is (might be a constant)
            expr.to_string()
        }
    }
}

/// Compile an orchestrator expression to TypeScript syntax
fn compile_orch_expr_ts(expr: &str, input_names: &[String]) -> String {
    if expr.contains('.') {
        // Context reference: "check_access.level" -> "ctx.check_access?.level"
        let parts: Vec<&str> = expr.split('.').collect();
        format!("ctx.{}", parts.join("?."))
    } else if input_names.contains(&expr.to_string()) {
        format!("input.{}", expr)
    } else {
        expr.to_string()
    }
}

/// Compile an orchestrator expression to Python syntax
fn compile_orch_expr_py(expr: &str, input_names: &[String]) -> String {
    if expr.contains('.') {
        // Context reference: "check_access.level" -> "ctx.check_access.get('level')"
        let parts: Vec<&str> = expr.split('.').collect();
        let mut result = format!("ctx.{}", parts[0]);
        for part in &parts[1..] {
            result = format!("{}.get('{}')", result, part);
        }
        result
    } else if input_names.contains(&expr.to_string()) {
        format!("input.{}", expr)
    } else {
        expr.to_string()
    }
}

/// Compile an orchestrator expression to Go syntax
fn compile_orch_expr_go(expr: &str, input_names: &[String]) -> String {
    if expr.contains('.') {
        // Context reference: "check_access.level" -> "ctx.CheckAccess[\"level\"]"
        let parts: Vec<&str> = expr.split('.').collect();
        let mut result = format!("ctx.{}", to_pascal_case(parts[0]));
        for part in &parts[1..] {
            result = format!("{}[\"{}\"]", result, part);
        }
        result
    } else if input_names.contains(&expr.to_string()) {
        format!("input.{}", to_pascal_case(expr))
    } else {
        expr.to_string()
    }
}

/// Compile an orchestrator expression to Java syntax
fn compile_orch_expr_java(expr: &str, input_names: &[String]) -> String {
    if expr.contains('.') {
        // Context reference: "check_access.level" -> "ctx.checkAccess.get(\"level\")"
        let parts: Vec<&str> = expr.split('.').collect();
        let mut result = format!("ctx.{}", to_camel_case(parts[0]));
        for part in &parts[1..] {
            result = format!("{}.get(\"{}\")", result, part);
        }
        result
    } else if input_names.contains(&expr.to_string()) {
        format!("input.{}", to_camel_case(expr))
    } else {
        expr.to_string()
    }
}

/// Compile an orchestrator expression to C# syntax
fn compile_orch_expr_csharp(expr: &str, input_names: &[String]) -> String {
    if expr.contains('.') {
        // Context reference: "check_access.level" -> "ctx.check_access[\"level\"]"
        let parts: Vec<&str> = expr.split('.').collect();
        let mut result = format!("ctx.{}", parts[0]);
        for part in &parts[1..] {
            result = format!("{}[\"{}\"]", result, part);
        }
        result
    } else if input_names.contains(&expr.to_string()) {
        format!("input.{}", to_pascal_case(expr))
    } else {
        expr.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(to_pascal_case("foo"), "Foo");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("hello_world"), "helloWorld");
        assert_eq!(to_camel_case("foo"), "foo");
    }

    #[test]
    fn test_is_expression() {
        assert!(is_expression("a + b"));
        assert!(is_expression("x == 5"));
        assert!(is_expression("foo_bar"));
        assert!(!is_expression("hello"));
        assert!(!is_expression("OK"));
    }

    #[test]
    fn test_replace_var_name() {
        // Simple replacement
        assert_eq!(
            replace_var_name("member_tier == \"gold\"", "member_tier", "memberTier"),
            "memberTier == \"gold\""
        );

        // Multiple occurrences
        assert_eq!(
            replace_var_name(
                "member_tier == \"gold\" && member_tier != \"\"",
                "member_tier",
                "memberTier"
            ),
            "memberTier == \"gold\" && memberTier != \"\""
        );

        // Don't replace partial matches
        assert_eq!(
            replace_var_name("non_member_tier == \"gold\"", "member_tier", "memberTier"),
            "non_member_tier == \"gold\""
        );

        // Don't replace when suffix matches
        assert_eq!(
            replace_var_name("some_member_tier_value == 1", "member_tier", "memberTier"),
            "some_member_tier_value == 1"
        );

        // Replace at boundaries (parentheses, operators)
        assert_eq!(
            replace_var_name("(member_tier)", "member_tier", "memberTier"),
            "(memberTier)"
        );
    }

    #[test]
    fn test_compile_csharp_expression() {
        // Test that snake_case variables are converted to camelCase in output expressions
        let input_names = vec!["weight_kg".to_string(), "member_tier".to_string()];
        let result = compile_csharp_expression("weight_kg * 25.0 + 50.0", &input_names);
        assert!(
            result.contains("weightKg"),
            "Expected weightKg in: {}",
            result
        );
        assert!(
            !result.contains("weight_kg"),
            "Should not contain weight_kg in: {}",
            result
        );
    }
}
