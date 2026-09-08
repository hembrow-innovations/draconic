use super::*;

impl Parser {
    /// `async? function *? name? (params) { body }` in expression position.
    pub(crate) fn parse_function_expression(&mut self) -> Result<Expr, Diagnostic> {
        let (is_async, start) = if self.check(&TokenKind::Async) {
            let start = self.bump().span.start.0;
            (true, start)
        } else {
            (false, self.current_span().start.0)
        };
        self.expect(&TokenKind::Function)?;
        let is_generator = if self.check(&TokenKind::Star) {
            self.bump();
            true
        } else {
            false
        };
        // Generator BindingIdentifier has [+Yield]: optional name cannot be `yield`.
        // Non-async FunctionExpression name/params/body are [~Await]; async are [+Await] (E19.52).
        self.with_ctx(
            |c| {
                c.in_generator = is_generator;
                c.in_await_context = is_async;
            },
            |p| {
                let name = if p.at_binding_ident() {
                    let name_tok = p.expect_ident()?;
                    let id = Ident {
                        name: name_tok.ident_name(),
                        span: name_tok.span,
                    };
                    // E19.49: strict BindingIdentifier cannot be `eval`/`arguments`.
                    if p.ctx.in_strict && is_strict_forbidden_binding_name(&id.name) {
                        return Err(Diagnostic::new(
                            format!("binding `{}` is invalid in strict mode", id.name),
                            id.span,
                        ));
                    }
                    Some(id)
                } else {
                    None
                };
                p.expect(&TokenKind::LParen)?;
                let params = p.parse_param_list()?;
                p.expect(&TokenKind::RParen)?;
                // E19.58: FormalParameters of a generator must not contain YieldExpression.
                if is_generator && params_contain_yield_expr(&params) {
                    return Err(Diagnostic::new(
                        "generator parameters cannot contain yield".to_string(),
                        Span::new(start, p.current_span().end.0),
                    ));
                }
                // E19.67: FormalParameters of an async function must not contain AwaitExpression.
                if is_async && params_contain_await_expr(&params) {
                    return Err(Diagnostic::new(
                        "async function parameters cannot contain await".to_string(),
                        Span::new(start, p.current_span().end.0),
                    ));
                }
                let return_type = p.parse_optional_type_ann()?;
                // E19.67: non-arrow functions introduce NewTarget.
                p.ctx.new_target_depth += 1;
                let body = Box::new(p.parse_function_body_block()?);
                if let Some(ref id) = name {
                    if is_strict_forbidden_binding_name(&id.name)
                        && block_has_use_strict_directive(&body)
                    {
                        return Err(Diagnostic::new(
                            format!("binding `{}` is invalid in strict mode", id.name),
                            id.span,
                        ));
                    }
                }
                let end = stmt_span(&body).end.0;
                Ok(Expr::FunctionExpression {
                    name,
                    params,
                    return_type,
                    body,
                    is_async,
                    is_generator,
                    is_method: false,
                    span: Span::new(start, end),
                })
            },
        )
    }

    pub(crate) fn parse_param_list(&mut self) -> Result<Vec<Param>, Diagnostic> {
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let param = self.parse_param()?;
                let is_rest = param.rest;
                params.push(param);
                if is_rest {
                    break;
                }
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    // Trailing comma before `)` is allowed; rest cannot follow a trailing comma
                    // after itself (already broken above).
                    if self.check(&TokenKind::RParen) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        Ok(params)
    }

    /// Binding pattern param, optional `: T` / `= default`, or `...name` / `...name: T`.
    fn parse_param(&mut self) -> Result<Param, Diagnostic> {
        if self.check(&TokenKind::DotDotDot) {
            let dots_start = self.current().span.start.0;
            self.bump();
            let p = self.expect_ident()?;
            let binding = BindingPattern::Ident(Ident {
                name: p.ident_name(),
                span: Span::new(dots_start, p.span.end.0),
            });
            let type_ann = self.parse_optional_type_ann()?;
            if self.check(&TokenKind::Eq) {
                return Err(Diagnostic::new(
                    "rest parameter cannot have a default",
                    self.current().span,
                ));
            }
            return Ok(Param {
                binding,
                type_ann,
                default: None,
                rest: true,
            });
        }
        let binding = self.parse_binding_pattern()?;
        let type_ann = self.parse_optional_type_ann()?;
        let default = if self.check(&TokenKind::Eq) {
            self.bump();
            Some(self.parse_assignment()?)
        } else {
            None
        };
        Ok(Param {
            binding,
            type_ann,
            default,
            rest: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn parse_function_expression() {
        let dump = parse_and_dump("let f = function (a) { return a; };").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: f
    init:
      FunctionExpression
        params:
          name: a
        body:
          Block
            Return
              Ident a
"
        );
    }

    #[test]
    fn parse_named_function_expression() {
        let dump = parse_and_dump("let f = function g(n) { return g(n); };").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: f
    init:
      FunctionExpression
        name: g
        params:
          name: n
        body:
          Block
            Return
              Call
                callee:
                  Ident g
                arg[0]:
                  Ident n
"
        );
    }

    #[test]
    fn parse_async_generators() {
        let dump = parse_and_dump(
            "async function* g() { yield 1; yield await p; } let f = async function* (x) { yield x; }; let o = { async *m() { yield 2; } }; class C { async *n() { yield 3; } static async *s() { yield 4; } }",
        )
        .unwrap();
        assert!(dump.contains("async: true"), "got:\n{dump}");
        assert!(dump.contains("generator: true"), "got:\n{dump}");
        assert!(dump.contains("Unary yield"), "got:\n{dump}");
        assert!(dump.contains("Unary await"), "got:\n{dump}");
        assert!(dump.contains("Method\n"), "got:\n{dump}");
        assert!(dump.contains("StaticMethod\n"), "got:\n{dump}");
    }

    #[test]
    fn parse_default_params() {
        let dump =
            parse_and_dump("function f(a = 1, b) { return a + b; } let g = (x = 2) => x;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  FunctionDeclaration
    name: f
    params:
      name: a
        default:
          Number 1
      name: b
    body:
      Block
        Return
          Binary +
            Ident a
            Ident b
  Let
    name: g
    init:
      ArrowFunction
        params:
          name: x
            default:
              Number 2
        body:
          Ident x
"
        );
    }

    #[test]
    fn parse_rest_params() {
        let dump = parse_and_dump(
            "function f(...a) { return a; } function g(x, ...rest) { return x; } let h = (...xs) => xs;",
        )
        .unwrap();
        assert_eq!(
            dump,
            "\
Program
  FunctionDeclaration
    name: f
    params:
      rest: a
    body:
      Block
        Return
          Ident a
  FunctionDeclaration
    name: g
    params:
      name: x
      rest: rest
    body:
      Block
        Return
          Ident x
  Let
    name: h
    init:
      ArrowFunction
        params:
          rest: xs
        body:
          Ident xs
"
        );
    }

    /// E19.58: yield in generator FormalParameters.
    #[test]
    fn parse_generator_param_default_yield_fails() {
        assert!(parse("function* g(x = yield) {}").is_err());
        assert!(parse("0, function*(x = yield) {};").is_err());
        assert!(parse("function* g(x = 1) { yield x; }").is_ok());
    }
}
