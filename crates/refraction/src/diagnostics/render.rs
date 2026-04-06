//! Rendering of diagnostics in the multi-line, Rust/Elm-style format
//! introduced in v4 Section 28 (`dx.errors`).
//!
//! Output shape:
//! ```text
//! error[E090]: Interface member 'takeDamage' not implemented
//!   --> src/Enemy.prsm:15:1
//!    |
//! 15 | component Enemy : MonoBehaviour, IDamageable {
//!    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//!    |
//!    = help: Add the missing method:
//!    = note: Required by IDamageable
//! ```
//!
//! ANSI colors are emitted only when `with_color` is set; the LSP and JSON
//! pipelines call the plain renderer.

use super::{Diagnostic, DiagnosticNote, Severity};
use crate::lexer::token::Span;

/// Style options for the renderer.
#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub with_color: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions { with_color: false }
    }
}

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_RED: &str = "\x1b[31m";
const ANSI_YELLOW: &str = "\x1b[33m";
const ANSI_BLUE: &str = "\x1b[34m";
const ANSI_CYAN: &str = "\x1b[36m";
const ANSI_GREEN: &str = "\x1b[32m";

fn paint(text: &str, color: &str, options: RenderOptions) -> String {
    if options.with_color {
        format!("{}{}{}{}", ANSI_BOLD, color, text, ANSI_RESET)
    } else {
        text.to_string()
    }
}

/// Render a diagnostic with the given source text. The renderer is tolerant of
/// missing or short source files: when a span is out of range it falls back to
/// the simple `file:line:col: code message` form.
pub fn render_diagnostic(
    diagnostic: &Diagnostic,
    file_path: &str,
    source: &str,
    options: RenderOptions,
) -> String {
    let severity_word = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let severity_color = match diagnostic.severity {
        Severity::Error => ANSI_RED,
        Severity::Warning => ANSI_YELLOW,
    };

    let header_severity = paint(severity_word, severity_color, options);
    let header_code = paint(&format!("[{}]", diagnostic.code), severity_color, options);
    let header = format!(
        "{}{}: {}",
        header_severity,
        header_code,
        if options.with_color {
            format!("{}{}{}", ANSI_BOLD, diagnostic.message, ANSI_RESET)
        } else {
            diagnostic.message.clone()
        },
    );

    let location = format!(
        "{} {}:{}:{}",
        paint("-->", ANSI_BLUE, options),
        file_path,
        diagnostic.span.start.line,
        diagnostic.span.start.col
    );

    let mut out = String::new();
    out.push_str(&header);
    out.push('\n');
    out.push_str("  ");
    out.push_str(&location);
    out.push('\n');

    let snippet = render_snippet(diagnostic, source, options);
    if !snippet.is_empty() {
        out.push_str(&snippet);
    }

    for note in &diagnostic.notes {
        let prefix = paint("=", ANSI_BLUE, options);
        match note {
            DiagnosticNote::Help(text) => {
                let label = paint("help", ANSI_GREEN, options);
                out.push_str(&format!("   {} {}: {}\n", prefix, label, text));
            }
            DiagnosticNote::Note(text) => {
                let label = paint("note", ANSI_CYAN, options);
                out.push_str(&format!("   {} {}: {}\n", prefix, label, text));
            }
        }
    }

    out
}

fn render_snippet(diagnostic: &Diagnostic, source: &str, options: RenderOptions) -> String {
    if source.is_empty() {
        return String::new();
    }
    let lines: Vec<&str> = source.split('\n').collect();
    let primary_line = diagnostic.span.start.line as usize;
    if primary_line == 0 || primary_line > lines.len() {
        return String::new();
    }

    let line_text = lines[primary_line - 1];
    let gutter_width = primary_line.to_string().len().max(2);
    let blank_gutter = " ".repeat(gutter_width);
    let bar = paint("|", ANSI_BLUE, options);

    let mut out = String::new();
    out.push_str(&format!("   {} {}\n", blank_gutter, bar));

    let line_label = paint(
        &format!("{:>width$}", primary_line, width = gutter_width),
        ANSI_BLUE,
        options,
    );
    out.push_str(&format!("   {} {} {}\n", line_label, bar, line_text));

    // Caret line: spaces up to start col then `^^^^` for span length.
    let start_col = diagnostic.span.start.col.saturating_sub(1) as usize;
    let end_col = if diagnostic.span.end.line == diagnostic.span.start.line {
        diagnostic.span.end.col.saturating_sub(1) as usize
    } else {
        line_text.len()
    };
    let caret_len = end_col.saturating_sub(start_col).max(1);
    let mut caret_line = String::new();
    caret_line.push_str(&" ".repeat(start_col));
    let carets = "^".repeat(caret_len);
    let caret_color = match diagnostic.severity {
        Severity::Error => ANSI_RED,
        Severity::Warning => ANSI_YELLOW,
    };
    caret_line.push_str(&paint(&carets, caret_color, options));

    // Inline label = first label whose span lies on this line, otherwise
    // the diagnostic message itself for severity continuity.
    let inline_label = diagnostic
        .labels
        .iter()
        .find(|label| label.span.start.line == diagnostic.span.start.line)
        .map(|label| label.message.clone());
    if let Some(text) = inline_label {
        caret_line.push(' ');
        caret_line.push_str(&paint(&text, caret_color, options));
    }

    out.push_str(&format!("   {} {} {}\n", blank_gutter, bar, caret_line));
    out.push_str(&format!("   {} {}\n", blank_gutter, bar));
    out
}

/// Levenshtein-distance suggestion picker — used for "did you mean" hints.
/// Returns the closest candidate within `max_distance` edits, ignoring case for
/// near-misses (e.g. `Trasform` vs `Transform`).
pub fn suggest<'a, I>(target: &str, candidates: I, max_distance: usize) -> Option<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let target_lower = target.to_lowercase();
    let mut best: Option<(usize, &str)> = None;
    for candidate in candidates {
        if candidate.is_empty() {
            continue;
        }
        let dist = levenshtein(&target_lower, &candidate.to_lowercase());
        if dist > max_distance {
            continue;
        }
        match best {
            Some((best_dist, _)) if dist >= best_dist => {}
            _ => best = Some((dist, candidate)),
        }
    }
    best.map(|(_, name)| name.to_string())
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    if a_chars.is_empty() {
        return b_chars.len();
    }
    if b_chars.is_empty() {
        return a_chars.len();
    }

    let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
    let mut curr: Vec<usize> = vec![0; b_chars.len() + 1];

    for (i, &ac) in a_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &bc) in b_chars.iter().enumerate() {
            let cost = if ac == bc { 0 } else { 1 };
            curr[j + 1] = (curr[j] + 1)
                .min(prev[j + 1] + 1)
                .min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_chars.len()]
}

/// Build a fresh "expected X, found Y" diagnostic in the v4 enhanced format —
/// used by the type-mismatch helper (Section 28.3.2).
pub fn type_mismatch_with_help(
    code: &'static str,
    expected: &str,
    found: &str,
    span: Span,
    suggestion: Option<String>,
) -> Diagnostic {
    let mut d = Diagnostic::error(
        code,
        format!("Type mismatch: expected '{}', found '{}'", expected, found),
        span,
    )
    .with_label(span, format!("expected `{}`, found `{}`", expected, found));
    if let Some(hint) = suggestion {
        d = d.with_help(hint);
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::token::{Position, Span};

    fn span(sl: u32, sc: u32, el: u32, ec: u32) -> Span {
        Span {
            start: Position { line: sl, col: sc },
            end: Position { line: el, col: ec },
        }
    }

    #[test]
    fn renders_basic_error_with_caret_and_help() {
        let source = "val x: Int = \"hello\"\n";
        let diag = Diagnostic::error("E001", "type mismatch", span(1, 14, 1, 21))
            .with_label(span(1, 14, 1, 21), "expected `Int`, found `String`")
            .with_help("did you mean to convert with toInt()?");
        let out = render_diagnostic(&diag, "src/example.prsm", source, RenderOptions::default());
        assert!(out.contains("error[E001]: type mismatch"));
        assert!(out.contains("--> src/example.prsm:1:14"));
        assert!(out.contains("val x: Int = \"hello\""));
        assert!(out.contains("^"));
        assert!(out.contains("expected `Int`, found `String`"));
        assert!(out.contains("help: did you mean to convert with toInt()?"));
    }

    #[test]
    fn renders_warning_severity_token() {
        let source = "val y = 1\n";
        let diag = Diagnostic::warning("W001", "unused", span(1, 5, 1, 6));
        let out = render_diagnostic(&diag, "f.prsm", source, RenderOptions::default());
        assert!(out.contains("warning[W001]"));
    }

    #[test]
    fn renders_note_below_help() {
        let source = "val x = 1\n";
        let diag = Diagnostic::error("E090", "missing impl", span(1, 1, 1, 4))
            .with_help("add the method")
            .with_note("Required by IDamageable");
        let out = render_diagnostic(&diag, "a.prsm", source, RenderOptions::default());
        let help_idx = out.find("help: add the method").expect("help line");
        let note_idx = out.find("note: Required by IDamageable").expect("note line");
        assert!(help_idx < note_idx);
    }

    #[test]
    fn handles_out_of_range_span_gracefully() {
        let source = "val x = 1\n";
        let diag = Diagnostic::error("E001", "boom", span(50, 1, 50, 5));
        let out = render_diagnostic(&diag, "a.prsm", source, RenderOptions::default());
        assert!(out.contains("error[E001]: boom"));
        // No snippet emitted, but no panic.
        assert!(!out.contains(" 50 |"));
    }

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("transform", "transform"), 0);
        assert_eq!(levenshtein("trasform", "transform"), 1);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn suggest_picks_closest_within_threshold() {
        let names = vec!["transform", "renderer", "rigidbody"];
        assert_eq!(
            suggest("trasform", names.iter().copied(), 2),
            Some("transform".to_string())
        );
    }

    #[test]
    fn suggest_returns_none_when_too_far() {
        let names = vec!["transform"];
        assert_eq!(suggest("xyz", names.iter().copied(), 2), None);
    }

    #[test]
    fn ansi_colors_only_when_enabled() {
        let source = "val x = 1\n";
        let diag = Diagnostic::error("E001", "boom", span(1, 1, 1, 4));
        let plain = render_diagnostic(&diag, "a.prsm", source, RenderOptions { with_color: false });
        let colored = render_diagnostic(&diag, "a.prsm", source, RenderOptions { with_color: true });
        assert!(!plain.contains("\x1b["));
        assert!(colored.contains("\x1b["));
    }

    #[test]
    fn type_mismatch_helper_builds_diag_with_help() {
        let s = span(1, 1, 1, 5);
        let d = type_mismatch_with_help(
            "E030",
            "Float",
            "Int",
            s,
            Some("Use '10.toFloat()' or change the type to 'Int'".into()),
        );
        assert_eq!(d.code, "E030");
        assert_eq!(d.labels.len(), 1);
        assert_eq!(d.notes.len(), 1);
        match &d.notes[0] {
            DiagnosticNote::Help(t) => assert!(t.contains("toFloat")),
            _ => panic!("expected help note"),
        }
    }
}
