use crate::lexer::token::Span;

pub mod render;

/// Severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A labeled span attached to a diagnostic — points at a sub-range with a short text.
#[derive(Debug, Clone)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub message: String,
}

/// Auxiliary annotation attached to a diagnostic — `help:` or `note:` (Rust/Elm style).
#[derive(Debug, Clone)]
pub enum DiagnosticNote {
    Help(String),
    Note(String),
}

/// A compiler diagnostic.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    /// Extra labels (Rust-style "expected X, found Y" pointers).
    pub labels: Vec<DiagnosticLabel>,
    /// Help/note lines printed below the source snippet.
    pub notes: Vec<DiagnosticNote>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            code,
            severity: Severity::Error,
            message: message.into(),
            span,
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn warning(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            code,
            severity: Severity::Warning,
            message: message.into(),
            span,
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel { span, message: message.into() });
        self
    }

    pub fn with_help(mut self, message: impl Into<String>) -> Self {
        self.notes.push(DiagnosticNote::Help(message.into()));
        self
    }

    pub fn with_note(mut self, message: impl Into<String>) -> Self {
        self.notes.push(DiagnosticNote::Note(message.into()));
        self
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

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect()
    }
}
