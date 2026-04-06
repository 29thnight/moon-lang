use crate::hir::{HirDefinition, HirDefinitionKind, HirFile};
use crate::lexer::token::Span;
use crate::lowering::csharp_ir::{CsFile, CsMember, CsStmt};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SourceMapFile {
    pub version: u32,
    pub source_file: PathBuf,
    pub generated_file: PathBuf,
    pub declaration: Option<SourceMapAnchor>,
    pub members: Vec<SourceMapAnchor>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SourceMapAnchor {
    pub kind: String,
    pub name: String,
    pub qualified_name: String,
    pub source_span: SourceMapSpan,
    pub generated_span: Option<SourceMapSpan>,
    pub generated_name_span: Option<SourceMapSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<SourceMapAnchor>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct SourceMapSpan {
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

pub fn source_map_path_for_generated(generated_file: &Path) -> PathBuf {
    let stem = generated_file
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("generated");
    generated_file.with_file_name(format!("{}.prsmmap.json", stem))
}

pub fn write_source_map(
    hir_file: &HirFile,
    generated_ir: &CsFile,
    generated_file: &Path,
    generated_source: &str,
) -> Result<PathBuf, String> {
    let path = source_map_path_for_generated(generated_file);
    let map = build_source_map(hir_file, generated_ir, generated_file, generated_source);
    let json = serde_json::to_string_pretty(&map)
        .map_err(|error| format!("Cannot serialize source map {}: {}", path.display(), error))?;
    fs::write(&path, json)
        .map_err(|error| format!("Cannot write source map {}: {}", path.display(), error))?;
    Ok(path)
}

pub fn build_source_map(
    hir_file: &HirFile,
    generated_ir: &CsFile,
    generated_file: &Path,
    generated_source: &str,
) -> SourceMapFile {
    let lines = generated_source.lines().collect::<Vec<_>>();
    let mut definitions = hir_file
        .definitions
        .iter()
        .filter(|definition| is_anchor_kind(definition.kind))
        .collect::<Vec<_>>();
    definitions.sort_by(|left, right| {
        left.span
            .start
            .line
            .cmp(&right.span.start.line)
            .then(left.span.start.col.cmp(&right.span.start.col))
            .then(left.qualified_name.cmp(&right.qualified_name))
    });

    let declaration_definition = definitions
        .iter()
        .copied()
        .find(|definition| definition.kind == HirDefinitionKind::Type);
    let member_definitions = definitions
        .into_iter()
        .filter(|definition| definition.kind != HirDefinitionKind::Type)
        .collect::<Vec<_>>();

    let declaration_generated = declaration_definition.and_then(|definition| find_declaration_anchor(&lines, definition));
    let class_header_line = declaration_generated.as_ref().map(|anchor| anchor.header_line).unwrap_or(1);
    let class_end_line = declaration_generated
        .as_ref()
        .map(|anchor| anchor.end_line)
        .unwrap_or(lines.len() as u32);

    let declaration = declaration_definition.map(|definition| SourceMapAnchor {
        kind: definition.kind.as_str().to_string(),
        name: definition.name.clone(),
        qualified_name: definition.qualified_name.clone(),
        source_span: SourceMapSpan::from_span(definition.span),
        generated_span: declaration_generated.as_ref().map(|anchor| anchor.generated_span),
        generated_name_span: declaration_generated.as_ref().map(|anchor| anchor.generated_name_span),
        segments: Vec::new(),
    });

    let mut found_members = Vec::with_capacity(member_definitions.len());
    let mut search_from_line = class_header_line.saturating_add(1);
    for definition in &member_definitions {
        let found = find_member_anchor(&lines, definition, search_from_line, class_end_line);
        if let Some(anchor) = found {
            search_from_line = anchor.header_line.saturating_add(1);
        }
        found_members.push(found);
    }

    let members = member_definitions
        .into_iter()
        .enumerate()
        .map(|(index, definition)| {
            let generated = found_members[index].as_ref().map(|anchor| {
                let next_header_start = found_members[index + 1..]
                    .iter()
                    .flatten()
                    .map(|candidate| candidate.start_line)
                    .next();
                let raw_end_line = next_header_start
                    .map(|line| line.saturating_sub(1))
                    .unwrap_or_else(|| class_end_line.saturating_sub(1).max(anchor.start_line));
                let end_line = find_previous_content_line(&lines, raw_end_line, anchor.start_line);
                GeneratedAnchor {
                    generated_span: SourceMapSpan {
                        line: anchor.start_line,
                        col: 1,
                        end_line,
                        end_col: line_end_col(lines.get(end_line.saturating_sub(1) as usize).copied()),
                    },
                    generated_name_span: anchor.generated_name_span,
                    start_line: anchor.start_line,
                    header_line: anchor.header_line,
                    end_line,
                }
            });

            SourceMapAnchor {
                kind: definition.kind.as_str().to_string(),
                name: definition.name.clone(),
                qualified_name: definition.qualified_name.clone(),
                source_span: SourceMapSpan::from_span(definition.span),
                generated_span: generated.as_ref().map(|anchor| anchor.generated_span),
                generated_name_span: generated.as_ref().map(|anchor| anchor.generated_name_span),
                segments: generated
                    .as_ref()
                    .map(|anchor| build_member_segments(lines.as_slice(), generated_ir, definition, anchor))
                    .unwrap_or_default(),
            }
        })
        .collect();

    SourceMapFile {
        version: 1,
        source_file: hir_file.path.clone(),
        generated_file: generated_file.to_path_buf(),
        declaration,
        members,
    }
}

fn build_member_segments(
    lines: &[&str],
    generated_ir: &CsFile,
    definition: &HirDefinition,
    generated_anchor: &GeneratedAnchor,
) -> Vec<SourceMapAnchor> {
    let Some(CsMember::Method { body, .. }) = find_method_member(generated_ir, &generated_member_name(definition)) else {
        return Vec::new();
    };

    let mut next_segment_id = 0u32;
    let (segments, _) = collect_statement_segments(
        lines,
        body,
        generated_anchor.header_line.saturating_add(2),
        &definition.qualified_name,
        &mut next_segment_id,
    );
    segments
}

fn find_method_member<'a>(generated_ir: &'a CsFile, generated_name: &str) -> Option<&'a CsMember> {
    generated_ir
        .class
        .members
        .iter()
        .find(|member| matches!(member, CsMember::Method { name, .. } if name == generated_name))
}

fn collect_statement_segments(
    lines: &[&str],
    statements: &[CsStmt],
    start_line: u32,
    parent_qualified_name: &str,
    next_segment_id: &mut u32,
) -> (Vec<SourceMapAnchor>, u32) {
    let mut segments = Vec::new();
    let mut current_line = start_line;

    for statement in statements {
        let (segment, next_line) = build_statement_segment(
            lines,
            statement,
            current_line,
            parent_qualified_name,
            next_segment_id,
        );
        if let Some(segment) = segment {
            segments.push(segment);
        }
        current_line = next_line;
    }

    (segments, current_line)
}

fn build_statement_segment(
    lines: &[&str],
    statement: &CsStmt,
    start_line: u32,
    parent_qualified_name: &str,
    next_segment_id: &mut u32,
) -> (Option<SourceMapAnchor>, u32) {
    let (children, next_line) = match statement {
        CsStmt::If { then_body, else_body, .. } => {
            let (mut children, after_then_body) = collect_statement_segments(
                lines,
                then_body,
                start_line.saturating_add(2),
                parent_qualified_name,
                next_segment_id,
            );
            let mut next_line = after_then_body.saturating_add(1);

            if let Some(else_body) = else_body {
                let (else_children, after_else_body) = collect_statement_segments(
                    lines,
                    else_body,
                    after_then_body.saturating_add(3),
                    parent_qualified_name,
                    next_segment_id,
                );
                children.extend(else_children);
                next_line = after_else_body.saturating_add(1);
            }

            (children, next_line)
        }
        CsStmt::Switch { cases, .. } => {
            let mut children = Vec::new();
            let mut line = start_line.saturating_add(2);
            for case in cases {
                let case_body_start = line.saturating_add(1);
                let (case_children, after_case_body) = collect_statement_segments(
                    lines,
                    &case.body,
                    case_body_start,
                    parent_qualified_name,
                    next_segment_id,
                );
                children.extend(case_children);
                line = after_case_body;
            }
            (children, line.saturating_add(1))
        }
        CsStmt::For { body, .. } | CsStmt::ForEach { body, .. } | CsStmt::While { body, .. } => {
            let (children, after_body) = collect_statement_segments(
                lines,
                body,
                start_line.saturating_add(2),
                parent_qualified_name,
                next_segment_id,
            );
            (children, after_body.saturating_add(1))
        }
        CsStmt::Block(statements, ..) => {
            collect_statement_segments(lines, statements, start_line, parent_qualified_name, next_segment_id)
        }
        CsStmt::Raw(code, ..) => (Vec::new(), start_line.saturating_add(raw_line_count(code))),
        _ => (Vec::new(), start_line.saturating_add(inline_statement_line_count(statement))),
    };

    let Some(source_span) = statement_source_span(statement) else {
        return (None, next_line);
    };

    *next_segment_id += 1;
    let segment_name = format!("stmt{}", next_segment_id);
    let end_line = next_line.saturating_sub(1).max(start_line);
    let generated_span = SourceMapSpan {
        line: start_line,
        col: 1,
        end_line,
        end_col: line_end_col(lines.get(end_line.saturating_sub(1) as usize).copied()),
    };

    (
        Some(SourceMapAnchor {
            kind: "statement".to_string(),
            name: segment_name.clone(),
            qualified_name: format!("{}#{}", parent_qualified_name, segment_name),
            source_span: SourceMapSpan::from_span(source_span),
            generated_span: Some(generated_span),
            generated_name_span: None,
            segments: children,
        }),
        next_line,
    )
}

fn raw_line_count(code: &str) -> u32 {
    code.lines().count().max(1) as u32
}

fn inline_statement_line_count(statement: &CsStmt) -> u32 {
    match statement {
        CsStmt::VarDecl { init, .. } => rendered_line_count(init),
        CsStmt::Assignment { value, .. } => rendered_line_count(value),
        CsStmt::Expr(expr, _) => rendered_line_count(expr),
        CsStmt::Return(Some(value), _) => rendered_line_count(value),
        CsStmt::YieldReturn(value, _) => rendered_line_count(value),
        _ => 1,
    }
}

fn rendered_line_count(rendered: &str) -> u32 {
    rendered.lines().count().max(1) as u32
}

fn statement_source_span(statement: &CsStmt) -> Option<Span> {
    match statement {
        CsStmt::VarDecl { source_span, .. }
        | CsStmt::Assignment { source_span, .. }
        | CsStmt::If { source_span, .. }
        | CsStmt::Switch { source_span, .. }
        | CsStmt::For { source_span, .. }
        | CsStmt::ForEach { source_span, .. }
        | CsStmt::While { source_span, .. } => *source_span,
        CsStmt::Expr(_, source_span)
        | CsStmt::Return(_, source_span)
        | CsStmt::YieldReturn(_, source_span)
        | CsStmt::Break(source_span)
        | CsStmt::Continue(source_span)
        | CsStmt::Raw(_, source_span)
        | CsStmt::Block(_, source_span)
        | CsStmt::TryCatch { source_span, .. }
        | CsStmt::Throw(_, source_span) => *source_span,
    }
}

fn is_anchor_kind(kind: HirDefinitionKind) -> bool {
    matches!(
        kind,
        HirDefinitionKind::Type
            | HirDefinitionKind::Field
            | HirDefinitionKind::Function
            | HirDefinitionKind::Coroutine
            | HirDefinitionKind::Lifecycle
            | HirDefinitionKind::EnumEntry
    )
}

fn find_declaration_anchor(lines: &[&str], definition: &HirDefinition) -> Option<GeneratedAnchor> {
    let generated_name = generated_type_name(definition);
    for (index, line) in lines.iter().enumerate() {
        let header_pattern = [
            format!("class {}", generated_name),
            format!("enum {}", generated_name),
            format!("struct {}", generated_name),
        ]
        .into_iter()
        .find(|pattern| line.contains(pattern));

        if header_pattern.is_none() {
            continue;
        }

        let header_line = (index + 1) as u32;
        let start_line = include_attribute_lines(lines, header_line, 1);
        let end_line = find_top_level_closing_line(lines, header_line).unwrap_or(lines.len() as u32);
        let name_col = line.find(&generated_name).map(|value| value as u32 + 1)?;
        let name_end_col = name_col + generated_name.chars().count() as u32 - 1;

        return Some(GeneratedAnchor {
            generated_span: SourceMapSpan {
                line: start_line,
                col: 1,
                end_line,
                end_col: line_end_col(lines.get(end_line.saturating_sub(1) as usize).copied()),
            },
            generated_name_span: SourceMapSpan {
                line: header_line,
                col: name_col,
                end_line: header_line,
                end_col: name_end_col,
            },
            start_line,
            header_line,
            end_line,
        });
    }

    None
}

fn find_member_anchor(
    lines: &[&str],
    definition: &HirDefinition,
    start_line: u32,
    class_end_line: u32,
) -> Option<GeneratedAnchor> {
    let generated_name = generated_member_name(definition);

    for line_index in start_line.max(1)..=class_end_line {
        let Some(line) = lines.get(line_index.saturating_sub(1) as usize).copied() else {
            break;
        };

        let name_col = match definition.kind {
            HirDefinitionKind::Field => find_field_name_col(lines, line_index, &generated_name),
            HirDefinitionKind::EnumEntry => find_enum_entry_name_col(line, &generated_name),
            HirDefinitionKind::Function | HirDefinitionKind::Coroutine | HirDefinitionKind::Lifecycle => {
                find_method_name_col(lines, line_index, &generated_name, class_end_line)
            }
            _ => None,
        };

        let Some(name_col) = name_col else {
            continue;
        };

        let header_line = line_index;
        let start_line = include_attribute_lines(lines, header_line, start_line);
        let name_end_col = name_col + generated_name.chars().count() as u32 - 1;
        return Some(GeneratedAnchor {
            generated_span: SourceMapSpan {
                line: start_line,
                col: 1,
                end_line: header_line,
                end_col: line_end_col(Some(line)),
            },
            generated_name_span: SourceMapSpan {
                line: header_line,
                col: name_col,
                end_line: header_line,
                end_col: name_end_col,
            },
            start_line,
            header_line,
            end_line: header_line,
        });
    }

    None
}

fn generated_type_name(definition: &HirDefinition) -> String {
    if definition.name.ends_with("Attribute") {
        return definition.name.clone();
    }

    [definition.name.clone(), format!("{}Attribute", definition.name)]
        .into_iter()
        .next()
        .unwrap_or_else(|| definition.name.clone())
}

fn generated_member_name(definition: &HirDefinition) -> String {
    if definition.kind != HirDefinitionKind::Lifecycle {
        return definition.name.clone();
    }

    match definition.name.as_str() {
        "awake" => "Awake".into(),
        "start" => "Start".into(),
        "update" => "Update".into(),
        "fixedUpdate" => "FixedUpdate".into(),
        "lateUpdate" => "LateUpdate".into(),
        "onEnable" => "OnEnable".into(),
        "onDisable" => "OnDisable".into(),
        "onDestroy" => "OnDestroy".into(),
        "onTriggerEnter" => "OnTriggerEnter".into(),
        "onTriggerExit" => "OnTriggerExit".into(),
        "onTriggerStay" => "OnTriggerStay".into(),
        "onCollisionEnter" => "OnCollisionEnter".into(),
        "onCollisionExit" => "OnCollisionExit".into(),
        "onCollisionStay" => "OnCollisionStay".into(),
        _ => definition.name.clone(),
    }
}

fn include_attribute_lines(lines: &[&str], header_line: u32, lower_bound: u32) -> u32 {
    let mut start = header_line;
    while start > lower_bound {
        let previous = lines
            .get(start.saturating_sub(2) as usize)
            .copied()
            .unwrap_or_default()
            .trim_start();
        if previous.starts_with('[') {
            start -= 1;
        } else {
            break;
        }
    }
    start
}

fn find_top_level_closing_line(lines: &[&str], header_line: u32) -> Option<u32> {
    let mut depth = 0u32;
    let mut saw_open_brace = false;

    for line_index in header_line..=lines.len() as u32 {
        let line = lines.get(line_index.saturating_sub(1) as usize)?;
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    saw_open_brace = true;
                }
                '}' => {
                    if depth == 0 {
                        continue;
                    }
                    depth -= 1;
                    if saw_open_brace && depth == 0 {
                        return Some(line_index);
                    }
                }
                _ => {}
            }
        }
    }

    None
}

fn find_method_name_col(lines: &[&str], line_index: u32, name: &str, class_end_line: u32) -> Option<u32> {
    let line = lines.get(line_index.saturating_sub(1) as usize)?.trim_end();
    let pattern = format!("{}(", name);
    if !line.contains(&pattern) || line.ends_with(';') {
        return None;
    }

    let next_line = next_non_empty_line(lines, line_index.saturating_add(1), class_end_line)?;
    if next_line.trim() != "{" {
        return None;
    }

    lines[line_index.saturating_sub(1) as usize]
        .find(&pattern)
        .map(|value| value as u32 + 1)
}

fn find_field_name_col(lines: &[&str], line_index: u32, name: &str) -> Option<u32> {
    let line = lines.get(line_index.saturating_sub(1) as usize)?.trim_end();
    if line.trim_start().starts_with('[') || line.trim() == "{" || line.trim() == "}" {
        return None;
    }

    let anchored_patterns = [
        format!(" {} =", name),
        format!(" {} =>", name),
        format!(" {};", name),
        format!(" {}\r", name),
    ];
    for pattern in anchored_patterns {
        if let Some(index) = lines[line_index.saturating_sub(1) as usize].find(&pattern) {
            return Some(index as u32 + 2);
        }
    }

    let property_pattern = format!(" {}", name);
    if !line.contains(&format!("{}(", name))
        && line.contains(&property_pattern)
        && next_non_empty_line(lines, line_index.saturating_add(1), line_index.saturating_add(2))
            .map(|next| next.trim() == "{")
            .unwrap_or(false)
    {
        return lines[line_index.saturating_sub(1) as usize]
            .find(&property_pattern)
            .map(|value| value as u32 + 2);
    }

    None
}

fn find_enum_entry_name_col(line: &str, name: &str) -> Option<u32> {
    let trimmed = line.trim_start();
    if trimmed.starts_with(&format!("{},", name)) {
        let indent = (line.len() - trimmed.len()) as u32;
        return Some(indent + 1);
    }
    None
}

fn next_non_empty_line<'a>(lines: &'a [&'a str], start_line: u32, end_line: u32) -> Option<&'a str> {
    for line_index in start_line..=end_line.min(lines.len() as u32) {
        let line = lines.get(line_index.saturating_sub(1) as usize)?;
        if !line.trim().is_empty() {
            return Some(*line);
        }
    }
    None
}

fn find_previous_content_line(lines: &[&str], mut line_index: u32, minimum: u32) -> u32 {
    line_index = line_index.min(lines.len() as u32);
    while line_index > minimum {
        if let Some(line) = lines.get(line_index.saturating_sub(1) as usize) {
            if !line.trim().is_empty() {
                return line_index;
            }
        }
        line_index -= 1;
    }
    minimum
}

fn line_end_col(line: Option<&str>) -> u32 {
    line.map(|value| value.chars().count() as u32)
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

impl SourceMapSpan {
    fn from_span(span: Span) -> Self {
        Self {
            line: span.start.line,
            col: span.start.col,
            end_line: span.end.line,
            end_col: span.end.col,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct GeneratedAnchor {
    generated_span: SourceMapSpan,
    generated_name_span: SourceMapSpan,
    start_line: u32,
    header_line: u32,
    end_line: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{HirDefinition, HirDefinitionKind, HirFile};
    use crate::lexer::token::{Position, Span};
    use crate::lowering::csharp_ir::{CsClass, CsFile, CsMember, CsStmt};
    use crate::semantic::types::{PrismType, PrimitiveKind};

    #[test]
    fn source_map_path_uses_prsmmap_extension() {
        let path = source_map_path_for_generated(Path::new("Generated/Player.cs"));
        assert_eq!(path, PathBuf::from("Generated/Player.prsmmap.json"));
    }

    #[test]
    fn build_source_map_tracks_declaration_and_members() {
        let hir_file = HirFile {
            path: PathBuf::from("Assets/Player.prsm"),
            definitions: vec![
                definition(1, "Player", "Player", HirDefinitionKind::Type, span(1, 11, 1, 16)),
                definition(2, "speed", "Player.speed", HirDefinitionKind::Field, span(2, 15, 2, 19)),
                definition(3, "update", "Player.update", HirDefinitionKind::Lifecycle, span(4, 5, 4, 10)),
                definition(4, "jump", "Player.jump", HirDefinitionKind::Function, span(8, 10, 8, 13)),
            ],
            references: vec![],
            pattern_bindings: vec![],
            listen_sites: vec![],
        };

        let generated = r#"// <auto-generated>
// This file was generated by the refraction compiler. Do not edit manually.
// </auto-generated>

using UnityEngine;

public class Player : MonoBehaviour
{
    [SerializeField]
    private float _speed = 5.0f;
    public float speed
    {
        get => _speed;
        set => _speed = value;
    }

    private void Update()
    {
        Debug.Log(speed);
    }

    public void jump()
    {
    }
}
"#;

        let map = build_source_map(&hir_file, &generated_ir(), Path::new("Generated/Player.cs"), generated);
        assert_eq!(map.version, 1);
        assert_eq!(map.declaration.as_ref().map(|anchor| anchor.name.as_str()), Some("Player"));
        assert_eq!(map.declaration.as_ref().and_then(|anchor| anchor.generated_name_span).map(|span| span.line), Some(7));
        assert_eq!(map.members.len(), 3);
        assert_eq!(map.members[0].name, "speed");
        assert_eq!(map.members[0].generated_name_span.map(|span| span.line), Some(11));
        assert_eq!(map.members[1].name, "update");
        assert_eq!(map.members[1].generated_name_span.map(|span| span.line), Some(17));
        assert_eq!(map.members[1].generated_name_span.map(|span| span.col), Some(18));
        assert_eq!(map.members[1].segments.len(), 1);
        assert_eq!(map.members[1].segments[0].source_span.line, 5);
        assert_eq!(map.members[1].segments[0].generated_span.map(|span| span.line), Some(19));
        assert_eq!(map.members[2].name, "jump");
        assert_eq!(map.members[2].generated_name_span.map(|span| span.line), Some(22));
    }

    fn generated_ir() -> CsFile {
        CsFile {
            header_comment: "// <auto-generated>".to_string(),
            usings: vec!["UnityEngine".to_string()],
            class: CsClass {
                attributes: vec![],
                modifiers: "public".to_string(),
                name: "Player".to_string(),
                base_class: Some("MonoBehaviour".to_string()),
                interfaces: vec![],
                where_clauses: vec![],
                members: vec![
                    CsMember::Field {
                        attributes: vec!["[SerializeField]".to_string()],
                        modifiers: "private".to_string(),
                        ty: "float".to_string(),
                        name: "_speed".to_string(),
                        init: Some("5.0f".to_string()),
                    },
                    CsMember::Property {
                        modifiers: "public".to_string(),
                        ty: "float".to_string(),
                        name: "speed".to_string(),
                        getter_expr: "_speed".to_string(),
                        setter: Some("set".to_string()),
                        setter_expr: Some("_speed".to_string()),
                    },
                    CsMember::Method {
                        attributes: vec![],
                        modifiers: "private".to_string(),
                        return_ty: "void".to_string(),
                        name: "Update".to_string(),
                        params: vec![],
                        where_clauses: vec![],
                        body: vec![CsStmt::Expr("Debug.Log(speed)".to_string(), Some(span(5, 9, 5, 24)))],
                        source_span: None,
                    },
                    CsMember::Method {
                        attributes: vec![],
                        modifiers: "public".to_string(),
                        return_ty: "void".to_string(),
                        name: "jump".to_string(),
                        params: vec![],
                        where_clauses: vec![],
                        body: vec![],
                        source_span: None,
                    },
                ],
            },
            extra_types: vec![],
        }
    }

    fn definition(
        id: u32,
        name: &str,
        qualified_name: &str,
        kind: HirDefinitionKind,
        span: Span,
    ) -> HirDefinition {
        HirDefinition {
            id,
            name: name.to_string(),
            qualified_name: qualified_name.to_string(),
            kind,
            ty: PrismType::Primitive(PrimitiveKind::Int),
            mutable: false,
            file_path: PathBuf::from("Assets/Player.prsm"),
            span,
        }
    }

    fn span(line: u32, col: u32, end_line: u32, end_col: u32) -> Span {
        Span {
            start: Position { line, col },
            end: Position { line: end_line, col: end_col },
        }
    }
}