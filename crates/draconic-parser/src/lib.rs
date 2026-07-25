use draconic_ast::{
    dump_program, BinaryOp, Expr, Ident, NumberLit, Program, Stmt, StringLit, UnaryOp,
};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_lexer::{Lexer, Token, TokenKind};

pub use draconic_ast::dump_program as dump_ast;

pub fn parse(source: &str) -> Result<Program, Diagnostic> {
    let tokens = Lexer::new(source).tokenize()?;
    Parser::new(tokens).parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let start = self.current_span().start.0;
        let mut body = Vec::new();
        while !self.check(&TokenKind::Eof) {
            body.push(self.parse_stmt()?);
        }
        let end = self.current_span().end.0;
        Ok(Program {
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        if self.check(&TokenKind::Semi) {
            let span = self.bump().span;
            return Ok(Stmt::Empty { span });
        }
        if self.check(&TokenKind::Let) {
            return self.parse_let();
        }
        // expression statement
        let expr = self.parse_expr()?;
        let expr_span = expr_span(&expr);
        let end = if self.check(&TokenKind::Semi) {
            self.bump().span.end.0
        } else if self.check(&TokenKind::Eof) {
            expr_span.end.0
        } else {
            // ASI: allow newline-terminated; for bootstrap require ; or eof/next stmt boundary
            expr_span.end.0
        };
        Ok(Stmt::Expression {
            expr,
            span: Span::new(expr_span.start.0, end),
        })
    }

    fn parse_let(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.bump().span.start.0; // let
        let name_tok = self.expect_ident()?;
        let name = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        let init = if self.check(&TokenKind::Eq) {
            self.bump();
            Some(self.parse_expr()?)
        } else {
            None
        };
        let end = if self.check(&TokenKind::Semi) {
            self.bump().span.end.0
        } else {
            init.as_ref()
                .map(expr_span)
                .map(|s| s.end.0)
                .unwrap_or(name.span.end.0)
        };
        Ok(Stmt::Let {
            name,
            init,
            span: Span::new(start, end),
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::OrOr) {
            self.bump();
            let right = self.parse_and()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_equality()?;
        while self.check(&TokenKind::AndAnd) {
            self.bump();
            let right = self.parse_equality()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_relational()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::EqEq => BinaryOp::EqEq,
                TokenKind::NotEq => BinaryOp::NotEq,
                TokenKind::EqEqEq => BinaryOp::EqEqEq,
                TokenKind::NotEqEq => BinaryOp::NotEqEq,
                _ => break,
            };
            self.bump();
            let right = self.parse_relational()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_term()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::LtEq => BinaryOp::LtEq,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::GtEq => BinaryOp::GtEq,
                _ => break,
            };
            self.bump();
            let right = self.parse_term()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_factor()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.parse_factor()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Rem,
                _ => break,
            };
            self.bump();
            let right = self.parse_unary()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        let op = match &self.current().kind {
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Minus => Some(UnaryOp::Minus),
            TokenKind::Bang => Some(UnaryOp::Not),
            TokenKind::TypeOf => Some(UnaryOp::TypeOf),
            TokenKind::Void => Some(UnaryOp::Void),
            TokenKind::Delete => Some(UnaryOp::Delete),
            _ => None,
        };
        if let Some(op) = op {
            let start = self.bump().span.start.0;
            let arg = self.parse_unary()?;
            let end = expr_span(&arg).end.0;
            return Ok(Expr::Unary {
                op,
                arg: Box::new(arg),
                span: Span::new(start, end),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_primary()?;
        while self.check(&TokenKind::LParen) {
            self.bump();
            let mut args = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    args.push(self.parse_expr()?);
                    if self.check(&TokenKind::Comma) {
                        self.bump();
                        continue;
                    }
                    break;
                }
            }
            let end = self.expect(&TokenKind::RParen)?.span.end.0;
            let start = expr_span(&expr).start.0;
            expr = Expr::Call {
                callee: Box::new(expr),
                args,
                span: Span::new(start, end),
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let tok = self.current().clone();
        match &tok.kind {
            TokenKind::Number(raw) => {
                self.bump();
                Ok(Expr::Number(NumberLit {
                    raw: raw.clone(),
                    span: tok.span,
                }))
            }
            TokenKind::String(value) => {
                self.bump();
                Ok(Expr::String(StringLit {
                    value: value.clone(),
                    span: tok.span,
                }))
            }
            TokenKind::True => {
                self.bump();
                Ok(Expr::Boolean {
                    value: true,
                    span: tok.span,
                })
            }
            TokenKind::False => {
                self.bump();
                Ok(Expr::Boolean {
                    value: false,
                    span: tok.span,
                })
            }
            TokenKind::Null => {
                self.bump();
                Ok(Expr::Null { span: tok.span })
            }
            TokenKind::Ident(name) => {
                self.bump();
                Ok(Expr::Ident(Ident {
                    name: name.clone(),
                    span: tok.span,
                }))
            }
            TokenKind::LParen => {
                let start = self.bump().span.start.0;
                let inner = self.parse_expr()?;
                let end = self.expect(&TokenKind::RParen)?.span.end.0;
                Ok(Expr::Paren {
                    expr: Box::new(inner),
                    span: Span::new(start, end),
                })
            }
            _ => Err(Diagnostic::new(
                format!("expected expression, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn current_span(&self) -> Span {
        self.current().span
    }

    fn check(&self, kind: &TokenKind) -> bool {
        &self.current().kind == kind
    }

    fn bump(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token, Diagnostic> {
        if self.check(kind) {
            Ok(self.bump())
        } else {
            Err(Diagnostic::new(
                format!("expected {:?}, found {:?}", kind, self.current().kind),
                self.current().span,
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<Token, Diagnostic> {
        let tok = self.current().clone();
        match &tok.kind {
            TokenKind::Ident(_) => {
                self.bump();
                Ok(tok)
            }
            _ => Err(Diagnostic::new(
                format!("expected identifier, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }
}

trait IdentName {
    fn ident_name(&self) -> String;
}

impl IdentName for Token {
    fn ident_name(&self) -> String {
        match &self.kind {
            TokenKind::Ident(n) => n.clone(),
            _ => unreachable!(),
        }
    }
}

fn expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Ident(i) => i.span,
        Expr::Number(n) => n.span,
        Expr::String(s) => s.span,
        Expr::Boolean { span, .. }
        | Expr::Null { span }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Call { span, .. }
        | Expr::Paren { span, .. } => *span,
    }
}

fn span_merge(a: Span, b: Span) -> Span {
    Span::new(a.start.0, b.end.0)
}

/// Helper for tests and CLI.
pub fn parse_and_dump(source: &str) -> Result<String, Diagnostic> {
    let program = parse(source)?;
    Ok(dump_program(&program))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_let_and_expr() {
        let dump = parse_and_dump("let x = 1 + 2 * 3;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary +
        Number 1
        Binary *
          Number 2
          Number 3
"
        );
    }

    #[test]
    fn parse_call() {
        let dump = parse_and_dump("foo(1, 2);").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ExpressionStatement
    Call
      callee:
        Ident foo
      arg[0]:
        Number 1
      arg[1]:
        Number 2
"
        );
    }

    #[test]
    fn parse_unary_and_bool() {
        let dump = parse_and_dump("let ok = !false;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: ok
    init:
      Unary !
        Boolean false
"
        );
    }

    #[test]
    fn parse_string_and_null() {
        let dump = parse_and_dump(r#"let s = "hi"; let n = null;"#).unwrap();
        assert!(dump.contains("String \"hi\""));
        assert!(dump.contains("Null"));
    }

    #[test]
    fn parse_comparison() {
        let dump = parse_and_dump("a === b && c !== d;").unwrap();
        assert!(dump.contains("Binary ==="));
        assert!(dump.contains("Binary &&"));
        assert!(dump.contains("Binary !=="));
    }
}
