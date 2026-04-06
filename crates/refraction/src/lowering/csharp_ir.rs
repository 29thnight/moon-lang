//! C# Intermediate Representation.
//!
//! A structured representation of C# code that can be emitted as source text.
//! This sits between the PrSM AST and the final C# output.

use crate::lexer::token::Span;

/// A complete C# file.
#[derive(Debug, Clone)]
pub struct CsFile {
    pub header_comment: String,
    pub usings: Vec<String>,
    pub class: CsClass,
    pub extra_types: Vec<CsClass>,
}

/// A C# class declaration.
#[derive(Debug, Clone)]
pub struct CsClass {
    pub attributes: Vec<String>,
    pub modifiers: String,       // "public", "public sealed", etc.
    pub name: String,
    pub base_class: Option<String>,
    pub interfaces: Vec<String>,
    pub where_clauses: Vec<String>, // e.g. ["where T : Component"]
    pub members: Vec<CsMember>,
}

/// A member of a C# class.
#[derive(Debug, Clone)]
pub enum CsMember {
    Field {
        attributes: Vec<String>,
        modifiers: String,
        ty: String,
        name: String,
        init: Option<String>,
    },
    Property {
        modifiers: String,
        ty: String,
        name: String,
        getter_expr: String,
        setter: Option<String>, // None=readonly, Some("set")=public set, Some("private set")=private set
        setter_expr: Option<String>,
    },
    Method {
        attributes: Vec<String>,
        modifiers: String,
        return_ty: String,
        name: String,
        params: Vec<CsParam>,
        where_clauses: Vec<String>, // e.g. ["where T : Component"]
        body: Vec<CsStmt>,
        source_span: Option<Span>,
    },
    RawCode(String),
}

#[derive(Debug, Clone)]
pub struct CsParam {
    pub ty: String,
    pub name: String,
    pub default: Option<String>,
}

/// A C# statement.
#[derive(Debug, Clone)]
pub enum CsStmt {
    VarDecl {
        ty: String,   // "var" or explicit type
        name: String,
        init: String,
        source_span: Option<Span>,
    },
    Assignment {
        target: String,
        op: String,
        value: String,
        source_span: Option<Span>,
    },
    Expr(String, Option<Span>),
    If {
        cond: String,
        then_body: Vec<CsStmt>,
        else_body: Option<Vec<CsStmt>>,
        source_span: Option<Span>,
    },
    Switch {
        subject: String,
        cases: Vec<CsSwitchCase>,
        source_span: Option<Span>,
    },
    For {
        init: String,
        cond: String,
        incr: String,
        body: Vec<CsStmt>,
        source_span: Option<Span>,
    },
    ForEach {
        ty: String,
        name: String,
        iterable: String,
        body: Vec<CsStmt>,
        source_span: Option<Span>,
    },
    While {
        cond: String,
        body: Vec<CsStmt>,
        source_span: Option<Span>,
    },
    Return(Option<String>, Option<Span>),
    YieldReturn(String, Option<Span>),
    Break(Option<Span>),
    Continue(Option<Span>),
    Raw(String, Option<Span>),
    Block(Vec<CsStmt>, Option<Span>),
}

#[derive(Debug, Clone)]
pub struct CsSwitchCase {
    pub pattern: String,   // "case X:" or "default:"
    pub body: Vec<CsStmt>,
}
