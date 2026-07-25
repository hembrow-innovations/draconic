use draconic_ast::{
    dump_program, AssignOp, BinaryOp, BindingKind, Expr, Ident, NumberLit, Program, Stmt, StringLit,
    SwitchCase, UnaryOp, UpdateOp,
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
        if self.check(&TokenKind::LBrace) {
            return self.parse_block();
        }
        if self.check(&TokenKind::If) {
            return self.parse_if();
        }
        if self.check(&TokenKind::While) {
            return self.parse_while();
        }
        if self.check(&TokenKind::Do) {
            return self.parse_do_while();
        }
        if self.check(&TokenKind::For) {
            return self.parse_for();
        }
        if self.check(&TokenKind::Break) {
            return self.parse_break();
        }
        if self.check(&TokenKind::Continue) {
            return self.parse_continue();
        }
        if self.check(&TokenKind::Switch) {
            return self.parse_switch();
        }
        if self.check(&TokenKind::Function) {
            return self.parse_function_decl();
        }
        if self.check(&TokenKind::Return) {
            return self.parse_return();
        }
        if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
            return self.parse_lexical_decl();
        }
        // `label: statement`
        if matches!(self.current().kind, TokenKind::Ident(_)) && self.peek_is(&TokenKind::Colon) {
            return self.parse_labeled();
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

    fn parse_labeled(&mut self) -> Result<Stmt, Diagnostic> {
        let name_tok = self.expect_ident()?;
        let label = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        let start = name_tok.span.start.0;
        self.expect(&TokenKind::Colon)?;
        let body = Box::new(self.parse_stmt()?);
        let end = stmt_span(&body).end.0;
        Ok(Stmt::Labeled {
            label,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_block(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::LBrace)?.span.start.0;
        let mut body = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            body.push(self.parse_stmt()?);
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(Stmt::Block {
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::If)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let test = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        let consequent = Box::new(self.parse_stmt()?);
        let alternate = if self.check(&TokenKind::Else) {
            self.bump();
            Some(Box::new(self.parse_stmt()?))
        } else {
            None
        };
        let end = alternate
            .as_ref()
            .map(|s| stmt_span(s).end.0)
            .unwrap_or_else(|| stmt_span(&consequent).end.0);
        Ok(Stmt::If {
            test,
            consequent,
            alternate,
            span: Span::new(start, end),
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::While)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let test = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        let end = stmt_span(&body).end.0;
        Ok(Stmt::While {
            test,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_do_while(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Do)?.span.start.0;
        let body = Box::new(self.parse_stmt()?);
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LParen)?;
        let test = self.parse_expr()?;
        let end = self.expect(&TokenKind::RParen)?.span.end.0;
        let end = if self.check(&TokenKind::Semi) {
            self.bump().span.end.0
        } else {
            end
        };
        Ok(Stmt::DoWhile {
            body,
            test,
            span: Span::new(start, end),
        })
    }

    fn parse_for(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::For)?.span.start.0;
        self.expect(&TokenKind::LParen)?;

        // `for (let/const name in/of right)` — binding without initializer.
        if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
            let kind = if self.check(&TokenKind::Const) {
                BindingKind::Const
            } else {
                BindingKind::Let
            };
            let let_start = self.bump().span.start.0;
            let name_tok = self.expect_ident()?;
            let name_end = name_tok.span.end.0;
            let name = Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            };
            if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
                let is_in = self.check(&TokenKind::In);
                self.bump();
                let right = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                let end = stmt_span(&body).end.0;
                let left = Box::new(Stmt::Let {
                    kind,
                    name,
                    init: None,
                    span: Span::new(let_start, name_end),
                });
                return if is_in {
                    Ok(Stmt::ForIn {
                        left,
                        right,
                        body,
                        span: Span::new(start, end),
                    })
                } else {
                    Ok(Stmt::ForOf {
                        left,
                        right,
                        body,
                        span: Span::new(start, end),
                    })
                };
            }
            // Classic `for (let/const name = init; …)` / `for (let name; …)`.
            let init_expr = if self.check(&TokenKind::Eq) {
                self.bump();
                Some(self.parse_assignment()?)
            } else if kind == BindingKind::Const {
                return Err(Diagnostic::new(
                    "const declaration requires an initializer".to_string(),
                    name.span,
                ));
            } else {
                None
            };
            let let_end = if let Some(ref e) = init_expr {
                expr_span(e).end.0
            } else {
                name.span.end.0
            };
            self.expect(&TokenKind::Semi)?;
            let left_init = Some(Box::new(Stmt::Let {
                kind,
                name,
                init: init_expr,
                span: Span::new(let_start, let_end),
            }));
            return self.finish_classic_for(start, left_init);
        }

        if self.check(&TokenKind::Semi) {
            self.bump();
            return self.finish_classic_for(start, None);
        }

        // Expression left: `for (lhs in/of right)` or classic `for (expr; …)`.
        let expr = self.parse_expr()?;
        let expr_span = expr_span(&expr);
        if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
            let is_in = self.check(&TokenKind::In);
            self.bump();
            let right = self.parse_expr()?;
            self.expect(&TokenKind::RParen)?;
            let body = Box::new(self.parse_stmt()?);
            let end = stmt_span(&body).end.0;
            let left = Box::new(Stmt::Expression {
                expr,
                span: expr_span,
            });
            return if is_in {
                Ok(Stmt::ForIn {
                    left,
                    right,
                    body,
                    span: Span::new(start, end),
                })
            } else {
                Ok(Stmt::ForOf {
                    left,
                    right,
                    body,
                    span: Span::new(start, end),
                })
            };
        }

        self.expect(&TokenKind::Semi)?;
        let init = Some(Box::new(Stmt::Expression {
            expr,
            span: expr_span,
        }));
        self.finish_classic_for(start, init)
    }

    fn finish_classic_for(
        &mut self,
        start: u32,
        init: Option<Box<Stmt>>,
    ) -> Result<Stmt, Diagnostic> {
        let test = if self.check(&TokenKind::Semi) {
            self.bump();
            None
        } else {
            let expr = self.parse_expr()?;
            self.expect(&TokenKind::Semi)?;
            Some(expr)
        };
        let update = if self.check(&TokenKind::RParen) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        let end = stmt_span(&body).end.0;
        Ok(Stmt::For {
            init,
            test,
            update,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_break(&mut self) -> Result<Stmt, Diagnostic> {
        let tok = self.expect(&TokenKind::Break)?;
        let start = tok.span.start.0;
        let mut end = tok.span.end.0;
        let label = if matches!(self.current().kind, TokenKind::Ident(_)) {
            let name_tok = self.expect_ident()?;
            end = name_tok.span.end.0;
            Some(Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            })
        } else {
            None
        };
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::Break {
            label,
            span: Span::new(start, end),
        })
    }

    fn parse_continue(&mut self) -> Result<Stmt, Diagnostic> {
        let tok = self.expect(&TokenKind::Continue)?;
        let start = tok.span.start.0;
        let mut end = tok.span.end.0;
        let label = if matches!(self.current().kind, TokenKind::Ident(_)) {
            let name_tok = self.expect_ident()?;
            end = name_tok.span.end.0;
            Some(Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            })
        } else {
            None
        };
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::Continue {
            label,
            span: Span::new(start, end),
        })
    }

    fn parse_switch(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Switch)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let discriminant = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::LBrace)?;
        let mut cases = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            cases.push(self.parse_switch_case()?);
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(Stmt::Switch {
            discriminant,
            cases,
            span: Span::new(start, end),
        })
    }

    fn parse_function_decl(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Function)?.span.start.0;
        let name_tok = self.expect_ident()?;
        let name = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let p = self.expect_ident()?;
                params.push(Ident {
                    name: p.ident_name(),
                    span: p.span,
                });
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_block()?);
        let end = stmt_span(&body).end.0;
        Ok(Stmt::FunctionDeclaration {
            name,
            params,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, Diagnostic> {
        let tok = self.expect(&TokenKind::Return)?;
        let start = tok.span.start.0;
        let mut end = tok.span.end.0;
        let argument = if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
            None
        } else if self.check(&TokenKind::RBrace) || self.check(&TokenKind::Eof) {
            None
        } else {
            let expr = self.parse_expr()?;
            end = expr_span(&expr).end.0;
            if self.check(&TokenKind::Semi) {
                end = self.bump().span.end.0;
            }
            Some(expr)
        };
        Ok(Stmt::Return {
            argument,
            span: Span::new(start, end),
        })
    }

    fn parse_switch_case(&mut self) -> Result<SwitchCase, Diagnostic> {
        if self.check(&TokenKind::Case) {
            let start = self.bump().span.start.0;
            let test = self.parse_expr()?;
            let colon_end = self.expect(&TokenKind::Colon)?.span.end.0;
            let mut body = Vec::new();
            while !self.check(&TokenKind::Case)
                && !self.check(&TokenKind::Default)
                && !self.check(&TokenKind::RBrace)
                && !self.check(&TokenKind::Eof)
            {
                body.push(self.parse_stmt()?);
            }
            let end = body
                .last()
                .map(|s| stmt_span(s).end.0)
                .unwrap_or(colon_end);
            Ok(SwitchCase {
                test: Some(test),
                body,
                span: Span::new(start, end),
            })
        } else if self.check(&TokenKind::Default) {
            let start = self.bump().span.start.0;
            let colon_end = self.expect(&TokenKind::Colon)?.span.end.0;
            let mut body = Vec::new();
            while !self.check(&TokenKind::Case)
                && !self.check(&TokenKind::Default)
                && !self.check(&TokenKind::RBrace)
                && !self.check(&TokenKind::Eof)
            {
                body.push(self.parse_stmt()?);
            }
            let end = body
                .last()
                .map(|s| stmt_span(s).end.0)
                .unwrap_or(colon_end);
            Ok(SwitchCase {
                test: None,
                body,
                span: Span::new(start, end),
            })
        } else {
            Err(Diagnostic::new(
                format!(
                    "expected case or default, found {:?}",
                    self.current().kind
                ),
                self.current().span,
            ))
        }
    }

    fn parse_lexical_decl(&mut self) -> Result<Stmt, Diagnostic> {
        let kind_tok = self.bump();
        let kind = match kind_tok.kind {
            TokenKind::Const => BindingKind::Const,
            _ => BindingKind::Let,
        };
        let start = kind_tok.span.start.0;
        let name_tok = self.expect_ident()?;
        let name = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        let init = if self.check(&TokenKind::Eq) {
            self.bump();
            // Initializer is AssignmentExpression (not Expression), so `,` is not
            // consumed here — multi-declarator lexical binding is a later feature.
            Some(self.parse_assignment()?)
        } else if kind == BindingKind::Const {
            return Err(Diagnostic::new(
                "const declaration requires an initializer".to_string(),
                name.span,
            ));
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
            kind,
            name,
            init,
            span: Span::new(start, end),
        })
    }

    /// Expression: `AssignmentExpression` (`,` `AssignmentExpression`)* left-assoc.
    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_assignment()?;
        while self.check(&TokenKind::Comma) {
            self.bump();
            let right = self.parse_assignment()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Comma,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    /// Right-associative assignment: `target = value` or compound `op=`.
    fn parse_assignment(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.parse_conditional()?;
        let Some(op) = self.peek_assign_op() else {
            return Ok(left);
        };
        self.bump();
        let value = self.parse_assignment()?;
        let span = span_merge(expr_span(&left), expr_span(&value));
        Ok(Expr::Assign {
            target: Box::new(left),
            op,
            value: Box::new(value),
            span,
        })
    }

    fn peek_assign_op(&self) -> Option<AssignOp> {
        match self.current().kind {
            TokenKind::Eq => Some(AssignOp::Eq),
            TokenKind::PlusEq => Some(AssignOp::AddEq),
            TokenKind::MinusEq => Some(AssignOp::SubEq),
            TokenKind::StarEq => Some(AssignOp::MulEq),
            TokenKind::SlashEq => Some(AssignOp::DivEq),
            TokenKind::PercentEq => Some(AssignOp::RemEq),
            TokenKind::StarStarEq => Some(AssignOp::PowEq),
            TokenKind::ShlEq => Some(AssignOp::ShlEq),
            TokenKind::ShrEq => Some(AssignOp::ShrEq),
            TokenKind::UShrEq => Some(AssignOp::UShrEq),
            TokenKind::BitAndEq => Some(AssignOp::BitAndEq),
            TokenKind::BitOrEq => Some(AssignOp::BitOrEq),
            TokenKind::BitXorEq => Some(AssignOp::BitXorEq),
            TokenKind::AndAndEq => Some(AssignOp::AndAndEq),
            TokenKind::OrOrEq => Some(AssignOp::OrOrEq),
            TokenKind::QuestionQuestionEq => Some(AssignOp::NullishEq),
            _ => None,
        }
    }

    /// Conditional: `test ? AssignmentExpression : AssignmentExpression`.
    fn parse_conditional(&mut self) -> Result<Expr, Diagnostic> {
        let test = self.parse_nullish()?;
        if !self.check(&TokenKind::Question) {
            return Ok(test);
        }
        self.bump();
        let consequent = self.parse_assignment()?;
        self.expect(&TokenKind::Colon)?;
        let alternate = self.parse_assignment()?;
        let span = span_merge(expr_span(&test), expr_span(&alternate));
        Ok(Expr::Conditional {
            test: Box::new(test),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
            span,
        })
    }

    /// Nullish coalescing: left-assoc `??`. Cannot mix with `&&` / `||` without parens.
    fn parse_nullish(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_or()?;
        while self.check(&TokenKind::QuestionQuestion) {
            if is_logical_and_or(&left) {
                return Err(Diagnostic::new(
                    "cannot mix '??' with '&&' or '||' without parentheses".to_string(),
                    self.current().span,
                ));
            }
            self.bump();
            // RHS is BitwiseORExpression (not LogicalOR / LogicalAND).
            let right = self.parse_bit_or()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Nullish,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
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
        let mut left = self.parse_bit_or()?;
        while self.check(&TokenKind::AndAnd) {
            self.bump();
            let right = self.parse_bit_or()?;
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

    fn parse_bit_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_bit_xor()?;
        while self.check(&TokenKind::BitOr) {
            self.bump();
            let right = self.parse_bit_xor()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::BitOr,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_bit_xor(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_bit_and()?;
        while self.check(&TokenKind::BitXor) {
            self.bump();
            let right = self.parse_bit_and()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::BitXor,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_bit_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_equality()?;
        while self.check(&TokenKind::BitAnd) {
            self.bump();
            let right = self.parse_equality()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::BitAnd,
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
        let mut left = self.parse_shift()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::LtEq => BinaryOp::LtEq,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::GtEq => BinaryOp::GtEq,
                _ => break,
            };
            self.bump();
            let right = self.parse_shift()?;
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

    fn parse_shift(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_term()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Shl => BinaryOp::Shl,
                TokenKind::Shr => BinaryOp::Shr,
                TokenKind::UShr => BinaryOp::UShr,
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
        let mut left = self.parse_exponentiation()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Rem,
                _ => break,
            };
            self.bump();
            let right = self.parse_exponentiation()?;
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

    /// Right-associative `**` (ECMA-262 ExponentiationExpression).
    fn parse_exponentiation(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.parse_unary()?;
        if self.check(&TokenKind::StarStar) {
            self.bump();
            let right = self.parse_exponentiation()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            return Ok(Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Pow,
                right: Box::new(right),
                span,
            });
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        let update_op = match &self.current().kind {
            TokenKind::PlusPlus => Some(UpdateOp::Inc),
            TokenKind::MinusMinus => Some(UpdateOp::Dec),
            _ => None,
        };
        if let Some(op) = update_op {
            let start = self.bump().span.start.0;
            let arg = self.parse_unary()?;
            let end = expr_span(&arg).end.0;
            return Ok(Expr::Update {
                op,
                arg: Box::new(arg),
                prefix: true,
                span: Span::new(start, end),
            });
        }
        let op = match &self.current().kind {
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Minus => Some(UnaryOp::Minus),
            TokenKind::Bang => Some(UnaryOp::Not),
            TokenKind::Tilde => Some(UnaryOp::BitNot),
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
        self.parse_update()
    }

    /// Postfix update (`lhs++` / `lhs--`) and call.
    fn parse_update(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_lhs()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::PlusPlus => UpdateOp::Inc,
                TokenKind::MinusMinus => UpdateOp::Dec,
                _ => break,
            };
            let end = self.bump().span.end.0;
            let start = expr_span(&expr).start.0;
            expr = Expr::Update {
                op,
                arg: Box::new(expr),
                prefix: false,
                span: Span::new(start, end),
            };
        }
        Ok(expr)
    }

    fn parse_lhs(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_primary()?;
        while self.check(&TokenKind::LParen) {
            self.bump();
            let mut args = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    // ArgumentList elements are AssignmentExpression, not Expression.
                    args.push(self.parse_assignment()?);
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

    fn peek_is(&self, kind: &TokenKind) -> bool {
        self.tokens
            .get(self.pos + 1)
            .map(|t| &t.kind == kind)
            .unwrap_or(false)
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
        | Expr::Conditional { span, .. }
        | Expr::Assign { span, .. }
        | Expr::Update { span, .. }
        | Expr::Call { span, .. }
        | Expr::Paren { span, .. } => *span,
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Expression { span, .. }
        | Stmt::Let { span, .. }
        | Stmt::Empty { span }
        | Stmt::Block { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoWhile { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForIn { span, .. }
        | Stmt::ForOf { span, .. }
        | Stmt::Break { span, .. }
        | Stmt::Continue { span, .. }
        | Stmt::Labeled { span, .. }
        | Stmt::Switch { span, .. }
        | Stmt::FunctionDeclaration { span, .. }
        | Stmt::Return { span, .. } => *span,
    }
}

fn is_logical_and_or(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Binary {
            op: BinaryOp::And | BinaryOp::Or,
            ..
        }
    )
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
    fn parse_const_decl() {
        let dump = parse_and_dump("const x = 1 + 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Const
    name: x
    init:
      Binary +
        Number 1
        Number 2
"
        );
    }

    #[test]
    fn parse_const_requires_initializer() {
        let err = parse("const x;").unwrap_err();
        assert!(
            err.message.contains("const declaration requires an initializer"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn parse_for_const_of() {
        let dump = parse_and_dump(r#"for (const c of "ab") c;"#).unwrap();
        assert!(dump.contains("ForOf"), "got:\n{dump}");
        assert!(dump.contains("Const"), "got:\n{dump}");
    }

    #[test]
    fn parse_if_else_with_block() {
        let dump = parse_and_dump("if (true) { x = 1; } else { x = 2; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  If
    test:
      Boolean true
    consequent:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Number 1
    alternate:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Number 2
"
        );
    }

    #[test]
    fn parse_while_with_block() {
        let dump = parse_and_dump("while (x < 3) { x = x + 1; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  While
    test:
      Binary <
        Ident x
        Number 3
    body:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Binary +
              Ident x
              Number 1
"
        );
    }

    #[test]
    fn parse_do_while_with_block() {
        let dump = parse_and_dump("do { x = x + 1; } while (x < 3);").unwrap();
        assert_eq!(
            dump,
            "\
Program
  DoWhile
    body:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Binary +
              Ident x
              Number 1
    test:
      Binary <
        Ident x
        Number 3
"
        );
    }

    #[test]
    fn parse_for_with_let_init() {
        let dump = parse_and_dump("for (let i = 0; i < 3; i = i + 1) { x = x + 1; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  For
    init:
      Let
        name: i
        init:
          Number 0
    test:
      Binary <
        Ident i
        Number 3
    update:
      Assign =
        Ident i
        Binary +
          Ident i
          Number 1
    body:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Binary +
              Ident x
              Number 1
"
        );
    }

    #[test]
    fn parse_for_omitted_clauses() {
        let dump = parse_and_dump("for (;;) x = 1;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  For
    body:
      ExpressionStatement
        Assign =
          Ident x
          Number 1
"
        );
    }

    #[test]
    fn parse_for_in_let() {
        let dump = parse_and_dump("for (let k in s) { x = k; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForIn
    left:
      Let
        name: k
    right:
      Ident s
    body:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Ident k
"
        );
    }

    #[test]
    fn parse_for_of_let() {
        let dump = parse_and_dump("for (let c of s) x = c;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForOf
    left:
      Let
        name: c
    right:
      Ident s
    body:
      ExpressionStatement
        Assign =
          Ident x
          Ident c
"
        );
    }

    #[test]
    fn parse_for_of_assign_target() {
        let dump = parse_and_dump("for (x of s) {}").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForOf
    left:
      ExpressionStatement
        Ident x
    right:
      Ident s
    body:
      Block
"
        );
    }

    #[test]
    fn parse_break_continue() {
        let dump = parse_and_dump("while (true) { break; continue; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  While
    test:
      Boolean true
    body:
      Block
        Break
        Continue
"
        );
    }

    #[test]
    fn parse_labeled_break_continue() {
        let dump =
            parse_and_dump("outer: while (true) { break outer; continue outer; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Labeled outer
    While
      test:
        Boolean true
      body:
        Block
          Break outer
          Continue outer
"
        );
    }

    #[test]
    fn parse_switch() {
        let dump = parse_and_dump(
            "switch (x) { case 1: a = 1; break; case 2: a = 2; default: a = 0; }",
        )
        .unwrap();
        assert_eq!(
            dump,
            "\
Program
  Switch
    discriminant:
      Ident x
    Case
      test:
        Number 1
      ExpressionStatement
        Assign =
          Ident a
          Number 1
      Break
    Case
      test:
        Number 2
      ExpressionStatement
        Assign =
          Ident a
          Number 2
    Default
      ExpressionStatement
        Assign =
          Ident a
          Number 0
"
        );
    }

    #[test]
    fn parse_function_decl_return() {
        let dump = parse_and_dump("function f() { return 1; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  FunctionDeclaration
    name: f
    body:
      Block
        Return
          Number 1
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

    #[test]
    fn parse_bitwise_precedence() {
        let dump = parse_and_dump("let x = 1 | 2 & 4;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary |
        Number 1
        Binary &
          Number 2
          Number 4
"
        );
    }

    #[test]
    fn parse_bitwise_not_and_shift() {
        let dump = parse_and_dump("let x = ~1 << 2;").unwrap();
        assert!(dump.contains("Binary <<"));
        assert!(dump.contains("Unary ~"));
    }

    #[test]
    fn parse_exponentiation_right_assoc() {
        let dump = parse_and_dump("let x = 2 ** 3 ** 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary **
        Number 2
        Binary **
          Number 3
          Number 2
"
        );
    }

    #[test]
    fn parse_exponentiation_precedence() {
        let dump = parse_and_dump("let x = 2 * 3 ** 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary *
        Number 2
        Binary **
          Number 3
          Number 2
"
        );
    }

    #[test]
    fn parse_conditional() {
        let dump = parse_and_dump("let x = true ? 1 : 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Conditional
        Boolean true
        Number 1
        Number 2
"
        );
    }

    #[test]
    fn parse_conditional_right_assoc() {
        let dump = parse_and_dump("let x = a ? b : c ? d : e;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Conditional
        Ident a
        Ident b
        Conditional
          Ident c
          Ident d
          Ident e
"
        );
    }

    #[test]
    fn parse_conditional_nested_consequent() {
        let dump = parse_and_dump("let x = a ? b ? c : d : e;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Conditional
        Ident a
        Conditional
          Ident b
          Ident c
          Ident d
        Ident e
"
        );
    }

    #[test]
    fn parse_assignment() {
        let dump = parse_and_dump("let x; x = 1;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
  ExpressionStatement
    Assign =
      Ident x
      Number 1
"
        );
    }

    #[test]
    fn parse_assignment_right_assoc() {
        let dump = parse_and_dump("let a; let b; a = b = 1;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: a
  Let
    name: b
  ExpressionStatement
    Assign =
      Ident a
      Assign =
        Ident b
        Number 1
"
        );
    }

    #[test]
    fn parse_assignment_in_conditional_alternate() {
        let dump = parse_and_dump("let a; let x = true ? 1 : a = 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: a
  Let
    name: x
    init:
      Conditional
        Boolean true
        Number 1
        Assign =
          Ident a
          Number 2
"
        );
    }

    #[test]
    fn parse_nullish() {
        let dump = parse_and_dump("let x = null ?? 1;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary ??
        Null
        Number 1
"
        );
    }

    #[test]
    fn parse_nullish_left_assoc() {
        let dump = parse_and_dump("let x = a ?? b ?? c;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary ??
        Binary ??
          Ident a
          Ident b
        Ident c
"
        );
    }

    #[test]
    fn parse_nullish_rejects_mix_with_or() {
        let err = parse_and_dump("let x = a || b ?? c;").unwrap_err();
        assert!(
            err.message.contains("??") && err.message.contains("||"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn parse_nullish_allows_paren_mix() {
        let dump = parse_and_dump("let x = (a || b) ?? c;").unwrap();
        assert!(dump.contains("Binary ??"));
        assert!(dump.contains("Paren"));
        assert!(dump.contains("Binary ||"));
    }

    #[test]
    fn parse_logical_assignment() {
        let dump = parse_and_dump("let x = 1; x &&= 2; x ||= 3; x ??= 4;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Number 1
  ExpressionStatement
    Assign &&=
      Ident x
      Number 2
  ExpressionStatement
    Assign ||=
      Ident x
      Number 3
  ExpressionStatement
    Assign ??=
      Ident x
      Number 4
"
        );
    }

    #[test]
    fn parse_compound_assignment() {
        let dump = parse_and_dump("let x = 1; x += 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Number 1
  ExpressionStatement
    Assign +=
      Ident x
      Number 2
"
        );
    }

    #[test]
    fn parse_compound_assignment_right_assoc() {
        let dump = parse_and_dump("let a = 1; let b = 2; a += b += 3;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: a
    init:
      Number 1
  Let
    name: b
    init:
      Number 2
  ExpressionStatement
    Assign +=
      Ident a
      Assign +=
        Ident b
        Number 3
"
        );
    }

    #[test]
    fn parse_update_prefix() {
        let dump = parse_and_dump("let x = 1; ++x; --x;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Number 1
  ExpressionStatement
    Update prefix ++
      Ident x
  ExpressionStatement
    Update prefix --
      Ident x
"
        );
    }

    #[test]
    fn parse_update_postfix() {
        let dump = parse_and_dump("let x = 1; x++; x--;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Number 1
  ExpressionStatement
    Update postfix ++
      Ident x
  ExpressionStatement
    Update postfix --
      Ident x
"
        );
    }

    #[test]
    fn parse_update_in_init() {
        let dump = parse_and_dump("let a = 1; let b = a++; let c = ++a;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: a
    init:
      Number 1
  Let
    name: b
    init:
      Update postfix ++
        Ident a
  Let
    name: c
    init:
      Update prefix ++
        Ident a
"
        );
    }

    #[test]
    fn parse_comma() {
        let dump = parse_and_dump("let x = (1, 2);").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Paren
        Binary ,
          Number 1
          Number 2
"
        );
    }

    #[test]
    fn parse_comma_left_assoc() {
        let dump = parse_and_dump("let x = (1, 2, 3);").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Paren
        Binary ,
          Binary ,
            Number 1
            Number 2
          Number 3
"
        );
    }

    #[test]
    fn parse_comma_with_assignment() {
        let dump = parse_and_dump("let a; let x = (a = 1, 2);").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: a
  Let
    name: x
    init:
      Paren
        Binary ,
          Assign =
            Ident a
            Number 1
          Number 2
"
        );
    }

    #[test]
    fn parse_call_args_not_comma_expr() {
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
}
