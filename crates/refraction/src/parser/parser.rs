use crate::ast::*;
use crate::lexer::token::*;

/// Parse error with location and message.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

/// Intermediate result returned by `try_parse_binding_pattern`.
struct BindingPatternResult {
    path: Vec<String>,
    bindings: Vec<String>,
    span: Span,
}

/// Recursive descent parser for PrSM.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<ParseError>,
    /// Issue #50: disables trailing-lambda call desugar inside control-flow
    /// condition positions (`if expr { body }`, `while expr { body }`,
    /// `for v in expr { body }`, `when expr { ... }`). Without this flag,
    /// the parser would try to interpret the control-flow body `{` as a
    /// trailing-lambda argument on the preceding call, breaking every
    /// existing `if foo() { ... }` example.
    no_trailing_lambda: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            errors: Vec::new(),
            no_trailing_lambda: false,
        }
    }

    /// Issue #50: run `f` with `no_trailing_lambda` temporarily set to
    /// true. Used by `if` / `while` / `for` / `when` subject parsing.
    fn with_no_trailing_lambda<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let prev = self.no_trailing_lambda;
        self.no_trailing_lambda = true;
        let result = f(self);
        self.no_trailing_lambda = prev;
        result
    }

    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    // ── Token navigation ─────────────────────────────────────────

    fn peek(&self) -> &TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn peek_at(&self, offset: usize) -> Option<&TokenKind> {
        self.tokens.get(self.pos + offset).map(|t| &t.kind)
    }

    fn peek_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span { start: Position { line: 0, col: 0 }, end: Position { line: 0, col: 0 } })
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if tok.kind != TokenKind::Eof {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    fn eat(&mut self, expected: &TokenKind) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<Span, ParseError> {
        if self.check(expected) {
            let span = self.peek_span();
            self.advance();
            Ok(span)
        } else {
            Err(self.error(format!("Expected {:?}, found {:?}", expected, self.peek())))
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        match self.peek().clone() {
            TokenKind::Identifier(name) => {
                let span = self.peek_span();
                self.advance();
                Ok((name, span))
            }
            _ => {
                Err(self.error(format!("Expected identifier, found {:?}", self.peek())))
            }
        }
    }

    /// Like `expect_ident`, but also accepts keyword tokens by mapping
    /// them back to their raw source text via `TokenKind::keyword_text`.
    /// Used in positions where a keyword may legitimately serve as an
    /// identifier — for example, parameter names (`Slice(start: Int)`)
    /// or state machine state names (`state Update { ... }`). The
    /// caller is responsible for guaranteeing that the keyword's normal
    /// meaning would not have triggered here (i.e. the production is
    /// already inside a context where only an identifier is expected).
    fn expect_ident_or_keyword(&mut self) -> Result<(String, Span), ParseError> {
        match self.peek().clone() {
            TokenKind::Identifier(name) => {
                let span = self.peek_span();
                self.advance();
                Ok((name, span))
            }
            ref kind => {
                if let Some(text) = kind.keyword_text() {
                    let span = self.peek_span();
                    self.advance();
                    Ok((text.to_string(), span))
                } else {
                    Err(self.error(format!("Expected identifier, found {:?}", self.peek())))
                }
            }
        }
    }

    /// Check if the current token is the contextual identifier with the given text.
    fn check_contextual(&self, text: &str) -> bool {
        matches!(self.peek(), TokenKind::Identifier(name) if name == text)
    }

    /// If the current token is the contextual identifier with the given text,
    /// consume it and return its span.
    fn eat_contextual(&mut self, text: &str) -> Option<Span> {
        if self.check_contextual(text) {
            let span = self.peek_span();
            self.advance();
            Some(span)
        } else {
            None
        }
    }

    /// Look ahead `offset` non-newline tokens and test whether that token is
    /// an Identifier with the given text. `offset = 0` tests the current token.
    fn peek_ahead_is_contextual(&self, offset: usize, text: &str) -> bool {
        let mut p = self.pos;
        let mut seen = 0usize;
        // skip any leading newlines
        while p < self.tokens.len() && self.tokens[p].kind == TokenKind::Newline {
            p += 1;
        }
        while p < self.tokens.len() {
            if seen == offset {
                return matches!(&self.tokens[p].kind, TokenKind::Identifier(n) if n == text);
            }
            p += 1;
            while p < self.tokens.len() && self.tokens[p].kind == TokenKind::Newline {
                p += 1;
            }
            seen += 1;
        }
        false
    }

    fn skip_newlines(&mut self) {
        while self.peek() == &TokenKind::Newline {
            self.advance();
        }
    }

    fn expect_newline_or_eof(&mut self) {
        match self.peek() {
            TokenKind::Newline => { self.advance(); }
            TokenKind::Eof => {}
            TokenKind::RBrace => {} // allow } to end a statement
            _ => {
                self.errors.push(self.error("Expected newline or end of statement".into()));
            }
        }
    }

    fn error(&self, message: String) -> ParseError {
        ParseError {
            message,
            span: self.peek_span(),
        }
    }

    // ── File parsing ─────────────────────────────────────────────

    pub fn parse_file(&mut self) -> File {
        self.skip_newlines();
        let start = self.peek_span();

        // Parse using declarations
        let mut usings = Vec::new();
        while self.peek() == &TokenKind::Using {
            match self.parse_using() {
                Ok(u) => usings.push(u),
                Err(e) => { self.errors.push(e); self.recover_to_newline(); }
            }
            self.skip_newlines();
        }

        // Parse the main declaration
        let mut decl_ok = false;
        let decl = match self.parse_decl() {
            Ok(d) => {
                decl_ok = true;
                d
            }
            Err(e) => {
                self.errors.push(e);
                // Return a dummy component
                Decl::Component {
                    is_singleton: false,
                    is_partial: false,
                    name: "<error>".into(),
                    name_span: self.peek_span(),
                    base_class: "MonoBehaviour".into(),
                    base_class_span: self.peek_span(),
                    interfaces: vec![],
                    interface_spans: vec![],
                    members: vec![],
                    span: self.peek_span(),
                }
            }
        };

        // Issue #8: PrSM requires exactly one top-level declaration per
        // file (S5.1). Earlier versions silently dropped any second
        // declaration without a diagnostic, producing the worst possible
        // developer experience ("where did my function go?"). Emit a
        // hard error pointing at the start of the offending declaration.
        //
        // Only check when the first declaration parsed successfully —
        // a failed first parse already produces a diagnostic and the
        // leftover check would surface a misleading second error.
        if decl_ok {
            self.skip_newlines();
            if self.peek() != &TokenKind::Eof {
                let leftover_span = self.peek_span();
                self.errors.push(ParseError {
                    message: "E189: Multiple top-level declarations in a single file. Each .prsm file shall contain exactly one top-level declaration (S5.1). Move the additional declaration into its own .prsm file.".into(),
                    span: leftover_span,
                });
            }
        }

        let end = self.peek_span();
        File {
            usings,
            decl,
            span: Span { start: start.start, end: end.end },
        }
    }

    fn parse_using(&mut self) -> Result<UsingDecl, ParseError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Using)?;

        let mut path = String::new();
        let (first, _) = self.expect_ident()?;
        path.push_str(&first);

        while self.eat(&TokenKind::Dot) {
            let (next, _) = self.expect_ident()?;
            path.push('.');
            path.push_str(&next);
        }

        self.expect_newline_or_eof();
        Ok(UsingDecl { path, span: Span { start: start.start, end: self.peek_span().end } })
    }

    // ── Declaration parsing ──────────────────────────────────────

    fn parse_decl(&mut self) -> Result<Decl, ParseError> {
        self.skip_newlines();

        // Collect annotations before declaration (for @targets on attribute)
        let annotations = self.parse_annotations()?;

        // v5 (deferred): `ref struct Name(...)` — only recognized when
        // the contextual `ref` is immediately followed by `struct`.
        if self.check_contextual("ref")
            && matches!(self.tokens.get(self.pos + 1).map(|t| t.kind.clone()), Some(TokenKind::Struct))
        {
            self.advance(); // consume 'ref'
            return self.parse_struct_with(true);
        }

        // v5 Sprint 5: optional `partial` modifier on a top-level
        // component or class declaration. The contextual keyword is
        // recognized only when the next token is `component` or `class`,
        // so existing identifiers named `partial` continue to parse.
        if self.check_contextual("partial") {
            let next_kind = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
            if matches!(next_kind, Some(TokenKind::Component) | Some(TokenKind::Class) | Some(TokenKind::Singleton)) {
                self.advance(); // consume 'partial'
                self.skip_newlines();
                return match self.peek().clone() {
                    TokenKind::Component => self.parse_component_decl_with(false, true),
                    TokenKind::Singleton => {
                        self.advance();
                        self.skip_newlines();
                        if !self.check(&TokenKind::Component) {
                            return Err(self.error("'partial singleton' must be followed by 'component'".into()));
                        }
                        self.parse_component_decl_with(true, true)
                    }
                    TokenKind::Class => self.parse_class_with_modifiers_full(false, false, true),
                    _ => Err(self.error("'partial' can only modify 'component' or 'class'".into())),
                };
            }
        }

        match self.peek().clone() {
            TokenKind::Component => self.parse_component_decl(false),
            TokenKind::Singleton => {
                self.advance(); // consume 'singleton'
                self.skip_newlines();
                if !self.check(&TokenKind::Component) {
                    return Err(self.error("'singleton' can only be used before 'component'".into()));
                }
                self.parse_component_decl(true)
            }
            TokenKind::Asset => self.parse_asset(),
            TokenKind::Data => self.parse_data_class(),
            TokenKind::Class => self.parse_class_with_modifiers(false, false),
            TokenKind::Abstract => {
                self.advance(); // consume 'abstract'
                self.skip_newlines();
                if !self.check(&TokenKind::Class) {
                    return Err(self.error("'abstract' can only be used before 'class'".into()));
                }
                self.parse_class_with_modifiers(true, false)
            }
            TokenKind::Sealed => {
                self.advance(); // consume 'sealed'
                self.skip_newlines();
                if !self.check(&TokenKind::Class) {
                    return Err(self.error("'sealed' can only be used before 'class'".into()));
                }
                self.parse_class_with_modifiers(false, true)
            }
            TokenKind::Enum => self.parse_enum(),
            TokenKind::Attribute => self.parse_attribute_decl(annotations),
            TokenKind::Interface => self.parse_interface(),
            TokenKind::TypeAlias => self.parse_type_alias(),
            TokenKind::Struct => self.parse_struct(),
            TokenKind::Extend => self.parse_extension(),
            // Issue #59: top-level `func` / `const` / `coroutine`. Wrap
            // the parsed member in a synthetic `Globals` partial class.
            // The partial modifier lets multiple files contribute to the
            // same `Globals` class, so `const MAX_HEALTH = 100` in A.prsm
            // and `func getName() = "Alice"` in B.prsm merge into one
            // `public static partial class Globals { ... }` in C#.
            TokenKind::Func | TokenKind::Const | TokenKind::Coroutine => {
                self.parse_top_level_item_as_globals(annotations)
            }
            _ => Err(self.error(format!("Expected declaration (component, singleton, asset, class, enum, attribute, interface, typealias, struct, extend, func, const, coroutine), found {:?}", self.peek()))),
        }
    }

    /// Issue #59: wrap a top-level `func` / `const` / `coroutine` in a
    /// synthetic `partial class Globals` so the existing Class lowering
    /// path handles them. The member is marked `public static` where
    /// applicable. If the file declares multiple items (currently
    /// unsupported — enforced by `parse_file`'s S5.1 check), each
    /// lands in its own Globals decl; the driver-level merge is future
    /// work.
    fn parse_top_level_item_as_globals(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        let member = match self.peek().clone() {
            TokenKind::Func => {
                // Parse as `public static func name(...)`. `parse_func_with_annotations`
                // consumes the `func` token and calls `parse_func_inner_ext`.
                self.parse_func_with_annotations(Visibility::Public, false, annotations.clone())
                    .and_then(|m| match m {
                        Member::Func {
                            name,
                            name_span,
                            type_params,
                            where_clauses,
                            params,
                            return_ty,
                            body,
                            is_operator,
                            is_async,
                            is_override,
                            is_abstract,
                            is_open,
                            annotations,
                            span,
                            ..
                        } => Ok(Member::Func {
                            visibility: Visibility::Public,
                            is_static: true,
                            is_override,
                            is_abstract,
                            is_open,
                            is_operator,
                            is_async,
                            annotations,
                            name,
                            name_span,
                            type_params,
                            where_clauses,
                            params,
                            return_ty,
                            body,
                            span,
                        }),
                        other => Ok(other),
                    })?
            }
            TokenKind::Const | TokenKind::Fixed => {
                // Parse `const NAME: Type = literal` as a public static
                // field on the Globals class.
                self.parse_val_var_field_with_static(Visibility::Public, true)?
            }
            TokenKind::Coroutine => {
                self.parse_coroutine()?
            }
            _ => unreachable!("parse_top_level_item_as_globals called with wrong token"),
        };
        // Consume optional trailing newlines after the member body.
        self.skip_newlines();
        let span = Span { start: start.start, end: self.peek_span().end };
        let name_span = start;
        Ok(Decl::Class {
            name: "Globals".to_string(),
            name_span,
            is_abstract: false,
            is_sealed: false,
            is_partial: true,
            type_params: vec![],
            where_clauses: vec![],
            super_class: None,
            super_class_span: None,
            interfaces: vec![],
            interface_spans: vec![],
            members: vec![member],
            primary_ctor_params: vec![],
            span,
        })
    }

    fn parse_interface(&mut self) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'interface'
        self.skip_newlines();

        let (name, name_span) = self.expect_ident()?;

        // Optional extends: interface Foo : Bar, Baz
        let mut extends = Vec::new();
        let mut extends_spans = Vec::new();
        if self.eat(&TokenKind::Colon) {
            let (ext_name, ext_span) = self.expect_ident()?;
            extends.push(ext_name);
            extends_spans.push(ext_span);
            while self.eat(&TokenKind::Comma) {
                let (ext_name, ext_span) = self.expect_ident()?;
                extends.push(ext_name);
                extends_spans.push(ext_span);
            }
        }

        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();

        let mut members = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.skip_newlines();
            if self.check(&TokenKind::RBrace) { break; }

            let member_start = self.peek_span();
            if self.check(&TokenKind::Func) {
                self.advance(); // consume 'func'
                let (fn_name, fn_name_span) = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                let params = if self.check(&TokenKind::RParen) {
                    Vec::new()
                } else {
                    self.parse_param_list()?
                };
                self.expect(&TokenKind::RParen)?;
                let return_ty = if self.eat(&TokenKind::Colon) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                // v4: optional default body — `func name() { body }` lowers to a
                // C# default interface method (DIM).
                let saved = self.pos;
                self.skip_newlines();
                let default_body = if self.check(&TokenKind::LBrace) {
                    Some(self.parse_block()?)
                } else {
                    self.pos = saved;
                    None
                };
                members.push(InterfaceMember::Func {
                    name: fn_name,
                    name_span: fn_name_span,
                    params,
                    return_ty,
                    default_body,
                    span: Span { start: member_start.start, end: self.peek_span().end },
                });
            } else if self.check(&TokenKind::Val) || self.check(&TokenKind::Var) {
                let mutable = self.peek() == &TokenKind::Var;
                self.advance(); // consume val/var
                let (prop_name, prop_name_span) = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                let ty = self.parse_type()?;
                members.push(InterfaceMember::Property {
                    name: prop_name,
                    name_span: prop_name_span,
                    ty,
                    mutable,
                    span: Span { start: member_start.start, end: self.peek_span().end },
                });
            } else {
                return Err(self.error(format!(
                    "Expected 'func', 'val', or 'var' in interface body, found {:?}",
                    self.peek()
                )));
            }
            self.expect_newline_or_eof();
            self.skip_newlines();
        }

        self.expect(&TokenKind::RBrace)?;

        Ok(Decl::Interface {
            name,
            name_span,
            extends,
            extends_spans,
            members,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_type_alias(&mut self) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'typealias'
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Eq)?;
        let target = self.parse_type()?;
        self.expect_newline_or_eof();
        Ok(Decl::TypeAlias {
            name,
            name_span,
            target,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_struct(&mut self) -> Result<Decl, ParseError> {
        self.parse_struct_with(false)
    }

    fn parse_struct_with(&mut self, is_ref: bool) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'struct'
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let fields = if self.check(&TokenKind::RParen) {
            Vec::new()
        } else {
            self.parse_param_list()?
        };
        self.expect(&TokenKind::RParen)?;

        // Optional body block with members
        let members = if self.check(&TokenKind::LBrace) || {
            // Check if next non-newline is LBrace
            let saved = self.pos;
            self.skip_newlines();
            let has_brace = self.check(&TokenKind::LBrace);
            if !has_brace { self.pos = saved; }
            has_brace
        } {
            self.expect(&TokenKind::LBrace)?;
            let ms = self.parse_members()?;
            self.expect(&TokenKind::RBrace)?;
            ms
        } else {
            self.expect_newline_or_eof();
            vec![]
        };

        Ok(Decl::Struct {
            name,
            name_span,
            is_ref,
            fields,
            members,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_component_decl(&mut self, is_singleton: bool) -> Result<Decl, ParseError> {
        self.parse_component_decl_with(is_singleton, false)
    }

    fn parse_component_decl_with(&mut self, is_singleton: bool, is_partial: bool) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'component'

        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let (base_class, base_class_span) = self.expect_ident()?;
        let (interfaces, interface_spans) = self.parse_interface_list()?;

        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        let members = self.parse_members()?;
        self.expect(&TokenKind::RBrace)?;

        Ok(Decl::Component {
            is_singleton,
            is_partial,
            name,
            name_span,
            base_class,
            base_class_span,
            interfaces,
            interface_spans,
            members,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_asset(&mut self) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'asset'

        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let (base_class, base_class_span) = self.expect_ident()?;

        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        let members = self.parse_members()?;
        self.expect(&TokenKind::RBrace)?;

        Ok(Decl::Asset {
            name,
            name_span,
            base_class,
            base_class_span,
            members,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_class_with_modifiers(&mut self, is_abstract: bool, is_sealed: bool) -> Result<Decl, ParseError> {
        self.parse_class_with_modifiers_full(is_abstract, is_sealed, false)
    }

    fn parse_class_with_modifiers_full(&mut self, is_abstract: bool, is_sealed: bool, is_partial: bool) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'class'

        let (name, name_span) = self.expect_ident()?;

        // Optional type parameters: <T, U>
        let type_params = if self.check(&TokenKind::Lt) {
            self.parse_type_params()?
        } else {
            Vec::new()
        };

        // Issue #37: optional primary constructor parameter list,
        // mirroring the data-class shape so the lang-4 sealed
        // hierarchy `class Circle(radius: Float) : Shape` parses.
        let primary_ctor_params = if self.eat(&TokenKind::LParen) {
            let mut params = Vec::new();
            self.skip_newlines();
            while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                self.eat(&TokenKind::Val);
                self.eat(&TokenKind::Var);
                let p = self.parse_param()?;
                params.push(p);
                self.skip_newlines();
                if !self.eat(&TokenKind::Comma) { break; }
                self.skip_newlines();
            }
            self.expect(&TokenKind::RParen)?;
            params
        } else {
            Vec::new()
        };

        let (super_class, super_class_span) = if self.eat(&TokenKind::Colon) {
            let (sc, span) = self.expect_ident()?;
            (Some(sc), Some(span))
        } else {
            (None, None)
        };
        let (interfaces, interface_spans) = if super_class.is_some() {
            self.parse_interface_list()?
        } else {
            (vec![], vec![])
        };

        // Optional where clauses before the class body
        let where_clauses = self.parse_where_clauses()?;

        // Issue #37: when a class has primary-ctor params and no
        // explicit body, the body block is optional. Otherwise the
        // body is required.
        self.skip_newlines();
        let members = if self.check(&TokenKind::LBrace) {
            self.advance();
            let ms = self.parse_members()?;
            self.expect(&TokenKind::RBrace)?;
            ms
        } else if !primary_ctor_params.is_empty() {
            Vec::new()
        } else {
            self.expect(&TokenKind::LBrace)?;
            Vec::new()
        };

        Ok(Decl::Class {
            name,
            name_span,
            is_abstract,
            is_sealed,
            is_partial,
            type_params,
            where_clauses,
            super_class,
            super_class_span,
            interfaces,
            interface_spans,
            members,
            primary_ctor_params,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_interface_list(&mut self) -> Result<(Vec<String>, Vec<Span>), ParseError> {
        let mut interfaces = Vec::new();
        let mut interface_spans = Vec::new();
        while self.eat(&TokenKind::Comma) {
            let (iface, span) = self.expect_ident()?;
            interfaces.push(iface);
            interface_spans.push(span);
        }
        Ok((interfaces, interface_spans))
    }

    /// Parse `<T, U>` type parameter list.
    fn parse_type_params(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(&TokenKind::Lt)?;
        let mut params = Vec::new();
        let (first, _) = self.expect_ident()?;
        params.push(first);
        while self.eat(&TokenKind::Comma) {
            let (p, _) = self.expect_ident()?;
            params.push(p);
        }
        self.expect(&TokenKind::Gt)?;
        Ok(params)
    }

    /// Parse optional `where T : Constraint, U : Other` clauses.
    /// Each clause: Identifier `:` Identifier { `,` Identifier }
    /// Multiple clauses separated by continuing to see identifiers after the
    /// previous constraint list (comma is used within a single clause for
    /// multiple constraints on the same type param).
    fn parse_where_clauses(&mut self) -> Result<Vec<WhereClause>, ParseError> {
        let mut clauses = Vec::new();
        // `where` is not a keyword token — it arrives as an Identifier
        if let TokenKind::Identifier(ref w) = self.peek().clone() {
            if w != "where" {
                return Ok(clauses);
            }
        } else {
            return Ok(clauses);
        }
        self.advance(); // consume 'where'
        loop {
            let (type_param, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let mut constraints = Vec::new();
            let (first, _) = self.expect_ident()?;
            constraints.push(first);
            // Consume additional constraints separated by commas, but only if
            // the token after the comma is an identifier followed by a colon
            // (which would start a new clause) — in that case we stop.
            while self.check(&TokenKind::Comma) {
                // Peek ahead: comma then ident then colon means new clause
                // comma then ident then NOT colon means continued constraints
                let saved = self.pos;
                self.advance(); // consume comma
                if let TokenKind::Identifier(next_name) = self.peek().clone() {
                    self.advance(); // consume ident — pos is now AFTER the ident
                    if self.check(&TokenKind::Colon) {
                        // This is a new where clause — backtrack fully so the
                        // outer loop re-enters `parse_where_clauses` with the
                        // comma intact.
                        self.pos = saved;
                        break;
                    } else {
                        // Issue #54: previously we reset `self.pos` back to
                        // the identifier, which left the parser stuck —
                        // subsequent iterations would re-see the same ident
                        // and `expect(LBrace)` in the caller would fail.
                        // Keep `self.pos` AFTER the ident so the next comma
                        // (if any) or `{` is observed correctly.
                        constraints.push(next_name);
                    }
                } else {
                    // Not an identifier after comma — backtrack
                    self.pos = saved;
                    break;
                }
            }
            clauses.push(WhereClause { type_param, constraints });
            // If the next token is an identifier (not 'where' again), it's another clause
            if let TokenKind::Identifier(ref w) = self.peek().clone() {
                if w == "where" || w == "{" {
                    break;
                }
                // Could be another clause like `U : Other`
                // Check: ident followed by colon?
                let saved = self.pos;
                self.advance();
                if self.check(&TokenKind::Colon) {
                    self.pos = saved; // backtrack, let the loop parse it
                    continue;
                } else {
                    self.pos = saved;
                    break;
                }
            } else {
                break;
            }
        }
        Ok(clauses)
    }

    fn parse_data_class(&mut self) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'data'
        self.expect(&TokenKind::Class)?;
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;

        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let mutability = if self.eat(&TokenKind::Val) {
                // val is default for data class fields
            } else {
                self.eat(&TokenKind::Var);
            };
            let _ = mutability; // not used yet

            let p = self.parse_param()?;
            fields.push(p);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RParen)?;

        // Issue #32: a `data class` declaration may carry a body
        // block of operator overloads, methods, computed properties,
        // and static constants. Parse it via the same `parse_members`
        // machinery used by `class` so the AST holds real `Member`
        // nodes that the lowering pipeline can emit.
        self.skip_newlines();
        let members = if self.eat(&TokenKind::LBrace) {
            let ms = self.parse_members()?;
            self.skip_newlines();
            self.expect(&TokenKind::RBrace)?;
            ms
        } else {
            Vec::new()
        };

        Ok(Decl::DataClass {
            name,
            name_span,
            fields,
            members,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_enum(&mut self) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'enum'
        let (name, name_span) = self.expect_ident()?;

        // Optional enum params: enum Weapon(val damage: Int)
        let params = if self.eat(&TokenKind::LParen) {
            let mut ps = Vec::new();
            self.skip_newlines();
            while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                self.eat(&TokenKind::Val); // consume optional 'val'
                let (pname, _) = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                let ty = self.parse_type()?;
                ps.push(EnumParam { name: pname, ty });
                self.skip_newlines();
                if !self.eat(&TokenKind::Comma) { break; }
                self.skip_newlines();
            }
            self.expect(&TokenKind::RParen)?;
            ps
        } else {
            vec![]
        };

        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();

        let mut entries = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let entry_span = self.peek_span();
            let (ename, entry_name_span) = self.expect_ident()?;
            // Issue #35: disambiguate between positional constructor
            // args for a shared-param enum (`Sword(10, 1.5)`) and the
            // Rust-style sum-type payload form (`Ok(value: Int)`).
            // Peek two tokens ahead: if we see `<ident>:` right after
            // the `(`, switch to the payload parser.
            let (args, payload) = if self.eat(&TokenKind::LParen) {
                self.skip_newlines();
                let is_payload_form = matches!(self.peek(), TokenKind::Identifier(_))
                    && matches!(self.peek_at(1), Some(TokenKind::Colon));
                if is_payload_form {
                    let mut fields = Vec::new();
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        let field_start = self.peek_span();
                        let (fname, _) = self.expect_ident()?;
                        self.expect(&TokenKind::Colon)?;
                        let fty = self.parse_type()?;
                        fields.push(EnumPayloadField {
                            name: fname,
                            ty: fty,
                            span: Span { start: field_start.start, end: self.peek_span().end },
                        });
                        self.skip_newlines();
                        if !self.eat(&TokenKind::Comma) { break; }
                        self.skip_newlines();
                    }
                    self.expect(&TokenKind::RParen)?;
                    (Vec::new(), fields)
                } else {
                    let mut a = Vec::new();
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        a.push(self.parse_expr()?);
                        if !self.eat(&TokenKind::Comma) { break; }
                    }
                    self.expect(&TokenKind::RParen)?;
                    (a, Vec::new())
                }
            } else {
                (vec![], vec![])
            };
            entries.push(EnumEntry {
                name: ename,
                name_span: entry_name_span,
                args,
                payload,
                span: Span { start: entry_span.start, end: self.peek_span().end },
            });
            self.skip_newlines();
            self.eat(&TokenKind::Comma);
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;

        Ok(Decl::Enum {
            name,
            name_span,
            params,
            entries,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_attribute_decl(&mut self, annotations: Vec<Annotation>) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'attribute'
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            self.eat(&TokenKind::Val);
            let p = self.parse_param()?;
            fields.push(p);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) { break; }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RParen)?;

        // Extract @targets(...) from annotations
        let mut targets = Vec::new();
        for ann in &annotations {
            if ann.name == "targets" {
                for arg in &ann.args {
                    if let Expr::Ident(name, _) = arg {
                        targets.push(name.clone());
                    }
                }
            }
        }

        Ok(Decl::Attribute {
            name,
            name_span,
            fields,
            targets,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    // ── Member parsing ───────────────────────────────────────────

    fn parse_members(&mut self) -> Result<Vec<Member>, ParseError> {
        let mut members = Vec::new();
        self.skip_newlines();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            match self.parse_member() {
                Ok(m) => members.push(m),
                Err(e) => {
                    self.errors.push(e);
                    self.recover_to_newline();
                }
            }
            self.skip_newlines();
        }

        Ok(members)
    }

    fn parse_member(&mut self) -> Result<Member, ParseError> {
        // Collect annotations (including v5 attribute targets like @field/@property/...)
        let (annotations, target_annotations) = self.parse_annotations_and_targets()?;

        // Contextual `event` keyword (Language 4): `event onDamaged: (Int) => Unit`
        if self.check_contextual("event") {
            return self.parse_event_member(Visibility::Public);
        }

        // Phase 5 contextual keywords: async / state machine / command / bind
        if self.check_contextual("async") {
            self.advance(); // consume 'async'
            self.expect(&TokenKind::Func)?;
            return self.parse_func_inner_async(Visibility::Public);
        }
        if self.check_contextual("state") && self.peek_ahead_is_contextual(1, "machine") {
            return self.parse_state_machine();
        }
        if self.check_contextual("command") {
            return self.parse_command_member();
        }
        if self.check_contextual("bind") {
            return self.parse_bind_member();
        }

        // v5 (deferred): generalized nested declarations. A `class`,
        // `data class`, `enum`, `struct`, `interface`, or even another
        // `component` token in member position is parsed as a nested
        // declaration via `parse_decl`. The result is wrapped in
        // `Member::NestedDecl`.
        let nested_starts_decl = matches!(
            self.peek(),
            TokenKind::Class
                | TokenKind::Data
                | TokenKind::Enum
                | TokenKind::Struct
                | TokenKind::Interface
                | TokenKind::TypeAlias
        ) || self.check_contextual("partial");
        if nested_starts_decl {
            let start = self.peek_span();
            let inner = self.parse_decl()?;
            return Ok(Member::NestedDecl {
                decl: Box::new(inner),
                span: Span { start: start.start, end: self.peek_span().end },
            });
        }

        match self.peek().clone() {
            TokenKind::Serialize => self.parse_serialize_field(annotations, target_annotations),
            TokenKind::Require => self.parse_require(),
            TokenKind::Optional => self.parse_optional(),
            TokenKind::Child => self.parse_child(),
            TokenKind::Parent => self.parse_parent(),
            TokenKind::Pool => self.parse_pool(),
            TokenKind::Func => self.parse_func_with_annotations(Visibility::Public, false, annotations),
            TokenKind::Coroutine => self.parse_coroutine(),
            TokenKind::Intrinsic => self.parse_intrinsic_member(),
            TokenKind::Override => {
                self.advance();
                self.expect(&TokenKind::Func)?;
                self.parse_func_inner(Visibility::Public, false, true)
            }
            TokenKind::Public | TokenKind::Private | TokenKind::Protected => {
                let vis = self.parse_visibility();
                if self.check_contextual("event") {
                    return self.parse_event_member(vis);
                }
                if self.check_contextual("async") {
                    self.advance(); // consume 'async'
                    self.expect(&TokenKind::Func)?;
                    return self.parse_func_inner_async(vis);
                }
                match self.peek().clone() {
                    TokenKind::Serialize => self.parse_serialize_field_with_vis(annotations, target_annotations, Some(vis)),
                    TokenKind::Func => self.parse_func_with_annotations(vis, false, annotations),
                    TokenKind::Override => {
                        self.advance();
                        self.expect(&TokenKind::Func)?;
                        self.parse_func_inner(vis, false, true)
                    }
                    // field: private rb: Rigidbody
                    TokenKind::Identifier(name) if name != "async" => self.parse_field(vis),
                    TokenKind::Val | TokenKind::Var | TokenKind::Const | TokenKind::Fixed => self.parse_val_var_field_or_property_with_targets(vis, false, target_annotations),
                    _ => Err(self.error(format!("Expected member after visibility, found {:?}", self.peek()))),
                }
            }
            TokenKind::Listen => {
                // Issue #60: member-position `listen event [until X] { ... }`.
                // Parse with the existing statement parser and repackage
                // the result as a `Member::ListenDecl`. The lifecycle
                // modifier is preserved so `until disable` / `until destroy`
                // behave the same as the body-level form.
                let listen_stmt = self.parse_listen_stmt()?;
                if let Stmt::Listen { event, params, lifetime, body, span, .. } = listen_stmt {
                    return Ok(Member::ListenDecl {
                        event,
                        params,
                        lifetime,
                        body,
                        span,
                    });
                }
                return Err(self.error(
                    "internal: parse_listen_stmt returned a non-Listen statement".into()
                ));
            }
            TokenKind::Static => {
                self.advance(); // consume 'static'
                match self.peek().clone() {
                    TokenKind::Func => self.parse_func_with_static(Visibility::Public, false, true),
                    TokenKind::Operator => self.parse_operator_member(),
                    TokenKind::Val | TokenKind::Var | TokenKind::Const | TokenKind::Fixed => self.parse_val_var_field_with_static(Visibility::Public, true),
                    _ => Err(self.error(format!("Expected 'func', 'val', or 'var' after 'static', found {:?}", self.peek()))),
                }
            }
            TokenKind::Abstract => {
                self.advance(); // consume 'abstract'
                self.expect(&TokenKind::Func)?;
                self.parse_func_inner_ext(Visibility::Public, false, false, true, false, false, vec![])
            }
            TokenKind::Open => {
                self.advance(); // consume 'open'
                self.expect(&TokenKind::Func)?;
                self.parse_func_inner_ext(Visibility::Public, false, false, false, true, false, vec![])
            }
            TokenKind::Operator => self.parse_operator_member(),
            TokenKind::Val | TokenKind::Var | TokenKind::Const | TokenKind::Fixed => self.parse_val_var_field_or_property_with_targets(Visibility::Public, false, target_annotations),
            // Lifecycle blocks
            TokenKind::Awake | TokenKind::Update | TokenKind::FixedUpdate
            | TokenKind::LateUpdate | TokenKind::OnEnable | TokenKind::OnDisable
            | TokenKind::OnDestroy | TokenKind::OnTriggerEnter | TokenKind::OnTriggerExit
            | TokenKind::OnTriggerStay | TokenKind::OnCollisionEnter
            | TokenKind::OnCollisionExit | TokenKind::OnCollisionStay => {
                self.parse_lifecycle()
            }
            // "start" as lifecycle (context-sensitive)
            TokenKind::Start if self.is_lifecycle_start() => {
                self.parse_lifecycle()
            }
            _ => Err(self.error(format!("Expected member declaration, found {:?}", self.peek()))),
        }
    }

    fn is_lifecycle_start(&self) -> bool {
        // "start" followed by "{" is a lifecycle block
        // "start" followed by identifier is "start coroutine()"
        let next_pos = self.pos + 1;
        // skip newlines
        let mut p = next_pos;
        while p < self.tokens.len() && self.tokens[p].kind == TokenKind::Newline {
            p += 1;
        }
        if p < self.tokens.len() {
            matches!(self.tokens[p].kind, TokenKind::LBrace)
        } else {
            false
        }
    }

    fn parse_annotations(&mut self) -> Result<Vec<Annotation>, ParseError> {
        let (annotations, _targets) = self.parse_annotations_and_targets()?;
        Ok(annotations)
    }

    /// Parse leading `@annotation(...)` metadata and split it into plain
    /// annotations and attribute-target annotations (v5 Sprint 1 feature 2).
    ///
    /// Target annotations are `@field`, `@property`, `@param`, `@return`,
    /// and `@type`. Their first argument must be an identifier naming the
    /// C# attribute to emit (e.g. `SerializeField`). Remaining arguments are
    /// literal exprs forwarded to the attribute invocation.
    fn parse_annotations_and_targets(
        &mut self,
    ) -> Result<(Vec<Annotation>, Vec<TargetAnnotation>), ParseError> {
        let mut annotations = Vec::new();
        let mut targets = Vec::new();
        while self.check(&TokenKind::At) {
            let start = self.peek_span();
            self.advance(); // consume '@'
            // Issue #19: an attribute target name may be a PrSM keyword
            // (`@return(notNull)`, `@type(...)`). The previous parser used
            // `expect_ident` which only accepted Identifier tokens, so the
            // `return` keyword token was rejected and the second member's
            // declaration was misparsed as a top-level decl.
            let (name, _) = self.expect_ident_or_keyword()?;
            // Issue #55: annotation arguments accept the same `name = expr`
            // and `name: expr` named forms as call arguments. We parse each
            // element as a `parse_call_arg`, which uses the same precedence
            // rules as function calls, and store the name alongside the
            // positional expression.
            let (args, arg_names) = if self.eat(&TokenKind::LParen) {
                let mut exprs = Vec::new();
                let mut names: Vec<Option<String>> = Vec::new();
                self.skip_newlines();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    let arg = self.parse_call_arg()?;
                    names.push(arg.name);
                    exprs.push(arg.value);
                    self.skip_newlines();
                    if !self.eat(&TokenKind::Comma) { break; }
                    self.skip_newlines();
                }
                self.skip_newlines();
                self.expect(&TokenKind::RParen)?;
                (exprs, names)
            } else {
                (vec![], vec![])
            };
            let ann_span = Span { start: start.start, end: self.peek_span().end };

            let target_kind = match name.as_str() {
                "field" => Some(AttrTargetKind::Field),
                "property" => Some(AttrTargetKind::Property),
                "param" => Some(AttrTargetKind::Param),
                "return" => Some(AttrTargetKind::Return),
                "type" => Some(AttrTargetKind::Type),
                _ => None,
            };

            if let Some(target) = target_kind {
                // Expect an identifier as the first argument (the attribute name).
                let (attr_name, rest) = extract_target_attr_name(&args);
                if let Some(attr_name) = attr_name {
                    targets.push(TargetAnnotation {
                        target,
                        attr_name,
                        args: rest,
                        span: ann_span,
                    });
                } else {
                    self.errors.push(ParseError {
                        message: format!(
                            "@{}(...) requires an identifier naming a C# attribute as its first argument",
                            name
                        ),
                        span: ann_span,
                    });
                }
            } else {
                annotations.push(Annotation { name, args, arg_names, span: ann_span });
            }
            self.skip_newlines();
        }
        Ok((annotations, targets))
    }

    fn parse_serialize_field(
        &mut self,
        annotations: Vec<Annotation>,
        target_annotations: Vec<TargetAnnotation>,
    ) -> Result<Member, ParseError> {
        self.parse_serialize_field_with_vis(annotations, target_annotations, None)
    }

    fn parse_serialize_field_with_vis(
        &mut self,
        annotations: Vec<Annotation>,
        target_annotations: Vec<TargetAnnotation>,
        visibility: Option<Visibility>,
    ) -> Result<Member, ParseError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Serialize)?;
        // Optional val/var after serialize
        let mutability = if self.eat(&TokenKind::Var) {
            Mutability::Var
        } else {
            self.eat(&TokenKind::Val); // optional, default = val
            Mutability::Val
        };
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let init = if self.eat(&TokenKind::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        // v5 Sprint 1: detect `serialize var hp: Int [= init] get [set]` form,
        // which is sugar for `[field: SerializeField] var hp: Int { get; set; }`.
        // The presence of `get` (or `set`) on the next non-newline token routes
        // us into the property accessors path with `is_serialize: true`.
        let saved = self.pos;
        self.skip_newlines();
        if self.check_contextual("get") || self.check_contextual("set") {
            return self.parse_property_accessors_full(
                start,
                mutability,
                name,
                name_span,
                ty,
                init,
                /*is_serialize*/ true,
                target_annotations,
            );
        }
        self.pos = saved;

        self.expect_newline_or_eof();
        // No accessors → fall back to the existing SerializeField member.
        // Target annotations on a SerializeField are not currently expressed
        // in the AST; they are silently dropped here. (E149 covers most
        // misuse cases at the property entry point.)
        let _ = target_annotations;
        Ok(Member::SerializeField {
            annotations,
            visibility,
            mutability,
            name,
            name_span,
            ty,
            init,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_require(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'require'
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        self.expect_newline_or_eof();
        Ok(Member::Require { name, name_span, ty, span: Span { start: start.start, end: self.peek_span().end } })
    }

    fn parse_optional(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        self.advance();
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        self.expect_newline_or_eof();
        Ok(Member::Optional { name, name_span, ty, span: Span { start: start.start, end: self.peek_span().end } })
    }

    fn parse_child(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        self.advance();
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        self.expect_newline_or_eof();
        Ok(Member::Child { name, name_span, ty, span: Span { start: start.start, end: self.peek_span().end } })
    }

    fn parse_parent(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        self.advance();
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        self.expect_newline_or_eof();
        Ok(Member::Parent { name, name_span, ty, span: Span { start: start.start, end: self.peek_span().end } })
    }

    fn parse_pool(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'pool'
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let item_type = self.parse_type()?;
        self.expect(&TokenKind::LParen)?;

        // Parse named arguments: capacity = N, max = M
        let mut capacity: Option<u32> = None;
        let mut max_size: Option<u32> = None;

        loop {
            if self.peek() == &TokenKind::RParen {
                break;
            }
            let (arg_name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Eq)?;
            if let TokenKind::IntLiteral(n) = self.peek().clone() {
                self.advance();
                match arg_name.as_str() {
                    "capacity" => capacity = Some(n as u32),
                    "max" => max_size = Some(n as u32),
                    _ => return Err(self.error(format!("Unknown pool argument '{}', expected 'capacity' or 'max'", arg_name))),
                }
            } else {
                return Err(self.error(format!("Expected integer literal for pool argument '{}', found {:?}", arg_name, self.peek())));
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }

        self.expect(&TokenKind::RParen)?;
        self.expect_newline_or_eof();

        let capacity = capacity.ok_or_else(|| self.error("Pool declaration missing 'capacity' argument".into()))?;
        let max_size = max_size.ok_or_else(|| self.error("Pool declaration missing 'max' argument".into()))?;

        Ok(Member::Pool {
            name,
            name_span,
            item_type,
            capacity,
            max_size,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Parse `event NAME : FuncType` member declaration (Language 4).
    fn parse_event_member(&mut self, visibility: Visibility) -> Result<Member, ParseError> {
        let start = self.peek_span();
        // Consume the contextual `event` identifier.
        let _ = self.eat_contextual("event");
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        self.expect_newline_or_eof();
        Ok(Member::Event {
            visibility,
            name,
            name_span,
            ty,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    // ── Phase 5: state machine member ───────────────────────────

    /// Parse `state machine Name { state S { enter { } exit { } on ev => T } }`.
    fn parse_state_machine(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        let _ = self.eat_contextual("state")
            .ok_or_else(|| self.error("Expected 'state'".into()))?;
        self.skip_newlines();
        let _ = self.eat_contextual("machine")
            .ok_or_else(|| self.error("Expected 'machine' after 'state'".into()))?;
        self.skip_newlines();
        let (name, name_span) = self.expect_ident()?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();

        let mut states = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let state = self.parse_state_decl()?;
            states.push(state);
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;
        self.expect_newline_or_eof();

        Ok(Member::StateMachine {
            name,
            name_span,
            states,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Parse a single `state Name { ... }` entry.
    fn parse_state_decl(&mut self) -> Result<StateDecl, ParseError> {
        let start = self.peek_span();
        let _ = self.eat_contextual("state")
            .ok_or_else(|| self.error("Expected 'state'".into()))?;
        // Issue #38: state names may shadow keyword tokens (e.g.
        // `state Open { ... }` — `Open` would lex as TokenKind::Open
        // when the source happens to spell the name the same as a
        // keyword, which is the canonical Door/traffic-light example).
        let (name, name_span) = self.expect_ident_or_keyword()?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();

        let mut enter: Option<Block> = None;
        let mut exit: Option<Block> = None;
        let mut transitions: Vec<StateTransition> = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if self.check_contextual("enter") {
                // Issue #98: reject duplicate `enter { }` blocks in
                // the same state. Previously the second block
                // silently overwrote the first, losing user code
                // with no diagnostic.
                let enter_span = self.peek_span();
                self.advance(); // consume 'enter'
                self.skip_newlines();
                let block = self.parse_block()?;
                if enter.is_some() {
                    return Err(ParseError {
                        message: format!(
                            "E212: duplicate 'enter' block in state '{}'. A state may declare at most one 'enter' block.",
                            name
                        ),
                        span: enter_span,
                    });
                }
                enter = Some(block);
                self.skip_newlines();
            } else if self.check_contextual("exit") {
                let exit_span = self.peek_span();
                self.advance(); // consume 'exit'
                self.skip_newlines();
                let block = self.parse_block()?;
                if exit.is_some() {
                    return Err(ParseError {
                        message: format!(
                            "E212: duplicate 'exit' block in state '{}'. A state may declare at most one 'exit' block.",
                            name
                        ),
                        span: exit_span,
                    });
                }
                exit = Some(block);
                self.skip_newlines();
            } else if self.check_contextual("on") {
                let t_start = self.peek_span();
                self.advance(); // consume 'on'
                // Issue #38: state-machine event names may collide
                // with keyword tokens (`open`, `close`, `lock`,
                // `unlock`, etc.). Accept keyword tokens as event
                // names by using the keyword-tolerant variant.
                let (event, event_span) = self.expect_ident_or_keyword()?;
                self.expect(&TokenKind::FatArrow)?;
                // Target state names may also collide with keywords
                // (e.g. `=> Open` would normally lex as `TokenKind::Open`
                // if the user spells the state lowercased). Allow
                // either an identifier or a keyword token.
                let (target, target_span) = self.expect_ident_or_keyword()?;
                self.expect_newline_or_eof();
                transitions.push(StateTransition {
                    event,
                    event_span,
                    target,
                    target_span,
                    span: Span { start: t_start.start, end: self.peek_span().end },
                });
                self.skip_newlines();
            } else {
                return Err(self.error(format!(
                    "Expected 'enter', 'exit', or 'on' inside state '{}', found {:?}",
                    name,
                    self.peek()
                )));
            }
        }
        self.expect(&TokenKind::RBrace)?;
        self.expect_newline_or_eof();

        Ok(StateDecl {
            name,
            name_span,
            enter,
            exit,
            transitions,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    // ── Phase 5: command member ─────────────────────────────────

    /// Parse `command Name(params) { stmts } [undo { stmts }] [canExecute = expr]`.
    fn parse_command_member(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        let _ = self.eat_contextual("command")
            .ok_or_else(|| self.error("Expected 'command'".into()))?;
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = if self.check(&TokenKind::RParen) {
            Vec::new()
        } else {
            self.parse_param_list()?
        };
        self.expect(&TokenKind::RParen)?;
        self.skip_newlines();
        let execute = self.parse_block()?;
        self.skip_newlines();

        let undo = if self.check_contextual("undo") {
            self.advance(); // consume 'undo'
            self.skip_newlines();
            let block = self.parse_block()?;
            self.skip_newlines();
            Some(block)
        } else {
            None
        };

        let can_execute = if self.check_contextual("canExecute") {
            self.advance(); // consume 'canExecute'
            self.expect(&TokenKind::Eq)?;
            let expr = self.parse_expr()?;
            self.expect_newline_or_eof();
            Some(expr)
        } else {
            None
        };

        Ok(Member::Command {
            name,
            name_span,
            params,
            execute,
            undo,
            can_execute,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    // ── Phase 5: bind member ────────────────────────────────────

    /// Parse `bind name: Type [= init]` member declaration (reactive property).
    fn parse_bind_member(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        let _ = self.eat_contextual("bind")
            .ok_or_else(|| self.error("Expected 'bind'".into()))?;
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let init = if self.eat(&TokenKind::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect_newline_or_eof();
        Ok(Member::BindProperty {
            name,
            name_span,
            ty,
            init,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_func(&mut self, vis: Visibility, is_override: bool) -> Result<Member, ParseError> {
        self.parse_func_with_annotations(vis, is_override, vec![])
    }

    fn parse_func_with_annotations(
        &mut self,
        vis: Visibility,
        is_override: bool,
        annotations: Vec<Annotation>,
    ) -> Result<Member, ParseError> {
        self.advance(); // consume 'func'
        self.parse_func_inner_ext(vis, false, is_override, false, false, false, annotations)
    }

    fn parse_func_with_static(&mut self, vis: Visibility, is_override: bool, is_static: bool) -> Result<Member, ParseError> {
        self.advance(); // consume 'func'
        self.parse_func_inner_ext(vis, is_static, is_override, false, false, false, vec![])
    }

    fn parse_func_inner(&mut self, vis: Visibility, is_static: bool, is_override: bool) -> Result<Member, ParseError> {
        self.parse_func_inner_ext(vis, is_static, is_override, false, false, false, vec![])
    }

    /// Convenience: parse `func` after a contextual `async` prefix has already been consumed.
    fn parse_func_inner_async(&mut self, vis: Visibility) -> Result<Member, ParseError> {
        self.parse_func_inner_ext(vis, false, false, false, false, true, vec![])
    }

    fn parse_func_inner_ext(&mut self, vis: Visibility, is_static: bool, is_override: bool, is_abstract: bool, is_open: bool, is_async: bool, annotations: Vec<Annotation>) -> Result<Member, ParseError> {
        let start = self.peek_span();
        let (name, name_span) = self.expect_ident()?;

        // Optional type parameters: <T, U>
        let type_params = if self.check(&TokenKind::Lt) {
            self.parse_type_params()?
        } else {
            Vec::new()
        };

        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;

        let return_ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Optional where clauses before the body
        let where_clauses = self.parse_where_clauses()?;

        // Abstract functions have no body — just a signature
        let body = if is_abstract {
            self.expect_newline_or_eof();
            FuncBody::Block(Block { stmts: vec![], span: self.peek_span() })
        } else if self.eat(&TokenKind::Eq) {
            // Expression body
            let expr = self.parse_expr()?;
            self.expect_newline_or_eof();
            FuncBody::ExprBody(expr)
        } else {
            self.skip_newlines();
            FuncBody::Block(self.parse_block()?)
        };

        Ok(Member::Func {
            visibility: vis,
            is_static,
            is_override,
            is_abstract,
            is_open,
            is_operator: false,
            is_async,
            annotations,
            name,
            name_span,
            type_params,
            where_clauses,
            params,
            return_ty,
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_coroutine(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'coroutine'
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        // v5 Sprint 1: optional return type — `coroutine countdown(): Seq<Int>`.
        // Used by yield value type checking and for choosing the lowered
        // C# return type (`IEnumerator` vs `IEnumerator<T>`).
        let return_ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Member::Coroutine {
            name,
            name_span,
            params,
            return_ty,
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_lifecycle(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        let kind = self.parse_lifecycle_kind()?;

        let params = if self.eat(&TokenKind::LParen) {
            let ps = self.parse_param_list()?;
            self.expect(&TokenKind::RParen)?;
            ps
        } else {
            vec![]
        };

        self.skip_newlines();
        let body = self.parse_block()?;

        Ok(Member::Lifecycle {
            kind,
            params,
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_lifecycle_kind(&mut self) -> Result<LifecycleKind, ParseError> {
        let kind = match self.peek() {
            TokenKind::Awake => LifecycleKind::Awake,
            TokenKind::Start => LifecycleKind::Start,
            TokenKind::Update => LifecycleKind::Update,
            TokenKind::FixedUpdate => LifecycleKind::FixedUpdate,
            TokenKind::LateUpdate => LifecycleKind::LateUpdate,
            TokenKind::OnEnable => LifecycleKind::OnEnable,
            TokenKind::OnDisable => LifecycleKind::OnDisable,
            TokenKind::OnDestroy => LifecycleKind::OnDestroy,
            TokenKind::OnTriggerEnter => LifecycleKind::OnTriggerEnter,
            TokenKind::OnTriggerExit => LifecycleKind::OnTriggerExit,
            TokenKind::OnTriggerStay => LifecycleKind::OnTriggerStay,
            TokenKind::OnCollisionEnter => LifecycleKind::OnCollisionEnter,
            TokenKind::OnCollisionExit => LifecycleKind::OnCollisionExit,
            TokenKind::OnCollisionStay => LifecycleKind::OnCollisionStay,
            _ => return Err(self.error(format!("Expected lifecycle keyword, found {:?}", self.peek()))),
        };
        self.advance();
        Ok(kind)
    }

    fn parse_intrinsic_member(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'intrinsic'

        match self.peek().clone() {
            TokenKind::Func => {
                self.advance();
                let vis = Visibility::Public;
                let (name, name_span) = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                let params = self.parse_param_list()?;
                self.expect(&TokenKind::RParen)?;
                let return_ty = if self.eat(&TokenKind::Colon) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                self.skip_newlines();
                let code = self.parse_raw_brace_block()?;
                Ok(Member::IntrinsicFunc {
                    visibility: vis,
                    name,
                    name_span,
                    params,
                    return_ty,
                    code,
                    span: Span { start: start.start, end: self.peek_span().end },
                })
            }
            TokenKind::Coroutine => {
                self.advance();
                let (name, name_span) = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                let params = self.parse_param_list()?;
                self.expect(&TokenKind::RParen)?;
                self.skip_newlines();
                let code = self.parse_raw_brace_block()?;
                Ok(Member::IntrinsicCoroutine {
                    name,
                    name_span,
                    params,
                    code,
                    span: Span { start: start.start, end: self.peek_span().end },
                })
            }
            _ => Err(self.error("Expected 'func' or 'coroutine' after 'intrinsic'".into())),
        }
    }

    fn parse_field(&mut self, vis: Visibility) -> Result<Member, ParseError> {
        // private name: Type = init
        let start = self.peek_span();
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let init = if self.eat(&TokenKind::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect_newline_or_eof();
        Ok(Member::Field {
            visibility: vis,
            is_static: false,
            mutability: Mutability::Var,
            name,
            name_span,
            ty: Some(ty),
            init,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_val_var_field(&mut self, vis: Visibility) -> Result<Member, ParseError> {
        self.parse_val_var_field_with_static(vis, false)
    }

    /// Parse val/var that may be a property (with get/set) or a plain field.
    fn parse_val_var_field_or_property(&mut self, vis: Visibility) -> Result<Member, ParseError> {
        self.parse_val_var_field_or_property_with_targets(vis, false, vec![])
    }

    fn parse_val_var_field_or_property_with_targets(
        &mut self,
        vis: Visibility,
        is_serialize: bool,
        target_annotations: Vec<TargetAnnotation>,
    ) -> Result<Member, ParseError> {
        let start = self.peek_span();
        let mutability = match self.peek() {
            TokenKind::Val => { self.advance(); Mutability::Val }
            TokenKind::Var => { self.advance(); Mutability::Var }
            TokenKind::Const => { self.advance(); Mutability::Const }
            TokenKind::Fixed => { self.advance(); Mutability::Fixed }
            _ => return Err(self.error("Expected val, var, const, or fixed".into())),
        };
        let (name, name_span) = self.expect_ident()?;
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Check if this is a property with get/set accessors.
        // After `val name: Type` or `var name: Type`, look ahead past newlines for `get` or `set`.
        if (mutability == Mutability::Val || mutability == Mutability::Var) && ty.is_some() {
            let saved = self.pos;
            // Save position, skip newlines, peek for get/set
            self.skip_newlines();
            if self.check_contextual("get") || self.check_contextual("set") {
                // This is a property declaration
                return self.parse_property_accessors_full(
                    start,
                    mutability,
                    name,
                    name_span,
                    ty.unwrap(),
                    None,
                    is_serialize,
                    target_annotations,
                );
            }
            // Not a property — restore position
            self.pos = saved;
        }

        let init = if self.eat(&TokenKind::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        // Issue #16: a property may also carry an explicit initializer
        // *and* `get`/`set` accessors:
        //
        //   var name: String = "Default"
        //       set(value) { field = value.trim() }
        //
        // After consuming the init, look one more time for an accessor
        // continuation. Without this check the field is closed off as a
        // plain Member::Field and the trailing `get`/`set` lines are
        // misparsed as a new top-level declaration (E189).
        if (mutability == Mutability::Val || mutability == Mutability::Var) && ty.is_some() {
            let saved = self.pos;
            self.skip_newlines();
            if self.check_contextual("get") || self.check_contextual("set") {
                return self.parse_property_accessors_full(
                    start,
                    mutability,
                    name,
                    name_span,
                    ty.unwrap(),
                    init,
                    is_serialize,
                    target_annotations,
                );
            }
            self.pos = saved;
        }
        self.expect_newline_or_eof();
        // Target annotations on a plain field have no current AST representation.
        let _ = target_annotations;
        Ok(Member::Field {
            visibility: vis,
            is_static: false,
            mutability,
            name,
            name_span,
            ty,
            init,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_val_var_field_with_static(&mut self, vis: Visibility, is_static: bool) -> Result<Member, ParseError> {
        let start = self.peek_span();
        let mutability = if self.eat(&TokenKind::Val) {
            Mutability::Val
        } else if self.eat(&TokenKind::Var) {
            Mutability::Var
        } else if self.eat(&TokenKind::Const) {
            Mutability::Const
        } else {
            self.expect(&TokenKind::Fixed)?;
            Mutability::Fixed
        };
        let (name, name_span) = self.expect_ident()?;
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let init = if self.eat(&TokenKind::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect_newline_or_eof();
        Ok(Member::Field {
            visibility: vis,
            is_static,
            mutability,
            name,
            name_span,
            ty,
            init,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_visibility(&mut self) -> Visibility {
        match self.peek() {
            TokenKind::Public => { self.advance(); Visibility::Public }
            TokenKind::Private => { self.advance(); Visibility::Private }
            TokenKind::Protected => { self.advance(); Visibility::Protected }
            _ => Visibility::Public,
        }
    }

    // ── Property accessor parsing ────────────────────────────────

    /// Parse the get/set accessors after `val name: Type` or `var name: Type`.
    fn parse_property_accessors(
        &mut self,
        start: Span,
        mutability: Mutability,
        name: String,
        name_span: Span,
        ty: TypeRef,
    ) -> Result<Member, ParseError> {
        self.parse_property_accessors_full(
            start, mutability, name, name_span, ty, None, false, vec![],
        )
    }

    /// v5 Sprint 1 extended form: also accepts an optional inline `init` expr
    /// (parsed before the accessors as `var name: Type = init get set`),
    /// the `is_serialize` flag for the `serialize` modifier, and any
    /// `@field/@property/@return/...` target annotations attached to the
    /// declaration. The `init` expression is currently dropped because the
    /// auto-property lowering does not yet model field initializers; this
    /// keeps backward compatibility with v4 Member::Property.
    fn parse_property_accessors_full(
        &mut self,
        start: Span,
        mutability: Mutability,
        name: String,
        name_span: Span,
        ty: TypeRef,
        _init: Option<Expr>,
        is_serialize: bool,
        target_annotations: Vec<TargetAnnotation>,
    ) -> Result<Member, ParseError> {
        let mut getter: Option<FuncBody> = None;
        let mut setter: Option<PropertySetter> = None;

        // Parse `get` and/or `set` in any order. Each accessor may be:
        //   • a bare keyword `get` / `set` (auto-property accessor)
        //   • `get = expr` or `set = expr` (expression-bodied)
        //   • `get { … }` or `set(value) { … }` (block body)
        loop {
            self.skip_newlines();
            if self.check_contextual("get") && getter.is_none() {
                self.advance(); // consume 'get'
                if self.eat(&TokenKind::Eq) {
                    // get = expr
                    let expr = self.parse_expr()?;
                    getter = Some(FuncBody::ExprBody(expr));
                } else if self.check(&TokenKind::LBrace) || matches!(self.peek_after_newlines(), TokenKind::LBrace) {
                    self.skip_newlines();
                    let block = self.parse_block()?;
                    getter = Some(FuncBody::Block(block));
                } else {
                    // Bare `get` accessor (auto-property): use an empty block as
                    // the marker. The lowering pass detects empty bodies and
                    // emits a `{ get; set; }` form.
                    getter = Some(FuncBody::Block(Block { stmts: vec![], span: name_span }));
                }
            } else if self.check_contextual("set") && setter.is_none() {
                self.advance(); // consume 'set'
                if self.eat(&TokenKind::LParen) {
                    let (param_name, _) = self.expect_ident()?;
                    self.expect(&TokenKind::RParen)?;
                    self.skip_newlines();
                    let block = self.parse_block()?;
                    setter = Some(PropertySetter { param_name, body: block });
                } else if self.check(&TokenKind::LBrace) || matches!(self.peek_after_newlines(), TokenKind::LBrace) {
                    self.skip_newlines();
                    let block = self.parse_block()?;
                    setter = Some(PropertySetter { param_name: "value".into(), body: block });
                } else {
                    // Bare `set` accessor (auto-property): use empty body marker.
                    setter = Some(PropertySetter {
                        param_name: "value".into(),
                        body: Block { stmts: vec![], span: name_span },
                    });
                }
            } else {
                break;
            }
        }

        Ok(Member::Property {
            mutability,
            name,
            name_span,
            ty,
            getter,
            setter,
            is_serialize,
            target_annotations,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Look at the next non-newline token without advancing.
    fn peek_after_newlines(&self) -> TokenKind {
        let mut p = self.pos;
        while p < self.tokens.len() && self.tokens[p].kind == TokenKind::Newline {
            p += 1;
        }
        self.tokens.get(p).map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof)
    }

    // ── Operator member parsing ─────────────────────────────────

    /// Parse `operator get(...)`, `operator set(...)`, `operator plus(...)`, etc.
    fn parse_operator_member(&mut self) -> Result<Member, ParseError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Operator)?;

        // The operator name: get, set, plus, minus, times, div, mod, equals, compareTo
        let (op_name, op_name_span) = self.expect_ident()?;

        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;

        let return_ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = if self.eat(&TokenKind::Eq) {
            let expr = self.parse_expr()?;
            self.expect_newline_or_eof();
            FuncBody::ExprBody(expr)
        } else {
            self.skip_newlines();
            FuncBody::Block(self.parse_block()?)
        };

        Ok(Member::Func {
            visibility: Visibility::Public,
            is_static: false,
            is_override: false,
            is_abstract: false,
            is_open: false,
            is_operator: true,
            is_async: false,
            annotations: vec![],
            name: op_name,
            name_span: op_name_span,
            type_params: vec![],
            where_clauses: vec![],
            params,
            return_ty,
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    // ── Extension declaration parsing ───────────────────────────

    fn parse_extension(&mut self) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'extend'
        let target_type = self.parse_type()?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        let members = self.parse_members()?;
        self.expect(&TokenKind::RBrace)?;
        Ok(Decl::Extension {
            target_type,
            members,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    // ── Block & Statement parsing ────────────────────────────────

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self.peek_span();
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();

        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            match self.parse_stmt() {
                Ok(s) => stmts.push(s),
                Err(e) => {
                    self.errors.push(e);
                    self.recover_to_newline();
                }
            }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Block { stmts, span: Span { start: start.start, end: self.peek_span().end } })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.skip_newlines();
        let start = self.peek_span();

        // Contextual `use` keyword (Language 4) — IDisposable resource management.
        // Two forms: `use val name = expr` (declaration) or `use name = expr { body }` (block).
        if self.check_contextual("use") {
            return self.parse_use_stmt();
        }

        // Phase 5: `bind source to target` — declarative push binding statement.
        // Parsed here so that `bind x to y` inside lifecycle blocks is a statement,
        // not interpreted as a call to a function named `bind`.
        if self.check_contextual("bind") && self.peek_ahead_is_contextual(2, "to") {
            return self.parse_bind_to_stmt();
        }

        // Language 5, Sprint 1: `yield expr` and `yield break`.
        // `yield` is a contextual keyword — it must not break existing
        // identifiers, so we look it up by text rather than as a token kind.
        if self.check_contextual("yield") {
            return self.parse_yield_stmt();
        }

        // Language 5, Sprint 1: `#if` preprocessor block.
        if self.check(&TokenKind::HashIf) {
            return self.parse_preprocessor_stmt();
        }

        match self.peek().clone() {
            TokenKind::Val | TokenKind::Const | TokenKind::Fixed => self.parse_val_stmt(),
            TokenKind::Var => self.parse_var_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::When => self.parse_when_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Wait => self.parse_wait_stmt(),
            TokenKind::Start => {
                self.advance();
                let call = self.parse_expr()?;
                self.expect_newline_or_eof();
                Ok(Stmt::Start { call, span: Span { start: start.start, end: self.peek_span().end } })
            }
            TokenKind::Stop => {
                self.advance();
                let target = self.parse_expr()?;
                self.expect_newline_or_eof();
                Ok(Stmt::Stop { target, span: Span { start: start.start, end: self.peek_span().end } })
            }
            TokenKind::StopAll => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                self.expect(&TokenKind::RParen)?;
                self.expect_newline_or_eof();
                Ok(Stmt::StopAll { span: Span { start: start.start, end: self.peek_span().end } })
            }
            TokenKind::Listen => self.parse_listen_stmt(),
            TokenKind::Unlisten => self.parse_unlisten_stmt(),
            TokenKind::Intrinsic => {
                self.advance();
                let code = self.parse_raw_brace_block()?;
                Ok(Stmt::IntrinsicBlock { code, span: Span { start: start.start, end: self.peek_span().end } })
            }
            TokenKind::Break => {
                self.advance();
                self.expect_newline_or_eof();
                Ok(Stmt::Break { span: start })
            }
            TokenKind::Continue => {
                self.advance();
                self.expect_newline_or_eof();
                Ok(Stmt::Continue { span: start })
            }
            TokenKind::Try => self.parse_try_stmt(),
            TokenKind::Throw => self.parse_throw_stmt(),
            _ => self.parse_expr_or_assignment_stmt(),
        }
    }

    fn parse_val_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'val'

        // v5 Sprint 3: `val ref name = ref expr` — reference local
        // declaration. We detect the optional `ref` modifier here and
        // require the init expression to be `ref expr` (Expr::RefOf).
        let is_ref = if self.check_contextual("ref") {
            self.advance();
            true
        } else {
            false
        };

        // v2: `val TypeName(a, b) = expr` — destructuring declaration.
        // Attempt speculative binding-pattern parse BEFORE consuming the name.
        if !is_ref {
        if let Some(bp) = self.try_parse_binding_pattern()? {
            self.expect(&TokenKind::Eq)?;
            let init = self.parse_expr()?;
            self.expect_newline_or_eof();
            let pattern = DestructurePattern {
                type_name: bp.path.join("."),
                bindings: bp.bindings,
                span: bp.span,
            };
            return Ok(Stmt::DestructureVal {
                pattern,
                init,
                span: Span { start: start.start, end: self.peek_span().end },
            });
        }

        // Issue #17: `val (a, b) = expr` — tuple destructuring. The
        // pattern is represented as a `DestructurePattern` with an
        // empty `type_name`, which the lowering pass turns into a C#
        // tuple deconstruction (`var (a, b) = expr;`). Bindings may be
        // identifiers or the discard placeholder `_`.
        if self.check(&TokenKind::LParen) {
            let pat_start = self.peek_span();
            let saved = self.pos;
            self.advance(); // consume '('
            let mut bindings: Vec<String> = Vec::new();
            let mut ok = true;
            loop {
                self.skip_newlines();
                if self.check_contextual("_") {
                    bindings.push("_".into());
                    self.advance();
                } else if let TokenKind::Identifier(name) = self.peek().clone() {
                    bindings.push(name);
                    self.advance();
                } else {
                    ok = false;
                    break;
                }
                self.skip_newlines();
                if !self.eat(&TokenKind::Comma) { break; }
            }
            if ok && self.eat(&TokenKind::RParen) && self.check(&TokenKind::Eq) {
                self.advance(); // consume '='
                let init = self.parse_expr()?;
                self.expect_newline_or_eof();
                return Ok(Stmt::DestructureVal {
                    pattern: DestructurePattern {
                        type_name: String::new(),
                        bindings,
                        span: Span {
                            start: pat_start.start,
                            end: self.peek_span().end,
                        },
                    },
                    init,
                    span: Span { start: start.start, end: self.peek_span().end },
                });
            }
            // Not a tuple destructure — restore for the regular path.
            self.pos = saved;
        }
        } // close `if !is_ref` for binding pattern guard

        // Issue #39: accept keyword tokens as the bound name so
        // `val data = ...`, `val class = ...` etc. parse. The
        // keyword's normal meaning is contextual (e.g. `data class
        // Foo`) and only matters at declaration sites.
        let (name, name_span) = self.expect_ident_or_keyword()?;
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&TokenKind::Eq)?;
        // Special case: `val token = listen event manual { … }`
        // `listen` is not a general expression, so we handle it here.
        if !is_ref && self.check(&TokenKind::Listen) {
            let mut listen = self.parse_listen_stmt()?;
            if let Stmt::Listen { ref mut lifetime, ref mut bound_name, .. } = listen {
                if *lifetime != ListenLifetime::Manual {
                    return Err(ParseError {
                        message: "val binding for listen requires the 'manual' lifetime modifier".into(),
                        span: start,
                    });
                }
                *bound_name = Some(name);
                // type annotation (if any) is discarded for manual listen
                let _ = ty;
            }
            return Ok(listen);
        }
        // For `val ref name = ref expr`, the parser produces `Expr::RefOf`
        // for the init by recognizing the leading `ref` keyword.
        let init = if is_ref {
            let ref_start = self.peek_span();
            if self.check_contextual("ref") {
                self.advance();
            } else {
                return Err(self.error(
                    "val ref binding must be initialized with `ref expr`".into()
                ));
            }
            let inner = self.parse_expr()?;
            Expr::RefOf {
                inner: Box::new(inner),
                span: Span { start: ref_start.start, end: self.peek_span().end },
            }
        } else {
            self.parse_expr()?
        };
        self.expect_newline_or_eof();
        Ok(Stmt::ValDecl {
            name,
            name_span,
            ty,
            init,
            is_ref,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_var_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'var'
        // v5 Sprint 3: optional `ref` modifier — `var ref name = ref expr`.
        let is_ref = if self.check_contextual("ref") {
            self.advance();
            true
        } else {
            false
        };
        // Issue #39: same keyword-as-identifier tolerance as `val`.
        let (name, name_span) = self.expect_ident_or_keyword()?;
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let init = if self.eat(&TokenKind::Eq) {
            // For `var ref name = ref expr`, wrap the inner expression
            // in `Expr::RefOf` so the lowering step can emit `ref expr`
            // verbatim.
            if is_ref {
                let ref_start = self.peek_span();
                if self.check_contextual("ref") {
                    self.advance();
                } else {
                    return Err(self.error(
                        "var ref binding must be initialized with `ref expr`".into()
                    ));
                }
                let inner = self.parse_expr()?;
                Some(Expr::RefOf {
                    inner: Box::new(inner),
                    span: Span { start: ref_start.start, end: self.peek_span().end },
                })
            } else {
                Some(self.parse_expr()?)
            }
        } else {
            None
        };
        self.expect_newline_or_eof();
        Ok(Stmt::VarDecl {
            name,
            name_span,
            ty,
            init,
            is_ref,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'if'
        // Issue #50: suppress trailing-lambda desugar inside the condition
        // so the `{ ... }` body is not misread as a lambda arg.
        let cond = self.with_no_trailing_lambda(|p| p.parse_expr())?;
        self.skip_newlines();
        let then_block = self.parse_block()?;

        let else_branch = if self.eat(&TokenKind::Else) {
            self.skip_newlines();
            if self.check(&TokenKind::If) {
                let elif = self.parse_if_stmt()?;
                Some(ElseBranch::ElseIf(Box::new(elif)))
            } else {
                let block = self.parse_block()?;
                Some(ElseBranch::ElseBlock(block))
            }
        } else {
            None
        };

        Ok(Stmt::If {
            cond,
            then_block,
            else_branch,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_when_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'when'

        let subject = if !self.check(&TokenKind::LBrace) && !self.check(&TokenKind::Newline) {
            // Issue #50: the `when` body `{` must not be consumed as a
            // trailing-lambda argument on the subject expression.
            Some(self.with_no_trailing_lambda(|p| p.parse_expr())?)
        } else {
            None
        };

        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();

        let mut branches = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let branch = self.parse_when_branch()?;
            branches.push(branch);
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;

        Ok(Stmt::When {
            subject,
            branches,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_when_branch(&mut self) -> Result<WhenBranch, ParseError> {
        let start = self.peek_span();

        // v2/v5: parse the first pattern, then optionally extend with `and`/`or`/comma combinators.
        let first_pattern = self.parse_pattern_atom()?;

        // v5 Sprint 4: `pattern and pattern` combinator. Parse iteratively
        // so chains like `> 0 and < 100 and != 50` left-associate naturally.
        // `and` binds tighter than `or`, mirroring boolean algebra.
        let mut combined = first_pattern;
        while self.check_contextual("and") {
            self.advance();
            let right = self.parse_pattern_atom()?;
            let span = Span { start: start.start, end: self.peek_span().end };
            combined = WhenPattern::And {
                left: Box::new(combined),
                right: Box::new(right),
                span,
            };
        }

        // Issue #52: pattern `or` combinator — `is Enemy or is Boss`,
        // `< 0 or > 1000`, etc. Collected into the existing `WhenPattern::Or`
        // variant (same shape as the comma form) so downstream switch
        // emission reuses the same path. We also re-enter the `and` loop
        // after each `or` so `a or b and c` parses as `a or (b and c)`.
        let mut or_patterns: Vec<WhenPattern> = Vec::new();
        while self.check_contextual("or") {
            self.advance();
            let mut next = self.parse_pattern_atom()?;
            while self.check_contextual("and") {
                self.advance();
                let right = self.parse_pattern_atom()?;
                let span = Span { start: start.start, end: self.peek_span().end };
                next = WhenPattern::And {
                    left: Box::new(next),
                    right: Box::new(right),
                    span,
                };
            }
            or_patterns.push(next);
        }
        let combined = if !or_patterns.is_empty() {
            let mut patterns = vec![combined];
            patterns.extend(or_patterns);
            WhenPattern::Or {
                span: Span { start: start.start, end: self.peek_span().end },
                patterns,
            }
        } else {
            combined
        };

        // v4: OR pattern — if we see ',' and then another pattern (not '=>'),
        // collect multiple patterns into a single Or wrapper.
        let pattern = if self.check(&TokenKind::Comma) && !matches!(combined, WhenPattern::Else | WhenPattern::Range { .. }) {
            let mut patterns = match combined {
                WhenPattern::Or { patterns, .. } => patterns,
                other => vec![other],
            };
            while self.eat(&TokenKind::Comma) {
                self.skip_newlines();
                let next = self.parse_pattern_atom()?;
                patterns.push(next);
            }
            WhenPattern::Or {
                span: Span { start: start.start, end: self.peek_span().end },
                patterns,
            }
        } else {
            combined
        };

        // v2: optional guard `if <cond>`
        let guard = if self.eat(&TokenKind::If) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(&TokenKind::FatArrow)?;

        let body = if self.check(&TokenKind::LBrace) {
            WhenBody::Block(self.parse_block()?)
        } else {
            let expr = self.parse_expr()?;
            self.skip_newlines();
            WhenBody::Expr(expr)
        };

        Ok(WhenBranch {
            pattern,
            guard,
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Parse a single `when` pattern atom (Language 5 update). Recognizes:
    ///
    /// * `else` — wildcard
    /// * `is Type` — type test
    /// * `in start..end` — range
    /// * `> n`, `< n`, `>= n`, `<= n`, `== n`, `!= n` — relational (v5)
    /// * `not pattern` — negation (v5)
    /// * `TypeName(bindings)` / `TypeName.Variant(bindings)` — binding
    /// * `Type(p1, p2)` with sub-patterns — positional (v5 deferred)
    /// * `Type { x: p1, y: p2 }` / `{ x: p1 }` — property (v5 deferred)
    /// * any other expression — equality match
    fn parse_pattern_atom(&mut self) -> Result<WhenPattern, ParseError> {
        if self.eat(&TokenKind::Else) {
            return Ok(WhenPattern::Else);
        }
        if self.eat(&TokenKind::Is) {
            let ty = self.parse_type()?;
            return Ok(WhenPattern::Is(ty));
        }
        if self.check(&TokenKind::In) {
            let range_start = self.peek_span();
            self.advance();
            let range_begin = self.parse_additive()?;
            self.expect(&TokenKind::DotDot)?;
            let range_end = self.parse_additive()?;
            return Ok(WhenPattern::Range {
                start: range_begin,
                end: range_end,
                span: Span { start: range_start.start, end: self.peek_span().end },
            });
        }
        // v5 Sprint 4: `not pattern` (right-associative).
        if self.check_contextual("not") {
            let start = self.peek_span();
            self.advance();
            let inner = self.parse_pattern_atom()?;
            return Ok(WhenPattern::Not {
                inner: Box::new(inner),
                span: Span { start: start.start, end: self.peek_span().end },
            });
        }
        // v5 Sprint 4: relational patterns. The leading operator is what
        // disambiguates them from a regular expression — only `<`, `>`,
        // `<=`, `>=`, `==`, `!=` qualify.
        if let Some(op) = self.peek_relational_op() {
            let span_start = self.peek_span();
            self.advance(); // consume operator
            let value = self.parse_additive()?;
            return Ok(WhenPattern::Relational {
                op,
                value,
                span: Span { start: span_start.start, end: self.peek_span().end },
            });
        }
        // v5 (deferred): bare property pattern `{ x: p1 }`.
        if self.check(&TokenKind::LBrace) {
            let start = self.peek_span();
            return self.parse_property_pattern_body(vec![], start);
        }
        // v5 (deferred): positional or property pattern with a leading
        // type path. We try the binding pattern first (it expects a
        // tighter shape — `Type(ident, ident)`), then fall back to a
        // generalized positional pattern that allows full sub-patterns,
        // then to a property pattern when followed by `{`.
        if let Some(bp) = self.try_parse_binding_pattern()? {
            return Ok(WhenPattern::Binding {
                path: bp.path,
                bindings: bp.bindings,
                span: bp.span,
            });
        }
        if let Some(positional_or_property) = self.try_parse_typed_pattern()? {
            return Ok(positional_or_property);
        }
        let expr = self.parse_expr()?;
        Ok(WhenPattern::Expression(expr))
    }

    /// v5 (deferred): try to parse `Type(p1, p2)` (positional sub-pattern)
    /// or `Type { x: p1 }` (property pattern). Returns `None` without
    /// consuming tokens if the speculative parse does not fit.
    fn try_parse_typed_pattern(&mut self) -> Result<Option<WhenPattern>, ParseError> {
        let saved = self.pos;
        let mut path: Vec<String> = Vec::new();
        let start = self.peek_span();
        if let TokenKind::Identifier(name) = self.peek().clone() {
            path.push(name);
            self.advance();
        } else {
            return Ok(None);
        }
        // Optional dotted path tail.
        while self.check(&TokenKind::Dot) {
            let after_dot = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
            if let Some(TokenKind::Identifier(_)) = after_dot {
                self.advance(); // consume '.'
                if let TokenKind::Identifier(name) = self.peek().clone() {
                    path.push(name);
                    self.advance();
                }
            } else {
                break;
            }
        }
        // Property pattern: `Type { ... }`.
        if self.check(&TokenKind::LBrace) {
            return Ok(Some(self.parse_property_pattern_body(path, start)?));
        }
        // Positional pattern: `Type(p1, p2)`. Only treat this as a
        // pattern when at least one entry contains a non-identifier
        // sub-pattern; otherwise the binding-pattern path would have
        // matched first.
        if self.check(&TokenKind::LParen) {
            let saved_inner = self.pos;
            self.advance(); // consume '('
            let mut entries: Vec<WhenPattern> = Vec::new();
            self.skip_newlines();
            if !self.check(&TokenKind::RParen) {
                loop {
                    self.skip_newlines();
                    if self.check(&TokenKind::RParen) { break; }
                    let entry = match self.parse_pattern_atom() {
                        Ok(p) => p,
                        Err(_) => {
                            self.pos = saved;
                            return Ok(None);
                        }
                    };
                    entries.push(entry);
                    if !self.eat(&TokenKind::Comma) { break; }
                }
            }
            if !self.eat(&TokenKind::RParen) {
                self.pos = saved_inner;
                return Ok(None);
            }
            return Ok(Some(WhenPattern::Positional {
                path,
                entries,
                span: Span { start: start.start, end: self.peek_span().end },
            }));
        }
        // Neither — restore position so the caller can fall through to
        // the regular expression path.
        self.pos = saved;
        Ok(None)
    }

    /// v5 (deferred): parse the `{ field: pattern, ... }` body of a
    /// property pattern. The leading type path (if any) was consumed
    /// by the caller.
    fn parse_property_pattern_body(
        &mut self,
        type_path: Vec<String>,
        start: Span,
    ) -> Result<WhenPattern, ParseError> {
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();
        let mut fields: Vec<(String, WhenPattern)> = Vec::new();
        if !self.check(&TokenKind::RBrace) {
            loop {
                self.skip_newlines();
                if self.check(&TokenKind::RBrace) { break; }
                let (name, _) = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                let sub_pat = self.parse_pattern_atom()?;
                fields.push((name, sub_pat));
                if !self.eat(&TokenKind::Comma) { break; }
                self.skip_newlines();
            }
        }
        self.skip_newlines();
        self.expect(&TokenKind::RBrace)?;
        Ok(WhenPattern::Property {
            type_path,
            fields,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Return the relational operator at the current token if any.
    fn peek_relational_op(&self) -> Option<RelationalOp> {
        match self.peek() {
            TokenKind::Lt => Some(RelationalOp::Lt),
            TokenKind::LtEq => Some(RelationalOp::Le),
            TokenKind::Gt => Some(RelationalOp::Gt),
            TokenKind::GtEq => Some(RelationalOp::Ge),
            TokenKind::EqEq => Some(RelationalOp::Eq),
            TokenKind::NotEq => Some(RelationalOp::Ne),
            _ => None,
        }
    }

    /// Speculatively parse a v2 binding pattern: `TypeName.Variant(a, b)` or `TypeName(a, b)`.
    /// Returns `None` WITHOUT consuming tokens if it does not match.
    fn try_parse_binding_pattern(&mut self) -> Result<Option<BindingPatternResult>, ParseError> {
        // We need at least: Identifier ('.' Identifier)* '(' (Identifier (',' Identifier)*)? ')'
        // Use a saved position to backtrack if the pattern doesn't match.
        let saved = self.pos;

        let mut path = Vec::new();
        // First segment must be Identifier
        if let TokenKind::Identifier(name) = self.peek().clone() {
            path.push(name);
            self.advance();
        } else {
            self.pos = saved;
            return Ok(None);
        }
        let pat_start = self.tokens[saved].span;

        // Additional '.Identifier' segments
        while self.check(&TokenKind::Dot) {
            let dot_pos = self.pos;
            self.advance(); // consume '.'
            if let TokenKind::Identifier(seg) = self.peek().clone() {
                path.push(seg);
                self.advance();
            } else {
                // Not a valid continuation — backtrack
                self.pos = dot_pos;
                break;
            }
        }

        // Must be followed by '('
        if !self.check(&TokenKind::LParen) {
            self.pos = saved;
            return Ok(None);
        }
        self.advance(); // consume '('

        // Parse comma-separated identifier bindings
        let mut bindings = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            self.skip_newlines();
            if let TokenKind::Identifier(b) = self.peek().clone() {
                bindings.push(b);
                self.advance();
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            } else {
                // Not all pure identifiers — not a binding pattern, backtrack
                self.pos = saved;
                return Ok(None);
            }
        }

        if !self.eat(&TokenKind::RParen) {
            self.pos = saved;
            return Ok(None);
        }

        // path must have at least 1 segment; bindings may be empty (unit payload)
        if path.is_empty() {
            self.pos = saved;
            return Ok(None);
        }

        let span = Span { start: pat_start.start, end: self.peek_span().end };
        Ok(Some(BindingPatternResult { path, bindings, span }))
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'for'

        // v2: optional destructuring pattern `for TypeName(a, b) in ...`
        // Try speculative binding pattern parse first.
        let for_pat = self.try_parse_binding_pattern()?;
        let (var_name, name_span, for_pattern) = if let Some(bp) = for_pat {
            // For a binding pattern `TypeName(a, b)`, use the first path segment as the loop
            // iteration variable name with a generated name; the pattern carries the bindings.
            let first = bp.path.first().cloned().unwrap_or_else(|| "_".to_string());
            (first, bp.span, Some(DestructurePattern {
                type_name: bp.path.join("."),
                bindings: bp.bindings,
                span: bp.span,
            }))
        } else if self.check(&TokenKind::LParen) {
            // Issue #62: `for (k, v) in pairs` — tuple destructuring loop.
            // Matches the existing `val (a, b) = expr` shape: an empty
            // `type_name` signals tuple deconstruction to lowering.
            let pat_start = self.peek_span();
            let saved = self.pos;
            self.advance(); // consume '('
            let mut bindings: Vec<String> = Vec::new();
            let mut ok = true;
            loop {
                self.skip_newlines();
                if self.check_contextual("_") {
                    bindings.push("_".into());
                    self.advance();
                } else if let TokenKind::Identifier(name) = self.peek().clone() {
                    bindings.push(name);
                    self.advance();
                } else {
                    ok = false;
                    break;
                }
                self.skip_newlines();
                if !self.eat(&TokenKind::Comma) { break; }
            }
            if ok && self.eat(&TokenKind::RParen) && bindings.len() >= 2 {
                let pat_span = Span {
                    start: pat_start.start,
                    end: self.peek_span().end,
                };
                // Use the first binding as the loop variable name (semantic
                // analyzer expects a non-empty var_name for scope bookkeeping).
                // Lowering detects `type_name == ""` and emits a C# tuple
                // deconstruction pattern instead.
                let first = bindings[0].clone();
                (first, pat_span, Some(DestructurePattern {
                    type_name: String::new(),
                    bindings,
                    span: pat_span,
                }))
            } else {
                // Not a tuple destructure — restore and fall back.
                self.pos = saved;
                let (n, s) = self.expect_ident()?;
                (n, s, None)
            }
        } else {
            let (n, s) = self.expect_ident()?;
            (n, s, None)
        };

        self.expect(&TokenKind::In)?;
        // Issue #50: suppress trailing-lambda in the iterable so the
        // loop body `{` is not misread as a lambda arg.
        let iterable = self.with_no_trailing_lambda(|p| p.parse_expr())?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::For {
            var_name,
            name_span,
            for_pattern,
            iterable,
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'while'
        // Issue #50: same cond-context trailing-lambda suppression as `if`.
        let cond = self.with_no_trailing_lambda(|p| p.parse_expr())?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::While {
            cond,
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'return'
        // v5 Sprint 3: `return ref expr` — only valid inside a function
        // declared with a `ref` return type. The parser unconditionally
        // accepts the modifier; downstream lowering wraps the expression
        // in `Expr::RefOf` so emission can produce `return ref expr;`.
        let is_ref = if self.check_contextual("ref")
            && !matches!(self.tokens.get(self.pos + 1).map(|t| t.kind.clone()), Some(TokenKind::Newline) | Some(TokenKind::Eof) | Some(TokenKind::RBrace))
        {
            self.advance();
            true
        } else {
            false
        };
        let value = if !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof | TokenKind::RBrace) {
            let inner = self.parse_expr()?;
            if is_ref {
                let span = Span { start: start.start, end: self.peek_span().end };
                Some(Expr::RefOf {
                    inner: Box::new(inner),
                    span,
                })
            } else {
                Some(inner)
            }
        } else {
            None
        };
        self.expect_newline_or_eof();
        Ok(Stmt::Return {
            value,
            is_ref,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_wait_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'wait'

        let form = match self.peek().clone() {
            TokenKind::NextFrame => {
                self.advance();
                WaitForm::NextFrame
            }
            TokenKind::FixedFrame => {
                self.advance();
                WaitForm::FixedFrame
            }
            TokenKind::Until => {
                self.advance();
                let cond = self.parse_expr()?;
                WaitForm::Until(cond)
            }
            TokenKind::While => {
                self.advance();
                let cond = self.parse_expr()?;
                WaitForm::While(cond)
            }
            TokenKind::DurationLiteral(d) => {
                let d = d;
                self.advance();
                WaitForm::Duration(Expr::DurationLit(d, self.peek_span()))
            }
            _ => {
                // wait <expr>.s or wait <expr>
                let expr = self.parse_expr()?;
                WaitForm::Duration(expr)
            }
        };

        self.expect_newline_or_eof();
        Ok(Stmt::Wait { form, span: Span { start: start.start, end: self.peek_span().end } })
    }

    fn parse_listen_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'listen'
        // Use parse_additive (not parse_expr) to avoid consuming `until` as a range operator.
        // listen events are always simple member-access expressions like `button.onClick`.
        // Issue #50: the `{ ... }` that follows an event is the listen body,
        // not a trailing lambda — suppress the trailing-lambda desugar
        // while parsing the event expression.
        let event = self.with_no_trailing_lambda(|p| p.parse_additive())?;

        // Parse optional lifetime modifier before the body block:
        //   listen event until disable { … }   → UntilDisable
        //   listen event until destroy { … }   → UntilDestroy
        //   listen event manual { … }           → Manual
        //   listen event { … }                  → Register (v1 default)
        self.skip_newlines();
        let lifetime = match self.peek().clone() {
            TokenKind::Until => {
                self.advance(); // consume 'until'
                self.skip_newlines();
                match self.peek().clone() {
                    TokenKind::Identifier(ref w) if w == "disable" => {
                        self.advance(); // consume 'disable'
                        ListenLifetime::UntilDisable
                    }
                    TokenKind::Identifier(ref w) if w == "destroy" => {
                        self.advance(); // consume 'destroy'
                        ListenLifetime::UntilDestroy
                    }
                    other => {
                        let span = self.peek_span();
                        return Err(ParseError {
                            message: format!("expected 'disable' or 'destroy' after 'until' in listen, got {:?}", other),
                            span,
                        });
                    }
                }
            }
            TokenKind::Manual => {
                self.advance(); // consume 'manual'
                ListenLifetime::Manual
            }
            _ => ListenLifetime::Register,
        };

        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();

        // Check for lambda params: { param => body }
        let mut params = Vec::new();
        if let TokenKind::Identifier(name) = self.peek().clone() {
            let mut ahead = self.pos + 1;
            while ahead < self.tokens.len() && self.tokens[ahead].kind == TokenKind::Newline {
                ahead += 1;
            }
            if ahead < self.tokens.len() && self.tokens[ahead].kind == TokenKind::FatArrow {
                params.push(name);
                self.advance(); // consume ident
                self.advance(); // consume =>
                self.skip_newlines();
            }
        }

        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            match self.parse_stmt() {
                Ok(s) => stmts.push(s),
                Err(e) => { self.errors.push(e); self.recover_to_newline(); }
            }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;

        let body = Block {
            stmts,
            span: Span { start: start.start, end: self.peek_span().end },
        };

        Ok(Stmt::Listen {
            event,
            params,
            lifetime,
            bound_name: None,
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_unlisten_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'unlisten'
        self.skip_newlines();
        // Issue #33: accept a richer token expression after `unlisten`
        // so documented idioms like `unlisten skipToken!!`,
        // `unlisten this.tk`, and `unlisten field?.tk` all parse. The
        // AST still stores the base identifier (the lowering walks
        // manual-listen records by name); we simply strip the
        // optional `this.` prefix, trailing member hops, and
        // non-null-assert (`!!`) / safe-call (`?.`) suffixes.
        let base_name = match self.peek().clone() {
            TokenKind::Identifier(n) => {
                self.advance();
                n
            }
            TokenKind::This => {
                self.advance();
                if !self.eat(&TokenKind::Dot) {
                    return Err(ParseError {
                        message: "expected `.` after `this` in `unlisten`".into(),
                        span: self.peek_span(),
                    });
                }
                let (n, _) = self.expect_ident()?;
                n
            }
            other => {
                return Err(ParseError {
                    message: format!("expected identifier after 'unlisten', got {:?}", other),
                    span: self.peek_span(),
                });
            }
        };
        // Consume optional postfix: chains of `.field` and `?.field`
        // that walk further into the token expression, plus a single
        // trailing `!!` non-null assertion. The *last* identifier is
        // the token name the lowering pass searches for.
        let mut token = base_name;
        loop {
            match self.peek().clone() {
                TokenKind::Dot | TokenKind::QuestionDot => {
                    self.advance();
                    let (next, _) = self.expect_ident()?;
                    token = next;
                }
                TokenKind::BangBang => {
                    self.advance();
                }
                _ => break,
            }
        }
        self.expect_newline_or_eof();
        Ok(Stmt::Unlisten {
            token,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_try_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'try'
        self.skip_newlines();
        let try_block = self.parse_block()?;

        let mut catches = Vec::new();
        while self.check(&TokenKind::Catch) {
            let catch_start = self.peek_span();
            self.advance(); // consume 'catch'
            self.expect(&TokenKind::LParen)?;
            let (name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            self.expect(&TokenKind::RParen)?;
            self.skip_newlines();
            let body = self.parse_block()?;
            catches.push(CatchClause {
                name,
                ty,
                body,
                span: Span { start: catch_start.start, end: self.peek_span().end },
            });
        }

        let finally_block = if self.check(&TokenKind::Finally) {
            self.advance(); // consume 'finally'
            self.skip_newlines();
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Stmt::Try {
            try_block,
            catches,
            finally_block,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Issue #49: `try { expr } catch (e: T) { expr }` in expression
    /// position. The lang-4 spec requires exactly one catch clause for
    /// the expression form. `finally` is not permitted here (use the
    /// statement form).
    fn parse_try_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'try'
        self.skip_newlines();
        let try_block = self.parse_block()?;

        let mut catches = Vec::new();
        while self.check(&TokenKind::Catch) {
            let catch_start = self.peek_span();
            self.advance(); // consume 'catch'
            // `catch (e: T)` — the expression form is strict about the
            // shape (no bare `catch { ... }`).
            self.expect(&TokenKind::LParen)?;
            let (name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            self.expect(&TokenKind::RParen)?;
            self.skip_newlines();
            let body = self.parse_block()?;
            catches.push(CatchClause {
                name,
                ty,
                body,
                span: Span { start: catch_start.start, end: self.peek_span().end },
            });
        }

        if catches.is_empty() {
            return Err(self.error(
                "try expression requires at least one catch clause".into(),
            ));
        }

        Ok(Expr::TryExpr {
            try_block,
            catches,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_throw_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'throw'
        let expr = self.parse_expr()?;
        self.expect_newline_or_eof();
        Ok(Stmt::Throw {
            expr,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Parse a `yield expr` or `yield break` statement (Language 5, Sprint 1).
    ///
    /// `yield` is a contextual keyword: only the two valid follow-on shapes
    /// here are recognized; anything else falls through to a parse error.
    /// The semantic analyzer is responsible for E147 (use outside an
    /// iterator) and E148 (element type mismatch).
    fn parse_yield_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        let _ = self.eat_contextual("yield");
        // `yield break` — terminate the iterator.
        if self.check(&TokenKind::Break) {
            self.advance();
            self.expect_newline_or_eof();
            return Ok(Stmt::YieldBreak {
                span: Span { start: start.start, end: self.peek_span().end },
            });
        }
        // `yield expr` — emit a value.
        let value = self.parse_expr()?;
        self.expect_newline_or_eof();
        Ok(Stmt::Yield {
            value,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Parse a preprocessor block (Language 5, Sprint 1):
    ///
    /// ```text
    /// #if cond
    ///     stmt*
    /// #elif cond
    ///     stmt*
    /// #else
    ///     stmt*
    /// #endif
    /// ```
    ///
    /// Each branch contains zero or more statements. Statements are parsed
    /// with the regular `parse_stmt` so all of PrSM is available inside an
    /// `#if` arm. Diagnostics for unterminated blocks (E151) and dangling
    /// `#elif`/`#else` (E152) are surfaced from the parser; W034 (unknown
    /// symbol) is reported by the semantic analyzer.
    fn parse_preprocessor_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.expect(&TokenKind::HashIf)?;
        let first_cond = self.parse_preprocessor_cond()?;
        self.expect_newline_or_eof();
        let first_body = self.parse_preprocessor_arm_body()?;
        let mut arms = vec![PreprocessorArm {
            cond: first_cond,
            body: first_body,
            span: start,
        }];
        // Zero or more `#elif` arms.
        while self.check(&TokenKind::HashElif) {
            let arm_start = self.peek_span();
            self.advance(); // consume #elif
            let cond = self.parse_preprocessor_cond()?;
            self.expect_newline_or_eof();
            let body = self.parse_preprocessor_arm_body()?;
            arms.push(PreprocessorArm {
                cond,
                body,
                span: arm_start,
            });
        }
        // Optional `#else` arm.
        let else_arm = if self.check(&TokenKind::HashElse) {
            self.advance(); // consume #else
            self.expect_newline_or_eof();
            Some(self.parse_preprocessor_arm_body()?)
        } else {
            None
        };
        // Mandatory `#endif`.
        if !self.check(&TokenKind::HashEndif) {
            self.errors.push(ParseError {
                message: "E151: unterminated '#if' block — expected '#endif'".into(),
                span: start,
            });
        } else {
            self.advance();
            self.expect_newline_or_eof();
        }
        Ok(Stmt::Preprocessor {
            arms,
            else_arm,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Parse statements for one preprocessor arm. Stops at the next
    /// `#elif`, `#else`, `#endif`, or end of file.
    fn parse_preprocessor_arm_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek() {
                TokenKind::HashElif
                | TokenKind::HashElse
                | TokenKind::HashEndif
                | TokenKind::Eof => break,
                // A nested `#if` is parsed as a regular statement, which
                // recurses into `parse_preprocessor_stmt`.
                _ => {
                    let stmt = self.parse_stmt()?;
                    stmts.push(stmt);
                }
            }
        }
        Ok(stmts)
    }

    /// Parse a preprocessor condition expression. Grammar:
    ///
    /// ```text
    /// Cond = Or
    /// Or   = And { "||" And }
    /// And  = Not { "&&" Not }
    /// Not  = "!" Not | Atom
    /// Atom = Symbol | "(" Or ")"
    /// ```
    fn parse_preprocessor_cond(&mut self) -> Result<PreprocessorCond, ParseError> {
        self.parse_pp_or()
    }

    fn parse_pp_or(&mut self) -> Result<PreprocessorCond, ParseError> {
        let mut left = self.parse_pp_and()?;
        while self.check(&TokenKind::PipePipe) {
            self.advance();
            let right = self.parse_pp_and()?;
            let span = pp_cond_span(&left);
            left = PreprocessorCond::Or {
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_pp_and(&mut self) -> Result<PreprocessorCond, ParseError> {
        let mut left = self.parse_pp_not()?;
        while self.check(&TokenKind::AmpAmp) {
            self.advance();
            let right = self.parse_pp_not()?;
            let span = pp_cond_span(&left);
            left = PreprocessorCond::And {
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_pp_not(&mut self) -> Result<PreprocessorCond, ParseError> {
        if self.check(&TokenKind::Bang) {
            let start = self.peek_span();
            self.advance();
            let inner = self.parse_pp_not()?;
            return Ok(PreprocessorCond::Not {
                inner: Box::new(inner),
                span: start,
            });
        }
        self.parse_pp_atom()
    }

    fn parse_pp_atom(&mut self) -> Result<PreprocessorCond, ParseError> {
        if self.eat(&TokenKind::LParen) {
            let inner = self.parse_pp_or()?;
            self.expect(&TokenKind::RParen)?;
            return Ok(inner);
        }
        // A symbol — accept any identifier (curated PrSM symbol or raw define).
        let span = self.peek_span();
        let name = match self.peek().clone() {
            TokenKind::Identifier(name) => {
                self.advance();
                name
            }
            other => {
                return Err(self.error(format!(
                    "Expected preprocessor symbol identifier, found {:?}",
                    other
                )));
            }
        };
        Ok(PreprocessorCond::Symbol { name, span })
    }

    /// Parse `use val name = expr` (declaration form) or `use name = expr { body }`
    /// (block form). Both lower to a C# `using` declaration / `using` statement.
    fn parse_use_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        let _ = self.eat_contextual("use");

        // Optional `val` keyword — declaration form binds for the rest of the
        // enclosing scope (no body block expected).
        let has_val = self.eat(&TokenKind::Val);

        let (name, name_span) = self.expect_ident()?;
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&TokenKind::Eq)?;
        // Issue #50: the optional `{ body }` after `use x = expr` is the
        // use statement body, not a trailing lambda — suppress trailing-
        // lambda desugar while parsing the initializer so `use s = openFile() { log(s) }`
        // keeps the braces for the body.
        let init = self.with_no_trailing_lambda(|p| p.parse_expr())?;

        let body = if has_val {
            // `use val ...` — declaration form, no body
            self.expect_newline_or_eof();
            None
        } else {
            // `use ... { ... }` — block form
            self.skip_newlines();
            if self.check(&TokenKind::LBrace) {
                Some(self.parse_block()?)
            } else {
                // Tolerate the omitted block: behave as a declaration form.
                self.expect_newline_or_eof();
                None
            }
        };

        Ok(Stmt::Use {
            name,
            name_span,
            ty,
            init,
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Phase 5: parse `bind source to target.expr` declarative binding statement.
    /// Form: `bind IDENT to EXPR` — wired via `BindTo` Stmt.
    fn parse_bind_to_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        let _ = self.eat_contextual("bind")
            .ok_or_else(|| self.error("Expected 'bind'".into()))?;
        let (source, source_span) = self.expect_ident()?;
        let _ = self.eat_contextual("to")
            .ok_or_else(|| self.error("Expected 'to' in bind statement".into()))?;
        let target = self.parse_expr()?;
        self.expect_newline_or_eof();
        Ok(Stmt::BindTo {
            source,
            source_span,
            target,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_expr_or_assignment_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        let expr = self.parse_expr()?;

        // Check for assignment operators
        let assign_op = match self.peek() {
            TokenKind::Eq => Some(AssignOp::Assign),
            TokenKind::PlusEq => Some(AssignOp::PlusAssign),
            TokenKind::MinusEq => Some(AssignOp::MinusAssign),
            TokenKind::StarEq => Some(AssignOp::StarAssign),
            TokenKind::SlashEq => Some(AssignOp::SlashAssign),
            TokenKind::PercentEq => Some(AssignOp::ModAssign),
            TokenKind::ElvisAssign => Some(AssignOp::NullCoalesceAssign),
            _ => None,
        };

        if let Some(op) = assign_op {
            self.advance();
            let value = self.parse_expr()?;
            self.expect_newline_or_eof();
            Ok(Stmt::Assignment {
                target: expr,
                op,
                value,
                span: Span { start: start.start, end: self.peek_span().end },
            })
        } else {
            self.expect_newline_or_eof();
            Ok(Stmt::Expr { expr, span: Span { start: start.start, end: self.peek_span().end } })
        }
    }

    // ── Expression parsing (Pratt parser) ────────────────────────

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_elvis()
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'if'
        // Issue #50: same condition-context trailing-lambda suppression.
        let cond = self.with_no_trailing_lambda(|p| p.parse_expr())?;
        self.skip_newlines();
        let then_block = self.parse_block()?;

        if !self.eat(&TokenKind::Else) {
            return Err(self.error("Expected else branch in if expression".into()));
        }

        self.skip_newlines();
        let else_block = if self.check(&TokenKind::If) {
            let nested_if = self.parse_if_expr()?;
            self.expr_to_block(nested_if)
        } else {
            self.parse_block()?
        };

        Ok(Expr::IfExpr {
            cond: Box::new(cond),
            then_block,
            else_block,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_when_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'when'

        let subject = if !self.check(&TokenKind::LBrace) && !self.check(&TokenKind::Newline) {
            // Issue #50: trailing-lambda suppression — see parse_when_stmt.
            Some(Box::new(self.with_no_trailing_lambda(|p| p.parse_expr())?))
        } else {
            None
        };

        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();

        let mut branches = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            branches.push(self.parse_when_branch()?);
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;

        Ok(Expr::WhenExpr {
            subject,
            branches,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn expr_to_block(&self, expr: Expr) -> Block {
        let span = expr.span();
        Block {
            stmts: vec![Stmt::Expr { expr, span }],
            span,
        }
    }

    // Elvis (?:) — lowest precedence
    fn parse_elvis(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_or()?;
        while self.check(&TokenKind::Elvis) {
            let start = left.span();
            self.advance();
            // Issue #56: allow newline(s) after the infix operator so that
            // wrapped expressions like `a ?:\n  b` continue correctly.
            self.skip_newlines();
            let right = self.parse_or()?;
            let span = Span { start: start.start, end: right.span().end };
            left = Expr::Elvis {
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::PipePipe) {
            let start = left.span();
            self.advance();
            self.skip_newlines();
            let right = self.parse_and()?;
            let span = Span { start: start.start, end: right.span().end };
            left = Expr::Binary { left: Box::new(left), op: BinOp::Or, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;
        while self.check(&TokenKind::AmpAmp) {
            let start = left.span();
            self.advance();
            self.skip_newlines();
            let right = self.parse_equality()?;
            let span = Span { start: start.start, end: right.span().end };
            left = Expr::Binary { left: Box::new(left), op: BinOp::And, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::NotEq => BinOp::NotEq,
                _ => break,
            };
            let start = left.span();
            self.advance();
            self.skip_newlines();
            let right = self.parse_comparison()?;
            let span = Span { start: start.start, end: right.span().end };
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_range()?;
        loop {
            let op = match self.peek() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::LtEq => BinOp::LtEq,
                TokenKind::GtEq => BinOp::GtEq,
                TokenKind::Is => {
                    let start = left.span();
                    self.advance();
                    self.skip_newlines();
                    let ty = self.parse_type()?;
                    let span = Span { start: start.start, end: self.peek_span().end };
                    left = Expr::Is { expr: Box::new(left), ty, span };
                    continue;
                }
                TokenKind::As => {
                    let start = left.span();
                    self.advance(); // consume 'as'
                    self.skip_newlines();
                    // Check for force cast: as! Type
                    if self.check(&TokenKind::Bang) {
                        self.advance(); // consume '!'
                        let ty = self.parse_type()?;
                        let span = Span { start: start.start, end: self.peek_span().end };
                        left = Expr::ForceCastExpr { expr: Box::new(left), target_type: ty, span };
                    } else {
                        // Safe cast: as Type?
                        let ty = self.parse_type()?;
                        // The type should already be nullable from parse_type consuming '?'
                        let span = Span { start: start.start, end: self.peek_span().end };
                        left = Expr::SafeCastExpr { expr: Box::new(left), target_type: ty, span };
                    }
                    continue;
                }
                TokenKind::In => {
                    let start = left.span();
                    self.advance();
                    self.skip_newlines();
                    let right = self.parse_range()?;
                    let span = Span { start: start.start, end: right.span().end };
                    left = Expr::Binary { left: Box::new(left), op: BinOp::In, right: Box::new(right), span };
                    continue;
                }
                _ => break,
            };
            let start = left.span();
            self.advance();
            self.skip_newlines();
            let right = self.parse_range()?;
            let span = Span { start: start.start, end: right.span().end };
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_range(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_additive()?;

        // range operators: .., until, downTo
        match self.peek().clone() {
            TokenKind::DotDot => {
                let start = left.span();
                self.advance();
                // Issue #58: open-ended upper form `a..`. If the next
                // token terminates the range context, skip the upper
                // parse and record `end = None`.
                let end = if self.range_upper_missing() {
                    None
                } else {
                    Some(Box::new(self.parse_additive()?))
                };
                let step = if self.eat(&TokenKind::Step) {
                    Some(Box::new(self.parse_additive()?))
                } else {
                    None
                };
                let span = Span { start: start.start, end: self.peek_span().end };
                Ok(Expr::Range { start: Some(Box::new(left)), end, inclusive: true, descending: false, step, span })
            }
            TokenKind::Until => {
                let start = left.span();
                self.advance();
                let end = if self.range_upper_missing() {
                    None
                } else {
                    Some(Box::new(self.parse_additive()?))
                };
                let step = if self.eat(&TokenKind::Step) {
                    Some(Box::new(self.parse_additive()?))
                } else {
                    None
                };
                let span = Span { start: start.start, end: self.peek_span().end };
                Ok(Expr::Range { start: Some(Box::new(left)), end, inclusive: false, descending: false, step, span })
            }
            TokenKind::DownTo => {
                let start = left.span();
                self.advance();
                let end = if self.range_upper_missing() {
                    None
                } else {
                    Some(Box::new(self.parse_additive()?))
                };
                let step = if self.eat(&TokenKind::Step) {
                    Some(Box::new(self.parse_additive()?))
                } else {
                    None
                };
                let span = Span { start: start.start, end: self.peek_span().end };
                // downTo is an inclusive descending range: [left, end] iterating downward
                Ok(Expr::Range { start: Some(Box::new(left)), end, inclusive: true, descending: true, step, span })
            }
            _ => Ok(left),
        }
    }

    /// Issue #58: detect whether the range-operator upper bound is
    /// missing. A missing upper bound is indicated by `]` (index slice),
    /// `)` (closing paren), `,` (tuple / call arg list), `}` (block),
    /// newline, or EOF. Valid upper-bound expressions always start with
    /// a prefix that is none of those.
    fn range_upper_missing(&self) -> bool {
        matches!(
            self.peek(),
            TokenKind::RBracket
                | TokenKind::RParen
                | TokenKind::RBrace
                | TokenKind::Comma
                | TokenKind::Newline
                | TokenKind::Eof
        )
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let start = left.span();
            self.advance();
            self.skip_newlines();
            let right = self.parse_multiplicative()?;
            let span = Span { start: start.start, end: right.span().end };
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            let start = left.span();
            self.advance();
            self.skip_newlines();
            let right = self.parse_unary()?;
            let span = Span { start: start.start, end: right.span().end };
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        // Phase 5: `await EXPR` is a prefix expression (contextual keyword).
        if self.check_contextual("await") {
            let start = self.peek_span();
            self.advance(); // consume 'await'
            let operand = self.parse_unary()?;
            let span = Span { start: start.start, end: operand.span().end };
            return Ok(Expr::Await { expr: Box::new(operand), span });
        }
        match self.peek().clone() {
            TokenKind::Bang => {
                let start = self.peek_span();
                self.advance();
                let operand = self.parse_unary()?;
                let span = Span { start: start.start, end: operand.span().end };
                Ok(Expr::Unary { op: UnaryOp::Not, operand: Box::new(operand), span })
            }
            TokenKind::Minus => {
                let start = self.peek_span();
                self.advance();
                let operand = self.parse_unary()?;
                let span = Span { start: start.start, end: operand.span().end };
                Ok(Expr::Unary { op: UnaryOp::Negate, operand: Box::new(operand), span })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            // Issue #57: leading-dot chain continuation across newlines.
            // When the current token is a newline AND the next non-newline
            // token is `.` or `?.`, consume the newline(s) so the postfix
            // loop can pick up the continuation. This must NOT consume
            // newlines unconditionally — doing so would merge unrelated
            // statements. We only skip when a dot-start continuation is
            // actually pending.
            if matches!(self.peek(), TokenKind::Newline) {
                let mut look = self.pos;
                while look < self.tokens.len()
                    && matches!(self.tokens[look].kind, TokenKind::Newline)
                {
                    look += 1;
                }
                if look < self.tokens.len()
                    && matches!(
                        self.tokens[look].kind,
                        TokenKind::Dot | TokenKind::QuestionDot
                    )
                {
                    self.pos = look;
                } else {
                    break;
                }
            }
            match self.peek().clone() {
                TokenKind::Dot => {
                    self.advance();
                    // Issue #57: trailing-dot chain continuation —
                    // `list.\n  where(...)`. Skip newlines after the dot so
                    // the identifier on the next line is picked up.
                    self.skip_newlines();
                    let (name, name_span) = self.expect_ident()?;

                    // Check if this is a method call: expr.name(args)
                    if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) {
                        let (type_args, mut args) = self.parse_call_args()?;
                        // Issue #50: trailing lambda after positional args —
                        // `xs.fold(0) { acc, x => acc + x }`. Suppressed in
                        // control-flow condition positions.
                        if !self.no_trailing_lambda && self.check(&TokenKind::LBrace) {
                            let lambda = self.parse_brace_expr()?;
                            args.push(Arg { name: None, call_modifier: ArgMod::None, value: lambda });
                        }
                        let span = Span { start: expr.span().start, end: self.peek_span().end };
                        expr = Expr::Call {
                            receiver: Some(Box::new(expr)),
                            name,
                            name_span,
                            type_args,
                            args,
                            span,
                        };
                    } else if !self.no_trailing_lambda && self.check(&TokenKind::LBrace) {
                        // Issue #50: trailing-lambda call with no explicit
                        // parenthesized args — `list.filter { it > 10 }`.
                        // Parse the brace expression as the sole argument.
                        let lambda = self.parse_brace_expr()?;
                        let arg = Arg { name: None, call_modifier: ArgMod::None, value: lambda };
                        let span = Span { start: expr.span().start, end: self.peek_span().end };
                        expr = Expr::Call {
                            receiver: Some(Box::new(expr)),
                            name,
                            name_span,
                            type_args: vec![],
                            args: vec![arg],
                            span,
                        };
                    } else {
                        let span = Span { start: expr.span().start, end: self.peek_span().end };
                        expr = Expr::MemberAccess {
                            receiver: Box::new(expr),
                            name,
                            name_span,
                            span,
                        };
                    }
                }
                TokenKind::QuestionDot => {
                    self.advance();
                    self.skip_newlines();
                    let (name, name_span) = self.expect_ident()?;

                    if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) {
                        let (type_args, mut args) = self.parse_call_args()?;
                        if !self.no_trailing_lambda && self.check(&TokenKind::LBrace) {
                            let lambda = self.parse_brace_expr()?;
                            args.push(Arg { name: None, call_modifier: ArgMod::None, value: lambda });
                        }
                        let span = Span { start: expr.span().start, end: self.peek_span().end };
                        expr = Expr::SafeMethodCall {
                            receiver: Box::new(expr),
                            name,
                            name_span,
                            type_args,
                            args,
                            span,
                        };
                    } else if !self.no_trailing_lambda && self.check(&TokenKind::LBrace) {
                        // Issue #50: trailing lambda on safe method call.
                        let lambda = self.parse_brace_expr()?;
                        let arg = Arg { name: None, call_modifier: ArgMod::None, value: lambda };
                        let span = Span { start: expr.span().start, end: self.peek_span().end };
                        expr = Expr::SafeMethodCall {
                            receiver: Box::new(expr),
                            name,
                            name_span,
                            type_args: vec![],
                            args: vec![arg],
                            span,
                        };
                    } else {
                        let span = Span { start: expr.span().start, end: self.peek_span().end };
                        expr = Expr::SafeCall {
                            receiver: Box::new(expr),
                            name,
                            name_span,
                            span,
                        };
                    }
                }
                // v5 Sprint 6: `arr?[index]` — null-conditional indexer.
                // The `?` token is followed immediately by `[`, so we
                // disambiguate from a regular ternary by peeking at the
                // next token in the postfix loop.
                TokenKind::Question if matches!(
                    self.tokens.get(self.pos + 1).map(|t| t.kind.clone()),
                    Some(TokenKind::LBracket)
                ) => {
                    self.advance(); // consume '?'
                    self.advance(); // consume '['
                    let index = self.parse_expr()?;
                    self.expect(&TokenKind::RBracket)?;
                    let span = Span { start: expr.span().start, end: self.peek_span().end };
                    expr = Expr::SafeIndexAccess {
                        receiver: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                // v5 (deferred): `expr with { field = value, ... }` —
                // record-style update expression. `with` is a contextual
                // keyword recognized only when followed by `{`.
                TokenKind::Identifier(ref name) if name == "with" && matches!(
                    self.tokens.get(self.pos + 1).map(|t| t.kind.clone()),
                    Some(TokenKind::LBrace)
                ) => {
                    self.advance(); // consume 'with'
                    self.advance(); // consume '{'
                    let mut updates: Vec<(String, Expr)> = Vec::new();
                    self.skip_newlines();
                    if !self.check(&TokenKind::RBrace) {
                        loop {
                            self.skip_newlines();
                            if self.check(&TokenKind::RBrace) { break; }
                            let (field_name, _) = self.expect_ident()?;
                            self.expect(&TokenKind::Eq)?;
                            let value = self.parse_expr()?;
                            updates.push((field_name, value));
                            if !self.eat(&TokenKind::Comma) { break; }
                            self.skip_newlines();
                        }
                    }
                    self.skip_newlines();
                    self.expect(&TokenKind::RBrace)?;
                    let span = Span { start: expr.span().start, end: self.peek_span().end };
                    expr = Expr::With {
                        receiver: Box::new(expr),
                        updates,
                        span,
                    };
                }
                TokenKind::BangBang => {
                    self.advance();
                    let span = Span { start: expr.span().start, end: self.peek_span().end };
                    expr = Expr::NonNullAssert { expr: Box::new(expr), span };
                }
                TokenKind::LBracket => {
                    self.advance();
                    // Issue #58: open-ended range slice `arr[..3]` — the
                    // `..` / `until` / `downTo` starts the index without
                    // a lower bound. Synthesize an Expr::Range with
                    // `start = None`. For `arr[..]` both bounds are None.
                    let index = if matches!(
                        self.peek(),
                        TokenKind::DotDot | TokenKind::Until | TokenKind::DownTo
                    ) {
                        let op_tok = self.peek().clone();
                        let op_span = self.peek_span();
                        self.advance();
                        let (inclusive, descending) = match op_tok {
                            TokenKind::DotDot => (true, false),
                            TokenKind::Until => (false, false),
                            TokenKind::DownTo => (true, true),
                            _ => (true, false),
                        };
                        let end = if self.range_upper_missing()
                            || matches!(self.peek(), TokenKind::RBracket)
                        {
                            None
                        } else {
                            Some(Box::new(self.parse_additive()?))
                        };
                        let step = if self.eat(&TokenKind::Step) {
                            Some(Box::new(self.parse_additive()?))
                        } else {
                            None
                        };
                        Expr::Range {
                            start: None,
                            end,
                            inclusive,
                            descending,
                            step,
                            span: Span {
                                start: op_span.start,
                                end: self.peek_span().end,
                            },
                        }
                    } else {
                        self.parse_expr()?
                    };
                    self.expect(&TokenKind::RBracket)?;
                    let span = Span { start: expr.span().start, end: self.peek_span().end };
                    expr = Expr::IndexAccess {
                        receiver: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                TokenKind::LParen => {
                    // Function call without receiver or with implicit receiver
                    // This handles cases like: ident(args), ident<T>(args)
                    if let Expr::Ident(name, name_span) = &expr {
                        let name = name.clone();
                        let name_span = *name_span;
                        let (type_args, mut args) = self.parse_call_args()?;
                        // Issue #50: trailing lambda on bare-ident call.
                        if !self.no_trailing_lambda && self.check(&TokenKind::LBrace) {
                            let lambda = self.parse_brace_expr()?;
                            args.push(Arg { name: None, call_modifier: ArgMod::None, value: lambda });
                        }
                        let span = Span { start: expr.span().start, end: self.peek_span().end };
                        expr = Expr::Call {
                            receiver: None,
                            name,
                            name_span,
                            type_args,
                            args,
                            span,
                        };
                    } else {
                        break;
                    }
                }
                TokenKind::Lt => {
                    // Could be generic call: ident<T>(args)
                    // Only try if expr is an identifier
                    if let Expr::Ident(name, name_span) = &expr {
                        let name = name.clone();
                        let name_span = *name_span;
                        // Try parsing as generic call, backtrack if not
                        let save = self.pos;
                        if let Ok((type_args, mut args)) = self.try_parse_generic_call() {
                            if !self.no_trailing_lambda && self.check(&TokenKind::LBrace) {
                                let lambda = self.parse_brace_expr()?;
                                args.push(Arg { name: None, call_modifier: ArgMod::None, value: lambda });
                            }
                            let span = Span { start: expr.span().start, end: self.peek_span().end };
                            expr = Expr::Call {
                                receiver: None,
                                name,
                                name_span,
                                type_args,
                                args,
                                span,
                            };
                        } else {
                            self.pos = save;
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_call_args(&mut self) -> Result<(Vec<TypeRef>, Vec<Arg>), ParseError> {
        let type_args = if self.eat(&TokenKind::Lt) {
            let mut ta = Vec::new();
            self.skip_newlines();
            while !self.check(&TokenKind::Gt) && !self.check(&TokenKind::Eof) {
                ta.push(self.parse_type()?);
                self.skip_newlines();
                if !self.eat(&TokenKind::Comma) { break; }
                self.skip_newlines();
            }
            self.expect(&TokenKind::Gt)?;
            ta
        } else {
            vec![]
        };

        self.expect(&TokenKind::LParen)?;
        // Issue #53: allow newlines inside a call argument list so that
        // wrapped invocations like `log(\n  "a",\n  "b"\n)` parse.
        self.skip_newlines();
        let mut args = Vec::new();
        // Issue #50: nested calls inside a paren arg list can have their
        // own trailing lambdas — reset the suppression flag across the
        // argument parse so `foo(bar.filter { it > 0 })` still works
        // inside an if/while cond.
        let prev_ntl = self.no_trailing_lambda;
        self.no_trailing_lambda = false;
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let arg = self.parse_call_arg()?;
            args.push(arg);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) { break; }
            self.skip_newlines();
        }
        self.no_trailing_lambda = prev_ntl;
        self.skip_newlines();
        self.expect(&TokenKind::RParen)?;
        Ok((type_args, args))
    }

    /// Parse a single call-site argument (Language 5, Sprint 2). Supports:
    ///
    /// * `expr` — positional
    /// * `name: expr` / `name = expr` — named (Kotlin `:` and legacy `=`)
    /// * `ref expr` — pass by reference
    /// * `out expr` — out argument with an existing variable
    /// * `out val name` / `out var name` — out declaration expression
    /// * `out _` — out discard
    fn parse_call_arg(&mut self) -> Result<Arg, ParseError> {
        // ── ref / out modifiers ────────────────────────────────────
        if self.check_contextual("ref") {
            self.advance();
            let value = self.parse_expr()?;
            return Ok(Arg {
                name: None,
                call_modifier: ArgMod::Ref,
                value,
            });
        }
        if self.check_contextual("out") {
            self.advance();
            // `out _` discard
            if self.check_contextual("_") {
                let span = self.peek_span();
                self.advance();
                return Ok(Arg {
                    name: None,
                    call_modifier: ArgMod::OutDiscard,
                    value: Expr::Ident("_".into(), span),
                });
            }
            // `out val name` / `out var name` declaration expression
            if self.check(&TokenKind::Val) || self.check(&TokenKind::Var) {
                self.advance();
                let (name, name_span) = self.expect_ident()?;
                return Ok(Arg {
                    name: None,
                    call_modifier: ArgMod::OutDecl(name.clone()),
                    value: Expr::Ident(name, name_span),
                });
            }
            // Plain `out expr`
            let value = self.parse_expr()?;
            return Ok(Arg {
                name: None,
                call_modifier: ArgMod::Out,
                value,
            });
        }

        // ── named argument (legacy `name = expr`, v5 `name: expr`) ──
        // Identifiers are always candidates for the name slot. Keyword
        // tokens are also candidates *only* when followed by `=` or `:`,
        // so an `if` / `when` expression starting with a keyword is
        // unaffected. This handles issue #6 where `parent: target`
        // failed because `parent` lexes as the `Parent` keyword.
        let arg_name_text: Option<String> = if let TokenKind::Identifier(name) = self.peek() {
            Some(name.clone())
        } else if let Some(text) = self.peek().keyword_text() {
            let next = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
            if matches!(next, Some(TokenKind::Eq) | Some(TokenKind::Colon)) {
                Some(text.to_string())
            } else {
                None
            }
        } else {
            None
        };
        if let Some(name) = arg_name_text {
            let save = self.pos;
            self.advance();
            if self.eat(&TokenKind::Eq) {
                let value = self.parse_expr()?;
                return Ok(Arg {
                    name: Some(name),
                    call_modifier: ArgMod::None,
                    value,
                });
            }
            // `name: expr` — but only when followed by something that
            // can start an expression. The `:` token is also used for
            // type annotations and map literals, so be conservative.
            if self.check(&TokenKind::Colon) {
                let next_kind = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
                let starts_expr = matches!(
                    next_kind,
                    Some(TokenKind::Identifier(_))
                        | Some(TokenKind::IntLiteral(_))
                        | Some(TokenKind::FloatLiteral(_))
                        | Some(TokenKind::StringLiteral(_))
                        | Some(TokenKind::StringStart(_))
                        | Some(TokenKind::BoolTrue)
                        | Some(TokenKind::BoolFalse)
                        | Some(TokenKind::Null)
                        | Some(TokenKind::This)
                        | Some(TokenKind::LParen)
                        | Some(TokenKind::LBracket)
                        | Some(TokenKind::Minus)
                        | Some(TokenKind::Bang)
                );
                if starts_expr {
                    self.advance(); // consume ':'
                    let value = self.parse_expr()?;
                    return Ok(Arg {
                        name: Some(name),
                        call_modifier: ArgMod::None,
                        value,
                    });
                }
            }
            self.pos = save;
        }

        let value = self.parse_expr()?;
        Ok(Arg {
            name: None,
            call_modifier: ArgMod::None,
            value,
        })
    }

    fn try_parse_generic_call(&mut self) -> Result<(Vec<TypeRef>, Vec<Arg>), ParseError> {
        // Try parsing <TypeArgs>(args)
        self.expect(&TokenKind::Lt)?;
        let mut type_args = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenKind::Gt) && !self.check(&TokenKind::Eof) {
            type_args.push(self.parse_type()?);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) { break; }
            self.skip_newlines();
        }
        self.expect(&TokenKind::Gt)?;
        self.expect(&TokenKind::LParen)?;
        // Issue #53: same multi-line support as parse_call_args.
        self.skip_newlines();
        let mut args = Vec::new();
        let prev_ntl = self.no_trailing_lambda;
        self.no_trailing_lambda = false;
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let arg = self.parse_call_arg()?;
            args.push(arg);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) { break; }
            self.skip_newlines();
        }
        self.no_trailing_lambda = prev_ntl;
        self.skip_newlines();
        self.expect(&TokenKind::RParen)?;
        Ok((type_args, args))
    }

    /// Issue #61: probe whether the current position begins a named tuple
    /// element (`name:` followed by an expression-start token). Returns
    /// the name without consuming it. The `:` token is also used for type
    /// annotations and map literals, so we match a conservative set of
    /// follow-tokens that can start an expression.
    fn try_peek_tuple_element_name(&self) -> Option<String> {
        let name = match self.peek() {
            TokenKind::Identifier(n) => n.clone(),
            _ => return None,
        };
        if !matches!(self.peek_at(1), Some(TokenKind::Colon)) {
            return None;
        }
        match self.peek_at(2) {
            Some(TokenKind::Identifier(_))
            | Some(TokenKind::IntLiteral(_))
            | Some(TokenKind::FloatLiteral(_))
            | Some(TokenKind::StringLiteral(_))
            | Some(TokenKind::StringStart(_))
            | Some(TokenKind::BoolTrue)
            | Some(TokenKind::BoolFalse)
            | Some(TokenKind::Null)
            | Some(TokenKind::This)
            | Some(TokenKind::LParen)
            | Some(TokenKind::LBracket)
            | Some(TokenKind::Minus)
            | Some(TokenKind::Bang) => Some(name),
            _ => None,
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let span = self.peek_span();
        // v5 Sprint 6: `throw expr` is allowed in expression position
        // (e.g. `val rb = body ?? throw Exception("missing")`).
        if self.check(&TokenKind::Throw) {
            self.advance();
            let exc = self.parse_expr()?;
            return Ok(Expr::ThrowExpr {
                exception: Box::new(exc),
                span: Span { start: span.start, end: self.peek_span().end },
            });
        }
        // Issue #49: `try { ... } catch (e: T) { ... }` as expression.
        // Recognized only in expression position and requires exactly
        // one catch clause (the lang-4 spec shape). Lowered downstream
        // by wrapping the try block in a helper that returns the last
        // expression of whichever arm executed.
        if self.check(&TokenKind::Try) {
            return self.parse_try_expr();
        }
        // v5 (deferred): `stackalloc[Type](size)` primary expression.
        // Recognized only as the contextual identifier `stackalloc`
        // followed by `[`.
        if self.check_contextual("stackalloc")
            && matches!(self.tokens.get(self.pos + 1).map(|t| t.kind.clone()), Some(TokenKind::LBracket))
        {
            self.advance(); // consume 'stackalloc'
            self.advance(); // consume '['
            let element_ty = self.parse_type()?;
            self.expect(&TokenKind::RBracket)?;
            self.expect(&TokenKind::LParen)?;
            let size = self.parse_expr()?;
            self.expect(&TokenKind::RParen)?;
            return Ok(Expr::StackAlloc {
                element_ty,
                size: Box::new(size),
                span: Span { start: span.start, end: self.peek_span().end },
            });
        }
        match self.peek().clone() {
            TokenKind::IntLiteral(n) => { self.advance(); Ok(Expr::IntLit(n, span)) }
            TokenKind::FloatLiteral(n) => { self.advance(); Ok(Expr::FloatLit(n, span)) }
            TokenKind::DurationLiteral(n) => { self.advance(); Ok(Expr::DurationLit(n, span)) }
            TokenKind::BoolTrue => { self.advance(); Ok(Expr::BoolLit(true, span)) }
            TokenKind::BoolFalse => { self.advance(); Ok(Expr::BoolLit(false, span)) }
            TokenKind::Null => { self.advance(); Ok(Expr::Null(span)) }
            TokenKind::This => { self.advance(); Ok(Expr::This(span)) }
            TokenKind::StringLiteral(s) => { self.advance(); Ok(Expr::StringLit(s, span)) }
            TokenKind::StringStart(s) => self.parse_string_interp(s),
            TokenKind::If => self.parse_if_expr(),
            TokenKind::When => self.parse_when_expr(),
            TokenKind::Identifier(name) => {
                // Language 5, Sprint 2: contextual `nameof(target)` expression.
                // Recognized only when immediately followed by `(`. The parsed
                // path is a dotted identifier sequence: `nameof(player.hp)`.
                if name == "nameof" {
                    let next_kind = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
                    if next_kind == Some(TokenKind::LParen) {
                        return self.parse_nameof_expr(span);
                    }
                }
                self.advance();
                Ok(Expr::Ident(name, span))
            }
            TokenKind::Require => {
                self.advance();
                Ok(Expr::Ident("require".into(), span))
            }
            TokenKind::Child => {
                self.advance();
                Ok(Expr::Ident("child".into(), span))
            }
            TokenKind::Parent => {
                self.advance();
                Ok(Expr::Ident("parent".into(), span))
            }
            // Issue #14: lifecycle keywords (`start`, `stop`, `update`,
            // ...) sometimes appear as field / parameter names in user
            // code (the lang-5 `ref struct Slice(start: Int, length: Int)`
            // example references `start` from inside the function body).
            // Statement forms like `start spawn()` are already consumed
            // by `parse_stmt` before expression parsing begins, so it is
            // safe to fall back to an identifier here.
            //
            // The set is restricted to keywords that have no expression-
            // level meaning. Control-flow keywords (`if`, `when`, `for`,
            // `while`, `throw`, `try`), value keywords (`null`, `this`,
            // `true`, `false`), and type-form keywords (`is`, `as`,
            // `in`, `until`, `downTo`, `step`) retain their normal role.
            TokenKind::Start
            | TokenKind::Stop
            | TokenKind::StopAll
            | TokenKind::Awake
            | TokenKind::Update
            | TokenKind::FixedUpdate
            | TokenKind::LateUpdate
            | TokenKind::OnEnable
            | TokenKind::OnDisable
            | TokenKind::OnDestroy
            | TokenKind::OnTriggerEnter
            | TokenKind::OnTriggerExit
            | TokenKind::OnTriggerStay
            | TokenKind::OnCollisionEnter
            | TokenKind::OnCollisionExit
            | TokenKind::OnCollisionStay
            | TokenKind::Wait
            | TokenKind::NextFrame
            | TokenKind::FixedFrame
            | TokenKind::Listen
            | TokenKind::Unlisten
            | TokenKind::Manual
            | TokenKind::Pool
            | TokenKind::Singleton
            | TokenKind::Serialize
            | TokenKind::Optional => {
                let text = self.peek().keyword_text().unwrap().to_string();
                self.advance();
                Ok(Expr::Ident(text, span))
            }
            TokenKind::LParen => {
                self.advance();
                // Issue #50: a parenthesized subexpression is a fresh
                // lambda context — nested calls inside `(...)` may still
                // take trailing lambdas, even when the outer context
                // disabled them (e.g. `if (list.filter { it > 10 }).any() { ... }`).
                let prev_ntl = self.no_trailing_lambda;
                self.no_trailing_lambda = false;
                // Issue #61: named tuple literal `(hp: 100, mp: 50)` —
                // detect a leading `name:` element before committing to
                // the positional path. The probe: an identifier followed
                // by `:` followed by something that can start an
                // expression. If we see that shape, parse as a named
                // tuple. Otherwise fall through to the positional path.
                let (first_name, first_expr) = if let Some(name) = self.try_peek_tuple_element_name() {
                    let name_token_pos = self.pos;
                    self.advance(); // consume name
                    self.advance(); // consume ':'
                    // Guard: if this is actually a type annotation on a
                    // single expression like `(x: Int)` in an unusual
                    // position, bail. The simple strategy — just parse an
                    // expression — works because `:` starting a type
                    // annotation isn't legal here; we already committed.
                    let value = self.parse_expr()?;
                    let _ = name_token_pos;
                    (Some(name), value)
                } else {
                    let expr = self.parse_expr()?;
                    (None, expr)
                };

                // Check if this is a tuple: (expr, expr, ...)
                let result = if self.check(&TokenKind::Comma) {
                    let mut elements = vec![first_expr];
                    let mut names = vec![first_name];
                    while self.eat(&TokenKind::Comma) {
                        self.skip_newlines();
                        if self.check(&TokenKind::RParen) { break; }
                        if let Some(name) = self.try_peek_tuple_element_name() {
                            self.advance(); // name
                            self.advance(); // ':'
                            elements.push(self.parse_expr()?);
                            names.push(Some(name));
                        } else {
                            elements.push(self.parse_expr()?);
                            names.push(None);
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    let end = self.peek_span();
                    Ok(Expr::Tuple { elements, names, span: Span { start: span.start, end: end.end } })
                } else {
                    self.expect(&TokenKind::RParen)?;
                    Ok(first_expr)
                };
                self.no_trailing_lambda = prev_ntl;
                return result;
            }
            TokenKind::LBrace => self.parse_brace_expr(),
            TokenKind::LBracket => self.parse_list_literal(),
            _ => Err(self.error(format!("Expected expression, found {:?}", self.peek()))),
        }
    }

    /// Parse `nameof(target)` — Language 5, Sprint 2.
    ///
    /// The target must be a dotted identifier path (no method calls,
    /// generics, or expressions). The parsed path is later joined with
    /// `.` and emitted as a verbatim C# `nameof(...)` expression.
    fn parse_nameof_expr(&mut self, start: Span) -> Result<Expr, ParseError> {
        self.advance(); // consume 'nameof'
        self.expect(&TokenKind::LParen)?;
        let mut path = Vec::new();
        let (first, _) = self.expect_ident()?;
        path.push(first);
        while self.eat(&TokenKind::Dot) {
            let (next, _) = self.expect_ident()?;
            path.push(next);
        }
        self.expect(&TokenKind::RParen)?;
        Ok(Expr::NameOf {
            path,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Parse a `[` expression — list literal `[1, 2, 3]` or empty `[]`.
    fn parse_list_literal(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume '['
        self.skip_newlines();
        let mut elements = Vec::new();
        if !self.check(&TokenKind::RBracket) {
            loop {
                self.skip_newlines();
                if self.check(&TokenKind::RBracket) { break; }
                elements.push(self.parse_expr()?);
                self.skip_newlines();
                if !self.eat(&TokenKind::Comma) { break; }
            }
        }
        self.skip_newlines();
        self.expect(&TokenKind::RBracket)?;
        Ok(Expr::ListLit {
            elements,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    /// Parse a `{` expression — disambiguates between map literal and lambda.
    /// Map literal: `{key: value, key: value}` (key followed by `:` then value).
    /// Lambda: `{}`, `{ expr }`, `{ x => body }`, `{ x, y => body }`.
    fn parse_brace_expr(&mut self) -> Result<Expr, ParseError> {
        let saved = self.pos;
        let start = self.peek_span();
        self.advance(); // consume '{'
        self.skip_newlines();

        // Empty `{}` → empty lambda (existing behavior). Map literal cannot be
        // empty without an explicit type annotation; the validation happens at
        // semantic time, but the lexer/parser treat it as a lambda here so that
        // existing behavior is preserved.
        if self.check(&TokenKind::RBrace) {
            self.pos = saved;
            return self.parse_lambda_expr();
        }

        // Speculative parse: try to parse the first expression and check whether
        // it is followed by `:` (map entry separator). If so, this is a map
        // literal; otherwise restore and fall through to lambda parsing.
        let probe_pos = self.pos;
        let saved_errors = self.errors.len();
        if let Ok(first_key) = self.parse_expr() {
            if self.check(&TokenKind::Colon) {
                // Confirmed map literal.
                self.advance(); // consume ':'
                let first_value = self.parse_expr()?;
                let mut entries = vec![(first_key, first_value)];
                while self.eat(&TokenKind::Comma) {
                    self.skip_newlines();
                    if self.check(&TokenKind::RBrace) { break; }
                    let key = self.parse_expr()?;
                    self.expect(&TokenKind::Colon)?;
                    let value = self.parse_expr()?;
                    entries.push((key, value));
                    self.skip_newlines();
                }
                self.skip_newlines();
                self.expect(&TokenKind::RBrace)?;
                return Ok(Expr::MapLit {
                    entries,
                    span: Span { start: start.start, end: self.peek_span().end },
                });
            }
        }
        // Not a map literal — restore parser state and parse as a lambda.
        self.pos = saved;
        self.errors.truncate(saved_errors);
        let _ = probe_pos;
        self.parse_lambda_expr()
    }

    /// Parse a lambda expression: `{ }`, `{ expr }`, `{ x => expr }`, `{ x, y => body }`
    fn parse_lambda_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume '{'
        self.skip_newlines();

        // Empty lambda: { }
        if self.check(&TokenKind::RBrace) {
            self.advance();
            return Ok(Expr::Lambda {
                params: vec![],
                body: LambdaBody::Block(Block { stmts: vec![], span: start }),
                span: Span { start: start.start, end: self.peek_span().end },
            });
        }

        // Try to detect `params =>` pattern
        // Look ahead to see if we have `ident =>` or `ident, ident, ... =>`
        let saved = self.pos;
        let mut maybe_params = Vec::new();
        let mut found_arrow = false;

        loop {
            match self.peek().clone() {
                TokenKind::Identifier(name) => {
                    self.advance();
                    maybe_params.push(LambdaParam { name, ty: None });
                }
                _ => break,
            }
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }
        self.skip_newlines();
        if self.check(&TokenKind::FatArrow) {
            self.advance(); // consume '=>'
            self.skip_newlines();
            found_arrow = true;
        }

        if found_arrow && !maybe_params.is_empty() {
            // We have params => body
            // Parse body: either a single expression or statements until '}'
            if self.check(&TokenKind::RBrace) {
                // Empty body after =>
                self.advance();
                return Ok(Expr::Lambda {
                    params: maybe_params,
                    body: LambdaBody::Block(Block { stmts: vec![], span: start }),
                    span: Span { start: start.start, end: self.peek_span().end },
                });
            }
            // Try to parse as block (multiple statements)
            let mut stmts = Vec::new();
            while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                match self.parse_stmt() {
                    Ok(s) => stmts.push(s),
                    Err(e) => { self.errors.push(e); self.recover_to_newline(); }
                }
                self.skip_newlines();
            }
            self.expect(&TokenKind::RBrace)?;
            let body_span = Span { start: start.start, end: self.peek_span().end };
            // If single expression statement, use Expr body
            if stmts.len() == 1 {
                if let Stmt::Expr { expr, .. } = &stmts[0] {
                    return Ok(Expr::Lambda {
                        params: maybe_params,
                        body: LambdaBody::Expr(Box::new(expr.clone())),
                        span: Span { start: start.start, end: self.peek_span().end },
                    });
                }
            }
            return Ok(Expr::Lambda {
                params: maybe_params,
                body: LambdaBody::Block(Block { stmts, span: body_span }),
                span: Span { start: start.start, end: self.peek_span().end },
            });
        }

        // Not a params => body lambda. Backtrack and parse as implicit `it` lambda.
        self.pos = saved;

        // Parse body statements
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            match self.parse_stmt() {
                Ok(s) => stmts.push(s),
                Err(e) => { self.errors.push(e); self.recover_to_newline(); }
            }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;
        let body_span = Span { start: start.start, end: self.peek_span().end };

        // Single expression: use Expr body, implicit `it`
        if stmts.len() == 1 {
            if let Stmt::Expr { expr, .. } = &stmts[0] {
                return Ok(Expr::Lambda {
                    params: vec![],
                    body: LambdaBody::Expr(Box::new(expr.clone())),
                    span: Span { start: start.start, end: self.peek_span().end },
                });
            }
        }

        Ok(Expr::Lambda {
            params: vec![],
            body: LambdaBody::Block(Block { stmts, span: body_span }),
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_string_interp(&mut self, first_text: String) -> Result<Expr, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume StringStart

        let mut parts = Vec::new();
        if !first_text.is_empty() {
            parts.push(StringPart::Literal(first_text));
        }

        // Parse the expression inside ${...}
        let expr = self.parse_expr()?;
        parts.push(StringPart::Expr(expr));

        // Continue: we expect StringMiddle or StringEnd
        loop {
            match self.peek().clone() {
                TokenKind::StringEnd(text) => {
                    self.advance();
                    if !text.is_empty() {
                        parts.push(StringPart::Literal(text));
                    }
                    break;
                }
                TokenKind::StringMiddle(text) => {
                    self.advance();
                    if !text.is_empty() {
                        parts.push(StringPart::Literal(text));
                    }
                    let expr = self.parse_expr()?;
                    parts.push(StringPart::Expr(expr));
                }
                _ => {
                    return Err(self.error("Expected string continuation or end".into()));
                }
            }
        }

        Ok(Expr::StringInterp { parts, span: Span { start: start.start, end: self.peek_span().end } })
    }

    // ── Type parsing ─────────────────────────────────────────────

    // Issue #36: parse a single tuple-type element, with the
    // optional `name:` prefix from named tuple syntax. Returns
    // `(Some(name), ty)` for `name: Type`, `(None, ty)` otherwise.
    fn parse_tuple_element_type(&mut self) -> Result<(Option<String>, TypeRef), ParseError> {
        let is_named = matches!(self.peek(), TokenKind::Identifier(_))
            && matches!(self.peek_at(1), Some(TokenKind::Colon));
        if is_named {
            let (name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            Ok((Some(name), ty))
        } else {
            let ty = self.parse_type()?;
            Ok((None, ty))
        }
    }

    fn parse_type(&mut self) -> Result<TypeRef, ParseError> {
        let start = self.peek_span();

        // Tuple type or function type: (Int, String) or (Int) => Bool
        if self.check(&TokenKind::LParen) {
            self.advance(); // consume '('
            // Handle empty parens: () => ReturnType
            if self.check(&TokenKind::RParen) {
                self.advance(); // consume ')'
                if self.check(&TokenKind::FatArrow) {
                    self.advance(); // consume '=>'
                    let return_type = self.parse_type()?;
                    let nullable = self.eat(&TokenKind::Question);
                    return Ok(TypeRef::Function {
                        param_types: vec![],
                        return_type: Box::new(return_type),
                        nullable,
                        span: Span { start: start.start, end: self.peek_span().end },
                    });
                }
                // Empty parens without => — error or treat as unit
                return Err(self.error("Expected '=>' after '()' in type position".into()));
            }
            // Issue #36: accept an optional `name:` prefix on each
            // element so lang-4 named tuple types
            // (`(hp: Int, mp: Int)`) parse without E100. We detect
            // the named form by peeking for `<ident>:` before parsing
            // each element.
            let (first_name, first) = self.parse_tuple_element_type()?;
            if self.check(&TokenKind::Comma) {
                let mut types = vec![first];
                let mut names = vec![first_name];
                while self.eat(&TokenKind::Comma) {
                    if self.check(&TokenKind::RParen) { break; }
                    let (n, t) = self.parse_tuple_element_type()?;
                    names.push(n);
                    types.push(t);
                }
                self.expect(&TokenKind::RParen)?;
                // Check for function type: (Int, Int) => Bool
                if self.check(&TokenKind::FatArrow) {
                    self.advance(); // consume '=>'
                    let return_type = self.parse_type()?;
                    let nullable = self.eat(&TokenKind::Question);
                    return Ok(TypeRef::Function {
                        param_types: types,
                        return_type: Box::new(return_type),
                        nullable,
                        span: Span { start: start.start, end: self.peek_span().end },
                    });
                }
                let nullable = self.eat(&TokenKind::Question);
                return Ok(TypeRef::Tuple {
                    types,
                    names,
                    nullable,
                    span: Span { start: start.start, end: self.peek_span().end },
                });
            } else {
                // Single type in parens
                self.expect(&TokenKind::RParen)?;
                // Check for function type: (Int) => Bool
                if self.check(&TokenKind::FatArrow) {
                    self.advance(); // consume '=>'
                    let return_type = self.parse_type()?;
                    let nullable = self.eat(&TokenKind::Question);
                    return Ok(TypeRef::Function {
                        param_types: vec![first],
                        return_type: Box::new(return_type),
                        nullable,
                        span: Span { start: start.start, end: self.peek_span().end },
                    });
                }
                return Ok(first);
            }
        }

        let (name, _) = self.expect_ident()?;

        // Check for qualified: Name.SubName
        if self.check(&TokenKind::Dot) {
            let save = self.pos;
            self.advance();
            if let Ok((sub, _)) = self.expect_ident() {
                let nullable = self.eat(&TokenKind::Question);
                return Ok(TypeRef::Qualified {
                    qualifier: name,
                    name: sub,
                    nullable,
                    span: Span { start: start.start, end: self.peek_span().end },
                });
            } else {
                self.pos = save;
            }
        }

        // Check for generic: Name<T, U>
        if self.check(&TokenKind::Lt) {
            let save = self.pos;
            self.advance();
            let mut type_args = Vec::new();
            let ok = loop {
                match self.parse_type() {
                    Ok(t) => type_args.push(t),
                    Err(_) => { self.pos = save; break false; }
                }
                if !self.eat(&TokenKind::Comma) { break true; }
            };
            if ok {
                if self.eat(&TokenKind::Gt) {
                    let nullable = self.eat(&TokenKind::Question);
                    return Ok(TypeRef::Generic {
                        name,
                        type_args,
                        nullable,
                        span: Span { start: start.start, end: self.peek_span().end },
                    });
                } else {
                    self.pos = save;
                }
            }
        }

        let nullable = self.eat(&TokenKind::Question);
        Ok(TypeRef::Simple {
            name,
            nullable,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    // ── Parameter parsing ────────────────────────────────────────

    fn parse_param_list(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            params.push(self.parse_param()?);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) { break; }
            self.skip_newlines();
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let start = self.peek_span();
        // v5 Sprint 2: optional `ref` / `out` / `vararg` modifier in front
        // of the parameter name. The modifier and the vararg flag are
        // contextual — they must be recognized only as identifiers in
        // parameter position so they don't break existing PrSM code that
        // uses these names as ordinary identifiers elsewhere.
        let mut modifier = ParamMod::None;
        let mut is_vararg = false;
        // The modifier must be followed by an identifier and a colon
        // (`ref name:`) — peek ahead to disambiguate from a parameter
        // literally named `ref` / `out` / `vararg`.
        let next_after = |this: &Self| -> TokenKind {
            let mut p = this.pos + 1;
            while p < this.tokens.len() && this.tokens[p].kind == TokenKind::Newline {
                p += 1;
            }
            this.tokens.get(p).map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof)
        };
        // Issue #4: parameter names may legitimately collide with PrSM
        // keywords (`ref struct Slice(start: Int, length: Int)` from the
        // lang-5 spec). Allow either an identifier or a keyword token in
        // the parameter-name slot. The keyword's normal meaning is not
        // in play here because the parser is already inside a parameter
        // list, where only a name-colon-type sequence is valid.
        //
        // The disambiguation for the `ref` / `out` / `vararg` modifiers
        // accepts a following identifier *or* a keyword (since the
        // parameter name itself is now allowed to be a keyword).
        let looks_like_param_name = |kind: &TokenKind| -> bool {
            matches!(kind, TokenKind::Identifier(_)) || kind.keyword_text().is_some()
        };
        if self.check_contextual("ref") && looks_like_param_name(&next_after(self)) {
            self.advance();
            modifier = ParamMod::Ref;
        } else if self.check_contextual("out") && looks_like_param_name(&next_after(self)) {
            self.advance();
            modifier = ParamMod::Out;
        } else if self.check_contextual("vararg") && looks_like_param_name(&next_after(self)) {
            self.advance();
            is_vararg = true;
        }
        let (name, name_span) = self.expect_ident_or_keyword()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let default = if self.eat(&TokenKind::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(Param {
            name,
            name_span,
            ty,
            default,
            modifier,
            is_vararg,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    // ── Raw brace block (for intrinsic) ──────────────────────────

    fn parse_raw_brace_block(&mut self) -> Result<String, ParseError> {
        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;

        // Scan raw text by tracking brace depth in the original token stream
        // We collect all tokens until we find the matching }
        let mut depth = 1u32;
        let mut code = String::new();
        let mut previous_kind: Option<TokenKind> = None;

        while !self.is_at_end_of_tokens() {
            match self.peek() {
                TokenKind::LBrace => {
                    depth += 1;
                    code.push('{');
                    self.advance();
                    previous_kind = Some(TokenKind::LBrace);
                }
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        self.advance(); // consume final }
                        break;
                    }
                    code.push('}');
                    self.advance();
                    previous_kind = Some(TokenKind::RBrace);
                }
                TokenKind::Newline => {
                    code.push('\n');
                    self.advance();
                    previous_kind = None;
                }
                TokenKind::Eof => {
                    return Err(self.error("Unterminated intrinsic block".into()));
                }
                _ => {
                    let tok = self.advance();
                    let tok_kind = tok.kind.clone();
                    if should_insert_raw_space(previous_kind.as_ref(), &tok_kind) {
                        code.push(' ');
                    }

                    // PascalCase method calls in intrinsic blocks:
                    // `.identifier(` → method call on a PrSM-generated type
                    // whose methods are PascalCase in C#. Apply the same
                    // transformation so users can write camelCase in
                    // intrinsic blocks. Field/property access (no `(`) is
                    // left as-is since PrSM properties keep their casing.
                    let is_method_call = matches!(
                        &previous_kind,
                        Some(TokenKind::Dot) | Some(TokenKind::QuestionDot)
                    ) && matches!(&tok_kind, TokenKind::Identifier(_))
                      && matches!(self.peek(), TokenKind::LParen);

                    if is_method_call {
                        if let TokenKind::Identifier(name) = &tok_kind {
                            code.push_str(&raw_pascal_case(name));
                        } else {
                            code.push_str(&token_to_source_text(&tok_kind));
                        }
                    } else {
                        code.push_str(&token_to_source_text(&tok_kind));
                    }

                    previous_kind = Some(tok_kind);
                }
            }
        }

        Ok(code.trim().to_string())
    }

    fn is_at_end_of_tokens(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].kind == TokenKind::Eof
    }

    // ── Error recovery ───────────────────────────────────────────

    fn recover_to_newline(&mut self) {
        while !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof | TokenKind::RBrace) {
            self.advance();
        }
        if self.peek() == &TokenKind::Newline {
            self.advance();
        }
    }
}

// ── Helper: reconstruct source text from token ───────────────────

/// Given the parsed `args` of an `@field(SerializeField)` / `@return(NotNull)` etc.
/// annotation, extract the first argument as the C# attribute identifier and
/// return the remaining arguments. Returns `None` if the first argument is
/// not an identifier expression.
fn extract_target_attr_name(args: &[Expr]) -> (Option<String>, Vec<Expr>) {
    if args.is_empty() {
        return (None, vec![]);
    }
    let first = match &args[0] {
        Expr::Ident(name, _) => Some(pascal_attr_case(name)),
        Expr::Call { name, .. } => Some(pascal_attr_case(name)),
        _ => None,
    };
    let rest = args.iter().skip(1).cloned().collect::<Vec<_>>();
    (first, rest)
}

/// Return the span of a preprocessor condition node — used by the parser
/// to construct enclosing `And`/`Or` spans without rebuilding them from
/// the operator position. The span is conservative; it points at the
/// left-most operand and the diagnostic positions remain stable for tests.
fn pp_cond_span(cond: &PreprocessorCond) -> Span {
    match cond {
        PreprocessorCond::Symbol { span, .. } => *span,
        PreprocessorCond::Not { span, .. } => *span,
        PreprocessorCond::And { span, .. } => *span,
        PreprocessorCond::Or { span, .. } => *span,
    }
}

/// Convert a PrSM-cased attribute name (e.g. `serializeField`, `nonSerialized`)
/// to its canonical C# form (e.g. `SerializeField`, `NonSerialized`) by
/// upper-casing the first character.  Names that already begin with an
/// upper-case letter are passed through unchanged.
fn pascal_attr_case(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) => {
            let mut out = String::new();
            for ch in c.to_uppercase() {
                out.push(ch);
            }
            out.extend(chars);
            out
        }
        None => String::new(),
    }
}

/// Inside a string literal that appears in an intrinsic block, PascalCase
/// any `.camelCase(` method-call pattern. This handles C# string
/// interpolation like `$"HP: {player.getHp()}"` where `getHp` needs to
/// become `GetHp` to match the PascalCase-lowered definition.
fn pascal_case_methods_in_string(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_lowercase() {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            if end < chars.len() && chars[end] == '(' {
                out.push('.');
                out.push(chars[start].to_ascii_uppercase());
                for j in (start + 1)..end {
                    out.push(chars[j]);
                }
                i = end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// PascalCase a camelCase name (first char to uppercase).
/// Used by `parse_raw_brace_block` to convert method calls in intrinsic
/// blocks to match PrSM's PascalCase lowering convention.
fn raw_pascal_case(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn token_to_source_text(kind: &TokenKind) -> String {
    match kind {
        TokenKind::IntLiteral(n) => n.to_string(),
        TokenKind::FloatLiteral(n) => format!("{n}f"),
        TokenKind::DurationLiteral(n) => format!("{n}s"),
        TokenKind::BoolTrue => "true".into(),
        TokenKind::BoolFalse => "false".into(),
        TokenKind::StringLiteral(s) => format!("\"{}\"", pascal_case_methods_in_string(s)),
        TokenKind::Identifier(s) => s.clone(),
        TokenKind::Using => "using".into(),
        TokenKind::Val => "val".into(),
        TokenKind::Var => "var".into(),
        TokenKind::Const => "const".into(),
        TokenKind::Fixed => "fixed".into(),
        TokenKind::Public => "public".into(),
        TokenKind::Private => "private".into(),
        TokenKind::Protected => "protected".into(),
        TokenKind::Func => "func".into(),
        TokenKind::Return => "return".into(),
        TokenKind::If => "if".into(),
        TokenKind::Else => "else".into(),
        TokenKind::For => "for".into(),
        TokenKind::While => "while".into(),
        TokenKind::In => "in".into(),
        TokenKind::Break => "break".into(),
        TokenKind::Continue => "continue".into(),
        TokenKind::Null => "null".into(),
        TokenKind::This => "this".into(),
        TokenKind::Plus => "+".into(),
        TokenKind::Minus => "-".into(),
        TokenKind::Star => "*".into(),
        TokenKind::Slash => "/".into(),
        TokenKind::Percent => "%".into(),
        TokenKind::Eq => "=".into(),
        TokenKind::EqEq => "==".into(),
        TokenKind::NotEq => "!=".into(),
        TokenKind::Lt => "<".into(),
        TokenKind::Gt => ">".into(),
        TokenKind::LtEq => "<=".into(),
        TokenKind::GtEq => ">=".into(),
        TokenKind::AmpAmp => "&&".into(),
        TokenKind::PipePipe => "||".into(),
        TokenKind::Bang => "!".into(),
        TokenKind::BangBang => "!!".into(),
        TokenKind::Dot => ".".into(),
        TokenKind::QuestionDot => "?.".into(),
        TokenKind::Elvis => "?:".into(),
        TokenKind::Question => "?".into(),
        TokenKind::Colon => ":".into(),
        TokenKind::FatArrow => "=>".into(),
        TokenKind::DotDot => "..".into(),
        TokenKind::LParen => "(".into(),
        TokenKind::RParen => ")".into(),
        TokenKind::LBrace => "{".into(),
        TokenKind::RBrace => "}".into(),
        TokenKind::LBracket => "[".into(),
        TokenKind::RBracket => "]".into(),
        TokenKind::Comma => ",".into(),
        TokenKind::Semicolon => ";".into(),
        TokenKind::At => "@".into(),
        TokenKind::PlusEq => "+=".into(),
        TokenKind::MinusEq => "-=".into(),
        TokenKind::StarEq => "*=".into(),
        TokenKind::SlashEq => "/=".into(),
        TokenKind::PercentEq => "%=".into(),
        TokenKind::ElvisAssign => "??=".into(),
        TokenKind::Error(msg) => {
            // Intrinsic blocks are tokenised by the PrSM lexer, so C#-only
            // characters (e.g. `$` in `$"..."`) produce Error tokens.
            // Extract the original character and emit it verbatim so the
            // raw C# code round-trips correctly.
            if let Some(ch) = msg
                .strip_prefix("Unexpected character: '")
                .and_then(|s| s.strip_suffix("'"))
            {
                ch.to_string()
            } else {
                format!("/* {} */", msg)
            }
        }
        _ => format!("{:?}", kind),
    }
}

fn should_insert_raw_space(previous: Option<&TokenKind>, current: &TokenKind) -> bool {
    let Some(previous) = previous else {
        return false;
    };

    if matches!(
        current,
        TokenKind::Dot
            | TokenKind::QuestionDot
            | TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
    ) {
        return false;
    }

    if matches!(
        previous,
        TokenKind::Dot | TokenKind::QuestionDot | TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace
    ) {
        return false;
    }

    // Error tokens from the lexer represent C#-only characters (e.g. `$`)
    // that must glue directly to the next token without whitespace.
    // Example: `$"hello"` must not become `$ "hello"`.
    if matches!(previous, TokenKind::Error(_)) {
        return false;
    }

    if matches!(current, TokenKind::LParen) {
        return !matches!(
            previous,
            TokenKind::Identifier(_)
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::This
                | TokenKind::Null
        );
    }

    true
}

// ── Expr span helper ─────────────────────────────────────────────

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit(_, s) | Expr::FloatLit(_, s) | Expr::DurationLit(_, s)
            | Expr::StringLit(_, s) | Expr::BoolLit(_, s) | Expr::Null(s)
            | Expr::Ident(_, s) | Expr::This(s) => *s,
            Expr::StringInterp { span, .. } | Expr::Binary { span, .. }
            | Expr::Unary { span, .. } | Expr::MemberAccess { span, .. }
            | Expr::SafeCall { span, .. } | Expr::SafeMethodCall { span, .. }
            | Expr::NonNullAssert { span, .. }
            | Expr::Elvis { span, .. } | Expr::Call { span, .. }
            | Expr::IndexAccess { span, .. } | Expr::IfExpr { span, .. }
            | Expr::WhenExpr { span, .. } | Expr::Range { span, .. }
            | Expr::Is { span, .. } | Expr::Lambda { span, .. }
            | Expr::IntrinsicExpr { span, .. }
            | Expr::SafeCastExpr { span, .. }
            | Expr::ForceCastExpr { span, .. }
            | Expr::Tuple { span, .. }
            | Expr::ListLit { span, .. }
            | Expr::MapLit { span, .. }
            | Expr::Await { span, .. }
            | Expr::NameOf { span, .. }
            | Expr::RefOf { span, .. }
            | Expr::SafeIndexAccess { span, .. }
            | Expr::ThrowExpr { span, .. }
            | Expr::With { span, .. }
            | Expr::StackAlloc { span, .. }
            | Expr::TryExpr { span, .. } => *span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lexer::Lexer;

    fn parse(input: &str) -> (File, Vec<ParseError>) {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let file = parser.parse_file();
        let errors = parser.errors().to_vec();
        (file, errors)
    }

    fn parse_ok(input: &str) -> File {
        let (file, errors) = parse(input);
        if !errors.is_empty() {
            panic!("Parse errors: {:?}", errors);
        }
        file
    }

    // === Using declarations ===

    #[test]
    fn test_using_single() {
        let file = parse_ok("using UnityEngine\ncomponent Foo : MonoBehaviour {}");
        assert_eq!(file.usings.len(), 1);
        assert_eq!(file.usings[0].path, "UnityEngine");
    }

    #[test]
    fn test_using_qualified() {
        let file = parse_ok("using UnityEngine.UI\ncomponent Foo : MonoBehaviour {}");
        assert_eq!(file.usings[0].path, "UnityEngine.UI");
    }

    #[test]
    fn test_multiple_usings() {
        let file = parse_ok("using UnityEngine\nusing UnityEngine.UI\ncomponent Foo : MonoBehaviour {}");
        assert_eq!(file.usings.len(), 2);
    }

    // === Component declaration ===

    #[test]
    fn test_empty_component() {
        let file = parse_ok("component Player : MonoBehaviour {}");
        match &file.decl {
            Decl::Component { name, base_class, .. } => {
                assert_eq!(name, "Player");
                assert_eq!(base_class, "MonoBehaviour");
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_component_with_serialize() {
        let file = parse_ok("component Player : MonoBehaviour {\n  serialize speed: Float = 5.0\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                assert_eq!(members.len(), 1);
                match &members[0] {
                    Member::SerializeField { name, .. } => assert_eq!(name, "speed"),
                    _ => panic!("Expected SerializeField"),
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_component_with_require() {
        let file = parse_ok("component Player : MonoBehaviour {\n  require rb: Rigidbody\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                assert_eq!(members.len(), 1);
                match &members[0] {
                    Member::Require { name, .. } => assert_eq!(name, "rb"),
                    _ => panic!("Expected Require"),
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_component_with_lifecycle() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  update {\n    move()\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                assert_eq!(members.len(), 1);
                match &members[0] {
                    Member::Lifecycle { kind, .. } => assert_eq!(*kind, LifecycleKind::Update),
                    _ => panic!("Expected Lifecycle"),
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_lifecycle_with_param() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  onTriggerEnter(other: Collider) {\n    print(other)\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                match &members[0] {
                    Member::Lifecycle { kind, params, .. } => {
                        assert_eq!(*kind, LifecycleKind::OnTriggerEnter);
                        assert_eq!(params.len(), 1);
                        assert_eq!(params[0].name, "other");
                    }
                    _ => panic!("Expected Lifecycle"),
                }
            }
            _ => panic!("Expected component"),
        }
    }

    // === Asset declaration ===

    #[test]
    fn test_asset() {
        let file = parse_ok("asset WeaponData : ScriptableObject {\n  serialize damage: Int = 10\n}");
        match &file.decl {
            Decl::Asset { name, base_class, members, .. } => {
                assert_eq!(name, "WeaponData");
                assert_eq!(base_class, "ScriptableObject");
                assert_eq!(members.len(), 1);
            }
            _ => panic!("Expected asset"),
        }
    }

    // === Class declaration ===

    #[test]
    fn test_simple_class() {
        let file = parse_ok("class Helper {\n  func doStuff() {\n  }\n}");
        match &file.decl {
            Decl::Class { name, super_class, interfaces, .. } => {
                assert_eq!(name, "Helper");
                assert!(super_class.is_none());
                assert!(interfaces.is_empty());
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn test_component_with_interfaces() {
        let file = parse_ok("component Foo : MonoBehaviour, IFoo, IBar {\n}");
        match &file.decl {
            Decl::Component { base_class, interfaces, .. } => {
                assert_eq!(base_class, "MonoBehaviour");
                assert_eq!(interfaces, &vec!["IFoo".to_string(), "IBar".to_string()]);
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_class_with_interfaces() {
        let file = parse_ok("class Helper : BaseHelper, IDisposable, IComparable {\n}\n");
        match &file.decl {
            Decl::Class { super_class, interfaces, .. } => {
                assert_eq!(super_class.as_deref(), Some("BaseHelper"));
                assert_eq!(interfaces, &vec!["IDisposable".to_string(), "IComparable".to_string()]);
            }
            _ => panic!("Expected class"),
        }
    }

    // === Data class ===

    #[test]
    fn test_data_class() {
        let file = parse_ok("data class DamageInfo(\n  val amount: Int,\n  val crit: Bool\n)");
        match &file.decl {
            Decl::DataClass { name, fields, .. } => {
                assert_eq!(name, "DamageInfo");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "amount");
                assert_eq!(fields[1].name, "crit");
            }
            _ => panic!("Expected data class"),
        }
    }

    // === Enum declaration ===

    #[test]
    fn test_simple_enum() {
        let file = parse_ok("enum EnemyState {\n  Idle,\n  Chase,\n  Attack\n}");
        match &file.decl {
            Decl::Enum { name, entries, .. } => {
                assert_eq!(name, "EnemyState");
                assert_eq!(entries.len(), 3);
                assert_eq!(entries[0].name, "Idle");
                assert_eq!(entries[1].name, "Chase");
                assert_eq!(entries[2].name, "Attack");
            }
            _ => panic!("Expected enum"),
        }
    }

    #[test]
    fn test_parameterized_enum() {
        let file = parse_ok("enum Weapon(val damage: Int) {\n  Sword(10),\n  Bow(7)\n}");
        match &file.decl {
            Decl::Enum { name, params, entries, .. } => {
                assert_eq!(name, "Weapon");
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "damage");
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].args.len(), 1);
            }
            _ => panic!("Expected enum"),
        }
    }

    // === Function declaration ===

    #[test]
    fn test_func_block_body() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  func move() {\n    print(1)\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                match &members[0] {
                    Member::Func { name, body, .. } => {
                        assert_eq!(name, "move");
                        assert!(matches!(body, FuncBody::Block(_)));
                    }
                    _ => panic!("Expected func"),
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_func_expr_body() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  func isDead(): Bool = hp <= 0\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                match &members[0] {
                    Member::Func { name, body, return_ty, .. } => {
                        assert_eq!(name, "isDead");
                        assert!(return_ty.is_some());
                        assert!(matches!(body, FuncBody::ExprBody(_)));
                    }
                    _ => panic!("Expected func"),
                }
            }
            _ => panic!("Expected component"),
        }
    }

    // === Coroutine ===

    #[test]
    fn test_coroutine() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  coroutine blink() {\n    wait 0.2s\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                match &members[0] {
                    Member::Coroutine { name, .. } => assert_eq!(name, "blink"),
                    _ => panic!("Expected coroutine"),
                }
            }
            _ => panic!("Expected component"),
        }
    }

    // === Statements ===

    #[test]
    fn test_val_decl() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  update {\n    val x = 5\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Lifecycle { body, .. } = &members[0] {
                    assert_eq!(body.stmts.len(), 1);
                    assert!(matches!(&body.stmts[0], Stmt::ValDecl { name, .. } if name == "x"));
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_if_else() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  update {\n    if hp <= 0 {\n      die()\n    } else {\n      run()\n    }\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Lifecycle { body, .. } = &members[0] {
                    assert!(matches!(&body.stmts[0], Stmt::If { else_branch: Some(_), .. }));
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_if_expr() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  func score(): Int = if hp <= 0 { 0 } else { 100 }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                match &members[0] {
                    Member::Func { body: FuncBody::ExprBody(Expr::IfExpr { .. }), .. } => {}
                    _ => panic!("Expected if expression body"),
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_for_loop() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  update {\n    for enemy in enemies {\n      attack(enemy)\n    }\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Lifecycle { body, .. } = &members[0] {
                    assert!(matches!(&body.stmts[0], Stmt::For { var_name, .. } if var_name == "enemy"));
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_when_stmt() {
        let src = "component Foo : MonoBehaviour {\n  update {\n    when state {\n      State.Idle => idle()\n      else => attack()\n    }\n  }\n}";
        let file = parse_ok(src);
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Lifecycle { body, .. } = &members[0] {
                    if let Stmt::When { branches, .. } = &body.stmts[0] {
                        assert_eq!(branches.len(), 2);
                    } else {
                        panic!("Expected when");
                    }
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_when_expr() {
        let src = "component Foo : MonoBehaviour {\n  func score(): Int = when state {\n    State.Idle => 0\n    else => 100\n  }\n}";
        let file = parse_ok(src);
        match &file.decl {
            Decl::Component { members, .. } => {
                match &members[0] {
                    Member::Func { body: FuncBody::ExprBody(Expr::WhenExpr { branches, .. }), .. } => {
                        assert_eq!(branches.len(), 2);
                    }
                    _ => panic!("Expected when expression body"),
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_wait_duration() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  coroutine blink() {\n    wait 1.0s\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Coroutine { body, .. } = &members[0] {
                    assert!(matches!(&body.stmts[0], Stmt::Wait { form: WaitForm::Duration(_), .. }));
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_wait_next_frame() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  coroutine blink() {\n    wait nextFrame\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Coroutine { body, .. } = &members[0] {
                    assert!(matches!(&body.stmts[0], Stmt::Wait { form: WaitForm::NextFrame, .. }));
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_wait_until() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  coroutine waitDead() {\n    wait until hp <= 0\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Coroutine { body, .. } = &members[0] {
                    assert!(matches!(&body.stmts[0], Stmt::Wait { form: WaitForm::Until(_), .. }));
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_start_stop() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  func go() {\n    start blink()\n    stopAll()\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Func { body: FuncBody::Block(block), .. } = &members[0] {
                    assert!(matches!(&block.stmts[0], Stmt::Start { .. }));
                    assert!(matches!(&block.stmts[1], Stmt::StopAll { .. }));
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_listen_stmt() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  start {\n    listen button.onClick {\n      play()\n    }\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Lifecycle { body, .. } = &members[0] {
                    assert!(matches!(&body.stmts[0], Stmt::Listen { .. }));
                }
            }
            _ => panic!("Expected component"),
        }
    }

    // === Expressions ===

    #[test]
    fn test_binary_expr() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  func f() {\n    val x = a + b * c\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Func { body: FuncBody::Block(block), .. } = &members[0] {
                    if let Stmt::ValDecl { init, .. } = &block.stmts[0] {
                        // a + (b * c) — mul has higher precedence
                        assert!(matches!(init, Expr::Binary { op: BinOp::Add, .. }));
                    }
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_safe_call() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  func f() {\n    animator?.play(\"Run\")\n  }\n}");
        // Should parse without errors
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Func { body: FuncBody::Block(block), .. } = &members[0] {
                    assert_eq!(block.stmts.len(), 1);
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_elvis_expr() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  func f() {\n    val name = playerName ?: \"Unknown\"\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Func { body: FuncBody::Block(block), .. } = &members[0] {
                    if let Stmt::ValDecl { init, .. } = &block.stmts[0] {
                        assert!(matches!(init, Expr::Elvis { .. }));
                    }
                }
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_assignment() {
        let file = parse_ok("component Foo : MonoBehaviour {\n  func f() {\n    hp -= 10\n  }\n}");
        match &file.decl {
            Decl::Component { members, .. } => {
                if let Member::Func { body: FuncBody::Block(block), .. } = &members[0] {
                    assert!(matches!(&block.stmts[0], Stmt::Assignment { op: AssignOp::MinusAssign, .. }));
                }
            }
            _ => panic!("Expected component"),
        }
    }

    // === Full sample parsing ===

    #[test]
    fn test_parse_player_controller_sample() {
        let src = r#"using UnityEngine

component PlayerController : MonoBehaviour {
    @header("Movement")
    serialize speed: Float = 5.0
    serialize jumpForce: Float = 8.0

    require rb: Rigidbody
    optional animator: Animator

    update {
        val h = input.axis("Horizontal")
        val v = input.axis("Vertical")
        val move = vec3(h, 0, v)
        rb.velocity = move * speed
    }

    func jump() {
        rb.addForce(vec3(0, jumpForce, 0))
        animator?.play("Jump")
    }
}"#;
        let file = parse_ok(src);
        assert_eq!(file.usings.len(), 1);
        match &file.decl {
            Decl::Component { name, members, .. } => {
                assert_eq!(name, "PlayerController");
                // serialize x2, require, optional, update, func = 6 members
                assert_eq!(members.len(), 6);
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_parse_player_health_sample() {
        let src = r#"using UnityEngine

component PlayerHealth : MonoBehaviour {
    serialize maxHp: Int = 100
    serialize invincibleTime: Float = 1.0

    var hp: Int = 100
    var invincible: Bool = false

    func damage(amount: Int) {
        if invincible { return }
        hp -= amount
        start hitInvincible()
        if hp <= 0 {
            die()
        }
    }

    coroutine hitInvincible() {
        invincible = true
        wait invincibleTime.s
        invincible = false
    }

    func die() {
        gameObject.setActive(false)
    }
}"#;
        let file = parse_ok(src);
        match &file.decl {
            Decl::Component { name, members, .. } => {
                assert_eq!(name, "PlayerHealth");
                // 2 serialize + 2 var + 2 func + 1 coroutine = 7
                assert_eq!(members.len(), 7);
            }
            _ => panic!("Expected component"),
        }
    }

    #[test]
    fn test_parse_enum_with_when() {
        let src = r#"enum EnemyState {
    Idle,
    Chase,
    Attack
}"#;
        let file = parse_ok(src);
        match &file.decl {
            Decl::Enum { name, entries, .. } => {
                assert_eq!(name, "EnemyState");
                assert_eq!(entries.len(), 3);
            }
            _ => panic!("Expected enum"),
        }
    }

    // === Error recovery ===

    #[test]
    fn test_error_recovery() {
        let src = "component Foo : MonoBehaviour {\n  ??? bad\n  serialize speed: Float = 5.0\n}";
        let (file, errors) = parse(src);
        assert!(!errors.is_empty());
        // Should still have parsed some members
        match &file.decl {
            Decl::Component { members, .. } => {
                // The serialize field should still be parsed after error recovery
                assert!(members.iter().any(|m| matches!(m, Member::SerializeField { name, .. } if name == "speed")));
            }
            _ => {}
        }
    }

    // ── v4 Phase 4 — event, use, collection literals, DIM ─────────

    #[test]
    fn test_parse_event_member() {
        let file = parse_ok(r#"component Boss : MonoBehaviour {
  event onDamaged: (Int) => Unit
}"#);
        match file.decl {
            Decl::Component { members, .. } => {
                assert!(members.iter().any(|m| matches!(m, Member::Event { name, .. } if name == "onDamaged")));
            }
            _ => panic!("expected component"),
        }
    }

    #[test]
    fn test_parse_event_with_visibility() {
        let file = parse_ok(r#"component Boss : MonoBehaviour {
  private event onDeath: () => Unit
}"#);
        match file.decl {
            Decl::Component { members, .. } => {
                let event = members.iter().find_map(|m| match m {
                    Member::Event { name, visibility, .. } if name == "onDeath" => Some(*visibility),
                    _ => None,
                }).expect("event member");
                assert_eq!(event, Visibility::Private);
            }
            _ => panic!("expected component"),
        }
    }

    #[test]
    fn test_parse_use_declaration_form() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    use val s = openFile()
  }
}"#;
        let file = parse_ok(src);
        // Drill down to find the Use stmt
        let mut found = false;
        if let Decl::Component { members, .. } = file.decl {
            for m in &members {
                if let Member::Func { body: FuncBody::Block(b), .. } = m {
                    for s in &b.stmts {
                        if matches!(s, Stmt::Use { name, body: None, .. } if name == "s") {
                            found = true;
                        }
                    }
                }
            }
        }
        assert!(found, "Use declaration not parsed");
    }

    #[test]
    fn test_parse_use_block_form() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    use s = openFile() {
      log(s)
    }
  }
}"#;
        let file = parse_ok(src);
        let mut found = false;
        if let Decl::Component { members, .. } = file.decl {
            for m in &members {
                if let Member::Func { body: FuncBody::Block(b), .. } = m {
                    for s in &b.stmts {
                        if matches!(s, Stmt::Use { name, body: Some(_), .. } if name == "s") {
                            found = true;
                        }
                    }
                }
            }
        }
        assert!(found, "Use block form not parsed");
    }

    #[test]
    fn test_parse_list_literal() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val xs = [1, 2, 3]
  }
}"#;
        let file = parse_ok(src);
        let mut found = false;
        if let Decl::Component { members, .. } = file.decl {
            for m in &members {
                if let Member::Func { body: FuncBody::Block(b), .. } = m {
                    for s in &b.stmts {
                        if let Stmt::ValDecl { init: Expr::ListLit { elements, .. }, .. } = s {
                            assert_eq!(elements.len(), 3);
                            found = true;
                        }
                    }
                }
            }
        }
        assert!(found, "List literal not parsed");
    }

    #[test]
    fn test_parse_map_literal() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val m = {"a": 1, "b": 2}
  }
}"#;
        let file = parse_ok(src);
        let mut found = false;
        if let Decl::Component { members, .. } = file.decl {
            for m in &members {
                if let Member::Func { body: FuncBody::Block(b), .. } = m {
                    for s in &b.stmts {
                        if let Stmt::ValDecl { init: Expr::MapLit { entries, .. }, .. } = s {
                            assert_eq!(entries.len(), 2);
                            found = true;
                        }
                    }
                }
            }
        }
        assert!(found, "Map literal not parsed");
    }

    #[test]
    fn test_parse_empty_list_literal() {
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val xs: List<Int> = []
  }
}"#;
        let file = parse_ok(src);
        let mut found = false;
        if let Decl::Component { members, .. } = file.decl {
            for m in &members {
                if let Member::Func { body: FuncBody::Block(b), .. } = m {
                    for s in &b.stmts {
                        if let Stmt::ValDecl { init: Expr::ListLit { elements, .. }, .. } = s {
                            assert!(elements.is_empty());
                            found = true;
                        }
                    }
                }
            }
        }
        assert!(found, "Empty list literal not parsed");
    }

    #[test]
    fn test_parse_lambda_still_works_after_brace_disambiguation() {
        // Make sure adding map literal support didn't break lambda parsing.
        let src = r#"component Foo : MonoBehaviour {
  func test() {
    val handler = { x => x + 1 }
  }
}"#;
        let file = parse_ok(src);
        // Just ensure no parse errors occurred (parse_ok asserts that).
        let _ = file;
    }

    #[test]
    fn test_parse_default_interface_method() {
        let src = r#"interface IMovable {
  func move() {
    log("default")
  }
}"#;
        let file = parse_ok(src);
        if let Decl::Interface { members, .. } = file.decl {
            let func = members.iter().find_map(|m| match m {
                InterfaceMember::Func { name, default_body, .. } if name == "move" => {
                    Some(default_body.is_some())
                }
                _ => None,
            }).expect("interface func");
            assert!(func, "expected default body");
        } else {
            panic!("expected interface");
        }
    }

    #[test]
    fn test_parse_interface_signature_only() {
        let src = r#"interface IMovable {
  func move()
}"#;
        let file = parse_ok(src);
        if let Decl::Interface { members, .. } = file.decl {
            let no_default = members.iter().any(|m| matches!(m,
                InterfaceMember::Func { name, default_body, .. }
                if name == "move" && default_body.is_none()
            ));
            assert!(no_default, "expected no default body");
        } else {
            panic!("expected interface");
        }
    }

    // ── Phase 5: async / state machine / command / bind ──────────

    #[test]
    fn test_parse_async_func() {
        let src = r#"component Loader : MonoBehaviour {
  async func loadProfile(): String {
    val payload = await fetch("/api/profile")
    return payload
  }
}"#;
        let file = parse_ok(src);
        if let Decl::Component { members, .. } = file.decl {
            let mut found = false;
            for m in &members {
                if let Member::Func { name, is_async, .. } = m {
                    if name == "loadProfile" {
                        assert!(*is_async, "loadProfile must be async");
                        found = true;
                    }
                }
            }
            assert!(found, "loadProfile not found");
        } else {
            panic!("expected component");
        }
    }

    #[test]
    fn test_parse_await_inside_async() {
        let src = r#"component Loader : MonoBehaviour {
  async func ping() {
    await delay(1)
  }
}"#;
        let file = parse_ok(src);
        let _ = file;
    }

    #[test]
    fn test_parse_state_machine_basic() {
        let src = r#"component EnemyAI : MonoBehaviour {
  state machine aiState {
    state Idle {
      on playerDetected => Chase
    }
    state Chase {
      on playerLost => Idle
    }
  }
}"#;
        let file = parse_ok(src);
        if let Decl::Component { members, .. } = file.decl {
            let sm = members.iter().find_map(|m| match m {
                Member::StateMachine { name, states, .. } if name == "aiState" => Some(states),
                _ => None,
            }).expect("state machine");
            assert_eq!(sm.len(), 2, "two states");
            assert_eq!(sm[0].name, "Idle");
            assert_eq!(sm[0].transitions.len(), 1);
            assert_eq!(sm[0].transitions[0].event, "playerDetected");
            assert_eq!(sm[0].transitions[0].target, "Chase");
        } else {
            panic!("expected component");
        }
    }

    #[test]
    fn test_parse_state_machine_enter_exit() {
        let src = r#"component EnemyAI : MonoBehaviour {
  state machine aiState {
    state Idle {
      enter { log("idle") }
      exit { log("leaving idle") }
      on go => Run
    }
    state Run {
      on stopRun => Idle
    }
  }
}"#;
        let file = parse_ok(src);
        if let Decl::Component { members, .. } = file.decl {
            let sm = members.iter().find_map(|m| match m {
                Member::StateMachine { states, .. } => Some(states),
                _ => None,
            }).expect("state machine");
            let idle = sm.iter().find(|s| s.name == "Idle").expect("Idle state");
            assert!(idle.enter.is_some(), "Idle.enter present");
            assert!(idle.exit.is_some(), "Idle.exit present");
        } else {
            panic!("expected component");
        }
    }

    #[test]
    fn test_parse_command_basic() {
        let src = r#"component Unit : MonoBehaviour {
  command moveTo(target: Vector3) {
    log("execute")
  }
}"#;
        let file = parse_ok(src);
        if let Decl::Component { members, .. } = file.decl {
            let cmd = members.iter().find_map(|m| match m {
                Member::Command { name, params, .. } if name == "moveTo" => Some(params.len()),
                _ => None,
            }).expect("command");
            assert_eq!(cmd, 1, "moveTo has one param");
        } else {
            panic!("expected component");
        }
    }

    #[test]
    fn test_parse_command_with_undo_and_canexecute() {
        let src = r#"component Unit : MonoBehaviour {
  command damage(amount: Int) {
    log("dealing damage")
  } undo {
    log("undo")
  } canExecute = true
}"#;
        let file = parse_ok(src);
        if let Decl::Component { members, .. } = file.decl {
            let (has_undo, has_can) = members.iter().find_map(|m| match m {
                Member::Command { name, undo, can_execute, .. } if name == "damage" => {
                    Some((undo.is_some(), can_execute.is_some()))
                }
                _ => None,
            }).expect("command");
            assert!(has_undo, "undo block present");
            assert!(has_can, "canExecute present");
        } else {
            panic!("expected component");
        }
    }

    #[test]
    fn test_parse_bind_property() {
        let src = r#"component HUD : MonoBehaviour {
  bind hp: Int = 100
}"#;
        let file = parse_ok(src);
        if let Decl::Component { members, .. } = file.decl {
            let found = members.iter().any(|m| matches!(m,
                Member::BindProperty { name, .. } if name == "hp"
            ));
            assert!(found, "bind property hp present");
        } else {
            panic!("expected component");
        }
    }

    #[test]
    fn test_parse_bind_to_statement() {
        let src = r#"component HUD : MonoBehaviour {
  bind hp: Int = 100
  awake {
    bind hp to label.text
  }
}"#;
        let file = parse_ok(src);
        if let Decl::Component { members, .. } = file.decl {
            let mut saw_bind_to = false;
            for m in &members {
                if let Member::Lifecycle { body, .. } = m {
                    for s in &body.stmts {
                        if matches!(s, Stmt::BindTo { source, .. } if source == "hp") {
                            saw_bind_to = true;
                        }
                    }
                }
            }
            assert!(saw_bind_to, "bind hp to statement present");
        } else {
            panic!("expected component");
        }
    }

    // Issue #98: the parser must reject duplicate `enter { }` or
    // `exit { }` blocks within a single state. Previously the
    // second block silently overwrote the first, losing user code.
    #[test]
    fn test_duplicate_enter_block_rejected() {
        let src = r#"component AI : MonoBehaviour {
  state machine ai {
    state Idle {
      enter { log("first") }
      enter { log("second") }
    }
  }
}"#;
        let (_file, errors) = parse(src);
        assert!(
            errors.iter().any(|e| e.message.contains("E212") && e.message.contains("enter")),
            "expected E212 duplicate enter block, got {:?}",
            errors
        );
    }

    #[test]
    fn test_duplicate_exit_block_rejected() {
        let src = r#"component AI : MonoBehaviour {
  state machine ai {
    state Idle {
      exit { log("first") }
      exit { log("second") }
    }
  }
}"#;
        let (_file, errors) = parse(src);
        assert!(
            errors.iter().any(|e| e.message.contains("E212") && e.message.contains("exit")),
            "expected E212 duplicate exit block, got {:?}",
            errors
        );
    }

    #[test]
    fn test_single_enter_exit_block_still_parses() {
        let src = r#"component AI : MonoBehaviour {
  state machine ai {
    state Idle {
      enter { log("enter") }
      exit { log("exit") }
      on go => Active
    }
    state Active {
      on stop => Idle
    }
  }
}"#;
        let _ = parse_ok(src);
    }
}
