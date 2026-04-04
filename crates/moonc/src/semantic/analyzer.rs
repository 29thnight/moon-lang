use crate::ast::*;
use crate::diagnostics::DiagnosticCollector;
use crate::lexer::token::Span;
use super::types::*;
use super::scope::*;

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
        }
    }

    /// Analyze a file.
    pub fn analyze_file(&mut self, file: &File) {
        // Phase 1: Register the top-level declaration
        self.register_decl(&file.decl);

        // Phase 2: Analyze the declaration body
        self.analyze_decl(&file.decl);
    }

    fn register_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Component { name, .. } => {
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty: MoonType::Component(name.clone()),
                    kind: SymbolKind::Type,
                    mutable: false,
                });
            }
            Decl::Asset { name, .. } => {
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty: MoonType::Asset(name.clone()),
                    kind: SymbolKind::Type,
                    mutable: false,
                });
            }
            Decl::Class { name, .. } => {
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty: MoonType::Class(name.clone()),
                    kind: SymbolKind::Type,
                    mutable: false,
                });
            }
            Decl::DataClass { name, .. } => {
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty: MoonType::Class(name.clone()),
                    kind: SymbolKind::Type,
                    mutable: false,
                });
            }
            Decl::Enum { name, entries, .. } => {
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty: MoonType::Enum(name.clone()),
                    kind: SymbolKind::Type,
                    mutable: false,
                });
                let entry_names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
                self.enum_entries.insert(name.clone(), entry_names);
            }
            Decl::Attribute { name, .. } => {
                self.scopes.define(Symbol {
                    name: name.clone(),
                    ty: MoonType::Class(name.clone()),
                    kind: SymbolKind::Type,
                    mutable: false,
                });
            }
        }
    }

    fn analyze_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Component { members, .. } => {
                self.decl_ctx = DeclContext::Component;
                self.scopes.push_scope();
                self.analyze_members(members);
                self.check_duplicate_lifecycles(members);
                self.scopes.pop_scope();
                self.decl_ctx = DeclContext::None;
            }
            Decl::Asset { members, .. } => {
                self.decl_ctx = DeclContext::Asset;
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
                self.decl_ctx = DeclContext::None;
            }
            Decl::Class { members, .. } => {
                self.decl_ctx = DeclContext::Class;
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
                self.decl_ctx = DeclContext::None;
            }
            Decl::DataClass { fields, span, .. } => {
                self.decl_ctx = DeclContext::DataClass;
                // Validate fields
                if fields.is_empty() {
                    self.diag.warning("W005", "Data class has no fields", *span);
                }
                self.decl_ctx = DeclContext::None;
            }
            Decl::Enum { name, entries, span, .. } => {
                self.decl_ctx = DeclContext::Enum;
                if entries.is_empty() {
                    self.diag.error("E050", format!("Enum '{}' must have at least one entry", name), *span);
                }
                // Check duplicate entries
                let mut seen = std::collections::HashSet::new();
                for entry in entries {
                    if !seen.insert(&entry.name) {
                        self.diag.error("E052", format!("Duplicate enum entry '{}'", entry.name), entry.span);
                    }
                }
                self.decl_ctx = DeclContext::None;
            }
            Decl::Attribute { .. } => {
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
            Member::SerializeField { name, ty, .. } => {
                let btype = self.resolve_typeref(ty);
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::SerializeField, mutable: true,
                });
            }
            Member::Field { name, ty, mutability, .. } => {
                let btype = ty.as_ref().map(|t| self.resolve_typeref(t)).unwrap_or(MoonType::Error);
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype,
                    kind: SymbolKind::Field,
                    mutable: *mutability == Mutability::Var,
                });
            }
            Member::Require { name, ty, .. } => {
                let btype = self.resolve_typeref(ty);
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::RequiredComponent, mutable: false,
                });
            }
            Member::Optional { name, ty, .. } => {
                let btype = self.resolve_typeref(ty).make_nullable();
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::OptionalComponent, mutable: false,
                });
            }
            Member::Child { name, ty, .. } => {
                let btype = self.resolve_typeref(ty);
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::RequiredComponent, mutable: false,
                });
            }
            Member::Parent { name, ty, .. } => {
                let btype = self.resolve_typeref(ty);
                self.scopes.define(Symbol {
                    name: name.clone(), ty: btype, kind: SymbolKind::RequiredComponent, mutable: false,
                });
            }
            Member::Func { name, return_ty, .. } => {
                let ret = return_ty.as_ref().map(|t| self.resolve_typeref(t)).unwrap_or(MoonType::Unit);
                self.scopes.define(Symbol {
                    name: name.clone(), ty: ret, kind: SymbolKind::Function, mutable: false,
                });
            }
            Member::Coroutine { name, .. } => {
                self.scopes.define(Symbol {
                    name: name.clone(), ty: MoonType::Unit, kind: SymbolKind::Coroutine, mutable: false,
                });
            }
            _ => {} // lifecycle, intrinsic — no symbol registration needed
        }
    }

    fn analyze_member_body(&mut self, member: &Member) {
        match member {
            Member::Func { params, body, .. } => {
                self.body_ctx = BodyContext::Function;
                self.scopes.push_scope();
                for p in params {
                    let ty = self.resolve_typeref(&p.ty);
                    self.scopes.define(Symbol {
                        name: p.name.clone(), ty, kind: SymbolKind::Parameter, mutable: false,
                    });
                }
                match body {
                    FuncBody::Block(block) => self.analyze_block(block),
                    FuncBody::ExprBody(expr) => { self.analyze_expr(expr); }
                }
                self.scopes.pop_scope();
                self.body_ctx = BodyContext::None;
            }
            Member::Coroutine { params, body, .. } => {
                self.body_ctx = BodyContext::Coroutine;
                self.scopes.push_scope();
                for p in params {
                    let ty = self.resolve_typeref(&p.ty);
                    self.scopes.define(Symbol {
                        name: p.name.clone(), ty, kind: SymbolKind::Parameter, mutable: false,
                    });
                }
                self.analyze_block(body);
                self.scopes.pop_scope();
                self.body_ctx = BodyContext::None;
            }
            Member::Lifecycle { params, body, .. } => {
                self.body_ctx = BodyContext::Lifecycle;
                self.scopes.push_scope();
                for p in params {
                    let ty = self.resolve_typeref(&p.ty);
                    self.scopes.define(Symbol {
                        name: p.name.clone(), ty, kind: SymbolKind::Parameter, mutable: false,
                    });
                }
                self.analyze_block(body);
                self.scopes.pop_scope();
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
            Stmt::ValDecl { name, ty, init, span } => {
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
                self.scopes.define(Symbol {
                    name: name.clone(), ty: declared_ty, kind: SymbolKind::Local, mutable: false,
                });
            }
            Stmt::VarDecl { name, ty, init, span } => {
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
                self.scopes.define(Symbol {
                    name: name.clone(), ty: declared_ty, kind: SymbolKind::Local, mutable: true,
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
            Stmt::For { var_name, iterable, body, .. } => {
                let iter_ty = self.analyze_expr(iterable);
                self.scopes.push_scope();
                // For range expressions, infer the element type
                let elem_ty = match &iter_ty {
                    // Range of ints → Int
                    _ => MoonType::External("var".into()), // simplified for v1
                };
                self.scopes.define(Symbol {
                    name: var_name.clone(), ty: elem_ty, kind: SymbolKind::Local, mutable: false,
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
            Expr::Ident(name, _) => {
                if let Some(sym) = self.scopes.lookup(name) {
                    sym.ty.clone()
                } else {
                    // Don't error for Unity API names — we can't resolve them without
                    // a full Unity type database. For v1, unknown idents are External.
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
            Expr::MemberAccess { receiver, name, .. } => {
                self.analyze_expr(receiver);
                // Cannot fully resolve member types without Unity type DB
                MoonType::External(name.clone())
            }
            Expr::SafeCall { receiver, name, .. } => {
                let _recv_ty = self.analyze_expr(receiver);
                MoonType::External(name.clone()).make_nullable()
            }
            Expr::SafeMethodCall { receiver, args, .. } => {
                self.analyze_expr(receiver);
                for arg in args {
                    self.analyze_expr(&arg.value);
                }
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
            Expr::Call { receiver, args, .. } => {
                if let Some(recv) = receiver {
                    self.analyze_expr(recv);
                }
                for arg in args {
                    self.analyze_expr(&arg.value);
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
                    self.analyze_expr(s);
                }
                for b in branches {
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

    fn resolve_typeref(&self, ty: &TypeRef) -> MoonType {
        match ty {
            TypeRef::Simple { name, nullable, .. } => {
                let base = resolve_type_name(name);
                if *nullable { base.make_nullable() } else { base }
            }
            TypeRef::Generic { name, type_args, nullable, .. } => {
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
