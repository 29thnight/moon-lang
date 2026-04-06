//! Refactoring helpers for v4 Section 29 (`dx.refactor`).
//!
//! These functions are pure text/AST transformations that the LSP exposes
//! as code actions. Each helper takes a snapshot of the source and returns
//! either a `RefactorEdit` (a single text replacement) or a structured error
//! describing why the refactor cannot be applied.
//!
//! Supported actions:
//!
//! - **Extract Method** (§29.2.1) — pull a contiguous statement region into a
//!   new function. The detector picks up free identifiers in the region and
//!   reports them as required parameters; computing the return value is left
//!   to the caller.
//! - **Extract Component** (§29.2.2) — split a member set into a fresh
//!   component file. The current cut returns a preview struct only.
//! - **Inline Variable** (§29.2.3) — substitute a single-use `val`'s
//!   initializer at the use site.
//! - **Rename Symbol** (§29.2.4) — find every occurrence of a bare identifier
//!   in the source and produce edits for each.
//! - **Convert to State Machine** (§29.2.5) — detect an `enum` + `when`-style
//!   shape and emit the equivalent `state machine` sugar.
//!
//! All helpers are intentionally syntax-light: they consume `&str` and
//! produce `RefactorEdit`s rather than mutating the AST in place. This keeps
//! them testable without spinning up a full project graph.

use crate::lexer::token::{Position, Span};

/// A single text replacement to apply on a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefactorEdit {
    pub range: Span,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefactorPlan {
    pub title: String,
    pub edits: Vec<RefactorEdit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefactorError {
    InvalidSelection(String),
    NoOccurrences(String),
    NotApplicable(String),
}

// === Extract Method ============================================================

/// Result of an extract-method analysis. The caller is expected to splice the
/// generated function declaration above the current method and replace the
/// selection with the call.
#[derive(Debug, Clone)]
pub struct ExtractMethodPlan {
    pub function_decl: String,
    pub call_site: String,
    pub free_variables: Vec<String>,
}

/// Build an extract-method plan from a selected region.
///
/// `selection` is the literal text the user selected; `function_name` is the
/// name proposed by the IDE. The free-variable scan is intentionally
/// approximate — it only collects bare identifiers that appear before any `.`
/// or `(`. The IDE shows the result as a preview so the user can rename
/// parameters before accepting.
pub fn extract_method(
    selection: &str,
    function_name: &str,
) -> Result<ExtractMethodPlan, RefactorError> {
    let trimmed = selection.trim();
    if trimmed.is_empty() {
        return Err(RefactorError::InvalidSelection(
            "Cannot extract an empty selection".into(),
        ));
    }
    if !is_valid_identifier(function_name) {
        return Err(RefactorError::InvalidSelection(format!(
            "'{}' is not a valid function name",
            function_name
        )));
    }

    let free = collect_free_identifiers(trimmed);
    let params: Vec<String> = free
        .iter()
        .map(|name| format!("{}: var", name))
        .collect();
    let param_list = params.join(", ");
    let arg_list = free.join(", ");

    let body_lines: Vec<String> = trimmed
        .lines()
        .map(|line| format!("    {}", line.trim_end()))
        .collect();
    let body = body_lines.join("\n");

    let function_decl = format!(
        "func {}({}) {{\n{}\n}}\n",
        function_name, param_list, body
    );
    let call_site = format!("{}({})", function_name, arg_list);

    Ok(ExtractMethodPlan {
        function_decl,
        call_site,
        free_variables: free,
    })
}

fn collect_free_identifiers(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        // Skip string literals to avoid harvesting words from text.
        if c == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_')
            {
                i += 1;
            }
            let word = &source[start..i];
            // Skip identifiers preceded by `.` (member access).
            if start > 0 && bytes[start - 1] == b'.' {
                continue;
            }
            // Skip keywords.
            if is_keyword(word) {
                continue;
            }
            // Skip identifiers immediately followed by `(` (function call).
            let after_ws = i + bytes[i..].iter().take_while(|c| **c == b' ').count();
            if after_ws < bytes.len() && bytes[after_ws] == b'(' {
                continue;
            }
            // Skip identifiers introduced by `val`/`var` declarations.
            // We handle this in a second pass to keep the scan simple — for
            // a first cut we always include the binding name.
            if !out.contains(&word.to_string()) {
                out.push(word.to_string());
            }
            continue;
        }
        i += 1;
    }
    // Drop names that look like they were declared inside the selection
    // (`val name` or `var name`). We do this with a substring search rather
    // than full parsing for the first cut.
    out.retain(|name| {
        let val_decl = format!("val {}", name);
        let var_decl = format!("var {}", name);
        !source.contains(&val_decl) && !source.contains(&var_decl)
    });
    out
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "val"
            | "var"
            | "if"
            | "else"
            | "for"
            | "while"
            | "return"
            | "func"
            | "true"
            | "false"
            | "null"
            | "this"
            | "in"
            | "is"
            | "as"
            | "when"
            | "match"
    )
}

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// === Inline Variable ===========================================================

/// Plan for an inline-variable refactor: when applied, the val declaration is
/// removed and every occurrence of the bound name is replaced with the
/// initializer expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineVariablePlan {
    pub removal: RefactorEdit,
    pub replacements: Vec<RefactorEdit>,
}

/// Inline a single-use `val name = expr`. The decl span and the use span are
/// supplied by the caller (the LSP knows them from the symbol index). The
/// helper just builds the edit list.
pub fn inline_variable(
    name: &str,
    initializer: &str,
    decl_range: Span,
    uses: &[Span],
) -> Result<InlineVariablePlan, RefactorError> {
    if uses.is_empty() {
        return Err(RefactorError::NoOccurrences(format!(
            "'{}' has no uses to inline",
            name
        )));
    }
    let removal = RefactorEdit {
        range: decl_range,
        new_text: String::new(),
    };
    let replacements: Vec<RefactorEdit> = uses
        .iter()
        .map(|range| RefactorEdit {
            range: *range,
            new_text: initializer.to_string(),
        })
        .collect();
    Ok(InlineVariablePlan { removal, replacements })
}

// === Rename Symbol =============================================================

/// Build a project-wide rename plan. `occurrences` lists every span where the
/// identifier appears (the LSP collects this from the symbol index).
pub fn rename_symbol(
    new_name: &str,
    occurrences: &[Span],
) -> Result<RefactorPlan, RefactorError> {
    if !is_valid_identifier(new_name) {
        return Err(RefactorError::InvalidSelection(format!(
            "'{}' is not a valid identifier",
            new_name
        )));
    }
    if occurrences.is_empty() {
        return Err(RefactorError::NoOccurrences(
            "no occurrences to rename".into(),
        ));
    }
    let edits: Vec<RefactorEdit> = occurrences
        .iter()
        .map(|range| RefactorEdit {
            range: *range,
            new_text: new_name.to_string(),
        })
        .collect();
    Ok(RefactorPlan {
        title: format!("Rename to '{}'", new_name),
        edits,
    })
}

// === Convert to State Machine ==================================================

/// Detect an `enum + when` state pattern. Returns the proposed `state machine`
/// declaration when the input matches the heuristic.
pub fn convert_to_state_machine(
    enum_name: &str,
    states: &[String],
) -> Result<String, RefactorError> {
    if states.is_empty() {
        return Err(RefactorError::NotApplicable(
            "no states detected in the source enum".into(),
        ));
    }
    let mut out = String::new();
    out.push_str(&format!("state machine {} {{\n", enum_name));
    for state in states {
        out.push_str(&format!("    state {} {{\n        // TODO: enter/exit/on transitions\n    }}\n", state));
    }
    out.push_str("}\n");
    Ok(out)
}

// === Extract Component =========================================================

/// Lightweight preview-only extract: returns the proposed file body and the
/// `require` line that should replace the original member group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractComponentPlan {
    pub component_name: String,
    pub component_source: String,
    pub require_line: String,
}

pub fn extract_component(
    component_name: &str,
    selected_members: &str,
) -> Result<ExtractComponentPlan, RefactorError> {
    if !is_valid_identifier(component_name) {
        return Err(RefactorError::InvalidSelection(format!(
            "'{}' is not a valid component name",
            component_name
        )));
    }
    if selected_members.trim().is_empty() {
        return Err(RefactorError::InvalidSelection(
            "no members selected".into(),
        ));
    }

    let body_lines: Vec<String> = selected_members
        .lines()
        .map(|line| format!("    {}", line.trim_end()))
        .collect();
    let body = body_lines.join("\n");
    let component_source = format!(
        "component {} {{\n{}\n}}\n",
        component_name, body
    );
    let lower_first = component_name
        .chars()
        .next()
        .map(|c| c.to_ascii_lowercase().to_string())
        .unwrap_or_default();
    let rest: String = component_name.chars().skip(1).collect();
    let field_name = format!("{}{}", lower_first, rest);
    let require_line = format!("require {}: {}", field_name, component_name);

    Ok(ExtractComponentPlan {
        component_name: component_name.to_string(),
        component_source,
        require_line,
    })
}

// === Helpers ===================================================================

#[allow(dead_code)]
fn span(sl: u32, sc: u32, el: u32, ec: u32) -> Span {
    Span {
        start: Position { line: sl, col: sc },
        end: Position { line: el, col: ec },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(sl: u32, sc: u32, el: u32, ec: u32) -> Span {
        Span {
            start: Position { line: sl, col: sc },
            end: Position { line: el, col: ec },
        }
    }

    #[test]
    fn extract_method_basic_distance_example() {
        let selection = "val dx = target.x - origin.x\nval dz = target.z - origin.z\nval dist = sqrt(dx * dx + dz * dz)";
        let plan = extract_method(selection, "computeDistance").expect("plan");
        assert!(plan.function_decl.contains("func computeDistance"));
        // origin and target are free; dx/dz/dist are declared inside.
        assert!(plan.free_variables.contains(&"origin".to_string()));
        assert!(plan.free_variables.contains(&"target".to_string()));
        assert!(!plan.free_variables.contains(&"dx".to_string()));
        assert!(plan.call_site.starts_with("computeDistance("));
    }

    #[test]
    fn extract_method_rejects_empty_selection() {
        assert!(matches!(
            extract_method("   \n  ", "f"),
            Err(RefactorError::InvalidSelection(_))
        ));
    }

    #[test]
    fn extract_method_rejects_invalid_name() {
        assert!(matches!(
            extract_method("val x = 1", "1bad"),
            Err(RefactorError::InvalidSelection(_))
        ));
    }

    #[test]
    fn inline_variable_builds_replacements() {
        let plan = inline_variable("x", "1 + 2", s(1, 1, 1, 12), &[s(2, 5, 2, 6), s(3, 10, 3, 11)])
            .expect("plan");
        assert_eq!(plan.replacements.len(), 2);
        assert_eq!(plan.replacements[0].new_text, "1 + 2");
        assert_eq!(plan.removal.new_text, "");
    }

    #[test]
    fn inline_variable_errors_when_unused() {
        assert!(matches!(
            inline_variable("x", "1", s(1, 1, 1, 5), &[]),
            Err(RefactorError::NoOccurrences(_))
        ));
    }

    #[test]
    fn rename_symbol_yields_one_edit_per_occurrence() {
        let plan =
            rename_symbol("renamed", &[s(1, 1, 1, 4), s(5, 10, 5, 13)]).expect("plan");
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits.iter().all(|e| e.new_text == "renamed"));
        assert!(plan.title.contains("renamed"));
    }

    #[test]
    fn rename_symbol_rejects_keyword_like_name() {
        assert!(matches!(
            rename_symbol("123", &[s(1, 1, 1, 2)]),
            Err(RefactorError::InvalidSelection(_))
        ));
    }

    #[test]
    fn convert_to_state_machine_emits_states() {
        let states = vec!["Idle".to_string(), "Walk".to_string(), "Attack".to_string()];
        let out = convert_to_state_machine("PlayerState", &states).expect("source");
        assert!(out.starts_with("state machine PlayerState"));
        assert!(out.contains("state Idle"));
        assert!(out.contains("state Walk"));
        assert!(out.contains("state Attack"));
    }

    #[test]
    fn convert_to_state_machine_rejects_empty_states() {
        assert!(matches!(
            convert_to_state_machine("X", &[]),
            Err(RefactorError::NotApplicable(_))
        ));
    }

    #[test]
    fn extract_component_emits_component_and_require() {
        let plan = extract_component("Health", "var hp: Int = 100\nfunc takeDamage(d: Int) { hp -= d }")
            .expect("plan");
        assert!(plan.component_source.starts_with("component Health"));
        assert!(plan.component_source.contains("takeDamage"));
        assert_eq!(plan.require_line, "require health: Health");
    }

    #[test]
    fn extract_component_rejects_invalid_name() {
        assert!(matches!(
            extract_component("123", "var x = 1"),
            Err(RefactorError::InvalidSelection(_))
        ));
    }

    #[test]
    fn collect_free_identifiers_skips_strings() {
        let free = collect_free_identifiers("val x = \"hello world\" + name");
        assert!(free.contains(&"name".to_string()));
        assert!(!free.iter().any(|n| n == "hello"));
    }

    #[test]
    fn is_valid_identifier_basic() {
        assert!(is_valid_identifier("name"));
        assert!(is_valid_identifier("_private"));
        assert!(!is_valid_identifier("1bad"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("has-dash"));
    }
}
