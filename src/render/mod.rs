//! Code rendering — generate source code from specs
//!
//! Renders decision tables to multiple target languages.
//! Same spec produces equivalent implementations in each language.
//!
//! This module uses generated code from specs/ for dogfooding:
//! - `type_mapping` for type conversions
//! - `bool_literal` for boolean literals
//! - `null_literal` for null/none literals

mod csharp;
mod go;
mod java;
mod python;
mod rust;
pub mod scoping;
mod typescript;

pub use scoping::{
    CSharpNamespace, GoPackage, GoPackageName, JavaPackage, LanguageScopingTyped, NamespaceError,
    PythonModule, ResolvedNamespace, RustModule, RustVisibility, ScopingConfig, TypeScriptModule,
};

use crate::cel::Target;
use crate::format::format_code;
use crate::spec::*;

pub use crate::cel::Target as RenderTarget;

/// Render spec to target language using templates
pub fn render(spec: &Spec, target: Target) -> String {
    // Try template-based rendering first
    match crate::templates::render_spec(spec, target, true) {
        Ok(code) => code,
        Err(_) => {
            // Fall back to legacy genco renderers if templates fail
            let code = Renderer::new(target).render(spec);
            // Apply formatting (silently fall back to unformatted if formatter fails)
            format_code(&code, target).unwrap_or(code)
        }
    }
}

/// Code renderer
pub struct Renderer {
    target: Target,
    config: RenderConfig,
}

/// Render configuration
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Include comments
    pub comments: bool,
    /// Include provenance header
    pub provenance: bool,
    /// Indentation
    pub indent: String,
    /// Resolved namespace for the target language
    pub namespace: Option<ResolvedNamespace>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            comments: true,
            provenance: true,
            indent: "    ".into(),
            namespace: None,
        }
    }
}

impl Renderer {
    pub fn new(target: Target) -> Self {
        Self {
            target,
            config: RenderConfig::default(),
        }
    }

    pub fn with_config(target: Target, config: RenderConfig) -> Self {
        Self { target, config }
    }

    /// Render spec to code
    pub fn render(&self, spec: &Spec) -> String {
        // Resolve namespace from spec's scoping configuration
        let config = self.resolve_config(spec);

        match self.target {
            Target::Rust => rust::render(spec, &config),
            Target::TypeScript => typescript::render(spec, &config),
            Target::Python => python::render(spec, &config),
            Target::CSharp => csharp::render(spec, &config),
            Target::Java => java::render(spec, &config),
            Target::Go => go::render(spec, &config),
        }
    }

    /// Resolve render config with namespace from spec
    fn resolve_config(&self, spec: &Spec) -> RenderConfig {
        let mut config = self.config.clone();

        // If spec has scoping configuration, resolve namespace for target
        if let Some(ref scoping) = spec.scoping {
            config.namespace = scoping.for_target(self.target);
        }

        config
    }
}

// Re-export from shared util module
pub(crate) use crate::util::{to_camel_case, to_pascal_case};

/// Variable translation mode for different languages
#[derive(Debug, Clone, Copy)]
pub(crate) enum VarTranslation {
    /// Convert snake_case to camelCase (TypeScript, C#)
    CamelCase,
    /// Convert snake_case to input.PascalCase (Go)
    InputPascal,
    /// Convert snake_case to input.camelCase (Java)
    InputCamel,
}

/// Translate variable names in an expression according to the target language convention
/// Consolidates the per-language translate_vars_* functions (FM-4 mitigation)
pub(crate) fn translate_vars(expr: &str, input_names: &[String], mode: VarTranslation) -> String {
    let mut result = expr.to_string();
    for name in input_names {
        let replacement = match mode {
            VarTranslation::CamelCase => {
                if name.contains('_') {
                    to_camel_case(name)
                } else {
                    continue; // No transformation needed
                }
            }
            VarTranslation::InputPascal => {
                format!("input.{}", to_pascal_case(name))
            }
            VarTranslation::InputCamel => {
                format!("input.{}", to_camel_case(name))
            }
        };
        result = result.replace(name.as_str(), &replacement);
    }
    result
}

/// Check if a string looks like a CEL expression (contains operators/variables)
/// vs a simple literal string
pub(crate) fn is_expression(s: &str) -> bool {
    // Check for arithmetic/comparison operators that indicate an expression
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

    // Check if it looks like a variable reference (identifier pattern)
    if !s.contains(' ')
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        && (s.contains('_') || s.contains('.'))
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> Spec {
        Spec::from_yaml(
            r#"
id: check_status
inputs:
  - name: rate_exceeded
    type: bool
  - name: locked
    type: bool
outputs:
  - name: status
    type: int
rules:
  - id: R1
    conditions:
      - var: rate_exceeded
        value: true
    then: 429
  - id: R2
    conditions:
      - var: rate_exceeded
        value: false
      - var: locked
        value: true
    then: 423
  - id: R3
    conditions:
      - var: rate_exceeded
        value: false
      - var: locked
        value: false
    then: 200
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_render_rust() {
        let spec = sample_spec();
        let code = render(&spec, Target::Rust);

        assert!(code.contains("pub fn check_status"));
        assert!(code.contains("match"));
        assert!(code.contains("429"));
        assert!(code.contains("423"));
        assert!(code.contains("200"));
    }

    #[test]
    fn test_render_typescript() {
        let spec = sample_spec();
        let code = render(&spec, Target::TypeScript);

        assert!(code.contains("export function checkStatus"));
        assert!(code.contains("CheckStatusInput"));
        assert!(code.contains("429"));
    }

    #[test]
    fn test_render_python() {
        let spec = sample_spec();
        let code = render(&spec, Target::Python);

        assert!(code.contains("def check_status"));
        assert!(code.contains("match"));
        assert!(code.contains("429"));
    }

    #[test]
    fn test_render_csharp() {
        let spec = sample_spec();
        let code = render(&spec, Target::CSharp);

        assert!(code.contains("CheckStatus"));
        assert!(code.contains("if (")); // C# uses if/else if
        assert!(code.contains("429"));
    }

    // =========================================================================
    // Behavioral tests for computed expressions
    // =========================================================================

    fn computed_spec() -> Spec {
        Spec::from_yaml(
            r#"
id: shipping_rate
inputs:
  - name: weight_kg
    type: float
  - name: zone
    type: string
outputs:
  - name: rate
    type: float
rules:
  - id: R1
    when: "zone == 'domestic'"
    then: "weight_kg * 5.0 + 7.0"
  - id: R2
    when: "zone == 'international'"
    then: "weight_kg * 20.0 + 40.0"
"#,
        )
        .unwrap()
    }

    // Rust: computed expressions should render as actual arithmetic
    #[test]
    fn rust_computed_expression_renders_as_code() {
        let spec = computed_spec();
        let code = render(&spec, Target::Rust);

        // MUST contain actual arithmetic expression, NOT a string literal
        // CEL compiler may add parentheses, so check for the components
        assert!(
            code.contains("weight_kg * 5.0") && code.contains("+ 7.0"),
            "Rust should render computed expression as code, not string. Got:\n{}",
            code
        );
        assert!(
            !code.contains("\"weight_kg * 5.0"),
            "Rust should NOT wrap computed expression in quotes. Got:\n{}",
            code
        );
    }

    // TypeScript: computed expressions should render as actual arithmetic
    #[test]
    fn typescript_computed_expression_renders_as_code() {
        let spec = computed_spec();
        let code = render(&spec, Target::TypeScript);

        // CEL compiler may add parentheses, so check for components with camelCase vars
        assert!(
            code.contains("weightKg * 5.0") && code.contains("+ 7.0"),
            "TypeScript should render computed expression as code with camelCase. Got:\n{}",
            code
        );
        assert!(
            !code.contains("\"weight_kg * 5.0"),
            "TypeScript should NOT wrap computed expression in quotes. Got:\n{}",
            code
        );
    }

    // Python: computed expressions should render as actual arithmetic
    #[test]
    fn python_computed_expression_renders_as_code() {
        let spec = computed_spec();
        let code = render(&spec, Target::Python);

        // CEL compiler may add parentheses
        assert!(
            code.contains("weight_kg * 5.0") && code.contains("+ 7.0"),
            "Python should render computed expression as code. Got:\n{}",
            code
        );
        assert!(
            !code.contains("\"weight_kg * 5.0"),
            "Python should NOT wrap computed expression in quotes. Got:\n{}",
            code
        );
    }

    // Go: computed expressions should render as actual arithmetic
    #[test]
    fn go_computed_expression_renders_as_code() {
        let spec = computed_spec();
        let code = render(&spec, Target::Go);

        // Go uses input.WeightKg, CEL compiler may add parentheses
        assert!(
            code.contains("input.WeightKg * 5.0") && code.contains("+ 7.0"),
            "Go should render computed expression as code. Got:\n{}",
            code
        );
        assert!(
            !code.contains("\"weight_kg * 5.0"),
            "Go should NOT wrap computed expression in quotes. Got:\n{}",
            code
        );
    }

    // C#: computed expressions should render as actual arithmetic
    #[test]
    fn csharp_computed_expression_renders_as_code() {
        let spec = computed_spec();
        let code = render(&spec, Target::CSharp);

        // C# uses camelCase for destructured vars, CEL compiler may add parentheses
        assert!(
            code.contains("weightKg * 5.0") && code.contains("+ 7.0"),
            "C# should render computed expression as code. Got:\n{}",
            code
        );
        assert!(
            !code.contains("\"weight_kg * 5.0"),
            "C# should NOT wrap computed expression in quotes. Got:\n{}",
            code
        );
    }

    // Java: computed expressions should render as actual arithmetic
    #[test]
    fn java_computed_expression_renders_as_code() {
        let spec = computed_spec();
        let code = render(&spec, Target::Java);

        // Java uses input.camelCase, CEL compiler may add parentheses
        assert!(
            code.contains("input.weightKg * 5.0") && code.contains("+ 7.0"),
            "Java should render computed expression as code. Got:\n{}",
            code
        );
        assert!(
            !code.contains("\"weight_kg * 5.0"),
            "Java should NOT wrap computed expression in quotes. Got:\n{}",
            code
        );
    }

    // =========================================================================
    // Behavioral tests for variable name translation in conditions
    // =========================================================================

    fn condition_spec() -> Spec {
        Spec::from_yaml(
            r#"
id: check_tier
inputs:
  - name: member_tier
    type: string
outputs:
  - name: discount
    type: float
rules:
  - id: R1
    when: "member_tier == 'gold'"
    then: 0.20
"#,
        )
        .unwrap()
    }

    // TypeScript: variable names in conditions should be camelCase
    #[test]
    fn typescript_condition_uses_camel_case() {
        let spec = condition_spec();
        let code = render(&spec, Target::TypeScript);

        assert!(
            code.contains("memberTier === \"gold\"") || code.contains("memberTier === 'gold'"),
            "TypeScript conditions should use camelCase variable names. Got:\n{}",
            code
        );
        assert!(
            !code.contains("member_tier ==="),
            "TypeScript should NOT use snake_case in conditions. Got:\n{}",
            code
        );
    }

    // Go: variable names should reference input struct
    #[test]
    fn go_condition_uses_input_struct() {
        let spec = condition_spec();
        let code = render(&spec, Target::Go);

        assert!(
            code.contains("input.MemberTier") || code.contains("MemberTier"),
            "Go conditions should reference input struct with PascalCase. Got:\n{}",
            code
        );
    }

    // C#: destructured variables should be used in conditions
    #[test]
    fn csharp_condition_uses_destructured_vars() {
        let spec = condition_spec();
        let code = render(&spec, Target::CSharp);

        // If destructured to memberTier, should use memberTier
        if code.contains("var memberTier") {
            assert!(
                code.contains("memberTier == \"gold\""),
                "C# should use destructured variable name in condition. Got:\n{}",
                code
            );
        }
    }

    // =========================================================================
    // Behavioral tests for literal vs computed outputs
    // =========================================================================

    fn literal_string_output_spec() -> Spec {
        Spec::from_yaml(
            r#"
id: get_status
inputs:
  - name: code
    type: int
outputs:
  - name: message
    type: string
rules:
  - id: R1
    when: "code == 200"
    then: "OK"
  - id: R2
    when: "code == 404"
    then: "Not Found"
"#,
        )
        .unwrap()
    }

    // Rust: literal string outputs should be quoted
    #[test]
    fn rust_literal_string_is_quoted() {
        let spec = literal_string_output_spec();
        let code = render(&spec, Target::Rust);

        assert!(
            code.contains("\"OK\"") || code.contains("\"OK\".to_string()"),
            "Rust should quote literal string outputs. Got:\n{}",
            code
        );
    }

    // TypeScript: literal string outputs should be quoted
    #[test]
    fn typescript_literal_string_is_quoted() {
        let spec = literal_string_output_spec();
        let code = render(&spec, Target::TypeScript);

        assert!(
            code.contains("\"OK\"") || code.contains("'OK'"),
            "TypeScript should quote literal string outputs. Got:\n{}",
            code
        );
    }

    // Python: should use if/else for CEL conditions, not wildcard match
    #[test]
    fn python_uses_if_else_for_cel_conditions() {
        let spec = condition_spec();
        let code = render(&spec, Target::Python);

        // Python should use if/elif for CEL conditions
        // CEL compiler may add parentheses around the condition
        let uses_if_else =
            code.contains("if (member_tier ==") || code.contains("if member_tier ==");

        assert!(
            uses_if_else,
            "Python should use if/elif for CEL conditions. Got:\n{}",
            code
        );
        // Should NOT use wildcard case patterns for CEL
        assert!(
            !code.contains("case _:") || code.contains("case _ if"),
            "Python should not use bare wildcard matches for CEL conditions. Got:\n{}",
            code
        );
    }

    // =========================================================================
    // Round-trip tests: render → parse → verify the code is syntactically valid
    // =========================================================================

    #[test]
    fn rust_rendered_code_parses_successfully() {
        use crate::parse::parse_rust;

        let spec = computed_spec();
        let code = render(&spec, Target::Rust);

        // Should parse without errors
        let result = parse_rust(&code);
        assert!(
            result.is_ok(),
            "Rendered Rust code should parse successfully. Error: {:?}\nCode:\n{}",
            result.err(),
            code
        );

        // Should find the expected function
        let ast = result.unwrap();
        assert_eq!(ast.functions.len(), 1, "Should have one function");
        assert_eq!(ast.functions[0].name, "shipping_rate");
    }

    #[test]
    fn rust_simple_spec_roundtrip() {
        use crate::parse::parse_rust;

        let spec = sample_spec();
        let code = render(&spec, Target::Rust);

        let result = parse_rust(&code);
        assert!(
            result.is_ok(),
            "Simple spec Rust code should parse. Error: {:?}\nCode:\n{}",
            result.err(),
            code
        );

        let ast = result.unwrap();
        assert_eq!(ast.functions[0].name, "check_status");
        assert_eq!(ast.functions[0].params.len(), 2); // rate_exceeded, locked
    }

    #[test]
    fn rust_string_output_roundtrip() {
        use crate::parse::parse_rust;

        let spec = literal_string_output_spec();
        let code = render(&spec, Target::Rust);

        let result = parse_rust(&code);
        assert!(
            result.is_ok(),
            "String output spec should parse. Error: {:?}\nCode:\n{}",
            result.err(),
            code
        );
    }

    // =========================================================================
    // Round-trip tests for ALL languages
    // =========================================================================

    #[test]
    fn typescript_roundtrip_parses_successfully() {
        use crate::parse::parse_typescript;

        let spec = computed_spec();
        let code = render(&spec, Target::TypeScript);

        let result = parse_typescript(&code);
        assert!(
            result.is_ok(),
            "TypeScript rendered code should parse. Error: {:?}\nCode:\n{}",
            result.err(),
            code
        );

        let ast = result.unwrap();
        assert_eq!(ast.functions.len(), 1, "Should have one function");
        assert_eq!(ast.functions[0].name, "shippingRate");
    }

    #[test]
    fn python_roundtrip_parses_successfully() {
        use crate::parse::parse_python;

        let spec = computed_spec();
        let code = render(&spec, Target::Python);

        let result = parse_python(&code);
        assert!(
            result.is_ok(),
            "Python rendered code should parse. Error: {:?}\nCode:\n{}",
            result.err(),
            code
        );

        let ast = result.unwrap();
        assert_eq!(ast.functions.len(), 1, "Should have one function");
        assert_eq!(ast.functions[0].name, "shipping_rate");
    }

    #[test]
    fn go_roundtrip_parses_successfully() {
        use crate::parse::parse_go;

        let spec = computed_spec();
        let code = render(&spec, Target::Go);

        // Go needs package declaration for valid syntax
        let full_code = format!("package main\n\n{}", code);

        let result = parse_go(&full_code);
        assert!(
            result.is_ok(),
            "Go rendered code should parse. Error: {:?}\nCode:\n{}",
            result.err(),
            full_code
        );

        let ast = result.unwrap();
        assert_eq!(ast.functions.len(), 1, "Should have one function");
    }

    #[test]
    fn csharp_roundtrip_parses_successfully() {
        use crate::parse::parse_csharp;

        let spec = computed_spec();
        let code = render(&spec, Target::CSharp);

        let result = parse_csharp(&code);
        assert!(
            result.is_ok(),
            "C# rendered code should parse. Error: {:?}\nCode:\n{}",
            result.err(),
            code
        );

        let ast = result.unwrap();
        assert!(!ast.functions.is_empty(), "Should have at least one method");
    }

    #[test]
    fn java_roundtrip_parses_successfully() {
        use crate::parse::parse_java;

        let spec = computed_spec();
        let code = render(&spec, Target::Java);

        let result = parse_java(&code);
        assert!(
            result.is_ok(),
            "Java rendered code should parse. Error: {:?}\nCode:\n{}",
            result.err(),
            code
        );

        let ast = result.unwrap();
        assert!(!ast.functions.is_empty(), "Should have at least one method");
    }

    // =========================================================================
    // Verify AST structure for all languages
    // =========================================================================

    #[test]
    fn rust_ast_has_if_else_structure() {
        use crate::ast::AstNode;
        use crate::parse::parse_rust;

        let spec = computed_spec();
        let code = render(&spec, Target::Rust);
        let ast = parse_rust(&code).unwrap();

        // Check the body contains an If expression (since CEL conditions generate if-else)
        if let AstNode::Block { result, .. } = &ast.functions[0].body {
            assert!(
                matches!(result.as_deref(), Some(AstNode::If { .. })),
                "Rust body should have If expression. Got: {:?}",
                result
            );
        }
    }

    #[test]
    fn typescript_ast_has_if_else_structure() {
        use crate::ast::AstNode;
        use crate::parse::parse_typescript;

        let spec = computed_spec();
        let code = render(&spec, Target::TypeScript);
        let ast = parse_typescript(&code).unwrap();

        // TypeScript renders with if-else for CEL conditions
        if let AstNode::Block { result, .. } = &ast.functions[0].body {
            assert!(
                matches!(result.as_deref(), Some(AstNode::If { .. })),
                "TypeScript body should have If expression"
            );
        }
    }

    #[test]
    fn python_ast_has_if_else_structure() {
        use crate::ast::AstNode;
        use crate::parse::parse_python;

        let spec = computed_spec();
        let code = render(&spec, Target::Python);
        let ast = parse_python(&code).unwrap();

        // Python uses if-else for CEL conditions
        if let AstNode::Block { result, .. } = &ast.functions[0].body {
            assert!(
                matches!(result.as_deref(), Some(AstNode::If { .. })),
                "Python body should have If expression. Got: {:?}",
                result
            );
        }
    }
}
