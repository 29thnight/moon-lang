use crate::lexer::token::*;
use std::collections::VecDeque;

/// The PrSM lexer — hand-written scanner for maximum control over
/// string interpolation, duration literals, and multi-char operators.
pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
    /// Stack for tracking string interpolation nesting.
    /// Each entry is the brace depth inside a `${}` interpolation.
    interp_brace_depth: Vec<u32>,
    pending_tokens: VecDeque<Token>,
    resume_string_after_interp: bool,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            interp_brace_depth: Vec::new(),
            pending_tokens: VecDeque::new(),
            resume_string_after_interp: false,
        }
    }

    /// Tokenize the entire source into a Vec<Token>.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    /// Produce the next token.
    pub fn next_token(&mut self) -> Token {
        if let Some(token) = self.pending_tokens.pop_front() {
            return token;
        }

        if self.resume_string_after_interp {
            self.resume_string_after_interp = false;
            let start = self.make_pos();
            return self.scan_string_continuation(start, start);
        }

        self.skip_whitespace_and_comments();

        // If we're inside a string interpolation `${}` and hit `}`,
        // we need to resume string scanning.
        // This check is AFTER skip_whitespace so `${ expr }` works.
        if let Some(depth) = self.interp_brace_depth.last() {
            if *depth == 0 && self.peek() == Some('}') {
                self.interp_brace_depth.pop();
                let start = self.make_pos();
                self.advance(); // consume `}`
                let end = self.make_pos();
                return self.scan_string_continuation(start, end);
            }
        }

        if self.is_at_end() {
            return self.make_token(TokenKind::Eof);
        }

        let ch = self.peek().unwrap();

        // Newline
        if ch == '\n' {
            return self.scan_newline();
        }
        if ch == '\r' {
            let start = self.make_pos();
            self.advance();
            if self.peek() == Some('\n') {
                self.advance();
            }
            self.line += 1;
            self.col = 1;
            let end = self.make_pos();
            return Token::new(TokenKind::Newline, Span { start, end });
        }

        // String literal
        if ch == '"' {
            return self.scan_string();
        }

        // Numbers
        if ch.is_ascii_digit() {
            return self.scan_number();
        }

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            return self.scan_identifier();
        }

        // Operators and delimiters
        self.scan_operator()
    }

    // === Character utilities ===

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.source.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
            self.col += 1;
        }
        ch
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn make_pos(&self) -> Position {
        Position {
            line: self.line,
            col: self.col,
        }
    }

    fn make_token(&self, kind: TokenKind) -> Token {
        let pos = self.make_pos();
        Token::new(kind, Span { start: pos, end: pos })
    }

    fn make_span_token(&self, kind: TokenKind, start: Position) -> Token {
        Token::new(kind, Span { start, end: self.make_pos() })
    }

    // === Whitespace and comments ===

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') => {
                    self.advance();
                }
                Some('/') => {
                    if self.peek_next() == Some('/') {
                        self.skip_line_comment();
                    } else if self.peek_next() == Some('*') {
                        self.skip_block_comment();
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    fn skip_line_comment(&mut self) {
        // consume //
        self.advance();
        self.advance();
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break; // don't consume newline — it becomes a Newline token
            }
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) {
        // consume /*
        self.advance();
        self.advance();
        let mut depth = 1u32;
        while !self.is_at_end() && depth > 0 {
            if self.peek() == Some('/') && self.peek_next() == Some('*') {
                self.advance();
                self.advance();
                depth += 1;
            } else if self.peek() == Some('*') && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                depth -= 1;
            } else {
                if self.peek() == Some('\n') {
                    self.line += 1;
                    self.col = 0; // advance() will make it 1
                }
                self.advance();
            }
        }
    }

    // === Newline ===

    fn scan_newline(&mut self) -> Token {
        let start = self.make_pos();
        self.advance(); // consume \n
        self.line += 1;
        self.col = 1;
        let end = self.make_pos();
        Token::new(TokenKind::Newline, Span { start, end })
    }

    // === Numbers ===

    fn scan_number(&mut self) -> Token {
        let start = self.make_pos();
        let mut num_str = String::new();
        let mut is_float = false;

        // Integer part
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '_' {
                if ch != '_' {
                    num_str.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }

        // Decimal part
        if self.peek() == Some('.') && self.peek_next().map_or(false, |c| c.is_ascii_digit()) {
            is_float = true;
            num_str.push('.');
            self.advance(); // consume '.'
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() || ch == '_' {
                    if ch != '_' {
                        num_str.push(ch);
                    }
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Duration suffix: 's' for seconds
        if self.peek() == Some('s')
            && !self.peek_next().map_or(false, |c| c.is_alphanumeric() || c == '_')
        {
            self.advance(); // consume 's'
            let value: f64 = num_str.parse().unwrap_or(0.0);
            return self.make_span_token(TokenKind::DurationLiteral(value), start);
        }

        // Float suffix: 'f' (optional, for clarity)
        if self.peek() == Some('f')
            && !self.peek_next().map_or(false, |c| c.is_alphanumeric() || c == '_')
        {
            self.advance(); // consume 'f'
            is_float = true;
        }

        // Long suffix: 'L'
        if !is_float
            && self.peek() == Some('L')
            && !self.peek_next().map_or(false, |c| c.is_alphanumeric() || c == '_')
        {
            self.advance(); // consume 'L'
            let value: i64 = num_str.parse().unwrap_or(0);
            return self.make_span_token(TokenKind::IntLiteral(value), start);
        }

        if is_float {
            let value: f64 = num_str.parse().unwrap_or(0.0);
            self.make_span_token(TokenKind::FloatLiteral(value), start)
        } else {
            let value: i64 = num_str.parse().unwrap_or(0);
            self.make_span_token(TokenKind::IntLiteral(value), start)
        }
    }

    // === Identifiers and keywords ===

    fn scan_identifier(&mut self) -> Token {
        let start = self.make_pos();
        let mut ident = String::new();

        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let kind = lookup_keyword(&ident).unwrap_or(TokenKind::Identifier(ident));
        self.make_span_token(kind, start)
    }

    // === Strings ===

    fn scan_string(&mut self) -> Token {
        let start = self.make_pos();
        self.advance(); // consume opening '"'

        let mut text = String::new();

        loop {
            match self.peek() {
                None | Some('\n') => {
                    // Unterminated string
                    return self.make_span_token(
                        TokenKind::Error("Unterminated string literal".into()),
                        start,
                    );
                }
                Some('"') => {
                    self.advance(); // consume closing '"'
                    return self.make_span_token(TokenKind::StringLiteral(text), start);
                }
                Some('\\') => {
                    self.advance(); // consume '\'
                    match self.peek() {
                        Some('n') => { text.push('\n'); self.advance(); }
                        Some('t') => { text.push('\t'); self.advance(); }
                        Some('r') => { text.push('\r'); self.advance(); }
                        Some('\\') => { text.push('\\'); self.advance(); }
                        Some('"') => { text.push('"'); self.advance(); }
                        Some('$') => { text.push('$'); self.advance(); }
                        Some(ch) => { text.push('\\'); text.push(ch); self.advance(); }
                        None => { text.push('\\'); }
                    }
                }
                Some('$') => {
                    if self.peek_next() == Some('{') {
                        // ${expr} interpolation
                        let kind = TokenKind::StringStart(text);
                        self.advance(); // consume '$'
                        self.advance(); // consume '{'
                        self.interp_brace_depth.push(0);
                        return self.make_span_token(kind, start);
                    } else if self.peek_next().map_or(false, |c| c.is_alphabetic() || c == '_') {
                        return self.emit_string_segment_with_ident(start, text, true);
                    } else {
                        text.push('$');
                        self.advance();
                    }
                }
                Some(ch) => {
                    text.push(ch);
                    self.advance();
                }
            }
        }
    }

    /// Resume scanning a string after an interpolation expression `${}` ends.
    fn scan_string_continuation(&mut self, _interp_start: Position, _interp_end: Position) -> Token {
        // First emit the InterpolationExprEnd
        // When `}` closes an interpolation, we immediately resume string scanning.
        // The token stream is: StringStart ... expr ... StringMiddle/StringEnd
        // No separate InterpolationExprEnd token — the `}` is implicit.
        let start = self.make_pos();
        let mut text = String::new();

        loop {
            match self.peek() {
                None | Some('\n') => {
                    return self.make_span_token(
                        TokenKind::Error("Unterminated string literal".into()),
                        start,
                    );
                }
                Some('"') => {
                    self.advance();
                    return self.make_span_token(TokenKind::StringEnd(text), start);
                }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('n') => { text.push('\n'); self.advance(); }
                        Some('t') => { text.push('\t'); self.advance(); }
                        Some('r') => { text.push('\r'); self.advance(); }
                        Some('\\') => { text.push('\\'); self.advance(); }
                        Some('"') => { text.push('"'); self.advance(); }
                        Some('$') => { text.push('$'); self.advance(); }
                        Some(ch) => { text.push('\\'); text.push(ch); self.advance(); }
                        None => { text.push('\\'); }
                    }
                }
                Some('$') if self.peek_next() == Some('{') => {
                    // Another interpolation
                    self.advance(); // $
                    self.advance(); // {
                    self.interp_brace_depth.push(0);
                    return self.make_span_token(TokenKind::StringMiddle(text), start);
                }
                Some('$') if self.peek_next().map_or(false, |c| c.is_alphabetic() || c == '_') => {
                    return self.emit_string_segment_with_ident(start, text, false);
                }
                Some(ch) => {
                    text.push(ch);
                    self.advance();
                }
            }
        }
    }

    fn emit_string_segment_with_ident(&mut self, start: Position, text: String, is_start: bool) -> Token {
        let segment_kind = if is_start {
            TokenKind::StringStart(text)
        } else {
            TokenKind::StringMiddle(text)
        };

        self.advance(); // consume '$'
        let ident_start = self.make_pos();
        let mut ident = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        self.pending_tokens.push_back(Token::new(
            TokenKind::Identifier(ident),
            Span {
                start: ident_start,
                end: self.make_pos(),
            },
        ));
        self.resume_string_after_interp = true;

        self.make_span_token(segment_kind, start)
    }

    // === Operators ===

    fn scan_operator(&mut self) -> Token {
        let start = self.make_pos();
        let ch = self.advance().unwrap();

        match ch {
            '+' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.make_span_token(TokenKind::PlusEq, start)
                } else {
                    self.make_span_token(TokenKind::Plus, start)
                }
            }
            '-' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.make_span_token(TokenKind::MinusEq, start)
                } else {
                    self.make_span_token(TokenKind::Minus, start)
                }
            }
            '*' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.make_span_token(TokenKind::StarEq, start)
                } else {
                    self.make_span_token(TokenKind::Star, start)
                }
            }
            '/' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.make_span_token(TokenKind::SlashEq, start)
                } else {
                    self.make_span_token(TokenKind::Slash, start)
                }
            }
            '%' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.make_span_token(TokenKind::PercentEq, start)
                } else {
                    self.make_span_token(TokenKind::Percent, start)
                }
            }
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.make_span_token(TokenKind::EqEq, start)
                } else if self.peek() == Some('>') {
                    self.advance();
                    self.make_span_token(TokenKind::FatArrow, start)
                } else {
                    self.make_span_token(TokenKind::Eq, start)
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.make_span_token(TokenKind::NotEq, start)
                } else if self.peek() == Some('!') {
                    self.advance();
                    self.make_span_token(TokenKind::BangBang, start)
                } else {
                    self.make_span_token(TokenKind::Bang, start)
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.make_span_token(TokenKind::LtEq, start)
                } else {
                    self.make_span_token(TokenKind::Lt, start)
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.make_span_token(TokenKind::GtEq, start)
                } else {
                    self.make_span_token(TokenKind::Gt, start)
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    self.make_span_token(TokenKind::AmpAmp, start)
                } else {
                    self.make_span_token(
                        TokenKind::Error("Expected '&&', got single '&'".into()),
                        start,
                    )
                }
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    self.make_span_token(TokenKind::PipePipe, start)
                } else {
                    self.make_span_token(
                        TokenKind::Error("Expected '||', got single '|'".into()),
                        start,
                    )
                }
            }
            '?' => {
                if self.peek() == Some('.') {
                    self.advance();
                    self.make_span_token(TokenKind::QuestionDot, start)
                } else if self.peek() == Some(':') {
                    self.advance();
                    self.make_span_token(TokenKind::Elvis, start)
                } else {
                    self.make_span_token(TokenKind::Question, start)
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.advance();
                    self.make_span_token(TokenKind::DotDot, start)
                } else {
                    self.make_span_token(TokenKind::Dot, start)
                }
            }
            ':' => self.make_span_token(TokenKind::Colon, start),
            '(' => self.make_span_token(TokenKind::LParen, start),
            ')' => self.make_span_token(TokenKind::RParen, start),
            '{' => {
                // Track brace depth for interpolation
                if let Some(depth) = self.interp_brace_depth.last_mut() {
                    *depth += 1;
                }
                self.make_span_token(TokenKind::LBrace, start)
            }
            '}' => {
                // Track brace depth for interpolation
                if let Some(depth) = self.interp_brace_depth.last_mut() {
                    if *depth > 0 {
                        *depth -= 1;
                    }
                    // If depth reaches 0 and we're here, the next next_token() call
                    // will handle the interpolation end via the check at the top.
                    // But wait, we already consumed the }. If depth was 1 and becomes 0,
                    // the NEXT } (depth 0) is the interpolation closer.
                    // Actually, this } at depth > 0 is a normal brace inside the interpolation.
                }
                self.make_span_token(TokenKind::RBrace, start)
            }
            '[' => self.make_span_token(TokenKind::LBracket, start),
            ']' => self.make_span_token(TokenKind::RBracket, start),
            ',' => self.make_span_token(TokenKind::Comma, start),
            ';' => self.make_span_token(TokenKind::Semicolon, start),
            '@' => self.make_span_token(TokenKind::At, start),
            _ => self.make_span_token(
                TokenKind::Error(format!("Unexpected character: '{}'", ch)),
                start,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(input);
        lexer
            .tokenize()
            .into_iter()
            .map(|t| t.kind)
            .filter(|k| !matches!(k, TokenKind::Newline | TokenKind::Eof))
            .collect()
    }

    fn lex_all(input: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(input);
        lexer.tokenize().into_iter().map(|t| t.kind).collect()
    }

    // === Keywords ===

    #[test]
    fn test_keywords() {
        assert_eq!(lex("component"), vec![TokenKind::Component]);
        assert_eq!(lex("asset"), vec![TokenKind::Asset]);
        assert_eq!(lex("class"), vec![TokenKind::Class]);
        assert_eq!(lex("enum"), vec![TokenKind::Enum]);
        assert_eq!(lex("serialize"), vec![TokenKind::Serialize]);
        assert_eq!(lex("require"), vec![TokenKind::Require]);
        assert_eq!(lex("optional"), vec![TokenKind::Optional]);
        assert_eq!(lex("val"), vec![TokenKind::Val]);
        assert_eq!(lex("var"), vec![TokenKind::Var]);
        assert_eq!(lex("func"), vec![TokenKind::Func]);
        assert_eq!(lex("return"), vec![TokenKind::Return]);
        assert_eq!(lex("if"), vec![TokenKind::If]);
        assert_eq!(lex("else"), vec![TokenKind::Else]);
        assert_eq!(lex("when"), vec![TokenKind::When]);
        assert_eq!(lex("for"), vec![TokenKind::For]);
        assert_eq!(lex("while"), vec![TokenKind::While]);
        assert_eq!(lex("in"), vec![TokenKind::In]);
        assert_eq!(lex("true"), vec![TokenKind::BoolTrue]);
        assert_eq!(lex("false"), vec![TokenKind::BoolFalse]);
        assert_eq!(lex("null"), vec![TokenKind::Null]);
        assert_eq!(lex("this"), vec![TokenKind::This]);
    }

    #[test]
    fn test_lifecycle_keywords() {
        assert_eq!(lex("awake"), vec![TokenKind::Awake]);
        assert_eq!(lex("update"), vec![TokenKind::Update]);
        assert_eq!(lex("fixedUpdate"), vec![TokenKind::FixedUpdate]);
        assert_eq!(lex("lateUpdate"), vec![TokenKind::LateUpdate]);
        assert_eq!(lex("onEnable"), vec![TokenKind::OnEnable]);
        assert_eq!(lex("onDisable"), vec![TokenKind::OnDisable]);
        assert_eq!(lex("onDestroy"), vec![TokenKind::OnDestroy]);
        assert_eq!(lex("onTriggerEnter"), vec![TokenKind::OnTriggerEnter]);
        assert_eq!(lex("onCollisionEnter"), vec![TokenKind::OnCollisionEnter]);
    }

    #[test]
    fn test_coroutine_keywords() {
        assert_eq!(lex("coroutine"), vec![TokenKind::Coroutine]);
        assert_eq!(lex("wait"), vec![TokenKind::Wait]);
        assert_eq!(lex("start"), vec![TokenKind::Start]);
        assert_eq!(lex("stop"), vec![TokenKind::Stop]);
        assert_eq!(lex("stopAll"), vec![TokenKind::StopAll]);
        assert_eq!(lex("nextFrame"), vec![TokenKind::NextFrame]);
        assert_eq!(lex("fixedFrame"), vec![TokenKind::FixedFrame]);
    }

    #[test]
    fn test_other_keywords() {
        assert_eq!(lex("listen"), vec![TokenKind::Listen]);
        assert_eq!(lex("intrinsic"), vec![TokenKind::Intrinsic]);
        assert_eq!(lex("using"), vec![TokenKind::Using]);
        assert_eq!(lex("data"), vec![TokenKind::Data]);
        assert_eq!(lex("override"), vec![TokenKind::Override]);
        assert_eq!(lex("is"), vec![TokenKind::Is]);
        assert_eq!(lex("until"), vec![TokenKind::Until]);
        assert_eq!(lex("downTo"), vec![TokenKind::DownTo]);
        assert_eq!(lex("step"), vec![TokenKind::Step]);
        assert_eq!(lex("child"), vec![TokenKind::Child]);
        assert_eq!(lex("parent"), vec![TokenKind::Parent]);
    }

    // === Identifiers ===

    #[test]
    fn test_identifiers() {
        assert_eq!(lex("foo"), vec![TokenKind::Identifier("foo".into())]);
        assert_eq!(lex("_bar"), vec![TokenKind::Identifier("_bar".into())]);
        assert_eq!(lex("myVar123"), vec![TokenKind::Identifier("myVar123".into())]);
        assert_eq!(lex("PlayerController"), vec![TokenKind::Identifier("PlayerController".into())]);
    }

    // === Integer literals ===

    #[test]
    fn test_int_literals() {
        assert_eq!(lex("0"), vec![TokenKind::IntLiteral(0)]);
        assert_eq!(lex("42"), vec![TokenKind::IntLiteral(42)]);
        assert_eq!(lex("1_000_000"), vec![TokenKind::IntLiteral(1000000)]);
        assert_eq!(lex("100L"), vec![TokenKind::IntLiteral(100)]);
    }

    // === Float literals ===

    #[test]
    fn test_float_literals() {
        assert_eq!(lex("3.14"), vec![TokenKind::FloatLiteral(3.14)]);
        assert_eq!(lex("0.5"), vec![TokenKind::FloatLiteral(0.5)]);
        assert_eq!(lex("1.0f"), vec![TokenKind::FloatLiteral(1.0)]);
        assert_eq!(lex("5.0"), vec![TokenKind::FloatLiteral(5.0)]);
    }

    // === Duration literals ===

    #[test]
    fn test_duration_literals() {
        assert_eq!(lex("1.0s"), vec![TokenKind::DurationLiteral(1.0)]);
        assert_eq!(lex("0.2s"), vec![TokenKind::DurationLiteral(0.2)]);
        assert_eq!(lex("30s"), vec![TokenKind::DurationLiteral(30.0)]);
    }

    // === String literals ===

    #[test]
    fn test_simple_strings() {
        assert_eq!(lex(r#""hello""#), vec![TokenKind::StringLiteral("hello".into())]);
        assert_eq!(lex(r#""""#), vec![TokenKind::StringLiteral("".into())]);
        assert_eq!(lex(r#""hello world""#), vec![TokenKind::StringLiteral("hello world".into())]);
    }

    #[test]
    fn test_string_escapes() {
        assert_eq!(lex(r#""line1\nline2""#), vec![TokenKind::StringLiteral("line1\nline2".into())]);
        assert_eq!(lex(r#""tab\there""#), vec![TokenKind::StringLiteral("tab\there".into())]);
        assert_eq!(lex(r#""escaped\"quote""#), vec![TokenKind::StringLiteral("escaped\"quote".into())]);
        assert_eq!(lex(r#""dollar\$sign""#), vec![TokenKind::StringLiteral("dollar$sign".into())]);
    }

    #[test]
    fn test_string_interpolation_expr() {
        // "hello ${name}!"
        let tokens = lex(r#""hello ${name}!""#);
        assert_eq!(tokens, vec![
            TokenKind::StringStart("hello ".into()),
            TokenKind::Identifier("name".into()),
            TokenKind::StringEnd("!".into()),
        ]);
    }

    #[test]
    fn test_string_interpolation_complex() {
        // "score: ${player.score}"
        let tokens = lex(r#""score: ${player.score}""#);
        assert_eq!(tokens, vec![
            TokenKind::StringStart("score: ".into()),
            TokenKind::Identifier("player".into()),
            TokenKind::Dot,
            TokenKind::Identifier("score".into()),
            TokenKind::StringEnd("".into()),
        ]);
    }

    #[test]
    fn test_string_interpolation_ident() {
        let tokens = lex(r#""hello $name!""#);
        assert_eq!(tokens, vec![
            TokenKind::StringStart("hello ".into()),
            TokenKind::Identifier("name".into()),
            TokenKind::StringEnd("!".into()),
        ]);
    }

    #[test]
    fn test_string_interpolation_mixed() {
        let tokens = lex(r#""$first scored ${points} pts""#);
        assert_eq!(tokens, vec![
            TokenKind::StringStart("".into()),
            TokenKind::Identifier("first".into()),
            TokenKind::StringMiddle(" scored ".into()),
            TokenKind::Identifier("points".into()),
            TokenKind::StringEnd(" pts".into()),
        ]);
    }

    // === Operators ===

    #[test]
    fn test_arithmetic_operators() {
        assert_eq!(lex("+ - * / %"), vec![
            TokenKind::Plus, TokenKind::Minus, TokenKind::Star,
            TokenKind::Slash, TokenKind::Percent,
        ]);
    }

    #[test]
    fn test_comparison_operators() {
        assert_eq!(lex("== != < > <= >="), vec![
            TokenKind::EqEq, TokenKind::NotEq, TokenKind::Lt,
            TokenKind::Gt, TokenKind::LtEq, TokenKind::GtEq,
        ]);
    }

    #[test]
    fn test_logical_operators() {
        assert_eq!(lex("&& ||"), vec![TokenKind::AmpAmp, TokenKind::PipePipe]);
        assert_eq!(lex("!"), vec![TokenKind::Bang]);
        assert_eq!(lex("!!"), vec![TokenKind::BangBang]);
    }

    #[test]
    fn test_assignment_operators() {
        assert_eq!(lex("= += -= *= /= %="), vec![
            TokenKind::Eq, TokenKind::PlusEq, TokenKind::MinusEq,
            TokenKind::StarEq, TokenKind::SlashEq, TokenKind::PercentEq,
        ]);
    }

    #[test]
    fn test_null_operators() {
        assert_eq!(lex("?."), vec![TokenKind::QuestionDot]);
        assert_eq!(lex("?:"), vec![TokenKind::Elvis]);
        assert_eq!(lex("?"), vec![TokenKind::Question]);
    }

    #[test]
    fn test_other_operators() {
        assert_eq!(lex("=>"), vec![TokenKind::FatArrow]);
        assert_eq!(lex(".."), vec![TokenKind::DotDot]);
        assert_eq!(lex("."), vec![TokenKind::Dot]);
        assert_eq!(lex(":"), vec![TokenKind::Colon]);
    }

    // === Delimiters ===

    #[test]
    fn test_delimiters() {
        assert_eq!(lex("( ) { } [ ] , @"), vec![
            TokenKind::LParen, TokenKind::RParen,
            TokenKind::LBrace, TokenKind::RBrace,
            TokenKind::LBracket, TokenKind::RBracket,
            TokenKind::Comma, TokenKind::At,
        ]);
    }

    // === Comments ===

    #[test]
    fn test_line_comment() {
        assert_eq!(lex("val x // comment"), vec![
            TokenKind::Val, TokenKind::Identifier("x".into()),
        ]);
    }

    #[test]
    fn test_block_comment() {
        assert_eq!(lex("val /* comment */ x"), vec![
            TokenKind::Val, TokenKind::Identifier("x".into()),
        ]);
    }

    #[test]
    fn test_nested_block_comment() {
        assert_eq!(lex("val /* outer /* inner */ still comment */ x"), vec![
            TokenKind::Val, TokenKind::Identifier("x".into()),
        ]);
    }

    // === Newlines ===

    #[test]
    fn test_newlines() {
        let tokens = lex_all("val x\nvar y");
        assert_eq!(tokens, vec![
            TokenKind::Val,
            TokenKind::Identifier("x".into()),
            TokenKind::Newline,
            TokenKind::Var,
            TokenKind::Identifier("y".into()),
            TokenKind::Eof,
        ]);
    }

    // === Position tracking ===

    #[test]
    fn test_position_tracking() {
        let mut lexer = Lexer::new("val x\nvar y");
        let tokens = lexer.tokenize();
        // "val" starts at line 1, col 1
        assert_eq!(tokens[0].span.start, Position { line: 1, col: 1 });
        // "x" starts at line 1, col 5
        assert_eq!(tokens[1].span.start, Position { line: 1, col: 5 });
        // newline at line 1, col 6
        assert_eq!(tokens[2].span.start, Position { line: 1, col: 6 });
        // "var" starts at line 2, col 1
        assert_eq!(tokens[3].span.start, Position { line: 2, col: 1 });
    }

    // === Compound expressions ===

    #[test]
    fn test_field_declaration() {
        assert_eq!(lex("serialize speed: Float = 5.0"), vec![
            TokenKind::Serialize,
            TokenKind::Identifier("speed".into()),
            TokenKind::Colon,
            TokenKind::Identifier("Float".into()),
            TokenKind::Eq,
            TokenKind::FloatLiteral(5.0),
        ]);
    }

    #[test]
    fn test_require_declaration() {
        assert_eq!(lex("require rb: Rigidbody"), vec![
            TokenKind::Require,
            TokenKind::Identifier("rb".into()),
            TokenKind::Colon,
            TokenKind::Identifier("Rigidbody".into()),
        ]);
    }

    #[test]
    fn test_function_call() {
        assert_eq!(lex("get<Rigidbody>()"), vec![
            TokenKind::Identifier("get".into()),
            TokenKind::Lt,
            TokenKind::Identifier("Rigidbody".into()),
            TokenKind::Gt,
            TokenKind::LParen,
            TokenKind::RParen,
        ]);
    }

    #[test]
    fn test_safe_call_chain() {
        assert_eq!(lex("animator?.play"), vec![
            TokenKind::Identifier("animator".into()),
            TokenKind::QuestionDot,
            TokenKind::Identifier("play".into()),
        ]);
    }

    #[test]
    fn test_elvis_operator() {
        assert_eq!(lex("name ?: \"Unknown\""), vec![
            TokenKind::Identifier("name".into()),
            TokenKind::Elvis,
            TokenKind::StringLiteral("Unknown".into()),
        ]);
    }

    #[test]
    fn test_when_branch() {
        assert_eq!(lex("State.Idle => idle()"), vec![
            TokenKind::Identifier("State".into()),
            TokenKind::Dot,
            TokenKind::Identifier("Idle".into()),
            TokenKind::FatArrow,
            TokenKind::Identifier("idle".into()),
            TokenKind::LParen,
            TokenKind::RParen,
        ]);
    }

    #[test]
    fn test_annotation() {
        assert_eq!(lex("@header(\"Movement\")"), vec![
            TokenKind::At,
            TokenKind::Identifier("header".into()),
            TokenKind::LParen,
            TokenKind::StringLiteral("Movement".into()),
            TokenKind::RParen,
        ]);
    }

    #[test]
    fn test_wait_duration() {
        assert_eq!(lex("wait 1.0s"), vec![
            TokenKind::Wait,
            TokenKind::DurationLiteral(1.0),
        ]);
    }

    #[test]
    fn test_wait_next_frame() {
        assert_eq!(lex("wait nextFrame"), vec![
            TokenKind::Wait,
            TokenKind::NextFrame,
        ]);
    }

    #[test]
    fn test_component_declaration() {
        assert_eq!(lex("component Player : MonoBehaviour"), vec![
            TokenKind::Component,
            TokenKind::Identifier("Player".into()),
            TokenKind::Colon,
            TokenKind::Identifier("MonoBehaviour".into()),
        ]);
    }

    #[test]
    fn test_data_class_keywords() {
        assert_eq!(lex("data class Foo"), vec![
            TokenKind::Data,
            TokenKind::Class,
            TokenKind::Identifier("Foo".into()),
        ]);
    }

    #[test]
    fn test_range_expression() {
        assert_eq!(lex("0 until 10"), vec![
            TokenKind::IntLiteral(0),
            TokenKind::Until,
            TokenKind::IntLiteral(10),
        ]);
        assert_eq!(lex("0..10"), vec![
            TokenKind::IntLiteral(0),
            TokenKind::DotDot,
            TokenKind::IntLiteral(10),
        ]);
    }

    // === Error cases ===

    #[test]
    fn test_unterminated_string() {
        let tokens = lex("\"hello");
        assert!(matches!(tokens[0], TokenKind::Error(_)));
    }

    #[test]
    fn test_unexpected_character() {
        let tokens = lex("~");
        assert!(matches!(tokens[0], TokenKind::Error(_)));
    }

    // === Whitespace handling ===

    #[test]
    fn test_whitespace_ignored() {
        assert_eq!(lex("val   x"), vec![
            TokenKind::Val,
            TokenKind::Identifier("x".into()),
        ]);
        assert_eq!(lex("val\tx"), vec![
            TokenKind::Val,
            TokenKind::Identifier("x".into()),
        ]);
    }

    // === Full sample snippets ===

    #[test]
    fn test_sample_field_with_annotation() {
        assert_eq!(lex("@range(0, 20)\nserialize speed: Float = 5.0"), vec![
            TokenKind::At,
            TokenKind::Identifier("range".into()),
            TokenKind::LParen,
            TokenKind::IntLiteral(0),
            TokenKind::Comma,
            TokenKind::IntLiteral(20),
            TokenKind::RParen,
            // newline filtered in lex()
            TokenKind::Serialize,
            TokenKind::Identifier("speed".into()),
            TokenKind::Colon,
            TokenKind::Identifier("Float".into()),
            TokenKind::Eq,
            TokenKind::FloatLiteral(5.0),
        ]);
    }

    #[test]
    fn test_nullable_type() {
        assert_eq!(lex("var target: Transform?"), vec![
            TokenKind::Var,
            TokenKind::Identifier("target".into()),
            TokenKind::Colon,
            TokenKind::Identifier("Transform".into()),
            TokenKind::Question,
        ]);
    }

    #[test]
    fn test_start_stop_coroutine() {
        assert_eq!(lex("start blink()"), vec![
            TokenKind::Start,
            TokenKind::Identifier("blink".into()),
            TokenKind::LParen,
            TokenKind::RParen,
        ]);
        assert_eq!(lex("stopAll()"), vec![
            TokenKind::StopAll,
            TokenKind::LParen,
            TokenKind::RParen,
        ]);
    }

    #[test]
    fn test_non_null_assert() {
        assert_eq!(lex("target!!"), vec![
            TokenKind::Identifier("target".into()),
            TokenKind::BangBang,
        ]);
    }

    // === Additional edge case tests ===

    #[test]
    fn test_number_underscore_separator() {
        assert_eq!(lex("1_000_000"), vec![TokenKind::IntLiteral(1_000_000)]);
        assert_eq!(lex("3.14_15"), vec![TokenKind::FloatLiteral(3.1415)]);
    }

    #[test]
    fn test_dot_vs_dotdot_vs_float() {
        // "0.5" is a float, "0..5" is int-range, "obj.x" is member access
        assert_eq!(lex("0.5"), vec![TokenKind::FloatLiteral(0.5)]);
        assert_eq!(lex("0..5"), vec![TokenKind::IntLiteral(0), TokenKind::DotDot, TokenKind::IntLiteral(5)]);
        assert_eq!(lex("obj.x"), vec![
            TokenKind::Identifier("obj".into()),
            TokenKind::Dot,
            TokenKind::Identifier("x".into()),
        ]);
    }

    #[test]
    fn test_duration_int_suffix() {
        // "30s" — integer with duration suffix
        assert_eq!(lex("30s"), vec![TokenKind::DurationLiteral(30.0)]);
    }

    #[test]
    fn test_using_statement() {
        assert_eq!(lex("using UnityEngine"), vec![
            TokenKind::Using,
            TokenKind::Identifier("UnityEngine".into()),
        ]);
    }

    #[test]
    fn test_qualified_name() {
        assert_eq!(lex("UnityEngine.UI"), vec![
            TokenKind::Identifier("UnityEngine".into()),
            TokenKind::Dot,
            TokenKind::Identifier("UI".into()),
        ]);
    }

    #[test]
    fn test_expression_body_func() {
        assert_eq!(lex("func isDead(): Bool = hp <= 0"), vec![
            TokenKind::Func,
            TokenKind::Identifier("isDead".into()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::Colon,
            TokenKind::Identifier("Bool".into()),
            TokenKind::Eq,
            TokenKind::Identifier("hp".into()),
            TokenKind::LtEq,
            TokenKind::IntLiteral(0),
        ]);
    }

    #[test]
    fn test_listen_sugar() {
        assert_eq!(lex("listen button.onClick"), vec![
            TokenKind::Listen,
            TokenKind::Identifier("button".into()),
            TokenKind::Dot,
            TokenKind::Identifier("onClick".into()),
        ]);
    }

    #[test]
    fn test_intrinsic_keyword() {
        assert_eq!(lex("intrinsic func"), vec![
            TokenKind::Intrinsic,
            TokenKind::Func,
        ]);
    }

    #[test]
    fn test_multiple_newlines_preserved() {
        let tokens = lex_all("a\n\nb");
        assert_eq!(tokens, vec![
            TokenKind::Identifier("a".into()),
            TokenKind::Newline,
            TokenKind::Newline,
            TokenKind::Identifier("b".into()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_empty_input() {
        let tokens = lex_all("");
        assert_eq!(tokens, vec![TokenKind::Eof]);
    }

    #[test]
    fn test_only_whitespace() {
        let tokens = lex("   \t  ");
        assert!(tokens.is_empty()); // lex() filters newlines and eof
    }

    #[test]
    fn test_complex_condition() {
        assert_eq!(lex("hp <= 0 && !invincible"), vec![
            TokenKind::Identifier("hp".into()),
            TokenKind::LtEq,
            TokenKind::IntLiteral(0),
            TokenKind::AmpAmp,
            TokenKind::Bang,
            TokenKind::Identifier("invincible".into()),
        ]);
    }

    #[test]
    fn test_index_access() {
        assert_eq!(lex("items[0]"), vec![
            TokenKind::Identifier("items".into()),
            TokenKind::LBracket,
            TokenKind::IntLiteral(0),
            TokenKind::RBracket,
        ]);
    }

    #[test]
    fn test_negative_number_is_minus_plus_int() {
        // Lexer doesn't handle unary minus — that's parser's job
        assert_eq!(lex("-5"), vec![TokenKind::Minus, TokenKind::IntLiteral(5)]);
    }

    #[test]
    fn test_protected_keyword() {
        assert_eq!(lex("protected"), vec![TokenKind::Protected]);
    }

    #[test]
    fn test_compound_assignment_context() {
        assert_eq!(lex("hp -= amount"), vec![
            TokenKind::Identifier("hp".into()),
            TokenKind::MinusEq,
            TokenKind::Identifier("amount".into()),
        ]);
    }

    #[test]
    fn test_early_return_elvis() {
        assert_eq!(lex("val t = target ?: return"), vec![
            TokenKind::Val,
            TokenKind::Identifier("t".into()),
            TokenKind::Eq,
            TokenKind::Identifier("target".into()),
            TokenKind::Elvis,
            TokenKind::Return,
        ]);
    }
}
