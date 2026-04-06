use crate::hir::HirProject;
use crate::project_index::{DeclarationKind, DeclarationSummary, MemberKind, MemberSummary, ProjectIndex};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone)]
enum CompletionContext {
    Member(MemberCompletionContext),
    Type(TypeCompletionContext),
    General(GeneralCompletionContext),
}

impl CompletionContext {
    fn prefix(&self) -> &str {
        match self {
            Self::Member(context) => &context.prefix,
            Self::Type(context) => &context.prefix,
            Self::General(context) => &context.prefix,
        }
    }
}

#[derive(Debug, Clone)]
struct MemberCompletionContext {
    receiver: String,
    receiver_chain: Vec<String>,
    prefix: String,
    receiver_col: u32,
}

#[derive(Debug, Clone)]
struct TypeCompletionContext {
    prefix: String,
}

#[derive(Debug, Clone)]
struct GeneralCompletionContext {
    prefix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CompletionEntryKind {
    Keyword,
    Function,
    Method,
    Property,
    Field,
    Class,
    Struct,
    Enum,
    EnumMember,
    TypeParameter,
    Event,
}

#[derive(Debug, Clone)]
struct CompletionEntry {
    label: String,
    kind: CompletionEntryKind,
    detail: String,
    documentation: Option<String>,
    insert_text: Option<String>,
    sort_group: u8,
}

#[derive(Debug, Clone, Copy)]
struct BuiltinCompletion {
    name: &'static str,
    signature: &'static str,
    description: &'static str,
    prsm_only: bool,
}

#[derive(Debug, Clone, Copy)]
struct CoreMember {
    name: &'static str,
    kind: CompletionEntryKind,
    signature: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct CoreType {
    name: &'static str,
    namespace: &'static str,
    kind: CompletionEntryKind,
    kind_label: &'static str,
    description: &'static str,
    members: &'static [CoreMember],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SidecarCompletionQuery {
    pub type_name: String,
    pub prefix: String,
    pub include_instance_members: bool,
    pub include_static_members: bool,
}

pub(crate) fn completion_items(
    document_text: &str,
    line: u32,
    col: u32,
    runtime_path: &Path,
    project_index: &ProjectIndex,
    hir_project: &HirProject,
) -> Vec<Value> {
    let context = completion_context_at(document_text, line, col);
    let entries = match &context {
        CompletionContext::Member(member_context) => {
            build_member_completion_entries(member_context, document_text, line, runtime_path, project_index, hir_project)
        }
        CompletionContext::Type(_) => build_type_completion_entries(project_index),
        CompletionContext::General(_) => build_general_completion_entries(project_index, runtime_path),
    };

    completion_entries_json(entries, context.prefix())
}

pub(crate) fn sidecar_completion_query(
    document_text: &str,
    line: u32,
    col: u32,
    runtime_path: &Path,
    project_index: &ProjectIndex,
    hir_project: &HirProject,
) -> Option<SidecarCompletionQuery> {
    let CompletionContext::Member(context) = completion_context_at(document_text, line, col) else {
        return None;
    };

    let receiver_type = resolve_receiver_type(
        &context,
        document_text,
        line,
        runtime_path,
        project_index,
        hir_project,
    )?;
    if !type_prefers_sidecar(&receiver_type, project_index) {
        return None;
    }

    let is_static_access = is_static_receiver(&context.receiver, &receiver_type);
    Some(SidecarCompletionQuery {
        type_name: canonical_type_name(&receiver_type),
        prefix: context.prefix,
        include_instance_members: !is_static_access,
        include_static_members: is_static_access,
    })
}

fn completion_context_at(text: &str, line: u32, col: u32) -> CompletionContext {
    let line_prefix = line_prefix_at(text, line, col);

    if let Some(context) = parse_member_completion_context(&line_prefix) {
        return CompletionContext::Member(context);
    }

    if let Some(context) = parse_type_completion_context(&line_prefix) {
        return CompletionContext::Type(context);
    }

    CompletionContext::General(GeneralCompletionContext {
        prefix: identifier_prefix(&line_prefix).to_string(),
    })
}

fn line_prefix_at(text: &str, line: u32, col: u32) -> String {
    let Some(line_text) = text.lines().nth(line.saturating_sub(1) as usize) else {
        return String::new();
    };
    let end = clamp_index(col.saturating_sub(1) as usize, line_text.len());
    line_text[..end].to_string()
}

fn parse_member_completion_context(line_prefix: &str) -> Option<MemberCompletionContext> {
    let prefix_start = identifier_start(line_prefix, line_prefix.len());
    let prefix = &line_prefix[prefix_start..];
    let before_prefix = &line_prefix[..prefix_start];
    if !before_prefix.ends_with('.') {
        return None;
    }

    let mut receiver_end = before_prefix.len().saturating_sub(1);
    if receiver_end > 0 && before_prefix.as_bytes()[receiver_end - 1] == b'?' {
        receiver_end = receiver_end.saturating_sub(1);
    }

    let receiver_start = identifier_start(before_prefix, receiver_end);
    if receiver_start == receiver_end {
        return None;
    }

    Some(MemberCompletionContext {
        receiver: before_prefix[receiver_start..receiver_end].to_string(),
        receiver_chain: receiver_chain(before_prefix, receiver_start, receiver_end),
        prefix: prefix.to_string(),
        receiver_col: receiver_start as u32 + 1,
    })
}

fn parse_type_completion_context(line_prefix: &str) -> Option<TypeCompletionContext> {
    let prefix_start = identifier_start(line_prefix, line_prefix.len());
    let mut boundary = prefix_start;
    while boundary > 0 && line_prefix.as_bytes()[boundary - 1].is_ascii_whitespace() {
        boundary -= 1;
    }

    if boundary > 0 && matches!(line_prefix.as_bytes()[boundary - 1], b':' | b'<') {
        return Some(TypeCompletionContext {
            prefix: line_prefix[prefix_start..].to_string(),
        });
    }

    None
}

fn identifier_prefix(line_prefix: &str) -> &str {
    let start = identifier_start(line_prefix, line_prefix.len());
    &line_prefix[start..]
}

fn receiver_chain(before_prefix: &str, receiver_start: usize, receiver_end: usize) -> Vec<String> {
    let mut segments = vec![before_prefix[receiver_start..receiver_end].to_string()];
    let mut current_start = receiver_start;

    while current_start > 0 {
        let mut separator_index = current_start;
        if before_prefix.as_bytes()[separator_index - 1] != b'.' {
            break;
        }
        separator_index -= 1;
        if separator_index > 0 && before_prefix.as_bytes()[separator_index - 1] == b'?' {
            separator_index -= 1;
        }

        let previous_end = separator_index;
        let previous_start = identifier_start(before_prefix, previous_end);
        if previous_start == previous_end {
            break;
        }

        segments.push(before_prefix[previous_start..previous_end].to_string());
        current_start = previous_start;
    }

    segments.reverse();
    segments
}

fn build_member_completion_entries(
    context: &MemberCompletionContext,
    document_text: &str,
    line: u32,
    runtime_path: &Path,
    project_index: &ProjectIndex,
    hir_project: &HirProject,
) -> Vec<CompletionEntry> {
    let Some(receiver_type) = resolve_receiver_type(context, document_text, line, runtime_path, project_index, hir_project) else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    let mut visited = HashSet::new();
    collect_member_completion_entries(&receiver_type, project_index, &mut visited, &mut entries);
    entries
}

fn build_type_completion_entries(project_index: &ProjectIndex) -> Vec<CompletionEntry> {
    let mut entries = primitive_type_completion_entries();
    entries.extend(core_type_completion_entries());
    entries.extend(project_type_completion_entries(project_index));
    entries
}

fn build_general_completion_entries(project_index: &ProjectIndex, runtime_path: &Path) -> Vec<CompletionEntry> {
    let mut entries = keyword_completion_entries();
    entries.extend(builtin_completion_entries());
    entries.extend(build_type_completion_entries(project_index));

    if let Some(file_summary) = find_file_summary(project_index, runtime_path) {
        entries.extend(
            file_summary
                .members
                .iter()
                .map(|member| project_member_completion_entry(member, &file_summary.name, 0)),
        );
    }

    entries
}

fn resolve_receiver_type(
    context: &MemberCompletionContext,
    document_text: &str,
    line: u32,
    runtime_path: &Path,
    project_index: &ProjectIndex,
    hir_project: &HirProject,
) -> Option<String> {
    if let Some(type_name) = resolve_identifier_type(
        &context.receiver,
        document_text,
        line,
        context.receiver_col,
        runtime_path,
        project_index,
        hir_project,
    ) {
        return Some(type_name);
    }

    if context.receiver_chain.len() > 1 {
        let mut current_type = resolve_chain_root_type(&context.receiver_chain[0], document_text, runtime_path, project_index)?;
        for member_name in context.receiver_chain.iter().skip(1) {
            current_type = resolve_member_result_type(&current_type, member_name, project_index)?;
        }
        return Some(current_type);
    }

    None
}

fn resolve_identifier_type(
    receiver: &str,
    document_text: &str,
    line: u32,
    receiver_col: u32,
    runtime_path: &Path,
    project_index: &ProjectIndex,
    hir_project: &HirProject,
) -> Option<String> {
    if receiver == "this" {
        return find_file_summary(project_index, runtime_path).map(|summary| summary.name.clone());
    }

    if let Some(type_name) = builtin_receiver_type(receiver) {
        return Some(type_name.to_string());
    }

    if let Some(definition) = hir_project.find_definition_for_position(runtime_path, line, receiver_col) {
        return Some(canonical_type_name(&definition.ty.display_name()));
    }

    if let Some(type_name) = resolve_receiver_type_from_text(document_text, receiver) {
        return Some(type_name);
    }

    if let Some(core_type) = core_type_by_name(receiver) {
        return Some(core_type.name.to_string());
    }

    project_declaration_by_name(project_index, receiver)
        .filter(|declaration| declaration.kind == DeclarationKind::Enum)
        .map(|declaration| declaration.name.clone())
}

fn resolve_chain_root_type(
    receiver: &str,
    document_text: &str,
    runtime_path: &Path,
    project_index: &ProjectIndex,
) -> Option<String> {
    if receiver == "this" {
        return find_file_summary(project_index, runtime_path).map(|summary| summary.name.clone());
    }

    if let Some(type_name) = builtin_receiver_type(receiver) {
        return Some(type_name.to_string());
    }

    if let Some(type_name) = resolve_receiver_type_from_text(document_text, receiver) {
        return Some(type_name);
    }

    core_type_by_name(receiver).map(|core_type| core_type.name.to_string())
}

fn resolve_receiver_type_from_text(text: &str, receiver: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        for prefix in [
            "require",
            "optional",
            "serialize",
            "child",
            "parent",
            "var",
            "val",
            "private",
            "public",
            "protected",
        ] {
            if let Some(type_name) = parse_named_type_annotation(trimmed, prefix, receiver) {
                return Some(type_name);
            }
        }

        if let Some(type_name) = parse_parameter_type_annotation(trimmed, receiver) {
            return Some(type_name);
        }
    }

    None
}

fn parse_named_type_annotation(line: &str, prefix: &str, receiver: &str) -> Option<String> {
    let after_prefix = line.strip_prefix(prefix)?.trim_start();
    let after_receiver = strip_identifier_prefix(after_prefix, receiver)?.trim_start();
    let type_fragment = after_receiver.strip_prefix(':')?.trim_start();
    Some(canonical_type_name(&read_type_fragment(type_fragment)))
}

fn parse_parameter_type_annotation(line: &str, receiver: &str) -> Option<String> {
    let open_paren = line.find('(')?;
    let close_paren = line[open_paren + 1..].find(')')? + open_paren + 1;
    let params = &line[open_paren + 1..close_paren];

    for param in params.split(',') {
        let param = param.trim();
        let after_receiver = strip_identifier_prefix(param, receiver)?.trim_start();
        let type_fragment = after_receiver.strip_prefix(':')?.trim_start();
        return Some(canonical_type_name(&read_type_fragment(type_fragment)));
    }

    None
}

fn strip_identifier_prefix<'a>(text: &'a str, identifier: &str) -> Option<&'a str> {
    let remainder = text.strip_prefix(identifier)?;
    if remainder
        .chars()
        .next()
        .map(is_identifier_char)
        .unwrap_or(false)
    {
        return None;
    }
    Some(remainder)
}

fn read_type_fragment(fragment: &str) -> String {
    let mut end = 0;
    for (index, ch) in fragment.char_indices() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '?' | '<' | '>' | '[' | ']') {
            end = index + ch.len_utf8();
        } else {
            break;
        }
    }
    fragment[..end].trim().to_string()
}

fn resolve_member_result_type(type_name: &str, member_name: &str, project_index: &ProjectIndex) -> Option<String> {
    let canonical_type = canonical_type_name(type_name);

    if let Some(declaration) = project_declaration_by_name(project_index, &canonical_type) {
        if let Some(member) = declaration
            .members
            .iter()
            .find(|member| member.name.eq_ignore_ascii_case(member_name))
        {
            return member_result_type(member);
        }
        if let Some(base_type) = declaration.base_type.as_ref() {
            return resolve_member_result_type(base_type, member_name, project_index);
        }
    }

    if let Some(core_type) = core_type_by_name(&canonical_type) {
        if let Some(member) = core_type
            .members
            .iter()
            .find(|member| member.name.eq_ignore_ascii_case(member_name))
        {
            return signature_result_type(member.signature);
        }
    }

    None
}

fn member_result_type(member: &MemberSummary) -> Option<String> {
    signature_result_type(&member.signature)
}

fn signature_result_type(signature: &str) -> Option<String> {
    if let Some(index) = signature.rfind("): ") {
        return Some(canonical_type_name(&signature[index + 3..]));
    }
    if let Some(index) = signature.rfind(": ") {
        return Some(canonical_type_name(&signature[index + 2..]));
    }
    None
}

fn collect_member_completion_entries(
    type_name: &str,
    project_index: &ProjectIndex,
    visited: &mut HashSet<String>,
    entries: &mut Vec<CompletionEntry>,
) {
    let canonical_type = canonical_type_name(type_name);
    if !visited.insert(canonical_type.clone()) {
        return;
    }

    if let Some(declaration) = project_declaration_by_name(project_index, &canonical_type) {
        entries.extend(
            declaration
                .members
                .iter()
                .map(|member| project_member_completion_entry(member, &declaration.name, 0)),
        );
        if let Some(base_type) = declaration.base_type.as_ref() {
            collect_member_completion_entries(base_type, project_index, visited, entries);
        }
    }

    if let Some(core_type) = core_type_by_name(&canonical_type) {
        entries.extend(core_type.members.iter().map(|member| core_member_completion_entry(member, core_type)));
    }
}

fn keyword_completion_entries() -> Vec<CompletionEntry> {
    PRSM_KEYWORDS
        .iter()
        .map(|keyword| CompletionEntry {
            label: (*keyword).to_string(),
            kind: CompletionEntryKind::Keyword,
            detail: "Keyword (prsm)".into(),
            documentation: None,
            insert_text: None,
            sort_group: 2,
        })
        .collect()
}

fn builtin_completion_entries() -> Vec<CompletionEntry> {
    PRSM_BUILTINS
        .iter()
        .map(|builtin| CompletionEntry {
            label: builtin.name.to_string(),
            kind: CompletionEntryKind::Function,
            detail: if builtin.prsm_only {
                "Builtin (prsm)".into()
            } else {
                "Builtin".into()
            },
            documentation: Some(format!(
                "```prsm\n{}{}\n```\n\n{}",
                builtin.name, builtin.signature, builtin.description
            )),
            insert_text: Some(builtin_snippet(builtin)),
            sort_group: 1,
        })
        .collect()
}

fn primitive_type_completion_entries() -> Vec<CompletionEntry> {
    PRIMITIVE_TYPE_NAMES
        .iter()
        .map(|name| CompletionEntry {
            label: (*name).to_string(),
            kind: CompletionEntryKind::TypeParameter,
            detail: "Primitive (prsm)".into(),
            documentation: None,
            insert_text: None,
            sort_group: 3,
        })
        .collect()
}

fn core_type_completion_entries() -> Vec<CompletionEntry> {
    CORE_TYPES
        .iter()
        .filter(|core_type| !PRIMITIVE_TYPE_NAMES.contains(&core_type.name))
        .map(core_type_completion_entry)
        .collect()
}

fn project_type_completion_entries(project_index: &ProjectIndex) -> Vec<CompletionEntry> {
    project_index
        .files
        .iter()
        .map(|file| project_declaration_completion_entry(&file.declaration))
        .collect()
}

fn project_declaration_completion_entry(declaration: &DeclarationSummary) -> CompletionEntry {
    let mut documentation = vec![format!("```prsm\n{}\n```", declaration.signature)];
    if let Some(base_type) = declaration.base_type.as_ref() {
        documentation.push(format!("Extends {}", base_type));
    }
    if !declaration.interfaces.is_empty() {
        documentation.push(format!("Implements {}", declaration.interfaces.join(", ")));
    }
    if !declaration.members.is_empty() {
        documentation.push(format!("Members: {}", declaration.members.len()));
    }

    CompletionEntry {
        label: declaration.name.clone(),
        kind: declaration_completion_kind(declaration.kind),
        detail: declaration.kind.as_str().to_string(),
        documentation: Some(documentation.join("\n\n")),
        insert_text: None,
        sort_group: 4,
    }
}

fn project_member_completion_entry(member: &MemberSummary, container_name: &str, sort_group: u8) -> CompletionEntry {
    let kind = member_completion_kind(member.kind);
    CompletionEntry {
        label: member.name.clone(),
        kind,
        detail: member.signature.clone(),
        documentation: Some(format!(
            "```prsm\n{}\n```\n\nDefined on {}.",
            member.signature, container_name
        )),
        insert_text: completion_snippet_for_signature(&member.name, &member.signature, kind),
        sort_group,
    }
}

fn core_type_completion_entry(core_type: &CoreType) -> CompletionEntry {
    CompletionEntry {
        label: core_type.name.to_string(),
        kind: core_type.kind,
        detail: if core_type.namespace.is_empty() {
            core_type.kind_label.to_string()
        } else {
            format!("{} ({})", core_type.kind_label, core_type.namespace)
        },
        documentation: Some(format!(
            "```prsm\n{} {}\n```\n\n{}",
            core_type.kind_label.to_ascii_lowercase(),
            core_type.name,
            core_type.description,
        )),
        insert_text: None,
        sort_group: 5,
    }
}

fn core_member_completion_entry(member: &CoreMember, core_type: &CoreType) -> CompletionEntry {
    CompletionEntry {
        label: member.name.to_string(),
        kind: member.kind,
        detail: member.signature.to_string(),
        documentation: Some(format!(
            "```prsm\n{}\n```\n\n{}\n\n_{}_.",
            member.signature,
            member.description,
            core_type.name,
        )),
        insert_text: completion_snippet_for_signature(member.name, member.signature, member.kind),
        sort_group: 0,
    }
}

fn completion_entries_json(entries: Vec<CompletionEntry>, prefix: &str) -> Vec<Value> {
    let prefix = prefix.to_ascii_lowercase();
    let mut unique = Vec::new();
    let mut seen = HashSet::new();

    for entry in entries {
        let label_key = entry.label.to_ascii_lowercase();
        if !prefix.is_empty() && !label_key.starts_with(&prefix) {
            continue;
        }
        if !seen.insert(label_key) {
            continue;
        }
        unique.push(entry);
    }

    unique.sort_by(|left, right| {
        left.sort_group
            .cmp(&right.sort_group)
            .then_with(|| left.label.to_ascii_lowercase().cmp(&right.label.to_ascii_lowercase()))
    });

    unique.into_iter().take(200).map(completion_entry_json).collect()
}

fn completion_entry_json(entry: CompletionEntry) -> Value {
    let mut value = Map::new();
    value.insert("label".into(), json!(entry.label));
    value.insert("kind".into(), json!(completion_item_kind_number(entry.kind)));
    value.insert("detail".into(), json!(entry.detail));
    if let Some(documentation) = entry.documentation {
        value.insert(
            "documentation".into(),
            json!({
                "kind": "markdown",
                "value": documentation,
            }),
        );
    }
    if let Some(insert_text) = entry.insert_text {
        value.insert("insertText".into(), json!(insert_text));
        value.insert("insertTextFormat".into(), json!(2));
    }
    Value::Object(value)
}

fn completion_snippet_for_signature(
    name: &str,
    signature: &str,
    kind: CompletionEntryKind,
) -> Option<String> {
    match kind {
        CompletionEntryKind::Method | CompletionEntryKind::Function => {
            if signature.contains("<T>()") {
                Some(format!("{}<$1>()", name))
            } else if signature.contains("()") {
                Some(format!("{}()", name))
            } else {
                Some(format!("{}($1)", name))
            }
        }
        _ => None,
    }
}

fn builtin_snippet(builtin: &BuiltinCompletion) -> String {
    if builtin.signature == "<T>()" {
        format!("{}<$1>()", builtin.name)
    } else {
        format!("{}($1)", builtin.name)
    }
}

fn builtin_receiver_type(receiver: &str) -> Option<&'static str> {
    match receiver {
        "gameObject" => Some("GameObject"),
        "transform" => Some("Transform"),
        "input" => Some("Input"),
        "Time" => Some("Time"),
        "Debug" => Some("Debug"),
        "Physics" => Some("Physics"),
        "Mathf" => Some("Mathf"),
        "Application" => Some("Application"),
        "SceneManager" => Some("SceneManager"),
        _ => None,
    }
}

fn find_file_summary<'a>(project_index: &'a ProjectIndex, runtime_path: &Path) -> Option<FileSummaryRef<'a>> {
    project_index.files.iter().find(|file| file.path == runtime_path).map(|file| FileSummaryRef {
        name: file.declaration.name.clone(),
        members: &file.declaration.members,
    })
}

struct FileSummaryRef<'a> {
    name: String,
    members: &'a [MemberSummary],
}

fn project_declaration_by_name<'a>(project_index: &'a ProjectIndex, type_name: &str) -> Option<&'a DeclarationSummary> {
    let canonical_type = canonical_type_name(type_name);
    project_index
        .files
        .iter()
        .map(|file| &file.declaration)
        .find(|declaration| declaration.name == canonical_type)
}

fn declaration_completion_kind(kind: DeclarationKind) -> CompletionEntryKind {
    match kind {
        DeclarationKind::Component | DeclarationKind::Class | DeclarationKind::Asset | DeclarationKind::Attribute => CompletionEntryKind::Class,
        DeclarationKind::DataClass => CompletionEntryKind::Struct,
        DeclarationKind::Enum => CompletionEntryKind::Enum,
        DeclarationKind::Interface => CompletionEntryKind::Class,
    }
}

fn member_completion_kind(kind: MemberKind) -> CompletionEntryKind {
    match kind {
        MemberKind::Field
        | MemberKind::SerializeField
        | MemberKind::RequiredComponent
        | MemberKind::OptionalComponent
        | MemberKind::ChildComponent
        | MemberKind::ParentComponent => CompletionEntryKind::Field,
        MemberKind::Function | MemberKind::Coroutine | MemberKind::Lifecycle => CompletionEntryKind::Method,
        MemberKind::EnumEntry => CompletionEntryKind::EnumMember,
    }
}

fn core_type_by_name(type_name: &str) -> Option<&'static CoreType> {
    let canonical_type = canonical_type_name(type_name);
    CORE_TYPES
        .iter()
        .find(|core_type| core_type.name.eq_ignore_ascii_case(&canonical_type))
}

pub(crate) fn unity_docs_url_for_type(type_name: &str) -> Option<String> {
    const UNITY_SCRIPT_REFERENCE_BASE: &str = "https://docs.unity3d.com/6000.3/Documentation/ScriptReference";

    let core_type = core_type_by_name(type_name)?;
    if !core_type.namespace.starts_with("Unity") {
        return None;
    }

    Some(format!("{}/{}.html", UNITY_SCRIPT_REFERENCE_BASE, core_type.name))
}

pub(crate) fn core_type_namespace(type_name: &str) -> Option<&'static str> {
    core_type_by_name(type_name).map(|core_type| core_type.namespace)
}

pub(crate) fn core_type_is_unity(type_name: &str) -> bool {
    core_type_namespace(type_name)
        .map(|namespace| namespace.starts_with("Unity"))
        .unwrap_or(false)
}

fn type_prefers_sidecar(type_name: &str, project_index: &ProjectIndex) -> bool {
    let canonical_type = canonical_type_name(type_name);
    if PRIMITIVE_TYPE_NAMES.contains(&canonical_type.as_str()) {
        return false;
    }
    if project_declaration_by_name(project_index, &canonical_type).is_some() {
        return false;
    }
    if let Some(namespace) = core_type_namespace(&canonical_type) {
        return namespace.starts_with("Unity");
    }

    true
}

fn is_static_receiver(receiver: &str, receiver_type: &str) -> bool {
    if receiver == "input" {
        return true;
    }

    let canonical_receiver = canonical_type_name(receiver);
    let canonical_type = canonical_type_name(receiver_type);
    if canonical_receiver == canonical_type {
        return true;
    }

    receiver
        .chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
        && core_type_by_name(receiver).is_some()
}

fn canonical_type_name(type_name: &str) -> String {
    let trimmed = type_name.trim();
    let without_nullable = trimmed.trim_end_matches('?');
    let without_namespace = without_nullable
        .rsplit_once('.')
        .map(|(_, tail)| tail)
        .unwrap_or(without_nullable);
    without_namespace
        .split('<')
        .next()
        .unwrap_or(without_namespace)
        .trim()
        .to_string()
}

fn completion_item_kind_number(kind: CompletionEntryKind) -> u32 {
    match kind {
        CompletionEntryKind::Method => 2,
        CompletionEntryKind::Function => 3,
        CompletionEntryKind::Field => 5,
        CompletionEntryKind::Class => 7,
        CompletionEntryKind::Property => 10,
        CompletionEntryKind::Enum => 13,
        CompletionEntryKind::Keyword => 14,
        CompletionEntryKind::EnumMember => 20,
        CompletionEntryKind::Struct => 22,
        CompletionEntryKind::Event => 23,
        CompletionEntryKind::TypeParameter => 25,
    }
}

fn identifier_start(text: &str, end: usize) -> usize {
    let bytes = text.as_bytes();
    let mut index = end;
    while index > 0 && is_identifier_byte(bytes[index - 1]) {
        index -= 1;
    }
    index
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn clamp_index(value: usize, max: usize) -> usize {
    value.min(max)
}

const PRIMITIVE_TYPE_NAMES: &[&str] = &["Int", "Float", "Double", "Bool", "String", "Long", "Byte", "Unit"];

const PRSM_KEYWORDS: &[&str] = &[
    "component",
    "asset",
    "class",
    "data",
    "enum",
    "serialize",
    "require",
    "optional",
    "child",
    "parent",
    "val",
    "var",
    "func",
    "coroutine",
    "override",
    "return",
    "if",
    "else",
    "when",
    "for",
    "while",
    "in",
    "until",
    "break",
    "continue",
    "wait",
    "start",
    "stop",
    "stopAll",
    "listen",
    "intrinsic",
    "using",
    "null",
    "this",
    "true",
    "false",
    "awake",
    "update",
    "fixedUpdate",
    "lateUpdate",
    "onEnable",
    "onDisable",
    "onDestroy",
    "onTriggerEnter",
    "onTriggerExit",
    "onCollisionEnter",
    "onCollisionExit",
    "nextFrame",
    "fixedFrame",
    "public",
    "private",
    "protected",
];

const PRSM_BUILTINS: &[BuiltinCompletion] = &[
    BuiltinCompletion { name: "vec2", signature: "(x, y)", description: "Create Vector2", prsm_only: true },
    BuiltinCompletion { name: "vec3", signature: "(x, y, z)", description: "Create Vector3", prsm_only: true },
    BuiltinCompletion { name: "color", signature: "(r, g, b, a)", description: "Create Color", prsm_only: true },
    BuiltinCompletion { name: "get", signature: "<T>()", description: "GetComponent<T>()", prsm_only: true },
    BuiltinCompletion { name: "find", signature: "<T>()", description: "FindFirstObjectByType<T>()", prsm_only: true },
    BuiltinCompletion { name: "Destroy", signature: "(obj)", description: "Destroy object", prsm_only: false },
    BuiltinCompletion { name: "print", signature: "(message, level?)", description: "Debug.Log / LogWarning / LogError", prsm_only: true },
    BuiltinCompletion { name: "log", signature: "(message)", description: "Debug.Log(message)", prsm_only: true },
    BuiltinCompletion { name: "warn", signature: "(message)", description: "Debug.LogWarning(message)", prsm_only: true },
    BuiltinCompletion { name: "error", signature: "(message)", description: "Debug.LogError(message)", prsm_only: true },
];

const MONO_BEHAVIOUR_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "gameObject", kind: CompletionEntryKind::Property, signature: "gameObject: GameObject", description: "The game object this component is attached to." },
    CoreMember { name: "transform", kind: CompletionEntryKind::Property, signature: "transform: Transform", description: "The Transform attached to this game object." },
    CoreMember { name: "enabled", kind: CompletionEntryKind::Property, signature: "enabled: Bool", description: "Enabled state of the component." },
    CoreMember { name: "tag", kind: CompletionEntryKind::Property, signature: "tag: String", description: "The tag of this game object." },
    CoreMember { name: "name", kind: CompletionEntryKind::Property, signature: "name: String", description: "The name of this object." },
    CoreMember { name: "destroyCancellationToken", kind: CompletionEntryKind::Property, signature: "destroyCancellationToken: CancellationToken", description: "Raised when the behaviour is destroyed." },
    CoreMember { name: "didAwake", kind: CompletionEntryKind::Property, signature: "didAwake: Bool", description: "Whether Awake has already run." },
    CoreMember { name: "didStart", kind: CompletionEntryKind::Property, signature: "didStart: Bool", description: "Whether Start has already run." },
    CoreMember { name: "runInEditMode", kind: CompletionEntryKind::Property, signature: "runInEditMode: Bool", description: "Allows the behaviour to run in edit mode." },
    CoreMember { name: "getComponent", kind: CompletionEntryKind::Method, signature: "getComponent<T>(): T", description: "Gets a component on this object." },
    CoreMember { name: "getComponentInChildren", kind: CompletionEntryKind::Method, signature: "getComponentInChildren<T>(): T", description: "Gets a component from children." },
    CoreMember { name: "getComponentInParent", kind: CompletionEntryKind::Method, signature: "getComponentInParent<T>(): T", description: "Gets a component from parents." },
    CoreMember { name: "startCoroutine", kind: CompletionEntryKind::Method, signature: "startCoroutine(routine: IEnumerator): Coroutine", description: "Starts a coroutine." },
    CoreMember { name: "stopCoroutine", kind: CompletionEntryKind::Method, signature: "stopCoroutine(routine: Coroutine): Unit", description: "Stops a coroutine." },
    CoreMember { name: "stopAllCoroutines", kind: CompletionEntryKind::Method, signature: "stopAllCoroutines(): Unit", description: "Stops all coroutines." },
    CoreMember { name: "invoke", kind: CompletionEntryKind::Method, signature: "invoke(methodName: String, time: Float): Unit", description: "Invokes a method after a delay." },
    CoreMember { name: "cancelInvoke", kind: CompletionEntryKind::Method, signature: "cancelInvoke(): Unit", description: "Cancels pending invokes." },
    CoreMember { name: "isInvoking", kind: CompletionEntryKind::Method, signature: "isInvoking(methodName: String): Bool", description: "Checks whether an invoke is pending." },
];

const GAME_OBJECT_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "transform", kind: CompletionEntryKind::Property, signature: "transform: Transform", description: "The Transform of this object." },
    CoreMember { name: "activeSelf", kind: CompletionEntryKind::Property, signature: "activeSelf: Bool", description: "Whether this object is active." },
    CoreMember { name: "activeInHierarchy", kind: CompletionEntryKind::Property, signature: "activeInHierarchy: Bool", description: "Whether this object is active in the hierarchy." },
    CoreMember { name: "tag", kind: CompletionEntryKind::Property, signature: "tag: String", description: "The tag of this object." },
    CoreMember { name: "name", kind: CompletionEntryKind::Property, signature: "name: String", description: "The name of this object." },
    CoreMember { name: "layer", kind: CompletionEntryKind::Property, signature: "layer: Int", description: "The layer index for this object." },
    CoreMember { name: "scene", kind: CompletionEntryKind::Property, signature: "scene: Scene", description: "The scene this object belongs to." },
    CoreMember { name: "setActive", kind: CompletionEntryKind::Method, signature: "setActive(value: Bool): Unit", description: "Activates or deactivates the object." },
    CoreMember { name: "getComponent", kind: CompletionEntryKind::Method, signature: "getComponent<T>(): T", description: "Gets a component." },
    CoreMember { name: "getComponentInChildren", kind: CompletionEntryKind::Method, signature: "getComponentInChildren<T>(): T", description: "Gets a component from children." },
    CoreMember { name: "getComponentInParent", kind: CompletionEntryKind::Method, signature: "getComponentInParent<T>(): T", description: "Gets a component from parents." },
    CoreMember { name: "getComponentsInChildren", kind: CompletionEntryKind::Method, signature: "getComponentsInChildren<T>(): T[]", description: "Gets matching components from children." },
    CoreMember { name: "addComponent", kind: CompletionEntryKind::Method, signature: "addComponent<T>(): T", description: "Adds a component." },
    CoreMember { name: "compareTag", kind: CompletionEntryKind::Method, signature: "compareTag(tag: String): Bool", description: "Compares the tag." },
    CoreMember { name: "find", kind: CompletionEntryKind::Method, signature: "find(name: String): GameObject", description: "Finds a game object by name." },
    CoreMember { name: "destroy", kind: CompletionEntryKind::Method, signature: "destroy(): Unit", description: "Destroys the object." },
];

const TRANSFORM_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "position", kind: CompletionEntryKind::Property, signature: "position: Vector3", description: "World space position." },
    CoreMember { name: "localPosition", kind: CompletionEntryKind::Property, signature: "localPosition: Vector3", description: "Local space position." },
    CoreMember { name: "rotation", kind: CompletionEntryKind::Property, signature: "rotation: Quaternion", description: "World space rotation." },
    CoreMember { name: "localRotation", kind: CompletionEntryKind::Property, signature: "localRotation: Quaternion", description: "Local space rotation." },
    CoreMember { name: "localScale", kind: CompletionEntryKind::Property, signature: "localScale: Vector3", description: "Local scale." },
    CoreMember { name: "forward", kind: CompletionEntryKind::Property, signature: "forward: Vector3", description: "Forward direction." },
    CoreMember { name: "right", kind: CompletionEntryKind::Property, signature: "right: Vector3", description: "Right direction." },
    CoreMember { name: "up", kind: CompletionEntryKind::Property, signature: "up: Vector3", description: "Up direction." },
    CoreMember { name: "parent", kind: CompletionEntryKind::Property, signature: "parent: Transform", description: "Parent transform." },
    CoreMember { name: "root", kind: CompletionEntryKind::Property, signature: "root: Transform", description: "Topmost transform in the hierarchy." },
    CoreMember { name: "childCount", kind: CompletionEntryKind::Property, signature: "childCount: Int", description: "Number of children." },
    CoreMember { name: "hasChanged", kind: CompletionEntryKind::Property, signature: "hasChanged: Bool", description: "Whether the transform changed since the last reset." },
    CoreMember { name: "lossyScale", kind: CompletionEntryKind::Property, signature: "lossyScale: Vector3", description: "World-space scale." },
    CoreMember { name: "translate", kind: CompletionEntryKind::Method, signature: "translate(translation: Vector3): Unit", description: "Moves the transform." },
    CoreMember { name: "rotate", kind: CompletionEntryKind::Method, signature: "rotate(eulers: Vector3): Unit", description: "Rotates the transform." },
    CoreMember { name: "lookAt", kind: CompletionEntryKind::Method, signature: "lookAt(target: Transform): Unit", description: "Looks at a target." },
    CoreMember { name: "setParent", kind: CompletionEntryKind::Method, signature: "setParent(parent: Transform): Unit", description: "Changes the parent transform." },
    CoreMember { name: "getChild", kind: CompletionEntryKind::Method, signature: "getChild(index: Int): Transform", description: "Gets the child at an index." },
    CoreMember { name: "transformPoint", kind: CompletionEntryKind::Method, signature: "transformPoint(position: Vector3): Vector3", description: "Transforms a local point into world space." },
    CoreMember { name: "inverseTransformPoint", kind: CompletionEntryKind::Method, signature: "inverseTransformPoint(position: Vector3): Vector3", description: "Transforms a world point into local space." },
];

const RIGIDBODY_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "velocity", kind: CompletionEntryKind::Property, signature: "velocity: Vector3", description: "Linear velocity." },
    CoreMember { name: "angularVelocity", kind: CompletionEntryKind::Property, signature: "angularVelocity: Vector3", description: "Angular velocity." },
    CoreMember { name: "mass", kind: CompletionEntryKind::Property, signature: "mass: Float", description: "Mass." },
    CoreMember { name: "drag", kind: CompletionEntryKind::Property, signature: "drag: Float", description: "Drag coefficient." },
    CoreMember { name: "useGravity", kind: CompletionEntryKind::Property, signature: "useGravity: Bool", description: "Whether gravity is enabled." },
    CoreMember { name: "isKinematic", kind: CompletionEntryKind::Property, signature: "isKinematic: Bool", description: "Whether the body is kinematic." },
    CoreMember { name: "position", kind: CompletionEntryKind::Property, signature: "position: Vector3", description: "World-space position." },
    CoreMember { name: "rotation", kind: CompletionEntryKind::Property, signature: "rotation: Quaternion", description: "World-space rotation." },
    CoreMember { name: "constraints", kind: CompletionEntryKind::Property, signature: "constraints: RigidbodyConstraints", description: "Applied movement constraints." },
    CoreMember { name: "addForce", kind: CompletionEntryKind::Method, signature: "addForce(force: Vector3): Unit", description: "Adds force." },
    CoreMember { name: "addTorque", kind: CompletionEntryKind::Method, signature: "addTorque(torque: Vector3): Unit", description: "Adds torque." },
    CoreMember { name: "movePosition", kind: CompletionEntryKind::Method, signature: "movePosition(position: Vector3): Unit", description: "Moves the rigidbody." },
    CoreMember { name: "sleep", kind: CompletionEntryKind::Method, signature: "sleep(): Unit", description: "Forces the rigidbody to sleep." },
    CoreMember { name: "wakeUp", kind: CompletionEntryKind::Method, signature: "wakeUp(): Unit", description: "Wakes the rigidbody if it is sleeping." },
];

const ANIMATOR_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "speed", kind: CompletionEntryKind::Property, signature: "speed: Float", description: "Playback speed." },
    CoreMember { name: "applyRootMotion", kind: CompletionEntryKind::Property, signature: "applyRootMotion: Bool", description: "Whether root motion is applied." },
    CoreMember { name: "play", kind: CompletionEntryKind::Method, signature: "play(stateName: String): Unit", description: "Plays an animation state." },
    CoreMember { name: "crossFade", kind: CompletionEntryKind::Method, signature: "crossFade(stateName: String, normalizedTransitionDuration: Float): Unit", description: "Cross-fades to another state." },
    CoreMember { name: "setBool", kind: CompletionEntryKind::Method, signature: "setBool(name: String, value: Bool): Unit", description: "Sets a bool parameter." },
    CoreMember { name: "setFloat", kind: CompletionEntryKind::Method, signature: "setFloat(name: String, value: Float): Unit", description: "Sets a float parameter." },
    CoreMember { name: "setInteger", kind: CompletionEntryKind::Method, signature: "setInteger(name: String, value: Int): Unit", description: "Sets an integer parameter." },
    CoreMember { name: "setTrigger", kind: CompletionEntryKind::Method, signature: "setTrigger(name: String): Unit", description: "Sets a trigger parameter." },
    CoreMember { name: "resetTrigger", kind: CompletionEntryKind::Method, signature: "resetTrigger(name: String): Unit", description: "Resets a trigger parameter." },
    CoreMember { name: "setLayerWeight", kind: CompletionEntryKind::Method, signature: "setLayerWeight(layerIndex: Int, weight: Float): Unit", description: "Sets the weight for a layer." },
];

const COLLIDER_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "enabled", kind: CompletionEntryKind::Property, signature: "enabled: Bool", description: "Enabled state." },
    CoreMember { name: "isTrigger", kind: CompletionEntryKind::Property, signature: "isTrigger: Bool", description: "Whether the collider is a trigger." },
    CoreMember { name: "gameObject", kind: CompletionEntryKind::Property, signature: "gameObject: GameObject", description: "Attached GameObject." },
    CoreMember { name: "transform", kind: CompletionEntryKind::Property, signature: "transform: Transform", description: "Attached Transform." },
    CoreMember { name: "bounds", kind: CompletionEntryKind::Property, signature: "bounds: Bounds", description: "World-space bounding volume." },
    CoreMember { name: "attachedRigidbody", kind: CompletionEntryKind::Property, signature: "attachedRigidbody: Rigidbody", description: "Attached rigidbody, if any." },
    CoreMember { name: "material", kind: CompletionEntryKind::Property, signature: "material: PhysicMaterial", description: "Assigned physics material." },
    CoreMember { name: "compareTag", kind: CompletionEntryKind::Method, signature: "compareTag(tag: String): Bool", description: "Compares the tag." },
];

const COLLISION_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "gameObject", kind: CompletionEntryKind::Property, signature: "gameObject: GameObject", description: "The other GameObject." },
    CoreMember { name: "transform", kind: CompletionEntryKind::Property, signature: "transform: Transform", description: "The other Transform." },
    CoreMember { name: "relativeVelocity", kind: CompletionEntryKind::Property, signature: "relativeVelocity: Vector3", description: "Relative velocity." },
    CoreMember { name: "collider", kind: CompletionEntryKind::Property, signature: "collider: Collider", description: "The other collider." },
];

const INPUT_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "axis", kind: CompletionEntryKind::Method, signature: "axis(axisName: String): Float", description: "Gets an axis value." },
    CoreMember { name: "getAxis", kind: CompletionEntryKind::Method, signature: "getAxis(axisName: String): Float", description: "Gets a smoothed axis value." },
    CoreMember { name: "getAxisRaw", kind: CompletionEntryKind::Method, signature: "getAxisRaw(axisName: String): Float", description: "Gets a raw axis value." },
    CoreMember { name: "getButton", kind: CompletionEntryKind::Method, signature: "getButton(buttonName: String): Bool", description: "Returns whether a named button is held." },
    CoreMember { name: "getButtonDown", kind: CompletionEntryKind::Method, signature: "getButtonDown(buttonName: String): Bool", description: "Returns whether a named button was pressed this frame." },
    CoreMember { name: "getKey", kind: CompletionEntryKind::Method, signature: "getKey(key: KeyCode): Bool", description: "Returns whether a key is held." },
    CoreMember { name: "getKeyDown", kind: CompletionEntryKind::Method, signature: "getKeyDown(key: KeyCode): Bool", description: "Returns whether a key was pressed this frame." },
    CoreMember { name: "getMouseButton", kind: CompletionEntryKind::Method, signature: "getMouseButton(button: Int): Bool", description: "Returns whether a mouse button is held." },
    CoreMember { name: "getMouseButtonDown", kind: CompletionEntryKind::Method, signature: "getMouseButtonDown(button: Int): Bool", description: "Returns whether a mouse button was pressed this frame." },
    CoreMember { name: "mousePosition", kind: CompletionEntryKind::Property, signature: "mousePosition: Vector3", description: "Mouse position." },
    CoreMember { name: "anyKeyDown", kind: CompletionEntryKind::Property, signature: "anyKeyDown: Bool", description: "Whether any key or mouse button was pressed this frame." },
];

const TIME_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "deltaTime", kind: CompletionEntryKind::Property, signature: "deltaTime: Float", description: "Time since the previous frame." },
    CoreMember { name: "fixedDeltaTime", kind: CompletionEntryKind::Property, signature: "fixedDeltaTime: Float", description: "Fixed timestep interval." },
    CoreMember { name: "time", kind: CompletionEntryKind::Property, signature: "time: Float", description: "Time since startup." },
    CoreMember { name: "fixedTime", kind: CompletionEntryKind::Property, signature: "fixedTime: Float", description: "Time since startup at the last fixed update." },
    CoreMember { name: "timeScale", kind: CompletionEntryKind::Property, signature: "timeScale: Float", description: "Global time scale." },
    CoreMember { name: "unscaledDeltaTime", kind: CompletionEntryKind::Property, signature: "unscaledDeltaTime: Float", description: "Frame delta time unaffected by timeScale." },
    CoreMember { name: "frameCount", kind: CompletionEntryKind::Property, signature: "frameCount: Int", description: "Rendered frame count." },
];

const DEBUG_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "log", kind: CompletionEntryKind::Method, signature: "log(message: Any): Unit", description: "Logs a message." },
    CoreMember { name: "logWarning", kind: CompletionEntryKind::Method, signature: "logWarning(message: Any): Unit", description: "Logs a warning." },
    CoreMember { name: "logError", kind: CompletionEntryKind::Method, signature: "logError(message: Any): Unit", description: "Logs an error." },
    CoreMember { name: "drawLine", kind: CompletionEntryKind::Method, signature: "drawLine(start: Vector3, end: Vector3, color: Color): Unit", description: "Draws a debug line." },
    CoreMember { name: "assert", kind: CompletionEntryKind::Method, signature: "assert(condition: Bool, message: String): Unit", description: "Logs an assertion if the condition is false." },
];

const PHYSICS_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "raycast", kind: CompletionEntryKind::Method, signature: "raycast(origin: Vector3, direction: Vector3, maxDistance: Float): Bool", description: "Casts a ray." },
    CoreMember { name: "gravity", kind: CompletionEntryKind::Property, signature: "gravity: Vector3", description: "Global gravity." },
    CoreMember { name: "sphereCast", kind: CompletionEntryKind::Method, signature: "sphereCast(origin: Vector3, radius: Float, direction: Vector3, maxDistance: Float): Bool", description: "Casts a sphere through the world." },
    CoreMember { name: "overlapSphere", kind: CompletionEntryKind::Method, signature: "overlapSphere(position: Vector3, radius: Float): Collider[]", description: "Returns colliders touching a sphere." },
];

const MATHF_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "abs", kind: CompletionEntryKind::Method, signature: "abs(value: Float): Float", description: "Absolute value." },
    CoreMember { name: "clamp", kind: CompletionEntryKind::Method, signature: "clamp(value: Float, min: Float, max: Float): Float", description: "Clamps a value." },
    CoreMember { name: "lerp", kind: CompletionEntryKind::Method, signature: "lerp(a: Float, b: Float, t: Float): Float", description: "Linear interpolation." },
    CoreMember { name: "min", kind: CompletionEntryKind::Method, signature: "min(a: Float, b: Float): Float", description: "Smaller of two values." },
    CoreMember { name: "max", kind: CompletionEntryKind::Method, signature: "max(a: Float, b: Float): Float", description: "Larger of two values." },
    CoreMember { name: "sin", kind: CompletionEntryKind::Method, signature: "sin(angle: Float): Float", description: "Sine of an angle in radians." },
    CoreMember { name: "cos", kind: CompletionEntryKind::Method, signature: "cos(angle: Float): Float", description: "Cosine of an angle in radians." },
    CoreMember { name: "smoothDamp", kind: CompletionEntryKind::Method, signature: "smoothDamp(current: Float, target: Float, velocity: Float, smoothTime: Float): Float", description: "Gradually changes a value toward a target." },
    CoreMember { name: "pi", kind: CompletionEntryKind::Field, signature: "pi: Float", description: "Pi constant." },
];

const VECTOR3_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "x", kind: CompletionEntryKind::Field, signature: "x: Float", description: "X component." },
    CoreMember { name: "y", kind: CompletionEntryKind::Field, signature: "y: Float", description: "Y component." },
    CoreMember { name: "z", kind: CompletionEntryKind::Field, signature: "z: Float", description: "Z component." },
    CoreMember { name: "magnitude", kind: CompletionEntryKind::Property, signature: "magnitude: Float", description: "Length." },
    CoreMember { name: "sqrMagnitude", kind: CompletionEntryKind::Property, signature: "sqrMagnitude: Float", description: "Squared length." },
    CoreMember { name: "normalized", kind: CompletionEntryKind::Property, signature: "normalized: Vector3", description: "Normalized vector." },
    CoreMember { name: "zero", kind: CompletionEntryKind::Property, signature: "zero: Vector3", description: "Zero vector." },
    CoreMember { name: "one", kind: CompletionEntryKind::Property, signature: "one: Vector3", description: "One vector." },
    CoreMember { name: "forward", kind: CompletionEntryKind::Property, signature: "forward: Vector3", description: "Forward vector." },
    CoreMember { name: "up", kind: CompletionEntryKind::Property, signature: "up: Vector3", description: "Up vector." },
    CoreMember { name: "right", kind: CompletionEntryKind::Property, signature: "right: Vector3", description: "Right vector." },
    CoreMember { name: "distance", kind: CompletionEntryKind::Method, signature: "distance(a: Vector3, b: Vector3): Float", description: "Distance between vectors." },
    CoreMember { name: "lerp", kind: CompletionEntryKind::Method, signature: "lerp(a: Vector3, b: Vector3, t: Float): Vector3", description: "Linear interpolation." },
    CoreMember { name: "dot", kind: CompletionEntryKind::Method, signature: "dot(a: Vector3, b: Vector3): Float", description: "Dot product." },
    CoreMember { name: "cross", kind: CompletionEntryKind::Method, signature: "cross(lhs: Vector3, rhs: Vector3): Vector3", description: "Cross product." },
];

const VECTOR2_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "x", kind: CompletionEntryKind::Field, signature: "x: Float", description: "X component." },
    CoreMember { name: "y", kind: CompletionEntryKind::Field, signature: "y: Float", description: "Y component." },
    CoreMember { name: "magnitude", kind: CompletionEntryKind::Property, signature: "magnitude: Float", description: "Length." },
    CoreMember { name: "normalized", kind: CompletionEntryKind::Property, signature: "normalized: Vector2", description: "Normalized vector." },
    CoreMember { name: "zero", kind: CompletionEntryKind::Property, signature: "zero: Vector2", description: "Zero vector." },
    CoreMember { name: "one", kind: CompletionEntryKind::Property, signature: "one: Vector2", description: "One vector." },
    CoreMember { name: "distance", kind: CompletionEntryKind::Method, signature: "distance(a: Vector2, b: Vector2): Float", description: "Distance between vectors." },
    CoreMember { name: "lerp", kind: CompletionEntryKind::Method, signature: "lerp(a: Vector2, b: Vector2, t: Float): Vector2", description: "Linear interpolation." },
];

const COLOR_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "r", kind: CompletionEntryKind::Field, signature: "r: Float", description: "Red." },
    CoreMember { name: "g", kind: CompletionEntryKind::Field, signature: "g: Float", description: "Green." },
    CoreMember { name: "b", kind: CompletionEntryKind::Field, signature: "b: Float", description: "Blue." },
    CoreMember { name: "a", kind: CompletionEntryKind::Field, signature: "a: Float", description: "Alpha." },
    CoreMember { name: "white", kind: CompletionEntryKind::Property, signature: "white: Color", description: "White color constant." },
    CoreMember { name: "black", kind: CompletionEntryKind::Property, signature: "black: Color", description: "Black color constant." },
    CoreMember { name: "red", kind: CompletionEntryKind::Property, signature: "red: Color", description: "Red color constant." },
    CoreMember { name: "green", kind: CompletionEntryKind::Property, signature: "green: Color", description: "Green color constant." },
    CoreMember { name: "blue", kind: CompletionEntryKind::Property, signature: "blue: Color", description: "Blue color constant." },
];

const QUATERNION_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "x", kind: CompletionEntryKind::Field, signature: "x: Float", description: "X component." },
    CoreMember { name: "y", kind: CompletionEntryKind::Field, signature: "y: Float", description: "Y component." },
    CoreMember { name: "z", kind: CompletionEntryKind::Field, signature: "z: Float", description: "Z component." },
    CoreMember { name: "w", kind: CompletionEntryKind::Field, signature: "w: Float", description: "W component." },
    CoreMember { name: "identity", kind: CompletionEntryKind::Property, signature: "identity: Quaternion", description: "Identity rotation." },
    CoreMember { name: "euler", kind: CompletionEntryKind::Method, signature: "euler(eulerAngles: Vector3): Quaternion", description: "Creates a rotation from Euler angles." },
    CoreMember { name: "slerp", kind: CompletionEntryKind::Method, signature: "slerp(a: Quaternion, b: Quaternion, t: Float): Quaternion", description: "Spherically interpolates between rotations." },
];

const STRING_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "length", kind: CompletionEntryKind::Property, signature: "length: Int", description: "Character count." },
    CoreMember { name: "contains", kind: CompletionEntryKind::Method, signature: "contains(value: String): Bool", description: "Contains substring." },
    CoreMember { name: "startsWith", kind: CompletionEntryKind::Method, signature: "startsWith(value: String): Bool", description: "Starts with substring." },
    CoreMember { name: "endsWith", kind: CompletionEntryKind::Method, signature: "endsWith(value: String): Bool", description: "Ends with substring." },
    CoreMember { name: "substring", kind: CompletionEntryKind::Method, signature: "substring(startIndex: Int, length: Int): String", description: "Extracts a substring." },
    CoreMember { name: "replace", kind: CompletionEntryKind::Method, signature: "replace(oldValue: String, newValue: String): String", description: "Replaces occurrences." },
    CoreMember { name: "trim", kind: CompletionEntryKind::Method, signature: "trim(): String", description: "Trims whitespace." },
    CoreMember { name: "toUpper", kind: CompletionEntryKind::Method, signature: "toUpper(): String", description: "Converts to uppercase." },
    CoreMember { name: "toLower", kind: CompletionEntryKind::Method, signature: "toLower(): String", description: "Converts to lowercase." },
    CoreMember { name: "toInt", kind: CompletionEntryKind::Method, signature: "toInt(): Int", description: "Parses as Int." },
    CoreMember { name: "toFloat", kind: CompletionEntryKind::Method, signature: "toFloat(): Float", description: "Parses as Float." },
];

const INT_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "toString", kind: CompletionEntryKind::Method, signature: "toString(): String", description: "Converts to string." },
    CoreMember { name: "toFloat", kind: CompletionEntryKind::Method, signature: "toFloat(): Float", description: "Converts to Float." },
    CoreMember { name: "toDouble", kind: CompletionEntryKind::Method, signature: "toDouble(): Double", description: "Converts to Double." },
    CoreMember { name: "abs", kind: CompletionEntryKind::Method, signature: "abs(): Int", description: "Absolute value." },
    CoreMember { name: "maxValue", kind: CompletionEntryKind::Field, signature: "maxValue: Int", description: "Maximum value." },
];

const FLOAT_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "toString", kind: CompletionEntryKind::Method, signature: "toString(): String", description: "Converts to string." },
    CoreMember { name: "toInt", kind: CompletionEntryKind::Method, signature: "toInt(): Int", description: "Converts to Int." },
    CoreMember { name: "toDouble", kind: CompletionEntryKind::Method, signature: "toDouble(): Double", description: "Converts to Double." },
    CoreMember { name: "isNaN", kind: CompletionEntryKind::Method, signature: "isNaN(): Bool", description: "Checks NaN." },
    CoreMember { name: "maxValue", kind: CompletionEntryKind::Field, signature: "maxValue: Float", description: "Maximum value." },
];

const DOUBLE_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "toString", kind: CompletionEntryKind::Method, signature: "toString(): String", description: "Converts to string." },
    CoreMember { name: "toInt", kind: CompletionEntryKind::Method, signature: "toInt(): Int", description: "Converts to Int." },
    CoreMember { name: "toFloat", kind: CompletionEntryKind::Method, signature: "toFloat(): Float", description: "Converts to Float." },
    CoreMember { name: "maxValue", kind: CompletionEntryKind::Field, signature: "maxValue: Double", description: "Maximum value." },
];

const BOOL_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "toString", kind: CompletionEntryKind::Method, signature: "toString(): String", description: "Converts to string." },
    CoreMember { name: "trueString", kind: CompletionEntryKind::Field, signature: "trueString: String", description: "True literal string." },
    CoreMember { name: "falseString", kind: CompletionEntryKind::Field, signature: "falseString: String", description: "False literal string." },
];

const LONG_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "toString", kind: CompletionEntryKind::Method, signature: "toString(): String", description: "Converts to string." },
    CoreMember { name: "toInt", kind: CompletionEntryKind::Method, signature: "toInt(): Int", description: "Converts to Int." },
    CoreMember { name: "maxValue", kind: CompletionEntryKind::Field, signature: "maxValue: Long", description: "Maximum value." },
];

const BYTE_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "toString", kind: CompletionEntryKind::Method, signature: "toString(): String", description: "Converts to string." },
    CoreMember { name: "toInt", kind: CompletionEntryKind::Method, signature: "toInt(): Int", description: "Converts to Int." },
    CoreMember { name: "maxValue", kind: CompletionEntryKind::Field, signature: "maxValue: Byte", description: "Maximum value." },
];

const APPLICATION_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "quit", kind: CompletionEntryKind::Method, signature: "quit(): Unit", description: "Quits the application." },
    CoreMember { name: "isPlaying", kind: CompletionEntryKind::Property, signature: "isPlaying: Bool", description: "Whether the player is running." },
    CoreMember { name: "targetFrameRate", kind: CompletionEntryKind::Property, signature: "targetFrameRate: Int", description: "Target frame rate." },
    CoreMember { name: "persistentDataPath", kind: CompletionEntryKind::Property, signature: "persistentDataPath: String", description: "Persistent data directory." },
    CoreMember { name: "version", kind: CompletionEntryKind::Property, signature: "version: String", description: "Application version string." },
];

const SCENE_MANAGER_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "loadScene", kind: CompletionEntryKind::Method, signature: "loadScene(sceneName: String): Unit", description: "Loads a scene." },
    CoreMember { name: "loadSceneAsync", kind: CompletionEntryKind::Method, signature: "loadSceneAsync(sceneName: String): AsyncOperation", description: "Loads a scene asynchronously." },
    CoreMember { name: "getActiveScene", kind: CompletionEntryKind::Method, signature: "getActiveScene(): Scene", description: "Gets the active scene." },
    CoreMember { name: "activeScene", kind: CompletionEntryKind::Property, signature: "activeScene: Scene", description: "Currently active scene." },
];

const CAMERA_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "main", kind: CompletionEntryKind::Property, signature: "main: Camera", description: "The first enabled camera tagged MainCamera." },
    CoreMember { name: "fieldOfView", kind: CompletionEntryKind::Property, signature: "fieldOfView: Float", description: "Vertical field of view." },
    CoreMember { name: "backgroundColor", kind: CompletionEntryKind::Property, signature: "backgroundColor: Color", description: "Clear color used by the camera." },
    CoreMember { name: "nearClipPlane", kind: CompletionEntryKind::Property, signature: "nearClipPlane: Float", description: "Near clipping plane distance." },
    CoreMember { name: "farClipPlane", kind: CompletionEntryKind::Property, signature: "farClipPlane: Float", description: "Far clipping plane distance." },
    CoreMember { name: "orthographic", kind: CompletionEntryKind::Property, signature: "orthographic: Bool", description: "Whether the camera uses orthographic projection." },
    CoreMember { name: "screenToWorldPoint", kind: CompletionEntryKind::Method, signature: "screenToWorldPoint(position: Vector3): Vector3", description: "Transforms screen coordinates to world space." },
    CoreMember { name: "worldToScreenPoint", kind: CompletionEntryKind::Method, signature: "worldToScreenPoint(position: Vector3): Vector3", description: "Transforms world coordinates to screen space." },
    CoreMember { name: "viewportToWorldPoint", kind: CompletionEntryKind::Method, signature: "viewportToWorldPoint(position: Vector3): Vector3", description: "Transforms viewport coordinates to world space." },
    CoreMember { name: "render", kind: CompletionEntryKind::Method, signature: "render(): Unit", description: "Renders the camera immediately." },
];

const AUDIO_SOURCE_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "clip", kind: CompletionEntryKind::Property, signature: "clip: AudioClip", description: "Assigned audio clip." },
    CoreMember { name: "volume", kind: CompletionEntryKind::Property, signature: "volume: Float", description: "Playback volume." },
    CoreMember { name: "pitch", kind: CompletionEntryKind::Property, signature: "pitch: Float", description: "Playback pitch." },
    CoreMember { name: "loop", kind: CompletionEntryKind::Property, signature: "loop: Bool", description: "Whether the clip loops." },
    CoreMember { name: "mute", kind: CompletionEntryKind::Property, signature: "mute: Bool", description: "Whether the source is muted." },
    CoreMember { name: "play", kind: CompletionEntryKind::Method, signature: "play(): Unit", description: "Starts playback." },
    CoreMember { name: "playOneShot", kind: CompletionEntryKind::Method, signature: "playOneShot(clip: AudioClip): Unit", description: "Plays a clip once without interrupting the current clip." },
    CoreMember { name: "pause", kind: CompletionEntryKind::Method, signature: "pause(): Unit", description: "Pauses playback." },
    CoreMember { name: "stop", kind: CompletionEntryKind::Method, signature: "stop(): Unit", description: "Stops playback." },
];

const SPRITE_RENDERER_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "sprite", kind: CompletionEntryKind::Property, signature: "sprite: Sprite", description: "Rendered sprite asset." },
    CoreMember { name: "color", kind: CompletionEntryKind::Property, signature: "color: Color", description: "Tint color." },
    CoreMember { name: "flipX", kind: CompletionEntryKind::Property, signature: "flipX: Bool", description: "Whether the sprite is flipped on X." },
    CoreMember { name: "flipY", kind: CompletionEntryKind::Property, signature: "flipY: Bool", description: "Whether the sprite is flipped on Y." },
    CoreMember { name: "size", kind: CompletionEntryKind::Property, signature: "size: Vector2", description: "Draw size when using tiled or sliced mode." },
    CoreMember { name: "sortingOrder", kind: CompletionEntryKind::Property, signature: "sortingOrder: Int", description: "Renderer sorting order." },
];

const SCREEN_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "width", kind: CompletionEntryKind::Property, signature: "width: Int", description: "Current screen width in pixels." },
    CoreMember { name: "height", kind: CompletionEntryKind::Property, signature: "height: Int", description: "Current screen height in pixels." },
    CoreMember { name: "dpi", kind: CompletionEntryKind::Property, signature: "dpi: Float", description: "Approximate screen DPI." },
    CoreMember { name: "orientation", kind: CompletionEntryKind::Property, signature: "orientation: ScreenOrientation", description: "Current screen orientation." },
    CoreMember { name: "fullScreen", kind: CompletionEntryKind::Property, signature: "fullScreen: Bool", description: "Whether the app runs fullscreen." },
    CoreMember { name: "setResolution", kind: CompletionEntryKind::Method, signature: "setResolution(width: Int, height: Int, fullScreen: Bool): Unit", description: "Changes the screen resolution." },
];

const CURSOR_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "visible", kind: CompletionEntryKind::Property, signature: "visible: Bool", description: "Whether the hardware cursor is visible." },
    CoreMember { name: "lockState", kind: CompletionEntryKind::Property, signature: "lockState: CursorLockMode", description: "Current cursor lock state." },
    CoreMember { name: "setCursor", kind: CompletionEntryKind::Method, signature: "setCursor(texture: Texture2D, hotspot: Vector2): Unit", description: "Sets a custom cursor texture." },
];

const RESOURCES_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "load", kind: CompletionEntryKind::Method, signature: "load<T>(path: String): T", description: "Loads an asset from a Resources folder." },
    CoreMember { name: "loadAll", kind: CompletionEntryKind::Method, signature: "loadAll<T>(path: String): T[]", description: "Loads all assets from a Resources path." },
    CoreMember { name: "unloadAsset", kind: CompletionEntryKind::Method, signature: "unloadAsset(asset: Object): Unit", description: "Unloads an asset from memory." },
];

const PLAYER_PREFS_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "setInt", kind: CompletionEntryKind::Method, signature: "setInt(key: String, value: Int): Unit", description: "Stores an integer preference." },
    CoreMember { name: "getInt", kind: CompletionEntryKind::Method, signature: "getInt(key: String): Int", description: "Reads an integer preference." },
    CoreMember { name: "setFloat", kind: CompletionEntryKind::Method, signature: "setFloat(key: String, value: Float): Unit", description: "Stores a float preference." },
    CoreMember { name: "getFloat", kind: CompletionEntryKind::Method, signature: "getFloat(key: String): Float", description: "Reads a float preference." },
    CoreMember { name: "setString", kind: CompletionEntryKind::Method, signature: "setString(key: String, value: String): Unit", description: "Stores a string preference." },
    CoreMember { name: "getString", kind: CompletionEntryKind::Method, signature: "getString(key: String): String", description: "Reads a string preference." },
    CoreMember { name: "hasKey", kind: CompletionEntryKind::Method, signature: "hasKey(key: String): Bool", description: "Checks whether a preference exists." },
    CoreMember { name: "deleteKey", kind: CompletionEntryKind::Method, signature: "deleteKey(key: String): Unit", description: "Deletes a stored preference." },
    CoreMember { name: "save", kind: CompletionEntryKind::Method, signature: "save(): Unit", description: "Writes modified preferences to disk." },
];

const UNITY_EVENT_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "addListener", kind: CompletionEntryKind::Method, signature: "addListener(callback: Any): Unit", description: "Registers a listener callback." },
    CoreMember { name: "removeListener", kind: CompletionEntryKind::Method, signature: "removeListener(callback: Any): Unit", description: "Removes a listener callback." },
    CoreMember { name: "removeAllListeners", kind: CompletionEntryKind::Method, signature: "removeAllListeners(): Unit", description: "Clears every registered listener." },
    CoreMember { name: "invoke", kind: CompletionEntryKind::Method, signature: "invoke(): Unit", description: "Invokes all listeners." },
];

const BUTTON_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "onClick", kind: CompletionEntryKind::Event, signature: "onClick: UnityEvent", description: "Button click event." },
    CoreMember { name: "interactable", kind: CompletionEntryKind::Property, signature: "interactable: Bool", description: "Whether the button is interactable." },
    CoreMember { name: "enabled", kind: CompletionEntryKind::Property, signature: "enabled: Bool", description: "Whether the button component is enabled." },
];

const SLIDER_MEMBERS: &[CoreMember] = &[
    CoreMember { name: "value", kind: CompletionEntryKind::Property, signature: "value: Float", description: "Current value." },
    CoreMember { name: "minValue", kind: CompletionEntryKind::Property, signature: "minValue: Float", description: "Minimum value." },
    CoreMember { name: "maxValue", kind: CompletionEntryKind::Property, signature: "maxValue: Float", description: "Maximum value." },
    CoreMember { name: "wholeNumbers", kind: CompletionEntryKind::Property, signature: "wholeNumbers: Bool", description: "Whether the slider rounds to whole numbers." },
    CoreMember { name: "onValueChanged", kind: CompletionEntryKind::Event, signature: "onValueChanged: UnityEvent", description: "Slider value changed event." },
];

const CORE_TYPES: &[CoreType] = &[
    CoreType { name: "MonoBehaviour", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Class", description: "Base class for Unity scripts.", members: MONO_BEHAVIOUR_MEMBERS },
    CoreType { name: "GameObject", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Class", description: "Base class for all scene objects.", members: GAME_OBJECT_MEMBERS },
    CoreType { name: "Transform", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Class", description: "Position, rotation, and scale of an object.", members: TRANSFORM_MEMBERS },
    CoreType { name: "Rigidbody", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Class", description: "Physics body for 3D objects.", members: RIGIDBODY_MEMBERS },
    CoreType { name: "Animator", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Class", description: "Controls animations.", members: ANIMATOR_MEMBERS },
    CoreType { name: "Collider", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Class", description: "Base collider type.", members: COLLIDER_MEMBERS },
    CoreType { name: "Collision", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Class", description: "Collision event payload.", members: COLLISION_MEMBERS },
    CoreType { name: "Camera", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Class", description: "Scene camera and projection settings.", members: CAMERA_MEMBERS },
    CoreType { name: "AudioSource", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Class", description: "Audio playback component.", members: AUDIO_SOURCE_MEMBERS },
    CoreType { name: "SpriteRenderer", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Class", description: "2D sprite rendering component.", members: SPRITE_RENDERER_MEMBERS },
    CoreType { name: "Input", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Input system.", members: INPUT_MEMBERS },
    CoreType { name: "Time", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Time management.", members: TIME_MEMBERS },
    CoreType { name: "Debug", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Debug utilities.", members: DEBUG_MEMBERS },
    CoreType { name: "Physics", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Physics queries.", members: PHYSICS_MEMBERS },
    CoreType { name: "Screen", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Screen and display state.", members: SCREEN_MEMBERS },
    CoreType { name: "Cursor", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Hardware cursor control.", members: CURSOR_MEMBERS },
    CoreType { name: "Resources", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Resources folder asset loading.", members: RESOURCES_MEMBERS },
    CoreType { name: "PlayerPrefs", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Simple persistent key-value storage.", members: PLAYER_PREFS_MEMBERS },
    CoreType { name: "Mathf", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Math utilities.", members: MATHF_MEMBERS },
    CoreType { name: "Vector3", namespace: "UnityEngine", kind: CompletionEntryKind::Struct, kind_label: "Struct", description: "3D vector.", members: VECTOR3_MEMBERS },
    CoreType { name: "Vector2", namespace: "UnityEngine", kind: CompletionEntryKind::Struct, kind_label: "Struct", description: "2D vector.", members: VECTOR2_MEMBERS },
    CoreType { name: "Color", namespace: "UnityEngine", kind: CompletionEntryKind::Struct, kind_label: "Struct", description: "RGBA color.", members: COLOR_MEMBERS },
    CoreType { name: "Quaternion", namespace: "UnityEngine", kind: CompletionEntryKind::Struct, kind_label: "Struct", description: "Rotation quaternion.", members: QUATERNION_MEMBERS },
    CoreType { name: "Application", namespace: "UnityEngine", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Application info and control.", members: APPLICATION_MEMBERS },
    CoreType { name: "SceneManager", namespace: "UnityEngine.SceneManagement", kind: CompletionEntryKind::Class, kind_label: "Static class", description: "Scene loading and management.", members: SCENE_MANAGER_MEMBERS },
    CoreType { name: "UnityEvent", namespace: "UnityEngine.Events", kind: CompletionEntryKind::Class, kind_label: "Class", description: "Event type used by Unity UI and serialized callbacks.", members: UNITY_EVENT_MEMBERS },
    CoreType { name: "Button", namespace: "UnityEngine.UI", kind: CompletionEntryKind::Class, kind_label: "Class", description: "UI button.", members: BUTTON_MEMBERS },
    CoreType { name: "Slider", namespace: "UnityEngine.UI", kind: CompletionEntryKind::Class, kind_label: "Class", description: "UI slider.", members: SLIDER_MEMBERS },
    CoreType { name: "Int", namespace: "prsm", kind: CompletionEntryKind::TypeParameter, kind_label: "Primitive", description: "PrSM Int.", members: INT_MEMBERS },
    CoreType { name: "Float", namespace: "prsm", kind: CompletionEntryKind::TypeParameter, kind_label: "Primitive", description: "PrSM Float.", members: FLOAT_MEMBERS },
    CoreType { name: "Double", namespace: "prsm", kind: CompletionEntryKind::TypeParameter, kind_label: "Primitive", description: "PrSM Double.", members: DOUBLE_MEMBERS },
    CoreType { name: "Bool", namespace: "prsm", kind: CompletionEntryKind::TypeParameter, kind_label: "Primitive", description: "PrSM Bool.", members: BOOL_MEMBERS },
    CoreType { name: "String", namespace: "prsm", kind: CompletionEntryKind::TypeParameter, kind_label: "Primitive", description: "PrSM String.", members: STRING_MEMBERS },
    CoreType { name: "Long", namespace: "prsm", kind: CompletionEntryKind::TypeParameter, kind_label: "Primitive", description: "PrSM Long.", members: LONG_MEMBERS },
    CoreType { name: "Byte", namespace: "prsm", kind: CompletionEntryKind::TypeParameter, kind_label: "Primitive", description: "PrSM Byte.", members: BYTE_MEMBERS },
];
