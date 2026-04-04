use crate::ast::*;
use crate::diagnostics::DiagnosticCollector;
use crate::hir::{HirDefinition, HirDefinitionKind, HirFile, HirReference, HirReferenceKind};
use crate::lexer::token::Span;
use super::types::*;
use super::scope::*;
use std::path::{Path, PathBuf};

/// Declaration context — what kind of declaration are we inside?
#[derive(Debug, Clone, Copy, PartialEq)]
enum DeclContext {
    Component,
    Asset,
    Class,
    DataClass,
    Enum,
    None,
}

/// Body context — are we inside a coroutine?
#[derive(Debug, Clone, Copy, PartialEq)]
enum BodyContext {
    Function,
    Coroutine,
    Lifecycle,
    None,
}

/// The semantic analyzer — validates an AST and produces diagnostics.
pub struct Analyzer {
    pub diag: DiagnosticCollector,
    scopes: ScopeStack,
    decl_ctx: DeclContext,
    body_ctx: BodyContext,
    loop_depth: u32,
    /// Known enum names → entries (for exhaustiveness checking)
    enum_entries: std::collections::HashMap<String, Vec<String>>,
    known_project_types: std::collections::HashMap<String, MoonType>,
    current_file_path: Option<PathBuf>,
    hir_definitions: Vec<HirDefinition>,
    hir_references: Vec<HirReference>,
    next_definition_id: u32,
    current_decl_name: Option<String>,
    current_member_name: Option<String>,
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer {
            diag: DiagnosticCollector::new(),
            scopes: ScopeStack::new(),
            decl_ctx: DeclContext::None,
            body_ctx: BodyContext::None,
            loop_depth: 0,
            enum_entries: std::collections::HashMap::new(),
            known_project_types: std::collections::HashMap::new(),
            current_file_path: None,
            hir_definitions: Vec::new(),
            hir_references: Vec::new(),
            next_definition_id: 1,
            current_decl_name: None,
            current_member_name: None,
        }
    }

    pub fn with_known_project_types(
        known_project_types: std::collections::HashMap<String, MoonType>,
    ) -> Self {
        let mut analyzer = Self::new();
        analyzer.known_project_types = known_project_types;
        analyzer
    }

    /// Analyze a file.
    pub fn analyze_file(&mut self, file: &File) {
        // Phase 1: Register the top-level declaration
        self.register_decl(&file.decl);

        // Phase 2: Analyze the declaration body
        self.analyze_decl(&file.decl);
    }

    pub fn analyze_file_with_hir(&mut self, file: &File, file_path: &Path) -> HirFile {
        self.begin_hir(file_path);
        self.analyze_file(file);
        self.finish_hir(file_path)
    }

    fn register_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Component { name, name_span, .. } => {
                let ty = MoonType::Component(name.clone());
                let definition_id = self.record_definition(
                    name.clone(),
                    name.clone(),
                    HirDefinitionKind::Type,
                    ty.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty,
                    kind: SymbolKind::Type,
                    mutable: false,
                    definition_id,
                });
            }
            Decl::Asset { name, name_span, .. } => {
                let ty = MoonType::Asset(name.clone());
                let definition_id = self.record_definition(
                    name.clone(),
                    name.clone(),
                    HirDefinitionKind::Type,
                    ty.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty,
                    kind: SymbolKind::Type,
                    mutable: false,
                    definition_id,
                });
            }
            Decl::Class { name, name_span, .. } => {
                let ty = MoonType::Class(name.clone());
                let definition_id = self.record_definition(
                    name.clone(),
                    name.clone(),
                    HirDefinitionKind::Type,
                    ty.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty,
                    kind: SymbolKind::Type,
                    mutable: false,
                    definition_id,
                });
            }
            Decl::DataClass { name, name_span, .. } => {
                let ty = MoonType::Class(name.clone());
                let definition_id = self.record_definition(
                    name.clone(),
                    name.clone(),
                    HirDefinitionKind::Type,
                    ty.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty,
                    kind: SymbolKind::Type,
                    mutable: false,
                    definition_id,
                });
            }
            Decl::Enum {
                name,
                name_span,
                entries,
                ..
            } => {
                let ty = MoonType::Enum(name.clone());
                let definition_id = self.record_definition(
                    name.clone(),
                    name.clone(),
                    HirDefinitionKind::Type,
                    ty.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty,
                    kind: SymbolKind::Type,
                    mutable: false,
                    definition_id,
                });
                let entry_names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
                self.enum_entries.insert(name.clone(), entry_names);
            }
            Decl::Attribute { name, name_span, .. } => {
                let ty = MoonType::Class(name.clone());
                let definition_id = self.record_definition(
                    name.clone(),
                    name.clone(),
                    HirDefinitionKind::Type,
                    ty.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty,
                    kind: SymbolKind::Type,
                    mutable: false,
                    definition_id,
                });
            }
        }
    }

    fn analyze_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Component {
                name,
                base_class,
                base_class_span,
                interfaces,
                interface_spans,
                members,
                ..
            } => {
                self.decl_ctx = DeclContext::Component;
                self.current_decl_name = Some(name.clone());
                let _ = self.resolve_named_typeref(base_class, *base_class_span);
                for (interface_name, interface_span) in interfaces.iter().zip(interface_spans.iter()) {
                    let _ = self.resolve_named_typeref(interface_name, *interface_span);
                }
                self.scopes.push_scope();
                self.analyze_members(members);
                self.check_duplicate_lifecycles(members);
                self.scopes.pop_scope();
                self.current_decl_name = None;
                self.decl_ctx = DeclContext::None;
            }
            Decl::Asset {
                name,
                base_class,
                base_class_span,
                members,
                ..
            } => {
                self.decl_ctx = DeclContext::Asset;
                self.current_decl_name = Some(name.clone());
                let _ = self.resolve_named_typeref(base_class, *base_class_span);
                self.scopes.push_scope();
                self.analyze_members(members);

                // Assets cannot have lifecycle blocks
                for m in members {
                    if let Member::Lifecycle { kind, span, .. } = m {
                        self.diag.error("E012",
                            format!("Lifecycle block '{:?}' is not valid inside an asset declaration", kind),
                            *span);
                    }
                    // Assets cannot have require/optional/child/parent
                    match m {
                        Member::Require { span, .. } => {
                            self.diag.error("E013", "'require' fields are only valid inside component declarations", *span);
                        }
                        Member::Optional { span, .. } => {
                            self.diag.error("E013", "'optional' fields are only valid inside component declarations", *span);
                        }
                        Member::Child { span, .. } => {
                            self.diag.error("E013", "'child' fields are only valid inside component declarations", *span);
                        }
                        Member::Parent { span, .. } => {
                            self.diag.error("E013", "'parent' fields are only valid inside component declarations", *span);
                        }
                        Member::Coroutine { span, .. } => {
                            self.diag.error("E060", "Coroutines are only valid inside component declarations", *span);
                        }
                        _ => {}
                    }
                }

                self.scopes.pop_scope();
                self.current_decl_name = None;
                self.decl_ctx = DeclContext::None;
            }
            Decl::Class {
                name,
                super_class,
                super_class_span,
                interfaces,
                interface_spans,
                members,
                ..
            } => {
                self.decl_ctx = DeclContext::Class;
                self.current_decl_name = Some(name.clone());
                if let (Some(super_class), Some(super_class_span)) = (super_class.as_ref(), super_class_span) {
                    let _ = self.resolve_named_typeref(super_class, *super_class_span);
                }
                for (interface_name, interface_span) in interfaces.iter().zip(interface_spans.iter()) {
                    let _ = self.resolve_named_typeref(interface_name, *interface_span);
                }
                self.scopes.push_scope();

                for m in members {
                    match m {
                        Member::Lifecycle { kind, span, .. } => {
                            self.diag.error("E012",
                                format!("Lifecycle block '{:?}' is not valid inside a class declaration", kind),
                                *span);
                        }
                        Member::Require { span, .. } | Member::Optional { span, .. }
                        | Member::Child { span, .. } | Member::Parent { span, .. } => {
                            self.diag.error("E013", "'require/optional/child/parent' fields are only valid inside component declarations", *span);
                        }
                        Member::Coroutine { span, .. } => {
                            self.diag.error("E060", "Coroutines are only valid inside component declarations", *span);
                        }
                        _ => {}
                    }
                }

                self.analyze_members(members);
                self.scopes.pop_scope();
                self.current_decl_name = None;
                self.decl_ctx = DeclContext::None;
            }
            Decl::DataClass { name, fields, span, .. } => {
                self.decl_ctx = DeclContext::DataClass;
                self.current_decl_name = Some(name.clone());
                // Validate fields
                if fields.is_empty() {
                    self.diag.warning("W005", "Data class has no fields", *span);
                }
                for field in fields {
                    let ty = self.resolve_typeref(&field.ty);
                    let _ = self.record_member_definition(
                        &field.name,
                        HirDefinitionKind::Field,
                        ty,
                        false,
                        field.name_span,
                    );
                }
                self.current_decl_name = None;
                self.decl_ctx = DeclContext::None;
            }
            Decl::Enum { name, params, entries, span, .. } => {
                self.decl_ctx = DeclContext::Enum;
                self.current_decl_name = Some(name.clone());
                if entries.is_empty() {
                    self.diag.error("E050", format!("Enum '{}' must have at least one entry", name), *span);
                }
                // Check duplicate entries
                let mut seen = std::collections::HashSet::new();
                for entry in entries {
                    let _ = self.record_member_definition(
                        &entry.name,
                        HirDefinitionKind::EnumEntry,
                        MoonType::Enum(name.clone()),
                        false,
                        entry.name_span,
                    );
                    if !seen.insert(&entry.name) {
                        self.diag.error("E052", format!("Duplicate enum entry '{}'", entry.name), entry.span);
                    }
                    if entry.args.len() != params.len() {
                        self.diag.error(
                            "E051",
                            format!(
                                "Enum entry '{}' of '{}' expects {} argument(s), found {}",
                                entry.name,
                                name,
                                params.len(),
                                entry.args.len(),
                            ),
                            entry.span,
                        );
                    }
                }
                self.current_decl_name = None;
                self.decl_ctx = DeclContext::None;
            }
            Decl::Attribute { name, fields, .. } => {
                self.current_decl_name = Some(name.clone());
                for field in fields {
                    let ty = self.resolve_typeref(&field.ty);
                    let _ = self.record_member_definition(
                        &field.name,
                        HirDefinitionKind::Field,
                        ty,
                        false,
                        field.name_span,
                    );
                }
                self.current_decl_name = None;
                // Attribute declarations are validated at parse time
            }
        }
    }

    fn analyze_members(&mut self, members: &[Member]) {
        // First pass: register all members in scope
        for m in members {
            self.register_member(m);
        }

        // Second pass: analyze member bodies
        for m in members {
            self.analyze_member_body(m);
        }
    }

    fn register_member(&mut self, member: &Member) {
        match member {
            Member::SerializeField {
                name,
                name_span,
                ty,
                ..
            } => {
                let btype = self.resolve_typeref(ty);
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Field,
                    btype.clone(),
                    true,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::SerializeField, mutable: true,
                    definition_id,
                });
            }
            Member::Field {
                name,
                name_span,
                ty,
                mutability,
                ..
            } => {
                let btype = ty.as_ref().map(|t| self.resolve_typeref(t)).unwrap_or(MoonType::Error);
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Field,
                    btype.clone(),
                    *mutability == Mutability::Var,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype,
                    kind: SymbolKind::Field,
                    mutable: *mutability == Mutability::Var,
                    definition_id,
                });
            }
            Member::Require {
                name,
                name_span,
                ty,
                ..
            } => {
                let btype = self.resolve_typeref(ty);
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Field,
                    btype.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::RequiredComponent, mutable: false,
                    definition_id,
                });
            }
            Member::Optional {
                name,
                name_span,
                ty,
                ..
            } => {
                let btype = self.resolve_typeref(ty).make_nullable();
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Field,
                    btype.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::OptionalComponent, mutable: false,
                    definition_id,
                });
            }
            Member::Child {
                name,
                name_span,
                ty,
                ..
            } => {
                let btype = self.resolve_typeref(ty);
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Field,
                    btype.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::RequiredComponent, mutable: false,
                    definition_id,
                });
            }
            Member::Parent {
                name,
                name_span,
                ty,
                ..
            } => {
                let btype = self.resolve_typeref(ty);
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Field,
                    btype.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::RequiredComponent, mutable: false,
                    definition_id,
                });
            }
            Member::Func {
                name,
                name_span,
                return_ty,
                ..
            } => {
                let ret = return_ty.as_ref().map(|t| self.resolve_typeref(t)).unwrap_or(MoonType::Unit);
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Function,
                    ret.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: ret, kind: SymbolKind::Function, mutable: false,
                    definition_id,
                });
            }
            Member::Coroutine {
                name,
                name_span,
                ..
            } => {
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Coroutine,
                    MoonType::Unit,
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: MoonType::Unit, kind: SymbolKind::Coroutine, mutable: false,
                    definition_id,
                });
            }
            Member::Lifecycle { kind, span, .. } => {
                let _ = self.record_member_definition(
                    lifecycle_name(*kind),
                    HirDefinitionKind::Lifecycle,
                    MoonType::Unit,
                    false,
                    *span,
                );
            }
            Member::IntrinsicFunc {
                name,
                name_span,
                return_ty,
                ..
            } => {
                let ret = return_ty.as_ref().map(|t| self.resolve_typeref(t)).unwrap_or(MoonType::Unit);
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Function,
                    ret.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: ret, kind: SymbolKind::Function, mutable: false,
                    definition_id,
                });
            }
            Member::IntrinsicCoroutine {
                name,
                name_span,
                ..
            } => {
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Coroutine,
                    MoonType::Unit,
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: MoonType::Unit, kind: SymbolKind::Coroutine, mutable: false,
                    definition_id,
                });
            }
        }
    }

    fn analyze_member_body(&mut self, member: &Member) {
        match member {
            Member::Func { name, params, body, .. } => {
                self.body_ctx = BodyContext::Function;
                self.current_member_name = Some(name.clone());
                self.scopes.push_scope();
                for p in params {
                    let ty = self.resolve_typeref(&p.ty);
                    let definition_id = self.record_nested_definition(
                        &p.name,
                        HirDefinitionKind::Parameter,
                        ty.clone(),
                        false,
                        p.name_span,
                    );
                    self.scopes.define(Symbol {
                        name: p.name.clone(), ty, kind: SymbolKind::Parameter, mutable: false,
                        definition_id,
                    });
                }
                match body {
                    FuncBody::Block(block) => self.analyze_block(block),
                    FuncBody::ExprBody(expr) => { self.analyze_expr(expr); }
                }
                self.scopes.pop_scope();
                self.current_member_name = None;
                self.body_ctx = BodyContext::None;
            }
            Member::Coroutine { name, params, body, .. } => {
                self.body_ctx = BodyContext::Coroutine;
                self.current_member_name = Some(name.clone());
                self.scopes.push_scope();
                for p in params {
                    let ty = self.resolve_typeref(&p.ty);
                    let definition_id = self.record_nested_definition(
                        &p.name,
                        HirDefinitionKind::Parameter,
                        ty.clone(),
                        false,
                        p.name_span,
                    );
                    self.scopes.define(Symbol {
                        name: p.name.clone(), ty, kind: SymbolKind::Parameter, mutable: false,
                        definition_id,
                    });
                }
                self.analyze_block(body);
                self.scopes.pop_scope();
                self.current_member_name = None;
                self.body_ctx = BodyContext::None;
            }
            Member::Lifecycle { kind, params, body, .. } => {
                self.body_ctx = BodyContext::Lifecycle;
                self.current_member_name = Some(lifecycle_name(*kind).to_string());
                self.scopes.push_scope();
                for p in params {
                    let ty = self.resolve_typeref(&p.ty);
                    let definition_id = self.record_nested_definition(
                        &p.name,
                        HirDefinitionKind::Parameter,
                        ty.clone(),
                        false,
                        p.name_span,
                    );
                    self.scopes.define(Symbol {
                        name: p.name.clone(), ty, kind: SymbolKind::Parameter, mutable: false,
                        definition_id,
                    });
                }
                self.analyze_block(body);
                self.scopes.pop_scope();
                self.current_member_name = None;
                self.body_ctx = BodyContext::None;
            }
            _ => {}
        }
    }

    fn analyze_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.analyze_stmt(stmt);
        }
    }

    fn analyze_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::ValDecl {
                name,
                name_span,
                ty,
                init,
                span,
            } => {
                let init_ty = self.analyze_expr(init);
                let declared_ty = if let Some(t) = ty {
                    let dt = self.resolve_typeref(t);
                    // Check type compatibility
                    if !init_ty.is_assignable_to(&dt) && !init_ty.is_error() {
                        self.diag.error("E020",
                            format!("Type mismatch. Expected '{}', found '{}'", dt.display_name(), init_ty.display_name()),
                            *span);
                    }
                    dt
                } else {
                    init_ty
                };
                let definition_id = self.record_nested_definition(
                    name,
                    HirDefinitionKind::Local,
                    declared_ty.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: declared_ty, kind: SymbolKind::Local, mutable: false,
                    definition_id,
                });
            }
            Stmt::VarDecl {
                name,
                name_span,
                ty,
                init,
                span,
            } => {
                let declared_ty = if let Some(t) = ty {
                    let dt = self.resolve_typeref(t);
                    if let Some(init_expr) = init {
                        let init_ty = self.analyze_expr(init_expr);
                        if !init_ty.is_assignable_to(&dt) && !init_ty.is_error() {
                            self.diag.error("E020",
                                format!("Type mismatch. Expected '{}', found '{}'", dt.display_name(), init_ty.display_name()),
                                *span);
                        }
                    }
                    dt
                } else if let Some(init_expr) = init {
                    self.analyze_expr(init_expr)
                } else {
                    self.diag.error("E022", "Variable without type annotation must have an initializer", *span);
                    MoonType::Error
                };
                let definition_id = self.record_nested_definition(
                    name,
                    HirDefinitionKind::Local,
                    declared_ty.clone(),
                    true,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: declared_ty, kind: SymbolKind::Local, mutable: true,
                    definition_id,
                });
            }
            Stmt::Assignment { target, value, .. } => {
                self.analyze_expr(target);
                self.analyze_expr(value);
                // Check that target is mutable
                if let Expr::Ident(name, ispan) = target {
                    if let Some(sym) = self.scopes.lookup(name) {
                        if !sym.mutable {
                            self.diag.error("E040",
                                format!("Cannot assign to immutable variable '{}'", name),
                                *ispan);
                        }
                        if sym.kind == SymbolKind::RequiredComponent {
                            self.diag.error("E041",
                                format!("Cannot assign to 'require' field '{}'. It is automatically initialized", name),
                                *ispan);
                        }
                    }
                }
            }
            Stmt::If { cond, then_block, else_branch, .. } => {
                self.analyze_expr(cond);
                self.scopes.push_scope();
                self.analyze_block(then_block);
                self.scopes.pop_scope();
                if let Some(eb) = else_branch {
                    match eb {
                        ElseBranch::ElseBlock(block) => {
                            self.scopes.push_scope();
                            self.analyze_block(block);
                            self.scopes.pop_scope();
                        }
                        ElseBranch::ElseIf(if_stmt) => {
                            self.analyze_stmt(if_stmt);
                        }
                    }
                }
            }
            Stmt::When { subject, branches, span } => {
                if let Some(subj) = subject {
                    let subj_ty = self.analyze_expr(subj);
                    // Exhaustiveness check for enums
                    if let MoonType::Enum(enum_name) = &subj_ty {
                        self.check_when_exhaustiveness(enum_name, branches, *span);
                    }
                }
                for branch in branches {
                    if let WhenPattern::Expression(expr) = &branch.pattern {
                        self.analyze_expr(expr);
                    }
                    match &branch.body {
                        WhenBody::Block(b) => self.analyze_block(b),
                        WhenBody::Expr(e) => { self.analyze_expr(e); }
                    }
                }
            }
            Stmt::For {
                var_name,
                name_span,
                iterable,
                body,
                span: _,
            } => {
                let iter_ty = self.analyze_expr(iterable);
                self.scopes.push_scope();
                // For range expressions, infer the element type
                let elem_ty = match &iter_ty {
                    // Range of ints → Int
                    _ => MoonType::External("var".into()), // simplified for v1
                };
                let definition_id = self.record_nested_definition(
                    var_name,
                    HirDefinitionKind::Local,
                    elem_ty.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: var_name.clone(), ty: elem_ty, kind: SymbolKind::Local, mutable: false,
                    definition_id,
                });
                self.loop_depth += 1;
                self.analyze_block(body);
                self.loop_depth -= 1;
                self.scopes.pop_scope();
            }
            Stmt::While { cond, body, .. } => {
                self.analyze_expr(cond);
                self.loop_depth += 1;
                self.scopes.push_scope();
                self.analyze_block(body);
                self.scopes.pop_scope();
                self.loop_depth -= 1;
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.analyze_expr(v);
                }
            }
            Stmt::Wait { form, span } => {
                if self.body_ctx != BodyContext::Coroutine {
                    self.diag.error("E032", "'wait' is only valid inside a coroutine declaration", *span);
                }
                match form {
                    WaitForm::Duration(expr) => { self.analyze_expr(expr); }
                    WaitForm::Until(expr) | WaitForm::While(expr) => { self.analyze_expr(expr); }
                    _ => {}
                }
            }
            Stmt::Start { call, .. } => {
                self.analyze_expr(call);
            }
            Stmt::Stop { target, .. } => {
                self.analyze_expr(target);
            }
            Stmt::StopAll { .. } => {}
            Stmt::Listen { event, body, .. } => {
                self.analyze_expr(event);
                self.scopes.push_scope();
                self.analyze_block(body);
                self.scopes.pop_scope();
            }
            Stmt::IntrinsicBlock { .. } => {
                // No analysis — raw C# is user's responsibility
            }
            Stmt::Break { span } => {
                if self.loop_depth == 0 {
                    self.diag.error("E031", "'break' is not allowed outside of a loop", *span);
                }
            }
            Stmt::Continue { span } => {
                if self.loop_depth == 0 {
                    self.diag.error("E031", "'continue' is not allowed outside of a loop", *span);
                }
            }
            Stmt::Expr { expr, .. } => {
                self.analyze_expr(expr);
            }
        }
    }

    fn analyze_expr(&mut self, expr: &Expr) -> MoonType {
        match expr {
            Expr::IntLit(_, _) => MoonType::Primitive(PrimitiveKind::Int),
            Expr::FloatLit(_, _) => MoonType::Primitive(PrimitiveKind::Float),
            Expr::DurationLit(_, _) => MoonType::Primitive(PrimitiveKind::Float),
            Expr::StringLit(_, _) => MoonType::Primitive(PrimitiveKind::String),
            Expr::StringInterp { parts, .. } => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.analyze_expr(e);
                    }
                }
                MoonType::Primitive(PrimitiveKind::String)
            }
            Expr::BoolLit(_, _) => MoonType::Primitive(PrimitiveKind::Bool),
            Expr::Null(_) => MoonType::Nullable(Box::new(MoonType::Error)),
            Expr::Ident(name, span) => {
                if let Some((ty, definition_id)) = self.lookup_symbol(name) {
                    self.record_reference(name.clone(), HirReferenceKind::Identifier, definition_id, None, *span);
                    ty
                } else if let Some(ty) = self.known_project_types.get(name).cloned() {
                    self.record_reference(
                        name.clone(),
                        HirReferenceKind::Identifier,
                        None,
                        Some(name.clone()),
                        *span,
                    );
                    ty
                } else {
                    // Don't error for Unity API names — we can't resolve them without
                    // a full Unity type database. For v1, unknown idents are External.
                    self.record_reference(name.clone(), HirReferenceKind::Identifier, None, None, *span);
                    MoonType::External(name.clone())
                }
            }
            Expr::This(_) => MoonType::External("this".into()),
            Expr::Binary { left, op, right, .. } => {
                let lt = self.analyze_expr(left);
                let rt = self.analyze_expr(right);
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        if lt.is_numeric() && rt.is_numeric() {
                            // Return wider type
                            if matches!(lt.non_null(), MoonType::Primitive(PrimitiveKind::Double))
                                || matches!(rt.non_null(), MoonType::Primitive(PrimitiveKind::Double))
                            {
                                MoonType::Primitive(PrimitiveKind::Double)
                            } else if matches!(lt.non_null(), MoonType::Primitive(PrimitiveKind::Float))
                                || matches!(rt.non_null(), MoonType::Primitive(PrimitiveKind::Float))
                            {
                                MoonType::Primitive(PrimitiveKind::Float)
                            } else {
                                MoonType::Primitive(PrimitiveKind::Int)
                            }
                        } else {
                            // Could be operator overload (Vector3 * Float etc.)
                            // For v1, return External
                            MoonType::External("var".into())
                        }
                    }
                    BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt
                    | BinOp::LtEq | BinOp::GtEq => {
                        MoonType::Primitive(PrimitiveKind::Bool)
                    }
                    BinOp::And | BinOp::Or => {
                        MoonType::Primitive(PrimitiveKind::Bool)
                    }
                }
            }
            Expr::Unary { op, operand, .. } => {
                let t = self.analyze_expr(operand);
                match op {
                    UnaryOp::Not => MoonType::Primitive(PrimitiveKind::Bool),
                    UnaryOp::Negate => t,
                }
            }
            Expr::MemberAccess {
                receiver,
                name,
                name_span,
                ..
            } => {
                let receiver_is_this = matches!(receiver.as_ref(), Expr::This(_));
                let receiver_ty = self.analyze_expr(receiver);
                if receiver_is_this {
                    if let Some((ty, definition_id)) = self.lookup_symbol(name) {
                        self.record_reference(name.clone(), HirReferenceKind::Member, definition_id, None, *name_span);
                        return ty;
                    }
                }
                let candidate = self.member_candidate_name(&receiver_ty, name);
                self.record_reference(name.clone(), HirReferenceKind::Member, None, candidate, *name_span);
                // Cannot fully resolve member types without Unity type DB
                MoonType::External(name.clone())
            }
            Expr::SafeCall {
                receiver,
                name,
                name_span,
                ..
            } => {
                let receiver_is_this = matches!(receiver.as_ref(), Expr::This(_));
                let receiver_ty = self.analyze_expr(receiver);
                if receiver_is_this {
                    if let Some((ty, definition_id)) = self.lookup_symbol(name) {
                        self.record_reference(name.clone(), HirReferenceKind::Member, definition_id, None, *name_span);
                        return ty.make_nullable();
                    }
                }
                let candidate = self.member_candidate_name(&receiver_ty, name);
                self.record_reference(name.clone(), HirReferenceKind::Member, None, candidate, *name_span);
                MoonType::External(name.clone()).make_nullable()
            }
            Expr::SafeMethodCall {
                receiver,
                name,
                name_span,
                args,
                ..
            } => {
                let receiver_is_this = matches!(receiver.as_ref(), Expr::This(_));
                let receiver_ty = self.analyze_expr(receiver);
                for arg in args {
                    self.analyze_expr(&arg.value);
                }
                if receiver_is_this {
                    if let Some((ty, definition_id)) = self.lookup_symbol(name) {
                        self.record_reference(name.clone(), HirReferenceKind::Call, definition_id, None, *name_span);
                        return ty.make_nullable();
                    }
                }
                let candidate = self.member_candidate_name(&receiver_ty, name);
                self.record_reference(name.clone(), HirReferenceKind::Call, None, candidate, *name_span);
                MoonType::External("var".into())
            }
            Expr::NonNullAssert { expr, span } => {
                let ty = self.analyze_expr(expr);
                if !ty.is_nullable() && !ty.is_error() {
                    self.diag.warning("W001",
                        format!("Unnecessary non-null assertion '!!' on non-null type '{}'", ty.display_name()),
                        *span);
                }
                ty.non_null().clone()
            }
            Expr::Elvis { left, right, .. } => {
                let lt = self.analyze_expr(left);
                self.analyze_expr(right);
                // Elvis returns non-null
                lt.non_null().clone()
            }
            Expr::Call {
                receiver,
                name,
                name_span,
                args,
                ..
            } => {
                if let Some(recv) = receiver {
                    let receiver_is_this = matches!(recv.as_ref(), Expr::This(_));
                    let receiver_ty = self.analyze_expr(recv);
                    if receiver_is_this {
                        if let Some((ty, definition_id)) = self.lookup_symbol(name) {
                            self.record_reference(name.clone(), HirReferenceKind::Call, definition_id, None, *name_span);
                            for arg in args {
                                self.analyze_expr(&arg.value);
                            }
                            return ty;
                        }
                    }
                    let candidate = self.member_candidate_name(&receiver_ty, name);
                    self.record_reference(name.clone(), HirReferenceKind::Call, None, candidate, *name_span);
                }
                for arg in args {
                    self.analyze_expr(&arg.value);
                }
                if receiver.is_none() {
                    if let Some((ty, definition_id)) = self.lookup_symbol(name) {
                        self.record_reference(name.clone(), HirReferenceKind::Call, definition_id, None, *name_span);
                        return ty;
                    }
                    self.record_reference(name.clone(), HirReferenceKind::Call, None, None, *name_span);
                }
                // Return type depends on the callee — for v1, return External
                MoonType::External("var".into())
            }
            Expr::IndexAccess { receiver, index, .. } => {
                self.analyze_expr(receiver);
                self.analyze_expr(index);
                MoonType::External("var".into())
            }
            Expr::Range { start, end, step, .. } => {
                self.analyze_expr(start);
                self.analyze_expr(end);
                if let Some(s) = step {
                    self.analyze_expr(s);
                }
                MoonType::External("Range".into())
            }
            Expr::Is { expr, .. } => {
                self.analyze_expr(expr);
                MoonType::Primitive(PrimitiveKind::Bool)
            }
            Expr::IfExpr { cond, then_block, else_block, .. } => {
                self.analyze_expr(cond);
                self.analyze_block(then_block);
                self.analyze_block(else_block);
                MoonType::External("var".into())
            }
            Expr::WhenExpr { subject, branches, .. } => {
                if let Some(s) = subject {
                    let subj_ty = self.analyze_expr(s);
                    if let MoonType::Enum(enum_name) = &subj_ty {
                        self.check_when_exhaustiveness(enum_name, branches, expr.span());
                    }
                }
                for b in branches {
                    if let WhenPattern::Expression(pattern_expr) = &b.pattern {
                        self.analyze_expr(pattern_expr);
                    }
                    match &b.body {
                        WhenBody::Expr(e) => { self.analyze_expr(e); }
                        WhenBody::Block(bl) => self.analyze_block(bl),
                    }
                }
                MoonType::External("var".into())
            }
            Expr::Lambda { body, .. } => {
                self.scopes.push_scope();
                self.analyze_block(body);
                self.scopes.pop_scope();
                MoonType::External("lambda".into())
            }
            Expr::IntrinsicExpr { .. } => MoonType::External("var".into()),
        }
    }

    // ── Validation helpers ────────────────────────────────────────

    fn check_duplicate_lifecycles(&mut self, members: &[Member]) {
        let mut seen = std::collections::HashMap::new();
        for m in members {
            if let Member::Lifecycle { kind, span, .. } = m {
                if seen.contains_key(kind) {
                    self.diag.error("E014",
                        format!("Duplicate lifecycle block '{:?}'", kind),
                        *span);
                } else {
                    seen.insert(*kind, *span);
                }
            }
        }
    }

    fn check_when_exhaustiveness(&mut self, enum_name: &str, branches: &[WhenBranch], span: Span) {
        let has_else = branches.iter().any(|b| matches!(b.pattern, WhenPattern::Else));
        if has_else {
            return; // else covers everything
        }

        if let Some(entries) = self.enum_entries.get(enum_name) {
            let covered: std::collections::HashSet<String> = branches.iter().filter_map(|b| {
                if let WhenPattern::Expression(Expr::MemberAccess { name, .. }) = &b.pattern {
                    Some(name.clone())
                } else if let WhenPattern::Expression(Expr::Ident(name, _)) = &b.pattern {
                    Some(name.clone())
                } else {
                    None
                }
            }).collect();

            let missing: Vec<&String> = entries.iter().filter(|e| !covered.contains(*e)).collect();
            if !missing.is_empty() {
                let missing_str: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
                self.diag.warning("W003",
                    format!("'when' statement does not cover all values of '{}'. Missing: {}", enum_name, missing_str.join(", ")),
                    span);
            }
        }
    }

    fn resolve_typeref(&mut self, ty: &TypeRef) -> MoonType {
        match ty {
            TypeRef::Simple { name, nullable, span } => {
                let base = self.resolve_named_typeref(name, *span);
                if *nullable { base.make_nullable() } else { base }
            }
            TypeRef::Generic {
                name,
                type_args,
                nullable,
                span,
            } => {
                self.record_named_type_reference(name, *span);
                let args: Vec<MoonType> = type_args.iter().map(|t| self.resolve_typeref(t)).collect();
                let base = MoonType::Generic(name.clone(), args);
                if *nullable { base.make_nullable() } else { base }
            }
            TypeRef::Qualified { qualifier, name, nullable, .. } => {
                let full = format!("{}.{}", qualifier, name);
                let base = MoonType::External(full);
                if *nullable { base.make_nullable() } else { base }
            }
        }
    }

    fn begin_hir(&mut self, file_path: &Path) {
        self.current_file_path = Some(file_path.to_path_buf());
        self.hir_definitions.clear();
        self.hir_references.clear();
        self.next_definition_id = 1;
        self.current_decl_name = None;
        self.current_member_name = None;
    }

    fn finish_hir(&mut self, file_path: &Path) -> HirFile {
        let definitions = std::mem::take(&mut self.hir_definitions);
        let references = std::mem::take(&mut self.hir_references);
        self.current_file_path = None;
        self.current_decl_name = None;
        self.current_member_name = None;

        HirFile {
            path: file_path.to_path_buf(),
            definitions,
            references,
        }
    }

    fn lookup_symbol(&self, name: &str) -> Option<(MoonType, Option<u32>)> {
        self.scopes
            .lookup(name)
            .map(|symbol| (symbol.ty.clone(), symbol.definition_id))
    }

    fn lookup_type_symbol(&self, name: &str) -> Option<(MoonType, Option<u32>)> {
        self.scopes.lookup(name).and_then(|symbol| {
            if symbol.kind == SymbolKind::Type {
                Some((symbol.ty.clone(), symbol.definition_id))
            } else {
                None
            }
        })
    }

    fn resolve_named_typeref(&mut self, name: &str, span: Span) -> MoonType {
        if let Some((ty, definition_id)) = self.lookup_type_symbol(name) {
            self.record_reference(
                name.to_string(),
                HirReferenceKind::Type,
                definition_id,
                None,
                span,
            );
            ty
        } else if let Some(ty) = self.known_project_types.get(name).cloned() {
            self.record_reference(
                name.to_string(),
                HirReferenceKind::Type,
                None,
                Some(name.to_string()),
                span,
            );
            ty
        } else {
            resolve_type_name(name)
        }
    }

    fn record_named_type_reference(&mut self, name: &str, span: Span) {
        if let Some((_, definition_id)) = self.lookup_type_symbol(name) {
            self.record_reference(
                name.to_string(),
                HirReferenceKind::Type,
                definition_id,
                None,
                span,
            );
        } else if self.known_project_types.contains_key(name) {
            self.record_reference(
                name.to_string(),
                HirReferenceKind::Type,
                None,
                Some(name.to_string()),
                span,
            );
        }
    }

    fn member_candidate_name(&self, receiver_ty: &MoonType, member_name: &str) -> Option<String> {
        match receiver_ty.non_null() {
            MoonType::Component(name)
            | MoonType::Asset(name)
            | MoonType::Class(name)
            | MoonType::Enum(name) => Some(format!("{}.{}", name, member_name)),
            _ => None,
        }
    }

    fn record_definition(
        &mut self,
        name: String,
        qualified_name: String,
        kind: HirDefinitionKind,
        ty: MoonType,
        mutable: bool,
        span: Span,
    ) -> Option<u32> {
        let file_path = self.current_file_path.clone()?;
        let definition_id = self.next_definition_id;
        self.next_definition_id += 1;
        self.hir_definitions.push(HirDefinition {
            id: definition_id,
            name,
            qualified_name,
            kind,
            ty,
            mutable,
            file_path,
            span,
        });
        Some(definition_id)
    }

    fn record_member_definition(
        &mut self,
        name: &str,
        kind: HirDefinitionKind,
        ty: MoonType,
        mutable: bool,
        span: Span,
    ) -> Option<u32> {
        let qualified_name = match &self.current_decl_name {
            Some(decl_name) => format!("{}.{}", decl_name, name),
            None => name.to_string(),
        };

        self.record_definition(name.to_string(), qualified_name, kind, ty, mutable, span)
    }

    fn record_nested_definition(
        &mut self,
        name: &str,
        kind: HirDefinitionKind,
        ty: MoonType,
        mutable: bool,
        span: Span,
    ) -> Option<u32> {
        let qualified_name = match (&self.current_decl_name, &self.current_member_name) {
            (Some(decl_name), Some(member_name)) => format!("{}.{}.{}", decl_name, member_name, name),
            (Some(decl_name), None) => format!("{}.{}", decl_name, name),
            _ => name.to_string(),
        };

        self.record_definition(name.to_string(), qualified_name, kind, ty, mutable, span)
    }

    fn record_reference(
        &mut self,
        name: String,
        kind: HirReferenceKind,
        resolved_definition_id: Option<u32>,
        candidate_qualified_name: Option<String>,
        span: Span,
    ) {
        let Some(file_path) = self.current_file_path.clone() else {
            return;
        };

        self.hir_references.push(HirReference {
            name,
            kind,
            resolved_definition_id,
            candidate_qualified_name,
            file_path,
            span,
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Diagnostic;
    use crate::lexer::lexer::Lexer;
    use crate::parser::parser::Parser;
    use crate::diagnostics::Severity;

    fn analyze(input: &str) -> Vec<Diagnostic> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let file = parser.parse_file();
        assert!(parser.errors().is_empty(), "Parse errors: {:?}", parser.errors());
        let mut analyzer = Analyzer::new();
        analyzer.analyze_file(&file);
        analyzer.diag.diagnostics
    }

    fn errors(input: &str) -> Vec<Diagnostic> {
        analyze(input).into_iter().filter(|d| d.severity == Severity::Error).collect()
    }

    fn warnings(input: &str) -> Vec<Diagnostic> {
        analyze(input).into_iter().filter(|d| d.severity == Severity::Warning).collect()
    }

    // === E012: Lifecycle in wrong context ===

    #[test]
    fn test_lifecycle_in_asset() {
        let diags = errors("asset Foo : ScriptableObject {\n  update {\n  }\n}");
        assert!(!diags.is_empty());
        assert!(diags[0].code == "E012");
    }

    #[test]
    fn test_lifecycle_in_component_ok() {
        let diags = errors("component Foo : MonoBehaviour {\n  update {\n  }\n}");
        assert!(diags.is_empty());
    }

    // === E013: require/optional in wrong context ===

    #[test]
    fn test_require_in_asset() {
        let diags = errors("asset Foo : ScriptableObject {\n  require rb: Rigidbody\n}");
        assert!(diags.iter().any(|d| d.code == "E013"));
    }

    #[test]
    fn test_require_in_class() {
        let diags = errors("class Foo {\n  require rb: Rigidbody\n}");
        assert!(diags.iter().any(|d| d.code == "E013"));
    }

    // === E014: Duplicate lifecycle ===

    #[test]
    fn test_duplicate_lifecycle() {
        let diags = errors("component Foo : MonoBehaviour {\n  update {\n  }\n  update {\n  }\n}");
        assert!(diags.iter().any(|d| d.code == "E014"));
    }

    // === E031: break/continue outside loop ===

    #[test]
    fn test_break_outside_loop() {
        let diags = errors("component Foo : MonoBehaviour {\n  func f() {\n    break\n  }\n}");
        assert!(diags.iter().any(|d| d.code == "E031"));
    }

    #[test]
    fn test_break_inside_loop_ok() {
        let diags = errors("component Foo : MonoBehaviour {\n  func f() {\n    while true {\n      break\n    }\n  }\n}");
        assert!(diags.is_empty());
    }

    // === E032: wait outside coroutine ===

    #[test]
    fn test_wait_outside_coroutine() {
        let diags = errors("component Foo : MonoBehaviour {\n  func f() {\n    wait 1.0s\n  }\n}");
        assert!(diags.iter().any(|d| d.code == "E032"));
    }

    #[test]
    fn test_wait_in_coroutine_ok() {
        let diags = errors("component Foo : MonoBehaviour {\n  coroutine blink() {\n    wait 1.0s\n  }\n}");
        assert!(diags.is_empty());
    }

    // === E040: Assign to val ===

    #[test]
    fn test_assign_to_val() {
        let diags = errors("component Foo : MonoBehaviour {\n  func f() {\n    val x = 5\n    x = 10\n  }\n}");
        assert!(diags.iter().any(|d| d.code == "E040"));
    }

    #[test]
    fn test_assign_to_var_ok() {
        let diags = errors("component Foo : MonoBehaviour {\n  func f() {\n    var x = 5\n    x = 10\n  }\n}");
        assert!(diags.is_empty());
    }

    // === E041: Assign to require field ===

    #[test]
    fn test_assign_to_require() {
        let diags = errors("component Foo : MonoBehaviour {\n  require rb: Rigidbody\n  func f() {\n    rb = null\n  }\n}");
        assert!(diags.iter().any(|d| d.code == "E041"));
    }

    // === E050: Empty enum ===

    #[test]
    fn test_empty_enum() {
        let diags = errors("enum Empty {\n}");
        assert!(diags.iter().any(|d| d.code == "E050"));
    }

    #[test]
    fn test_parameterized_enum_arg_mismatch() {
        let diags = errors("enum Weapon(val damage: Int, val range: Float) {\n  Sword(10)\n}");
        assert!(diags.iter().any(|d| d.code == "E051"));
    }

    // === E060: Coroutine in wrong context ===

    #[test]
    fn test_coroutine_in_asset() {
        let diags = errors("asset Foo : ScriptableObject {\n  coroutine blink() {\n    wait 1.0s\n  }\n}");
        assert!(diags.iter().any(|d| d.code == "E060"));
    }

    // === W001: Unnecessary !! ===

    #[test]
    fn test_unnecessary_non_null_assert() {
        let diags = warnings("component Foo : MonoBehaviour {\n  func f() {\n    val x = 5\n    val y = x!!\n  }\n}");
        // x is Int (non-null), so !! is unnecessary
        // Note: this may not trigger because x resolves to Int but we need
        // the analyzer to track that. Let's check...
        // Actually x will be Int from the literal. !! on Int should warn.
        assert!(diags.iter().any(|d| d.code == "W001"));
    }

    // === Full sample analysis ===

    #[test]
    fn test_player_controller_no_errors() {
        let src = r#"using UnityEngine

component PlayerController : MonoBehaviour {
    serialize speed: Float = 5.0
    serialize jumpForce: Float = 8.0

    require rb: Rigidbody
    optional animator: Animator

    update {
        val h = input.axis("Horizontal")
        val v = input.axis("Vertical")
    }

    func jump() {
        animator?.play("Jump")
    }
}"#;
        let diags = errors(src);
        assert!(diags.is_empty(), "Unexpected errors: {:?}", diags);
    }

    #[test]
    fn test_player_health_no_errors() {
        let src = r#"using UnityEngine

component PlayerHealth : MonoBehaviour {
    serialize maxHp: Int = 100
    var hp: Int = 100
    var invincible: Bool = false

    func damage(amount: Int) {
        if invincible { return }
        hp -= amount
        if hp <= 0 {
            die()
        }
    }

    coroutine hitInvincible() {
        invincible = true
        wait 1.0s
        invincible = false
    }

    func die() {
        gameObject.setActive(false)
    }
}"#;
        let diags = errors(src);
        assert!(diags.is_empty(), "Unexpected errors: {:?}", diags);
    }
}
