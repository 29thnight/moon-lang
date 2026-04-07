//! PrSM AST node definitions.
//!
//! The AST is an immutable tree of nodes. Every node carries a `Span`
//! for diagnostics. Rust enums provide exhaustive pattern matching —
//! the compiler catches any unhandled node variants.

use crate::lexer::token::Span;

// ── File ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct File {
    pub usings: Vec<UsingDecl>,
    pub decl: Decl,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UsingDecl {
    pub path: String, // e.g. "UnityEngine.UI"
    pub span: Span,
}

// ── Declarations ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Decl {
    Component {
        is_singleton: bool,
        name: String,
        name_span: Span,
        base_class: String,
        base_class_span: Span,
        interfaces: Vec<String>,
        interface_spans: Vec<Span>,
        members: Vec<Member>,
        span: Span,
    },
    Asset {
        name: String,
        name_span: Span,
        base_class: String,
        base_class_span: Span,
        members: Vec<Member>,
        span: Span,
    },
    Class {
        name: String,
        name_span: Span,
        is_abstract: bool,
        is_sealed: bool,
        type_params: Vec<String>,
        where_clauses: Vec<WhereClause>,
        super_class: Option<String>,
        super_class_span: Option<Span>,
        interfaces: Vec<String>,
        interface_spans: Vec<Span>,
        members: Vec<Member>,
        span: Span,
    },
    DataClass {
        name: String,
        name_span: Span,
        fields: Vec<Param>,
        span: Span,
    },
    Enum {
        name: String,
        name_span: Span,
        params: Vec<EnumParam>,
        entries: Vec<EnumEntry>,
        span: Span,
    },
    /// `attribute Name(params) : Target, Target` → lowered to C# attribute class
    Attribute {
        name: String,
        name_span: Span,
        fields: Vec<Param>,
        targets: Vec<String>,  // e.g. ["Method", "Property"]. Empty = All
        span: Span,
    },
    /// `interface Name [: SuperInterfaces] { func ...; val ...; }` (since Language 3)
    Interface {
        name: String,
        name_span: Span,
        extends: Vec<String>,
        extends_spans: Vec<Span>,
        members: Vec<InterfaceMember>,
        span: Span,
    },
    /// `typealias Name = Type` (since Language 4)
    TypeAlias {
        name: String,
        name_span: Span,
        target: TypeRef,
        span: Span,
    },
    /// `struct Name(fields) { optional members }` (since Language 4)
    Struct {
        name: String,
        name_span: Span,
        fields: Vec<Param>,
        members: Vec<Member>,
        span: Span,
    },
    /// `extend TypeName { members }` — extension methods (since Language 4)
    Extension {
        target_type: TypeRef,
        members: Vec<Member>,
        span: Span,
    },
}

/// A member of an interface declaration — method signature or property.
#[derive(Debug, Clone)]
pub enum InterfaceMember {
    Func {
        name: String,
        name_span: Span,
        params: Vec<Param>,
        return_ty: Option<TypeRef>,
        /// Default implementation body — `Some(block)` enables a default
        /// interface method (DIM). `None` is a pure signature.
        default_body: Option<Block>,
        span: Span,
    },
    Property {
        name: String,
        name_span: Span,
        ty: TypeRef,
        mutable: bool, // val = false, var = true
        span: Span,
    },
}

/// Generic where clause: `where T : MonoBehaviour, IDamageable`
#[derive(Debug, Clone)]
pub struct WhereClause {
    pub type_param: String,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EnumParam {
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Debug, Clone)]
pub struct EnumEntry {
    pub name: String,
    pub name_span: Span,
    pub args: Vec<Expr>,
    pub span: Span,
}

// ── Members ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Member {
    SerializeField {
        annotations: Vec<Annotation>,
        visibility: Option<Visibility>,
        mutability: Mutability,  // val or var
        name: String,
        name_span: Span,
        ty: TypeRef,
        init: Option<Expr>,
        span: Span,
    },
    Field {
        visibility: Visibility,
        is_static: bool,
        mutability: Mutability,
        name: String,
        name_span: Span,
        ty: Option<TypeRef>,
        init: Option<Expr>,
        span: Span,
    },
    Require {
        name: String,
        name_span: Span,
        ty: TypeRef,
        span: Span,
    },
    Optional {
        name: String,
        name_span: Span,
        ty: TypeRef,
        span: Span,
    },
    Child {
        name: String,
        name_span: Span,
        ty: TypeRef,
        span: Span,
    },
    Parent {
        name: String,
        name_span: Span,
        ty: TypeRef,
        span: Span,
    },
    Func {
        visibility: Visibility,
        is_static: bool,
        is_override: bool,
        is_abstract: bool,
        is_open: bool,
        is_operator: bool,
        /// v4: `async func` — lowered to async Task/UniTask method.
        is_async: bool,
        /// v5 Sprint 2: leading `@burst`/`@header`/etc. annotations attached
        /// to this function. The lowering pass uses these to drive
        /// `[BurstCompile]` emission and burst analysis.
        annotations: Vec<Annotation>,
        name: String,
        name_span: Span,
        type_params: Vec<String>,
        where_clauses: Vec<WhereClause>,
        params: Vec<Param>,
        return_ty: Option<TypeRef>,
        body: FuncBody,
        span: Span,
    },
    /// Custom property with get/set (since Language 4)
    Property {
        mutability: Mutability,
        name: String,
        name_span: Span,
        ty: TypeRef,
        getter: Option<FuncBody>,
        setter: Option<PropertySetter>,
        /// v5 Sprint 1: `serialize` modifier was applied. When combined with
        /// an auto-property (empty get/set bodies) this triggers
        /// `[field: SerializeField]` lowering for the backing field.
        is_serialize: bool,
        /// v5 Sprint 1: `@field(...)` / `@property(...)` / `@return(...)`
        /// attribute target annotations attached to this property.
        target_annotations: Vec<TargetAnnotation>,
        span: Span,
    },
    Coroutine {
        name: String,
        name_span: Span,
        params: Vec<Param>,
        /// v5 Sprint 1: optional declared return type for the iterator.
        /// `None` lowers to `IEnumerator`. `Some(Seq<T>)` / `Some(IEnumerator<T>)`
        /// lower to `IEnumerator<T>`. Used by yield value type checking (E148).
        return_ty: Option<TypeRef>,
        body: Block,
        span: Span,
    },
    Lifecycle {
        kind: LifecycleKind,
        params: Vec<Param>,
        body: Block,
        span: Span,
    },
    IntrinsicFunc {
        visibility: Visibility,
        name: String,
        name_span: Span,
        params: Vec<Param>,
        return_ty: Option<TypeRef>,
        code: String,
        span: Span,
    },
    IntrinsicCoroutine {
        name: String,
        name_span: Span,
        params: Vec<Param>,
        code: String,
        span: Span,
    },
    Pool {
        name: String,
        name_span: Span,
        item_type: TypeRef,
        capacity: u32,
        max_size: u32,
        span: Span,
    },
    /// `event onDamaged: (Int) => Unit` — multicast delegate (Language 4)
    Event {
        visibility: Visibility,
        name: String,
        name_span: Span,
        /// Function type describing the delegate signature.
        ty: TypeRef,
        span: Span,
    },
    /// `state machine Name { state S { enter { } exit { } on ev => T } }`
    /// Phase 5: finite state machine sugar (sugar.state).
    StateMachine {
        name: String,
        name_span: Span,
        states: Vec<StateDecl>,
        span: Span,
    },
    /// `command Name(params) { execute { } undo { } canExecute = expr }`
    /// Phase 5: command pattern sugar (sugar.command).
    Command {
        name: String,
        name_span: Span,
        params: Vec<Param>,
        execute: Block,
        undo: Option<Block>,
        can_execute: Option<Expr>,
        span: Span,
    },
    /// `bind name: Type = init` — reactive property with change notification.
    /// Phase 5: bind / MVVM sugar (sugar.bind).
    BindProperty {
        name: String,
        name_span: Span,
        ty: TypeRef,
        init: Option<Expr>,
        span: Span,
    },
}

/// A single state inside a `state machine` declaration.
#[derive(Debug, Clone)]
pub struct StateDecl {
    pub name: String,
    pub name_span: Span,
    pub enter: Option<Block>,
    pub exit: Option<Block>,
    pub transitions: Vec<StateTransition>,
    pub span: Span,
}

/// `on event => targetState` transition rule.
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub event: String,
    pub event_span: Span,
    pub target: String,
    pub target_span: Span,
    pub span: Span,
}

// ── Statements ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    ValDecl {
        name: String,
        name_span: Span,
        ty: Option<TypeRef>,
        init: Expr,
        /// v5 Sprint 3: `val ref name = ref expr` declares a `ref readonly`
        /// local. The init expression must be an `Expr::RefOf`.
        is_ref: bool,
        span: Span,
    },
    VarDecl {
        name: String,
        name_span: Span,
        ty: Option<TypeRef>,
        init: Option<Expr>,
        /// v5 Sprint 3: `var ref name = ref expr` declares a `ref` local.
        is_ref: bool,
        span: Span,
    },
    Assignment {
        target: Expr,
        op: AssignOp,
        value: Expr,
        span: Span,
    },
    Expr {
        expr: Expr,
        span: Span,
    },
    If {
        cond: Expr,
        then_block: Block,
        else_branch: Option<ElseBranch>,
        span: Span,
    },
    When {
        subject: Option<Expr>,
        branches: Vec<WhenBranch>,
        span: Span,
    },
    For {
        var_name: String,
        name_span: Span,
        /// v2: optional destructuring pattern — `for EnemySpawn(x, y) in ...`
        for_pattern: Option<DestructurePattern>,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    /// v2: `val PlayerStats(hp, speed) = stats` — data-class / enum-payload destructuring
    DestructureVal {
        pattern: DestructurePattern,
        init: Expr,
        span: Span,
    },
    While {
        cond: Expr,
        body: Block,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        /// v5 Sprint 3: `return ref expr` — only valid inside a function
        /// declared with a `ref` return type. The value must be an
        /// addressable expression.
        is_ref: bool,
        span: Span,
    },
    Wait {
        form: WaitForm,
        span: Span,
    },
    Start {
        call: Expr,
        span: Span,
    },
    Stop {
        target: Expr,
        span: Span,
    },
    StopAll {
        span: Span,
    },
    Listen {
        event: Expr,
        params: Vec<String>,
        lifetime: ListenLifetime,
        /// For `val name = listen event manual { }` — the bound variable name.
        bound_name: Option<String>,
        body: Block,
        span: Span,
    },
    Unlisten {
        token: String,
        span: Span,
    },
    IntrinsicBlock {
        code: String,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    /// `try { } catch (e: Type) { } finally { }` (since Language 4)
    Try {
        try_block: Block,
        catches: Vec<CatchClause>,
        finally_block: Option<Block>,
        span: Span,
    },
    /// `throw expr` (since Language 4)
    Throw {
        expr: Expr,
        span: Span,
    },
    /// `yield expr` — iterator value (Language 5, Sprint 1)
    /// Valid only inside a coroutine declaration or a func returning
    /// `Seq<T>`/`IEnumerator`/`IEnumerator<T>`/`IEnumerable`/`IEnumerable<T>`.
    Yield {
        value: Expr,
        span: Span,
    },
    /// `yield break` — terminate iterator (Language 5, Sprint 1)
    YieldBreak {
        span: Span,
    },
    /// `#if cond ... #elif cond ... #else ... #endif` — preprocessor directive
    /// block (Language 5, Sprint 1). Each arm holds statements that appear
    /// inside the branch body; the lowering pass emits the literal C#
    /// preprocessor around them unchanged.
    Preprocessor {
        arms: Vec<PreprocessorArm>,
        else_arm: Option<Vec<Stmt>>,
        span: Span,
    },
    /// `use val name = expr` (declaration form — disposed at scope exit)
    /// or `use name = expr { body }` (block form — disposed at block exit).
    /// Lowered to C# `using` declaration / `using` statement.
    Use {
        name: String,
        name_span: Span,
        ty: Option<TypeRef>,
        init: Expr,
        /// `Some(block)` → block form; `None` → declaration form (`use val ...`)
        body: Option<Block>,
        span: Span,
    },
    /// `bind source to target.member` — Phase 5 bind/MVVM statement form.
    /// Registers a one-way push so that subsequent changes to `source`
    /// propagate to `target` automatically (lowered to a direct assignment
    /// plus an entry in the owning component's bind table).
    BindTo {
        /// The bound source identifier (must refer to a `bind` property).
        source: String,
        source_span: Span,
        /// The target expression, usually a member access like `label.text`.
        target: Expr,
        span: Span,
    },
}

/// A catch clause: `catch (name: Type) { body }`
#[derive(Debug, Clone)]
pub struct CatchClause {
    pub name: String,
    pub ty: TypeRef,
    pub body: Block,
    pub span: Span,
}

/// A single `#if` / `#elif` arm inside a preprocessor directive block.
/// `cond` is the raw PrSM-level condition AST that the lowering pass
/// translates to the matching C# preprocessor expression.
#[derive(Debug, Clone)]
pub struct PreprocessorArm {
    pub cond: PreprocessorCond,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// Preprocessor condition expression.
///
/// Grammar (Language 5, Sprint 1):
///
/// ```text
/// Cond = Symbol | "!" Cond | Cond "&&" Cond | Cond "||" Cond | "(" Cond ")"
/// ```
///
/// Symbols may be a curated PrSM alias (e.g. `editor`, `ios`, `il2cpp`)
/// that maps to a Unity `UNITY_*` define, or an arbitrary identifier
/// that passes through verbatim (with a W034 warning).
#[derive(Debug, Clone)]
pub enum PreprocessorCond {
    Symbol { name: String, span: Span },
    Not { inner: Box<PreprocessorCond>, span: Span },
    And { left: Box<PreprocessorCond>, right: Box<PreprocessorCond>, span: Span },
    Or { left: Box<PreprocessorCond>, right: Box<PreprocessorCond>, span: Span },
}

#[derive(Debug, Clone)]
pub enum ElseBranch {
    ElseBlock(Block),
    ElseIf(Box<Stmt>), // must be Stmt::If
}

#[derive(Debug, Clone)]
pub struct WhenBranch {
    pub pattern: WhenPattern,
    /// v2: optional guard condition — `EnemyState.Stunned(d) if d > 0.0 => …`
    pub guard: Option<Expr>,
    pub body: WhenBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum WhenPattern {
    Expression(Expr),
    Is(TypeRef),
    Else,
    /// v2: `EnemyState.Chase(target)` — member access path + optional binding names
    Binding {
        /// Fully-qualified enum path, e.g. ["EnemyState", "Chase"]
        path: Vec<String>,
        /// Bound variable names from payload, e.g. ["target"]
        bindings: Vec<String>,
        span: Span,
    },
    /// v4: OR pattern — multiple patterns separated by comma
    Or {
        patterns: Vec<WhenPattern>,
        span: Span,
    },
    /// v4: Range pattern — `in start..end`
    Range {
        start: Expr,
        end: Expr,
        span: Span,
    },
}

/// v2: destructuring pattern for val/for statements.
#[derive(Debug, Clone)]
pub struct DestructurePattern {
    /// Type name, e.g. "PlayerStats"
    pub type_name: String,
    /// Bound variable names
    pub bindings: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum WhenBody {
    Block(Block),
    Expr(Expr),
}

/// Setter definition for a custom property.
#[derive(Debug, Clone)]
pub struct PropertySetter {
    /// The name of the value parameter, e.g. "value"
    pub param_name: String,
    pub body: Block,
}

/// Lambda parameter (may have optional type annotation).
#[derive(Debug, Clone)]
pub struct LambdaParam {
    pub name: String,
    pub ty: Option<TypeRef>,
}

/// How long a `listen` subscription lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenLifetime {
    /// v1: register only, never deregister (default when no modifier)
    Register,
    /// Auto-remove in OnDisable: `listen event until disable { … }`
    UntilDisable,
    /// Auto-remove in OnDestroy: `listen event until destroy { … }`
    UntilDestroy,
    /// Return subscription token; user calls `unlisten token`: `val t = listen event manual { … }`
    Manual,
}

#[derive(Debug, Clone)]
pub enum WaitForm {
    Duration(Expr),
    NextFrame,
    FixedFrame,
    Until(Expr),
    While(Expr),
}

// ── Expressions ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64, Span),
    FloatLit(f64, Span),
    DurationLit(f64, Span),
    StringLit(String, Span),
    StringInterp {
        parts: Vec<StringPart>,
        span: Span,
    },
    BoolLit(bool, Span),
    Null(Span),
    Ident(String, Span),
    This(Span),
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    MemberAccess {
        receiver: Box<Expr>,
        name: String,
        name_span: Span,
        span: Span,
    },
    SafeCall {
        receiver: Box<Expr>,
        name: String,
        name_span: Span,
        span: Span,
    },
    /// Safe method call: expr?.name(args) — null check + method call
    SafeMethodCall {
        receiver: Box<Expr>,
        name: String,
        name_span: Span,
        type_args: Vec<TypeRef>,
        args: Vec<Arg>,
        span: Span,
    },
    NonNullAssert {
        expr: Box<Expr>,
        span: Span,
    },
    Elvis {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Call {
        receiver: Option<Box<Expr>>,
        name: String,
        name_span: Span,
        type_args: Vec<TypeRef>,
        args: Vec<Arg>,
        span: Span,
    },
    IndexAccess {
        receiver: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    IfExpr {
        cond: Box<Expr>,
        then_block: Block,
        else_block: Block,
        span: Span,
    },
    WhenExpr {
        subject: Option<Box<Expr>>,
        branches: Vec<WhenBranch>,
        span: Span,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        step: Option<Box<Expr>>,
        span: Span,
    },
    Is {
        expr: Box<Expr>,
        ty: TypeRef,
        span: Span,
    },
    Lambda {
        params: Vec<LambdaParam>,
        body: LambdaBody,
        span: Span,
    },
    IntrinsicExpr {
        ty: TypeRef,
        code: String,
        span: Span,
    },
    /// `expr as Type?` — safe cast returning null on failure (Language 4)
    SafeCastExpr {
        expr: Box<Expr>,
        target_type: TypeRef,
        span: Span,
    },
    /// `expr as! Type` — force cast throwing on failure (Language 4)
    ForceCastExpr {
        expr: Box<Expr>,
        target_type: TypeRef,
        span: Span,
    },
    /// `(a, b, c)` — tuple expression (Language 4)
    Tuple {
        elements: Vec<Expr>,
        span: Span,
    },
    /// `[1, 2, 3]` — list literal (Language 4)
    ListLit {
        elements: Vec<Expr>,
        span: Span,
    },
    /// `{"a": 1, "b": 2}` — map literal (Language 4)
    MapLit {
        entries: Vec<(Expr, Expr)>,
        span: Span,
    },
    /// `await expr` — Phase 5 async/await (stmt.async).
    /// Modeled as an expression so `val x = await foo()` works naturally.
    Await {
        expr: Box<Expr>,
        span: Span,
    },
    /// `nameof(target)` — yields the source identifier of `target` as a
    /// string literal at compile time. Path is a dotted reference like
    /// `nameof(player.hp)` or `nameof(Player)`. The semantic analyzer
    /// validates that the leaf identifier exists; lowering emits the
    /// matching `nameof(...)` expression in C# verbatim.
    /// (Language 5, Sprint 2)
    NameOf {
        /// Dot-separated path components, e.g. `["player", "hp"]`.
        path: Vec<String>,
        span: Span,
    },
    /// v5 Sprint 3: `ref expr` — used as the init expression of a
    /// `val ref` / `var ref` declaration and as the operand of `ref return`.
    /// The inner expression must be addressable (a field or local). The
    /// lowering pass emits `ref expr` verbatim in the matching C# slot.
    RefOf {
        inner: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Literal(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct Arg {
    /// Named argument: `instantiate(prefab, parent: rootTransform)`.
    /// (v5 Sprint 2 enables the parser path; the AST field has existed.)
    pub name: Option<String>,
    /// v5 Sprint 2: `ref` / `out` / `out val` / `out var` / `out _`
    /// modifier on the argument. Lowered to the matching C# call-site
    /// modifier.
    pub call_modifier: ArgMod,
    pub value: Expr,
}

/// v5 Sprint 2: call-site modifier for an argument. Mirrors the C#
/// `ref` / `out` keywords. `OutDecl` corresponds to PrSM `out val name`
/// or `out var name`, lowered to C# `out var name` (declaration form).
/// `OutDiscard` corresponds to `out _`, lowered to C# `out _`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ArgMod {
    #[default]
    None,
    Ref,
    Out,
    /// `out val name` / `out var name` — declaration expression. The
    /// inner `String` is the bound variable name; the outer expression
    /// is `Expr::Ident(name, span)` so existing semantic checks treat it
    /// uniformly.
    OutDecl(String),
    /// `out _` — discard the result.
    OutDiscard,
}

// ── Types ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TypeRef {
    Simple {
        name: String,
        nullable: bool,
        span: Span,
    },
    Generic {
        name: String,
        type_args: Vec<TypeRef>,
        nullable: bool,
        span: Span,
    },
    Qualified {
        qualifier: String,
        name: String,
        nullable: bool,
        span: Span,
    },
    /// `(Int, String)` — tuple type (Language 4)
    Tuple {
        types: Vec<TypeRef>,
        nullable: bool,
        span: Span,
    },
    /// `(Int, Int) => Bool` — function type (Language 4)
    Function {
        param_types: Vec<TypeRef>,
        return_type: Box<TypeRef>,
        nullable: bool,
        span: Span,
    },
}

// ── Supporting types ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub name_span: Span,
    pub ty: TypeRef,
    pub default: Option<Expr>,
    /// v5 Sprint 2: parameter passing mode — `ref`, `out`, or absent.
    pub modifier: ParamMod,
    /// v5 Sprint 2: `vararg` modifier (Kotlin-style `params T[]` in C#).
    /// Only the last parameter of a function may be `vararg`.
    pub is_vararg: bool,
    pub span: Span,
}

/// v5 Sprint 2: parameter passing mode for `ref` / `out` parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamMod {
    /// Standard pass-by-value parameter.
    None,
    /// `ref` — caller passes a writable reference; lowered to C# `ref T`.
    Ref,
    /// `out` — callee shall assign before all return paths; lowered to
    /// C# `out T`. The semantic analyzer enforces E154.
    Out,
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub name: String,
    pub args: Vec<Expr>,
    pub span: Span,
}

/// v5 Sprint 1: attribute target annotation of the form
/// `@field(name)`, `@property(name)`, `@param(name)`, `@return(name)`, `@type(name)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrTargetKind {
    Field,
    Property,
    Param,
    Return,
    Type,
}

#[derive(Debug, Clone)]
pub struct TargetAnnotation {
    pub target: AttrTargetKind,
    /// The raw attribute name as it should appear inside the C# brackets,
    /// e.g. `"SerializeField"` or `"NonSerialized"`.
    pub attr_name: String,
    /// Optional attribute arguments (literal exprs only for now).
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    Val,
    Var,
    Const,  // compile-time constant → C# const
    Fixed,  // runtime immutable → C# readonly
}

#[derive(Debug, Clone)]
pub enum FuncBody {
    Block(Block),
    ExprBody(Expr),
}

/// Lambda body — either a block or a single expression.
#[derive(Debug, Clone)]
pub enum LambdaBody {
    Block(Block),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleKind {
    Awake,
    Start,
    Update,
    FixedUpdate,
    LateUpdate,
    OnEnable,
    OnDisable,
    OnDestroy,
    OnTriggerEnter,
    OnTriggerExit,
    OnTriggerStay,
    OnCollisionEnter,
    OnCollisionExit,
    OnCollisionStay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    ModAssign,
    NullCoalesceAssign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    In,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}
