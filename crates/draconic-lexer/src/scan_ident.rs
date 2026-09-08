use crate::lexer::Lexer;
use crate::token::TokenKind;
use draconic_diagnostics::{Diagnostic, Span};

impl Lexer<'_> {
    /// `#IdentifierName` private identifier.
    pub(crate) fn private_ident(&mut self, start: u32) -> Result<TokenKind, Diagnostic> {
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

    pub(crate) fn ident_or_keyword(&mut self) -> Result<(TokenKind, bool), Diagnostic> {
        let (name, had_escape) = self.scan_identifier_name()?;
        if had_escape {
            return Ok((TokenKind::Ident(name), true));
        }
        let kind = match name.as_str() {
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
        };
        Ok((kind, false))
    }

    /// Scan IdentifierName (start + continues). Returns (decoded name, had Unicode escape).
    pub(crate) fn scan_identifier_name(&mut self) -> Result<(String, bool), Diagnostic> {
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

    pub(crate) fn can_start_ident(&self) -> bool {
        if self.is_eof() {
            return false;
        }
        if self.peek() == b'\\' {
            return self.peek_at(1) == Some(b'u');
        }
        is_ident_start_char(self.peek_char())
    }

    pub(crate) fn scan_ident_start_char(&mut self) -> Result<(char, bool), Diagnostic> {
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
    pub(crate) fn scan_ident_unicode_escape(&mut self, start: bool) -> Result<(char, bool), Diagnostic> {
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
}

/// IdentifierStart: Unicode ID_Start | `$` | `_`
pub(crate) fn is_ident_start_char(ch: char) -> bool {
    ch == '$' || ch == '_' || unicode_id_start::is_id_start(ch)
}

/// IdentifierPart: Unicode ID_Continue | `$` | ZWNJ | ZWJ
pub(crate) fn is_ident_continue_char(ch: char) -> bool {
    ch == '$' || ch == '\u{200C}' || ch == '\u{200D}' || unicode_id_start::is_id_continue(ch)
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
}
