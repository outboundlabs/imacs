//! Type-safe namespace/scoping types for code generation
//!
//! This module provides validated namespace types for each target language,
//! ensuring compile-time safety and runtime validation of namespace formats.
//!
//! Each language has specific conventions:
//! - C#: PascalCase.Separated (e.g., "Company.Rules.Auth")
//! - Java: lowercase.separated (e.g., "com.company.rules")
//! - Go: single lowercase identifier (e.g., "rules")
//! - Python: snake_case.separated (e.g., "company.rules")
//! - Rust: snake_case::separated (e.g., "crate::rules::auth")
//! - TypeScript: path segments (e.g., ["rules", "auth"])

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

use super::to_pascal_case;

// ============================================================================
// Namespace Errors
// ============================================================================

/// Namespace validation errors with actionable messages
#[derive(Error, Debug, Clone, PartialEq)]
pub enum NamespaceError {
    #[error("Namespace is required but not specified for target language")]
    Required,

    #[error("Namespace segment cannot be empty")]
    EmptySegment,

    #[error("Namespace segment must start with a letter or underscore, got '{0}'")]
    InvalidStart(char),

    #[error("Namespace segment contains invalid character: '{0}'")]
    InvalidChar(char),

    #[error("C# namespace must use PascalCase: '{0}' (suggestion: '{1}')")]
    CSharpNotPascalCase(String, String),

    #[error("Java package must be lowercase: '{0}' (suggestion: '{1}')")]
    JavaNotLowercase(String, String),

    #[error("Go package must be a single lowercase identifier without dots: '{0}'")]
    GoInvalidPackage(String),

    #[error("Go package must be lowercase: '{0}' (suggestion: '{1}')")]
    GoNotLowercase(String, String),

    #[error("Python module segment must be lowercase/snake_case: '{0}' (suggestion: '{1}')")]
    PythonNotSnakeCase(String, String),

    #[error("Rust module segment must be lowercase/snake_case: '{0}' (suggestion: '{1}')")]
    RustNotSnakeCase(String, String),

    #[error("'{0}' is a reserved word in {1}")]
    ReservedWord(String, String),
}

// ============================================================================
// C# Namespace
// ============================================================================

/// C# namespace (PascalCase.Separated)
///
/// Example: "Company.Rules.Auth"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct CSharpNamespace {
    segments: Vec<String>,
}

impl CSharpNamespace {
    /// Create a new C# namespace from segments
    pub fn new(segments: Vec<String>) -> Result<Self, NamespaceError> {
        if segments.is_empty() {
            return Err(NamespaceError::EmptySegment);
        }
        for seg in &segments {
            validate_csharp_segment(seg)?;
        }
        Ok(Self { segments })
    }

    /// Get the namespace segments
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Render as C# namespace declaration
    pub fn render(&self) -> String {
        self.segments.join(".")
    }
}

impl TryFrom<String> for CSharpNamespace {
    type Error = NamespaceError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err(NamespaceError::EmptySegment);
        }
        let segments: Vec<String> = s.split('.').map(String::from).collect();
        Self::new(segments)
    }
}

impl From<CSharpNamespace> for String {
    fn from(ns: CSharpNamespace) -> Self {
        ns.render()
    }
}

impl fmt::Display for CSharpNamespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

fn validate_csharp_segment(seg: &str) -> Result<(), NamespaceError> {
    if seg.is_empty() {
        return Err(NamespaceError::EmptySegment);
    }

    let first_char = seg.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(NamespaceError::InvalidStart(first_char));
    }

    for c in seg.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(NamespaceError::InvalidChar(c));
        }
    }

    // Check PascalCase convention (first char should be uppercase)
    if first_char.is_ascii_lowercase() {
        let suggestion = to_pascal_case(seg);
        return Err(NamespaceError::CSharpNotPascalCase(
            seg.to_string(),
            suggestion,
        ));
    }

    // Check for C# reserved words
    if is_csharp_reserved(seg) {
        return Err(NamespaceError::ReservedWord(seg.to_string(), "C#".into()));
    }

    Ok(())
}

fn is_csharp_reserved(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "abstract"
            | "as"
            | "base"
            | "bool"
            | "break"
            | "byte"
            | "case"
            | "catch"
            | "char"
            | "checked"
            | "class"
            | "const"
            | "continue"
            | "decimal"
            | "default"
            | "delegate"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "event"
            | "explicit"
            | "extern"
            | "false"
            | "finally"
            | "fixed"
            | "float"
            | "for"
            | "foreach"
            | "goto"
            | "if"
            | "implicit"
            | "in"
            | "int"
            | "interface"
            | "internal"
            | "is"
            | "lock"
            | "long"
            | "namespace"
            | "new"
            | "null"
            | "object"
            | "operator"
            | "out"
            | "override"
            | "params"
            | "private"
            | "protected"
            | "public"
            | "readonly"
            | "ref"
            | "return"
            | "sbyte"
            | "sealed"
            | "short"
            | "sizeof"
            | "stackalloc"
            | "static"
            | "string"
            | "struct"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "uint"
            | "ulong"
            | "unchecked"
            | "unsafe"
            | "ushort"
            | "using"
            | "virtual"
            | "void"
            | "volatile"
            | "while"
    )
}

// ============================================================================
// Java Package
// ============================================================================

/// Java package (lowercase.separated)
///
/// Example: "com.company.rules.auth"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct JavaPackage {
    segments: Vec<String>,
}

impl JavaPackage {
    /// Create a new Java package from segments
    pub fn new(segments: Vec<String>) -> Result<Self, NamespaceError> {
        if segments.is_empty() {
            return Err(NamespaceError::EmptySegment);
        }
        for seg in &segments {
            validate_java_segment(seg)?;
        }
        Ok(Self { segments })
    }

    /// Get the package segments
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Render as Java package declaration
    pub fn render(&self) -> String {
        self.segments.join(".")
    }
}

impl TryFrom<String> for JavaPackage {
    type Error = NamespaceError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err(NamespaceError::EmptySegment);
        }
        let segments: Vec<String> = s.split('.').map(String::from).collect();
        Self::new(segments)
    }
}

impl From<JavaPackage> for String {
    fn from(pkg: JavaPackage) -> Self {
        pkg.render()
    }
}

impl fmt::Display for JavaPackage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

fn validate_java_segment(seg: &str) -> Result<(), NamespaceError> {
    if seg.is_empty() {
        return Err(NamespaceError::EmptySegment);
    }

    let first_char = seg.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(NamespaceError::InvalidStart(first_char));
    }

    for c in seg.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(NamespaceError::InvalidChar(c));
        }
    }

    // Check lowercase convention
    if seg != seg.to_lowercase() {
        return Err(NamespaceError::JavaNotLowercase(
            seg.to_string(),
            seg.to_lowercase(),
        ));
    }

    // Check for Java reserved words
    if is_java_reserved(seg) {
        return Err(NamespaceError::ReservedWord(seg.to_string(), "Java".into()));
    }

    Ok(())
}

fn is_java_reserved(word: &str) -> bool {
    matches!(
        word,
        "abstract"
            | "assert"
            | "boolean"
            | "break"
            | "byte"
            | "case"
            | "catch"
            | "char"
            | "class"
            | "const"
            | "continue"
            | "default"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "extends"
            | "final"
            | "finally"
            | "float"
            | "for"
            | "goto"
            | "if"
            | "implements"
            | "import"
            | "instanceof"
            | "int"
            | "interface"
            | "long"
            | "native"
            | "new"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "return"
            | "short"
            | "static"
            | "strictfp"
            | "super"
            | "switch"
            | "synchronized"
            | "this"
            | "throw"
            | "throws"
            | "transient"
            | "try"
            | "void"
            | "volatile"
            | "while"
            | "true"
            | "false"
            | "null"
    )
}

// ============================================================================
// Go Package
// ============================================================================

/// Go package name (single lowercase identifier)
///
/// Example: "rules"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct GoPackageName(String);

impl GoPackageName {
    /// Create a new Go package name
    pub fn new(name: String) -> Result<Self, NamespaceError> {
        validate_go_package_name(&name)?;
        Ok(Self(name))
    }

    /// Get the package name
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for GoPackageName {
    type Error = NamespaceError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<GoPackageName> for String {
    fn from(pkg: GoPackageName) -> Self {
        pkg.0
    }
}

impl fmt::Display for GoPackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn validate_go_package_name(name: &str) -> Result<(), NamespaceError> {
    if name.is_empty() {
        return Err(NamespaceError::EmptySegment);
    }

    // Go packages must be single identifier (no dots or slashes)
    if name.contains('.') || name.contains('/') {
        return Err(NamespaceError::GoInvalidPackage(name.to_string()));
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(NamespaceError::InvalidStart(first_char));
    }

    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(NamespaceError::InvalidChar(c));
        }
    }

    // Go packages should be lowercase
    if name != name.to_lowercase() {
        return Err(NamespaceError::GoNotLowercase(
            name.to_string(),
            name.to_lowercase(),
        ));
    }

    // Check for Go reserved words
    if is_go_reserved(name) {
        return Err(NamespaceError::ReservedWord(name.to_string(), "Go".into()));
    }

    Ok(())
}

fn is_go_reserved(word: &str) -> bool {
    matches!(
        word,
        "break"
            | "case"
            | "chan"
            | "const"
            | "continue"
            | "default"
            | "defer"
            | "else"
            | "fallthrough"
            | "for"
            | "func"
            | "go"
            | "goto"
            | "if"
            | "import"
            | "interface"
            | "map"
            | "package"
            | "range"
            | "return"
            | "select"
            | "struct"
            | "switch"
            | "type"
            | "var"
    )
}

/// Go package configuration with optional module path for imports
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GoPackage {
    /// Package name (single lowercase identifier)
    pub name: GoPackageName,

    /// Optional module path for imports (e.g., "github.com/company/project")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_path: Option<String>,
}

impl GoPackage {
    /// Create a new Go package
    pub fn new(name: GoPackageName, module_path: Option<String>) -> Self {
        Self { name, module_path }
    }

    /// Render as Go package declaration
    pub fn render(&self) -> String {
        self.name.name().to_string()
    }
}

// ============================================================================
// Python Module
// ============================================================================

/// Python module path
///
/// Example: ["company", "rules", "auth"]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PythonModule {
    /// Module path segments (e.g., ["company", "rules", "auth"])
    pub path: Vec<String>,

    /// Generate __init__.py files for hierarchical output
    #[serde(default = "default_true")]
    pub generate_init: bool,
}

fn default_true() -> bool {
    true
}

impl PythonModule {
    /// Create a new Python module from path segments
    pub fn new(path: Vec<String>, generate_init: bool) -> Result<Self, NamespaceError> {
        if path.is_empty() {
            return Err(NamespaceError::EmptySegment);
        }
        for seg in &path {
            validate_python_segment(seg)?;
        }
        Ok(Self {
            path,
            generate_init,
        })
    }

    /// Get the module path segments
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Render as Python module path (dot-separated)
    pub fn render(&self) -> String {
        self.path.join(".")
    }
}

impl TryFrom<String> for PythonModule {
    type Error = NamespaceError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err(NamespaceError::EmptySegment);
        }
        let path: Vec<String> = s.split('.').map(String::from).collect();
        Self::new(path, true)
    }
}

impl fmt::Display for PythonModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

fn validate_python_segment(seg: &str) -> Result<(), NamespaceError> {
    if seg.is_empty() {
        return Err(NamespaceError::EmptySegment);
    }

    let first_char = seg.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(NamespaceError::InvalidStart(first_char));
    }

    for c in seg.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(NamespaceError::InvalidChar(c));
        }
    }

    // Python modules should be lowercase/snake_case
    if seg != seg.to_lowercase() {
        return Err(NamespaceError::PythonNotSnakeCase(
            seg.to_string(),
            crate::util::to_snake_case(seg),
        ));
    }

    // Check for Python reserved words
    if is_python_reserved(seg) {
        return Err(NamespaceError::ReservedWord(
            seg.to_string(),
            "Python".into(),
        ));
    }

    Ok(())
}

fn is_python_reserved(word: &str) -> bool {
    matches!(
        word,
        "False"
            | "None"
            | "True"
            | "and"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "class"
            | "continue"
            | "def"
            | "del"
            | "elif"
            | "else"
            | "except"
            | "finally"
            | "for"
            | "from"
            | "global"
            | "if"
            | "import"
            | "in"
            | "is"
            | "lambda"
            | "nonlocal"
            | "not"
            | "or"
            | "pass"
            | "raise"
            | "return"
            | "try"
            | "while"
            | "with"
            | "yield"
    )
}

// ============================================================================
// Rust Module
// ============================================================================

/// Rust visibility level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RustVisibility {
    /// `pub` - public to all
    #[default]
    Pub,
    /// `pub(crate)` - public within crate
    PubCrate,
    /// `pub(super)` - public to parent module
    PubSuper,
    /// Private (no visibility modifier)
    Private,
}

impl RustVisibility {
    /// Render the visibility modifier
    pub fn render(&self) -> &'static str {
        match self {
            RustVisibility::Pub => "pub ",
            RustVisibility::PubCrate => "pub(crate) ",
            RustVisibility::PubSuper => "pub(super) ",
            RustVisibility::Private => "",
        }
    }
}

/// Rust module path
///
/// Example: ["crate", "rules", "auth"]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RustModule {
    /// Module path segments (e.g., ["crate", "rules", "auth"])
    pub path: Vec<String>,

    /// Visibility for generated items
    #[serde(default)]
    pub visibility: RustVisibility,
}

impl RustModule {
    /// Create a new Rust module from path segments
    pub fn new(path: Vec<String>, visibility: RustVisibility) -> Result<Self, NamespaceError> {
        if path.is_empty() {
            return Err(NamespaceError::EmptySegment);
        }
        for seg in &path {
            validate_rust_segment(seg)?;
        }
        Ok(Self { path, visibility })
    }

    /// Get the module path segments
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Render as Rust module path (:: separated)
    pub fn render(&self) -> String {
        self.path.join("::")
    }
}

impl TryFrom<String> for RustModule {
    type Error = NamespaceError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err(NamespaceError::EmptySegment);
        }
        let path: Vec<String> = s.split("::").map(String::from).collect();
        Self::new(path, RustVisibility::default())
    }
}

impl fmt::Display for RustModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

fn validate_rust_segment(seg: &str) -> Result<(), NamespaceError> {
    if seg.is_empty() {
        return Err(NamespaceError::EmptySegment);
    }

    // Allow "crate", "self", "super" as special segments
    if matches!(seg, "crate" | "self" | "super") {
        return Ok(());
    }

    let first_char = seg.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(NamespaceError::InvalidStart(first_char));
    }

    for c in seg.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(NamespaceError::InvalidChar(c));
        }
    }

    // Rust modules should be lowercase/snake_case
    if seg != seg.to_lowercase() {
        return Err(NamespaceError::RustNotSnakeCase(
            seg.to_string(),
            crate::util::to_snake_case(seg),
        ));
    }

    // Check for Rust reserved words (excluding allowed module names)
    if is_rust_reserved(seg) {
        return Err(NamespaceError::ReservedWord(seg.to_string(), "Rust".into()));
    }

    Ok(())
}

fn is_rust_reserved(word: &str) -> bool {
    matches!(
        word,
        "as" | "break"
            | "const"
            | "continue"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "static"
            | "struct"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}

// ============================================================================
// TypeScript Module
// ============================================================================

/// TypeScript module path
///
/// Example: ["rules", "auth"]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TypeScriptModule {
    /// Module path segments (e.g., ["rules", "auth"])
    pub path: Vec<String>,

    /// Generate index.ts barrel exports
    #[serde(default = "default_true")]
    pub barrel_exports: bool,
}

impl TypeScriptModule {
    /// Create a new TypeScript module from path segments
    pub fn new(path: Vec<String>, barrel_exports: bool) -> Result<Self, NamespaceError> {
        if path.is_empty() {
            return Err(NamespaceError::EmptySegment);
        }
        for seg in &path {
            validate_typescript_segment(seg)?;
        }
        Ok(Self {
            path,
            barrel_exports,
        })
    }

    /// Get the module path segments
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Render as TypeScript module path (/ separated)
    pub fn render(&self) -> String {
        self.path.join("/")
    }
}

impl TryFrom<String> for TypeScriptModule {
    type Error = NamespaceError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err(NamespaceError::EmptySegment);
        }
        let path: Vec<String> = s.split('/').map(String::from).collect();
        Self::new(path, true)
    }
}

impl fmt::Display for TypeScriptModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

fn validate_typescript_segment(seg: &str) -> Result<(), NamespaceError> {
    if seg.is_empty() {
        return Err(NamespaceError::EmptySegment);
    }

    let first_char = seg.chars().next().unwrap();
    // TypeScript allows @ for scoped packages, and - for kebab-case
    if !first_char.is_ascii_alphabetic() && first_char != '_' && first_char != '@' {
        return Err(NamespaceError::InvalidStart(first_char));
    }

    for c in seg.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '@' {
            return Err(NamespaceError::InvalidChar(c));
        }
    }

    // Check for TypeScript/JavaScript reserved words
    if is_typescript_reserved(seg) {
        return Err(NamespaceError::ReservedWord(
            seg.to_string(),
            "TypeScript".into(),
        ));
    }

    Ok(())
}

fn is_typescript_reserved(word: &str) -> bool {
    matches!(
        word,
        "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "as"
            | "implements"
            | "interface"
            | "let"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "yield"
            | "any"
            | "boolean"
            | "constructor"
            | "declare"
            | "get"
            | "module"
            | "require"
            | "number"
            | "set"
            | "string"
            | "symbol"
            | "type"
            | "from"
            | "of"
            | "async"
            | "await"
    )
}

// ============================================================================
// Scoping Configuration
// ============================================================================

/// Scoping configuration for a spec
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ScopingConfig {
    /// Per-language namespace configurations
    pub languages: LanguageScopingTyped,
}

impl ScopingConfig {
    /// Check if scoping is configured for any language
    pub fn is_empty(&self) -> bool {
        self.languages.is_empty()
    }

    /// Get the namespace for a specific target language
    pub fn for_target(&self, target: crate::cel::Target) -> Option<ResolvedNamespace> {
        use crate::cel::Target;
        match target {
            Target::CSharp => self
                .languages
                .csharp
                .as_ref()
                .map(|ns| ResolvedNamespace::CSharp(ns.clone())),
            Target::Java => self
                .languages
                .java
                .as_ref()
                .map(|pkg| ResolvedNamespace::Java(pkg.clone())),
            Target::Go => self
                .languages
                .go
                .as_ref()
                .map(|pkg| ResolvedNamespace::Go(pkg.clone())),
            Target::Python => self
                .languages
                .python
                .as_ref()
                .map(|m| ResolvedNamespace::Python(m.clone())),
            Target::Rust => self
                .languages
                .rust
                .as_ref()
                .map(|m| ResolvedNamespace::Rust(m.clone())),
            Target::TypeScript => self
                .languages
                .typescript
                .as_ref()
                .map(|m| ResolvedNamespace::TypeScript(m.clone())),
        }
    }

    /// Validate that all required targets have scoping configured
    pub fn validate(&self, targets: &[crate::cel::Target]) -> Result<(), NamespaceError> {
        for target in targets {
            if self.for_target(*target).is_none() {
                return Err(NamespaceError::Required);
            }
        }
        Ok(())
    }
}

/// Per-language namespace configurations (type-safe)
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct LanguageScopingTyped {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust: Option<RustModule>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typescript: Option<TypeScriptModule>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonModule>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub go: Option<GoPackage>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java: Option<JavaPackage>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csharp: Option<CSharpNamespace>,
}

impl LanguageScopingTyped {
    /// Check if all language scoping options are None
    pub fn is_empty(&self) -> bool {
        self.rust.is_none()
            && self.typescript.is_none()
            && self.python.is_none()
            && self.go.is_none()
            && self.java.is_none()
            && self.csharp.is_none()
    }
}

/// Resolved namespace for a specific target language
#[derive(Debug, Clone)]
pub enum ResolvedNamespace {
    CSharp(CSharpNamespace),
    Java(JavaPackage),
    Go(GoPackage),
    Python(PythonModule),
    Rust(RustModule),
    TypeScript(TypeScriptModule),
}

impl ResolvedNamespace {
    /// Render the namespace as a string appropriate for the language
    pub fn render(&self) -> String {
        match self {
            ResolvedNamespace::CSharp(ns) => ns.render(),
            ResolvedNamespace::Java(pkg) => pkg.render(),
            ResolvedNamespace::Go(pkg) => pkg.render(),
            ResolvedNamespace::Python(m) => m.render(),
            ResolvedNamespace::Rust(m) => m.render(),
            ResolvedNamespace::TypeScript(m) => m.render(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // C# Namespace tests
    #[test]
    fn test_csharp_namespace_valid() {
        let ns = CSharpNamespace::try_from("Company.Rules.Auth".to_string()).unwrap();
        assert_eq!(ns.segments(), &["Company", "Rules", "Auth"]);
        assert_eq!(ns.render(), "Company.Rules.Auth");
    }

    #[test]
    fn test_csharp_namespace_lowercase_rejected() {
        let result = CSharpNamespace::try_from("company.rules".to_string());
        assert!(matches!(
            result,
            Err(NamespaceError::CSharpNotPascalCase(_, _))
        ));
    }

    #[test]
    fn test_csharp_namespace_reserved_word() {
        let result = CSharpNamespace::try_from("Namespace.Test".to_string());
        assert!(matches!(result, Err(NamespaceError::ReservedWord(_, _))));
    }

    // Java Package tests
    #[test]
    fn test_java_package_valid() {
        let pkg = JavaPackage::try_from("com.company.rules".to_string()).unwrap();
        assert_eq!(pkg.segments(), &["com", "company", "rules"]);
        assert_eq!(pkg.render(), "com.company.rules");
    }

    #[test]
    fn test_java_package_uppercase_rejected() {
        let result = JavaPackage::try_from("com.Company.rules".to_string());
        assert!(matches!(
            result,
            Err(NamespaceError::JavaNotLowercase(_, _))
        ));
    }

    // Go Package tests
    #[test]
    fn test_go_package_valid() {
        let pkg = GoPackageName::try_from("rules".to_string()).unwrap();
        assert_eq!(pkg.name(), "rules");
    }

    #[test]
    fn test_go_package_dot_rejected() {
        let result = GoPackageName::try_from("com.rules".to_string());
        assert!(matches!(result, Err(NamespaceError::GoInvalidPackage(_))));
    }

    #[test]
    fn test_go_package_uppercase_rejected() {
        let result = GoPackageName::try_from("Rules".to_string());
        assert!(matches!(result, Err(NamespaceError::GoNotLowercase(_, _))));
    }

    // Python Module tests
    #[test]
    fn test_python_module_valid() {
        let m = PythonModule::try_from("company.rules.auth".to_string()).unwrap();
        assert_eq!(m.path(), &["company", "rules", "auth"]);
        assert_eq!(m.render(), "company.rules.auth");
    }

    #[test]
    fn test_python_module_uppercase_rejected() {
        let result = PythonModule::try_from("Company.rules".to_string());
        assert!(matches!(
            result,
            Err(NamespaceError::PythonNotSnakeCase(_, _))
        ));
    }

    // Rust Module tests
    #[test]
    fn test_rust_module_valid() {
        let m = RustModule::try_from("crate::rules::auth".to_string()).unwrap();
        assert_eq!(m.path(), &["crate", "rules", "auth"]);
        assert_eq!(m.render(), "crate::rules::auth");
    }

    #[test]
    fn test_rust_module_crate_allowed() {
        let m = RustModule::try_from("crate::my_module".to_string()).unwrap();
        assert_eq!(m.path(), &["crate", "my_module"]);
    }

    #[test]
    fn test_rust_module_uppercase_rejected() {
        let result = RustModule::try_from("crate::MyModule".to_string());
        assert!(matches!(
            result,
            Err(NamespaceError::RustNotSnakeCase(_, _))
        ));
    }

    // TypeScript Module tests
    #[test]
    fn test_typescript_module_valid() {
        let m = TypeScriptModule::try_from("rules/auth".to_string()).unwrap();
        assert_eq!(m.path(), &["rules", "auth"]);
        assert_eq!(m.render(), "rules/auth");
    }

    #[test]
    fn test_typescript_module_scoped_package() {
        let m = TypeScriptModule::try_from("@company/rules".to_string()).unwrap();
        assert_eq!(m.path(), &["@company", "rules"]);
    }

    #[test]
    fn test_typescript_module_kebab_case() {
        let m = TypeScriptModule::try_from("my-rules/auth-module".to_string()).unwrap();
        assert_eq!(m.path(), &["my-rules", "auth-module"]);
    }

    // ScopingConfig tests
    #[test]
    fn test_scoping_config_for_target() {
        let config = ScopingConfig {
            languages: LanguageScopingTyped {
                csharp: Some(CSharpNamespace::try_from("Company.Rules".to_string()).unwrap()),
                java: Some(JavaPackage::try_from("com.company.rules".to_string()).unwrap()),
                ..Default::default()
            },
        };

        assert!(config.for_target(crate::cel::Target::CSharp).is_some());
        assert!(config.for_target(crate::cel::Target::Java).is_some());
        assert!(config.for_target(crate::cel::Target::Go).is_none());
    }

    #[test]
    fn test_scoping_config_validate() {
        let config = ScopingConfig {
            languages: LanguageScopingTyped {
                csharp: Some(CSharpNamespace::try_from("Company.Rules".to_string()).unwrap()),
                ..Default::default()
            },
        };

        // Should pass for C# only
        assert!(config.validate(&[crate::cel::Target::CSharp]).is_ok());

        // Should fail for Java (not configured)
        assert!(matches!(
            config.validate(&[crate::cel::Target::Java]),
            Err(NamespaceError::Required)
        ));
    }
}
