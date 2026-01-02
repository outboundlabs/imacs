//! Code analysis — complexity metrics and issue detection
//!
//! Analyzes existing code to:
//! - Measure complexity (cyclomatic, nesting, lines)
//! - Detect issues (magic numbers, deep nesting, etc.)
//! - Recommend extraction targets

use crate::ast::*;
use serde::{Deserialize, Serialize};

/// Analyze code AST
pub fn analyze(code: &CodeAst) -> AnalysisReport {
    Analyzer::new().analyze(code)
}

/// Code analyzer
pub struct Analyzer {
    config: AnalyzerConfig,
}

/// Analyzer configuration
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    pub complexity_warn: usize,
    pub complexity_error: usize,
    pub max_nesting: usize,
    pub max_lines: usize,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            complexity_warn: 10,
            complexity_error: 20,
            max_nesting: 4,
            max_lines: 50,
        }
    }
}

/// Analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub functions: Vec<FunctionAnalysis>,
    pub overall: OverallMetrics,
    pub issues: Vec<Issue>,
    pub recommendations: Vec<Recommendation>,
}

/// Analysis of a single function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionAnalysis {
    pub name: String,
    pub metrics: FunctionMetrics,
    pub issues: Vec<Issue>,
}

/// Metrics for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetrics {
    pub lines: usize,
    pub cyclomatic_complexity: usize,
    pub max_nesting: usize,
    pub parameters: usize,
    pub decision_points: usize,
    pub return_points: usize,
}

/// Overall file metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallMetrics {
    pub total_lines: usize,
    pub total_functions: usize,
    pub avg_complexity: f32,
    pub max_complexity: usize,
    pub total_issues: usize,
}

/// An issue found in code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub kind: IssueKind,
    pub severity: Severity,
    pub line: usize,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueKind {
    HighComplexity,
    DeepNesting,
    LongFunction,
    MagicNumber,
    TooManyParams,
    MissingDefault,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Warning => write!(f, "WARN"),
            Severity::Error => write!(f, "ERROR"),
        }
    }
}

/// Recommendation for improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Recommendation {
    ExtractSpec {
        function: String,
        reason: String,
        estimated_rules: usize,
    },
    SplitFunction {
        function: String,
        reason: String,
    },
    ExtractConstant {
        line: usize,
        value: String,
        suggested_name: String,
    },
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            config: AnalyzerConfig::default(),
        }
    }

    pub fn with_config(config: AnalyzerConfig) -> Self {
        Self { config }
    }

    pub fn analyze(&self, code: &CodeAst) -> AnalysisReport {
        let mut functions = Vec::new();
        let mut all_issues = Vec::new();
        let mut recommendations = Vec::new();

        for func in &code.functions {
            let analysis = self.analyze_function(func);
            all_issues.extend(analysis.issues.clone());

            if analysis.metrics.cyclomatic_complexity > self.config.complexity_warn {
                recommendations.push(Recommendation::ExtractSpec {
                    function: func.name.clone(),
                    reason: format!(
                        "Complexity {} suggests decision logic",
                        analysis.metrics.cyclomatic_complexity
                    ),
                    estimated_rules: analysis.metrics.decision_points,
                });
            }

            if analysis.metrics.lines > self.config.max_lines {
                recommendations.push(Recommendation::SplitFunction {
                    function: func.name.clone(),
                    reason: format!(
                        "{} lines exceeds {} limit",
                        analysis.metrics.lines, self.config.max_lines
                    ),
                });
            }

            functions.push(analysis);
        }

        let total_complexity: usize = functions
            .iter()
            .map(|f| f.metrics.cyclomatic_complexity)
            .sum();
        let max_complexity = functions
            .iter()
            .map(|f| f.metrics.cyclomatic_complexity)
            .max()
            .unwrap_or(0);

        let overall = OverallMetrics {
            total_lines: functions.iter().map(|f| f.metrics.lines).sum(),
            total_functions: functions.len(),
            avg_complexity: if functions.is_empty() {
                0.0
            } else {
                total_complexity as f32 / functions.len() as f32
            },
            max_complexity,
            total_issues: all_issues.len(),
        };

        AnalysisReport {
            functions,
            overall,
            issues: all_issues,
            recommendations,
        }
    }

    fn analyze_function(&self, func: &Function) -> FunctionAnalysis {
        let mut issues = Vec::new();

        let lines = func.span.end_line.saturating_sub(func.span.start_line) + 1;
        let (complexity, max_nesting, decisions) = self.calculate_complexity(&func.body);
        let return_points = self.count_returns(&func.body);

        if complexity > self.config.complexity_error {
            issues.push(Issue {
                kind: IssueKind::HighComplexity,
                severity: Severity::Error,
                line: func.span.start_line,
                message: format!(
                    "Cyclomatic complexity {} exceeds {}",
                    complexity, self.config.complexity_error
                ),
                suggestion: Some("Extract decision logic into spec".into()),
            });
        } else if complexity > self.config.complexity_warn {
            issues.push(Issue {
                kind: IssueKind::HighComplexity,
                severity: Severity::Warning,
                line: func.span.start_line,
                message: format!("Cyclomatic complexity {} is high", complexity),
                suggestion: Some("Consider extracting complex logic".into()),
            });
        }

        if max_nesting > self.config.max_nesting {
            issues.push(Issue {
                kind: IssueKind::DeepNesting,
                severity: Severity::Warning,
                line: func.span.start_line,
                message: format!(
                    "Nesting depth {} exceeds {}",
                    max_nesting, self.config.max_nesting
                ),
                suggestion: Some("Flatten with early returns".into()),
            });
        }

        if lines > self.config.max_lines {
            issues.push(Issue {
                kind: IssueKind::LongFunction,
                severity: Severity::Warning,
                line: func.span.start_line,
                message: format!("Function has {} lines", lines),
                suggestion: Some("Split into smaller functions".into()),
            });
        }

        if func.params.len() > 5 {
            issues.push(Issue {
                kind: IssueKind::TooManyParams,
                severity: Severity::Warning,
                line: func.span.start_line,
                message: format!("Function has {} parameters", func.params.len()),
                suggestion: Some("Group into struct".into()),
            });
        }

        self.find_magic_numbers(&func.body, &mut issues);

        FunctionMetrics {
            lines,
            cyclomatic_complexity: complexity,
            max_nesting,
            parameters: func.params.len(),
            decision_points: decisions,
            return_points,
        };

        FunctionAnalysis {
            name: func.name.clone(),
            metrics: FunctionMetrics {
                lines,
                cyclomatic_complexity: complexity,
                max_nesting,
                parameters: func.params.len(),
                decision_points: decisions,
                return_points,
            },
            issues,
        }
    }

    fn calculate_complexity(&self, node: &AstNode) -> (usize, usize, usize) {
        self.calc_recursive(node, 0)
    }

    fn calc_recursive(&self, node: &AstNode, depth: usize) -> (usize, usize, usize) {
        match node {
            AstNode::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let (c1, n1, d1) = self.calc_recursive(condition, depth);
                let (c2, n2, d2) = self.calc_recursive(then_branch, depth + 1);
                let (c3, n3, d3) = else_branch
                    .as_ref()
                    .map(|e| self.calc_recursive(e, depth + 1))
                    .unwrap_or((0, 0, 0));

                (
                    1 + c1 + c2 + c3,
                    (depth + 1).max(n1).max(n2).max(n3),
                    1 + d1 + d2 + d3,
                )
            }

            AstNode::Match { arms, .. } => {
                let mut total_c = arms.len().saturating_sub(1);
                let mut max_n = depth + 1;
                let mut total_d = 1;

                for arm in arms {
                    let (c, n, d) = self.calc_recursive(&arm.body, depth + 1);
                    total_c += c;
                    max_n = max_n.max(n);
                    total_d += d;
                }

                (total_c, max_n, total_d)
            }

            AstNode::Binary {
                op: BinaryOp::And | BinaryOp::Or,
                left,
                right,
                ..
            } => {
                let (c1, n1, d1) = self.calc_recursive(left, depth);
                let (c2, n2, d2) = self.calc_recursive(right, depth);
                (1 + c1 + c2, n1.max(n2), d1 + d2)
            }

            AstNode::Block {
                statements, result, ..
            } => {
                let mut total = (0, depth, 0);
                for stmt in statements {
                    let (c, n, d) = self.calc_recursive(stmt, depth);
                    total.0 += c;
                    total.1 = total.1.max(n);
                    total.2 += d;
                }
                if let Some(r) = result {
                    let (c, n, d) = self.calc_recursive(r, depth);
                    total.0 += c;
                    total.1 = total.1.max(n);
                    total.2 += d;
                }
                total
            }

            _ => (0, depth, 0),
        }
    }

    fn count_returns(&self, node: &AstNode) -> usize {
        match node {
            AstNode::Return { .. } => 1,
            AstNode::Block {
                statements, result, ..
            } => {
                let mut count: usize = statements.iter().map(|s| self.count_returns(s)).sum();
                if let Some(r) = result {
                    count += self.count_returns(r);
                }
                count
            }
            AstNode::If {
                then_branch,
                else_branch,
                ..
            } => {
                let mut count = self.count_returns(then_branch);
                if let Some(e) = else_branch {
                    count += self.count_returns(e);
                }
                count
            }
            AstNode::Match { arms, .. } => arms.iter().map(|a| self.count_returns(&a.body)).sum(),
            _ => 0,
        }
    }

    fn find_magic_numbers(&self, node: &AstNode, issues: &mut Vec<Issue>) {
        match node {
            AstNode::Literal {
                value: LiteralValue::Int(n),
                span,
            } if *n > 1 && *n != 100 && *n != 1000 => {
                issues.push(Issue {
                    kind: IssueKind::MagicNumber,
                    severity: Severity::Info,
                    line: span.start_line,
                    message: format!("Magic number: {}", n),
                    suggestion: Some("Extract to named constant".into()),
                });
            }

            AstNode::Block {
                statements, result, ..
            } => {
                for stmt in statements {
                    self.find_magic_numbers(stmt, issues);
                }
                if let Some(r) = result {
                    self.find_magic_numbers(r, issues);
                }
            }

            AstNode::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.find_magic_numbers(condition, issues);
                self.find_magic_numbers(then_branch, issues);
                if let Some(e) = else_branch {
                    self.find_magic_numbers(e, issues);
                }
            }

            AstNode::Match { arms, .. } => {
                for arm in arms {
                    self.find_magic_numbers(&arm.body, issues);
                }
            }

            AstNode::Binary { left, right, .. } => {
                self.find_magic_numbers(left, issues);
                self.find_magic_numbers(right, issues);
            }

            _ => {}
        }
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisReport {
    pub fn to_report(&self) -> String {
        let mut out = String::new();

        out.push_str("ANALYSIS REPORT\n");
        out.push_str("═══════════════════════════════════════════════════════════════\n\n");

        out.push_str("Overall Metrics:\n");
        out.push_str(&format!("  Functions: {}\n", self.overall.total_functions));
        out.push_str(&format!("  Total lines: {}\n", self.overall.total_lines));
        out.push_str(&format!(
            "  Avg complexity: {:.1}\n",
            self.overall.avg_complexity
        ));
        out.push_str(&format!(
            "  Max complexity: {}\n",
            self.overall.max_complexity
        ));
        out.push_str(&format!("  Issues: {}\n\n", self.overall.total_issues));

        for func in &self.functions {
            out.push_str(&format!("Function: {}\n", func.name));
            out.push_str(&format!("  Lines: {}\n", func.metrics.lines));
            out.push_str(&format!(
                "  Complexity: {}\n",
                func.metrics.cyclomatic_complexity
            ));
            out.push_str(&format!("  Max nesting: {}\n", func.metrics.max_nesting));
            out.push_str(&format!(
                "  Decision points: {}\n",
                func.metrics.decision_points
            ));

            if !func.issues.is_empty() {
                out.push_str("  Issues:\n");
                for issue in &func.issues {
                    out.push_str(&format!("    [{}] {}\n", issue.severity, issue.message));
                }
            }
            out.push('\n');
        }

        if !self.recommendations.is_empty() {
            out.push_str("Recommendations:\n");
            for rec in &self.recommendations {
                match rec {
                    Recommendation::ExtractSpec {
                        function,
                        reason,
                        estimated_rules,
                    } => {
                        out.push_str(&format!(
                            "  • Extract spec from '{}': {} (~{} rules)\n",
                            function, reason, estimated_rules
                        ));
                    }
                    Recommendation::SplitFunction { function, reason } => {
                        out.push_str(&format!("  • Split '{}': {}\n", function, reason));
                    }
                    Recommendation::ExtractConstant {
                        value,
                        suggested_name,
                        ..
                    } => {
                        out.push_str(&format!("  • Extract {} as {}\n", value, suggested_name));
                    }
                }
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_rust;

    #[test]
    fn test_analyze_simple() {
        let code = r#"
fn simple(x: bool) -> i32 {
    if x { 1 } else { 0 }
}
"#;
        let ast = parse_rust(code).unwrap();
        let report = analyze(&ast);

        assert_eq!(report.functions.len(), 1);
        assert!(report.functions[0].metrics.cyclomatic_complexity >= 1);
    }

    #[test]
    fn test_analyze_complex() {
        let code = r#"
fn complex(a: bool, b: bool, c: bool) -> i32 {
    if a {
        if b {
            if c { 1 } else { 2 }
        } else { 3 }
    } else { 4 }
}
"#;
        let ast = parse_rust(code).unwrap();
        let report = analyze(&ast);

        assert_eq!(report.functions.len(), 1);
        assert!(report.functions[0].metrics.cyclomatic_complexity >= 3);
        assert!(report.functions[0].metrics.max_nesting >= 3);
    }

    #[test]
    fn test_analyze_match() {
        let code = r#"
fn with_match(x: i32) -> &'static str {
    match x {
        1 => "one",
        2 => "two",
        3 => "three",
        _ => "other",
    }
}
"#;
        let ast = parse_rust(code).unwrap();
        let report = analyze(&ast);

        assert_eq!(report.functions.len(), 1);
        assert!(report.functions[0].metrics.decision_points >= 1);
    }
}
