use crate::ast::*;
use crate::lexer::token::*;

/// Parse error with location and message.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

/// Recursive descent parser for Moon.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
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
        if let TokenKind::Identifier(name) = self.peek().clone() {
            let span = self.peek_span();
            self.advance();
            Ok((name, span))
        } else {
            Err(self.error(format!("Expected identifier, found {:?}", self.peek())))
        }
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
        let decl = match self.parse_decl() {
            Ok(d) => d,
            Err(e) => {
                self.errors.push(e);
                // Return a dummy component
                Decl::Component {
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

        match self.peek().clone() {
            TokenKind::Component => self.parse_component(),
            TokenKind::Asset => self.parse_asset(),
            TokenKind::Data => self.parse_data_class(),
            TokenKind::Class => self.parse_class(),
            TokenKind::Enum => self.parse_enum(),
            TokenKind::Attribute => self.parse_attribute_decl(annotations),
            _ => Err(self.error(format!("Expected declaration (component, asset, class, enum, attribute), found {:?}", self.peek()))),
        }
    }

    fn parse_component(&mut self) -> Result<Decl, ParseError> {
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

    fn parse_class(&mut self) -> Result<Decl, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'class'

        let (name, name_span) = self.expect_ident()?;

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

        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        let members = self.parse_members()?;
        self.expect(&TokenKind::RBrace)?;

        Ok(Decl::Class {
            name,
            name_span,
            super_class,
            super_class_span,
            interfaces,
            interface_spans,
            members,
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

        Ok(Decl::DataClass {
            name,
            name_span,
            fields,
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
            let args = if self.eat(&TokenKind::LParen) {
                let mut a = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    a.push(self.parse_expr()?);
                    if !self.eat(&TokenKind::Comma) { break; }
                }
                self.expect(&TokenKind::RParen)?;
                a
            } else {
                vec![]
            };
            entries.push(EnumEntry {
                name: ename,
                name_span: entry_name_span,
                args,
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
        // Collect annotations
        let annotations = self.parse_annotations()?;

        match self.peek().clone() {
            TokenKind::Serialize => self.parse_serialize_field(annotations),
            TokenKind::Require => self.parse_require(),
            TokenKind::Optional => self.parse_optional(),
            TokenKind::Child => self.parse_child(),
            TokenKind::Parent => self.parse_parent(),
            TokenKind::Func => self.parse_func(Visibility::Public, false),
            TokenKind::Coroutine => self.parse_coroutine(),
            TokenKind::Intrinsic => self.parse_intrinsic_member(),
            TokenKind::Override => {
                self.advance();
                self.expect(&TokenKind::Func)?;
                self.parse_func_inner(Visibility::Public, true)
            }
            TokenKind::Public | TokenKind::Private | TokenKind::Protected => {
                let vis = self.parse_visibility();
                match self.peek().clone() {
                    TokenKind::Serialize => self.parse_serialize_field_with_vis(annotations, Some(vis)),
                    TokenKind::Func => self.parse_func(vis, false),
                    TokenKind::Override => {
                        self.advance();
                        self.expect(&TokenKind::Func)?;
                        self.parse_func_inner(vis, true)
                    }
                    // field: private rb: Rigidbody
                    TokenKind::Identifier(_) => self.parse_field(vis),
                    TokenKind::Val | TokenKind::Var | TokenKind::Const | TokenKind::Fixed => self.parse_val_var_field(vis),
                    _ => Err(self.error(format!("Expected member after visibility, found {:?}", self.peek()))),
                }
            }
            TokenKind::Val | TokenKind::Var | TokenKind::Const | TokenKind::Fixed => self.parse_val_var_field(Visibility::Public),
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
        let mut annotations = Vec::new();
        while self.check(&TokenKind::At) {
            let start = self.peek_span();
            self.advance(); // consume '@'
            let (name, _) = self.expect_ident()?;
            let args = if self.eat(&TokenKind::LParen) {
                let mut a = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    a.push(self.parse_expr()?);
                    if !self.eat(&TokenKind::Comma) { break; }
                }
                self.expect(&TokenKind::RParen)?;
                a
            } else {
                vec![]
            };
            annotations.push(Annotation { name, args, span: Span { start: start.start, end: self.peek_span().end } });
            self.skip_newlines();
        }
        Ok(annotations)
    }

    fn parse_serialize_field(&mut self, annotations: Vec<Annotation>) -> Result<Member, ParseError> {
        self.parse_serialize_field_with_vis(annotations, None)
    }

    fn parse_serialize_field_with_vis(&mut self, annotations: Vec<Annotation>, visibility: Option<Visibility>) -> Result<Member, ParseError> {
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
        self.expect_newline_or_eof();
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

    fn parse_func(&mut self, vis: Visibility, is_override: bool) -> Result<Member, ParseError> {
        self.advance(); // consume 'func'
        self.parse_func_inner(vis, is_override)
    }

    fn parse_func_inner(&mut self, vis: Visibility, is_override: bool) -> Result<Member, ParseError> {
        let start = self.peek_span();
        let (name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;

        let return_ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = if self.eat(&TokenKind::Eq) {
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
            is_override,
            name,
            name_span,
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
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Member::Coroutine {
            name,
            name_span,
            params,
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
            mutability: Mutability::Var,
            name,
            name_span,
            ty: Some(ty),
            init,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_val_var_field(&mut self, vis: Visibility) -> Result<Member, ParseError> {
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
            _ => self.parse_expr_or_assignment_stmt(),
        }
    }

    fn parse_val_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'val'
        let (name, name_span) = self.expect_ident()?;
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&TokenKind::Eq)?;
        let init = self.parse_expr()?;
        self.expect_newline_or_eof();
        Ok(Stmt::ValDecl { name, name_span, ty, init, span: Span { start: start.start, end: self.peek_span().end } })
    }

    fn parse_var_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'var'
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
        Ok(Stmt::VarDecl { name, name_span, ty, init, span: Span { start: start.start, end: self.peek_span().end } })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'if'
        let cond = self.parse_expr()?;
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
            Some(self.parse_expr()?)
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

        let pattern = if self.eat(&TokenKind::Else) {
            WhenPattern::Else
        } else if self.eat(&TokenKind::Is) {
            let ty = self.parse_type()?;
            WhenPattern::Is(ty)
        } else {
            let expr = self.parse_expr()?;
            WhenPattern::Expression(expr)
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
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'for'
        let (var_name, name_span) = self.expect_ident()?;
        self.expect(&TokenKind::In)?;
        let iterable = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::For {
            var_name,
            name_span,
            iterable,
            body,
            span: Span { start: start.start, end: self.peek_span().end },
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // consume 'while'
        let cond = self.parse_expr()?;
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
        let value = if !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof | TokenKind::RBrace) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect_newline_or_eof();
        Ok(Stmt::Return { value, span: Span { start: start.start, end: self.peek_span().end } })
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
        let event = self.parse_expr()?;

        self.skip_newlines();
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();

        // Check for lambda params: { param -> body }
        let mut params = Vec::new();
        if let TokenKind::Identifier(name) = self.peek().clone() {
            // Look ahead for ->
            let mut ahead = self.pos + 1;
            while ahead < self.tokens.len() && self.tokens[ahead].kind == TokenKind::Newline {
                ahead += 1;
            }
            // Check for "ident ->" pattern (using FatArrow which is =>)
            // Actually listen uses { param -> ... } where -> is not a token yet.
            // Let's use => for consistency with when, or add a -> token.
            // For v1, let's use the pattern: { ident => body }
            // Actually the spec says { value -> body }. Let me check...
            // The spec example: listen slider.onValueChanged { value -> setVolume(value) }
            // We don't have a -> token. Let me use a different approach:
            // Look for "ident =>" pattern
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
            body,
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
        let cond = self.parse_expr()?;
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
            Some(Box::new(self.parse_expr()?))
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
                    let ty = self.parse_type()?;
                    let span = Span { start: start.start, end: self.peek_span().end };
                    left = Expr::Is { expr: Box::new(left), ty, span };
                    continue;
                }
                _ => break,
            };
            let start = left.span();
            self.advance();
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
                let end = self.parse_additive()?;
                let step = if self.eat(&TokenKind::Step) {
                    Some(Box::new(self.parse_additive()?))
                } else {
                    None
                };
                let span = Span { start: start.start, end: self.peek_span().end };
                Ok(Expr::Range { start: Box::new(left), end: Box::new(end), inclusive: true, step, span })
            }
            TokenKind::Until => {
                let start = left.span();
                self.advance();
                let end = self.parse_additive()?;
                let step = if self.eat(&TokenKind::Step) {
                    Some(Box::new(self.parse_additive()?))
                } else {
                    None
                };
                let span = Span { start: start.start, end: self.peek_span().end };
                Ok(Expr::Range { start: Box::new(left), end: Box::new(end), inclusive: false, step, span })
            }
            TokenKind::DownTo => {
                let start = left.span();
                self.advance();
                let end = self.parse_additive()?;
                let step = if self.eat(&TokenKind::Step) {
                    Some(Box::new(self.parse_additive()?))
                } else {
                    None
                };
                let span = Span { start: start.start, end: self.peek_span().end };
                // downTo is a range from left to end (descending)
                // We represent it as Range with inclusive=true and negative step semantics
                Ok(Expr::Range { start: Box::new(left), end: Box::new(end), inclusive: true, step, span })
            }
            _ => Ok(left),
        }
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
            let right = self.parse_unary()?;
            let span = Span { start: start.start, end: right.span().end };
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
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
            match self.peek().clone() {
                TokenKind::Dot => {
                    self.advance();
                    let (name, name_span) = self.expect_ident()?;

                    // Check if this is a method call: expr.name(args)
                    if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) {
                        let (type_args, args) = self.parse_call_args()?;
                        let span = Span { start: expr.span().start, end: self.peek_span().end };
                        expr = Expr::Call {
                            receiver: Some(Box::new(expr)),
                            name,
                            name_span,
                            type_args,
                            args,
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
                    let (name, name_span) = self.expect_ident()?;

                    if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) {
                        let (type_args, args) = self.parse_call_args()?;
                        let span = Span { start: expr.span().start, end: self.peek_span().end };
                        expr = Expr::SafeMethodCall {
                            receiver: Box::new(expr),
                            name,
                            name_span,
                            type_args,
                            args,
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
                TokenKind::BangBang => {
                    self.advance();
                    let span = Span { start: expr.span().start, end: self.peek_span().end };
                    expr = Expr::NonNullAssert { expr: Box::new(expr), span };
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
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
                        let (type_args, args) = self.parse_call_args()?;
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
                        if let Ok((type_args, args)) = self.try_parse_generic_call() {
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
            while !self.check(&TokenKind::Gt) && !self.check(&TokenKind::Eof) {
                ta.push(self.parse_type()?);
                if !self.eat(&TokenKind::Comma) { break; }
            }
            self.expect(&TokenKind::Gt)?;
            ta
        } else {
            vec![]
        };

        self.expect(&TokenKind::LParen)?;
        let mut args = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            // Check for named argument: name = expr
            let arg = if let TokenKind::Identifier(name) = self.peek().clone() {
                let save = self.pos;
                self.advance();
                if self.eat(&TokenKind::Eq) {
                    let value = self.parse_expr()?;
                    Arg { name: Some(name), value }
                } else {
                    self.pos = save;
                    let value = self.parse_expr()?;
                    Arg { name: None, value }
                }
            } else {
                let value = self.parse_expr()?;
                Arg { name: None, value }
            };
            args.push(arg);
            if !self.eat(&TokenKind::Comma) { break; }
        }
        self.expect(&TokenKind::RParen)?;
        Ok((type_args, args))
    }

    fn try_parse_generic_call(&mut self) -> Result<(Vec<TypeRef>, Vec<Arg>), ParseError> {
        // Try parsing <TypeArgs>(args)
        self.expect(&TokenKind::Lt)?;
        let mut type_args = Vec::new();
        while !self.check(&TokenKind::Gt) && !self.check(&TokenKind::Eof) {
            type_args.push(self.parse_type()?);
            if !self.eat(&TokenKind::Comma) { break; }
        }
        self.expect(&TokenKind::Gt)?;
        self.expect(&TokenKind::LParen)?;
        let mut args = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let value = self.parse_expr()?;
            args.push(Arg { name: None, value });
            if !self.eat(&TokenKind::Comma) { break; }
        }
        self.expect(&TokenKind::RParen)?;
        Ok((type_args, args))
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let span = self.peek_span();
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
                self.advance();
                Ok(Expr::Ident(name, span))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            _ => Err(self.error(format!("Expected expression, found {:?}", self.peek()))),
        }
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

    fn parse_type(&mut self) -> Result<TypeRef, ParseError> {
        let start = self.peek_span();
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
        let (name, name_span) = self.expect_ident()?;
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
        let mut last_line = 0u32;

        while !self.is_at_end_of_tokens() {
            match self.peek() {
                TokenKind::LBrace => {
                    depth += 1;
                    code.push('{');
                    self.advance();
                }
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        self.advance(); // consume final }
                        break;
                    }
                    code.push('}');
                    self.advance();
                }
                TokenKind::Newline => {
                    code.push('\n');
                    self.advance();
                }
                TokenKind::Eof => {
                    return Err(self.error("Unterminated intrinsic block".into()));
                }
                _ => {
                    // For intrinsic blocks, we reconstruct from tokens.
                    // This is an approximation — in a real implementation
                    // we'd work on the raw source text.
                    let tok = self.advance();
                    let tok_text = token_to_source_text(&tok.kind);
                    if tok.span.start.line != last_line && last_line != 0 {
                        // Ensure whitespace between tokens
                    }
                    if !code.is_empty() && !code.ends_with('\n') && !code.ends_with(' ') {
                        code.push(' ');
                    }
                    code.push_str(&tok_text);
                    last_line = tok.span.start.line;
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

fn token_to_source_text(kind: &TokenKind) -> String {
    match kind {
        TokenKind::IntLiteral(n) => n.to_string(),
        TokenKind::FloatLiteral(n) => format!("{n}"),
        TokenKind::DurationLiteral(n) => format!("{n}s"),
        TokenKind::BoolTrue => "true".into(),
        TokenKind::BoolFalse => "false".into(),
        TokenKind::StringLiteral(s) => format!("\"{s}\""),
        TokenKind::Identifier(s) => s.clone(),
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
        TokenKind::At => "@".into(),
        _ => format!("{:?}", kind),
    }
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
            | Expr::IntrinsicExpr { span, .. } => *span,
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
}
