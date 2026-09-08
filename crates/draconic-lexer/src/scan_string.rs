use crate::js_string::JsString;
use crate::lexer::{hex_digit, Lexer};
use crate::token::{Token, TokenKind};
use draconic_diagnostics::{Diagnostic, Span};

enum TemplateScanEnd {
    Tick,
    DollarBrace,
}

impl Lexer<'_> {
    pub(crate) fn string_literal(&mut self) -> Result<TokenKind, Diagnostic> {
        let quote = self.bump();
        let start = self.pos as u32;
        let mut value = JsString::new();
        let mut legacy = false;
        while !self.is_eof() {
            let c = self.peek();
            if c == quote {
                self.bump();
                if legacy {
                    self.pending_legacy_octal = true;
                }
                return Ok(TokenKind::String(value));
            }
            if c == b'\\' {
                self.bump();
                if self.scan_escape_into(&mut value, true)? {
                    legacy = true;
                }
            } else if c == b'\n' || c == b'\r' {
                // E19.69: LF/CR terminate strings (unterminated); LS/PS allowed since ES2019.
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
    pub(crate) fn template_literal(&mut self) -> Result<TokenKind, Diagnostic> {
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
    pub(crate) fn template_continuation(&mut self, start: u32) -> Result<Token, Diagnostic> {
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
    pub(crate) fn scan_source_char_into(&mut self, value: &mut JsString) {
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
    /// Returns `true` when a legacy octal / NonOctalDecimal escape was used (E19.69).
    pub(crate) fn scan_escape_into(
        &mut self,
        value: &mut JsString,
        allow_legacy_octal: bool,
    ) -> Result<bool, Diagnostic> {
        if self.is_eof() {
            return Err(Diagnostic::new(
                "unterminated escape sequence",
                Span::new(self.pos.saturating_sub(1) as u32, self.pos as u32),
            ));
        }
        // LineContinuation :: `\` LineTerminatorSequence → empty SV.
        if self.try_consume_line_terminator_sequence() {
            return Ok(false);
        }
        let esc_start = self.pos as u32;
        let esc = self.peek();
        match esc {
            b'b' => {
                self.bump();
                value.push_scalar('\u{0008}');
                Ok(false)
            }
            b'f' => {
                self.bump();
                value.push_scalar('\u{000C}');
                Ok(false)
            }
            b'n' => {
                self.bump();
                value.push_scalar('\n');
                Ok(false)
            }
            b'r' => {
                self.bump();
                value.push_scalar('\r');
                Ok(false)
            }
            b't' => {
                self.bump();
                value.push_scalar('\t');
                Ok(false)
            }
            b'v' => {
                self.bump();
                value.push_scalar('\u{000B}');
                Ok(false)
            }
            b'\\' => {
                self.bump();
                value.push_scalar('\\');
                Ok(false)
            }
            b'\'' => {
                self.bump();
                value.push_scalar('\'');
                Ok(false)
            }
            b'"' => {
                self.bump();
                value.push_scalar('"');
                Ok(false)
            }
            b'`' => {
                self.bump();
                value.push_scalar('`');
                Ok(false)
            }
            b'$' => {
                self.bump();
                value.push_scalar('$');
                Ok(false)
            }
            // TemplateEscapeSequence: `0` only when not followed by DecimalDigit.
            // Legacy octal / NonOctalDecimal are SyntaxError in templates (always).
            b'0' if !allow_legacy_octal => {
                self.bump();
                if !self.is_eof() && self.peek().is_ascii_digit() {
                    return Err(Diagnostic::new(
                        "octal escape sequences are not allowed in template literals",
                        Span::new(esc_start.saturating_sub(1), self.pos as u32 + 1),
                    ));
                }
                value.push_scalar('\0');
                Ok(false)
            }
            b'1'..=b'9' if !allow_legacy_octal => Err(Diagnostic::new(
                "octal escape sequences are not allowed in template literals",
                Span::new(esc_start.saturating_sub(1), self.pos as u32 + 1),
            )),
            b'0'..=b'7' if allow_legacy_octal => {
                let first = self.bump();
                // Standard EscapeSequence `0` requires lookahead ∉ DecimalDigit.
                // `\0`+digit / `\1`–`\7` are LegacyOctalEscapeSequence (E19.69 strict error).
                let is_legacy = first != b'0' || (!self.is_eof() && self.peek().is_ascii_digit());
                self.scan_legacy_octal_escape_into(value, first);
                Ok(is_legacy)
            }
            b'8' | b'9' if allow_legacy_octal => {
                // Annex B NonOctalDecimalEscapeSequence.
                self.scan_source_char_into(value);
                Ok(true)
            }
            b'0' => {
                self.bump();
                value.push_scalar('\0');
                Ok(false)
            }
            b'x' => {
                self.bump();
                let cp = self.scan_hex_digits(2, esc_start)?;
                push_code_point(value, cp, esc_start, self.pos as u32, false)?;
                Ok(false)
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
                Ok(false)
            }
            // IdentityEscape / NonEscapeSequence — full UTF-8 scalar (e.g. Cyrillic `"\А"`).
            _ => {
                self.scan_source_char_into(value);
                Ok(false)
            }
        }
    }

    /// Consume one LineTerminatorSequence if present at `pos`. Returns true when consumed.
    pub(crate) fn try_consume_line_terminator_sequence(&mut self) -> bool {
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
    pub(crate) fn scan_legacy_octal_escape_into(&mut self, value: &mut JsString, first: u8) {
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

    pub(crate) fn peek_octal_digit(&self) -> Option<u16> {
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

    pub(crate) fn scan_hex_digits(&mut self, n: usize, esc_start: u32) -> Result<u32, Diagnostic> {
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

    pub(crate) fn scan_braced_hex(&mut self, esc_start: u32) -> Result<u32, Diagnostic> {
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
    use crate::{JsString, Lexer, TokenKind};

    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::new(src)
            .tokenize()
            .expect("lex")
            .into_iter()
            .map(|t| t.kind)
            .collect()
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

    /// E19.69: legacy string escapes flagged; bare `\\0` is not legacy; templates reject octal.
    #[test]
    fn lex_legacy_escape_flags_and_template_reject() {
        let s = Lexer::new(r"'\1' '\0' '\8'").tokenize().unwrap();
        assert!(s[0].legacy_octal, r"\1");
        assert!(!s[1].legacy_octal, r"\0");
        assert!(s[2].legacy_octal, r"\8");
        assert!(Lexer::new(r"`\00`").tokenize().is_err());
        assert!(Lexer::new(r"`\1`").tokenize().is_err());
        assert!(Lexer::new("'\r'").tokenize().is_err());
        assert!(Lexer::new("/\u{2028}/").tokenize().is_err());
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
