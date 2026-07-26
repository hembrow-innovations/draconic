use draconic_diagnostics::{Diagnostic, Span};
use std::fmt;

/// ECMAScript string value: a sequence of UTF-16 code units (may include unpaired surrogates).
#[derive(Clone, PartialEq, Eq, Default)]
pub struct JsString {
    units: Vec<u16>,
}

impl JsString {
    pub fn new() -> Self {
        Self { units: Vec::new() }
    }

    pub fn units(&self) -> &[u16] {
        &self.units
    }

    pub fn push_code_unit(&mut self, unit: u16) {
        self.units.push(unit);
    }

    /// Push a Unicode scalar value as one or two UTF-16 code units.
    pub fn push_scalar(&mut self, c: char) {
        let mut buf = [0u16; 2];
        for u in c.encode_utf16(&mut buf) {
            self.units.push(*u);
        }
    }

    /// Push a code point from `\xHH` / `\uXXXX` (any 16-bit unit, incl. surrogates)
    /// or `\u{…}` scalar (validated by caller for braced form).
    pub fn push_code_point_unit(&mut self, cp: u32) -> Result<(), ()> {
        if cp <= 0xFFFF {
            self.units.push(cp as u16);
            Ok(())
        } else if cp <= 0x10FFFF {
            let c = cp - 0x10000;
            self.units.push(0xD800 + ((c >> 10) as u16));
            self.units.push(0xDC00 + ((c & 0x3FF) as u16));
            Ok(())
        } else {
            Err(())
        }
    }

    /// Lossy UTF-8 for diagnostics/dumps (unpaired surrogates → U+FFFD).
    pub fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(&self.units)
    }

    /// Well-formed UTF-16 only; `None` if unpaired surrogates present.
    pub fn to_string_strict(&self) -> Option<String> {
        String::from_utf16(&self.units).ok()
    }
}

impl From<&str> for JsString {
    fn from(s: &str) -> Self {
        Self {
            units: s.encode_utf16().collect(),
        }
    }
}

impl From<String> for JsString {
    fn from(s: String) -> Self {
        JsString::from(s.as_str())
    }
}

impl fmt::Debug for JsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("JsString")
            .field(&self.to_string_lossy())
            .finish()
    }
}

impl fmt::Display for JsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// True when a LineTerminator was skipped immediately before this token
    /// (restricted productions: postfix `++`/`--`, `continue`/`break`/`return`/`throw`).
    pub preceded_by_line_terminator: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // punctuators
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Comma,
    Dot,
    /// `...` rest/spread
    DotDotDot,
    Colon,
    Question,
    /// `?.` optional chaining punctuator (not when followed by a decimal digit).
    QuestionDot,
    QuestionQuestion,
    QuestionQuestionEq,
    // operators
    Plus,
    PlusPlus,
    PlusEq,
    Minus,
    MinusMinus,
    MinusEq,
    Star,
    StarStar,
    StarStarEq,
    StarEq,
    Slash,
    SlashEq,
    Percent,
    PercentEq,
    Bang,
    Eq,
    EqEq,
    EqEqEq,
    /// `=>` arrow function punctuator
    Arrow,
    NotEq,
    NotEqEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    AndAndEq,
    OrOr,
    OrOrEq,
    BitAnd,
    BitAndEq,
    BitOr,
    BitOrEq,
    BitXor,
    BitXorEq,
    Tilde,
    Shl,
    ShlEq,
    Shr,
    ShrEq,
    UShr,
    UShrEq,
    // keywords / atoms
    Ident(String),
    /// `#name` private identifier (name without `#`).
    PrivateIdent(String),
    Number(String),
    /// BigInt integer literal including `n` suffix (e.g. `1n`, `0xffn`).
    BigInt(String),
    String(JsString),
    /// `` `foo` `` — no `${` interpolations.
    TemplateNoSubstitution(JsString),
    /// `` `foo${ `` — cooked head before first interpolation.
    TemplateHead(JsString),
    /// `` }foo${ `` — cooked middle between interpolations.
    TemplateMiddle(JsString),
    /// `` }foo` `` — cooked tail after last interpolation.
    TemplateTail(JsString),
    True,
    False,
    Null,
    Let,
    Const,
    Var,
    TypeOf,
    Void,
    Delete,
    If,
    Else,
    While,
    Do,
    For,
    Break,
    Continue,
    Switch,
    Case,
    Default,
    In,
    InstanceOf,
    Of,
    Function,
    Async,
    Await,
    Yield,
    Return,
    This,
    New,
    Class,
    Extends,
    Super,
    Static,
    Throw,
    Try,
    Catch,
    Finally,
    With,
    Import,
    Export,
    From,
    As,
    /// `/pattern/flags` regular expression literal (pattern body without slashes).
    RegExp {
        pattern: String,
        flags: String,
    },
    // other
    Eof,
}

enum TemplateScanEnd {
    Tick,
    DollarBrace,
}

pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    /// Stack of open template `${…}` frames. Each entry is the `{`/`}` nesting
    /// depth *inside* that expression (0 ⇒ next `}` closes the interpolation).
    template_expr_braces: Vec<u32>,
    /// True at BOF or after a line terminator (Annex B HTML close comment).
    at_line_start: bool,
    /// When true, `/` starts a RegularExpressionLiteral rather than `/` or `/=`.
    allow_regexp: bool,
    /// Set while skipping trivia that includes a LineTerminator; consumed by next token.
    had_line_terminator: bool,
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
        }
    }

    fn finish_token(&mut self, kind: TokenKind, span: Span) -> Token {
        let preceded_by_line_terminator = self.had_line_terminator;
        self.had_line_terminator = false;
        Token {
            kind,
            span,
            preceded_by_line_terminator,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, Diagnostic> {
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

    fn next_token(&mut self) -> Result<Token, Diagnostic> {
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
            _ if self.can_start_ident() => self.ident_or_keyword()?,
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

    fn skip_trivia(&mut self) -> Result<(), Diagnostic> {
        loop {
            if self.is_eof() {
                return Ok(());
            }
            match self.peek() {
                // WhiteSpace: TAB, VT, FF, SP (multi-byte USP/NBSP/ZWNBSP below).
                b' ' | b'\t' | 0x0b | 0x0c => {
                    self.bump();
                }
                b'\r' => {
                    self.bump();
                    if !self.is_eof() && self.peek() == b'\n' {
                        self.bump();
                    }
                    self.at_line_start = true;
                    self.had_line_terminator = true;
                }
                b'\n' => {
                    self.bump();
                    self.at_line_start = true;
                    self.had_line_terminator = true;
                }
                b'/' if self.peek_at(1) == Some(b'/') => {
                    self.bump();
                    self.bump();
                    while !self.is_eof() {
                        let ch = self.peek_char();
                        if is_line_terminator_char(ch) {
                            break;
                        }
                        self.bump_char();
                    }
                }
                b'/' if self.peek_at(1) == Some(b'*') => {
                    let start = self.pos as u32;
                    self.bump();
                    self.bump();
                    let mut saw_line_terminator = false;
                    loop {
                        if self.is_eof() {
                            return Err(Diagnostic::new(
                                "unterminated block comment",
                                Span::new(start, self.pos as u32),
                            ));
                        }
                        if self.peek() == b'*' && self.peek_at(1) == Some(b'/') {
                            self.bump();
                            self.bump();
                            break;
                        }
                        let ch = self.peek_char();
                        if is_line_terminator_char(ch) {
                            saw_line_terminator = true;
                        }
                        self.bump_char();
                    }
                    if saw_line_terminator {
                        self.at_line_start = true;
                        self.had_line_terminator = true;
                    }
                }
                // Annex B.1.3 SingleLineHTMLOpenComment: `<!--` …
                b'<' if self.peek_at(1) == Some(b'!')
                    && self.peek_at(2) == Some(b'-')
                    && self.peek_at(3) == Some(b'-') =>
                {
                    self.bump();
                    self.bump();
                    self.bump();
                    self.bump();
                    while !self.is_eof() {
                        let ch = self.peek_char();
                        if is_line_terminator_char(ch) {
                            break;
                        }
                        self.bump_char();
                    }
                }
                // Annex B.1.3 HTMLCloseComment at line start: `-->` …
                b'-' if self.at_line_start
                    && self.peek_at(1) == Some(b'-')
                    && self.peek_at(2) == Some(b'>') =>
                {
                    self.bump();
                    self.bump();
                    self.bump();
                    while !self.is_eof() {
                        let ch = self.peek_char();
                        if is_line_terminator_char(ch) {
                            break;
                        }
                        self.bump_char();
                    }
                }
                _ => {
                    let ch = self.peek_char();
                    if is_whitespace_char(ch) {
                        self.bump_char();
                    } else if ch == '\u{2028}' || ch == '\u{2029}' {
                        self.bump_char();
                        self.at_line_start = true;
                        self.had_line_terminator = true;
                    } else {
                        return Ok(());
                    }
                }
            }
        }
    }

    fn string_literal(&mut self) -> Result<TokenKind, Diagnostic> {
        let quote = self.bump();
        let start = self.pos as u32;
        let mut value = JsString::new();
        while !self.is_eof() {
            let c = self.peek();
            if c == quote {
                self.bump();
                return Ok(TokenKind::String(value));
            }
            if c == b'\\' {
                self.bump();
                self.scan_escape_into(&mut value, true)?;
            } else if c == b'\n' {
                return Err(Diagnostic::new(
                    "unterminated string literal",
                    Span::new(start.saturating_sub(1), self.pos as u32),
                ));
            } else {
                self.scan_source_char_into(&mut value);
            }
        }
        Err(Diagnostic::new(
            "unterminated string literal",
            Span::new(start.saturating_sub(1), self.pos as u32),
        ))
    }

    /// Scan `` `…` `` or `` `…${ `` at the opening backtick.
    fn template_literal(&mut self) -> Result<TokenKind, Diagnostic> {
        let start = self.pos as u32;
        self.bump(); // `
        let (value, end) = self.scan_template_chars(start)?;
        match end {
            TemplateScanEnd::Tick => Ok(TokenKind::TemplateNoSubstitution(value)),
            TemplateScanEnd::DollarBrace => {
                self.template_expr_braces.push(0);
                Ok(TokenKind::TemplateHead(value))
            }
        }
    }

    /// After a template `${expr` closes with `}`, scan middle/tail (leading `}` already at pos).
    fn template_continuation(&mut self, start: u32) -> Result<Token, Diagnostic> {
        self.bump(); // consume closing `}` of `${…}`
        let (value, end) = self.scan_template_chars(start)?;
        let kind = match end {
            TemplateScanEnd::Tick => TokenKind::TemplateTail(value),
            TemplateScanEnd::DollarBrace => {
                self.template_expr_braces.push(0);
                TokenKind::TemplateMiddle(value)
            }
        };
        self.at_line_start = false;
        Ok(self.finish_token(kind, Span::new(start, self.pos as u32)))
    }

    fn scan_template_chars(
        &mut self,
        start: u32,
    ) -> Result<(JsString, TemplateScanEnd), Diagnostic> {
        let mut value = JsString::new();
        while !self.is_eof() {
            let c = self.peek();
            if c == b'`' {
                self.bump();
                return Ok((value, TemplateScanEnd::Tick));
            }
            if c == b'$' && self.peek_at(1) == Some(b'{') {
                self.bump(); // $
                self.bump(); // {
                return Ok((value, TemplateScanEnd::DollarBrace));
            }
            if c == b'\\' {
                self.bump();
                self.scan_escape_into(&mut value, false)?;
            } else {
                self.scan_source_char_into(&mut value);
            }
        }
        Err(Diagnostic::new(
            "unterminated template literal",
            Span::new(start, self.pos as u32),
        ))
    }

    /// Decode one UTF-8 scalar from source into UTF-16 code units.
    fn scan_source_char_into(&mut self, value: &mut JsString) {
        let ch = self.src[self.pos..].chars().next().expect("eof checked");
        self.pos += ch.len_utf8();
        value.push_scalar(ch);
    }

    /// Cook a single escape sequence after the leading `\`.
    /// Supports basic escapes, `\xHH`, `\uXXXX` (any code unit incl. surrogates),
    /// and `\u{X…}` (well-formed scalar values only).
    /// When `allow_legacy_octal` (string literals, Annex B.1.2): `\0`–`\377` octal
    /// and NonOctalDecimal `\8`/`\9`. Templates pass `false` (bare `\0` only).
    /// LineContinuation (`\` + LineTerminatorSequence) contributes no code units.
    /// IdentityEscape / NonEscapeSequence consume a full UTF-8 scalar (not one byte).
    fn scan_escape_into(
        &mut self,
        value: &mut JsString,
        allow_legacy_octal: bool,
    ) -> Result<(), Diagnostic> {
        if self.is_eof() {
            return Err(Diagnostic::new(
                "unterminated escape sequence",
                Span::new(self.pos.saturating_sub(1) as u32, self.pos as u32),
            ));
        }
        // LineContinuation :: `\` LineTerminatorSequence → empty SV.
        if self.try_consume_line_terminator_sequence() {
            return Ok(());
        }
        let esc_start = self.pos as u32;
        let esc = self.peek();
        match esc {
            b'b' => {
                self.bump();
                value.push_scalar('\u{0008}');
            }
            b'f' => {
                self.bump();
                value.push_scalar('\u{000C}');
            }
            b'n' => {
                self.bump();
                value.push_scalar('\n');
            }
            b'r' => {
                self.bump();
                value.push_scalar('\r');
            }
            b't' => {
                self.bump();
                value.push_scalar('\t');
            }
            b'v' => {
                self.bump();
                value.push_scalar('\u{000B}');
            }
            b'\\' => {
                self.bump();
                value.push_scalar('\\');
            }
            b'\'' => {
                self.bump();
                value.push_scalar('\'');
            }
            b'"' => {
                self.bump();
                value.push_scalar('"');
            }
            b'`' => {
                self.bump();
                value.push_scalar('`');
            }
            b'$' => {
                self.bump();
                value.push_scalar('$');
            }
            b'0'..=b'7' if allow_legacy_octal => {
                let first = self.bump();
                self.scan_legacy_octal_escape_into(value, first);
            }
            b'0' => {
                self.bump();
                value.push_scalar('\0');
            }
            b'x' => {
                self.bump();
                let cp = self.scan_hex_digits(2, esc_start)?;
                push_code_point(value, cp, esc_start, self.pos as u32, false)?;
            }
            b'u' => {
                self.bump();
                if self.peek() == b'{' {
                    self.bump(); // {
                    let cp = self.scan_braced_hex(esc_start)?;
                    // Braced form: scalar values only (no lone surrogates).
                    push_code_point(value, cp, esc_start, self.pos as u32, true)?;
                } else {
                    let cp = self.scan_hex_digits(4, esc_start)?;
                    // `\uXXXX` may be any 16-bit code unit, including surrogates.
                    push_code_point(value, cp, esc_start, self.pos as u32, false)?;
                }
            }
            // Annex B NonOctalDecimalEscapeSequence `\8` / `\9`, and IdentityEscape /
            // NonEscapeSequence — full UTF-8 scalar (e.g. Cyrillic `"\А"`).
            _ => self.scan_source_char_into(value),
        }
        Ok(())
    }

    /// Consume one LineTerminatorSequence if present at `pos`. Returns true when consumed.
    fn try_consume_line_terminator_sequence(&mut self) -> bool {
        if self.is_eof() {
            return false;
        }
        // `src` is valid UTF-8; `pos` must stay on a char boundary (callers ensure this).
        let ch = self.src[self.pos..].chars().next().expect("eof checked");
        match ch {
            '\n' => {
                self.pos += 1;
                true
            }
            '\r' => {
                self.pos += 1;
                if !self.is_eof() && self.peek() == b'\n' {
                    self.pos += 1;
                }
                true
            }
            '\u{2028}' | '\u{2029}' => {
                self.pos += ch.len_utf8();
                true
            }
            _ => false,
        }
    }

    /// Annex B.1.2 LegacyOctalEscapeSequence after the first OctalDigit `first` (already consumed).
    fn scan_legacy_octal_escape_into(&mut self, value: &mut JsString, first: u8) {
        let d0 = (first - b'0') as u16;
        let mut n = d0;
        if first <= b'3' {
            if let Some(d1) = self.peek_octal_digit() {
                self.bump();
                n = n * 8 + d1;
                if let Some(d2) = self.peek_octal_digit() {
                    self.bump();
                    n = n * 8 + d2;
                }
            }
        } else if let Some(d1) = self.peek_octal_digit() {
            // FourToSeven OctalDigit — at most two digits total.
            self.bump();
            n = n * 8 + d1;
        }
        value.push_code_unit(n);
    }

    fn peek_octal_digit(&self) -> Option<u16> {
        if self.is_eof() {
            return None;
        }
        let b = self.peek();
        if (b'0'..=b'7').contains(&b) {
            Some((b - b'0') as u16)
        } else {
            None
        }
    }

    fn scan_hex_digits(&mut self, n: usize, esc_start: u32) -> Result<u32, Diagnostic> {
        let mut value: u32 = 0;
        for _ in 0..n {
            if self.is_eof() {
                return Err(Diagnostic::new(
                    "invalid hex escape sequence",
                    Span::new(esc_start, self.pos as u32),
                ));
            }
            let b = self.peek();
            let digit = match hex_digit(b) {
                Some(d) => d,
                None => {
                    return Err(Diagnostic::new(
                        "invalid hex escape sequence",
                        Span::new(esc_start, self.pos as u32),
                    ));
                }
            };
            self.bump();
            value = (value << 4) | digit;
        }
        Ok(value)
    }

    fn scan_braced_hex(&mut self, esc_start: u32) -> Result<u32, Diagnostic> {
        if self.is_eof() || hex_digit(self.peek()).is_none() {
            return Err(Diagnostic::new(
                "invalid Unicode escape sequence",
                Span::new(esc_start, self.pos as u32),
            ));
        }
        let mut value: u32 = 0;
        let mut digits = 0usize;
        while !self.is_eof() {
            let b = self.peek();
            if b == b'}' {
                self.bump();
                if digits == 0 {
                    return Err(Diagnostic::new(
                        "invalid Unicode escape sequence",
                        Span::new(esc_start, self.pos as u32),
                    ));
                }
                return Ok(value);
            }
            let digit = match hex_digit(b) {
                Some(d) => d,
                None => {
                    return Err(Diagnostic::new(
                        "invalid Unicode escape sequence",
                        Span::new(esc_start, self.pos as u32),
                    ));
                }
            };
            self.bump();
            digits += 1;
            if digits > 6 {
                return Err(Diagnostic::new(
                    "invalid Unicode escape sequence",
                    Span::new(esc_start, self.pos as u32),
                ));
            }
            value = (value << 4) | digit;
        }
        Err(Diagnostic::new(
            "invalid Unicode escape sequence",
            Span::new(esc_start, self.pos as u32),
        ))
    }

    fn number_literal(&mut self) -> Result<TokenKind, Diagnostic> {
        let start = self.pos;
        // Non-decimal integer: 0x / 0b / 0o (case-insensitive prefix).
        if self.peek() == b'0' {
            match self.peek_at(1) {
                Some(b'x' | b'X') => {
                    self.bump(); // 0
                    self.bump(); // x
                    self.scan_radix_digits(16, start)?;
                    return self.finish_number_or_bigint(start, true);
                }
                Some(b'b' | b'B') => {
                    self.bump();
                    self.bump();
                    self.scan_radix_digits(2, start)?;
                    return self.finish_number_or_bigint(start, true);
                }
                Some(b'o' | b'O') => {
                    self.bump();
                    self.bump();
                    self.scan_radix_digits(8, start)?;
                    return self.finish_number_or_bigint(start, true);
                }
                // Annex B.1.1: `0` + digit → LegacyOctalIntegerLiteral or NonOctalDecimalIntegerLiteral.
                // Also reject numeric separators after a lone leading `0` (`0_1`).
                Some(b'0'..=b'9' | b'_') => {
                    return self.scan_zero_prefixed_decimal_or_legacy_octal(start);
                }
                _ => {}
            }
        }

        // DecimalIntegerLiteral (with optional numeric separators).
        self.scan_decimal_integer_digits(start)?;

        // Optional fractional part: `.` DecimalDigits_opt (then ExponentPart_opt).
        // Consume `.` when it continues the DecimalLiteral: digit, invalid `_…`, or
        // exponent. Leave `.` for member access (`1.toString`) and `...`.
        let mut is_integer = true;
        if !self.is_eof() && self.peek() == b'.' {
            let next = self.peek_at(1);
            if next != Some(b'.')
                && (next.is_some_and(|b| b.is_ascii_digit())
                    || next == Some(b'_')
                    || next == Some(b'e')
                    || next == Some(b'E'))
            {
                self.bump(); // .
                // DecimalDigits_opt — empty ok before exponent; `_` alone / leading `_` invalid.
                if !self.is_eof() && (self.peek().is_ascii_digit() || self.peek() == b'_') {
                    self.scan_decimal_digits_required(start)?;
                }
                is_integer = false;
            }
        }

        let had_exponent = self.scan_exponent_opt(start)?;
        if had_exponent {
            is_integer = false;
        }
        self.finish_number_or_bigint(start, is_integer)
    }

    /// Annex B.1.1 / DecimalIntegerLiteral NonOctalDecimal branch after a leading `0`
    /// when the next character is a digit or `_`.
    ///
    /// - Pure octal digits → LegacyOctalIntegerLiteral (MV base-8); no `.`/`e`/`n`.
    /// - Any `8`/`9` → NonOctalDecimalIntegerLiteral (MV decimal); `.`/`e` allowed; no `n`.
    /// - `_` after leading `0` is always invalid.
    fn scan_zero_prefixed_decimal_or_legacy_octal(
        &mut self,
        start: usize,
    ) -> Result<TokenKind, Diagnostic> {
        debug_assert_eq!(self.peek(), b'0');
        self.bump(); // leading 0

        if !self.is_eof() && self.peek() == b'_' {
            return Err(Diagnostic::new(
                "numeric separator cannot be used after leading 0",
                Span::new(start as u32, (self.pos + 1) as u32),
            ));
        }

        let mut has_non_octal = false;
        while !self.is_eof() {
            let b = self.peek();
            if b.is_ascii_digit() {
                if b == b'8' || b == b'9' {
                    has_non_octal = true;
                }
                self.bump();
                continue;
            }
            if b == b'_' {
                return Err(Diagnostic::new(
                    "numeric separator cannot be used after leading 0",
                    Span::new(start as u32, (self.pos + 1) as u32),
                ));
            }
            break;
        }

        if has_non_octal {
            // NonOctalDecimalIntegerLiteral: optional fraction + exponent (decimal MV).
            if !self.is_eof() && self.peek() == b'.' {
                let next = self.peek_at(1);
                if next != Some(b'.')
                    && (next.is_some_and(|b| b.is_ascii_digit())
                        || next == Some(b'_')
                        || next == Some(b'e')
                        || next == Some(b'E'))
                {
                    self.bump(); // .
                    if !self.is_eof() && (self.peek().is_ascii_digit() || self.peek() == b'_') {
                        self.scan_decimal_digits_required(start)?;
                    }
                }
            }
            self.scan_exponent_opt(start)?;
            // BigInt suffix not allowed on zero-prefixed multi-digit forms.
            if !self.is_eof() && self.peek() == b'n' && !self.is_ident_continue_at(self.pos + 1) {
                return Err(Diagnostic::new(
                    "Invalid BigInt literal",
                    Span::new(start as u32, (self.pos + 1) as u32),
                ));
            }
            let raw = self.src[start..self.pos].to_string();
            return Ok(TokenKind::Number(canonicalize_leading_zero_decimal(&raw)));
        }

        // LegacyOctalIntegerLiteral: do not consume `.`/`e` as part of this token.
        if !self.is_eof() && self.peek() == b'n' && !self.is_ident_continue_at(self.pos + 1) {
            return Err(Diagnostic::new(
                "Invalid BigInt literal",
                Span::new(start as u32, (self.pos + 1) as u32),
            ));
        }
        let raw = &self.src[start..self.pos];
        let mv = legacy_octal_mv(raw);
        Ok(TokenKind::Number(mv))
    }

    /// Leading-dot decimal: `.` DecimalDigits ExponentPart_opt
    fn number_literal_leading_dot(&mut self) -> Result<TokenKind, Diagnostic> {
        let start = self.pos;
        self.bump(); // .
        self.scan_decimal_digits_required(start)?;
        self.scan_exponent_opt(start)?;
        // Leading-dot forms are never BigInt (`n` after a float is invalid).
        self.finish_number_or_bigint(start, false)
    }

    /// Finish a numeric token; optional `n` suffix yields BigInt when `allow_bigint`.
    fn finish_number_or_bigint(
        &mut self,
        start: usize,
        allow_bigint: bool,
    ) -> Result<TokenKind, Diagnostic> {
        if !self.is_eof() && self.peek() == b'n' && !self.is_ident_continue_at(self.pos + 1) {
            if !allow_bigint {
                return Err(Diagnostic::new(
                    "Invalid BigInt literal",
                    Span::new(start as u32, (self.pos + 1) as u32),
                ));
            }
            self.bump(); // n
            let raw = self.src[start..self.pos].to_string();
            return Ok(TokenKind::BigInt(raw));
        }
        let raw = self.src[start..self.pos].to_string();
        Ok(TokenKind::Number(raw))
    }

    /// Returns `true` if an exponent part was consumed.
    fn scan_exponent_opt(&mut self, start: usize) -> Result<bool, Diagnostic> {
        if self.is_eof() {
            return Ok(false);
        }
        let e = self.peek();
        if e != b'e' && e != b'E' {
            return Ok(false);
        }
        self.bump();
        if !self.is_eof() && (self.peek() == b'+' || self.peek() == b'-') {
            self.bump();
        }
        self.scan_decimal_digits_required(start)?;
        Ok(true)
    }

    /// Decimal integer digits with optional `_` separators (at least one digit already at pos).
    fn scan_decimal_integer_digits(&mut self, start: usize) -> Result<(), Diagnostic> {
        if self.is_eof() || !self.peek().is_ascii_digit() {
            return Err(Diagnostic::new(
                "invalid number literal",
                Span::new(start as u32, self.pos as u32),
            ));
        }
        self.scan_decimal_digits_required(start)
    }

    /// One or more decimal digits with optional `_` between digits (not leading/trailing/adjacent).
    fn scan_decimal_digits_required(&mut self, start: usize) -> Result<(), Diagnostic> {
        if self.is_eof() || !self.peek().is_ascii_digit() {
            return Err(Diagnostic::new(
                "invalid number literal",
                Span::new(start as u32, self.pos as u32),
            ));
        }
        self.bump();
        loop {
            if self.is_eof() {
                break;
            }
            if self.peek().is_ascii_digit() {
                self.bump();
                continue;
            }
            if self.peek() == b'_' {
                let after = self.peek_at(1);
                if after.is_some_and(|b| b.is_ascii_digit()) {
                    self.bump(); // _
                    self.bump(); // digit
                    continue;
                }
                return Err(Diagnostic::new(
                    "invalid numeric separator in number literal",
                    Span::new(start as u32, (self.pos + 1) as u32),
                ));
            }
            break;
        }
        Ok(())
    }

    /// Radix digits after `0x`/`0b`/`0o` prefix; requires ≥1 digit; allows `_` separators.
    fn scan_radix_digits(&mut self, radix: u32, start: usize) -> Result<(), Diagnostic> {
        let is_digit = |b: u8| -> bool {
            match radix {
                2 => b == b'0' || b == b'1',
                8 => (b'0'..=b'7').contains(&b),
                16 => hex_digit(b).is_some(),
                _ => false,
            }
        };

        if self.is_eof() || !is_digit(self.peek()) {
            return Err(Diagnostic::new(
                "invalid number literal",
                Span::new(start as u32, self.pos as u32),
            ));
        }
        self.bump();
        loop {
            if self.is_eof() {
                break;
            }
            if is_digit(self.peek()) {
                self.bump();
                continue;
            }
            if self.peek() == b'_' {
                let after = self.peek_at(1);
                if after.is_some_and(|b| is_digit(b)) {
                    self.bump(); // _
                    self.bump(); // digit
                    continue;
                }
                return Err(Diagnostic::new(
                    "invalid numeric separator in number literal",
                    Span::new(start as u32, (self.pos + 1) as u32),
                ));
            }
            break;
        }
        Ok(())
    }

    /// `#IdentifierName` private identifier.
    fn private_ident(&mut self, start: u32) -> Result<TokenKind, Diagnostic> {
        self.bump(); // `#`
        if self.is_eof() || !self.can_start_ident() {
            return Err(Diagnostic::new(
                "expected identifier after `#`",
                Span::new(start, self.pos as u32),
            ));
        }
        let (name, _) = self.scan_identifier_name()?;
        Ok(TokenKind::PrivateIdent(name))
    }

    fn ident_or_keyword(&mut self) -> Result<TokenKind, Diagnostic> {
        let (name, had_escape) = self.scan_identifier_name()?;
        if had_escape {
            return Ok(TokenKind::Ident(name));
        }
        Ok(match name.as_str() {
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "var" => TokenKind::Var,
            "typeof" => TokenKind::TypeOf,
            "void" => TokenKind::Void,
            "delete" => TokenKind::Delete,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "do" => TokenKind::Do,
            "for" => TokenKind::For,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "in" => TokenKind::In,
            "instanceof" => TokenKind::InstanceOf,
            "of" => TokenKind::Of,
            "function" => TokenKind::Function,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "yield" => TokenKind::Yield,
            "return" => TokenKind::Return,
            "this" => TokenKind::This,
            "new" => TokenKind::New,
            "class" => TokenKind::Class,
            "extends" => TokenKind::Extends,
            "super" => TokenKind::Super,
            "static" => TokenKind::Static,
            "throw" => TokenKind::Throw,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "with" => TokenKind::With,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "from" => TokenKind::From,
            "as" => TokenKind::As,
            _ => TokenKind::Ident(name),
        })
    }

    /// Scan IdentifierName (start + continues). Returns (decoded name, had Unicode escape).
    fn scan_identifier_name(&mut self) -> Result<(String, bool), Diagnostic> {
        let mut name = String::new();
        let mut had_escape = false;
        let (first, esc) = self.scan_ident_start_char()?;
        name.push(first);
        had_escape |= esc;
        while !self.is_eof() {
            if self.peek() == b'\\' {
                let (ch, _) = self.scan_ident_unicode_escape(false)?;
                name.push(ch);
                had_escape = true;
            } else {
                let ch = self.peek_char();
                if !is_ident_continue_char(ch) {
                    break;
                }
                self.bump_char();
                name.push(ch);
            }
        }
        Ok((name, had_escape))
    }

    fn can_start_ident(&self) -> bool {
        if self.is_eof() {
            return false;
        }
        if self.peek() == b'\\' {
            return self.peek_at(1) == Some(b'u');
        }
        is_ident_start_char(self.peek_char())
    }

    fn scan_ident_start_char(&mut self) -> Result<(char, bool), Diagnostic> {
        if self.is_eof() {
            return Err(Diagnostic::new(
                "expected identifier",
                Span::new(self.pos as u32, self.pos as u32),
            ));
        }
        if self.peek() == b'\\' {
            return self.scan_ident_unicode_escape(true);
        }
        let ch = self.peek_char();
        if !is_ident_start_char(ch) {
            return Err(Diagnostic::new(
                format!("unexpected character {:?}", ch),
                Span::new(self.pos as u32, self.pos as u32 + ch.len_utf8() as u32),
            ));
        }
        self.bump_char();
        Ok((ch, false))
    }

    /// `\UnicodeEscapeSequence` in IdentifierName. `start` selects ID_Start vs ID_Continue.
    fn scan_ident_unicode_escape(&mut self, start: bool) -> Result<(char, bool), Diagnostic> {
        let esc_start = self.pos as u32;
        if self.bump() != b'\\' {
            return Err(Diagnostic::new(
                "expected Unicode escape in identifier",
                Span::new(esc_start, self.pos as u32),
            ));
        }
        if self.is_eof() || self.peek() != b'u' {
            return Err(Diagnostic::new(
                "invalid Unicode escape in identifier",
                Span::new(esc_start, self.pos as u32),
            ));
        }
        self.bump(); // u
        let cp = if !self.is_eof() && self.peek() == b'{' {
            self.bump(); // {
            self.scan_braced_hex(esc_start)?
        } else {
            self.scan_hex_digits(4, esc_start)?
        };
        let ch = char::from_u32(cp).ok_or_else(|| {
            Diagnostic::new(
                "invalid Unicode escape in identifier",
                Span::new(esc_start, self.pos as u32),
            )
        })?;
        let ok = if start {
            is_ident_start_char(ch)
        } else {
            is_ident_continue_char(ch)
        };
        if !ok {
            return Err(Diagnostic::new(
                "invalid identifier escape",
                Span::new(esc_start, self.pos as u32),
            ));
        }
        Ok((ch, true))
    }

    /// `/ RegularExpressionBody / RegularExpressionFlags` (InputElementRegExp).
    fn scan_regexp_literal(&mut self, start: u32) -> Result<Token, Diagnostic> {
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
                if is_line_terminator_byte(self.peek()) {
                    return Err(Diagnostic::new(
                        "line terminator in regular expression literal",
                        Span::new(self.pos as u32, self.pos as u32 + 1),
                    ));
                }
                self.bump();
                continue;
            }
            if is_line_terminator_byte(b) {
                return Err(Diagnostic::new(
                    "line terminator in regular expression literal",
                    Span::new(self.pos as u32, self.pos as u32 + 1),
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
            self.bump();
        }
        let pattern = self.src[pattern_start..self.pos].to_string();
        self.bump(); // closing `/`
        let flags_start = self.pos;
        while !self.is_eof() && is_ident_continue_char(self.peek_char()) {
            self.bump_char();
        }
        let flags = self.src[flags_start..self.pos].to_string();
        self.at_line_start = false;
        Ok(self.finish_token(
            TokenKind::RegExp { pattern, flags },
            Span::new(start, self.pos as u32),
        ))
    }

    fn peek(&self) -> u8 {
        self.bytes[self.pos]
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn peek_char(&self) -> char {
        self.src[self.pos..].chars().next().expect("eof checked")
    }

    fn bump(&mut self) -> u8 {
        let b = self.bytes[self.pos];
        self.pos += 1;
        b
    }

    fn bump_char(&mut self) -> char {
        let ch = self.peek_char();
        self.pos += ch.len_utf8();
        ch
    }

    fn eat(&mut self, b: u8) -> bool {
        if !self.is_eof() && self.peek() == b {
            self.bump();
            true
        } else {
            false
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    /// Whether the UTF-8 scalar at `pos` is IdentifierPart (char-boundary `pos`).
    fn is_ident_continue_at(&self, pos: usize) -> bool {
        if pos >= self.bytes.len() {
            return false;
        }
        let ch = self.src[pos..].chars().next().expect("pos in bounds");
        is_ident_continue_char(ch)
    }
}

/// After `kind`, may the next `/` start a regexp literal (vs division)?
fn regexp_allowed_after(kind: &TokenKind) -> bool {
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

fn is_line_terminator_byte(b: u8) -> bool {
    b == b'\n' || b == b'\r'
}

fn is_line_terminator_char(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// ECMA-262 WhiteSpace beyond single-byte TAB/VT/FF/SP (handled inline).
fn is_whitespace_char(ch: char) -> bool {
    matches!(ch, '\u{00A0}' | '\u{FEFF}') || unicode_general_category_space_separator(ch)
}

fn unicode_general_category_space_separator(ch: char) -> bool {
    // Zs: Space_Separator (USP in ECMA-262).
    matches!(
        ch,
        '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

/// IdentifierStart: Unicode ID_Start | `$` | `_`
fn is_ident_start_char(ch: char) -> bool {
    ch == '$' || ch == '_' || unicode_id_start::is_id_start(ch)
}

/// IdentifierPart: Unicode ID_Continue | `$` | ZWNJ | ZWJ
fn is_ident_continue_char(ch: char) -> bool {
    ch == '$'
        || ch == '\u{200C}'
        || ch == '\u{200D}'
        || unicode_id_start::is_id_continue(ch)
}

fn hex_digit(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a') as u32 + 10),
        b'A'..=b'F' => Some((b - b'A') as u32 + 10),
        _ => None,
    }
}

/// Annex B.1.1 LegacyOctalIntegerLiteral mathematical value as a decimal digit string.
fn legacy_octal_mv(raw: &str) -> String {
    let mut val: u64 = 0;
    let mut overflow = false;
    for b in raw.bytes() {
        debug_assert!((b'0'..=b'7').contains(&b));
        match val
            .checked_mul(8)
            .and_then(|v| v.checked_add(u64::from(b - b'0')))
        {
            Some(next) => val = next,
            None => {
                overflow = true;
                break;
            }
        }
    }
    if !overflow {
        return val.to_string();
    }
    // Past u64: f64 MV (JS Number semantics for large integers).
    let mut f = 0.0f64;
    for b in raw.bytes() {
        f = f * 8.0 + f64::from(b - b'0');
    }
    if f.is_finite() && f.fract() == 0.0 && f.abs() <= (1u64 << 53) as f64 {
        format!("{}", f as u64)
    } else {
        format!("{f}")
    }
}

/// Strip redundant leading zeros from a NonOctalDecimalIntegerLiteral (and optional frac/exp).
fn canonicalize_leading_zero_decimal(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() && bytes[i] == b'0' && bytes[i + 1].is_ascii_digit() {
        i += 1;
    }
    raw[i..].to_string()
}

fn push_code_point(
    value: &mut JsString,
    cp: u32,
    start: u32,
    end: u32,
    scalar_only: bool,
) -> Result<(), Diagnostic> {
    if scalar_only {
        match char::from_u32(cp) {
            Some(c) => {
                value.push_scalar(c);
                Ok(())
            }
            None => Err(Diagnostic::new(
                "invalid Unicode escape sequence",
                Span::new(start, end),
            )),
        }
    } else if value.push_code_point_unit(cp).is_ok() {
        Ok(())
    } else {
        Err(Diagnostic::new(
            "invalid Unicode escape sequence",
            Span::new(start, end),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::new(src)
            .tokenize()
            .expect("lex")
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn lex_whitespace_vt_ff_nbsp_between_tokens() {
        assert_eq!(
            kinds("var\u{0b}x\u{0b}=\u{0b}1\u{0b};"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("var\u{0c}x=1;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("var\u{00a0}x\u{00a0}=\u{00a0}2;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Number("2".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("var\u{feff}x=1;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("var\u{2003}x=1;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_line_separator_as_line_terminator() {
        assert_eq!(
            kinds("var x=1\u{2028}var y=2"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Var,
                TokenKind::Ident("y".into()),
                TokenKind::Eq,
                TokenKind::Number("2".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_unicode_identifiers() {
        assert_eq!(
            kinds("var а = 1;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("а".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        // Other_ID_Start U+2118
        assert_eq!(
            kinds("var ℘ = 1;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("℘".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        // Other_ID_Continue U+00B7
        assert_eq!(
            kinds("var a· = 1;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("a·".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        // ZWNJ in IdentifierPart
        assert_eq!(
            kinds("var a\u{200c}b = 1;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("a\u{200c}b".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        // Vertical tilde U+2E2F is not ID_Start
        assert!(Lexer::new("var ⸯ;").tokenize().is_err());
    }

    #[test]
    fn lex_identifier_unicode_escapes() {
        assert_eq!(
            kinds(r"var \u0078 = 1;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds(r"var \u{61}bc = 1;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("abc".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        // Escaped keyword is Ident, not keyword token
        assert_eq!(
            kinds(r"\u0062reak"),
            vec![TokenKind::Ident("break".into()), TokenKind::Eof,]
        );
        assert_eq!(
            kinds(r"var x\u0061 = 1;"),
            vec![
                TokenKind::Var,
                TokenKind::Ident("xa".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_private_ident_unicode() {
        assert_eq!(
            kinds("this.#а"),
            vec![
                TokenKind::This,
                TokenKind::Dot,
                TokenKind::PrivateIdent("а".into()),
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds(r"this.#\u0078"),
            vec![
                TokenKind::This,
                TokenKind::Dot,
                TokenKind::PrivateIdent("x".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_let_assignment() {
        assert_eq!(
            kinds("let x = 1;"),
            vec![
                TokenKind::Let,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Number("1".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
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
    fn lex_skips_comments() {
        assert_eq!(
            kinds("1 // comment\n+ /* block */ 2"),
            vec![
                TokenKind::Number("1".into()),
                TokenKind::Plus,
                TokenKind::Number("2".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_html_open_comment() {
        assert_eq!(
            kinds("1 <!-- ignored\n+ 2"),
            vec![
                TokenKind::Number("1".into()),
                TokenKind::Plus,
                TokenKind::Number("2".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_html_close_comment_at_line_start() {
        assert_eq!(
            kinds("1\n--> ignored\n+ 2"),
            vec![
                TokenKind::Number("1".into()),
                TokenKind::Plus,
                TokenKind::Number("2".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_html_close_comment_after_whitespace() {
        assert_eq!(
            kinds("1\n  --> ignored\n+ 2"),
            vec![
                TokenKind::Number("1".into()),
                TokenKind::Plus,
                TokenKind::Number("2".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_html_close_comment_at_bof() {
        assert_eq!(
            kinds("--> ignored\n1"),
            vec![TokenKind::Number("1".into()), TokenKind::Eof,]
        );
    }

    #[test]
    fn lex_html_close_not_mid_line() {
        // `f-->0` is postfix decrement then greater-than, not an HTML close comment.
        assert_eq!(
            kinds("f-->0"),
            vec![
                TokenKind::Ident("f".into()),
                TokenKind::MinusMinus,
                TokenKind::Gt,
                TokenKind::Number("0".into()),
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
    fn lex_float() {
        assert_eq!(
            kinds("3.14"),
            vec![TokenKind::Number("3.14".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn lex_number_scientific() {
        assert_eq!(
            kinds("1e3 1.5E+2 2e-1"),
            vec![
                TokenKind::Number("1e3".into()),
                TokenKind::Number("1.5E+2".into()),
                TokenKind::Number("2e-1".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_number_radix() {
        assert_eq!(
            kinds("0xff 0b1010 0o17 0XFF 0B10 0O7"),
            vec![
                TokenKind::Number("0xff".into()),
                TokenKind::Number("0b1010".into()),
                TokenKind::Number("0o17".into()),
                TokenKind::Number("0XFF".into()),
                TokenKind::Number("0B10".into()),
                TokenKind::Number("0O7".into()),
                TokenKind::Eof,
            ]
        );
    }

    /// Annex B.1.1: legacy octal MV rewritten to decimal; NonOctalDecimal stays decimal.
    #[test]
    fn lex_legacy_octal_numeric_literals() {
        assert_eq!(
            kinds("010 077 00 0010 0123"),
            vec![
                TokenKind::Number("8".into()),
                TokenKind::Number("63".into()),
                TokenKind::Number("0".into()),
                TokenKind::Number("8".into()),
                TokenKind::Number("83".into()),
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("08 09 089 0008 08.5 08e2"),
            vec![
                TokenKind::Number("8".into()),
                TokenKind::Number("9".into()),
                TokenKind::Number("89".into()),
                TokenKind::Number("8".into()),
                TokenKind::Number("8.5".into()),
                TokenKind::Number("8e2".into()),
                TokenKind::Eof,
            ]
        );
        // Pure legacy octal does not swallow `.digit` (next token is leading-dot number).
        assert_eq!(
            kinds("010.5"),
            vec![
                TokenKind::Number("8".into()),
                TokenKind::Number(".5".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_number_leading_dot() {
        assert_eq!(
            kinds(".5 .25e1"),
            vec![
                TokenKind::Number(".5".into()),
                TokenKind::Number(".25e1".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_number_separators() {
        assert_eq!(
            kinds("1_000 0xFF_FF 0b1010_0001 1_000.5_00 1e1_0"),
            vec![
                TokenKind::Number("1_000".into()),
                TokenKind::Number("0xFF_FF".into()),
                TokenKind::Number("0b1010_0001".into()),
                TokenKind::Number("1_000.5_00".into()),
                TokenKind::Number("1e1_0".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_bigint_literals() {
        assert_eq!(
            kinds("1n 0n 0xffn 0b1010n 0o17n 1_000n 0xFF_FFn"),
            vec![
                TokenKind::BigInt("1n".into()),
                TokenKind::BigInt("0n".into()),
                TokenKind::BigInt("0xffn".into()),
                TokenKind::BigInt("0b1010n".into()),
                TokenKind::BigInt("0o17n".into()),
                TokenKind::BigInt("1_000n".into()),
                TokenKind::BigInt("0xFF_FFn".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_bigint_rejects_float_suffix() {
        let err = Lexer::new("1.0n").tokenize().unwrap_err();
        assert!(
            err.message.contains("Invalid BigInt"),
            "unexpected: {}",
            err.message
        );
        let err = Lexer::new("1e2n").tokenize().unwrap_err();
        assert!(
            err.message.contains("Invalid BigInt"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn lex_number_dot_still_member() {
        // Identifier after `.` is member access (not a fraction / exponent).
        assert_eq!(
            kinds("1.toString"),
            vec![
                TokenKind::Number("1".into()),
                TokenKind::Dot,
                TokenKind::Ident("toString".into()),
                TokenKind::Eof,
            ]
        );
        // `10.e1` is a DecimalLiteral with empty fraction + exponent.
        assert_eq!(
            kinds("10.e1"),
            vec![TokenKind::Number("10.e1".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn lex_number_rejects_separator_after_dot() {
        // `10._` / `10._e1` / `10._1`: `_` cannot start DecimalDigits after `.`.
        for src in ["10._", "10._e1", "10._1"] {
            let err = Lexer::new(src).tokenize().unwrap_err();
            assert!(
                err.message.contains("invalid number")
                    || err.message.contains("numeric separator"),
                "src={src:?} unexpected: {}",
                err.message
            );
        }
    }

    #[test]
    fn lex_string_single_escape_bfnv() {
        assert_eq!(
            kinds(r#""\b\f\v\n\r\t""#),
            vec![
                TokenKind::String("\u{0008}\u{000C}\u{000B}\n\r\t".into()),
                TokenKind::Eof
            ]
        );
        assert_eq!(
            kinds(r#"'\b\f\v'"#),
            vec![
                TokenKind::String("\u{0008}\u{000C}\u{000B}".into()),
                TokenKind::Eof
            ]
        );
        assert_eq!(
            kinds(r#"`\b\t`"#),
            vec![
                TokenKind::TemplateNoSubstitution("\u{0008}\t".into()),
                TokenKind::Eof
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

    #[test]
    fn lex_template_no_substitution() {
        assert_eq!(
            kinds("`hello`"),
            vec![
                TokenKind::TemplateNoSubstitution("hello".into()),
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds(r#"`a\`b\n`"#),
            vec![
                TokenKind::TemplateNoSubstitution("a`b\n".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_template_with_interpolation() {
        assert_eq!(
            kinds("`a${x}b${y}c`"),
            vec![
                TokenKind::TemplateHead("a".into()),
                TokenKind::Ident("x".into()),
                TokenKind::TemplateMiddle("b".into()),
                TokenKind::Ident("y".into()),
                TokenKind::TemplateTail("c".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_template_nested_and_braces() {
        assert_eq!(
            kinds("`o${{a:1}.a}z`"),
            vec![
                TokenKind::TemplateHead("o".into()),
                TokenKind::LBrace,
                TokenKind::Ident("a".into()),
                TokenKind::Colon,
                TokenKind::Number("1".into()),
                TokenKind::RBrace,
                TokenKind::Dot,
                TokenKind::Ident("a".into()),
                TokenKind::TemplateTail("z".into()),
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("`a${`b${c}`}d`"),
            vec![
                TokenKind::TemplateHead("a".into()),
                TokenKind::TemplateHead("b".into()),
                TokenKind::Ident("c".into()),
                TokenKind::TemplateTail("".into()),
                TokenKind::TemplateTail("d".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_string_identity_escape_multibyte_utf8() {
        // NonEscapeSequence / IdentityEscape: `\` + multi-byte UTF-8 scalar (Cyrillic А).
        assert_eq!(
            kinds("\"\\А\""),
            vec![TokenKind::String("А".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds("\"\\А\\Б\""),
            vec![TokenKind::String("АБ".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds("'\\а'"),
            vec![TokenKind::String("а".into()), TokenKind::Eof]
        );
        // ASCII NonEscapeSequence still works.
        assert_eq!(
            kinds(r#""\a\q""#),
            vec![TokenKind::String("aq".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn lex_string_line_continuation() {
        assert_eq!(
            kinds("\"\\\n\""),
            vec![TokenKind::String("".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds("\"\\\r\""),
            vec![TokenKind::String("".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds("\"\\\r\n\""),
            vec![TokenKind::String("".into()), TokenKind::Eof]
        );
        // U+2028 LINE SEPARATOR / U+2029 PARAGRAPH SEPARATOR
        assert_eq!(
            kinds("\"\\\u{2028}\""),
            vec![TokenKind::String("".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds("\"\\\u{2029}\""),
            vec![TokenKind::String("".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds("'a\\\nb'"),
            vec![TokenKind::String("ab".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds("`a\\\nb`"),
            vec![
                TokenKind::TemplateNoSubstitution("ab".into()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lex_string_hex_and_unicode_escapes() {
        assert_eq!(
            kinds(r#""\x41\x42""#),
            vec![TokenKind::String("AB".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\u0041""#),
            vec![TokenKind::String("A".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\u{1F600}""#),
            vec![TokenKind::String("😀".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#"'A\u0042C'"#),
            vec![TokenKind::String("ABC".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\x00""#),
            vec![TokenKind::String("\0".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn lex_string_legacy_octal_escapes() {
        assert_eq!(
            kinds(r#""\101""#),
            vec![TokenKind::String("A".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\12""#),
            vec![TokenKind::String("\n".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\377""#),
            vec![TokenKind::String("\u{00FF}".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\0""#),
            vec![TokenKind::String("\0".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\01""#),
            vec![TokenKind::String("\u{0001}".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\8""#),
            vec![TokenKind::String("8".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\9""#),
            vec![TokenKind::String("9".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""\400""#),
            vec![TokenKind::String(" 0".into()), TokenKind::Eof]
        );
        let i = match &kinds(r#""\08""#)[..] {
            [TokenKind::String(s), TokenKind::Eof] => s.clone(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(i.units(), &[0, 56]);
        assert_eq!(
            kinds(r#""\777""#),
            vec![TokenKind::String("?7".into()), TokenKind::Eof]
        );
        let k = match &kinds(r#""\38""#)[..] {
            [TokenKind::String(s), TokenKind::Eof] => s.clone(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(k.units(), &[3, 56]);
        assert_eq!(
            kinds(r#"'x\101y'"#),
            vec![TokenKind::String("xAy".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn lex_string_lone_surrogates_and_pairs() {
        let hi = match &kinds(r#""\uD800""#)[..] {
            [TokenKind::String(s), TokenKind::Eof] => s.clone(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(hi.units(), &[0xD800]);

        let pair = match &kinds(r#""\uD83D\uDE00""#)[..] {
            [TokenKind::String(s), TokenKind::Eof] => s.clone(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(pair.units(), &[0xD83D, 0xDE00]);
        assert_eq!(pair, JsString::from("😀"));

        let braced = match &kinds(r#""\u{1F600}""#)[..] {
            [TokenKind::String(s), TokenKind::Eof] => s.clone(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(braced.units(), &[0xD83D, 0xDE00]);
    }

    #[test]
    fn lex_template_hex_and_unicode_escapes() {
        assert_eq!(
            kinds(r#"`\x48i`"#),
            vec![
                TokenKind::TemplateNoSubstitution("Hi".into()),
                TokenKind::Eof
            ]
        );
        assert_eq!(
            kinds(r#"`\u004F\u004B`"#),
            vec![
                TokenKind::TemplateNoSubstitution("OK".into()),
                TokenKind::Eof
            ]
        );
        assert_eq!(
            kinds(r#"`a\u{41}${x}\u{42}`"#),
            vec![
                TokenKind::TemplateHead("aA".into()),
                TokenKind::Ident("x".into()),
                TokenKind::TemplateTail("B".into()),
                TokenKind::Eof,
            ]
        );
    }
}
