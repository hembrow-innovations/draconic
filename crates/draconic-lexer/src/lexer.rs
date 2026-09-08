use crate::scan_ident::is_ident_continue_char;
use crate::token::{Token, TokenKind};
use draconic_diagnostics::{Diagnostic, Span};

pub struct Lexer<'a> {
    pub(crate) src: &'a str,
    pub(crate) bytes: &'a [u8],
    pub(crate) pos: usize,
    /// Stack of open template `${…}` frames. Each entry is the `{`/`}` nesting
    /// depth *inside* that expression (0 ⇒ next `}` closes the interpolation).
    pub(crate) template_expr_braces: Vec<u32>,
    /// True at BOF or after a line terminator (Annex B HTML close comment).
    pub(crate) at_line_start: bool,
    /// When true, `/` starts a RegularExpressionLiteral rather than `/` or `/=`.
    pub(crate) allow_regexp: bool,
    /// Set while skipping trivia that includes a LineTerminator; consumed by next token.
    pub(crate) had_line_terminator: bool,
    /// Annex B HTML-like comments (`<!--` / `-->`) — Script only (E19.67 modules reject).
    pub(crate) allow_html_comments: bool,
    /// Set by numeric/string scanners; consumed by `finish_token` (E19.69).
    pub(crate) pending_legacy_octal: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            template_expr_braces: Vec::new(),
            at_line_start: true,
            allow_regexp: true,
            had_line_terminator: false,
            allow_html_comments: true,
            pending_legacy_octal: false,
        }
    }

    /// Module goal: Annex B HTML-like comments are not allowed (E19.67).
    pub fn new_module(src: &'a str) -> Self {
        let mut lex = Self::new(src);
        lex.allow_html_comments = false;
        lex
    }

    pub(crate) fn finish_token(&mut self, kind: TokenKind, span: Span) -> Token {
        self.finish_token_escaped(kind, span, false)
    }

    pub(crate) fn finish_token_escaped(&mut self, kind: TokenKind, span: Span, escaped: bool) -> Token {
        let preceded_by_line_terminator = self.had_line_terminator;
        self.had_line_terminator = false;
        let legacy_octal = self.pending_legacy_octal;
        self.pending_legacy_octal = false;
        Token {
            kind,
            span,
            preceded_by_line_terminator,
            escaped,
            legacy_octal,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, Diagnostic> {
        // E19.39: HashbangComment only at the absolute start of source (`#!…`).
        self.skip_hashbang_comment();
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            if !is_eof {
                self.allow_regexp = regexp_allowed_after(&tok.kind);
            }
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// `# ! SingleLineCommentChars_opt` — only when source begins with those two bytes.
    pub(crate) fn skip_hashbang_comment(&mut self) {
        if self.pos != 0 {
            return;
        }
        if self.bytes.len() >= 2 && self.bytes[0] == b'#' && self.bytes[1] == b'!' {
            self.pos = 2;
            while self.pos < self.bytes.len() {
                let b = self.bytes[self.pos];
                // LineTerminator: LF, CR, LS (U+2028), PS (U+2029)
                if b == b'\n' || b == b'\r' {
                    break;
                }
                if b >= 0x80 {
                    let ch = self.peek_char();
                    if ch == '\u{2028}' || ch == '\u{2029}' {
                        break;
                    }
                    self.bump_char();
                } else {
                    self.pos += 1;
                }
            }
            self.at_line_start = true;
            self.had_line_terminator = true;
        }
    }

    pub(crate) fn next_token(&mut self) -> Result<Token, Diagnostic> {
        self.skip_trivia()?;
        let start = self.pos as u32;
        if self.is_eof() {
            return Ok(self.finish_token(TokenKind::Eof, Span::new(start, start)));
        }

        let b = self.peek();
        let kind = match b {
            b'(' => {
                self.bump();
                TokenKind::LParen
            }
            b')' => {
                self.bump();
                TokenKind::RParen
            }
            b'{' => {
                self.bump();
                if let Some(depth) = self.template_expr_braces.last_mut() {
                    *depth += 1;
                }
                TokenKind::LBrace
            }
            b'}' => match self.template_expr_braces.last().copied() {
                Some(0) => {
                    // Close `${…}` and resume the template.
                    self.template_expr_braces.pop();
                    return self.template_continuation(start);
                }
                Some(_) => {
                    *self.template_expr_braces.last_mut().unwrap() -= 1;
                    self.bump();
                    TokenKind::RBrace
                }
                None => {
                    self.bump();
                    TokenKind::RBrace
                }
            },
            b'[' => {
                self.bump();
                TokenKind::LBracket
            }
            b']' => {
                self.bump();
                TokenKind::RBracket
            }
            b';' => {
                self.bump();
                TokenKind::Semi
            }
            b',' => {
                self.bump();
                TokenKind::Comma
            }
            b'.' => {
                if self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) {
                    self.number_literal_leading_dot()?
                } else {
                    self.bump();
                    if self.eat(b'.') {
                        if self.eat(b'.') {
                            TokenKind::DotDotDot
                        } else {
                            return Err(Diagnostic::new(
                                "expected `...`",
                                Span::new(start, self.pos as u32),
                            ));
                        }
                    } else {
                        TokenKind::Dot
                    }
                }
            }
            b':' => {
                self.bump();
                TokenKind::Colon
            }
            b'@' => {
                self.bump();
                TokenKind::At
            }
            b'?' => {
                self.bump();
                if self.eat(b'?') {
                    if self.eat(b'=') {
                        TokenKind::QuestionQuestionEq
                    } else {
                        TokenKind::QuestionQuestion
                    }
                } else if !self.is_eof()
                    && self.peek() == b'.'
                    && !self.peek_at(1).is_some_and(|b| b.is_ascii_digit())
                {
                    // `?.` optional chaining; not when `?.` is followed by a digit (`x?.3:y`).
                    self.bump();
                    TokenKind::QuestionDot
                } else {
                    TokenKind::Question
                }
            }
            b'+' => {
                self.bump();
                if self.eat(b'+') {
                    TokenKind::PlusPlus
                } else if self.eat(b'=') {
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            b'-' => {
                self.bump();
                if self.eat(b'-') {
                    TokenKind::MinusMinus
                } else if self.eat(b'=') {
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }
            b'*' => {
                self.bump();
                if self.eat(b'*') {
                    if self.eat(b'=') {
                        TokenKind::StarStarEq
                    } else {
                        TokenKind::StarStar
                    }
                } else if self.eat(b'=') {
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            b'%' => {
                self.bump();
                if self.eat(b'=') {
                    TokenKind::PercentEq
                } else {
                    TokenKind::Percent
                }
            }
            b'/' => {
                // line/block comments handled in skip_trivia.
                if self.allow_regexp {
                    return self.scan_regexp_literal(start);
                }
                self.bump();
                if self.eat(b'=') {
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            b'!' => {
                self.bump();
                if self.eat(b'=') {
                    if self.eat(b'=') {
                        TokenKind::NotEqEq
                    } else {
                        TokenKind::NotEq
                    }
                } else {
                    TokenKind::Bang
                }
            }
            b'=' => {
                self.bump();
                if self.eat(b'=') {
                    if self.eat(b'=') {
                        TokenKind::EqEqEq
                    } else {
                        TokenKind::EqEq
                    }
                } else if self.eat(b'>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Eq
                }
            }
            b'<' => {
                self.bump();
                if self.eat(b'<') {
                    if self.eat(b'=') {
                        TokenKind::ShlEq
                    } else {
                        TokenKind::Shl
                    }
                } else if self.eat(b'=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            b'>' => {
                self.bump();
                if self.eat(b'>') {
                    if self.eat(b'>') {
                        if self.eat(b'=') {
                            TokenKind::UShrEq
                        } else {
                            TokenKind::UShr
                        }
                    } else if self.eat(b'=') {
                        TokenKind::ShrEq
                    } else {
                        TokenKind::Shr
                    }
                } else if self.eat(b'=') {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            b'&' => {
                self.bump();
                if self.eat(b'&') {
                    if self.eat(b'=') {
                        TokenKind::AndAndEq
                    } else {
                        TokenKind::AndAnd
                    }
                } else if self.eat(b'=') {
                    TokenKind::BitAndEq
                } else {
                    TokenKind::BitAnd
                }
            }
            b'|' => {
                self.bump();
                if self.eat(b'|') {
                    if self.eat(b'=') {
                        TokenKind::OrOrEq
                    } else {
                        TokenKind::OrOr
                    }
                } else if self.eat(b'=') {
                    TokenKind::BitOrEq
                } else {
                    TokenKind::BitOr
                }
            }
            b'^' => {
                self.bump();
                if self.eat(b'=') {
                    TokenKind::BitXorEq
                } else {
                    TokenKind::BitXor
                }
            }
            b'~' => {
                self.bump();
                TokenKind::Tilde
            }
            b'#' => self.private_ident(start)?,
            b'"' | b'\'' => self.string_literal()?,
            b'`' => self.template_literal()?,
            b if b.is_ascii_digit() => self.number_literal()?,
            _ if self.can_start_ident() => {
                let (kind, escaped) = self.ident_or_keyword()?;
                self.at_line_start = false;
                return Ok(self.finish_token_escaped(
                    kind,
                    Span::new(start, self.pos as u32),
                    escaped,
                ));
            }
            _ => {
                let ch = self.src[self.pos..].chars().next().expect("eof checked");
                return Err(Diagnostic::new(
                    format!("unexpected character {:?}", ch),
                    Span::new(start, start + ch.len_utf8() as u32),
                ));
            }
        };

        self.at_line_start = false;
        Ok(self.finish_token(kind, Span::new(start, self.pos as u32)))
    }

    /// `/ RegularExpressionBody / RegularExpressionFlags` (InputElementRegExp).
    pub(crate) fn scan_regexp_literal(&mut self, start: u32) -> Result<Token, Diagnostic> {
        self.bump(); // opening `/`
        let pattern_start = self.pos;
        let mut in_class = false;
        loop {
            if self.is_eof() {
                return Err(Diagnostic::new(
                    "unterminated regular expression literal",
                    Span::new(start, self.pos as u32),
                ));
            }
            let b = self.peek();
            if b == b'\\' {
                self.bump();
                if self.is_eof() {
                    return Err(Diagnostic::new(
                        "unterminated regular expression literal",
                        Span::new(start, self.pos as u32),
                    ));
                }
                // E19.69: LineTerminator includes LS/PS (not only LF/CR).
                let ch = self.peek_char();
                if is_line_terminator_char(ch) {
                    return Err(Diagnostic::new(
                        "line terminator in regular expression literal",
                        Span::new(self.pos as u32, self.pos as u32 + ch.len_utf8() as u32),
                    ));
                }
                self.bump_char();
                continue;
            }
            let ch = self.peek_char();
            if is_line_terminator_char(ch) {
                return Err(Diagnostic::new(
                    "line terminator in regular expression literal",
                    Span::new(self.pos as u32, self.pos as u32 + ch.len_utf8() as u32),
                ));
            }
            if b == b'[' && !in_class {
                in_class = true;
                self.bump();
                continue;
            }
            if b == b']' && in_class {
                in_class = false;
                self.bump();
                continue;
            }
            if b == b'/' && !in_class {
                break;
            }
            self.bump_char();
        }
        let pattern = self.src[pattern_start..self.pos].to_string();
        self.bump(); // closing `/`
        let flags_start = self.pos;
        // RegularExpressionFlags: IdentifierPart source chars only (no Unicode escapes).
        // A `\` here would begin a Unicode escape in flags → early SyntaxError.
        if !self.is_eof() && self.peek() == b'\\' {
            return Err(Diagnostic::new(
                "unicode escape sequence in regular expression flags",
                Span::new(self.pos as u32, self.pos as u32 + 1),
            ));
        }
        while !self.is_eof() && is_ident_continue_char(self.peek_char()) {
            self.bump_char();
        }
        // Trailing `\` after flags (e.g. `/./i\u0067`) is also a flags unicode-escape error.
        if !self.is_eof() && self.peek() == b'\\' {
            return Err(Diagnostic::new(
                "unicode escape sequence in regular expression flags",
                Span::new(self.pos as u32, self.pos as u32 + 1),
            ));
        }
        let flags = self.src[flags_start..self.pos].to_string();
        if let Err(mut err) = crate::regexp::validate_regexp_literal(&pattern, &flags) {
            err.span = Span::new(start, self.pos as u32);
            return Err(err);
        }
        self.at_line_start = false;
        Ok(self.finish_token(
            TokenKind::RegExp { pattern, flags },
            Span::new(start, self.pos as u32),
        ))
    }

    pub(crate) fn peek(&self) -> u8 {
        self.bytes[self.pos]
    }

    pub(crate) fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    pub(crate) fn peek_char(&self) -> char {
        self.src[self.pos..].chars().next().expect("eof checked")
    }

    pub(crate) fn bump(&mut self) -> u8 {
        let b = self.bytes[self.pos];
        self.pos += 1;
        b
    }

    pub(crate) fn bump_char(&mut self) -> char {
        let ch = self.peek_char();
        self.pos += ch.len_utf8();
        ch
    }

    pub(crate) fn eat(&mut self, b: u8) -> bool {
        if !self.is_eof() && self.peek() == b {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    /// Whether the UTF-8 scalar at `pos` is IdentifierPart (char-boundary `pos`).
    pub(crate) fn is_ident_continue_at(&self, pos: usize) -> bool {
        if pos >= self.bytes.len() {
            return false;
        }
        let ch = self.src[pos..].chars().next().expect("pos in bounds");
        is_ident_continue_char(ch)
    }
}
/// After `kind`, may the next `/` start a regexp literal (vs division)?
pub(crate) fn regexp_allowed_after(kind: &TokenKind) -> bool {
    !matches!(
        kind,
        TokenKind::Ident(_)
            | TokenKind::PrivateIdent(_)
            | TokenKind::Number(_)
            | TokenKind::BigInt(_)
            | TokenKind::String(_)
            | TokenKind::TemplateNoSubstitution(_)
            | TokenKind::TemplateTail(_)
            | TokenKind::RegExp { .. }
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null
            | TokenKind::This
            | TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::PlusPlus
            | TokenKind::MinusMinus
    )
}

pub(crate) fn is_line_terminator_char(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// ECMA-262 WhiteSpace beyond single-byte TAB/VT/FF/SP (handled inline).
pub(crate) fn is_whitespace_char(ch: char) -> bool {
    matches!(ch, '\u{00A0}' | '\u{FEFF}') || unicode_general_category_space_separator(ch)
}

fn unicode_general_category_space_separator(ch: char) -> bool {
    // Zs: Space_Separator (USP in ECMA-262).
    matches!(
        ch,
        '\u{1680}' | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}
pub(crate) fn hex_digit(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a') as u32 + 10),
        b'A'..=b'F' => Some((b - b'A') as u32 + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{Lexer, TokenKind};

    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::new(src)
            .tokenize()
            .expect("lex")
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn lex_hashbang_at_start() {
        assert_eq!(
            kinds("#! anything\n1"),
            vec![TokenKind::Number("1".into()), TokenKind::Eof]
        );
        // Escaped bang is not a hashbang — `#` starts a private ident.
        assert!(Lexer::new("#\\u{21}\n1").tokenize().is_err());
    }
    #[test]
    fn lex_regexp_literal() {
        assert_eq!(
            kinds(r#"let r = /a+b/i;"#),
            vec![
                TokenKind::Let,
                TokenKind::Ident("r".into()),
                TokenKind::Eq,
                TokenKind::RegExp {
                    pattern: "a+b".into(),
                    flags: "i".into(),
                },
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds(r#"/a\/b/"#),
            vec![
                TokenKind::RegExp {
                    pattern: r#"a\/b"#.into(),
                    flags: "".into(),
                },
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds(r#"/[a/]/"#),
            vec![
                TokenKind::RegExp {
                    pattern: "[a/]".into(),
                    flags: "".into(),
                },
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("10 / 2"),
            vec![
                TokenKind::Number("10".into()),
                TokenKind::Slash,
                TokenKind::Number("2".into()),
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("a /= b"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::SlashEq,
                TokenKind::Ident("b".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_regexp_literal_early_errors() {
        // Invalid / duplicate flags, invalid pattern, unicode escape in flags.
        assert!(Lexer::new("/./G").tokenize().is_err());
        assert!(Lexer::new("/./gig").tokenize().is_err());
        assert!(Lexer::new("/?/").tokenize().is_err());
        assert!(Lexer::new("/./\\u0067").tokenize().is_err());
        assert!(Lexer::new("/./uv").tokenize().is_err());
    }
    #[test]
    fn lex_operators_and_strings() {
        assert_eq!(
            kinds(r#"a === "hi" && b !== 'x'"#),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::EqEqEq,
                TokenKind::String("hi".into()),
                TokenKind::AndAnd,
                TokenKind::Ident("b".into()),
                TokenKind::NotEqEq,
                TokenKind::String("x".into()),
                TokenKind::Eof,
            ]
        );
    }
    #[test]
    fn lex_keywords() {
        assert_eq!(
            kinds("true false null typeof void delete"),
            vec![
                TokenKind::True,
                TokenKind::False,
                TokenKind::Null,
                TokenKind::TypeOf,
                TokenKind::Void,
                TokenKind::Delete,
                TokenKind::Eof,
            ]
        );
    }
    #[test]
    fn lex_star_star() {
        assert_eq!(
            kinds("2 ** 3 * 4"),
            vec![
                TokenKind::Number("2".into()),
                TokenKind::StarStar,
                TokenKind::Number("3".into()),
                TokenKind::Star,
                TokenKind::Number("4".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_dot_dot_dot() {
        assert_eq!(
            kinds("function f(...a) {}"),
            vec![
                TokenKind::Function,
                TokenKind::Ident("f".into()),
                TokenKind::LParen,
                TokenKind::DotDotDot,
                TokenKind::Ident("a".into()),
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_arrow() {
        assert_eq!(
            kinds("(x) => x"),
            vec![
                TokenKind::LParen,
                TokenKind::Ident("x".into()),
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Ident("x".into()),
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("a => b = 1"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::Arrow,
                TokenKind::Ident("b".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_bitwise_ops() {
        assert_eq!(
            kinds("a & b | c ^ ~d << e >> f >>> g"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::BitAnd,
                TokenKind::Ident("b".into()),
                TokenKind::BitOr,
                TokenKind::Ident("c".into()),
                TokenKind::BitXor,
                TokenKind::Tilde,
                TokenKind::Ident("d".into()),
                TokenKind::Shl,
                TokenKind::Ident("e".into()),
                TokenKind::Shr,
                TokenKind::Ident("f".into()),
                TokenKind::UShr,
                TokenKind::Ident("g".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_update_ops() {
        assert_eq!(
            kinds("++a --b c++ d-- +e -f"),
            vec![
                TokenKind::PlusPlus,
                TokenKind::Ident("a".into()),
                TokenKind::MinusMinus,
                TokenKind::Ident("b".into()),
                TokenKind::Ident("c".into()),
                TokenKind::PlusPlus,
                TokenKind::Ident("d".into()),
                TokenKind::MinusMinus,
                TokenKind::Plus,
                TokenKind::Ident("e".into()),
                TokenKind::Minus,
                TokenKind::Ident("f".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_nullish_and_logical_assign() {
        assert_eq!(
            kinds("a ?? b ??= c &&= d ||= e ? f : g"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::QuestionQuestion,
                TokenKind::Ident("b".into()),
                TokenKind::QuestionQuestionEq,
                TokenKind::Ident("c".into()),
                TokenKind::AndAndEq,
                TokenKind::Ident("d".into()),
                TokenKind::OrOrEq,
                TokenKind::Ident("e".into()),
                TokenKind::Question,
                TokenKind::Ident("f".into()),
                TokenKind::Colon,
                TokenKind::Ident("g".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_optional_chain() {
        assert_eq!(
            kinds("a?.b?.[c]?.()"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::QuestionDot,
                TokenKind::Ident("b".into()),
                TokenKind::QuestionDot,
                TokenKind::LBracket,
                TokenKind::Ident("c".into()),
                TokenKind::RBracket,
                TokenKind::QuestionDot,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
        // `?.` followed by digit is ternary + leading-dot number, not optional chain.
        assert_eq!(
            kinds("a?.3:0"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::Question,
                TokenKind::Number(".3".into()),
                TokenKind::Colon,
                TokenKind::Number("0".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_compound_assignment() {
        assert_eq!(
            kinds("a += b -= c *= d /= e %= f **= g <<= h >>= i >>>= j &= k ^= l |= m"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::PlusEq,
                TokenKind::Ident("b".into()),
                TokenKind::MinusEq,
                TokenKind::Ident("c".into()),
                TokenKind::StarEq,
                TokenKind::Ident("d".into()),
                TokenKind::SlashEq,
                TokenKind::Ident("e".into()),
                TokenKind::PercentEq,
                TokenKind::Ident("f".into()),
                TokenKind::StarStarEq,
                TokenKind::Ident("g".into()),
                TokenKind::ShlEq,
                TokenKind::Ident("h".into()),
                TokenKind::ShrEq,
                TokenKind::Ident("i".into()),
                TokenKind::UShrEq,
                TokenKind::Ident("j".into()),
                TokenKind::BitAndEq,
                TokenKind::Ident("k".into()),
                TokenKind::BitXorEq,
                TokenKind::Ident("l".into()),
                TokenKind::BitOrEq,
                TokenKind::Ident("m".into()),
                TokenKind::Eof,
            ]
        );
    }
}
