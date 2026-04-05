use crate::lexer::token::{Position, Span};
use crate::semantic::types::PrismType;
use serde::Serialize;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HirDefinitionKind {
    Type,
    Field,
    Function,
    Coroutine,
    Lifecycle,
    Parameter,
    Local,
    EnumEntry,
}

impl HirDefinitionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Field => "field",
            Self::Function => "function",
            Self::Coroutine => "coroutine",
            Self::Lifecycle => "lifecycle",
            Self::Parameter => "parameter",
            Self::Local => "local",
            Self::EnumEntry => "enum-entry",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HirReferenceKind {
    Type,
    Identifier,
    Call,
    Member,
}

impl HirReferenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Identifier => "identifier",
            Self::Call => "call",
            Self::Member => "member",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HirDefinition {
    pub id: u32,
    pub name: String,
    pub qualified_name: String,
    pub kind: HirDefinitionKind,
    pub ty: PrismType,
    pub mutable: bool,
    pub file_path: PathBuf,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct HirReference {
    pub name: String,
    pub kind: HirReferenceKind,
    pub resolved_definition_id: Option<u32>,
    pub candidate_qualified_name: Option<String>,
    pub file_path: PathBuf,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct HirFile {
    pub path: PathBuf,
    pub definitions: Vec<HirDefinition>,
    pub references: Vec<HirReference>,
}

impl HirFile {
    pub fn find_definition(&self, id: u32) -> Option<&HirDefinition> {
        self.definitions.iter().find(|definition| definition.id == id)
    }

    pub fn find_definition_at(&self, line: u32, col: u32) -> Option<&HirDefinition> {
        let needle = Position { line, col };
        self.definitions
            .iter()
            .filter(|definition| span_contains(definition.span, needle))
            .min_by(|left, right| compare_span(left.span, right.span))
    }

    pub fn find_reference_at(&self, line: u32, col: u32) -> Option<&HirReference> {
        let needle = Position { line, col };
        self.references
            .iter()
            .filter(|reference| span_contains(reference.span, needle))
            .min_by(|left, right| compare_span(left.span, right.span))
    }

    pub fn find_definition_for_position(&self, line: u32, col: u32) -> Option<&HirDefinition> {
        if let Some(reference) = self.find_reference_at(line, col) {
            if let Some(definition_id) = reference.resolved_definition_id {
                if let Some(definition) = self.find_definition(definition_id) {
                    return Some(definition);
                }
            }
        }

        self.find_definition_at(line, col)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct HirProject {
    pub files: Vec<HirFile>,
    pub skipped_files: Vec<PathBuf>,
}

impl HirProject {
    pub fn stats(&self) -> HirStats {
        let definitions = self
            .files
            .iter()
            .map(|file| file.definitions.len())
            .sum::<usize>();
        let references = self
            .files
            .iter()
            .map(|file| file.references.len())
            .sum::<usize>();
        let resolved_references = self
            .files
            .iter()
            .flat_map(|file| &file.references)
            .filter(|reference| reference.resolved_definition_id.is_some())
            .count();

        HirStats {
            files_indexed: self.files.len(),
            files_skipped: self.skipped_files.len(),
            definitions,
            references,
            resolved_references,
            unresolved_references: references.saturating_sub(resolved_references),
        }
    }

    pub fn find_definition_for_position(
        &self,
        file_path: &Path,
        line: u32,
        col: u32,
    ) -> Option<&HirDefinition> {
        let file = self.files.iter().find(|file| file.path == file_path)?;

        if let Some(reference) = file.find_reference_at(line, col) {
            if let Some(definition_id) = reference.resolved_definition_id {
                if let Some(definition) = file.find_definition(definition_id) {
                    return Some(definition);
                }
            }

            if let Some(candidate_qualified_name) = &reference.candidate_qualified_name {
                if let Some(definition) = self.find_definition_by_qualified_name(candidate_qualified_name) {
                    return Some(definition);
                }
            }
        }

        file.find_definition_at(line, col)
    }

    pub fn find_definition_by_qualified_name(&self, qualified_name: &str) -> Option<&HirDefinition> {
        self.files
            .iter()
            .flat_map(|file| &file.definitions)
            .find(|definition| definition.qualified_name == qualified_name)
    }

    pub fn find_references_by_qualified_name(&self, qualified_name: &str) -> Vec<&HirReference> {
        let mut references = self
            .files
            .iter()
            .flat_map(|file| {
                file.references
                    .iter()
                    .filter(move |reference| reference_matches_qualified_name(file, reference, qualified_name))
            })
            .collect::<Vec<_>>();

        references.sort_by(|left, right| {
            left.file_path
                .cmp(&right.file_path)
                .then(compare_span(left.span, right.span))
        });

        references
    }

    pub fn find_references_for_position(
        &self,
        file_path: &Path,
        line: u32,
        col: u32,
    ) -> Option<(&HirDefinition, Vec<&HirReference>)> {
        let definition = self.find_definition_for_position(file_path, line, col)?;
        let references = self.find_references_by_qualified_name(&definition.qualified_name);
        Some((definition, references))
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HirStats {
    pub files_indexed: usize,
    pub files_skipped: usize,
    pub definitions: usize,
    pub references: usize,
    pub resolved_references: usize,
    pub unresolved_references: usize,
}

fn span_contains(span: Span, position: Position) -> bool {
    if position.line < span.start.line || position.line > span.end.line {
        return false;
    }
    if position.line == span.start.line && position.col < span.start.col {
        return false;
    }
    if position.line == span.end.line && position.col > span.end.col {
        return false;
    }
    true
}

fn reference_matches_qualified_name(
    file: &HirFile,
    reference: &HirReference,
    qualified_name: &str,
) -> bool {
    if reference.candidate_qualified_name.as_deref() == Some(qualified_name) {
        return true;
    }

    reference
        .resolved_definition_id
        .and_then(|definition_id| file.find_definition(definition_id))
        .map(|definition| definition.qualified_name == qualified_name)
        .unwrap_or(false)
}

fn compare_span(left: Span, right: Span) -> Ordering {
    span_size(left)
        .cmp(&span_size(right))
        .then(left.start.line.cmp(&right.start.line))
        .then(left.start.col.cmp(&right.start.col))
}

fn span_size(span: Span) -> u64 {
    let line_delta = span.end.line.saturating_sub(span.start.line) as u64;
    let col_delta = span.end.col.saturating_sub(span.start.col) as u64;
    line_delta * 10_000 + col_delta
}