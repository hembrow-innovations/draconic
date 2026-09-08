use super::*;

impl Parser {
    /// Conditional: `test ? AssignmentExpression : AssignmentExpression`.
    pub(crate) fn parse_conditional(&mut self) -> Result<Expr, Diagnostic> {
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
    pub(crate) fn parse_nullish(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_bit_or(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_bit_xor(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_bit_and(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_equality(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_relational(&mut self) -> Result<Expr, Diagnostic> {
        // `#name in object` (E18.40) — PrivateIdentifier is only valid as LHS of `in`.
        if self.ctx.allow_in {
            if let TokenKind::PrivateIdent(pname) = &self.current().kind {
                let name = pname.clone();
                let name_span = self.bump().span;
                self.expect(&TokenKind::In)?;
                let object = self.parse_shift()?;
                let span = span_merge(name_span, expr_span(&object));
                return Ok(Expr::PrivateIn {
                    name: Ident {
                        name,
                        span: name_span,
                    },
                    object: Box::new(object),
                    span,
                });
            }
        }
        let mut left = self.parse_shift()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::LtEq => BinaryOp::LtEq,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::GtEq => BinaryOp::GtEq,
                TokenKind::In if self.ctx.allow_in => BinaryOp::In,
                TokenKind::InstanceOf => BinaryOp::InstanceOf,
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

    pub(crate) fn parse_shift(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_factor(&mut self) -> Result<Expr, Diagnostic> {
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
    /// Left must be UpdateExpression; unparenthesized UnaryExpression base is early SyntaxError (E19.58).
    pub(crate) fn parse_exponentiation(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.parse_unary()?;
        if self.check(&TokenKind::StarStar) {
            if expr_is_unparenthesized_unary_op(&left) {
                return Err(Diagnostic::new(
                    "unary expression cannot be used as exponentiation base without parentheses"
                        .to_string(),
                    expr_span(&left),
                ));
            }
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

    pub(crate) fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
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
            // AwaitExpression only when [+Await]; else IdentifierReference (E19.52).
            TokenKind::Await if self.ctx.in_await_context => Some(UnaryOp::Await),
            // N03.03 native pointers: `&x` address-of, `*p` dereference.
            TokenKind::BitAnd => Some(UnaryOp::Ref),
            TokenKind::Star => Some(UnaryOp::Deref),
            _ => None,
        };
        if let Some(op) = op {
            let start = self.bump().span.start.0;
            let arg = self.parse_unary()?;
            let end = expr_span(&arg).end.0;
            // E19.36: `delete` of MemberExpression/CallExpression.PrivateName is early SyntaxError
            // (class bodies are strict; also covered parenthesized forms).
            if matches!(op, UnaryOp::Delete) && expr_is_private_member_reference(&arg) {
                return Err(Diagnostic::new(
                    "cannot delete private field or method".to_string(),
                    Span::new(start, end),
                ));
            }
            return Ok(Expr::Unary {
                op,
                arg: Box::new(arg),
                span: Span::new(start, end),
            });
        }
        self.parse_as()
    }

    /// Dual-worlds / type boundary: `expr as T` (postfix after update; T06).
    pub(crate) fn parse_as(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_update()?;
        while self.check(&TokenKind::As) {
            self.bump();
            let ty = self.parse_type()?;
            let end = ty.span().end.0;
            let start = expr_span(&expr).start.0;
            expr = Expr::As {
                expr: Box::new(expr),
                ty,
                span: Span::new(start, end),
            };
        }
        Ok(expr)
    }

    /// Postfix update (`lhs++` / `lhs--`) and call.
    pub(crate) fn parse_update(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_lhs()?;
        loop {
            let tok = self.current();
            let op = match &tok.kind {
                TokenKind::PlusPlus => UpdateOp::Inc,
                TokenKind::MinusMinus => UpdateOp::Dec,
                _ => break,
            };
            // ECMA-262: no LineTerminator between LeftHandSideExpression and `++`/`--`.
            if tok.preceded_by_line_terminator {
                break;
            }
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
}

/// True when `expr` is an unparenthesized UnaryExpression with a unary operator
/// (not UpdateExpression). Invalid as the left operand of `**` (E19.58).
pub(crate) fn expr_is_unparenthesized_unary_op(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Unary {
            op: UnaryOp::Plus
                | UnaryOp::Minus
                | UnaryOp::Not
                | UnaryOp::BitNot
                | UnaryOp::TypeOf
                | UnaryOp::Void
                | UnaryOp::Delete
                | UnaryOp::Await
                | UnaryOp::Ref
                | UnaryOp::Deref,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::super::*;

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
    fn parse_comparison() {
        let dump = parse_and_dump("a === b && c !== d;").unwrap();
        assert!(dump.contains("Binary ==="));
        assert!(dump.contains("Binary &&"));
        assert!(dump.contains("Binary !=="));
    }

    #[test]
    fn parse_in_operator() {
        let dump = parse_and_dump(r#""a" in obj;"#).unwrap();
        assert!(dump.contains("Binary in"), "dump={dump}");
        assert!(dump.contains("String \"a\""));
        assert!(dump.contains("Ident obj"));
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

    /// E19.58: unparenthesized UnaryExpression cannot be `**` base.
    #[test]
    fn parse_exponentiation_unary_base_fails() {
        assert!(parse("-3 ** 2;").is_err());
        assert!(parse("+3 ** 2;").is_err());
        assert!(parse("!3 ** 2;").is_err());
        assert!(parse("~3 ** 2;").is_err());
        assert!(parse("typeof 3 ** 2;").is_err());
        assert!(parse("void 3 ** 2;").is_err());
        assert!(parse("delete x ** 2;").is_err());
        assert!(parse("(-3) ** 2;").is_ok());
        assert!(parse("2 ** -1;").is_ok());
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
    fn parse_as_type_assertion() {
        let dump = parse_and_dump("let x = n as i32;").unwrap();
        assert!(
            dump.contains("As\n") && dump.contains("Ident n") && dump.contains("NamedType i32"),
            "expected As node, got:\n{dump}"
        );
        let chain = parse_and_dump("let y = (n + 1) as number as i32;").unwrap();
        assert!(
            chain.matches("As\n").count() >= 2,
            "chained as, got:\n{chain}"
        );
    }

    #[test]
    fn parse_private_in() {
        let dump = parse_and_dump("class C { #x = 1; m(o) { return #x in o; } }").unwrap();
        assert!(dump.contains("PrivateIn"), "{dump}");
        assert!(dump.contains("name: #x"), "{dump}");
        assert!(dump.contains("Ident o"), "{dump}");
    }
}
