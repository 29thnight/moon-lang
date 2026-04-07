use crate::ast::*;
use crate::diagnostics::DiagnosticCollector;
use crate::hir::{
    HirDefinition, HirDefinitionKind, HirFile, HirListenLifetime, HirListenSite,
    HirPatternBinding, HirPatternBindingKind, HirReference, HirReferenceKind,
};
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
    /// Whether we are currently inside an `async func` body.
    /// Used by Phase 5 to validate `await` usage (E135) and track
    /// "async without await" (W025).
    in_async_fn: bool,
    /// Whether the current async function actually contains an `await` site.
    async_used_await: bool,
    loop_depth: u32,
    /// Known enum names → entries (for exhaustiveness checking)
    enum_entries: std::collections::HashMap<String, Vec<String>>,
    /// Enum name → (entry name → payload arity) for pattern binding validation
    enum_payloads: std::collections::HashMap<String, std::collections::HashMap<String, usize>>,
    /// Data class name → field names for destructure validation
    dataclass_fields: std::collections::HashMap<String, Vec<String>>,
    known_project_types: std::collections::HashMap<String, PrismType>,
    current_file_path: Option<PathBuf>,
    hir_definitions: Vec<HirDefinition>,
    hir_references: Vec<HirReference>,
    hir_pattern_bindings: Vec<HirPatternBinding>,
    hir_listen_sites: Vec<HirListenSite>,
    next_definition_id: u32,
    current_decl_name: Option<String>,
    current_member_name: Option<String>,
    /// Whether the `input-system` language feature is enabled for this project.
    input_system_enabled: bool,
    /// Language 5, Sprint 1: tracks whether the current iterator-bodied
    /// member has produced at least one value via `yield`. Used by W033
    /// (`coroutine declares Seq<T> but never yields`).
    coroutine_yielded_values: bool,
    /// Language 5, Sprint 1: the declared element type of the surrounding
    /// iterator body, if known. Set when entering a coroutine with an
    /// explicit return type or a func returning `Seq<T>`/`IEnumerator<T>`/
    /// `IEnumerable<T>`. Used by E148 to validate `yield expr`.
    coroutine_element_type: Option<PrismType>,
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer {
            diag: DiagnosticCollector::new(),
            scopes: ScopeStack::new(),
            decl_ctx: DeclContext::None,
            body_ctx: BodyContext::None,
            in_async_fn: false,
            async_used_await: false,
            loop_depth: 0,
            enum_entries: std::collections::HashMap::new(),
            enum_payloads: std::collections::HashMap::new(),
            dataclass_fields: std::collections::HashMap::new(),
            known_project_types: std::collections::HashMap::new(),
            current_file_path: None,
            hir_definitions: Vec::new(),
            hir_references: Vec::new(),
            hir_pattern_bindings: Vec::new(),
            hir_listen_sites: Vec::new(),
            next_definition_id: 1,
            current_decl_name: None,
            current_member_name: None,
            input_system_enabled: false,
            coroutine_yielded_values: false,
            coroutine_element_type: None,
        }
    }

    pub fn with_input_system_enabled(mut self, enabled: bool) -> Self {
        self.input_system_enabled = enabled;
        self
    }

    pub fn with_known_project_types(
        known_project_types: std::collections::HashMap<String, PrismType>,
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

    /// Issue #12: validate that a `require` / `optional` / `child` /
    /// `parent` field references a Unity Component subtype. The full
    /// subtype check requires the Roslyn sidecar's type registry; as a
    /// fallback we maintain a small blacklist of well-known non-Component
    /// types (primitives, collections, function types) that the
    /// component-lookup qualifiers should never apply to. The diagnostic
    /// guides users to a regular `val name: Type? = null` field instead.
    fn check_component_lookup_type(
        &mut self,
        qualifier: &str,
        field_name: &str,
        ty: &TypeRef,
        name_span: Span,
    ) {
        let type_name: &str = match ty {
            TypeRef::Simple { name, .. } => name,
            TypeRef::Generic { name, .. } => name,
            _ => return,
        };
        if is_known_non_component_type(type_name) {
            self.diag.error(
                "E191",
                format!(
                    "'{}' field '{}' must reference a UnityEngine.Component subtype, but '{}' is not a Component. Use a regular `val {}: {}? = null` field instead.",
                    qualifier, field_name, type_name, field_name, type_name
                ),
                name_span,
            );
        }
    }

    pub fn analyze_file_with_hir(&mut self, file: &File, file_path: &Path) -> HirFile {
        self.begin_hir(file_path);
        self.analyze_file(file);
        self.finish_hir(file_path)
    }

    fn register_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Component { name, name_span, .. } => {
                let ty = PrismType::Component(name.clone());
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
                let ty = PrismType::Asset(name.clone());
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
                let ty = PrismType::Class(name.clone());
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
                let ty = PrismType::Class(name.clone());
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
                let ty = PrismType::Enum(name.clone());
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
                let ty = PrismType::Class(name.clone());
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
            Decl::Interface { name, name_span, .. } => {
                let ty = PrismType::External(name.clone());
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
            Decl::TypeAlias { name, name_span, target, .. } => {
                // Register type alias as a type synonym
                let target_ty = self.resolve_typeref(target);
                let definition_id = self.record_definition(
                    name.clone(),
                    name.clone(),
                    HirDefinitionKind::Type,
                    target_ty.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty: target_ty,
                    kind: SymbolKind::Type,
                    mutable: false,
                    definition_id,
                });
            }
            Decl::Struct { name, name_span, .. } => {
                let ty = PrismType::Class(name.clone());
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
            Decl::Extension { .. } => {
                // Extension blocks don't register a new type
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
                // SOLID analysis warnings (Language 3)
                self.check_solid_warnings(name, members, decl);
                // Language 5, Sprint 3: W031 — `bind` member never read.
                // After member analysis the HIR reference set carries every
                // observed identifier; if a bind name has no references
                // (and no `bind to` site) we surface the warning.
                self.check_unused_bind_members(members);

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
                // Store field names for destructure pattern validation.
                let field_names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
                self.dataclass_fields.insert(name.clone(), field_names);
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
                        PrismType::Enum(name.clone()),
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
                // Store payload arity per entry for pattern binding validation.
                let mut payloads = std::collections::HashMap::new();
                for entry in entries {
                    payloads.insert(entry.name.clone(), params.len());
                }
                self.enum_payloads.insert(name.clone(), payloads);
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
            Decl::Interface { .. } => {
                // Interface members are signatures only — no body analysis needed.
            }
            Decl::TypeAlias { .. } => {
                // Type aliases are resolved during registration; nothing to analyze.
            }
            Decl::Extension { members, .. } => {
                // Extension blocks: analyze members in a fresh scope
                self.scopes.push_scope();
                self.analyze_members(members);
                self.scopes.pop_scope();
            }
            Decl::Struct { name, fields, members, span, .. } => {
                self.current_decl_name = Some(name.clone());
                if fields.is_empty() {
                    self.diag.warning("W005", "Struct has no fields", *span);
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
                if !members.is_empty() {
                    self.scopes.push_scope();
                    self.analyze_members(members);
                    self.scopes.pop_scope();
                }
                self.current_decl_name = None;
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
                let btype = ty.as_ref().map(|t| self.resolve_typeref(t)).unwrap_or(PrismType::Error);
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
                self.check_component_lookup_type("require", name, ty, *name_span);
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
                self.check_component_lookup_type("optional", name, ty, *name_span);
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
                self.check_component_lookup_type("child", name, ty, *name_span);
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
                self.check_component_lookup_type("parent", name, ty, *name_span);
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
                is_operator,
                ..
            } => {
                // Issue #23: reserved built-in sugar names (`get`, `find`)
                // cannot be used as user function names because they map
                // to the Unity API GetComponent / FindFirstObjectByType
                // sugar. The check must be skipped for `operator get` and
                // `operator set` declarations, which are the lang-4
                // indexer syntax and lower to a C# `this[...]` member
                // (no name collision with the sugar).
                const RESERVED_SUGAR_NAMES: &[&str] = &["get", "find"];
                if !*is_operator && RESERVED_SUGAR_NAMES.contains(&name.as_str()) {
                    self.diag.error(
                        "E101",
                        format!("'{}' is a reserved built-in method name (maps to Unity API). Choose a different name.", name),
                        *name_span,
                    );
                }
                let ret = return_ty.as_ref().map(|t| self.resolve_typeref(t)).unwrap_or(PrismType::Unit);
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
                    PrismType::Unit,
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: PrismType::Unit, kind: SymbolKind::Coroutine, mutable: false,
                    definition_id,
                });
            }
            Member::Lifecycle { kind, span, .. } => {
                let _ = self.record_member_definition(
                    lifecycle_name(*kind),
                    HirDefinitionKind::Lifecycle,
                    PrismType::Unit,
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
                let ret = return_ty.as_ref().map(|t| self.resolve_typeref(t)).unwrap_or(PrismType::Unit);
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
                    PrismType::Unit,
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: PrismType::Unit, kind: SymbolKind::Coroutine, mutable: false,
                    definition_id,
                });
            }
            Member::Pool {
                name,
                name_span,
                item_type,
                ..
            } => {
                let btype = self.resolve_typeref(item_type);
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Field,
                    btype.clone(),
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::Field, mutable: false,
                    definition_id,
                });
            }
            Member::Property {
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
                    name: name.clone(), ty: btype, kind: SymbolKind::Field, mutable: false,
                    definition_id,
                });
            }
            Member::Event {
                name,
                name_span,
                ty,
                ..
            } => {
                // event field is a delegate variable; resolved as External and
                // registered in scope so `name += handler` and `name(args)` work.
                let btype = self.resolve_typeref(ty);
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Field,
                    btype.clone(),
                    true,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::Field, mutable: true,
                    definition_id,
                });
            }
            Member::StateMachine { name, name_span, .. } => {
                // A state machine introduces a named field of enum type that
                // lives on the owning component; expose it as a Field symbol.
                let btype = PrismType::External(format!("{}State", ascii_capitalize(name)));
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Field,
                    btype.clone(),
                    true,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::Field, mutable: true,
                    definition_id,
                });
            }
            Member::Command { name, name_span, .. } => {
                // A command lowers to a nested class + helper method; expose the
                // name so the user can reference it as a callable symbol.
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Function,
                    PrismType::Unit,
                    false,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: PrismType::Unit, kind: SymbolKind::Function, mutable: false,
                    definition_id,
                });
            }
            Member::BindProperty { name, name_span, ty, .. } => {
                let btype = self.resolve_typeref(ty);
                let definition_id = self.record_member_definition(
                    name,
                    HirDefinitionKind::Field,
                    btype.clone(),
                    true,
                    *name_span,
                );
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::Field, mutable: true,
                    definition_id,
                });
            }
            // v5: nested declaration. Recursively register the nested
            // declaration so its enclosed types/members are visible to
            // the rest of the analyzer.
            Member::NestedDecl { decl, .. } => {
                self.register_decl(decl);
            }
        }
    }

    fn analyze_member_body(&mut self, member: &Member) {
        match member {
            Member::Func { name, params, body, return_ty, is_async, span, .. } => {
                // Language 5, Sprint 1: a func with an iterator-shaped return
                // type (`Seq<T>`, `IEnumerator(<T>)`, `IEnumerable(<T>)`) is
                // promoted to a Coroutine context so `yield expr` / `yield break`
                // is permitted in its body. The element type is captured for
                // E148.
                let iter_elem = return_ty
                    .as_ref()
                    .and_then(|t| iterator_element_type(&self.resolve_typeref(t)));
                let is_iter_func = iter_elem.is_some()
                    || return_ty.as_ref().map(|t| is_non_generic_iterator(&self.resolve_typeref(t))).unwrap_or(false);
                self.body_ctx = if is_iter_func {
                    BodyContext::Coroutine
                } else {
                    BodyContext::Function
                };
                let prev_yielded = std::mem::replace(&mut self.coroutine_yielded_values, false);
                let prev_elem = std::mem::replace(&mut self.coroutine_element_type, iter_elem.clone());
                self.current_member_name = Some(name.clone());
                let prev_in_async = self.in_async_fn;
                let prev_used_await = self.async_used_await;
                if *is_async {
                    self.in_async_fn = true;
                    self.async_used_await = false;
                }
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
                    FuncBody::ExprBody(expr) => {
                        let expected_return_ty = return_ty.as_ref().map(|ty| self.resolve_typeref(ty));
                        self.analyze_expr_with_expected(expr, expected_return_ty.as_ref());
                    }
                }
                self.scopes.pop_scope();
                if *is_async && !self.async_used_await {
                    self.diag.warning(
                        "W025",
                        format!("async func '{}' never awaits — consider removing 'async'", name),
                        *span,
                    );
                }
                // W033 — iterator declared a typed element but never produced one.
                if is_iter_func && self.coroutine_element_type.is_some() && !self.coroutine_yielded_values {
                    self.diag.warning(
                        "W033",
                        format!("iterator function '{}' declares an element type but never yields a value", name),
                        *span,
                    );
                }
                self.coroutine_yielded_values = prev_yielded;
                self.coroutine_element_type = prev_elem;
                self.in_async_fn = prev_in_async;
                self.async_used_await = prev_used_await;
                self.current_member_name = None;
                self.body_ctx = BodyContext::None;
            }
            Member::Coroutine { name, params, body, return_ty, span, .. } => {
                self.body_ctx = BodyContext::Coroutine;
                let elem_ty = return_ty
                    .as_ref()
                    .and_then(|t| iterator_element_type(&self.resolve_typeref(t)));
                let prev_yielded = std::mem::replace(&mut self.coroutine_yielded_values, false);
                let prev_elem = std::mem::replace(&mut self.coroutine_element_type, elem_ty.clone());
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
                if elem_ty.is_some() && !self.coroutine_yielded_values {
                    self.diag.warning(
                        "W033",
                        format!("coroutine '{}' declares an element type but never yields a value", name),
                        *span,
                    );
                }
                self.coroutine_yielded_values = prev_yielded;
                self.coroutine_element_type = prev_elem;
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
            Member::StateMachine { name, states, .. } => {
                // Validate: E140 — every transition target must be a declared state.
                // E141 — no duplicate state names in the same machine.
                let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
                for s in states {
                    if !seen.insert(&s.name) {
                        self.diag.error(
                            "E141",
                            format!("Duplicate state '{}' in state machine '{}'", s.name, name),
                            s.name_span,
                        );
                    }
                }
                let valid_states: std::collections::HashSet<&str> =
                    states.iter().map(|s| s.name.as_str()).collect();
                for s in states {
                    for tr in &s.transitions {
                        if !valid_states.contains(tr.target.as_str()) {
                            self.diag.error(
                                "E140",
                                format!(
                                    "Transition to undeclared state '{}' in state machine '{}'",
                                    tr.target, name
                                ),
                                tr.target_span,
                            );
                        }
                    }
                }
                // Analyze enter/exit blocks so variable references inside them
                // are validated like normal statement bodies.
                for s in states {
                    if let Some(enter) = &s.enter {
                        self.body_ctx = BodyContext::Function;
                        self.scopes.push_scope();
                        self.analyze_block(enter);
                        self.scopes.pop_scope();
                        self.body_ctx = BodyContext::None;
                    }
                    if let Some(exit) = &s.exit {
                        self.body_ctx = BodyContext::Function;
                        self.scopes.push_scope();
                        self.analyze_block(exit);
                        self.scopes.pop_scope();
                        self.body_ctx = BodyContext::None;
                    }
                }
            }
            Member::Command { params, execute, undo, can_execute, .. } => {
                self.body_ctx = BodyContext::Function;
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
                self.analyze_block(execute);
                if let Some(undo_block) = undo {
                    self.analyze_block(undo_block);
                }
                if let Some(ce) = can_execute {
                    let _ = self.analyze_expr(ce);
                }
                self.scopes.pop_scope();
                self.body_ctx = BodyContext::None;
            }
            Member::BindProperty { init, .. } => {
                if let Some(expr) = init {
                    let _ = self.analyze_expr(expr);
                }
            }
            // v5: nested declaration — recurse into the inner decl so
            // its body is analyzed under the surrounding scope.
            Member::NestedDecl { decl, .. } => {
                self.analyze_decl(decl);
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
                is_ref,
                span,
                ..
            } => {
                // Issue #3: `val ref` requires an explicit type annotation
                // because C# does not support `ref readonly var`. The
                // existing lowering at lower.rs:2068 emits the invalid
                // `ref readonly var` form when no type is supplied. Reject
                // the unannotated case here so the user receives a clear
                // diagnostic instead of a downstream C# compile failure.
                if *is_ref && ty.is_none() {
                    self.diag.error("E190",
                        format!("'val ref' local '{}' requires an explicit type annotation. C# does not support 'ref readonly var'; write 'val ref {}: SomeType = ref expr' instead.", name, name),
                        *span);
                }
                let declared_ty = if let Some(t) = ty {
                    let dt = self.resolve_typeref(t);
                    let init_ty = self.analyze_expr_with_expected(init, Some(&dt));
                    // Check type compatibility. ref locals trust the
                    // explicit annotation because the right-hand side is
                    // a `Expr::RefOf` whose inner Unity API type is not
                    // always inferable from the current symbol table.
                    if !*is_ref && !init_ty.is_assignable_to(&dt) && !init_ty.is_error() {
                        self.diag.error("E020",
                            format!("Type mismatch. Expected '{}', found '{}'", dt.display_name(), init_ty.display_name()),
                            *span);
                    }
                    dt
                } else {
                    self.check_collection_literal_unannotated(init);
                    self.analyze_expr(init)
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
                is_ref,
                span,
                ..
            } => {
                // Issue #3: `var ref` requires an explicit type annotation
                // because C# does not support `ref var`. Mirror the
                // `val ref` check above.
                if *is_ref && ty.is_none() {
                    self.diag.error("E190",
                        format!("'var ref' local '{}' requires an explicit type annotation. C# does not support 'ref var'; write 'var ref {}: SomeType = ref expr' instead.", name, name),
                        *span);
                }
                let declared_ty = if let Some(t) = ty {
                    let dt = self.resolve_typeref(t);
                    if let Some(init_expr) = init {
                        let init_ty = self.analyze_expr_with_expected(init_expr, Some(&dt));
                        // Same trust-the-annotation policy as `val ref`.
                        if !*is_ref && !init_ty.is_assignable_to(&dt) && !init_ty.is_error() {
                            self.diag.error("E020",
                                format!("Type mismatch. Expected '{}', found '{}'", dt.display_name(), init_ty.display_name()),
                                *span);
                        }
                    }
                    dt
                } else if let Some(init_expr) = init {
                    self.check_collection_literal_unannotated(init_expr);
                    self.analyze_expr(init_expr)
                } else {
                    self.diag.error("E022", "Variable without type annotation must have an initializer", *span);
                    PrismType::Error
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
                    if let PrismType::Enum(enum_name) = &subj_ty {
                        self.check_when_exhaustiveness(enum_name, branches, *span);
                    }
                }
                for branch in branches {
                    if let WhenPattern::Binding { path, bindings, span } = &branch.pattern {
                        self.record_pattern_binding(
                            HirPatternBindingKind::When,
                            path.join("."),
                            bindings.clone(),
                            branch.guard.is_some(),
                            *span,
                        );
                    }
                    if let WhenPattern::Expression(expr) = &branch.pattern {
                        self.analyze_expr(expr);
                    }
                    if let Some(guard) = &branch.guard {
                        self.analyze_expr(guard);
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
                for_pattern,
                iterable,
                body,
                ..
            } => {
                let _iter_ty = self.analyze_expr(iterable);
                if let Some(pattern) = for_pattern {
                    self.record_pattern_binding(
                        HirPatternBindingKind::ForDestructure,
                        pattern.type_name.clone(),
                        pattern.bindings.clone(),
                        false,
                        pattern.span,
                    );
                }
                self.scopes.push_scope();
                // Issue #7: infer the induction variable type from the
                // iterable's static shape. For a `Range` expression
                // (`a until b`, `a..b`, `a downTo b`) the element type
                // is the type of the bounds (`Int` for `0 until 5`,
                // `Float` for `0.0..1.0`). Other iterables fall back to
                // `var` until full collection-type inference lands.
                // The previous fall-through to `var` produced a false
                // positive E148 inside `for i in 0 until 5 { yield i }`.
                let elem_ty = match iterable {
                    Expr::Range { start, .. } => {
                        let t = self.analyze_expr(start);
                        if t.is_error() {
                            PrismType::External("var".into())
                        } else {
                            t
                        }
                    }
                    _ => PrismType::External("var".into()),
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
            Stmt::DestructureVal { pattern, init, .. } => {
                self.record_pattern_binding(
                    HirPatternBindingKind::ValDestructure,
                    pattern.type_name.clone(),
                    pattern.bindings.clone(),
                    false,
                    pattern.span,
                );
                self.analyze_expr(init);
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
            Stmt::Listen { event, lifetime, bound_name, body, span, .. } => {
                // Validate that lifecycle-bound listen (until disable/destroy) is only
                // used inside a component. In a class or asset, OnDisable/OnDestroy
                // are not available, so the cleanup code would be invalid.
                if matches!(lifetime, ListenLifetime::UntilDisable | ListenLifetime::UntilDestroy | ListenLifetime::Manual)
                    && self.decl_ctx != DeclContext::Component
                {
                    let keyword = match lifetime {
                        ListenLifetime::UntilDisable => "until disable",
                        ListenLifetime::UntilDestroy => "until destroy",
                        ListenLifetime::Manual => "manual",
                        _ => "",
                    };
                    self.diag.error(
                        "E083",
                        format!("'listen {} {{ }}' is only valid inside a component declaration", keyword),
                        *span,
                    );
                }
                self.record_listen_site(lifetime.clone(), bound_name.clone(), *span);
                self.analyze_expr(event);
                self.scopes.push_scope();
                self.analyze_block(body);
                self.scopes.pop_scope();
            }
            Stmt::Unlisten { .. } => {
                // No semantic analysis needed for unlisten — resolved during lowering
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
            Stmt::Try { try_block, catches, finally_block, .. } => {
                self.scopes.push_scope();
                self.analyze_block(try_block);
                self.scopes.pop_scope();
                for catch in catches {
                    self.scopes.push_scope();
                    let catch_ty = self.resolve_typeref(&catch.ty);
                    self.scopes.define(Symbol {
                        name: catch.name.clone(),
                        ty: catch_ty,
                        kind: SymbolKind::Local,
                        mutable: false,
                        definition_id: None,
                    });
                    self.analyze_block(&catch.body);
                    self.scopes.pop_scope();
                }
                if let Some(finally) = finally_block {
                    self.scopes.push_scope();
                    self.analyze_block(finally);
                    self.scopes.pop_scope();
                }
            }
            Stmt::Throw { expr, .. } => {
                self.analyze_expr(expr);
            }
            Stmt::Use { name, name_span, ty, init, body, .. } => {
                // The bound name is in scope for the body (block form) or for
                // the rest of the enclosing scope (declaration form).
                let declared_ty = if let Some(t) = ty {
                    let dt = self.resolve_typeref(t);
                    self.analyze_expr_with_expected(init, Some(&dt));
                    dt
                } else {
                    self.analyze_expr(init)
                };
                let definition_id = self.record_nested_definition(
                    name,
                    HirDefinitionKind::Local,
                    declared_ty.clone(),
                    false,
                    *name_span,
                );
                if let Some(block) = body {
                    self.scopes.push_scope();
                    self.scopes.define(Symbol {
                        name: name.clone(), ty: declared_ty, kind: SymbolKind::Local, mutable: false,
                        definition_id,
                    });
                    self.analyze_block(block);
                    self.scopes.pop_scope();
                } else {
                    self.scopes.define(Symbol {
                        name: name.clone(), ty: declared_ty, kind: SymbolKind::Local, mutable: false,
                        definition_id,
                    });
                }
            }
            Stmt::BindTo { source, source_span, target, span } => {
                // E143: the target expression must be assignable (MemberAccess or Ident).
                // We perform a lightweight check rather than full lvalue analysis.
                let source_ty = if let Some((ty, _)) = self.lookup_symbol(source) {
                    ty
                } else {
                    self.diag.error(
                        "E016",
                        format!("'{}' is not defined", source),
                        *source_span,
                    );
                    PrismType::Error
                };
                let target_ty = self.analyze_expr(target);
                let is_writable = matches!(
                    target,
                    Expr::MemberAccess { .. } | Expr::Ident(_, _) | Expr::IndexAccess { .. }
                );
                if !is_writable {
                    self.diag.error(
                        "E143",
                        "'bind ... to' target must be a writable member or variable",
                        *span,
                    );
                }
                // E144: silently skip if either side is Error to avoid cascades.
                //
                // Issue #22: also skip when the target type analyzes as
                // a bare `External(member_name)`. PrSM does not have a
                // Unity type registry, so `Expr::MemberAccess` lowers
                // to `External(name)` where `name` is the member name
                // itself (`text`, `value`, ...) rather than the actual
                // member type. The previous strict check produced a
                // confusing E144 ("source is String but target is text")
                // for the canonical `bind playerName to nameLabel.text`
                // pattern. Trust the user's bind site when the target
                // type is unknown — the C# compiler will surface a real
                // type mismatch downstream if the binding is wrong.
                let target_is_unknown_member = matches!(
                    (&target, &target_ty),
                    (Expr::MemberAccess { .. }, PrismType::External(_))
                );
                if !matches!(source_ty, PrismType::Error)
                    && !matches!(target_ty, PrismType::Error)
                    && !target_is_unknown_member
                    && !source_ty.is_assignable_to(&target_ty)
                    && !target_ty.is_assignable_to(&source_ty)
                {
                    self.diag.error(
                        "E144",
                        format!(
                            "bind type mismatch: source is {} but target is {}",
                            source_ty.display_name(),
                            target_ty.display_name()
                        ),
                        *span,
                    );
                }
            }
            // ── Language 5, Sprint 1: yield statements ───────────
            Stmt::Yield { value, span } => {
                // E147: `yield` is only valid inside a coroutine declaration
                // or a func returning Seq<T>/IEnumerator(<T>)/IEnumerable(<T>).
                // The body context is set to Coroutine for both cases (the
                // analyze_decl path is responsible for entering that context
                // when a func has an iterator return type).
                if self.body_ctx != BodyContext::Coroutine {
                    self.diag.error(
                        "E147",
                        "'yield' is only valid inside a coroutine or iterator-returning function",
                        *span,
                    );
                }
                // Track that the current iterator body actually yielded a value.
                self.coroutine_yielded_values = true;
                let value_ty = self.analyze_expr(value);
                // E148: when an element type is declared (Seq<T>, IEnumerator<T>,
                // IEnumerable<T>), the yielded value must be assignable to T.
                if let Some(elem_ty) = self.coroutine_element_type.clone() {
                    if !value_ty.is_assignable_to(&elem_ty) && !value_ty.is_error() && !elem_ty.is_error() {
                        self.diag.error(
                            "E148",
                            format!(
                                "yield value type '{}' does not match declared element type '{}'",
                                value_ty.display_name(),
                                elem_ty.display_name()
                            ),
                            *span,
                        );
                    }
                }
            }
            Stmt::YieldBreak { span } => {
                if self.body_ctx != BodyContext::Coroutine {
                    self.diag.error(
                        "E147",
                        "'yield break' is only valid inside a coroutine or iterator-returning function",
                        *span,
                    );
                }
            }
            // ── Language 5, Sprint 1: preprocessor block ─────────
            Stmt::Preprocessor { arms, else_arm, .. } => {
                // Walk every branch — diagnostics from inactive arms are still
                // surfaced because PrSM does not evaluate the conditions at
                // compile time. We do, however, warn (W034) on unknown
                // symbols inside the conditions.
                for arm in arms {
                    self.check_preprocessor_cond(&arm.cond);
                    for s in &arm.body {
                        self.analyze_stmt(s);
                    }
                }
                if let Some(else_stmts) = else_arm {
                    for s in else_stmts {
                        self.analyze_stmt(s);
                    }
                }
            }
        }
    }

    /// Language 5, Sprint 3: emit W031 for any `bind` member that is never
    /// referenced from the rest of the component. The check uses the
    /// recorded HIR references — both regular identifier reads and the
    /// `bind to` source string — so a member is considered "read" if it
    /// appears in any expression position (including `bind to source`).
    fn check_unused_bind_members(&mut self, members: &[Member]) {
        // Collect every name in the recorded HIR reference set.
        let mut referenced: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for r in &self.hir_references {
            referenced.insert(r.name.as_str());
        }
        // Also count `bind to source` statements as a use of `source`.
        for m in members {
            if let Member::Lifecycle { body, .. } = m {
                collect_bind_to_sources(&body.stmts, &mut referenced);
            }
            if let Member::Func { body: FuncBody::Block(block), .. } = m {
                collect_bind_to_sources(&block.stmts, &mut referenced);
            }
            if let Member::Coroutine { body, .. } = m {
                collect_bind_to_sources(&body.stmts, &mut referenced);
            }
        }
        for m in members {
            if let Member::BindProperty { name, name_span, .. } = m {
                if !referenced.contains(name.as_str()) {
                    self.diag.warning(
                        "W031",
                        format!("bind property '{}' is never read", name),
                        *name_span,
                    );
                }
            }
        }
    }

    /// Walk a preprocessor condition tree and emit W034 for any unknown
    /// symbols (symbols that are neither curated PrSM aliases nor in the
    /// recognized set of common Unity defines). Unknown symbols still
    /// pass through verbatim — the warning is informational.
    fn check_preprocessor_cond(&mut self, cond: &PreprocessorCond) {
        match cond {
            PreprocessorCond::Symbol { name, span } => {
                if !is_known_preprocessor_symbol(name) {
                    self.diag.warning(
                        "W034",
                        format!(
                            "unknown preprocessor symbol '{}' — passes through verbatim and may not exist in target",
                            name
                        ),
                        *span,
                    );
                }
            }
            PreprocessorCond::Not { inner, .. } => self.check_preprocessor_cond(inner),
            PreprocessorCond::And { left, right, .. } | PreprocessorCond::Or { left, right, .. } => {
                self.check_preprocessor_cond(left);
                self.check_preprocessor_cond(right);
            }
        }
    }

    fn analyze_expr(&mut self, expr: &Expr) -> PrismType {
        match expr {
            Expr::IntLit(_, _) => PrismType::Primitive(PrimitiveKind::Int),
            Expr::FloatLit(_, _) => PrismType::Primitive(PrimitiveKind::Float),
            Expr::DurationLit(_, _) => PrismType::Primitive(PrimitiveKind::Float),
            Expr::StringLit(_, _) => PrismType::Primitive(PrimitiveKind::String),
            Expr::StringInterp { parts, .. } => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.analyze_expr(e);
                    }
                }
                PrismType::Primitive(PrimitiveKind::String)
            }
            Expr::BoolLit(_, _) => PrismType::Primitive(PrimitiveKind::Bool),
            Expr::Null(_) => PrismType::Nullable(Box::new(PrismType::Error)),
            Expr::Ident(name, span) => {
                if let Some((ty, definition_id)) = self.lookup_symbol(name) {
                    self.record_reference(name.clone(), HirReferenceKind::Identifier, definition_id, None, None, *span);
                    ty
                } else if let Some(ty) = self.known_project_types.get(name).cloned() {
                    self.record_reference(
                        name.clone(),
                        HirReferenceKind::Identifier,
                        None,
                        Some(name.clone()),
                        None,
                        *span,
                    );
                    ty
                } else {
                    // Don't error for Unity API names — we can't resolve them without
                    // a full Unity type database. For v1, unknown idents are External.
                    self.record_reference(name.clone(), HirReferenceKind::Identifier, None, None, None, *span);
                    PrismType::External(name.clone())
                }
            }
            Expr::This(_) => PrismType::External("this".into()),
            Expr::Binary { left, op, right, .. } => {
                let lt = self.analyze_expr(left);
                let rt = self.analyze_expr(right);
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        if lt.is_numeric() && rt.is_numeric() {
                            // Return wider type
                            if matches!(lt.non_null(), PrismType::Primitive(PrimitiveKind::Double))
                                || matches!(rt.non_null(), PrismType::Primitive(PrimitiveKind::Double))
                            {
                                PrismType::Primitive(PrimitiveKind::Double)
                            } else if matches!(lt.non_null(), PrismType::Primitive(PrimitiveKind::Float))
                                || matches!(rt.non_null(), PrismType::Primitive(PrimitiveKind::Float))
                            {
                                PrismType::Primitive(PrimitiveKind::Float)
                            } else {
                                PrismType::Primitive(PrimitiveKind::Int)
                            }
                        } else {
                            // Could be operator overload (Vector3 * Float etc.)
                            // For v1, return External
                            PrismType::External("var".into())
                        }
                    }
                    BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt
                    | BinOp::LtEq | BinOp::GtEq => {
                        PrismType::Primitive(PrimitiveKind::Bool)
                    }
                    BinOp::And | BinOp::Or | BinOp::In => {
                        PrismType::Primitive(PrimitiveKind::Bool)
                    }
                }
            }
            Expr::Unary { op, operand, .. } => {
                let t = self.analyze_expr(operand);
                match op {
                    UnaryOp::Not => PrismType::Primitive(PrimitiveKind::Bool),
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
                        self.record_reference(name.clone(), HirReferenceKind::Member, definition_id, None, None, *name_span);
                        return ty;
                    }
                }
                let candidate = self.member_candidate_name(&receiver_ty, name);
                self.record_reference(name.clone(), HirReferenceKind::Member, None, candidate, None, *name_span);
                // Cannot fully resolve member types without Unity type DB
                PrismType::External(name.clone())
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
                        self.record_reference(name.clone(), HirReferenceKind::Member, definition_id, None, None, *name_span);
                        return ty.make_nullable();
                    }
                }
                let candidate = self.member_candidate_name(&receiver_ty, name);
                self.record_reference(name.clone(), HirReferenceKind::Member, None, candidate, None, *name_span);
                PrismType::External(name.clone()).make_nullable()
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
                        self.record_reference(name.clone(), HirReferenceKind::Call, definition_id, None, None, *name_span);
                        return ty.make_nullable();
                    }
                }
                let candidate = self.member_candidate_name(&receiver_ty, name);
                self.record_reference(name.clone(), HirReferenceKind::Call, None, candidate, None, *name_span);
                PrismType::External("var".into())
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
                    // v2 feature gate: `input.action("X")` requires `input-system` feature.
                    if name == "action" {
                        if let Expr::Ident(id, _) = recv.as_ref() {
                            if id == "input" && !self.input_system_enabled {
                                self.diag.error(
                                    "E070",
                                    "New Input System sugar (`input.action(...)`) requires the `input-system` feature. Add `features = [\"input-system\"]` under `[language]` in your .prsmproject.",
                                    *name_span,
                                );
                            }
                        }
                    }
                    let receiver_is_this = matches!(recv.as_ref(), Expr::This(_));
                    let receiver_ty = self.analyze_expr(recv);
                    if receiver_is_this {
                        if let Some((ty, definition_id)) = self.lookup_symbol(name) {
                            self.record_reference(name.clone(), HirReferenceKind::Call, definition_id, None, None, *name_span);
                            for arg in args {
                                self.analyze_expr(&arg.value);
                            }
                            return ty;
                        }
                    }
                    let candidate = self.member_candidate_name(&receiver_ty, name);
                    self.record_reference(name.clone(), HirReferenceKind::Call, None, candidate, None, *name_span);
                }
                for arg in args {
                    self.analyze_expr(&arg.value);
                }
                if receiver.is_none() {
                    if let Some((ty, definition_id)) = self.lookup_symbol(name) {
                        self.record_reference(name.clone(), HirReferenceKind::Call, definition_id, None, None, *name_span);
                        return ty;
                    }
                    self.record_reference(name.clone(), HirReferenceKind::Call, None, None, None, *name_span);
                }
                // Return type depends on the callee — for v1, return External
                PrismType::External("var".into())
            }
            Expr::IndexAccess { receiver, index, .. } => {
                self.analyze_expr(receiver);
                self.analyze_expr(index);
                PrismType::External("var".into())
            }
            Expr::Range { start, end, step, .. } => {
                self.analyze_expr(start);
                self.analyze_expr(end);
                if let Some(s) = step {
                    self.analyze_expr(s);
                }
                PrismType::External("Range".into())
            }
            Expr::Is { expr, .. } => {
                self.analyze_expr(expr);
                PrismType::Primitive(PrimitiveKind::Bool)
            }
            Expr::IfExpr { cond, then_block, else_block, .. } => {
                self.analyze_expr(cond);
                self.analyze_block(then_block);
                self.analyze_block(else_block);
                PrismType::External("var".into())
            }
            Expr::WhenExpr { subject, branches, .. } => {
                if let Some(s) = subject {
                    let subj_ty = self.analyze_expr(s);
                    if let PrismType::Enum(enum_name) = &subj_ty {
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
                PrismType::External("var".into())
            }
            Expr::Lambda { body, .. } => {
                self.scopes.push_scope();
                match body {
                    LambdaBody::Block(block) => self.analyze_block(block),
                    LambdaBody::Expr(expr) => { self.analyze_expr(expr); }
                }
                self.scopes.pop_scope();
                PrismType::External("lambda".into())
            }
            Expr::IntrinsicExpr { .. } => PrismType::External("var".into()),
            Expr::SafeCastExpr { expr, target_type, .. } => {
                self.analyze_expr(expr);
                let ty = self.resolve_typeref(target_type);
                PrismType::Nullable(Box::new(ty))
            }
            Expr::ForceCastExpr { expr, target_type, .. } => {
                self.analyze_expr(expr);
                self.resolve_typeref(target_type)
            }
            Expr::Tuple { elements, .. } => {
                for e in elements {
                    self.analyze_expr(e);
                }
                PrismType::External("tuple".into())
            }
            Expr::ListLit { elements, span } => {
                if elements.is_empty() {
                    // Empty literal — only valid with an explicit type annotation
                    // somewhere in the surrounding context. Without that
                    // context we cannot infer; semantic emits a diagnostic
                    // unless the analyzer is currently using analyze_expr_with_expected.
                    // We do not error here unconditionally — defer to
                    // analyze_expr_with_expected for context-aware checks.
                    let _ = span;
                }
                for e in elements {
                    self.analyze_expr(e);
                }
                PrismType::External("list".into())
            }
            Expr::MapLit { entries, .. } => {
                for (k, v) in entries {
                    self.analyze_expr(k);
                    self.analyze_expr(v);
                }
                PrismType::External("map".into())
            }
            Expr::Await { expr: inner, span } => {
                // E135: await is only allowed inside an async func body.
                if !self.in_async_fn {
                    self.diag.error(
                        "E135",
                        "'await' is only allowed inside an async func",
                        *span,
                    );
                }
                self.async_used_await = true;
                self.analyze_expr(inner);
                // Type inference for awaited values is approximate — we do not
                // unwrap Task<T>/UniTask<T> into T yet; downstream inference
                // treats it as `var`.
                PrismType::External("var".into())
            }
            // Language 5, Sprint 2: nameof — yields a string literal at
            // compile time. We validate that the leaf identifier exists
            // in some scope (no E???) — for now we accept anything and
            // just return `String`.
            Expr::NameOf { .. } => PrismType::Primitive(PrimitiveKind::String),
            // Language 5, Sprint 3: `ref expr` — type is the inner
            // expression's type. Reference-ness is tracked at the
            // declaration level, not in the type system today.
            Expr::RefOf { inner, .. } => self.analyze_expr(inner),
            // v5 (deferred): `with` produces a value of the same type
            // as the receiver. The update expressions are analyzed
            // recursively so type errors inside them surface.
            Expr::With { receiver, updates, .. } => {
                let recv_ty = self.analyze_expr(receiver);
                for (_, value) in updates {
                    let _ = self.analyze_expr(value);
                }
                recv_ty
            }
            // v5 (deferred): stackalloc result is `Span<T>`. We model
            // it as an external `Span` with the element type so the
            // rest of the analyzer treats it as a known type.
            Expr::StackAlloc { element_ty, size, .. } => {
                let _ = self.analyze_expr(size);
                let elem = self.resolve_typeref(element_ty);
                PrismType::Generic("Span".into(), vec![elem])
            }
            // Language 5, Sprint 6: `arr?[index]` — type is the element
            // type of the indexed collection (best-effort: we just
            // recurse and use the receiver's type for now).
            Expr::SafeIndexAccess { receiver, .. } => {
                let recv_ty = self.analyze_expr(receiver);
                if let PrismType::Generic(_, args) = &recv_ty {
                    if args.len() == 1 {
                        return PrismType::Nullable(Box::new(args[0].clone()));
                    }
                }
                PrismType::Error
            }
            // Language 5, Sprint 6: `throw expr` in expression position
            // never returns; the type is `Nothing` which the type
            // checker treats as compatible with any expected type.
            Expr::ThrowExpr { exception, .. } => {
                self.analyze_expr(exception);
                PrismType::Error
            }
        }
    }

    fn analyze_expr_with_expected(
        &mut self,
        expr: &Expr,
        expected_type: Option<&PrismType>,
    ) -> PrismType {
        let analyzed = self.analyze_expr(expr);

        if analyzed.is_error() {
            return analyzed;
        }

        let Some(expected_type) = expected_type else {
            return analyzed;
        };

        if let Expr::Call { receiver, name, type_args, .. } = expr {
            if type_args.is_empty() && supports_expected_type_inference_call(receiver.as_deref(), name) {
                return expected_type.non_null().clone();
            }
        }

        analyzed
    }

    /// Walk an expression tree and report v4 collection-literal diagnostics
    /// (E107: empty literal without type) for any nested empty literal that
    /// has no contextual type. This is a best-effort check used by val/var
    /// statements that lack an explicit type annotation.
    fn check_collection_literal_unannotated(&mut self, expr: &Expr) {
        match expr {
            Expr::ListLit { elements, span } if elements.is_empty() => {
                self.diag.error(
                    "E107",
                    "Empty list literal '[]' requires an explicit type annotation",
                    *span,
                );
            }
            Expr::MapLit { entries, span } if entries.is_empty() => {
                self.diag.error(
                    "E107",
                    "Empty map literal '{}' requires an explicit type annotation",
                    *span,
                );
            }
            _ => {}
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

    fn resolve_typeref(&mut self, ty: &TypeRef) -> PrismType {
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
                let args: Vec<PrismType> = type_args.iter().map(|t| self.resolve_typeref(t)).collect();
                let base = PrismType::Generic(name.clone(), args);
                if *nullable { base.make_nullable() } else { base }
            }
            TypeRef::Qualified { qualifier, name, nullable, .. } => {
                let full = format!("{}.{}", qualifier, name);
                let base = PrismType::External(full);
                if *nullable { base.make_nullable() } else { base }
            }
            TypeRef::Tuple { types, nullable, .. } => {
                let _inner: Vec<PrismType> = types.iter().map(|t| self.resolve_typeref(t)).collect();
                let base = PrismType::External("tuple".into());
                if *nullable { base.make_nullable() } else { base }
            }
            TypeRef::Function { param_types, return_type, nullable, .. } => {
                for pt in param_types {
                    self.resolve_typeref(pt);
                }
                self.resolve_typeref(return_type);
                let base = PrismType::External("function".into());
                if *nullable { base.make_nullable() } else { base }
            }
        }
    }

    fn begin_hir(&mut self, file_path: &Path) {
        self.current_file_path = Some(file_path.to_path_buf());
        self.hir_definitions.clear();
        self.hir_references.clear();
        self.hir_pattern_bindings.clear();
        self.hir_listen_sites.clear();
        self.next_definition_id = 1;
        self.current_decl_name = None;
        self.current_member_name = None;
    }

    fn finish_hir(&mut self, file_path: &Path) -> HirFile {
        let definitions = std::mem::take(&mut self.hir_definitions);
        let references = std::mem::take(&mut self.hir_references);
        let pattern_bindings = std::mem::take(&mut self.hir_pattern_bindings);
        let listen_sites = std::mem::take(&mut self.hir_listen_sites);
        self.current_file_path = None;
        self.current_decl_name = None;
        self.current_member_name = None;

        HirFile {
            path: file_path.to_path_buf(),
            definitions,
            references,
            pattern_bindings,
            listen_sites,
        }
    }

    fn lookup_symbol(&self, name: &str) -> Option<(PrismType, Option<u32>)> {
        self.scopes
            .lookup(name)
            .map(|symbol| (symbol.ty.clone(), symbol.definition_id))
    }

    fn lookup_type_symbol(&self, name: &str) -> Option<(PrismType, Option<u32>)> {
        self.scopes.lookup(name).and_then(|symbol| {
            if symbol.kind == SymbolKind::Type {
                Some((symbol.ty.clone(), symbol.definition_id))
            } else {
                None
            }
        })
    }

    fn resolve_named_typeref(&mut self, name: &str, span: Span) -> PrismType {
        if let Some((ty, definition_id)) = self.lookup_type_symbol(name) {
            self.record_reference(
                name.to_string(),
                HirReferenceKind::Type,
                definition_id,
                None,
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
                None,
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
                None,
                span,
            );
        } else if self.known_project_types.contains_key(name) {
            self.record_reference(
                name.to_string(),
                HirReferenceKind::Type,
                None,
                Some(name.to_string()),
                None,
                span,
            );
        }
    }

    fn member_candidate_name(&self, receiver_ty: &PrismType, member_name: &str) -> Option<String> {
        match receiver_ty.non_null() {
            PrismType::Component(name)
            | PrismType::Asset(name)
            | PrismType::Class(name)
            | PrismType::Enum(name) => Some(format!("{}.{}", name, member_name)),
            _ => None,
        }
    }

    fn record_definition(
        &mut self,
        name: String,
        qualified_name: String,
        kind: HirDefinitionKind,
        ty: PrismType,
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
        ty: PrismType,
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
        ty: PrismType,
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
        resolved_type: Option<PrismType>,
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
            resolved_type,
            file_path,
            span,
        });
    }

    fn record_pattern_binding(
        &mut self,
        kind: HirPatternBindingKind,
        type_name: String,
        bindings: Vec<String>,
        has_guard: bool,
        span: Span,
    ) {
        // Validate pattern bindings against known types and retrieve expected arity.
        let expected_arity = self.validate_pattern_bindings(&kind, &type_name, &bindings, span);

        let Some(file_path) = self.current_file_path.clone() else {
            return;
        };

        self.hir_pattern_bindings.push(HirPatternBinding {
            kind,
            owner_qualified_name: self.current_owner_qualified_name(),
            type_name,
            bindings,
            has_guard,
            expected_arity,
            file_path,
            span,
        });
    }

    /// Validate pattern binding arity against known enum payloads / data class fields.
    /// Returns the expected arity if the type is known (for HIR enrichment).
    fn validate_pattern_bindings(
        &mut self,
        kind: &HirPatternBindingKind,
        type_name: &str,
        bindings: &[String],
        span: Span,
    ) -> Option<usize> {
        let mut arity = None;
        match kind {
            HirPatternBindingKind::When => {
                // type_name is e.g. "EnemyState.Chase" — split into enum + variant.
                let segments: Vec<&str> = type_name.splitn(2, '.').collect();
                if segments.len() < 2 {
                    return None; // Single-segment pattern — nothing to validate here.
                }
                let enum_name = segments[0];
                let variant = segments[1];

                // Only validate against enums defined in this file.
                let Some(entries) = self.enum_entries.get(enum_name) else {
                    return None;
                };
                if !entries.contains(&variant.to_string()) {
                    self.diag.error(
                        "E081",
                        format!("Unknown variant '{}' for enum '{}'", variant, enum_name),
                        span,
                    );
                    return None;
                }
                if let Some(payloads) = self.enum_payloads.get(enum_name) {
                    if let Some(&expected_arity) = payloads.get(variant) {
                        arity = Some(expected_arity);
                        if !bindings.is_empty() && bindings.len() != expected_arity {
                            self.diag.error(
                                "E082",
                                format!(
                                    "Pattern binds {} variable(s) but '{}.{}' expects {}",
                                    bindings.len(), enum_name, variant, expected_arity,
                                ),
                                span,
                            );
                        }
                    }
                }
            }
            HirPatternBindingKind::ValDestructure | HirPatternBindingKind::ForDestructure => {
                if let Some(fields) = self.dataclass_fields.get(type_name) {
                    arity = Some(fields.len());
                    if bindings.len() != fields.len() {
                        self.diag.error(
                            "E082",
                            format!(
                                "Pattern binds {} variable(s) but '{}' has {} field(s)",
                                bindings.len(), type_name, fields.len(),
                            ),
                            span,
                        );
                    }
                }
                // Unknown type_name → skip (could be external type).
            }
        }
        arity
    }

    /// SOLID principle warnings for components (Language 3).
    fn check_solid_warnings(&mut self, name: &str, members: &[Member], decl: &Decl) {
        let decl_span = match decl {
            Decl::Component { span, .. } => *span,
            _ => return,
        };

        // W010: Too many public methods (Single Responsibility)
        let public_method_count = members.iter().filter(|m| matches!(m,
            Member::Func { visibility, .. } if *visibility != Visibility::Private
        )).count() + members.iter().filter(|m| matches!(m, Member::Lifecycle { .. })).count();

        if public_method_count > 8 {
            self.diag.warning(
                "W010",
                format!("Component '{}' has {} public methods. Consider splitting responsibilities.", name, public_method_count),
                decl_span,
            );
        }

        // W011: Too many dependencies (Dependency Inversion)
        let dep_count = members.iter().filter(|m| matches!(m,
            Member::Require { .. } | Member::Optional { .. } | Member::Child { .. } | Member::Parent { .. }
        )).count();

        if dep_count > 6 {
            self.diag.warning(
                "W011",
                format!("Component '{}' has {} dependency fields. Consider reducing dependencies.", name, dep_count),
                decl_span,
            );
        }

        // W012: Method too long
        for m in members {
            match m {
                Member::Func { name: fn_name, body: FuncBody::Block(block), .. } => {
                    if block.stmts.len() > 50 {
                        self.diag.warning(
                            "W012",
                            format!("Method '{}' has {} statements. Consider extracting helper methods.", fn_name, block.stmts.len()),
                            block.span,
                        );
                    }
                }
                Member::Lifecycle { kind, body, .. } => {
                    if body.stmts.len() > 50 {
                        self.diag.warning(
                            "W012",
                            format!("Lifecycle '{:?}' has {} statements. Consider extracting helper methods.", kind, body.stmts.len()),
                            body.span,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn record_listen_site(
        &mut self,
        lifetime: ListenLifetime,
        bound_name: Option<String>,
        span: Span,
    ) {
        let Some(file_path) = self.current_file_path.clone() else {
            return;
        };

        self.hir_listen_sites.push(HirListenSite {
            owner_qualified_name: self.current_owner_qualified_name(),
            lifetime: hir_listen_lifetime(lifetime),
            bound_name,
            file_path,
            span,
        });
    }

    fn current_owner_qualified_name(&self) -> Option<String> {
        match (&self.current_decl_name, &self.current_member_name) {
            (Some(decl_name), Some(member_name)) => Some(format!("{}.{}", decl_name, member_name)),
            (Some(decl_name), None) => Some(decl_name.clone()),
            _ => None,
        }
    }
}

fn supports_expected_type_inference_call(receiver: Option<&Expr>, name: &str) -> bool {
    match receiver {
        None => matches!(name, "get" | "require" | "find" | "child" | "parent" | "loadJson"),
        Some(_) => matches!(
            name,
            "getComponent" | "getComponentInChildren" | "getComponentInParent" | "findFirstObjectByType"
        ),
    }
}

fn hir_listen_lifetime(lifetime: ListenLifetime) -> HirListenLifetime {
    match lifetime {
        ListenLifetime::Register => HirListenLifetime::Register,
        ListenLifetime::UntilDisable => HirListenLifetime::UntilDisable,
        ListenLifetime::UntilDestroy => HirListenLifetime::UntilDestroy,
        ListenLifetime::Manual => HirListenLifetime::Manual,
    }
}

/// Uppercase the first ASCII character of `s` (leaves non-ASCII unchanged).
/// Used by Phase 5 lowerings that derive C# type names from camelCase sources.
fn ascii_capitalize(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        Some(c) => c.to_ascii_uppercase().to_string() + it.as_str(),
        None => String::new(),
    }
}

/// Language 5, Sprint 3: walk a sequence of statements and collect every
/// `bind X to ...` source name into the provided set. The recursion
/// covers control-flow constructs and the bodies of nested blocks so
/// nested registrations are also accounted for by W031.
fn collect_bind_to_sources<'a>(
    stmts: &'a [Stmt],
    out: &mut std::collections::HashSet<&'a str>,
) {
    for s in stmts {
        match s {
            Stmt::BindTo { source, .. } => {
                out.insert(source.as_str());
            }
            Stmt::If { then_block, else_branch, .. } => {
                collect_bind_to_sources(&then_block.stmts, out);
                if let Some(eb) = else_branch {
                    match eb {
                        ElseBranch::ElseBlock(block) => {
                            collect_bind_to_sources(&block.stmts, out);
                        }
                        ElseBranch::ElseIf(stmt) => {
                            collect_bind_to_sources(std::slice::from_ref(stmt.as_ref()), out);
                        }
                    }
                }
            }
            Stmt::For { body, .. } | Stmt::While { body, .. } => {
                collect_bind_to_sources(&body.stmts, out);
            }
            Stmt::Try { try_block, catches, finally_block, .. } => {
                collect_bind_to_sources(&try_block.stmts, out);
                for c in catches {
                    collect_bind_to_sources(&c.body.stmts, out);
                }
                if let Some(fb) = finally_block {
                    collect_bind_to_sources(&fb.stmts, out);
                }
            }
            Stmt::Use { body: Some(body), .. } => {
                collect_bind_to_sources(&body.stmts, out);
            }
            Stmt::Listen { body, .. } => {
                collect_bind_to_sources(&body.stmts, out);
            }
            Stmt::Preprocessor { arms, else_arm, .. } => {
                for arm in arms {
                    collect_bind_to_sources(&arm.body, out);
                }
                if let Some(stmts) = else_arm {
                    collect_bind_to_sources(stmts, out);
                }
            }
            _ => {}
        }
    }
}

/// Language 5, Sprint 1: extract the element type T from an iterator-shaped
/// PrSM type. Returns `Some(T)` for `Seq<T>`, `IEnumerator<T>`, and
/// `IEnumerable<T>`. Returns `None` for non-generic iterators or for
/// non-iterator types.
fn iterator_element_type(ty: &PrismType) -> Option<PrismType> {
    if let PrismType::Generic(name, args) = ty {
        if matches!(name.as_str(), "Seq" | "IEnumerator" | "IEnumerable") && args.len() == 1 {
            return Some(args[0].clone());
        }
    }
    None
}

/// Returns true for the non-generic iterator types `IEnumerator` and
/// `IEnumerable` — these are valid iterator return types but place no
/// constraint on the yielded value type, so E148 is suppressed.
fn is_non_generic_iterator(ty: &PrismType) -> bool {
    matches!(ty, PrismType::External(name) if name == "IEnumerator" || name == "IEnumerable")
}

/// The curated set of preprocessor symbols documented in the v5 spec.
/// Symbols outside this list still pass through to the C# preprocessor —
/// the analyzer surfaces W034 only as a hint to the user.
/// Issue #12: well-known type names that should never be the target of
/// a `require` / `optional` / `child` / `parent` qualifier. Used by
/// `Analyzer::check_component_lookup_type` to surface diagnostic E191.
/// The list intentionally errs on the side of false negatives — only
/// types we are sure are not Unity Components are flagged. A more
/// authoritative check would consult the Roslyn sidecar's type registry.
fn is_known_non_component_type(name: &str) -> bool {
    matches!(
        name,
        // Primitives
        "Int" | "Float" | "Double" | "Bool" | "String" | "Long" | "Byte"
            | "Char" | "Unit" | "Object"
        // Collections (PrSM aliases and C# names)
            | "List" | "MutableList" | "Map" | "MutableMap" | "Set"
            | "MutableSet" | "Queue" | "Stack" | "Array" | "Seq"
            | "Dictionary" | "HashSet" | "IList" | "IDictionary"
            | "IEnumerable" | "IEnumerator" | "ICollection"
        // Function types
            | "Action" | "Func" | "Predicate"
    )
}

fn is_known_preprocessor_symbol(name: &str) -> bool {
    matches!(
        name,
        "editor"
            | "debug"
            | "release"
            | "ios"
            | "android"
            | "standalone"
            | "il2cpp"
            | "mono"
            | "unity20223"
            | "unity20231"
            | "unity6"
            // Allow common all-caps user defines without W034 noise.
            | "DEBUG"
            | "TRACE"
            | "RELEASE"
            | "UNITY_EDITOR"
            | "UNITY_IOS"
            | "UNITY_ANDROID"
            | "UNITY_STANDALONE"
            | "ENABLE_IL2CPP"
            | "ENABLE_MONO"
    ) || name.contains('_')
        || name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
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

    /// Analyze multiple source strings together so that types from earlier
    /// sources are visible when analyzing later ones (simulates multi-file
    /// project where enum/data class definitions are shared).
    fn analyze_multi(sources: &[&str]) -> Vec<Diagnostic> {
        let mut analyzer = Analyzer::new();
        for source in sources {
            let mut lexer = Lexer::new(source);
            let tokens = lexer.tokenize();
            let mut parser = Parser::new(tokens);
            let file = parser.parse_file();
            assert!(parser.errors().is_empty(), "Parse errors: {:?}", parser.errors());
            analyzer.analyze_file(&file);
        }
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

    // === W010: SOLID — too many public methods ===

    #[test]
    fn test_solid_too_many_methods() {
        let src = "component Bloated : MonoBehaviour {\n  func a() {}\n  func b() {}\n  func c() {}\n  func d() {}\n  func e() {}\n  func f() {}\n  func g() {}\n  func h() {}\n  func i() {}\n}";
        let diags = warnings(src);
        assert!(diags.iter().any(|d| d.code == "W010"), "expected W010 for too many public methods, got: {:?}", diags);
    }

    #[test]
    fn test_solid_few_methods_no_warning() {
        let src = "component Small : MonoBehaviour {\n  func a() {}\n  func b() {}\n}";
        let diags = warnings(src);
        assert!(!diags.iter().any(|d| d.code == "W010"), "should NOT warn with only 2 methods");
    }

    // === E101: Reserved sugar method name ===

    #[test]
    fn test_reserved_sugar_name_get() {
        let diags = errors("component Foo : MonoBehaviour {\n  func get() {}\n}");
        assert!(diags.iter().any(|d| d.code == "E101"), "expected E101 for reserved name 'get', got: {:?}", diags);
    }

    #[test]
    fn test_reserved_sugar_name_find() {
        let diags = errors("component Foo : MonoBehaviour {\n  func find() {}\n}");
        assert!(diags.iter().any(|d| d.code == "E101"), "expected E101 for reserved name 'find', got: {:?}", diags);
    }

    #[test]
    fn test_non_reserved_name_ok() {
        let diags = errors("component Foo : MonoBehaviour {\n  func getData() {}\n}");
        assert!(!diags.iter().any(|d| d.code == "E101"), "should NOT error for non-reserved name");
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

    // === E081: Unknown enum variant in pattern ===

    #[test]
    fn test_pattern_unknown_variant() {
        let diags = analyze_multi(&[
            "enum State {\n  Idle,\n  Running\n}",
            "component Foo : MonoBehaviour {\n  var state: State = State.Idle\n  update {\n    when state {\n      State.NonExistent(x) => fail()\n    }\n  }\n}",
        ]);
        let errs: Vec<_> = diags.iter().filter(|d| d.severity == Severity::Error).collect();
        assert!(errs.iter().any(|d| d.code == "E081"), "expected E081 for unknown variant, got: {:?}", errs);
    }

    // === E082: Pattern arity mismatch ===

    #[test]
    fn test_pattern_arity_mismatch() {
        let diags = analyze_multi(&[
            "enum Action(val target: String) {\n  Move(\"p\"),\n  Attack(\"e\")\n}",
            "component Foo : MonoBehaviour {\n  var action: Action = Action.Move(\"p\")\n  update {\n    when action {\n      Action.Move(target, extra) => doMove(target)\n    }\n  }\n}",
        ]);
        let errs: Vec<_> = diags.iter().filter(|d| d.severity == Severity::Error).collect();
        assert!(errs.iter().any(|d| d.code == "E082"), "expected E082 for arity mismatch, got: {:?}", errs);
    }

    #[test]
    fn test_destructure_arity_mismatch() {
        let diags = analyze_multi(&[
            "data class Stats(hp: Int, speed: Float)",
            "component Foo : MonoBehaviour {\n  func f() {\n    val Stats(hp, speed, extra) = getStats()\n  }\n}",
        ]);
        let errs: Vec<_> = diags.iter().filter(|d| d.severity == Severity::Error).collect();
        assert!(errs.iter().any(|d| d.code == "E082"), "expected E082 for destructure arity mismatch, got: {:?}", errs);
    }

    // === E083: listen lifetime in wrong context ===

    #[test]
    fn test_listen_until_disable_in_class() {
        let src = "class Helper {\n  func setup(button: Button) {\n    listen button.onClick until disable {\n      fire()\n    }\n  }\n}";
        let diags = errors(src);
        assert!(diags.iter().any(|d| d.code == "E083"), "expected E083 for listen until disable in class, got: {:?}", diags);
    }

    #[test]
    fn test_listen_manual_in_asset() {
        let src = "asset Config : ScriptableObject {\n  func setup(button: Button) {\n    val token = listen button.onClick manual {\n      fire()\n    }\n  }\n}";
        let diags = errors(src);
        assert!(diags.iter().any(|d| d.code == "E083"), "expected E083 for listen manual in asset, got: {:?}", diags);
    }

    #[test]
    fn test_listen_until_disable_in_component_ok() {
        let src = "component Foo : MonoBehaviour {\n  serialize button: Button\n  start {\n    listen button.onClick until disable {\n      fire()\n    }\n  }\n}";
        let diags = errors(src);
        assert!(!diags.iter().any(|d| d.code == "E083"), "should not error for listen until disable in component");
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

    // === v4 Phase 4 — collection literals (E107) ===

    #[test]
    fn test_empty_list_without_type_annotation_errors() {
        let diags = errors(
            "component Foo : MonoBehaviour {\n  func f() {\n    val xs = []\n  }\n}",
        );
        assert!(
            diags.iter().any(|d| d.code == "E107"),
            "expected E107: {:?}",
            diags
        );
    }

    #[test]
    fn test_empty_list_with_type_annotation_ok() {
        let diags = errors(
            "component Foo : MonoBehaviour {\n  func f() {\n    val xs: List<Int> = []\n  }\n}",
        );
        assert!(
            !diags.iter().any(|d| d.code == "E107"),
            "did not expect E107: {:?}",
            diags
        );
    }

    #[test]
    fn test_list_literal_with_elements_ok() {
        let diags = errors(
            "component Foo : MonoBehaviour {\n  func f() {\n    val xs = [1, 2, 3]\n  }\n}",
        );
        assert!(
            !diags.iter().any(|d| d.code == "E107"),
            "did not expect E107 for non-empty list: {:?}",
            diags
        );
    }

    #[test]
    fn test_event_member_no_errors() {
        let diags = errors(
            "component Boss : MonoBehaviour {\n  event onDeath: () => Unit\n}",
        );
        assert!(diags.is_empty(), "Unexpected errors: {:?}", diags);
    }

    #[test]
    fn test_use_stmt_block_no_errors() {
        let diags = errors(
            "component Foo : MonoBehaviour {\n  func f() {\n    use s = openFile() {\n      log(s)\n    }\n  }\n}",
        );
        assert!(diags.is_empty(), "Unexpected errors: {:?}", diags);
    }

    // ── Phase 5: async / state machine / command / bind ──────────

    #[test]
    fn test_async_func_with_await_no_errors() {
        let diags = errors(
            "component Loader : MonoBehaviour {\n  async func ping() {\n    await delay(1)\n  }\n}",
        );
        assert!(diags.is_empty(), "Unexpected errors: {:?}", diags);
    }

    #[test]
    fn test_await_outside_async_e135() {
        let diags = errors(
            "component Foo : MonoBehaviour {\n  func ping() {\n    await delay(1)\n  }\n}",
        );
        assert!(
            diags.iter().any(|d| d.code == "E135"),
            "expected E135 for await outside async: {:?}",
            diags
        );
    }

    #[test]
    fn test_async_func_no_await_warning_w025() {
        let warns = warnings(
            "component Foo : MonoBehaviour {\n  async func empty() {\n    log(\"no await\")\n  }\n}",
        );
        assert!(
            warns.iter().any(|d| d.code == "W025"),
            "expected W025 for async without await: {:?}",
            warns
        );
    }

    #[test]
    fn test_state_machine_valid_no_errors() {
        let diags = errors(
            "component AI : MonoBehaviour {\n  state machine ai {\n    state Idle { on go => Run }\n    state Run { on stopRun => Idle }\n  }\n}",
        );
        assert!(diags.is_empty(), "Unexpected errors: {:?}", diags);
    }

    #[test]
    fn test_state_machine_unknown_target_e140() {
        let diags = errors(
            "component AI : MonoBehaviour {\n  state machine ai {\n    state Idle { on go => Missing }\n  }\n}",
        );
        assert!(
            diags.iter().any(|d| d.code == "E140"),
            "expected E140 for unknown target state: {:?}",
            diags
        );
    }

    #[test]
    fn test_state_machine_duplicate_state_e141() {
        let diags = errors(
            "component AI : MonoBehaviour {\n  state machine ai {\n    state Idle { on go => Idle }\n    state Idle { on stopRun => Idle }\n  }\n}",
        );
        assert!(
            diags.iter().any(|d| d.code == "E141"),
            "expected E141 for duplicate state: {:?}",
            diags
        );
    }

    #[test]
    fn test_command_member_no_errors() {
        let diags = errors(
            "component Unit : MonoBehaviour {\n  command moveTo(target: Vector3) {\n    log(\"go\")\n  }\n}",
        );
        assert!(diags.is_empty(), "Unexpected errors: {:?}", diags);
    }

    #[test]
    fn test_command_with_undo_no_errors() {
        let diags = errors(
            "component Unit : MonoBehaviour {\n  command damage(amount: Int) {\n    log(\"hurt\")\n  } undo {\n    log(\"heal\")\n  } canExecute = true\n}",
        );
        assert!(diags.is_empty(), "Unexpected errors: {:?}", diags);
    }

    #[test]
    fn test_bind_property_no_errors() {
        let diags = errors(
            "component HUD : MonoBehaviour {\n  bind hp: Int = 100\n}",
        );
        assert!(diags.is_empty(), "Unexpected errors: {:?}", diags);
    }

    #[test]
    fn test_bind_to_invalid_target_e143() {
        // Target is a literal — not writable. Expect E143.
        let diags = errors(
            "component HUD : MonoBehaviour {\n  bind hp: Int = 100\n  awake {\n    bind hp to 42\n  }\n}",
        );
        assert!(
            diags.iter().any(|d| d.code == "E143"),
            "expected E143: {:?}",
            diags
        );
    }

    // ── Language 5, Sprint 1 ───────────────────────────────────────

    // E147: yield outside an iterator context.
    #[test]
    fn test_yield_outside_iterator_e147() {
        let diags = errors(
            "component Foo : MonoBehaviour {\n  func f() {\n    yield 1\n  }\n}",
        );
        assert!(
            diags.iter().any(|d| d.code == "E147"),
            "expected E147 for yield outside iterator: {:?}",
            diags
        );
    }

    // yield inside a coroutine — accepted with no diagnostics.
    #[test]
    fn test_yield_in_coroutine_ok() {
        let diags = errors(
            "component Foo : MonoBehaviour {\n  coroutine count(): Seq<Int> {\n    yield 1\n    yield 2\n    yield break\n  }\n}",
        );
        assert!(
            diags.is_empty(),
            "Unexpected errors for yield in coroutine: {:?}",
            diags
        );
    }

    // yield inside a func with iterator return type — also valid.
    #[test]
    fn test_yield_in_iterator_func_ok() {
        let diags = errors(
            "component Foo : MonoBehaviour {\n  func nums(): Seq<Int> {\n    yield 1\n    yield 2\n  }\n}",
        );
        assert!(
            diags.is_empty(),
            "Unexpected errors for yield in iterator-returning func: {:?}",
            diags
        );
    }

    // W033: coroutine declares an element type but never yields a value.
    #[test]
    fn test_coroutine_without_yield_w033() {
        let diags = warnings(
            "component Foo : MonoBehaviour {\n  coroutine empty(): Seq<Int> {\n    val x = 1\n  }\n}",
        );
        assert!(
            diags.iter().any(|d| d.code == "W033"),
            "expected W033 when iterator never yields: {:?}",
            diags
        );
    }

    // Preprocessor block — known PrSM symbols pass without W034.
    #[test]
    fn test_preprocessor_known_symbol_no_w034() {
        let diags = analyze(
            "component Foo : MonoBehaviour {\n  update {\n    #if editor\n      val x = 1\n    #endif\n  }\n}",
        );
        assert!(
            !diags.iter().any(|d| d.code == "W034"),
            "should not warn on known symbol 'editor': {:?}",
            diags
        );
    }

    // ── Language 5, Sprint 3 ───────────────────────────────────────

    // W031: a `bind` member that has no readers anywhere in the
    // component should warn.
    #[test]
    fn test_bind_member_never_read_w031() {
        let diags = warnings(
            "component HUD : MonoBehaviour {\n  bind hp: Int = 100\n}",
        );
        assert!(
            diags.iter().any(|d| d.code == "W031"),
            "expected W031 for unread bind member: {:?}",
            diags
        );
    }

    // A bind member that has at least one `bind to` site is considered
    // read and should not trigger W031.
    #[test]
    fn test_bind_member_with_bind_to_no_w031() {
        let diags = warnings(
            "component HUD : MonoBehaviour {\n  bind hp: Int = 100\n  awake {\n    bind hp to label.text\n  }\n}",
        );
        assert!(
            !diags.iter().any(|d| d.code == "W031"),
            "should not warn when bind member has a `bind to` site: {:?}",
            diags
        );
    }

    // Preprocessor block — unknown lowercase symbol triggers W034.
    #[test]
    fn test_preprocessor_unknown_symbol_w034() {
        let diags = warnings(
            "component Foo : MonoBehaviour {\n  update {\n    #if maybeFeature\n      val x = 1\n    #endif\n  }\n}",
        );
        assert!(
            diags.iter().any(|d| d.code == "W034"),
            "expected W034 for unknown preprocessor symbol: {:?}",
            diags
        );
    }
}
