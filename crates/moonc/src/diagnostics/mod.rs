use crate::lexer::token::Span;

/// Severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A compiler diagnostic.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Diagnostic { code, severity: Severity::Error, message: message.into(), span }
    }

    pub fn warning(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Diagnostic { code, severity: Severity::Warning, message: message.into(), span }
    }
}

/// Collects diagnostics during analysis.
#[derive(Debug, Default)]
pub struct DiagnosticCollector {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn error(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(code, message, span));
    }

    pub fn warning(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::warning(code, message, span));
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect()
    }
}
