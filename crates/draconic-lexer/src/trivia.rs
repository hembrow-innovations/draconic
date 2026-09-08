use crate::lexer::{is_line_terminator_char, is_whitespace_char, Lexer};
use draconic_diagnostics::{Diagnostic, Span};

impl Lexer<'_> {
    pub(crate) fn skip_trivia(&mut self) -> Result<(), Diagnostic> {
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
                // Annex B.1.3 SingleLineHTMLOpenComment: `<!--` … (Script only; E19.67).
                b'<' if self.allow_html_comments
                    && self.peek_at(1) == Some(b'!')
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
                // Annex B.1.3 HTMLCloseComment at line start: `-->` … (Script only; E19.67).
                b'-' if self.allow_html_comments
                    && self.at_line_start
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
}
