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
}

/// A member of an interface declaration — method signature or property.
#[derive(Debug, Clone)]
pub enum InterfaceMember {
    Func {
        name: String,
        name_span: Span,
        params: Vec<Param>,
        return_ty: Option<TypeRef>,
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
        is_override: bool,
        name: String,
        name_span: Span,
        type_params: Vec<String>,
        where_clauses: Vec<WhereClause>,
        params: Vec<Param>,
        return_ty: Option<TypeRef>,
        body: FuncBody,
        span: Span,
    },
    Coroutine {
        name: String,
        name_span: Span,
        params: Vec<Param>,
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
}

// ── Statements ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    ValDecl {
        name: String,
        name_span: Span,
        ty: Option<TypeRef>,
        init: Expr,
        span: Span,
    },
    VarDecl {
        name: String,
        name_span: Span,
        ty: Option<TypeRef>,
        init: Option<Expr>,
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
        params: Vec<String>,
        body: Block,
        span: Span,
    },
    IntrinsicExpr {
        ty: TypeRef,
        code: String,
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
    pub name: Option<String>,
    pub value: Expr,
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
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub name: String,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}
