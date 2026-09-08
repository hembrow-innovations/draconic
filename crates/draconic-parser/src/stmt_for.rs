use super::*;

impl Parser {
    pub(crate) fn parse_for(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::For)?.span.start.0;
        // `for await (… of …)` — async iteration (E18.42); only when [+Await].
        let is_await = if self.ctx.in_await_context && self.check(&TokenKind::Await) {
            self.bump();
            true
        } else {
            false
        };
        self.expect(&TokenKind::LParen)?;

        // E19.44: `for (using x of …)` / `for (await using x of …)` / classic `for (using x = …; …)`.
        // ForStatement / ForInOfStatement heads are always allowed (even at script top level).
        if self.await_using_starts_declaration() || self.using_starts_declaration() {
            let using_await = self.await_using_starts_declaration();
            let kind = if using_await {
                BindingKind::AwaitUsing
            } else {
                BindingKind::Using
            };
            let let_start = self.current().span.start.0;
            if using_await {
                self.expect(&TokenKind::Await)?;
                let using_tok = self.bump();
                if !matches!(&using_tok.kind, TokenKind::Ident(n) if n == "using") {
                    return Err(Diagnostic::new(
                        "expected using after await".to_string(),
                        using_tok.span,
                    ));
                }
            } else {
                self.bump(); // using
            }
            if self.check(&TokenKind::LBracket) || self.check(&TokenKind::LBrace) {
                return Err(Diagnostic::new(
                    "using declaration does not allow binding patterns".to_string(),
                    self.current().span,
                ));
            }
            let name_tok = self.expect_ident()?;
            let binding = BindingPattern::Ident(Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            });
            let binding_end = binding.span().end.0;
            if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
                let is_in = self.check(&TokenKind::In);
                self.bump();
                if is_in {
                    return Err(Diagnostic::new(
                        "using declaration is not allowed in for-in".to_string(),
                        Span::new(let_start, binding_end),
                    ));
                }
                // E19.67: for-of RHS is AssignmentExpression (not Expression/comma).
                let right = self.parse_assignment()?;
                self.expect(&TokenKind::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                Self::reject_labelled_function(&body)?;
                let end = stmt_span(&body).end.0;
                let left = Box::new(Stmt::Let {
                    kind,
                    binding,
                    type_ann: None,
                    init: None,
                    span: Span::new(let_start, binding_end),
                });
                return Ok(Stmt::ForOf {
                    left,
                    right,
                    body,
                    is_await,
                    span: Span::new(start, end),
                });
            }
            // Classic `for (using x = init; …)` — initializer required.
            if !self.check(&TokenKind::Eq) {
                return Err(Diagnostic::new(
                    "using declaration requires an initializer".to_string(),
                    binding.span(),
                ));
            }
            self.bump();
            let init_expr = self.with_ctx(|c| c.allow_in = false, Self::parse_assignment)?;
            if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
                return Err(Diagnostic::new(
                    "for-of binding cannot have an initializer".to_string(),
                    binding.span(),
                ));
            }
            let let_end = expr_span(&init_expr).end.0;
            self.expect(&TokenKind::Semi)?;
            let left_init = Some(Box::new(Stmt::Let {
                kind,
                binding,
                type_ann: None,
                init: Some(init_expr),
                span: Span::new(let_start, let_end),
            }));
            return self.finish_classic_for(start, left_init);
        }

        // `for (let/const/var binding in/of right)` and classic `for (let/const/var …; …; …)`.
        // Annex B.3.5: `for (var name = init in right)` only (ident binding).
        if self.check(&TokenKind::Let)
            || self.check(&TokenKind::Const)
            || self.check(&TokenKind::Var)
        {
            let kind = if self.check(&TokenKind::Const) {
                BindingKind::Const
            } else if self.check(&TokenKind::Var) {
                BindingKind::Var
            } else {
                BindingKind::Let
            };
            let let_start = self.bump().span.start.0;
            let binding = self.parse_binding_pattern()?;
            if kind != BindingKind::Var && binding_pattern_bound_names_contain_let(&binding) {
                return Err(Diagnostic::new(
                    "'let' is not allowed as a lexical binding name".to_string(),
                    binding.span(),
                ));
            }
            let binding_end = binding.span().end.0;
            if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
                let is_in = self.check(&TokenKind::In);
                self.bump();
                // E19.67: for-in/of RHS is AssignmentExpression (not Expression/comma).
                let right = self.parse_assignment()?;
                self.expect(&TokenKind::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                Self::reject_labelled_function(&body)?;
                let end = stmt_span(&body).end.0;
                let left = Box::new(Stmt::Let {
                    kind,
                    binding,
                    type_ann: None,
                    init: None,
                    span: Span::new(let_start, binding_end),
                });
                return if is_in {
                    if is_await {
                        return Err(Diagnostic::new(
                            "for await…in is not allowed".to_string(),
                            Span::new(start, end),
                        ));
                    }
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
                        is_await,
                        span: Span::new(start, end),
                    })
                };
            }
            if is_await {
                return Err(Diagnostic::new(
                    "for await requires `of`".to_string(),
                    Span::new(start, binding_end),
                ));
            }
            // Classic `for (let/const/var binding: T? = init; …)` / Annex B `for (var name = init in …)`.
            // Disable relational `in` while parsing the initializer so
            // `for (var k = 1 in obj)` is Annex B, not `k = (1 in obj)`.
            let type_ann = if matches!(binding, BindingPattern::Ident(_)) {
                self.parse_optional_type_ann()?
            } else {
                None
            };
            let init_expr = if self.check(&TokenKind::Eq) {
                self.bump();
                Some(self.with_ctx(|c| c.allow_in = false, Self::parse_assignment)?)
            } else if matches!(
                binding,
                BindingPattern::Array { .. } | BindingPattern::Object { .. }
            ) {
                return Err(Diagnostic::new(
                    "destructuring declaration requires an initializer".to_string(),
                    binding.span(),
                ));
            } else if kind == BindingKind::Const {
                return Err(Diagnostic::new(
                    "const declaration requires an initializer".to_string(),
                    binding.span(),
                ));
            } else {
                None
            };
            // Annex B.3.5 / for-of reject: initializer then `in`/`of`.
            // E19.69: Annex B for-in initializer is prohibited in strict mode.
            if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
                let is_in = self.check(&TokenKind::In);
                if !is_in {
                    return Err(Diagnostic::new(
                        "for-of binding cannot have an initializer".to_string(),
                        binding.span(),
                    ));
                }
                if kind != BindingKind::Var
                    || type_ann.is_some()
                    || !matches!(binding, BindingPattern::Ident(_))
                {
                    return Err(Diagnostic::new(
                        "for-in binding cannot have an initializer".to_string(),
                        binding.span(),
                    ));
                }
                if self.ctx.in_strict {
                    return Err(Diagnostic::new(
                        "for-in binding cannot have an initializer in strict mode".to_string(),
                        binding.span(),
                    ));
                }
                self.bump();
                // E19.67: for-in RHS is AssignmentExpression (not Expression/comma).
                let right = self.parse_assignment()?;
                self.expect(&TokenKind::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                Self::reject_labelled_function(&body)?;
                let end = stmt_span(&body).end.0;
                let let_end = if let Some(ref e) = init_expr {
                    expr_span(e).end.0
                } else {
                    binding_end
                };
                let left = Box::new(Stmt::Let {
                    kind,
                    binding,
                    type_ann: None,
                    init: init_expr,
                    span: Span::new(let_start, let_end),
                });
                return Ok(Stmt::ForIn {
                    left,
                    right,
                    body,
                    span: Span::new(start, end),
                });
            }
            let let_end = if let Some(ref e) = init_expr {
                expr_span(e).end.0
            } else if let Some(ref ann) = type_ann {
                ann.span().end.0
            } else {
                binding_end
            };
            self.expect(&TokenKind::Semi)?;
            let left_init = Some(Box::new(Stmt::Let {
                kind,
                binding,
                type_ann,
                init: init_expr,
                span: Span::new(let_start, let_end),
            }));
            return self.finish_classic_for(start, left_init);
        }

        if self.check(&TokenKind::Semi) {
            if is_await {
                return Err(Diagnostic::new(
                    "for await requires `of`".to_string(),
                    Span::new(start, self.current_span().start.0),
                ));
            }
            self.bump();
            return self.finish_classic_for(start, None);
        }

        // Expression left: `for (lhs in/of right)` or classic `for (expr; …)`.
        // Disable relational `in` so `for (z in obj)` does not consume `in` here.
        let expr = self.with_ctx(|c| c.allow_in = false, Self::parse_expr)?;
        let mut left_span = expr_span(&expr);
        if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
            let is_in = self.check(&TokenKind::In);
            self.bump();
            // E19.67: for-in/of RHS is AssignmentExpression (not Expression/comma).
            let right = self.parse_assignment()?;
            self.expect(&TokenKind::RParen)?;
            let body = Box::new(self.parse_stmt()?);
            Self::reject_labelled_function(&body)?;
            let end = stmt_span(&body).end.0;
            // Reinterpret array/object literals as assignment patterns for for-in/of LHS.
            let expr = array_expr_to_pattern(&expr)
                .or_else(|| object_expr_to_pattern(&expr))
                .unwrap_or(expr);
            left_span = expr_span(&expr);
            let left = Box::new(Stmt::Expression {
                expr,
                span: left_span,
            });
            return if is_in {
                if is_await {
                    return Err(Diagnostic::new(
                        "for await…in is not allowed".to_string(),
                        Span::new(start, end),
                    ));
                }
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
                    is_await,
                    span: Span::new(start, end),
                })
            };
        }

        if is_await {
            return Err(Diagnostic::new(
                "for await requires `of`".to_string(),
                Span::new(start, left_span.end.0),
            ));
        }
        self.expect(&TokenKind::Semi)?;
        let init = Some(Box::new(Stmt::Expression {
            expr,
            span: left_span,
        }));
        self.finish_classic_for(start, init)
    }

    pub(crate) fn finish_classic_for(
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
        Self::reject_labelled_function(&body)?;
        let end = stmt_span(&body).end.0;
        Ok(Stmt::For {
            init,
            test,
            update,
            body,
            span: Span::new(start, end),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn parse_for_const_of() {
        let dump = parse_and_dump(r#"for (const c of "ab") c;"#).unwrap();
        assert!(dump.contains("ForOf"), "got:\n{dump}");
        assert!(dump.contains("Const"), "got:\n{dump}");
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
    fn parse_for_in_var() {
        let dump = parse_and_dump("for (var k in s) { x = k; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForIn
    left:
      Var
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
    fn parse_for_var_classic() {
        let dump = parse_and_dump("for (var i = 0; i < 3; i = i + 1) sum = i;").unwrap();
        assert!(dump.contains("Var"), "{dump}");
        assert!(dump.contains("For\n"), "{dump}");
    }

    #[test]
    fn parse_for_in_var_init_annex_b() {
        let dump = parse_and_dump("for (var k = 1 in s) x = k;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForIn
    left:
      Var
        name: k
        init:
          Number 1
    right:
      Ident s
    body:
      ExpressionStatement
        Assign =
          Ident x
          Ident k
"
        );
    }

    #[test]
    fn parse_for_of_var_init_rejected() {
        let err = parse_and_dump("for (var k = 1 of s) x = k;").unwrap_err();
        assert!(
            err.message
                .contains("for-of binding cannot have an initializer"),
            "{err:?}"
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
    fn parse_for_of_const_array_pattern() {
        let dump = parse_and_dump("for (const [a] of s) x = a;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForOf
    left:
      Const
        ArrayPattern
          name: a
    right:
      Ident s
    body:
      ExpressionStatement
        Assign =
          Ident x
          Ident a
"
        );
    }

    #[test]
    fn parse_for_of_let_object_pattern() {
        let dump = parse_and_dump("for (let {x} of s) y = x;").unwrap();
        assert!(dump.contains("ForOf"), "got:\n{dump}");
        assert!(dump.contains("ObjectPattern"), "got:\n{dump}");
        assert!(dump.contains("name: x"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_in_var_array_pattern() {
        let dump = parse_and_dump("for (var [a] in s) x = a;").unwrap();
        assert!(dump.contains("ForIn"), "got:\n{dump}");
        assert!(dump.contains("ArrayPattern"), "got:\n{dump}");
        assert!(dump.contains("Var"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_of_assign_array_pattern() {
        let dump = parse_and_dump("for ([a] of s) {}").unwrap();
        assert!(dump.contains("ForOf"), "got:\n{dump}");
        assert!(dump.contains("ArrayPattern"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_classic_let_array_pattern() {
        let dump = parse_and_dump("for (let [a] = arr; a; ) x = a;").unwrap();
        assert!(dump.contains("For\n"), "got:\n{dump}");
        assert!(dump.contains("ArrayPattern"), "got:\n{dump}");
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
    fn parse_for_await_of_let() {
        let dump =
            parse_and_dump("async function f() { for await (let x of a) { y = x; } }").unwrap();
        assert!(dump.contains("ForOf await"), "got:\n{dump}");
        assert!(dump.contains("name: x"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_await_of_assign() {
        let dump = parse_and_dump("async function f() { for await (x of a) {} }").unwrap();
        assert!(dump.contains("ForOf await"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_await_in_rejected() {
        let err = parse_and_dump("async function f() { for await (let x in a) {} }").unwrap_err();
        assert!(
            err.message.contains("for await") && err.message.contains("in"),
            "{err:?}"
        );
    }

    /// E19.78: `in` allowed inside computed property names even in for-in cover.
    #[test]
    fn parse_computed_key_in_inside_for() {
        let dump = parse_and_dump(
            "for (C = class { get ['x' in empty]() { return 1; } }; ;) { break; }\n",
        )
        .unwrap();
        assert!(
            dump.contains("ClassExpression") && dump.contains("Binary"),
            "expected class with `in` in computed key, got:\n{dump}"
        );
    }
}
