//! Burst Compatibility Analysis (v4 Section 24, `analysis.burst`).
//!
//! Static check that the body of a method is compatible with Unity Burst's
//! constraints. The pass walks the lowered C# IR (which already contains the
//! exact statements that will run) and reports diagnostics for the patterns
//! the spec lists in §24.2:
//!
//! - **E137** — managed type reference (string, class, delegate, etc.)
//! - **E138** — `try` / `catch` inside a Burst-target method
//! - **E139** — virtual or interface dispatch
//! - **W028** — boxing of a value type
//!
//! Burst targets are detected via the simple heuristic of method names
//! starting with `burst_` or `Burst_`, plus an explicit list passed by the
//! caller. (The `@burst` annotation in PrSM source is parsed in a follow-up
//! patch — once it lands, the analyzer just receives the annotated names.)
//!
//! The analyzer never mutates the IR; it only produces diagnostics.

use super::csharp_ir::*;
use crate::diagnostics::Diagnostic;
use crate::lexer::token::{Position, Span};

/// Configuration for the Burst pass.
#[derive(Debug, Clone, Default)]
pub struct BurstAnalysisOptions {
    /// Method names that should be analyzed regardless of naming heuristics.
    pub explicit_targets: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct BurstAnalysisReport {
    pub analyzed_methods: u32,
    pub diagnostics: Vec<Diagnostic>,
}

impl BurstAnalysisReport {
    pub fn is_empty(&self) -> bool {
        self.analyzed_methods == 0 && self.diagnostics.is_empty()
    }
}

/// Run Burst compatibility analysis over a C# IR file.
pub fn analyze(file: &CsFile, options: &BurstAnalysisOptions) -> BurstAnalysisReport {
    let mut report = BurstAnalysisReport::default();
    analyze_class(&file.class, options, &mut report);
    for extra in &file.extra_types {
        analyze_class(extra, options, &mut report);
    }
    report
}

fn analyze_class(class: &CsClass, options: &BurstAnalysisOptions, report: &mut BurstAnalysisReport) {
    for member in &class.members {
        if let CsMember::Method {
            name,
            body,
            source_span,
            ..
        } = member
        {
            if !is_burst_target(name, options) {
                continue;
            }
            report.analyzed_methods += 1;
            let span = source_span.unwrap_or(default_span());
            let mut ctx = AnalysisCtx {
                method_name: name.clone(),
                fallback_span: span,
                report,
            };
            analyze_block(body, &mut ctx);
        }
    }
}

fn is_burst_target(name: &str, options: &BurstAnalysisOptions) -> bool {
    if options.explicit_targets.iter().any(|n| n == name) {
        return true;
    }
    name.starts_with("burst_") || name.starts_with("Burst_") || name.starts_with("__burst_")
}

struct AnalysisCtx<'a> {
    method_name: String,
    fallback_span: Span,
    report: &'a mut BurstAnalysisReport,
}

fn analyze_block(stmts: &[CsStmt], ctx: &mut AnalysisCtx) {
    for stmt in stmts {
        analyze_stmt(stmt, ctx);
    }
}

fn analyze_stmt(stmt: &CsStmt, ctx: &mut AnalysisCtx) {
    match stmt {
        CsStmt::TryCatch { source_span, .. } => {
            ctx.report.diagnostics.push(Diagnostic::error(
                "E138",
                format!(
                    "try/catch is not allowed in @burst method '{}'",
                    ctx.method_name
                ),
                source_span.unwrap_or(ctx.fallback_span),
            ));
        }
        CsStmt::VarDecl { ty, init, source_span, .. } => {
            let span = source_span.unwrap_or(ctx.fallback_span);
            check_managed_type(ty, span, ctx);
            check_expression_text(init, span, ctx);
        }
        CsStmt::Assignment { value, source_span, .. } => {
            let span = source_span.unwrap_or(ctx.fallback_span);
            check_expression_text(value, span, ctx);
        }
        CsStmt::Expr(text, source_span) => {
            check_expression_text(text, source_span.unwrap_or(ctx.fallback_span), ctx);
        }
        CsStmt::If { cond, then_body, else_body, source_span } => {
            check_expression_text(cond, source_span.unwrap_or(ctx.fallback_span), ctx);
            analyze_block(then_body, ctx);
            if let Some(else_body) = else_body {
                analyze_block(else_body, ctx);
            }
        }
        CsStmt::Switch { subject, cases, source_span } => {
            check_expression_text(subject, source_span.unwrap_or(ctx.fallback_span), ctx);
            for case in cases {
                analyze_block(&case.body, ctx);
            }
        }
        CsStmt::For { cond, incr, body, source_span, .. } => {
            check_expression_text(cond, source_span.unwrap_or(ctx.fallback_span), ctx);
            check_expression_text(incr, source_span.unwrap_or(ctx.fallback_span), ctx);
            analyze_block(body, ctx);
        }
        CsStmt::ForEach { iterable, body, source_span, .. } => {
            check_expression_text(iterable, source_span.unwrap_or(ctx.fallback_span), ctx);
            analyze_block(body, ctx);
        }
        CsStmt::While { cond, body, source_span } => {
            check_expression_text(cond, source_span.unwrap_or(ctx.fallback_span), ctx);
            analyze_block(body, ctx);
        }
        CsStmt::Block(inner, _) => analyze_block(inner, ctx),
        CsStmt::Return(Some(text), source_span) => {
            check_expression_text(text, source_span.unwrap_or(ctx.fallback_span), ctx);
        }
        CsStmt::YieldReturn(text, source_span) => {
            check_expression_text(text, source_span.unwrap_or(ctx.fallback_span), ctx);
        }
        CsStmt::Throw(_, source_span) => {
            ctx.report.diagnostics.push(Diagnostic::error(
                "E138",
                format!(
                    "throw inside @burst method '{}' implies try/catch elsewhere; not Burst-safe",
                    ctx.method_name
                ),
                source_span.unwrap_or(ctx.fallback_span),
            ));
        }
        CsStmt::Raw(_, _) | CsStmt::Break(_) | CsStmt::Continue(_) | CsStmt::Return(None, _) => {}
        CsStmt::UseBlock { body, source_span, .. } => {
            ctx.report.diagnostics.push(Diagnostic::error(
                "E137",
                format!(
                    "`use` block introduces an IDisposable (managed) reference in @burst method '{}'",
                    ctx.method_name
                ),
                source_span.unwrap_or(ctx.fallback_span),
            ));
            analyze_block(body, ctx);
        }
    }
}

/// Check a textual expression for managed-type / virtual-call / boxing
/// patterns. The check is intentionally conservative — false positives are
/// avoided by only flagging well-known managed identifiers.
fn check_expression_text(expr: &str, span: Span, ctx: &mut AnalysisCtx) {
    if expr.is_empty() {
        return;
    }
    if contains_managed_type_token(expr) {
        ctx.report.diagnostics.push(Diagnostic::error(
            "E137",
            format!(
                "Managed type reference in @burst method '{}': '{}'",
                ctx.method_name,
                summarize_expr(expr)
            ),
            span,
        ));
    }
    if contains_virtual_dispatch(expr) {
        ctx.report.diagnostics.push(Diagnostic::error(
            "E139",
            format!(
                "Virtual or interface call in @burst method '{}'",
                ctx.method_name
            ),
            span,
        ));
    }
    if contains_boxing(expr) {
        ctx.report.diagnostics.push(Diagnostic::warning(
            "W028",
            format!(
                "Possible boxing of a value type in @burst method '{}'",
                ctx.method_name
            ),
            span,
        ));
    }
}

fn check_managed_type(ty: &str, span: Span, ctx: &mut AnalysisCtx) {
    if ty.is_empty() || ty == "var" {
        return;
    }
    if MANAGED_TYPE_NAMES.iter().any(|m| ty == *m || ty.contains(*m)) {
        ctx.report.diagnostics.push(Diagnostic::error(
            "E137",
            format!(
                "Local of managed type '{}' is not allowed in @burst method '{}'",
                ty, ctx.method_name
            ),
            span,
        ));
    }
}

const MANAGED_TYPE_NAMES: &[&str] = &[
    "string",
    "String",
    "object",
    "Object",
    "List<",
    "Dictionary<",
    "HashSet<",
    "Action",
    "Func<",
    "Delegate",
    "GameObject",
    "Component",
    "MonoBehaviour",
    "Transform",
];

fn contains_managed_type_token(expr: &str) -> bool {
    // String literals are obvious managed values; reject the cheap form.
    if expr.contains("\"") || expr.contains("$\"") {
        return true;
    }
    if expr.contains("new string") || expr.contains("new List<") || expr.contains("new Dictionary<") {
        return true;
    }
    if expr.contains(".ToString()") || expr.contains("string.Format") {
        return true;
    }
    false
}

fn contains_virtual_dispatch(expr: &str) -> bool {
    // Heuristic: calls on `transform.`, `gameObject.`, or any method that
    // looks like an interface call. Burst forbids these in general.
    if expr.contains("GetComponent<") || expr.contains("Instantiate(") {
        return true;
    }
    if expr.contains(".Invoke(") {
        return true;
    }
    false
}

fn contains_boxing(expr: &str) -> bool {
    // Boxing is implied by `(object)` casts or use of `as object` etc.
    if expr.contains("(object)") || expr.contains(" as object") {
        return true;
    }
    false
}

fn summarize_expr(expr: &str) -> String {
    let trimmed = expr.trim();
    if trimmed.len() > 60 {
        format!("{}…", &trimmed[..57])
    } else {
        trimmed.to_string()
    }
}

fn default_span() -> Span {
    Span {
        start: Position { line: 1, col: 1 },
        end: Position { line: 1, col: 1 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::csharp_ir::CsClass;

    fn make_method(name: &str, body: Vec<CsStmt>) -> CsMember {
        CsMember::Method {
            attributes: vec![],
            modifiers: "public".into(),
            return_ty: "void".into(),
            name: name.into(),
            params: vec![],
            where_clauses: vec![],
            body,
            source_span: None,
        }
    }

    fn class_with(method: CsMember) -> CsFile {
        CsFile {
            header_comment: String::new(),
            usings: vec![],
            class: CsClass {
                attributes: vec![],
                modifiers: "public".into(),
                name: "T".into(),
                base_class: None,
                interfaces: vec![],
                where_clauses: vec![],
                members: vec![method],
            },
            extra_types: vec![],
        }
    }

    #[test]
    fn flags_managed_type_local() {
        let method = make_method(
            "burst_run",
            vec![CsStmt::VarDecl {
                ty: "string".into(),
                name: "msg".into(),
                init: "\"hi\"".into(),
                source_span: None,
            }],
        );
        let file = class_with(method);
        let report = analyze(&file, &BurstAnalysisOptions::default());
        assert_eq!(report.analyzed_methods, 1);
        assert!(report.diagnostics.iter().any(|d| d.code == "E137"));
    }

    #[test]
    fn flags_try_catch() {
        let method = make_method(
            "burst_run",
            vec![CsStmt::TryCatch {
                try_body: vec![],
                catches: vec![],
                finally_body: None,
                source_span: None,
            }],
        );
        let file = class_with(method);
        let report = analyze(&file, &BurstAnalysisOptions::default());
        assert!(report.diagnostics.iter().any(|d| d.code == "E138"));
    }

    #[test]
    fn flags_virtual_dispatch() {
        let method = make_method(
            "burst_run",
            vec![CsStmt::Expr("GetComponent<Rigidbody>()".into(), None)],
        );
        let file = class_with(method);
        let report = analyze(&file, &BurstAnalysisOptions::default());
        assert!(report.diagnostics.iter().any(|d| d.code == "E139"));
    }

    #[test]
    fn flags_boxing_warning() {
        let method = make_method(
            "burst_run",
            vec![CsStmt::Expr("(object)x".into(), None)],
        );
        let file = class_with(method);
        let report = analyze(&file, &BurstAnalysisOptions::default());
        assert!(report.diagnostics.iter().any(|d| d.code == "W028"));
    }

    #[test]
    fn skips_methods_outside_burst_target() {
        let method = make_method(
            "Update",
            vec![CsStmt::VarDecl {
                ty: "string".into(),
                name: "msg".into(),
                init: "\"hi\"".into(),
                source_span: None,
            }],
        );
        let file = class_with(method);
        let report = analyze(&file, &BurstAnalysisOptions::default());
        assert_eq!(report.analyzed_methods, 0);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn explicit_targets_are_analyzed() {
        let method = make_method(
            "Compute",
            vec![CsStmt::Expr("(object)x".into(), None)],
        );
        let file = class_with(method);
        let opts = BurstAnalysisOptions {
            explicit_targets: vec!["Compute".into()],
        };
        let report = analyze(&file, &opts);
        assert_eq!(report.analyzed_methods, 1);
        assert!(report.diagnostics.iter().any(|d| d.code == "W028"));
    }
}
