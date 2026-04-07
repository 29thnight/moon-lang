use crate::ast::{
    Arg, Block, Decl, ElseBranch, EnumEntry, Expr, FuncBody, LambdaBody, LifecycleKind, Member,
    Param, Stmt, StringPart, TypeRef, Visibility, WaitForm, WhenBody, WhenBranch, WhenPattern,
};
use crate::lexer::lexer::Lexer;
use crate::lexer::token::{Position, Span};
use crate::parser::parser::Parser;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarationKind {
    Component,
    Asset,
    Class,
    DataClass,
    Enum,
    Attribute,
    Interface,
}

impl DeclarationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Component => "component",
            Self::Asset => "asset",
            Self::Class => "class",
            Self::DataClass => "data class",
            Self::Enum => "enum",
            Self::Attribute => "attribute",
            Self::Interface => "interface",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Field,
    SerializeField,
    RequiredComponent,
    OptionalComponent,
    ChildComponent,
    ParentComponent,
    Function,
    Coroutine,
    Lifecycle,
    EnumEntry,
}

impl MemberKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Field => "field",
            Self::SerializeField => "serialize-field",
            Self::RequiredComponent => "required-component",
            Self::OptionalComponent => "optional-component",
            Self::ChildComponent => "child-component",
            Self::ParentComponent => "parent-component",
            Self::Function => "function",
            Self::Coroutine => "coroutine",
            Self::Lifecycle => "lifecycle",
            Self::EnumEntry => "enum-entry",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexedSymbolKind {
    Declaration(DeclarationKind),
    Member(MemberKind),
}

impl IndexedSymbolKind {
    pub fn is_top_level(&self) -> bool {
        matches!(self, Self::Declaration(_))
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Declaration(kind) => kind.as_str(),
            Self::Member(kind) => kind.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexedReferenceKind {
    Type,
}

impl IndexedReferenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemberSummary {
    pub name: String,
    pub kind: MemberKind,
    pub signature: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct DeclarationSummary {
    pub name: String,
    pub kind: DeclarationKind,
    pub base_type: Option<String>,
    pub interfaces: Vec<String>,
    pub signature: String,
    pub members: Vec<MemberSummary>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FileSummary {
    pub path: PathBuf,
    pub usings: Vec<String>,
    pub declaration: DeclarationSummary,
    pub symbol_count: usize,
}

#[derive(Debug, Clone)]
pub struct IndexedSymbol {
    pub name: String,
    pub qualified_name: String,
    pub container_name: Option<String>,
    pub kind: IndexedSymbolKind,
    pub file_path: PathBuf,
    pub signature: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IndexedReference {
    pub name: String,
    pub container_name: Option<String>,
    pub kind: IndexedReferenceKind,
    pub file_path: PathBuf,
    pub span: Span,
    pub target_qualified_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SymbolIndex {
    pub symbols: Vec<IndexedSymbol>,
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceIndex {
    pub references: Vec<IndexedReference>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolQuery {
    pub name: Option<String>,
    pub qualified_name: Option<String>,
}

impl SymbolIndex {
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    pub fn find_by_name(&self, name: &str) -> Vec<&IndexedSymbol> {
        self.symbols.iter().filter(|symbol| symbol.name == name).collect()
    }

    pub fn find_by_qualified_name(&self, qualified_name: &str) -> Option<&IndexedSymbol> {
        self.symbols
            .iter()
            .find(|symbol| symbol.qualified_name == qualified_name)
    }

    pub fn query(&self, query: &SymbolQuery) -> Vec<&IndexedSymbol> {
        let mut matches = self
            .symbols
            .iter()
            .filter(|symbol| {
                query
                    .name
                    .as_ref()
                    .map(|name| symbol.name == *name)
                    .unwrap_or(true)
                    && query
                        .qualified_name
                        .as_ref()
                        .map(|qualified_name| symbol.qualified_name == *qualified_name)
                        .unwrap_or(true)
            })
            .collect::<Vec<_>>();

        matches.sort_by(|left, right| {
            left.qualified_name
                .cmp(&right.qualified_name)
                .then(left.file_path.cmp(&right.file_path))
                .then(left.span.start.line.cmp(&right.span.start.line))
                .then(left.span.start.col.cmp(&right.span.start.col))
        });
        matches
    }

    pub fn find_at_position(&self, file_path: &Path, line: u32, col: u32) -> Option<&IndexedSymbol> {
        let needle = Position { line, col };

        self.symbols
            .iter()
            .filter(|symbol| symbol.file_path == file_path && span_contains(symbol.span, needle))
            .min_by(|left, right| compare_symbol_span(left.span, right.span))
    }
}

impl ReferenceIndex {
    pub fn len(&self) -> usize {
        self.references.len()
    }

    pub fn is_empty(&self) -> bool {
        self.references.is_empty()
    }

    pub fn find_at_position(&self, file_path: &Path, line: u32, col: u32) -> Option<&IndexedReference> {
        let needle = Position { line, col };

        self.references
            .iter()
            .filter(|reference| reference.file_path == file_path && span_contains(reference.span, needle))
            .min_by(|left, right| compare_symbol_span(left.span, right.span))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProjectIndex {
    pub files: Vec<FileSummary>,
    pub symbols: SymbolIndex,
    pub references: ReferenceIndex,
    pub skipped_files: Vec<PathBuf>,
}

impl ProjectIndex {
    pub fn stats(&self) -> ProjectIndexStats {
        let top_level_symbols = self
            .symbols
            .symbols
            .iter()
            .filter(|symbol| symbol.kind.is_top_level())
            .count();
        let total_symbols = self.symbols.len();

        ProjectIndexStats {
            files_indexed: self.files.len(),
            files_skipped: self.skipped_files.len(),
            top_level_symbols,
            member_symbols: total_symbols.saturating_sub(top_level_symbols),
            total_symbols,
        }
    }

    pub fn query_symbols(&self, query: &SymbolQuery) -> Vec<&IndexedSymbol> {
        self.symbols.query(query)
    }

    pub fn find_symbol_at(&self, file_path: &Path, line: u32, col: u32) -> Option<&IndexedSymbol> {
        self.symbols.find_at_position(file_path, line, col)
    }

    pub fn find_reference_at(&self, file_path: &Path, line: u32, col: u32) -> Option<&IndexedReference> {
        self.references.find_at_position(file_path, line, col)
    }

    pub fn resolve_reference_target<'a>(&'a self, reference: &IndexedReference) -> Option<&'a IndexedSymbol> {
        let qualified_name = reference.target_qualified_name.as_deref()?;
        self.symbols.find_by_qualified_name(qualified_name)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectIndexStats {
    pub files_indexed: usize,
    pub files_skipped: usize,
    pub top_level_symbols: usize,
    pub member_symbols: usize,
    pub total_symbols: usize,
}

pub fn build_project_index(source_files: &[PathBuf]) -> ProjectIndex {
    let mut index = ProjectIndex::default();

    for source_file in source_files {
        match summarize_file(source_file) {
            Some((summary, symbols, references)) => {
                index.files.push(summary);
                index.symbols.symbols.extend(symbols);
                index.references.references.extend(references);
            }
            None => index.skipped_files.push(source_file.clone()),
        }
    }

    index
}

fn summarize_file(path: &Path) -> Option<(FileSummary, Vec<IndexedSymbol>, Vec<IndexedReference>)> {
    let source = fs::read_to_string(path).ok()?;
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let file = parser.parse_file();

    if !parser.errors().is_empty() {
        return None;
    }

    let usings = file.usings.iter().map(|using| using.path.clone()).collect::<Vec<_>>();
    let (declaration, mut symbols, mut references) = summarize_decl(path, &file.decl);
    let symbol_count = symbols.len();

    Some((
        FileSummary {
            path: path.to_path_buf(),
            usings,
            declaration,
            symbol_count,
        },
        {
            symbols.shrink_to_fit();
            symbols
        },
        {
            references.shrink_to_fit();
            references
        },
    ))
}

fn summarize_decl(path: &Path, decl: &Decl) -> (DeclarationSummary, Vec<IndexedSymbol>, Vec<IndexedReference>) {
    match decl {
        Decl::Component {
            name,
            name_span,
            base_class,
            base_class_span,
            interfaces,
            interface_spans,
            members,
            ..
        } => summarize_named_decl(
            path,
            name,
            DeclarationKind::Component,
            Some(base_class.clone()),
            interfaces.clone(),
            component_signature(name, base_class, interfaces),
            summarize_members(path, name, members),
            {
                let mut references = summarize_decl_header_type_references(
                    path,
                    name,
                    Some((base_class.as_str(), *base_class_span)),
                    interfaces,
                    interface_spans,
                );
                references.extend(summarize_member_type_references(path, name, members));
                references
            },
            *name_span,
        ),
        Decl::Asset {
            name,
            name_span,
            base_class,
            base_class_span,
            members,
            ..
        } => summarize_named_decl(
            path,
            name,
            DeclarationKind::Asset,
            Some(base_class.clone()),
            Vec::new(),
            format!("asset {} : {}", name, base_class),
            summarize_members(path, name, members),
            {
                let mut references = summarize_decl_header_type_references(
                    path,
                    name,
                    Some((base_class.as_str(), *base_class_span)),
                    &[],
                    &[],
                );
                references.extend(summarize_member_type_references(path, name, members));
                references
            },
            *name_span,
        ),
        Decl::Class {
            name,
            name_span,
            super_class,
            super_class_span,
            interfaces,
            interface_spans,
            members,
            ..
        } => summarize_named_decl(
            path,
            name,
            DeclarationKind::Class,
            super_class.clone(),
            interfaces.clone(),
            class_signature(name, super_class.as_ref(), interfaces),
            summarize_members(path, name, members),
            {
                let mut references = summarize_decl_header_type_references(
                    path,
                    name,
                    super_class
                        .as_deref()
                        .zip(super_class_span.as_ref())
                        .map(|(super_class, span)| (super_class, *span)),
                    interfaces,
                    interface_spans,
                );
                references.extend(summarize_member_type_references(path, name, members));
                references
            },
            *name_span,
        ),
        Decl::DataClass {
            name,
            name_span,
            fields,
            ..
        } => summarize_named_decl(
            path,
            name,
            DeclarationKind::DataClass,
            None,
            Vec::new(),
            format!("data class {}({})", name, format_params(fields)),
            summarize_param_members(path, name, fields),
            summarize_param_type_references(path, name, fields),
            *name_span,
        ),
        Decl::Enum {
            name,
            name_span,
            params,
            entries,
            ..
        } => summarize_named_decl(
            path,
            name,
            DeclarationKind::Enum,
            None,
            Vec::new(),
            enum_signature(name, params),
            summarize_enum_entries(path, name, entries),
            summarize_enum_param_type_references(path, name, params),
            *name_span,
        ),
        Decl::Attribute {
            name,
            name_span,
            fields,
            targets,
            ..
        } => summarize_named_decl(
            path,
            name,
            DeclarationKind::Attribute,
            None,
            targets.clone(),
            attribute_signature(name, fields, targets),
            summarize_param_members(path, name, fields),
            summarize_param_type_references(path, name, fields),
            *name_span,
        ),
        Decl::Interface {
            name,
            name_span,
            extends,
            extends_spans,
            ..
        } => summarize_named_decl(
            path,
            name,
            DeclarationKind::Interface,
            None,
            extends.clone(),
            interface_signature(name, extends),
            vec![],
            summarize_decl_header_type_references(
                path,
                name,
                None,
                extends,
                extends_spans,
            ),
            *name_span,
        ),
        Decl::TypeAlias { name, name_span, target, .. } => {
            let sig = format!("typealias {} = {}", name, type_ref_signature(target));
            summarize_named_decl(
                path,
                name,
                DeclarationKind::Class,
                None,
                vec![],
                sig,
                vec![],
                vec![],
                *name_span,
            )
        }
        Decl::Struct { name, name_span, fields, .. } => {
            summarize_named_decl(
                path,
                name,
                DeclarationKind::DataClass,
                None,
                Vec::new(),
                format!("struct {}({})", name, format_params(fields)),
                summarize_param_members(path, name, fields),
                summarize_param_type_references(path, name, fields),
                *name_span,
            )
        }
        Decl::Extension { target_type, members, span } => {
            // Extensions don't introduce a new declaration of their own; instead they
            // augment an existing type. We synthesize an "extend" entry whose name is
            // the target type so that members are still indexed for navigation/lookup.
            let name = type_ref_signature(target_type);
            let synthesized_name = format!("extend {}", name);
            summarize_named_decl(
                path,
                &synthesized_name,
                DeclarationKind::Class,
                None,
                Vec::new(),
                format!("extend {}", name),
                summarize_members(path, &synthesized_name, members),
                {
                    let mut references = Vec::new();
                    collect_type_references(path, &synthesized_name, target_type, &mut references);
                    references.extend(summarize_member_type_references(path, &synthesized_name, members));
                    references
                },
                *span,
            )
        }
    }
}

fn summarize_named_decl(
    path: &Path,
    name: &str,
    kind: DeclarationKind,
    base_type: Option<String>,
    interfaces: Vec<String>,
    signature: String,
    members: Vec<MemberSummary>,
    references: Vec<IndexedReference>,
    span: Span,
) -> (DeclarationSummary, Vec<IndexedSymbol>, Vec<IndexedReference>) {
    let mut symbols = Vec::with_capacity(members.len() + 1);
    symbols.push(IndexedSymbol {
        name: name.to_string(),
        qualified_name: name.to_string(),
        container_name: None,
        kind: IndexedSymbolKind::Declaration(kind),
        file_path: path.to_path_buf(),
        signature: signature.clone(),
        span,
    });

    symbols.extend(members.iter().map(|member| IndexedSymbol {
        name: member.name.clone(),
        qualified_name: format!("{}.{}", name, member.name),
        container_name: Some(name.to_string()),
        kind: IndexedSymbolKind::Member(member.kind),
        file_path: path.to_path_buf(),
        signature: member.signature.clone(),
        span: member.span,
    }));

    (
        DeclarationSummary {
            name: name.to_string(),
            kind,
            base_type,
            interfaces,
            signature,
            members,
            span,
        },
        symbols,
        references,
    )
}

fn summarize_decl_header_type_references(
    path: &Path,
    container_name: &str,
    base_type: Option<(&str, Span)>,
    interfaces: &[String],
    interface_spans: &[Span],
) -> Vec<IndexedReference> {
    let mut references = Vec::with_capacity(base_type.iter().count() + interfaces.len());

    if let Some((base_name, base_span)) = base_type {
        references.push(summarize_named_type_reference(path, container_name, base_name, base_span));
    }

    for (interface_name, interface_span) in interfaces.iter().zip(interface_spans.iter()) {
        references.push(summarize_named_type_reference(path, container_name, interface_name, *interface_span));
    }

    references
}

fn summarize_named_type_reference(
    path: &Path,
    container_name: &str,
    name: &str,
    span: Span,
) -> IndexedReference {
    IndexedReference {
        name: name.to_string(),
        container_name: Some(container_name.to_string()),
        kind: IndexedReferenceKind::Type,
        file_path: path.to_path_buf(),
        span,
        target_qualified_name: Some(name.to_string()),
    }
}

fn summarize_param_type_references(path: &Path, container_name: &str, params: &[Param]) -> Vec<IndexedReference> {
    let mut references = Vec::new();
    for param in params {
        collect_type_references(path, container_name, &param.ty, &mut references);
    }
    references
}

fn summarize_enum_param_type_references(
    path: &Path,
    container_name: &str,
    params: &[crate::ast::EnumParam],
) -> Vec<IndexedReference> {
    let mut references = Vec::new();
    for param in params {
        collect_type_references(path, container_name, &param.ty, &mut references);
    }
    references
}

fn summarize_member_type_references(path: &Path, decl_name: &str, members: &[Member]) -> Vec<IndexedReference> {
    let mut references = Vec::new();

    for member in members {
        match member {
            Member::SerializeField { name, ty, .. }
            | Member::Require { name, ty, .. }
            | Member::Optional { name, ty, .. }
            | Member::Child { name, ty, .. }
            | Member::Parent { name, ty, .. } => {
                collect_type_references(path, &format!("{}.{}", decl_name, name), ty, &mut references);
            }
            Member::Field { name, ty, .. } => {
                if let Some(ty) = ty {
                    collect_type_references(path, &format!("{}.{}", decl_name, name), ty, &mut references);
                }
            }
            Member::Func {
                name,
                params,
                return_ty,
                body,
                ..
            } => {
                let container_name = format!("{}.{}", decl_name, name);
                references.extend(summarize_param_type_references(path, &container_name, params));
                if let Some(return_ty) = return_ty {
                    collect_type_references(path, &container_name, return_ty, &mut references);
                }
                collect_func_body_type_references(path, &container_name, body, &mut references);
            }
            Member::Coroutine { name, params, body, .. } => {
                let container_name = format!("{}.{}", decl_name, name);
                references.extend(summarize_param_type_references(path, &container_name, params));
                collect_block_type_references(path, &container_name, body, &mut references);
            }
            Member::Lifecycle { kind, params, body, .. } => {
                let container_name = format!("{}.{}", decl_name, lifecycle_name(*kind));
                references.extend(summarize_param_type_references(path, &container_name, params));
                collect_block_type_references(path, &container_name, body, &mut references);
            }
            Member::IntrinsicFunc {
                name,
                params,
                return_ty,
                ..
            } => {
                let container_name = format!("{}.{}", decl_name, name);
                references.extend(summarize_param_type_references(path, &container_name, params));
                if let Some(return_ty) = return_ty {
                    collect_type_references(path, &container_name, return_ty, &mut references);
                }
            }
            Member::IntrinsicCoroutine { name, params, .. } => {
                let container_name = format!("{}.{}", decl_name, name);
                references.extend(summarize_param_type_references(path, &container_name, params));
            }
            Member::Pool { name, item_type, .. } => {
                collect_type_references(path, &format!("{}.{}", decl_name, name), item_type, &mut references);
            }
            Member::Property {
                name,
                ty,
                getter,
                setter,
                ..
            } => {
                let container_name = format!("{}.{}", decl_name, name);
                collect_type_references(path, &container_name, ty, &mut references);
                if let Some(getter) = getter {
                    collect_func_body_type_references(path, &container_name, getter, &mut references);
                }
                if let Some(setter) = setter {
                    collect_block_type_references(path, &container_name, &setter.body, &mut references);
                }
            }
            Member::Event { name, ty, .. } => {
                collect_type_references(path, &format!("{}.{}", decl_name, name), ty, &mut references);
            }
            Member::StateMachine { name, states, .. } => {
                let container_name = format!("{}.{}", decl_name, name);
                for s in states {
                    if let Some(b) = &s.enter {
                        collect_block_type_references(path, &container_name, b, &mut references);
                    }
                    if let Some(b) = &s.exit {
                        collect_block_type_references(path, &container_name, b, &mut references);
                    }
                }
            }
            Member::Command { name, params, execute, undo, can_execute, .. } => {
                let container_name = format!("{}.{}", decl_name, name);
                references.extend(summarize_param_type_references(path, &container_name, params));
                collect_block_type_references(path, &container_name, execute, &mut references);
                if let Some(b) = undo {
                    collect_block_type_references(path, &container_name, b, &mut references);
                }
                if let Some(ce) = can_execute {
                    collect_expr_type_references(path, &container_name, ce, &mut references);
                }
            }
            Member::BindProperty { name, ty, init, .. } => {
                collect_type_references(path, &format!("{}.{}", decl_name, name), ty, &mut references);
                if let Some(expr) = init {
                    collect_expr_type_references(path, &format!("{}.{}", decl_name, name), expr, &mut references);
                }
            }
        }
    }

    references
}

fn collect_func_body_type_references(
    path: &Path,
    container_name: &str,
    body: &FuncBody,
    references: &mut Vec<IndexedReference>,
) {
    match body {
        FuncBody::Block(block) => collect_block_type_references(path, container_name, block, references),
        FuncBody::ExprBody(expr) => collect_expr_type_references(path, container_name, expr, references),
    }
}

fn collect_block_type_references(
    path: &Path,
    container_name: &str,
    block: &Block,
    references: &mut Vec<IndexedReference>,
) {
    for stmt in &block.stmts {
        collect_stmt_type_references(path, container_name, stmt, references);
    }
}

fn collect_stmt_type_references(
    path: &Path,
    container_name: &str,
    stmt: &Stmt,
    references: &mut Vec<IndexedReference>,
) {
    match stmt {
        Stmt::ValDecl { ty, init, .. } => {
            if let Some(ty) = ty {
                collect_type_references(path, container_name, ty, references);
            }
            collect_expr_type_references(path, container_name, init, references);
        }
        Stmt::VarDecl { ty, init, .. } => {
            if let Some(ty) = ty {
                collect_type_references(path, container_name, ty, references);
            }
            if let Some(init) = init {
                collect_expr_type_references(path, container_name, init, references);
            }
        }
        Stmt::Assignment { target, value, .. } => {
            collect_expr_type_references(path, container_name, target, references);
            collect_expr_type_references(path, container_name, value, references);
        }
        Stmt::Expr { expr, .. } => collect_expr_type_references(path, container_name, expr, references),
        Stmt::If {
            cond,
            then_block,
            else_branch,
            ..
        } => {
            collect_expr_type_references(path, container_name, cond, references);
            collect_block_type_references(path, container_name, then_block, references);
            if let Some(else_branch) = else_branch {
                collect_else_branch_type_references(path, container_name, else_branch, references);
            }
        }
        Stmt::When { subject, branches, .. } => {
            if let Some(subject) = subject {
                collect_expr_type_references(path, container_name, subject, references);
            }
            for branch in branches {
                collect_when_branch_type_references(path, container_name, branch, references);
            }
        }
        Stmt::For { iterable, body, .. } => {
            collect_expr_type_references(path, container_name, iterable, references);
            collect_block_type_references(path, container_name, body, references);
        }
        Stmt::While { cond, body, .. } => {
            collect_expr_type_references(path, container_name, cond, references);
            collect_block_type_references(path, container_name, body, references);
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                collect_expr_type_references(path, container_name, value, references);
            }
        }
        Stmt::Wait { form, .. } => collect_wait_form_type_references(path, container_name, form, references),
        Stmt::Start { call, .. } => collect_expr_type_references(path, container_name, call, references),
        Stmt::Stop { target, .. } => collect_expr_type_references(path, container_name, target, references),
        Stmt::Listen { event, body, .. } => {
            collect_expr_type_references(path, container_name, event, references);
            collect_block_type_references(path, container_name, body, references);
        }
        Stmt::DestructureVal { init, .. } => {
            collect_expr_type_references(path, container_name, init, references);
        }
        Stmt::StopAll { .. } | Stmt::IntrinsicBlock { .. } | Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Unlisten { .. } => {}
        Stmt::Try { try_block, catches, finally_block, .. } => {
            collect_block_type_references(path, container_name, try_block, references);
            for c in catches {
                collect_type_references(path, container_name, &c.ty, references);
                collect_block_type_references(path, container_name, &c.body, references);
            }
            if let Some(fb) = finally_block {
                collect_block_type_references(path, container_name, fb, references);
            }
        }
        Stmt::Throw { expr, .. } => {
            collect_expr_type_references(path, container_name, expr, references);
        }
        Stmt::Use { ty, init, body, .. } => {
            if let Some(ty) = ty {
                collect_type_references(path, container_name, ty, references);
            }
            collect_expr_type_references(path, container_name, init, references);
            if let Some(body) = body {
                collect_block_type_references(path, container_name, body, references);
            }
        }
        Stmt::BindTo { target, .. } => {
            collect_expr_type_references(path, container_name, target, references);
        }
        // Language 5, Sprint 1: yield + preprocessor walks.
        Stmt::Yield { value, .. } => {
            collect_expr_type_references(path, container_name, value, references);
        }
        Stmt::YieldBreak { .. } => {}
        Stmt::Preprocessor { arms, else_arm, .. } => {
            for arm in arms {
                for s in &arm.body {
                    collect_stmt_type_references(path, container_name, s, references);
                }
            }
            if let Some(else_stmts) = else_arm {
                for s in else_stmts {
                    collect_stmt_type_references(path, container_name, s, references);
                }
            }
        }
    }
}

fn collect_else_branch_type_references(
    path: &Path,
    container_name: &str,
    else_branch: &ElseBranch,
    references: &mut Vec<IndexedReference>,
) {
    match else_branch {
        ElseBranch::ElseBlock(block) => collect_block_type_references(path, container_name, block, references),
        ElseBranch::ElseIf(stmt) => collect_stmt_type_references(path, container_name, stmt, references),
    }
}

fn collect_when_branch_type_references(
    path: &Path,
    container_name: &str,
    branch: &WhenBranch,
    references: &mut Vec<IndexedReference>,
) {
    match &branch.pattern {
        WhenPattern::Expression(expr) => collect_expr_type_references(path, container_name, expr, references),
        WhenPattern::Is(ty) => collect_type_references(path, container_name, ty, references),
        WhenPattern::Binding { .. } => {} // no type references to collect from binding names
        WhenPattern::Else => {}
        WhenPattern::Or { patterns, .. } => {
            for p in patterns {
                match p {
                    WhenPattern::Expression(e) => collect_expr_type_references(path, container_name, e, references),
                    WhenPattern::Is(ty) => collect_type_references(path, container_name, ty, references),
                    _ => {}
                }
            }
        }
        WhenPattern::Range { start, end, .. } => {
            collect_expr_type_references(path, container_name, start, references);
            collect_expr_type_references(path, container_name, end, references);
        }
    }

    match &branch.body {
        WhenBody::Block(block) => collect_block_type_references(path, container_name, block, references),
        WhenBody::Expr(expr) => collect_expr_type_references(path, container_name, expr, references),
    }
}

fn collect_wait_form_type_references(
    path: &Path,
    container_name: &str,
    form: &WaitForm,
    references: &mut Vec<IndexedReference>,
) {
    match form {
        WaitForm::Duration(expr)
        | WaitForm::Until(expr)
        | WaitForm::While(expr) => collect_expr_type_references(path, container_name, expr, references),
        WaitForm::NextFrame | WaitForm::FixedFrame => {}
    }
}

fn collect_expr_type_references(
    path: &Path,
    container_name: &str,
    expr: &Expr,
    references: &mut Vec<IndexedReference>,
) {
    match expr {
        Expr::IntLit(..)
        | Expr::FloatLit(..)
        | Expr::DurationLit(..)
        | Expr::StringLit(..)
        | Expr::BoolLit(..)
        | Expr::Null(..)
        | Expr::Ident(..)
        | Expr::This(..)
        // Language 5, Sprint 2: nameof has no nested type references.
        | Expr::NameOf { .. } => {}
        Expr::StringInterp { parts, .. } => {
            for part in parts {
                if let StringPart::Expr(expr) = part {
                    collect_expr_type_references(path, container_name, expr, references);
                }
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_type_references(path, container_name, left, references);
            collect_expr_type_references(path, container_name, right, references);
        }
        Expr::Unary { operand, .. } | Expr::NonNullAssert { expr: operand, .. } => {
            collect_expr_type_references(path, container_name, operand, references);
        }
        Expr::MemberAccess { receiver, .. } | Expr::SafeCall { receiver, .. } => {
            collect_expr_type_references(path, container_name, receiver, references);
        }
        Expr::SafeMethodCall {
            receiver,
            type_args,
            args,
            ..
        } => {
            collect_expr_type_references(path, container_name, receiver, references);
            collect_call_type_references(path, container_name, type_args, args, references);
        }
        Expr::Call {
            receiver,
            type_args,
            args,
            ..
        } => {
            if let Some(receiver) = receiver {
                collect_expr_type_references(path, container_name, receiver, references);
            }
            collect_call_type_references(path, container_name, type_args, args, references);
        }
        Expr::Elvis { left, right, .. } => {
            collect_expr_type_references(path, container_name, left, references);
            collect_expr_type_references(path, container_name, right, references);
        }
        Expr::IndexAccess { receiver, index, .. } => {
            collect_expr_type_references(path, container_name, receiver, references);
            collect_expr_type_references(path, container_name, index, references);
        }
        Expr::IfExpr {
            cond,
            then_block,
            else_block,
            ..
        } => {
            collect_expr_type_references(path, container_name, cond, references);
            collect_block_type_references(path, container_name, then_block, references);
            collect_block_type_references(path, container_name, else_block, references);
        }
        Expr::WhenExpr { subject, branches, .. } => {
            if let Some(subject) = subject {
                collect_expr_type_references(path, container_name, subject, references);
            }
            for branch in branches {
                collect_when_branch_type_references(path, container_name, branch, references);
            }
        }
        Expr::Range { start, end, step, .. } => {
            collect_expr_type_references(path, container_name, start, references);
            collect_expr_type_references(path, container_name, end, references);
            if let Some(step) = step {
                collect_expr_type_references(path, container_name, step, references);
            }
        }
        Expr::Is { expr, ty, .. } => {
            collect_expr_type_references(path, container_name, expr, references);
            collect_type_references(path, container_name, ty, references);
        }
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Block(block) => collect_block_type_references(path, container_name, block, references),
            LambdaBody::Expr(expr) => collect_expr_type_references(path, container_name, expr, references),
        },
        Expr::IntrinsicExpr { ty, .. } => collect_type_references(path, container_name, ty, references),
        Expr::SafeCastExpr { expr, target_type, .. } => {
            collect_expr_type_references(path, container_name, expr, references);
            collect_type_references(path, container_name, target_type, references);
        }
        Expr::ForceCastExpr { expr, target_type, .. } => {
            collect_expr_type_references(path, container_name, expr, references);
            collect_type_references(path, container_name, target_type, references);
        }
        Expr::Tuple { elements, .. } => {
            for e in elements {
                collect_expr_type_references(path, container_name, e, references);
            }
        }
        Expr::ListLit { elements, .. } => {
            for e in elements {
                collect_expr_type_references(path, container_name, e, references);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_expr_type_references(path, container_name, k, references);
                collect_expr_type_references(path, container_name, v, references);
            }
        }
        Expr::Await { expr: inner, .. } => {
            collect_expr_type_references(path, container_name, inner, references);
        }
    }
}

fn collect_call_type_references(
    path: &Path,
    container_name: &str,
    type_args: &[TypeRef],
    args: &[Arg],
    references: &mut Vec<IndexedReference>,
) {
    for type_arg in type_args {
        collect_type_references(path, container_name, type_arg, references);
    }
    for arg in args {
        collect_expr_type_references(path, container_name, &arg.value, references);
    }
}

fn collect_type_references(
    path: &Path,
    container_name: &str,
    ty: &TypeRef,
    references: &mut Vec<IndexedReference>,
) {
    match ty {
        TypeRef::Simple { name, span, .. } => {
            references.push(summarize_named_type_reference(path, container_name, name, *span));
        }
        TypeRef::Generic {
            name,
            type_args,
            span,
            ..
        } => {
            references.push(summarize_named_type_reference(path, container_name, name, *span));
            for type_arg in type_args {
                collect_type_references(path, container_name, type_arg, references);
            }
        }
        TypeRef::Qualified {
            qualifier,
            name,
            span,
            ..
        } => {
            references.push(summarize_named_type_reference(
                path,
                container_name,
                &format!("{}.{}", qualifier, name),
                *span,
            ));
        }
        TypeRef::Tuple { types, .. } => {
            for t in types {
                collect_type_references(path, container_name, t, references);
            }
        }
        TypeRef::Function { param_types, return_type, .. } => {
            for t in param_types {
                collect_type_references(path, container_name, t, references);
            }
            collect_type_references(path, container_name, return_type, references);
        }
    }
}

fn summarize_members(path: &Path, container_name: &str, members: &[Member]) -> Vec<MemberSummary> {
    let _ = path;
    let _ = container_name;

    members
        .iter()
        .map(|member| match member {
            Member::SerializeField {
                mutability,
                name,
                name_span,
                ty,
                visibility,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::SerializeField,
                signature: format!(
                    "{}serialize {} {}: {}",
                    visibility_prefix(*visibility),
                    mutability_keyword(*mutability),
                    name,
                    format_type_ref(ty)
                )
                .trim()
                .to_string(),
                span: *name_span,
            },
            Member::Field {
                visibility,
                mutability,
                name,
                name_span,
                ty,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Field,
                signature: match ty {
                    Some(ty) => format!(
                        "{}{} {}: {}",
                        visibility_prefix(Some(*visibility)),
                        mutability_keyword(*mutability),
                        name,
                        format_type_ref(ty)
                    ),
                    None => format!(
                        "{}{} {}",
                        visibility_prefix(Some(*visibility)),
                        mutability_keyword(*mutability),
                        name
                    ),
                }
                .trim()
                .to_string(),
                span: *name_span,
            },
            Member::Require {
                name,
                name_span,
                ty,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::RequiredComponent,
                signature: format!("require {}: {}", name, format_type_ref(ty)),
                span: *name_span,
            },
            Member::Optional {
                name,
                name_span,
                ty,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::OptionalComponent,
                signature: format!("optional {}: {}", name, format_type_ref(ty)),
                span: *name_span,
            },
            Member::Child {
                name,
                name_span,
                ty,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::ChildComponent,
                signature: format!("child {}: {}", name, format_type_ref(ty)),
                span: *name_span,
            },
            Member::Parent {
                name,
                name_span,
                ty,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::ParentComponent,
                signature: format!("parent {}: {}", name, format_type_ref(ty)),
                span: *name_span,
            },
            Member::Func {
                visibility,
                name,
                name_span,
                params,
                return_ty,
                body,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Function,
                signature: format!(
                    "{}func {}({}){}{}",
                    visibility_prefix(Some(*visibility)),
                    name,
                    format_params(params),
                    return_type_suffix(return_ty.as_ref()),
                    match body {
                        FuncBody::ExprBody(_) => " = ...",
                        FuncBody::Block(_) => "",
                    }
                )
                .trim()
                .to_string(),
                span: *name_span,
            },
            Member::Coroutine {
                name,
                name_span,
                params,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Coroutine,
                signature: format!("coroutine {}({})", name, format_params(params)),
                span: *name_span,
            },
            Member::Lifecycle {
                kind,
                params,
                span,
                ..
            } => MemberSummary {
                name: lifecycle_name(*kind).to_string(),
                kind: MemberKind::Lifecycle,
                signature: format!("{}({})", lifecycle_name(*kind), format_params(params)),
                span: *span,
            },
            Member::IntrinsicFunc {
                visibility,
                name,
                name_span,
                params,
                return_ty,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Function,
                signature: format!(
                    "{}intrinsic func {}({}){}",
                    visibility_prefix(Some(*visibility)),
                    name,
                    format_params(params),
                    return_type_suffix(return_ty.as_ref())
                )
                .trim()
                .to_string(),
                span: *name_span,
            },
            Member::IntrinsicCoroutine {
                name,
                name_span,
                params,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Coroutine,
                signature: format!("intrinsic coroutine {}({})", name, format_params(params)),
                span: *name_span,
            },
            Member::Pool {
                name,
                name_span,
                item_type,
                capacity,
                max_size,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Field,
                signature: format!(
                    "pool {}: {}(capacity = {}, max = {})",
                    name,
                    format_type_ref(item_type),
                    capacity,
                    max_size
                ),
                span: *name_span,
            },
            Member::Event {
                visibility,
                name,
                name_span,
                ty,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Field,
                signature: format!(
                    "{}event {}: {}",
                    visibility_prefix(Some(*visibility)),
                    name,
                    format_type_ref(ty)
                )
                .trim()
                .to_string(),
                span: *name_span,
            },
            Member::Property {
                mutability,
                name,
                name_span,
                ty,
                ..
            } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Field,
                signature: format!(
                    "{} {}: {}",
                    mutability_keyword(*mutability),
                    name,
                    format_type_ref(ty)
                )
                .trim()
                .to_string(),
                span: *name_span,
            },
            Member::StateMachine { name, name_span, .. } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Field,
                signature: format!("state machine {}", name),
                span: *name_span,
            },
            Member::Command { name, name_span, params, .. } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Function,
                signature: format!("command {}({})", name, format_params(params)),
                span: *name_span,
            },
            Member::BindProperty { name, name_span, ty, .. } => MemberSummary {
                name: name.clone(),
                kind: MemberKind::Field,
                signature: format!("bind {}: {}", name, format_type_ref(ty)),
                span: *name_span,
            },
        })
        .collect()
}

fn summarize_param_members(_path: &Path, _container_name: &str, params: &[Param]) -> Vec<MemberSummary> {
    params
        .iter()
        .map(|param| MemberSummary {
            name: param.name.clone(),
            kind: MemberKind::Field,
            signature: format!("{}: {}", param.name, format_type_ref(&param.ty)),
            span: param.name_span,
        })
        .collect()
}

fn summarize_enum_entries(_path: &Path, _container_name: &str, entries: &[EnumEntry]) -> Vec<MemberSummary> {
    entries
        .iter()
        .map(|entry| MemberSummary {
            name: entry.name.clone(),
            kind: MemberKind::EnumEntry,
            signature: if entry.args.is_empty() {
                entry.name.clone()
            } else {
                format!("{}(...)", entry.name)
            },
            span: entry.name_span,
        })
        .collect()
}

fn component_signature(name: &str, base_class: &str, interfaces: &[String]) -> String {
    let mut bases = vec![base_class.to_string()];
    bases.extend(interfaces.iter().cloned());
    format!("component {} : {}", name, bases.join(", "))
}

fn class_signature(name: &str, super_class: Option<&String>, interfaces: &[String]) -> String {
    let mut bases = Vec::new();
    if let Some(super_class) = super_class {
        bases.push(super_class.clone());
    }
    bases.extend(interfaces.iter().cloned());

    if bases.is_empty() {
        format!("class {}", name)
    } else {
        format!("class {} : {}", name, bases.join(", "))
    }
}

fn enum_signature(name: &str, params: &[crate::ast::EnumParam]) -> String {
    if params.is_empty() {
        format!("enum {}", name)
    } else {
        let params = params
            .iter()
            .map(|param| format!("val {}: {}", param.name, format_type_ref(&param.ty)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("enum {}({})", name, params)
    }
}

fn type_ref_signature(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Simple { name, nullable, .. } => {
            if *nullable { format!("{}?", name) } else { name.clone() }
        }
        TypeRef::Generic { name, type_args, nullable, .. } => {
            let args: Vec<String> = type_args.iter().map(type_ref_signature).collect();
            let base = format!("{}<{}>", name, args.join(", "));
            if *nullable { format!("{}?", base) } else { base }
        }
        TypeRef::Qualified { qualifier, name, nullable, .. } => {
            let base = format!("{}.{}", qualifier, name);
            if *nullable { format!("{}?", base) } else { base }
        }
        TypeRef::Tuple { types, nullable, .. } => {
            let inner: Vec<String> = types.iter().map(type_ref_signature).collect();
            let base = format!("({})", inner.join(", "));
            if *nullable { format!("{}?", base) } else { base }
        }
        TypeRef::Function { param_types, return_type, nullable, .. } => {
            let inner: Vec<String> = param_types.iter().map(type_ref_signature).collect();
            let base = format!("({}) => {}", inner.join(", "), type_ref_signature(return_type));
            if *nullable { format!("{}?", base) } else { base }
        }
    }
}

fn interface_signature(name: &str, extends: &[String]) -> String {
    if extends.is_empty() {
        format!("interface {}", name)
    } else {
        format!("interface {} : {}", name, extends.join(", "))
    }
}

fn attribute_signature(name: &str, fields: &[Param], targets: &[String]) -> String {
    let mut signature = format!("attribute {}({})", name, format_params(fields));
    if !targets.is_empty() {
        signature.push_str(" : ");
        signature.push_str(&targets.join(", "));
    }
    signature
}

fn format_params(params: &[Param]) -> String {
    params
        .iter()
        .map(|param| format!("{}: {}", param.name, format_type_ref(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn return_type_suffix(return_ty: Option<&TypeRef>) -> String {
    match return_ty {
        Some(ty) => format!(": {}", format_type_ref(ty)),
        None => String::new(),
    }
}

fn format_type_ref(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Simple { name, nullable, .. } => format_nullable(name.clone(), *nullable),
        TypeRef::Generic {
            name,
            type_args,
            nullable,
            ..
        } => format_nullable(
            format!(
                "{}<{}>",
                name,
                type_args
                    .iter()
                    .map(format_type_ref)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            *nullable,
        ),
        TypeRef::Qualified {
            qualifier,
            name,
            nullable,
            ..
        } => format_nullable(format!("{}.{}", qualifier, name), *nullable),
        TypeRef::Tuple { types, nullable, .. } => {
            let inner: Vec<String> = types.iter().map(format_type_ref).collect();
            format_nullable(format!("({})", inner.join(", ")), *nullable)
        }
        TypeRef::Function { param_types, return_type, nullable, .. } => {
            let inner: Vec<String> = param_types.iter().map(format_type_ref).collect();
            format_nullable(
                format!("({}) => {}", inner.join(", "), format_type_ref(return_type)),
                *nullable,
            )
        }
    }
}

fn format_nullable(value: String, nullable: bool) -> String {
    if nullable {
        format!("{}?", value)
    } else {
        value
    }
}

fn lifecycle_name(kind: LifecycleKind) -> &'static str {
    match kind {
        LifecycleKind::Awake => "awake",
        LifecycleKind::Start => "start",
        LifecycleKind::Update => "update",
        LifecycleKind::FixedUpdate => "fixedUpdate",
        LifecycleKind::LateUpdate => "lateUpdate",
        LifecycleKind::OnEnable => "onEnable",
        LifecycleKind::OnDisable => "onDisable",
        LifecycleKind::OnDestroy => "onDestroy",
        LifecycleKind::OnTriggerEnter => "onTriggerEnter",
        LifecycleKind::OnTriggerExit => "onTriggerExit",
        LifecycleKind::OnTriggerStay => "onTriggerStay",
        LifecycleKind::OnCollisionEnter => "onCollisionEnter",
        LifecycleKind::OnCollisionExit => "onCollisionExit",
        LifecycleKind::OnCollisionStay => "onCollisionStay",
    }
}

fn visibility_prefix(visibility: Option<Visibility>) -> &'static str {
    match visibility {
        Some(Visibility::Public) => "public ",
        Some(Visibility::Private) => "private ",
        Some(Visibility::Protected) => "protected ",
        None => "",
    }
}

fn mutability_keyword(mutability: crate::ast::Mutability) -> &'static str {
    match mutability {
        crate::ast::Mutability::Val => "val",
        crate::ast::Mutability::Var => "var",
        crate::ast::Mutability::Const => "const",
        crate::ast::Mutability::Fixed => "fixed",
    }
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

fn compare_symbol_span(left: Span, right: Span) -> std::cmp::Ordering {
    let left_size = span_size(left);
    let right_size = span_size(right);

    left_size
        .cmp(&right_size)
        .then(left.start.line.cmp(&right.start.line))
        .then(left.start.col.cmp(&right.start.col))
}

fn span_size(span: Span) -> u64 {
    let line_delta = span.end.line.saturating_sub(span.start.line) as u64;
    let col_delta = span.end.col.saturating_sub(span.start.col) as u64;
    line_delta * 10_000 + col_delta
}

#[cfg(test)]
mod tests {
    use super::{
        build_project_index, DeclarationKind, IndexedReferenceKind, IndexedSymbolKind, MemberKind, SymbolQuery,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{}_{}", prefix, unique));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn build_project_index_collects_files_and_symbols() {
        let root = unique_temp_dir("prism_project_index");
        let player = root.join("Player.prsm");
        let state = root.join("EnemyState.prsm");

        write_file(
            &player,
            r#"using UnityEngine

component Player : MonoBehaviour {
    serialize var speed: Float = 5.0

    func jump(height: Float): Unit {
        return
    }

    onDisable { }
}
"#,
        );
        write_file(
            &state,
            r#"enum EnemyState {
    Idle,
    Chase,
}
"#,
        );

        let index = build_project_index(&[player.clone(), state.clone()]);
        let stats = index.stats();

        assert_eq!(stats.files_indexed, 2);
        assert_eq!(stats.files_skipped, 0);
        assert_eq!(stats.top_level_symbols, 2);
        assert_eq!(stats.member_symbols, 5);
        assert_eq!(stats.total_symbols, 7);

        let player_summary = index
            .files
            .iter()
            .find(|summary| summary.path == player)
            .unwrap();
        assert_eq!(player_summary.usings, vec!["UnityEngine"]);
        assert_eq!(player_summary.declaration.kind, DeclarationKind::Component);
        assert_eq!(player_summary.declaration.members.len(), 3);
        assert_eq!(player_summary.symbol_count, 4);

        let jump = index
            .symbols
            .find_by_qualified_name("Player.jump")
            .unwrap();
        assert_eq!(jump.signature, "public func jump(height: Float): Unit");
        assert_eq!(jump.kind, IndexedSymbolKind::Member(MemberKind::Function));

        let chase = index
            .symbols
            .find_by_qualified_name("EnemyState.Chase")
            .unwrap();
        assert_eq!(chase.kind, IndexedSymbolKind::Member(MemberKind::EnumEntry));

        let jump_results = index.query_symbols(&SymbolQuery {
            name: Some("jump".into()),
            qualified_name: None,
        });
        assert_eq!(jump_results.len(), 1);
        assert_eq!(jump_results[0].qualified_name, "Player.jump");

        let hover_target = index
            .symbols
            .find_at_position(&player, 6, 10)
            .expect("expected symbol at function span");
        assert_eq!(hover_target.qualified_name, "Player.jump");

        let base_type_reference = index
            .find_reference_at(&player, 3, 24)
            .expect("expected header type reference");
        assert_eq!(base_type_reference.name, "MonoBehaviour");
        assert_eq!(base_type_reference.kind, IndexedReferenceKind::Type);
        assert!(index.resolve_reference_target(base_type_reference).is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn build_project_index_skips_invalid_files() {
        let root = unique_temp_dir("prism_project_index_invalid");
        let broken = root.join("Broken.prsm");
        write_file(&broken, "component Broken : MonoBehaviour { func }");

        let index = build_project_index(&[broken.clone()]);
        let stats = index.stats();

        assert_eq!(stats.files_indexed, 0);
        assert_eq!(stats.files_skipped, 1);
        assert!(index.symbols.is_empty());
        assert_eq!(index.skipped_files, vec![broken]);

        let _ = fs::remove_dir_all(root);
    }
}