use super::*;

impl Parser {
    /// Expression: `AssignmentExpression` (`,` `AssignmentExpression`)* left-assoc.
    pub(crate) fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
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
    /// Also covers arrow functions (`params => body`) and `yield` (YieldExpression),
    /// which are AssignmentExpressions in ECMA-262.
    pub(crate) fn parse_assignment(&mut self) -> Result<Expr, Diagnostic> {
        if self.is_arrow_start() {
            return self.parse_arrow_function();
        }
        // YieldExpression only in generator bodies/params; else IdentifierReference (E19.37).
        if self.check(&TokenKind::Yield) && self.ctx.in_generator {
            return self.parse_yield();
        }
        let left = self.parse_conditional()?;
        let Some(op) = self.peek_assign_op() else {
            // E19.67: CoverInitializedName is only valid as AssignmentPattern, not ObjectLiteral.
            if expr_contains_cover_initialized_name(&left) {
                return Err(Diagnostic::new(
                    "CoverInitializedName is not valid in object literal".to_string(),
                    expr_span(&left),
                ));
            }
            return Ok(left);
        };
        self.bump();
        let value = self.parse_assignment()?;
        let span = span_merge(expr_span(&left), expr_span(&value));
        let target = if op == AssignOp::Eq {
            if let Some(pat) = array_expr_to_pattern(&left) {
                pat
            } else if let Some(pat) = object_expr_to_pattern(&left) {
                pat
            } else {
                left
            }
        } else {
            // Compound assignment: CoverInitializedName invalid on LHS.
            if expr_contains_cover_initialized_name(&left) {
                return Err(Diagnostic::new(
                    "CoverInitializedName is not valid in object literal".to_string(),
                    expr_span(&left),
                ));
            }
            left
        };
        Ok(Expr::Assign {
            target: Box::new(target),
            op,
            value: Box::new(value),
            span,
        })
    }

    /// `yield` / `yield AssignmentExpression` / `yield* AssignmentExpression`.
    /// Bare `yield` / `yield;` → yield undefined (`void 0`).
    pub(crate) fn parse_yield(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::Yield)?.span.start.0;
        // No LineTerminator between `yield` and `*` (E19.39 yield-star-after-newline).
        let delegate =
            if self.check(&TokenKind::Star) && !self.current().preceded_by_line_terminator {
                self.bump();
                true
            } else {
                false
            };
        // No LineTerminator before the operand (incl. bare `yield` then `*` on next line).
        let arg = if !delegate
            && (self.current().preceded_by_line_terminator
                || self.check(&TokenKind::Semi)
                || self.check(&TokenKind::RBrace)
                || self.check(&TokenKind::RParen)
                || self.check(&TokenKind::RBracket)
                || self.check(&TokenKind::Comma)
                || self.check(&TokenKind::Colon)
                || self.check(&TokenKind::Eof))
        {
            Expr::Unary {
                op: UnaryOp::Void,
                arg: Box::new(Expr::Number(NumberLit {
                    raw: "0".into(),
                    span: Span::new(start, start),
                })),
                span: Span::new(start, start),
            }
        } else {
            // Right-associative: `yield yield 1` and `yield 1 + 2` / `yield x = 1`.
            // `yield*` requires an AssignmentExpression operand.
            self.parse_assignment()?
        };
        let end = expr_span(&arg).end.0;
        Ok(Expr::Unary {
            op: if delegate {
                UnaryOp::YieldStar
            } else {
                UnaryOp::Yield
            },
            arg: Box::new(arg),
            span: Span::new(start, end),
        })
    }

    /// `async? ident =>` or `async? (params) =>` with simple ident params only.
    pub(crate) fn is_arrow_start(&self) -> bool {
        if self.check(&TokenKind::Async) && !self.peek_is(&TokenKind::Function) {
            // E19.58: no LineTerminator between `async` and ArrowFormalParameters / binding.
            if self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| t.preceded_by_line_terminator)
            {
                return false;
            }
            let next = self.tokens.get(self.pos + 1).map(|t| &t.kind);
            let after = self.tokens.get(self.pos + 2);
            let next_is_ident = matches!(next, Some(TokenKind::Ident(_)))
                || (matches!(next, Some(TokenKind::Yield)) && self.yield_is_ident())
                || (matches!(next, Some(TokenKind::Await)) && self.await_is_ident());
            if next_is_ident
                && matches!(after.map(|t| &t.kind), Some(TokenKind::Arrow))
                && !after.is_some_and(|t| t.preceded_by_line_terminator)
            {
                return true;
            }
            if self.peek_is(&TokenKind::LParen) {
                return self.lookahead_paren_arrow_from(self.pos + 1);
            }
            return false;
        }
        if self.at_binding_ident() && self.peek_is(&TokenKind::Arrow) {
            // E19.58: no LineTerminator between ArrowParameters and `=>`.
            return !self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| t.preceded_by_line_terminator);
        }
        if self.check(&TokenKind::LParen) {
            return self.lookahead_paren_arrow_from(self.pos);
        }
        false
    }

    pub(crate) fn lookahead_paren_arrow_from(&self, mut i: usize) -> bool {
        if !matches!(self.tokens.get(i).map(|t| &t.kind), Some(TokenKind::LParen)) {
            return false;
        }
        i += 1;
        let mut depth = 1usize;
        while i < self.tokens.len() && depth > 0 {
            match &self.tokens[i].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => depth -= 1,
                TokenKind::Eof => return false,
                _ => {}
            }
            i += 1;
        }
        if depth != 0 {
            return false;
        }
        // Optional return type `: Ident` between `)` and `=>`.
        if matches!(self.tokens.get(i).map(|t| &t.kind), Some(TokenKind::Colon)) {
            i += 1;
            if !matches!(
                self.tokens.get(i).map(|t| &t.kind),
                Some(TokenKind::Ident(_))
            ) {
                return false;
            }
            i += 1;
        }
        // E19.58: no LineTerminator between ArrowParameters and `=>`.
        matches!(self.tokens.get(i).map(|t| &t.kind), Some(TokenKind::Arrow))
            && !self
                .tokens
                .get(i)
                .is_some_and(|t| t.preceded_by_line_terminator)
    }

    /// `async? (params): ret? => body` or bare `async? param => body`.
    pub(crate) fn parse_arrow_function(&mut self) -> Result<Expr, Diagnostic> {
        let (is_async, start) = if self.check(&TokenKind::Async) {
            let start = self.bump().span.start.0;
            (true, start)
        } else {
            (false, self.current().span.start.0)
        };
        // Arrows inherit [Yield] from the surrounding context (not a new generator).
        // Params inherit outer [Await] for non-async; async params/body are [+Await].
        // Non-async ConciseBody is always [~Await] (E19.52).
        self.with_ctx(
            |c| {
                if is_async {
                    c.in_await_context = true;
                }
            },
            |p| {
                let (params, return_type) = if p.at_binding_ident() {
                    let ident = p.expect_ident()?;
                    (
                        vec![Param {
                            binding: BindingPattern::Ident(Ident {
                                name: ident.ident_name(),
                                span: ident.span,
                            }),
                            type_ann: None,
                            default: None,
                            rest: false,
                        }],
                        None,
                    )
                } else {
                    p.expect(&TokenKind::LParen)?;
                    let params = p.parse_param_list()?;
                    p.expect(&TokenKind::RParen)?;
                    let return_type = p.parse_optional_type_ann()?;
                    (params, return_type)
                };
                // E19.58: no LineTerminator between ArrowParameters and `=>`.
                if p.current().preceded_by_line_terminator && p.check(&TokenKind::Arrow) {
                    return Err(Diagnostic::new(
                        "line terminator not allowed before '=>'".to_string(),
                        p.current_span(),
                    ));
                }
                p.expect(&TokenKind::Arrow)?;
                if !is_async {
                    p.ctx.in_await_context = false;
                }
                let body = if p.check(&TokenKind::LBrace) {
                    ArrowBody::Block(Box::new(p.parse_function_body_block()?))
                } else {
                    ArrowBody::Expr(Box::new(p.parse_assignment()?))
                };
                let end = match &body {
                    ArrowBody::Block(s) => stmt_span(s).end.0,
                    ArrowBody::Expr(e) => expr_span(e).end.0,
                };
                Ok(Expr::ArrowFunction {
                    params,
                    return_type,
                    body,
                    is_async,
                    span: Span::new(start, end),
                })
            },
        )
    }

    pub(crate) fn peek_assign_op(&self) -> Option<AssignOp> {
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
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn parse_arrow_expression_body() {
        let dump = parse_and_dump("let f = (a) => a;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: f
    init:
      ArrowFunction
        params:
          name: a
        body:
          Ident a
"
        );
    }

    #[test]
    fn parse_async_arrow() {
        let dump = parse_and_dump(
            "let f = async () => 1; let g = async (x) => { return await x; }; let h = async y => y;",
        )
        .unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: f
    init:
      ArrowFunction
        async: true
        body:
          Number 1
  Let
    name: g
    init:
      ArrowFunction
        async: true
        params:
          name: x
        body:
          Block
            Return
              Unary await
                Ident x
  Let
    name: h
    init:
      ArrowFunction
        async: true
        params:
          name: y
        body:
          Ident y
"
        );
    }

    #[test]
    fn parse_arrow_block_body_and_bare_param() {
        let dump = parse_and_dump("let f = x => { return x; }; let g = () => 1;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: f
    init:
      ArrowFunction
        params:
          name: x
        body:
          Block
            Return
              Ident x
  Let
    name: g
    init:
      ArrowFunction
        body:
          Number 1
"
        );
    }

    /// E19.58: no LineTerminator before `=>`.
    #[test]
    fn parse_arrow_asi_restriction_fails() {
        assert!(parse("var af = ()\n=> {};").is_err());
        assert!(parse("var af = x\n=> {};").is_err());
        assert!(parse("var af = x\n=> x;").is_err());
        assert!(parse("async\n(foo) => {};").is_err());
        assert!(parse("() => {};").is_ok());
        assert!(parse("async (foo) => {};").is_ok());
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
    fn parse_yield_assignment_expr_rhs() {
        // `yield` is AssignmentExpression-level: `yield 1 + 2` → yield (1 + 2), not (yield 1) + 2.
        let dump = parse_and_dump("function* g() { yield 1 + 2; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  FunctionDeclaration
    generator: true
    name: g
    body:
      Block
        ExpressionStatement
          Unary yield
            Binary +
              Number 1
              Number 2
"
        );
    }

    #[test]
    fn parse_yield_bare_and_conditional_rhs() {
        let bare = parse_and_dump("function* g() { yield; }").unwrap();
        assert!(bare.contains("Unary yield"), "got:\n{bare}");
        assert!(
            bare.contains("Unary void"),
            "bare yield → void 0, got:\n{bare}"
        );

        let cond = parse_and_dump("function* g() { yield 1 ? 2 : 3; }").unwrap();
        assert!(
            cond.contains("Unary yield") && cond.contains("Conditional"),
            "yield RHS includes conditional, got:\n{cond}"
        );
        // Conditional must be under yield, not yield under binary/conditional left.
        let yield_idx = cond.find("Unary yield").expect("yield");
        let cond_idx = cond.find("Conditional").expect("conditional");
        assert!(
            yield_idx < cond_idx,
            "expected yield to wrap conditional, got:\n{cond}"
        );
    }

    #[test]
    fn parse_yield_star_delegate() {
        let dump = parse_and_dump("function* g() { yield* inner(); }").unwrap();
        assert!(
            dump.contains("Unary yield*"),
            "expected yield* unary, got:\n{dump}"
        );
        assert!(
            dump.contains("Call"),
            "yield* operand should parse as call, got:\n{dump}"
        );
        let star = parse_and_dump("function* g() { yield* [1, 2]; }").unwrap();
        assert!(
            star.contains("Unary yield*") && star.contains("Array"),
            "yield* array iterable, got:\n{star}"
        );
    }
}
