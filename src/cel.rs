//! CEL (Common Expression Language) parsing, evaluation, and compilation
//!
//! CEL is used in specs to express conditions. This module:
//! - Parses CEL strings to AST (using cel-parser crate)
//! - Evaluates CEL expressions at runtime (using cel-interpreter)
//! - Compiles CEL AST to target languages (Rust, TypeScript, Python, etc.)
//!
//! CEL evaluation is used for validation and testing.
//! Generated code has no CEL dependency - only the compiled target language code.

use crate::error::{Error, Result};
use std::collections::HashMap;

// cel-parser for AST-based compilation to target languages
pub use cel_parser::Expression as CelExpr;
use cel_parser::{parse, ArithmeticOp, Atom, Member, RelationOp, UnaryOp as CelUnaryOp};

// cel-interpreter for runtime evaluation
use cel_interpreter::{Context, Program, Value};

/// Target language for CEL compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Rust,
    TypeScript,
    Python,
    CSharp,
    Java,
    Go,
}

/// CEL compiler - parses, evaluates, and renders to target languages
pub struct CelCompiler;

/// Re-export cel-interpreter Value for use in evaluation
pub use cel_interpreter::Value as CelValue;

impl CelCompiler {
    /// Parse CEL expression string to AST (using cel-parser)
    pub fn parse(expr: &str) -> Result<CelExpr> {
        parse(expr).map_err(|e| Error::CelParse(format!("{}: {:?}", expr, e)))
    }

    /// Check if a string is a valid CEL expression
    /// Uses cel-parser for validation (cel-interpreter's parser panics on syntax errors)
    pub fn is_valid(expr: &str) -> bool {
        parse(expr).is_ok()
    }

    /// Evaluate a CEL expression with the given variable bindings
    /// Returns the evaluated Value
    pub fn eval(expr: &str, vars: &HashMap<String, CelValue>) -> Result<CelValue> {
        let program =
            Program::compile(expr).map_err(|e| Error::CelParse(format!("{}: {:?}", expr, e)))?;

        let mut context = Context::default();
        for (name, value) in vars {
            context.add_variable_from_value(name.clone(), value.clone());
        }

        program
            .execute(&context)
            .map_err(|e| Error::CelEval(format!("{}: {:?}", expr, e)))
    }

    /// Evaluate a CEL expression and return result as bool
    pub fn eval_bool(expr: &str, vars: &HashMap<String, CelValue>) -> Result<bool> {
        let result = Self::eval(expr, vars)?;
        match result {
            Value::Bool(b) => Ok(b),
            other => Err(Error::CelEval(format!(
                "Expected bool result, got {:?}",
                other
            ))),
        }
    }

    /// Evaluate a CEL expression and return result as i64
    pub fn eval_int(expr: &str, vars: &HashMap<String, CelValue>) -> Result<i64> {
        let result = Self::eval(expr, vars)?;
        match result {
            Value::Int(i) => Ok(i),
            Value::UInt(u) => Ok(u as i64),
            other => Err(Error::CelEval(format!(
                "Expected int result, got {:?}",
                other
            ))),
        }
    }

    /// Evaluate a CEL expression and return result as f64
    pub fn eval_float(expr: &str, vars: &HashMap<String, CelValue>) -> Result<f64> {
        let result = Self::eval(expr, vars)?;
        match result {
            Value::Float(f) => Ok(f),
            Value::Int(i) => Ok(i as f64),
            Value::UInt(u) => Ok(u as f64),
            other => Err(Error::CelEval(format!(
                "Expected float result, got {:?}",
                other
            ))),
        }
    }

    /// Evaluate a CEL expression and return result as String
    pub fn eval_string(expr: &str, vars: &HashMap<String, CelValue>) -> Result<String> {
        let result = Self::eval(expr, vars)?;
        match result {
            Value::String(s) => Ok(s.to_string()),
            other => Err(Error::CelEval(format!(
                "Expected string result, got {:?}",
                other
            ))),
        }
    }

    /// PY-4: Extract all variable names referenced in a CEL expression
    pub fn extract_variables(expr: &str) -> Result<Vec<String>> {
        let ast = Self::parse(expr)?;
        let mut vars = Vec::new();
        Self::collect_variables(&ast, &mut vars);
        vars.sort();
        vars.dedup();
        Ok(vars)
    }

    /// Recursively collect variable names from CEL AST
    fn collect_variables(expr: &CelExpr, vars: &mut Vec<String>) {
        match expr {
            CelExpr::Ident(name) => {
                // Skip built-in values
                let name_str = name.as_str();
                if name_str != "true" && name_str != "false" && name_str != "null" {
                    vars.push(name.to_string());
                }
            }
            CelExpr::Member(base, member) => {
                // Only collect the base variable, not nested members
                if let CelExpr::Ident(name) = base.as_ref() {
                    let name_str = name.as_str();
                    if name_str != "true" && name_str != "false" && name_str != "null" {
                        vars.push(name.to_string());
                    }
                } else {
                    Self::collect_variables(base, vars);
                }
                // Check for function call arguments
                if let Member::FunctionCall(args) = member.as_ref() {
                    for arg in args {
                        Self::collect_variables(arg, vars);
                    }
                }
            }
            CelExpr::Arithmetic(left, _, right) => {
                Self::collect_variables(left, vars);
                Self::collect_variables(right, vars);
            }
            CelExpr::Relation(left, _, right) => {
                Self::collect_variables(left, vars);
                Self::collect_variables(right, vars);
            }
            CelExpr::Unary(_, inner) => {
                Self::collect_variables(inner, vars);
            }
            CelExpr::Or(left, right) => {
                Self::collect_variables(left, vars);
                Self::collect_variables(right, vars);
            }
            CelExpr::And(left, right) => {
                Self::collect_variables(left, vars);
                Self::collect_variables(right, vars);
            }
            CelExpr::Ternary(cond, true_branch, false_branch) => {
                Self::collect_variables(cond, vars);
                Self::collect_variables(true_branch, vars);
                Self::collect_variables(false_branch, vars);
            }
            CelExpr::List(items) => {
                for item in items {
                    Self::collect_variables(item, vars);
                }
            }
            CelExpr::Map(entries) => {
                for (_, value) in entries {
                    Self::collect_variables(value, vars);
                }
            }
            CelExpr::Atom(_) => {
                // Atoms are literals, no variables
            }
        }
    }

    /// PY-4: Validate that all variables in a CEL expression are defined
    pub fn validate_variables(expr: &str, valid_names: &[&str]) -> Result<()> {
        let referenced = Self::extract_variables(expr)?;
        let valid_set: std::collections::HashSet<_> = valid_names.iter().copied().collect();

        for var in &referenced {
            if !valid_set.contains(var.as_str()) {
                return Err(Error::CelParse(format!(
                    "Undefined variable '{}' in expression: {}",
                    var, expr
                )));
            }
        }
        Ok(())
    }

    /// Compile CEL expression to target language
    pub fn compile(expr: &str, target: Target) -> Result<String> {
        let ast = Self::parse(expr)?;
        Ok(Self::render(&ast, target))
    }

    /// Render CEL AST to Rust
    pub fn to_rust(expr: &CelExpr) -> String {
        Self::render(expr, Target::Rust)
    }

    /// Render CEL AST to TypeScript
    pub fn to_typescript(expr: &CelExpr) -> String {
        Self::render(expr, Target::TypeScript)
    }

    /// Render CEL AST to Python
    pub fn to_python(expr: &CelExpr) -> String {
        Self::render(expr, Target::Python)
    }

    /// Render CEL AST to C#
    pub fn to_csharp(expr: &CelExpr) -> String {
        Self::render(expr, Target::CSharp)
    }

    /// Render CEL AST to Java
    pub fn to_java(expr: &CelExpr) -> String {
        Self::render(expr, Target::Java)
    }

    /// Render CEL AST to Go
    pub fn to_go(expr: &CelExpr) -> String {
        Self::render(expr, Target::Go)
    }

    /// Render CEL AST to target language
    pub fn render(expr: &CelExpr, target: Target) -> String {
        match expr {
            CelExpr::Atom(atom) => Self::render_atom(atom, target),

            CelExpr::Ident(name) => name.to_string(),

            CelExpr::Member(base, member) => {
                // Check if this is a function call like size(x), has(x), etc.
                if let (CelExpr::Ident(func_name), Member::FunctionCall(args)) =
                    (base.as_ref(), member.as_ref())
                {
                    return Self::render_function(func_name, args, target);
                }
                let base_str = Self::render(base, target);
                Self::render_member(&base_str, member, target)
            }

            CelExpr::Arithmetic(left, op, right) => {
                let l = Self::render(left, target);
                let r = Self::render(right, target);
                let op_str = Self::arith_op(op.clone());
                format!("({} {} {})", l, op_str, r)
            }

            CelExpr::Relation(left, op, right) => {
                Self::render_relation(op.clone(), left, right, target)
            }

            CelExpr::Unary(op, inner) => {
                let inner_str = Self::render(inner, target);
                match (op, target) {
                    (CelUnaryOp::Not, Target::Python) => format!("(not {})", inner_str),
                    (CelUnaryOp::Not, _) => format!("(!{})", inner_str),
                    (CelUnaryOp::DoubleNot, _) => format!("(!!{})", inner_str),
                    (CelUnaryOp::Minus, _) => format!("(-{})", inner_str),
                    (CelUnaryOp::DoubleMinus, _) => format!("(--{})", inner_str),
                }
            }

            CelExpr::And(left, right) => {
                let l = Self::render(left, target);
                let r = Self::render(right, target);
                match target {
                    Target::Python => format!("({} and {})", l, r),
                    _ => format!("({} && {})", l, r),
                }
            }

            CelExpr::Or(left, right) => {
                let l = Self::render(left, target);
                let r = Self::render(right, target);
                match target {
                    Target::Python => format!("({} or {})", l, r),
                    _ => format!("({} || {})", l, r),
                }
            }

            CelExpr::Ternary(cond, if_true, if_false) => {
                let c = Self::render(cond, target);
                let t = Self::render(if_true, target);
                let f = Self::render(if_false, target);
                match target {
                    Target::Python => format!("({} if {} else {})", t, c, f),
                    _ => format!("({} ? {} : {})", c, t, f),
                }
            }

            CelExpr::List(items) => {
                let items_str: Vec<_> = items.iter().map(|i| Self::render(i, target)).collect();
                format!("[{}]", items_str.join(", "))
            }

            CelExpr::Map(entries) => {
                let entries_str: Vec<_> = entries
                    .iter()
                    .map(|(k, v)| {
                        let key = Self::render(k, target);
                        let val = Self::render(v, target);
                        format!("{}: {}", key, val)
                    })
                    .collect();
                match target {
                    Target::Rust => {
                        format!("HashMap::from([{}])", entries_str.join(", "))
                    }
                    _ => format!("{{{}}}", entries_str.join(", ")),
                }
            }
        }
    }

    fn render_atom(atom: &Atom, target: Target) -> String {
        match atom {
            Atom::Int(i) => i.to_string(),
            Atom::UInt(u) => u.to_string(),
            Atom::Float(f) => format!("{:?}", f), // Ensure decimal point
            Atom::String(s) => format!("\"{}\"", s.escape_default()),
            Atom::Bytes(b) => format!("{:?}", b),
            Atom::Bool(b) => match target {
                Target::Python => {
                    if *b {
                        "True".to_string()
                    } else {
                        "False".to_string()
                    }
                }
                _ => b.to_string(),
            },
            Atom::Null => match target {
                Target::Python => "None".to_string(),
                Target::TypeScript | Target::CSharp | Target::Java | Target::Go => {
                    "null".to_string()
                }
                Target::Rust => "None".to_string(),
            },
        }
    }

    fn render_member(base: &str, member: &Member, target: Target) -> String {
        match member {
            Member::Attribute(attr) => format!("{}.{}", base, attr),

            Member::Index(idx) => {
                let idx_str = Self::render(idx, target);
                format!("{}[{}]", base, idx_str)
            }

            Member::Fields(fields) => {
                // Struct construction
                let fields_str: Vec<_> = fields
                    .iter()
                    .map(|(k, v)| {
                        let val = Self::render(v, target);
                        format!("{}: {}", k, val)
                    })
                    .collect();
                format!("{} {{ {} }}", base, fields_str.join(", "))
            }

            Member::FunctionCall(args) => {
                // Method call: base.method(args)
                // The base already includes the method name from attribute access
                let args_str: Vec<_> = args.iter().map(|a| Self::render(a, target)).collect();
                format!("{}({})", base, args_str.join(", "))
            }
        }
    }

    fn render_relation(op: RelationOp, left: &CelExpr, right: &CelExpr, target: Target) -> String {
        let l = Self::render(left, target);
        let r = Self::render(right, target);

        match op {
            RelationOp::In => {
                // x in [a, b, c]
                match target {
                    Target::Rust => format!("[{}].contains(&{})", r, l),
                    Target::TypeScript => format!("{}.includes({})", r, l),
                    Target::Python => format!("({} in {})", l, r),
                    Target::CSharp | Target::Java => format!("{}.contains({})", r, l),
                    Target::Go => format!("contains({}, {})", r, l),
                }
            }
            _ => {
                let op_str = Self::relation_op(op, target);
                format!("({} {} {})", l, op_str, r)
            }
        }
    }

    fn arith_op(op: ArithmeticOp) -> &'static str {
        match op {
            ArithmeticOp::Add => "+",
            ArithmeticOp::Subtract => "-",
            ArithmeticOp::Multiply => "*",
            ArithmeticOp::Divide => "/",
            ArithmeticOp::Modulus => "%",
        }
    }

    fn relation_op(op: RelationOp, target: Target) -> &'static str {
        match (op, target) {
            (RelationOp::Equals, Target::TypeScript) => "===",
            (RelationOp::Equals, _) => "==",
            (RelationOp::NotEquals, Target::TypeScript) => "!==",
            (RelationOp::NotEquals, _) => "!=",
            (RelationOp::LessThan, _) => "<",
            (RelationOp::LessThanEq, _) => "<=",
            (RelationOp::GreaterThan, _) => ">",
            (RelationOp::GreaterThanEq, _) => ">=",
            (RelationOp::In, _) => "in", // Handled specially
        }
    }

    fn render_function(name: &str, args: &[CelExpr], target: Target) -> String {
        let args_rendered: Vec<_> = args.iter().map(|a| Self::render(a, target)).collect();

        match (name, target) {
            // size() function
            ("size", Target::Rust) => format!("{}.len()", args_rendered[0]),
            ("size", Target::TypeScript) => format!("{}.length", args_rendered[0]),
            ("size", Target::Python) => format!("len({})", args_rendered[0]),
            ("size", Target::CSharp | Target::Java) => format!("{}.size()", args_rendered[0]),
            ("size", Target::Go) => format!("len({})", args_rendered[0]),

            // has() function
            ("has", Target::Rust) => format!("{}.is_some()", args_rendered[0]),
            ("has", Target::TypeScript) => format!("({} !== undefined)", args_rendered[0]),
            ("has", Target::Python) => format!("({} is not None)", args_rendered[0]),
            ("has", Target::CSharp | Target::Java) => format!("({} != null)", args_rendered[0]),
            ("has", Target::Go) => format!("({} != nil)", args_rendered[0]),

            // type() function
            ("type", Target::Rust) => format!("type_of({})", args_rendered[0]),
            ("type", Target::TypeScript) => format!("typeof {}", args_rendered[0]),
            ("type", Target::Python) => format!("type({})", args_rendered[0]),
            ("type", Target::CSharp) => format!("{}.GetType()", args_rendered[0]),
            ("type", Target::Java) => format!("{}.getClass()", args_rendered[0]),
            ("type", Target::Go) => format!("reflect.TypeOf({})", args_rendered[0]),

            // string functions
            ("contains", _) if args.len() >= 2 => {
                format!("{}.contains({})", args_rendered[0], args_rendered[1])
            }
            ("startsWith", Target::Rust) => {
                format!("{}.starts_with({})", args_rendered[0], args_rendered[1])
            }
            ("startsWith", Target::Python) => {
                format!("{}.startswith({})", args_rendered[0], args_rendered[1])
            }
            ("startsWith", Target::TypeScript | Target::CSharp | Target::Java) => {
                format!("{}.startsWith({})", args_rendered[0], args_rendered[1])
            }
            ("startsWith", Target::Go) => {
                format!(
                    "strings.HasPrefix({}, {})",
                    args_rendered[0], args_rendered[1]
                )
            }
            ("endsWith", Target::Rust) => {
                format!("{}.ends_with({})", args_rendered[0], args_rendered[1])
            }
            ("endsWith", Target::Python) => {
                format!("{}.endswith({})", args_rendered[0], args_rendered[1])
            }
            ("endsWith", Target::TypeScript | Target::CSharp | Target::Java) => {
                format!("{}.endsWith({})", args_rendered[0], args_rendered[1])
            }
            ("endsWith", Target::Go) => {
                format!(
                    "strings.HasSuffix({}, {})",
                    args_rendered[0], args_rendered[1]
                )
            }
            ("matches", Target::Rust) => {
                format!(
                    "Regex::new({}).unwrap().is_match({})",
                    args_rendered[1], args_rendered[0]
                )
            }
            ("matches", Target::Python) => {
                format!("re.match({}, {})", args_rendered[1], args_rendered[0])
            }
            ("matches", Target::TypeScript) => {
                format!("{}.match({})", args_rendered[0], args_rendered[1])
            }
            ("matches", Target::CSharp) => {
                format!("Regex.IsMatch({}, {})", args_rendered[0], args_rendered[1])
            }
            ("matches", Target::Java) => {
                format!("{}.matches({})", args_rendered[0], args_rendered[1])
            }
            ("matches", Target::Go) => {
                format!(
                    "regexp.MatchString({}, {})",
                    args_rendered[1], args_rendered[0]
                )
            }

            // int/float conversion
            ("int", Target::Rust) => format!("{} as i64", args_rendered[0]),
            ("int", Target::TypeScript) => format!("parseInt({})", args_rendered[0]),
            ("int", Target::Python) => format!("int({})", args_rendered[0]),
            ("int", Target::CSharp) => format!("(long){}", args_rendered[0]),
            ("int", Target::Java) => format!("(long){}", args_rendered[0]),
            ("int", Target::Go) => format!("int64({})", args_rendered[0]),

            ("double" | "float", Target::Rust) => format!("{} as f64", args_rendered[0]),
            ("double" | "float", Target::TypeScript) => format!("parseFloat({})", args_rendered[0]),
            ("double" | "float", Target::Python) => format!("float({})", args_rendered[0]),
            ("double" | "float", Target::CSharp | Target::Java) => {
                format!("(double){}", args_rendered[0])
            }
            ("double" | "float", Target::Go) => format!("float64({})", args_rendered[0]),

            // string conversion
            ("string", Target::Rust) => format!("{}.to_string()", args_rendered[0]),
            ("string", Target::TypeScript) => format!("String({})", args_rendered[0]),
            ("string", Target::Python) => format!("str({})", args_rendered[0]),
            ("string", Target::CSharp | Target::Java) => format!("{}.toString()", args_rendered[0]),
            ("string", Target::Go) => format!("fmt.Sprintf(\"%v\", {})", args_rendered[0]),

            // Default: preserve as function call
            _ => format!("{}({})", name, args_rendered.join(", ")),
        }
    }
}

/// Render macros for comprehensions
impl CelCompiler {
    /// Render list.all(x, predicate)
    pub fn render_all(list: &str, var: &str, predicate: &CelExpr, target: Target) -> String {
        let pred = Self::render(predicate, target);
        match target {
            Target::Rust => format!("{}.iter().all(|{}| {})", list, var, pred),
            Target::TypeScript => format!("{}.every({} => {})", list, var, pred),
            Target::Python => format!("all({} for {} in {})", pred, var, list),
            Target::CSharp => format!("{}.All({} => {})", list, var, pred),
            Target::Java => format!("{}.stream().allMatch({} -> {})", list, var, pred),
            Target::Go => format!("all({}, func({} T) bool {{ return {} }})", list, var, pred),
        }
    }

    /// Render list.exists(x, predicate)
    pub fn render_exists(list: &str, var: &str, predicate: &CelExpr, target: Target) -> String {
        let pred = Self::render(predicate, target);
        match target {
            Target::Rust => format!("{}.iter().any(|{}| {})", list, var, pred),
            Target::TypeScript => format!("{}.some({} => {})", list, var, pred),
            Target::Python => format!("any({} for {} in {})", pred, var, list),
            Target::CSharp => format!("{}.Any({} => {})", list, var, pred),
            Target::Java => format!("{}.stream().anyMatch({} -> {})", list, var, pred),
            Target::Go => format!("any({}, func({} T) bool {{ return {} }})", list, var, pred),
        }
    }

    /// Render list.map(x, transform)
    pub fn render_map(list: &str, var: &str, transform: &CelExpr, target: Target) -> String {
        let trans = Self::render(transform, target);
        match target {
            Target::Rust => format!(
                "{}.iter().map(|{}| {}).collect::<Vec<_>>()",
                list, var, trans
            ),
            Target::TypeScript => format!("{}.map({} => {})", list, var, trans),
            Target::Python => format!("[{} for {} in {}]", trans, var, list),
            Target::CSharp => format!("{}.Select({} => {}).ToList()", list, var, trans),
            Target::Java => format!(
                "{}.stream().map({} -> {}).collect(Collectors.toList())",
                list, var, trans
            ),
            Target::Go => format!(
                "mapSlice({}, func({} T) R {{ return {} }})",
                list, var, trans
            ),
        }
    }

    /// Render list.filter(x, predicate)
    pub fn render_filter(list: &str, var: &str, predicate: &CelExpr, target: Target) -> String {
        let pred = Self::render(predicate, target);
        match target {
            Target::Rust => format!(
                "{}.iter().filter(|{}| {}).collect::<Vec<_>>()",
                list, var, pred
            ),
            Target::TypeScript => format!("{}.filter({} => {})", list, var, pred),
            Target::Python => format!("[{} for {} in {} if {}]", var, var, list, pred),
            Target::CSharp => format!("{}.Where({} => {}).ToList()", list, var, pred),
            Target::Java => format!(
                "{}.stream().filter({} -> {}).collect(Collectors.toList())",
                list, var, pred
            ),
            Target::Go => format!(
                "filter({}, func({} T) bool {{ return {} }})",
                list, var, pred
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_comparison() {
        let rust = CelCompiler::compile("amount > 1000", Target::Rust).unwrap();
        assert!(rust.contains("amount") && rust.contains(">") && rust.contains("1000"));
    }

    #[test]
    fn test_logical_and() {
        let rust = CelCompiler::compile("a && b", Target::Rust).unwrap();
        let python = CelCompiler::compile("a && b", Target::Python).unwrap();

        assert!(rust.contains("&&"));
        assert!(python.contains("and"));
    }

    #[test]
    fn test_negation() {
        let rust = CelCompiler::compile("!verified", Target::Rust).unwrap();
        let python = CelCompiler::compile("!verified", Target::Python).unwrap();

        assert!(rust.contains("!"));
        assert!(python.contains("not"));
    }

    #[test]
    fn test_in_operator() {
        let rust = CelCompiler::compile("x in [1, 2, 3]", Target::Rust).unwrap();
        let ts = CelCompiler::compile("x in [1, 2, 3]", Target::TypeScript).unwrap();
        let py = CelCompiler::compile("x in [1, 2, 3]", Target::Python).unwrap();

        assert!(rust.contains(".contains("));
        assert!(ts.contains(".includes("));
        assert!(py.contains(" in "));
    }

    #[test]
    fn test_boolean_literals() {
        let py_true = CelCompiler::compile("true", Target::Python).unwrap();
        let py_false = CelCompiler::compile("false", Target::Python).unwrap();

        assert!(py_true.contains("True"));
        assert!(py_false.contains("False"));
    }

    #[test]
    fn test_ternary() {
        let rust = CelCompiler::compile("x > 0 ? 1 : 0", Target::Rust).unwrap();
        let python = CelCompiler::compile("x > 0 ? 1 : 0", Target::Python).unwrap();

        assert!(rust.contains("?"));
        assert!(python.contains("if") && python.contains("else"));
    }

    #[test]
    fn test_member_access() {
        let result = CelCompiler::compile("user.account.verified", Target::Rust).unwrap();
        assert!(result.contains("user.account.verified"));
    }

    #[test]
    fn test_function_size() {
        let rust = CelCompiler::compile("size(items)", Target::Rust).unwrap();
        let ts = CelCompiler::compile("size(items)", Target::TypeScript).unwrap();
        let py = CelCompiler::compile("size(items)", Target::Python).unwrap();

        assert!(rust.contains(".len()"));
        assert!(ts.contains(".length"));
        assert!(py.contains("len("));
    }

    #[test]
    fn test_complex_expression() {
        let expr = "amount > 1000 && !verified && status in [\"pending\", \"review\"]";
        let rust = CelCompiler::compile(expr, Target::Rust).unwrap();

        assert!(rust.contains("&&"));
        assert!(rust.contains("!"));
        assert!(rust.contains(".contains("));
    }

    // ==================== Evaluation Tests ====================

    #[test]
    fn test_is_valid() {
        assert!(CelCompiler::is_valid("x > 10"));
        assert!(CelCompiler::is_valid("a && b || c"));
        assert!(CelCompiler::is_valid("size(items) > 0"));
        // Invalid expressions
        assert!(!CelCompiler::is_valid("x >>")); // syntax error
        assert!(!CelCompiler::is_valid("&&")); // missing operands
    }

    #[test]
    fn test_eval_bool_simple() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Int(10));

        let result = CelCompiler::eval_bool("x > 5", &vars).unwrap();
        assert!(result);

        let result = CelCompiler::eval_bool("x < 5", &vars).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_eval_bool_string_comparison() {
        let mut vars = HashMap::new();
        vars.insert("role".to_string(), Value::from("admin"));

        let result = CelCompiler::eval_bool("role == \"admin\"", &vars).unwrap();
        assert!(result);

        let result = CelCompiler::eval_bool("role == \"user\"", &vars).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_eval_bool_logical() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Value::Bool(true));
        vars.insert("b".to_string(), Value::Bool(false));

        let result = CelCompiler::eval_bool("a && b", &vars).unwrap();
        assert!(!result);

        let result = CelCompiler::eval_bool("a || b", &vars).unwrap();
        assert!(result);

        let result = CelCompiler::eval_bool("!b", &vars).unwrap();
        assert!(result);
    }

    #[test]
    fn test_eval_int() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Int(10));
        vars.insert("y".to_string(), Value::Int(5));

        let result = CelCompiler::eval_int("x + y", &vars).unwrap();
        assert_eq!(result, 15);

        let result = CelCompiler::eval_int("x * y", &vars).unwrap();
        assert_eq!(result, 50);
    }

    #[test]
    fn test_eval_float() {
        let mut vars = HashMap::new();
        vars.insert("weight".to_string(), Value::Float(10.5));
        vars.insert("rate".to_string(), Value::Float(2.0));

        let result = CelCompiler::eval_float("weight * rate", &vars).unwrap();
        assert!((result - 21.0).abs() < 0.001);
    }

    #[test]
    fn test_eval_complex_condition() {
        let mut vars = HashMap::new();
        vars.insert("role".to_string(), Value::from("member"));
        vars.insert("verified".to_string(), Value::Bool(true));
        vars.insert("level".to_string(), Value::Int(50));

        // role == "member" && verified && level >= 50
        let result =
            CelCompiler::eval_bool("role == \"member\" && verified && level >= 50", &vars).unwrap();
        assert!(result);

        // Change one condition
        vars.insert("verified".to_string(), Value::Bool(false));
        let result =
            CelCompiler::eval_bool("role == \"member\" && verified && level >= 50", &vars).unwrap();
        assert!(!result);
    }
}
