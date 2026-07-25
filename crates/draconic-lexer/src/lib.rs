use draconic_diagnostics::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
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
    Colon,
    Question,
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
    Number(String),
    String(String),
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
    Of,
    Function,
    Return,
    // other
    Eof,
}

pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
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
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span::new(start, start),
            });
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
                TokenKind::LBrace
            }
            b'}' => {
                self.bump();
                TokenKind::RBrace
            }
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
                self.bump();
                TokenKind::Dot
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
                // line/block comments handled in skip_trivia; here it's divide
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
            b'"' | b'\'' => self.string_literal()?,
            b if b.is_ascii_digit() => self.number_literal()?,
            b if is_ident_start(b) => self.ident_or_keyword()?,
            _ => {
                return Err(Diagnostic::new(
                    format!("unexpected character {:?}", b as char),
                    Span::new(start, start + 1),
                ));
            }
        };

        Ok(Token {
            kind,
            span: Span::new(start, self.pos as u32),
        })
    }

    fn skip_trivia(&mut self) -> Result<(), Diagnostic> {
        loop {
            if self.is_eof() {
                return Ok(());
            }
            match self.peek() {
                b' ' | b'\t' | b'\r' | b'\n' => {
                    self.bump();
                }
                b'/' if self.peek_at(1) == Some(b'/') => {
                    self.bump();
                    self.bump();
                    while !self.is_eof() && self.peek() != b'\n' {
                        self.bump();
                    }
                }
                b'/' if self.peek_at(1) == Some(b'*') => {
                    let start = self.pos as u32;
                    self.bump();
                    self.bump();
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
                        self.bump();
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn string_literal(&mut self) -> Result<TokenKind, Diagnostic> {
        let quote = self.bump();
        let start = self.pos as u32;
        let mut value = String::new();
        while !self.is_eof() {
            let c = self.bump();
            if c == quote {
                return Ok(TokenKind::String(value));
            }
            if c == b'\\' {
                if self.is_eof() {
                    break;
                }
                let esc = self.bump();
                match esc {
                    b'n' => value.push('\n'),
                    b'r' => value.push('\r'),
                    b't' => value.push('\t'),
                    b'\\' => value.push('\\'),
                    b'\'' => value.push('\''),
                    b'"' => value.push('"'),
                    b'0' => value.push('\0'),
                    other => value.push(other as char),
                }
            } else if c == b'\n' {
                return Err(Diagnostic::new(
                    "unterminated string literal",
                    Span::new(start.saturating_sub(1), self.pos as u32),
                ));
            } else {
                value.push(c as char);
            }
        }
        Err(Diagnostic::new(
            "unterminated string literal",
            Span::new(start.saturating_sub(1), self.pos as u32),
        ))
    }

    fn number_literal(&mut self) -> Result<TokenKind, Diagnostic> {
        let start = self.pos;
        while !self.is_eof() && self.peek().is_ascii_digit() {
            self.bump();
        }
        if !self.is_eof() && self.peek() == b'.' {
            let next = self.peek_at(1);
            if next.is_some_and(|b| b.is_ascii_digit()) {
                self.bump();
                while !self.is_eof() && self.peek().is_ascii_digit() {
                    self.bump();
                }
            }
        }
        let raw = self.src[start..self.pos].to_string();
        Ok(TokenKind::Number(raw))
    }

    fn ident_or_keyword(&mut self) -> Result<TokenKind, Diagnostic> {
        let start = self.pos;
        self.bump();
        while !self.is_eof() && is_ident_continue(self.peek()) {
            self.bump();
        }
        let name = &self.src[start..self.pos];
        Ok(match name {
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
            "of" => TokenKind::Of,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            _ => TokenKind::Ident(name.to_string()),
        })
    }

    fn peek(&self) -> u8 {
        self.bytes[self.pos]
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> u8 {
        let b = self.bytes[self.pos];
        self.pos += 1;
        b
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
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

fn is_ident_continue(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
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
