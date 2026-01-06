//! Template-based code generation
//!
//! Uses MiniJinja templates for properly formatted code generation.
//! Templates are embedded by default, with optional override via:
//! - `--template-dir` CLI flag
//! - `.imacs_root` config: `defaults.template_dir`

pub mod context;
pub mod filters;

use minijinja::Environment;
use std::path::Path;
use std::sync::OnceLock;

use crate::cel::Target;

// Embedded templates (compiled into binary)
mod embedded {
    // Spec templates
    pub const RUST_SPEC: &str = include_str!("../../templates/specs/rust.jinja");
    pub const TYPESCRIPT_SPEC: &str = include_str!("../../templates/specs/typescript.jinja");
    pub const PYTHON_SPEC: &str = include_str!("../../templates/specs/python.jinja");
    pub const GO_SPEC: &str = include_str!("../../templates/specs/go.jinja");
    pub const JAVA_SPEC: &str = include_str!("../../templates/specs/java.jinja");
    pub const CSHARP_SPEC: &str = include_str!("../../templates/specs/csharp.jinja");

    // Orchestrator templates
    pub const RUST_ORCH: &str = include_str!("../../templates/orchestrators/rust.jinja");
    pub const TYPESCRIPT_ORCH: &str =
        include_str!("../../templates/orchestrators/typescript.jinja");
    pub const PYTHON_ORCH: &str = include_str!("../../templates/orchestrators/python.jinja");
    pub const GO_ORCH: &str = include_str!("../../templates/orchestrators/go.jinja");
    pub const JAVA_ORCH: &str = include_str!("../../templates/orchestrators/java.jinja");
    pub const CSHARP_ORCH: &str = include_str!("../../templates/orchestrators/csharp.jinja");
}

/// Template engine singleton
static ENGINE: OnceLock<Environment<'static>> = OnceLock::new();

/// Initialize the template engine with embedded templates
fn init_engine() -> Environment<'static> {
    let mut env = Environment::new();

    // Register custom filters
    filters::register_filters(&mut env);

    // Load embedded spec templates
    env.add_template("specs/rust.jinja", embedded::RUST_SPEC)
        .expect("Failed to load rust spec template");
    env.add_template("specs/typescript.jinja", embedded::TYPESCRIPT_SPEC)
        .expect("Failed to load typescript spec template");
    env.add_template("specs/python.jinja", embedded::PYTHON_SPEC)
        .expect("Failed to load python spec template");
    env.add_template("specs/go.jinja", embedded::GO_SPEC)
        .expect("Failed to load go spec template");
    env.add_template("specs/java.jinja", embedded::JAVA_SPEC)
        .expect("Failed to load java spec template");
    env.add_template("specs/csharp.jinja", embedded::CSHARP_SPEC)
        .expect("Failed to load csharp spec template");

    // Load embedded orchestrator templates
    env.add_template("orchestrators/rust.jinja", embedded::RUST_ORCH)
        .expect("Failed to load rust orchestrator template");
    env.add_template("orchestrators/typescript.jinja", embedded::TYPESCRIPT_ORCH)
        .expect("Failed to load typescript orchestrator template");
    env.add_template("orchestrators/python.jinja", embedded::PYTHON_ORCH)
        .expect("Failed to load python orchestrator template");
    env.add_template("orchestrators/go.jinja", embedded::GO_ORCH)
        .expect("Failed to load go orchestrator template");
    env.add_template("orchestrators/java.jinja", embedded::JAVA_ORCH)
        .expect("Failed to load java orchestrator template");
    env.add_template("orchestrators/csharp.jinja", embedded::CSHARP_ORCH)
        .expect("Failed to load csharp orchestrator template");

    env
}

/// Get the global template engine
pub fn engine() -> &'static Environment<'static> {
    ENGINE.get_or_init(init_engine)
}

/// Create a new template engine with custom template directory
/// Templates in custom_dir override embedded templates
pub fn engine_with_override(custom_dir: &Path) -> Result<Environment<'static>, TemplateError> {
    let mut env = init_engine();

    // Load custom templates, overriding embedded ones
    load_custom_templates(&mut env, custom_dir)?;

    Ok(env)
}

/// Load custom templates from a directory
fn load_custom_templates(env: &mut Environment<'static>, dir: &Path) -> Result<(), TemplateError> {
    // Load spec templates if they exist
    for (target, filename) in [
        ("rust", "rust.jinja"),
        ("typescript", "typescript.jinja"),
        ("python", "python.jinja"),
        ("go", "go.jinja"),
        ("java", "java.jinja"),
        ("csharp", "csharp.jinja"),
    ] {
        let spec_path = dir.join("specs").join(filename);
        if spec_path.exists() {
            let content = std::fs::read_to_string(&spec_path).map_err(|e| {
                TemplateError::IoError(format!("Failed to read {}: {}", spec_path.display(), e))
            })?;
            let template_name = format!("specs/{}", filename);
            // Note: This leaks the strings, but it's acceptable for config-time loading
            let leaked_name: &'static str = Box::leak(template_name.into_boxed_str());
            let leaked_content: &'static str = Box::leak(content.into_boxed_str());
            env.add_template(leaked_name, leaked_content)
                .map_err(|e| TemplateError::ParseError(target.into(), e.to_string()))?;
        }

        let orch_path = dir.join("orchestrators").join(filename);
        if orch_path.exists() {
            let content = std::fs::read_to_string(&orch_path).map_err(|e| {
                TemplateError::IoError(format!("Failed to read {}: {}", orch_path.display(), e))
            })?;
            let template_name = format!("orchestrators/{}", filename);
            let leaked_name: &'static str = Box::leak(template_name.into_boxed_str());
            let leaked_content: &'static str = Box::leak(content.into_boxed_str());
            env.add_template(leaked_name, leaked_content)
                .map_err(|e| TemplateError::ParseError(target.into(), e.to_string()))?;
        }
    }

    Ok(())
}

/// Get the template name for a target language (specs)
pub fn spec_template_name(target: Target) -> &'static str {
    match target {
        Target::Rust => "specs/rust.jinja",
        Target::TypeScript => "specs/typescript.jinja",
        Target::Python => "specs/python.jinja",
        Target::Go => "specs/go.jinja",
        Target::Java => "specs/java.jinja",
        Target::CSharp => "specs/csharp.jinja",
    }
}

/// Get the template name for a target language (orchestrators)
pub fn orchestrator_template_name(target: Target) -> &'static str {
    match target {
        Target::Rust => "orchestrators/rust.jinja",
        Target::TypeScript => "orchestrators/typescript.jinja",
        Target::Python => "orchestrators/python.jinja",
        Target::Go => "orchestrators/go.jinja",
        Target::Java => "orchestrators/java.jinja",
        Target::CSharp => "orchestrators/csharp.jinja",
    }
}

/// Render a spec using templates
pub fn render_spec(
    spec: &crate::spec::Spec,
    target: Target,
    provenance: bool,
) -> Result<String, TemplateError> {
    let env = engine();
    let template = env
        .get_template(spec_template_name(target))
        .map_err(|e| TemplateError::TemplateNotFound(e.to_string()))?;

    let ctx = context::SpecContext::from_spec(spec, target, provenance);
    template
        .render(&ctx)
        .map_err(|e| TemplateError::RenderError(e.to_string()))
}

/// Render an orchestrator using templates
pub fn render_orchestrator(
    orch: &crate::orchestrate::Orchestrator,
    specs: &std::collections::HashMap<String, crate::spec::Spec>,
    target: Target,
    provenance: bool,
) -> Result<String, TemplateError> {
    let env = engine();
    let template = env
        .get_template(orchestrator_template_name(target))
        .map_err(|e| TemplateError::TemplateNotFound(e.to_string()))?;

    let ctx = context::OrchestratorContext::from_orchestrator(orch, specs, target, provenance);
    template
        .render(&ctx)
        .map_err(|e| TemplateError::RenderError(e.to_string()))
}

/// Template errors
#[derive(Debug, Clone)]
pub enum TemplateError {
    /// Template not found
    TemplateNotFound(String),
    /// Template parse error
    ParseError(String, String),
    /// Template render error
    RenderError(String),
    /// IO error loading custom templates
    IoError(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::TemplateNotFound(msg) => write!(f, "Template not found: {}", msg),
            TemplateError::ParseError(lang, msg) => {
                write!(f, "Template parse error for {}: {}", lang, msg)
            }
            TemplateError::RenderError(msg) => write!(f, "Template render error: {}", msg),
            TemplateError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for TemplateError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Spec;

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
    when: "rate_exceeded"
    then: 429
  - id: R2
    when: "!rate_exceeded && locked"
    then: 423
  - id: R3
    when: "!rate_exceeded && !locked"
    then: 200
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_engine_initialization() {
        let env = engine();
        // Should be able to get all spec templates
        assert!(env.get_template("specs/rust.jinja").is_ok());
        assert!(env.get_template("specs/typescript.jinja").is_ok());
        assert!(env.get_template("specs/python.jinja").is_ok());
        assert!(env.get_template("specs/go.jinja").is_ok());
        assert!(env.get_template("specs/java.jinja").is_ok());
        assert!(env.get_template("specs/csharp.jinja").is_ok());
    }

    #[test]
    fn test_template_names() {
        assert_eq!(spec_template_name(Target::Rust), "specs/rust.jinja");
        assert_eq!(
            orchestrator_template_name(Target::TypeScript),
            "orchestrators/typescript.jinja"
        );
    }

    #[test]
    fn test_render_rust_spec() {
        let spec = sample_spec();
        let result = render_spec(&spec, Target::Rust, true);
        assert!(result.is_ok(), "Rust render failed: {:?}", result.err());

        let code = result.unwrap();
        assert!(
            code.contains("pub fn check_status"),
            "Missing function name"
        );
        assert!(code.contains("GENERATED FROM:"), "Missing provenance");
        assert!(code.contains("429"), "Missing rule R1 output");
    }

    #[test]
    fn test_render_typescript_spec() {
        let spec = sample_spec();
        let result = render_spec(&spec, Target::TypeScript, true);
        assert!(
            result.is_ok(),
            "TypeScript render failed: {:?}",
            result.err()
        );

        let code = result.unwrap();
        assert!(
            code.contains("export function checkStatus"),
            "Missing function name"
        );
        assert!(code.contains("CheckStatusInput"), "Missing input interface");
        assert!(code.contains("429"), "Missing rule R1 output");
    }

    #[test]
    fn test_render_python_spec() {
        let spec = sample_spec();
        let result = render_spec(&spec, Target::Python, true);
        assert!(result.is_ok(), "Python render failed: {:?}", result.err());

        let code = result.unwrap();
        assert!(code.contains("def check_status"), "Missing function name");
        assert!(code.contains("CheckStatusInput"), "Missing input class");
        assert!(code.contains("429"), "Missing rule R1 output");
    }

    #[test]
    fn test_render_go_spec() {
        let spec = sample_spec();
        let result = render_spec(&spec, Target::Go, true);
        assert!(result.is_ok(), "Go render failed: {:?}", result.err());

        let code = result.unwrap();
        assert!(code.contains("func CheckStatus"), "Missing function name");
        assert!(code.contains("CheckStatusInput"), "Missing input struct");
        assert!(code.contains("429"), "Missing rule R1 output");
    }

    #[test]
    fn test_render_java_spec() {
        let spec = sample_spec();
        let result = render_spec(&spec, Target::Java, true);
        assert!(result.is_ok(), "Java render failed: {:?}", result.err());

        let code = result.unwrap();
        assert!(
            code.contains("public class CheckStatus"),
            "Missing class name"
        );
        assert!(code.contains("429"), "Missing rule R1 output");
    }

    #[test]
    fn test_render_csharp_spec() {
        let spec = sample_spec();
        let result = render_spec(&spec, Target::CSharp, true);
        assert!(result.is_ok(), "C# render failed: {:?}", result.err());

        let code = result.unwrap();
        assert!(code.contains("CheckStatusInput"), "Missing input class");
        assert!(code.contains("Evaluate"), "Missing method name");
        assert!(code.contains("429"), "Missing rule R1 output");
    }

    // Integration test: verify template output is properly formatted
    #[test]
    fn test_template_output_formatting() {
        let spec = sample_spec();

        // Rust - check proper indentation
        let rust_code = render_spec(&spec, Target::Rust, false).unwrap();
        assert!(
            rust_code.contains("    if"),
            "Rust should have proper indentation"
        );
        assert!(
            rust_code.lines().count() > 5,
            "Rust should have multiple lines"
        );

        // Python - check proper indentation (4 spaces)
        let py_code = render_spec(&spec, Target::Python, false).unwrap();
        assert!(
            py_code.contains("    "),
            "Python should have 4-space indentation"
        );
        assert!(py_code.contains("if "), "Python should have if statements");

        // C# - check braces on new lines (proper formatting)
        let cs_code = render_spec(&spec, Target::CSharp, false).unwrap();
        assert!(
            cs_code.contains("{\n"),
            "C# should have braces on proper lines"
        );
    }

    // Orchestrator template tests
    fn sample_orchestrator() -> crate::orchestrate::Orchestrator {
        crate::orchestrate::Orchestrator::from_yaml(
            r#"
id: test_flow
inputs:
  - name: user_id
    type: string
  - name: amount
    type: float
outputs:
  - name: approved
    type: bool
chain:
  - step: gate
    id: check_input
    condition: "user_id != ''"
  - step: call
    id: validate
    spec: validate_user
    inputs:
      id: "user_id"
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_render_orchestrator_rust() {
        let orch = sample_orchestrator();
        let specs = std::collections::HashMap::new();
        let result = render_orchestrator(&orch, &specs, Target::Rust, true);
        assert!(
            result.is_ok(),
            "Rust orchestrator render failed: {:?}",
            result.err()
        );

        let code = result.unwrap();
        assert!(code.contains("TestFlow"), "Missing struct name");
        assert!(code.contains("pub fn test_flow"), "Missing function name");
        assert!(code.lines().count() > 10, "Should have multiple lines");
    }

    #[test]
    fn test_render_orchestrator_python() {
        let orch = sample_orchestrator();
        let specs = std::collections::HashMap::new();
        let result = render_orchestrator(&orch, &specs, Target::Python, true);
        assert!(
            result.is_ok(),
            "Python orchestrator render failed: {:?}",
            result.err()
        );

        let code = result.unwrap();
        assert!(code.contains("TestFlow"), "Missing class name");
        assert!(code.contains("def test_flow"), "Missing function name");
        // Check proper formatting
        assert!(code.lines().count() > 10, "Should have multiple lines");
        assert!(
            !code.contains("# Generated orchestrator: test_flow # DO NOT EDIT"),
            "Output should not be single-line genco format"
        );
    }

    #[test]
    fn test_render_orchestrator_typescript() {
        let orch = sample_orchestrator();
        let specs = std::collections::HashMap::new();
        let result = render_orchestrator(&orch, &specs, Target::TypeScript, true);
        assert!(
            result.is_ok(),
            "TypeScript orchestrator render failed: {:?}",
            result.err()
        );

        let code = result.unwrap();
        assert!(code.contains("TestFlowInput"), "Missing input interface");
        assert!(code.contains("async function"), "Missing async function");
        assert!(code.lines().count() > 10, "Should have multiple lines");
    }
}
