//! Debugger Integration helpers (v4 Section 30, `dx.debugger`).
//!
//! The compiler already produces a rich source-map (`source_map.rs`) that the
//! VS Code extension can consume, but the format the spec mandates in
//! §30.2 is the simpler line-pair shape:
//!
//! ```json
//! {
//!     "version": 1,
//!     "source": "src/Player.prsm",
//!     "generated": "Generated/Player.cs",
//!     "mappings": [
//!         { "prsmLine": 5, "csLine": 12 },
//!         { "prsmLine": 6, "csLine": 13 }
//!     ]
//! }
//! ```
//!
//! This module flattens the existing rich `SourceMapFile` into that shape,
//! exposes a stable variable rename helper for the debugger adapter, and
//! provides a `DebugAdapterInfo` struct that the editor can read on startup —
//! the actual DAP server is out of scope for this phase, but the entry point
//! is wired so an external adapter can be plugged in later without touching
//! the compiler again.

use crate::source_map::{SourceMapAnchor, SourceMapFile, SourceMapSpan};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Spec-compliant flat source map (v4 §30.2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebugSourceMap {
    pub version: u32,
    pub source: String,
    pub generated: String,
    pub mappings: Vec<DebugMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebugMapping {
    #[serde(rename = "prsmLine")]
    pub prsm_line: u32,
    #[serde(rename = "csLine")]
    pub cs_line: u32,
}

/// DAP adapter discovery info written alongside generated source. The
/// extension reads this on startup so it knows which adapter to launch and
/// where to find the source maps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebugAdapterInfo {
    pub adapter_kind: String,
    pub language: String,
    pub source_map_glob: String,
    pub generated_glob: String,
    pub step_filter_patterns: Vec<String>,
}

impl Default for DebugAdapterInfo {
    fn default() -> Self {
        DebugAdapterInfo {
            adapter_kind: "vscode-cs".into(),
            language: "prsm".into(),
            source_map_glob: "**/*.prsmmap.json".into(),
            generated_glob: "**/*.cs".into(),
            step_filter_patterns: default_step_filters(),
        }
    }
}

/// Default skip-list for §30.3.3 — compiler-generated boilerplate the user
/// should never step through.
pub fn default_step_filters() -> Vec<String> {
    vec![
        "__opt_*".into(),
        "__cached_*".into(),
        "__prev_*".into(),
        "*PoolFactory_*".into(),
        "*StateMachineDispatch".into(),
        "*Singleton.Awake".into(),
    ]
}

/// Build a flat mapping from a rich `SourceMapFile`. Each member anchor is
/// exploded into one mapping per line of its source span, paired with the
/// matching line of its generated span. When a member has no `generated_span`
/// it is silently skipped.
pub fn flatten_source_map(map: &SourceMapFile) -> DebugSourceMap {
    let mut mappings: Vec<DebugMapping> = Vec::new();
    if let Some(decl) = &map.declaration {
        push_anchor_mappings(decl, &mut mappings);
    }
    for member in &map.members {
        push_anchor_mappings(member, &mut mappings);
    }
    mappings.sort_by(|a, b| a.prsm_line.cmp(&b.prsm_line).then(a.cs_line.cmp(&b.cs_line)));
    mappings.dedup();

    DebugSourceMap {
        version: 1,
        source: map.source_file.to_string_lossy().to_string(),
        generated: map.generated_file.to_string_lossy().to_string(),
        mappings,
    }
}

fn push_anchor_mappings(anchor: &SourceMapAnchor, out: &mut Vec<DebugMapping>) {
    if let Some(generated) = &anchor.generated_span {
        push_pair(anchor.source_span, *generated, out);
    }
    if let Some(name_gen) = &anchor.generated_name_span {
        push_pair(anchor.source_span, *name_gen, out);
    }
    for segment in &anchor.segments {
        push_anchor_mappings(segment, out);
    }
}

fn push_pair(src: SourceMapSpan, gen: SourceMapSpan, out: &mut Vec<DebugMapping>) {
    let src_lines = src.line..=src.end_line.max(src.line);
    let gen_lines = gen.line..=gen.end_line.max(gen.line);
    let src_count = (src.end_line.saturating_sub(src.line)) as usize + 1;
    let gen_count = (gen.end_line.saturating_sub(gen.line)) as usize + 1;
    let pair_count = src_count.min(gen_count).max(1);

    let src_vec: Vec<u32> = src_lines.collect();
    let gen_vec: Vec<u32> = gen_lines.collect();
    for i in 0..pair_count {
        let src_line = src_vec.get(i).copied().unwrap_or(src.line);
        let cs_line = gen_vec.get(i).copied().unwrap_or(gen.line);
        out.push(DebugMapping {
            prsm_line: src_line,
            cs_line,
        });
    }
}

/// Compute the path of the flat debug map next to the generated `.cs` file.
pub fn debug_map_path_for_generated(generated_file: &Path) -> PathBuf {
    let stem = generated_file
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("generated");
    generated_file.with_file_name(format!("{}.prsm.map", stem))
}

/// Reverse mapping for §30.3.2: when a debugger sees a generated identifier
/// like `_prsm_d` it can ask the compiler for the user-facing name. The
/// compiler emits a small lookup table per file.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct VariableNameTable {
    pub entries: Vec<VariableNameEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableNameEntry {
    pub generated: String,
    pub original: String,
}

impl VariableNameTable {
    pub fn add(&mut self, generated: impl Into<String>, original: impl Into<String>) {
        self.entries.push(VariableNameEntry {
            generated: generated.into(),
            original: original.into(),
        });
    }

    pub fn lookup<'a>(&'a self, generated: &str) -> Option<&'a str> {
        self.entries
            .iter()
            .find(|entry| entry.generated == generated)
            .map(|entry| entry.original.as_str())
    }
}

/// Should the debugger step into a generated symbol? Returns `false` for
/// names that match any of the configured filter patterns (glob-ish: `*` is
/// the only metachar). Used by the VS Code extension via JSON-RPC.
pub fn should_step_into(symbol: &str, filters: &[String]) -> bool {
    !filters.iter().any(|pattern| glob_match(pattern, symbol))
}

fn glob_match(pattern: &str, value: &str) -> bool {
    // Tiny glob: only `*` (any-chars) and literal segments. Sufficient for
    // the small skip list shipped by default — keeps us free of an external
    // dependency.
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut cursor = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 && !pattern.starts_with('*') {
            if !value[cursor..].starts_with(part) {
                return false;
            }
            cursor += part.len();
            continue;
        }
        if i == parts.len() - 1 && !pattern.ends_with('*') {
            return value[cursor..].ends_with(part);
        }
        if let Some(found) = value[cursor..].find(part) {
            cursor += found + part.len();
        } else {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::token::{Position, Span};
    use crate::source_map::{SourceMapAnchor, SourceMapFile, SourceMapSpan};

    fn span_pair(sl: u32, el: u32) -> SourceMapSpan {
        SourceMapSpan {
            line: sl,
            col: 1,
            end_line: el,
            end_col: 1,
        }
    }

    fn rich_map() -> SourceMapFile {
        SourceMapFile {
            version: 1,
            source_file: PathBuf::from("src/Player.prsm"),
            generated_file: PathBuf::from("Generated/Player.cs"),
            declaration: Some(SourceMapAnchor {
                kind: "Type".into(),
                name: "Player".into(),
                qualified_name: "Player".into(),
                source_span: span_pair(1, 30),
                generated_span: Some(span_pair(5, 90)),
                generated_name_span: None,
                segments: vec![],
            }),
            members: vec![SourceMapAnchor {
                kind: "Func".into(),
                name: "Update".into(),
                qualified_name: "Player.Update".into(),
                source_span: span_pair(10, 12),
                generated_span: Some(span_pair(20, 22)),
                generated_name_span: None,
                segments: vec![],
            }],
        }
    }

    #[test]
    fn flatten_source_map_emits_one_pair_per_line() {
        let map = rich_map();
        let flat = flatten_source_map(&map);
        assert_eq!(flat.version, 1);
        assert_eq!(flat.source, "src/Player.prsm");
        assert_eq!(flat.generated, "Generated/Player.cs");
        // Declaration spans 1..=30 mapped to 5..=90 → first line pair (1, 5).
        assert!(flat.mappings.iter().any(|m| m.prsm_line == 1 && m.cs_line == 5));
        // Member spans 10..=12 mapped to 20..=22 → expect 3 pairs.
        assert!(flat.mappings.iter().any(|m| m.prsm_line == 10 && m.cs_line == 20));
        assert!(flat.mappings.iter().any(|m| m.prsm_line == 11 && m.cs_line == 21));
        assert!(flat.mappings.iter().any(|m| m.prsm_line == 12 && m.cs_line == 22));
    }

    #[test]
    fn debug_map_path_uses_dot_prsm_dot_map_extension() {
        let path = debug_map_path_for_generated(&PathBuf::from("Generated/Player.cs"));
        assert!(path.to_string_lossy().ends_with("Player.prsm.map"));
    }

    #[test]
    fn variable_name_table_lookup() {
        let mut table = VariableNameTable::default();
        table.add("_prsm_d", "damage");
        table.add("__hp", "hp");
        assert_eq!(table.lookup("_prsm_d"), Some("damage"));
        assert_eq!(table.lookup("__hp"), Some("hp"));
        assert_eq!(table.lookup("missing"), None);
    }

    #[test]
    fn should_step_into_skips_optimizer_temps() {
        let filters = default_step_filters();
        assert!(!should_step_into("__opt_cached_label_text", &filters));
        assert!(!should_step_into("__opt_prev_label_text_hp", &filters));
        assert!(should_step_into("Update", &filters));
        assert!(should_step_into("ComputeDamage", &filters));
    }

    #[test]
    fn glob_match_basic_patterns() {
        assert!(glob_match("__opt_*", "__opt_cached_x"));
        assert!(glob_match("*Singleton.Awake", "PlayerSingleton.Awake"));
        assert!(glob_match("*StateMachineDispatch", "FooStateMachineDispatch"));
        assert!(!glob_match("__opt_*", "Update"));
    }

    #[test]
    fn debug_adapter_info_default_has_filters() {
        let info = DebugAdapterInfo::default();
        assert_eq!(info.adapter_kind, "vscode-cs");
        assert!(info.step_filter_patterns.iter().any(|p| p == "__opt_*"));
        assert!(!info.source_map_glob.is_empty());
    }

    #[test]
    fn flat_map_serializes_to_spec_keys() {
        let map = rich_map();
        let flat = flatten_source_map(&map);
        let json = serde_json::to_string(&flat).expect("serialize");
        assert!(json.contains("\"prsmLine\""));
        assert!(json.contains("\"csLine\""));
        assert!(json.contains("\"version\":1"));
    }

    fn _unused_span_helper() -> Span {
        Span {
            start: Position { line: 1, col: 1 },
            end: Position { line: 1, col: 1 },
        }
    }
}
