//! AST types for code representation
//!
//! Language-agnostic AST that captures decision logic structure.
//! Parsed from source code via tree-sitter.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parsed code AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAst {
    /// Source language
    pub language: Language,

    /// Top-level functions
    pub functions: Vec<Function>,

    /// Hash of source for change detection
    pub source_hash: String,
}

/// Source language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
    CSharp,
    Java,
    Unknown,
}

/// Result of parsing with diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    /// The parsed AST
    pub ast: CodeAst,
    /// Diagnostics collected during parsing
    pub diagnostics: ParseDiagnostics,
}

/// Diagnostics collected during parsing
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParseDiagnostics {
    /// Nodes that couldn't be parsed into known types
    pub unknown_nodes: Vec<UnknownNodeInfo>,
    /// Syntax errors detected by tree-sitter
    pub syntax_errors: Vec<SyntaxErrorInfo>,
}

impl ParseDiagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_issues(&self) -> bool {
        !self.unknown_nodes.is_empty() || !self.syntax_errors.is_empty()
    }

    pub fn unknown_count(&self) -> usize {
        self.unknown_nodes.len()
    }

    pub fn error_count(&self) -> usize {
        self.syntax_errors.len()
    }
}

/// Information about an unknown/unparsed node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownNodeInfo {
    /// The tree-sitter node kind (e.g., "macro_invocation", "type_cast_expression")
    pub kind: String,
    /// Source location
    pub span: Span,
    /// The source text that couldn't be parsed
    pub source_text: String,
}

/// Information about a syntax error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxErrorInfo {
    /// Error message
    pub message: String,
    /// Source location
    pub span: Span,
    /// The source text with the error
    pub source_text: String,
}

/// A function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// Function name
    pub name: String,

    /// Parameters
    pub params: Vec<Parameter>,

    /// Return type (as string)
    pub return_type: Option<String>,

    /// Function body
    pub body: AstNode,

    /// Source location
    pub span: Span,
}

/// Function parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub typ: String,
}

/// Source location
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

/// AST node representing code structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AstNode {
    /// Literal value
    Literal { value: LiteralValue, span: Span },

    /// Variable reference
    Var { name: String, span: Span },

    /// Binary operation
    Binary {
        op: BinaryOp,
        left: Box<AstNode>,
        right: Box<AstNode>,
        span: Span,
    },

    /// Unary operation
    Unary {
        op: UnaryOp,
        operand: Box<AstNode>,
        span: Span,
    },

    /// If expression
    If {
        condition: Box<AstNode>,
        then_branch: Box<AstNode>,
        else_branch: Option<Box<AstNode>>,
        span: Span,
    },

    /// Match expression
    Match {
        scrutinee: Box<AstNode>,
        arms: Vec<MatchArm>,
        span: Span,
    },

    /// Block of statements
    Block {
        statements: Vec<AstNode>,
        result: Option<Box<AstNode>>,
        span: Span,
    },

    /// Return statement
    Return {
        value: Option<Box<AstNode>>,
        span: Span,
    },

    /// Let binding
    Let {
        name: String,
        value: Box<AstNode>,
        span: Span,
    },

    /// Function call
    Call {
        function: String,
        args: Vec<AstNode>,
        span: Span,
    },

    /// Field access
    Field {
        object: Box<AstNode>,
        field: String,
        span: Span,
    },

    /// Index access
    Index {
        object: Box<AstNode>,
        index: Box<AstNode>,
        span: Span,
    },

    /// Tuple expression
    Tuple { elements: Vec<AstNode>, span: Span },

    /// Array/list expression
    Array { elements: Vec<AstNode>, span: Span },

    /// For loop (counter-based)
    For {
        counter: String,
        start: Box<AstNode>,
        end: Box<AstNode>,
        body: Box<AstNode>,
        span: Span,
    },

    /// For-each loop (iteration)
    ForEach {
        item: String,
        index: Option<String>,
        collection: Box<AstNode>,
        body: Box<AstNode>,
        span: Span,
    },

    /// While loop
    While {
        condition: Box<AstNode>,
        body: Box<AstNode>,
        span: Span,
    },

    /// Try/catch/finally
    Try {
        try_block: Box<AstNode>,
        catch_var: Option<String>,
        catch_block: Option<Box<AstNode>>,
        finally_block: Option<Box<AstNode>>,
        span: Span,
    },

    /// Assignment expression
    Assign {
        target: Box<AstNode>,
        value: Box<AstNode>,
        span: Span,
    },

    /// Await expression (async)
    Await { expr: Box<AstNode>, span: Span },

    /// Closure/lambda expression
    Closure {
        params: Vec<String>,
        body: Box<AstNode>,
        span: Span,
    },

    /// Macro call (e.g., println!(...))
    MacroCall {
        name: String,
        args: String, // Raw arguments as string (macros have arbitrary syntax)
        span: Span,
    },

    /// Reference expression (e.g., &x, &mut x)
    Ref {
        mutable: bool,
        expr: Box<AstNode>,
        span: Span,
    },

    /// Type cast expression (e.g., x as i32)
    Cast {
        expr: Box<AstNode>,
        target_type: String,
        span: Span,
    },

    /// Syntax error from parser
    SyntaxError {
        message: String,
        source_text: String,
        span: Span,
    },

    /// Unknown/unparsed node
    Unknown { kind: String, span: Span },
}

impl AstNode {
    /// Get the span of this node
    pub fn span(&self) -> Span {
        match self {
            AstNode::Literal { span, .. } => *span,
            AstNode::Var { span, .. } => *span,
            AstNode::Binary { span, .. } => *span,
            AstNode::Unary { span, .. } => *span,
            AstNode::If { span, .. } => *span,
            AstNode::Match { span, .. } => *span,
            AstNode::Block { span, .. } => *span,
            AstNode::Return { span, .. } => *span,
            AstNode::Let { span, .. } => *span,
            AstNode::Call { span, .. } => *span,
            AstNode::Field { span, .. } => *span,
            AstNode::Index { span, .. } => *span,
            AstNode::Tuple { span, .. } => *span,
            AstNode::Array { span, .. } => *span,
            AstNode::For { span, .. } => *span,
            AstNode::ForEach { span, .. } => *span,
            AstNode::While { span, .. } => *span,
            AstNode::Try { span, .. } => *span,
            AstNode::Assign { span, .. } => *span,
            AstNode::Await { span, .. } => *span,
            AstNode::Closure { span, .. } => *span,
            AstNode::MacroCall { span, .. } => *span,
            AstNode::Ref { span, .. } => *span,
            AstNode::Cast { span, .. } => *span,
            AstNode::SyntaxError { span, .. } => *span,
            AstNode::Unknown { span, .. } => *span,
        }
    }
}

/// Literal values
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LiteralValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Unit,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical
    And,
    Or,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl std::fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mod => write!(f, "%"),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Ne => write!(f, "!="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Le => write!(f, "<="),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Ge => write!(f, ">="),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
            BinaryOp::BitAnd => write!(f, "&"),
            BinaryOp::BitOr => write!(f, "|"),
            BinaryOp::BitXor => write!(f, "^"),
            BinaryOp::Shl => write!(f, "<<"),
            BinaryOp::Shr => write!(f, ">>"),
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

impl std::fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::BitNot => write!(f, "~"),
        }
    }
}

/// Match arm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArm {
    /// Pattern to match
    pub pattern: Pattern,

    /// Optional guard condition
    pub guard: Option<AstNode>,

    /// Body expression
    pub body: AstNode,

    /// Source location
    pub span: Span,
}

/// Match patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Pattern {
    /// Wildcard `_`
    Wildcard,

    /// Variable binding
    Binding(String),

    /// Literal pattern
    Literal(LiteralValue),

    /// Tuple pattern
    Tuple(Vec<Pattern>),

    /// Constructor pattern (enum variant)
    Constructor { name: String, fields: Vec<Pattern> },

    /// Or pattern
    Or(Vec<Pattern>),

    /// Rest pattern `..`
    Rest,
}

impl Pattern {
    /// Check if this is a wildcard or binding (catches anything)
    pub fn is_catch_all(&self) -> bool {
        matches!(self, Pattern::Wildcard | Pattern::Binding(_))
    }

    /// Extract literal value if this is a literal pattern
    pub fn as_literal(&self) -> Option<&LiteralValue> {
        match self {
            Pattern::Literal(v) => Some(v),
            _ => None,
        }
    }
}

impl CodeAst {
    /// Find function by name
    pub fn get_function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Get all function names
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.iter().map(|f| f.name.as_str()).collect()
    }
}
