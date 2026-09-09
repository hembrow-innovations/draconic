use crate::*;

impl Parser {
    pub(crate) fn parse_object_expression(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::LBrace)?.span.start.0;
        let mut properties = Vec::new();
        let mut proto_count = 0u32;
        if !self.check(&TokenKind::RBrace) {
            loop {
                let prop = self.parse_object_prop()?;
                // E19.67: duplicate `__proto__` PropertyName : AssignmentExpression is early error.
                if object_prop_is_proto_data(&prop) {
                    proto_count += 1;
                    if proto_count > 1 {
                        return Err(Diagnostic::new(
                            "duplicate __proto__ fields are not allowed in object literal"
                                .to_string(),
                            object_prop_span(&prop),
                        ));
                    }
                }
                properties.push(prop);
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    if self.check(&TokenKind::RBrace) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(Expr::ObjectExpression {
            properties,
            span: Span::new(start, end),
        })
    }

    /// `key: value`, shorthand `{ a }`, method `{ m() {} }` / `{ *m() {} }` / `{ async m() {} }`,
    /// accessor `{ get k(){} }` / `{ set k(v){} }`,
    /// spread `{ ...e }`, or computed `{ [e]: v }` / `{ [e]() {} }` / `{ *[e]() {} }`.
    pub(crate) fn parse_object_prop(&mut self) -> Result<ObjectProp, Diagnostic> {
        let prop_start = self.current_span().start.0;
        if self.check(&TokenKind::DotDotDot) {
            self.bump();
            let expr = self.parse_assignment()?;
            let end = expr_span(&expr).end.0;
            return Ok(ObjectProp::Spread {
                expr,
                span: Span::new(prop_start, end),
            });
        }
        // Accessor: `get name() {}` / `set name(v) {}` (not `get:` / `get()` / shorthand `get`).
        if let Some(kind) = self.peek_accessor_kind() {
            self.bump(); // consume get/set
            let key = self.parse_object_key()?;
            self.expect(&TokenKind::LParen)?;
            return self.with_ctx(
                |c| c.in_await_context = false,
                |p| {
                    let params = p.parse_param_list()?;
                    p.expect(&TokenKind::RParen)?;
                    if kind == AccessorKind::Get && !params.is_empty() {
                        return Err(Diagnostic::new(
                            "getter must have zero parameters".to_string(),
                            p.current_span(),
                        ));
                    }
                    if kind == AccessorKind::Set && params.len() != 1 {
                        return Err(Diagnostic::new(
                            "setter must have exactly one parameter".to_string(),
                            p.current_span(),
                        ));
                    }
                    // E19.67: object accessors introduce NewTarget and allow SuperProperty.
                    p.ctx.new_target_depth += 1;
                    p.ctx.super_property_depth += 1;
                    let body = Box::new(p.parse_function_body_block()?);
                    let end = stmt_span(&body).end.0;
                    Ok(ObjectProp::Accessor {
                        kind,
                        key,
                        params,
                        body,
                        span: Span::new(prop_start, end),
                    })
                },
            );
        }
        // `async m()` / `async *m()` — not property/method named `async`.
        // No LineTerminator between `async` and the method name (E19.39).
        let is_async = if self.check(&TokenKind::Async)
            && self.peek_starts_method_name()
            && !self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| t.preceded_by_line_terminator)
        {
            self.bump();
            true
        } else {
            false
        };
        let is_generator = if self.check(&TokenKind::Star) {
            self.bump();
            true
        } else {
            false
        };
        let key_tok = self.current().clone();
        match &key_tok.kind {
            TokenKind::LBracket => {
                let key_start = if is_async || is_generator {
                    prop_start
                } else {
                    key_tok.span.start.0
                };
                self.bump();
                let key_expr = self.with_ctx(|c| c.allow_in = true, Self::parse_assignment)?;
                self.expect(&TokenKind::RBracket)?;
                let key = ObjectKey::Computed(Box::new(key_expr));
                if self.check(&TokenKind::LParen) {
                    let value = self.parse_method_function(key_start, is_async, is_generator)?;
                    let end = expr_span(&value).end.0;
                    return Ok(ObjectProp::Property {
                        key,
                        value,
                        shorthand: false,
                        span: Span::new(key_start, end),
                    });
                }
                if is_async || is_generator {
                    return Err(Diagnostic::new(
                        "async/generator method requires `(params) { body }`".to_string(),
                        self.current_span(),
                    ));
                }
                self.expect(&TokenKind::Colon)?;
                let value = self.parse_assignment()?;
                let end = expr_span(&value).end.0;
                Ok(ObjectProp::Property {
                    key,
                    value,
                    shorthand: false,
                    span: Span::new(key_start, end),
                })
            }
            _ if key_tok.ident_name_opt().is_some() => {
                let name = key_tok.ident_name();
                let key_span = key_tok.span;
                let span_start = if is_async || is_generator {
                    prop_start
                } else {
                    key_span.start.0
                };
                self.bump();
                let key = ObjectKey::Ident(Ident {
                    name: name.clone(),
                    span: key_span,
                });
                // Method shorthand: `m(params) { body }` / `*m` / `async m`
                if self.check(&TokenKind::LParen) {
                    let value = self.parse_method_function(span_start, is_async, is_generator)?;
                    let end = expr_span(&value).end.0;
                    return Ok(ObjectProp::Property {
                        key,
                        value,
                        shorthand: false,
                        span: Span::new(span_start, end),
                    });
                }
                if is_async || is_generator {
                    return Err(Diagnostic::new(
                        "async/generator method requires `(params) { body }`".to_string(),
                        self.current_span(),
                    ));
                }
                // Property shorthand: `{ a }` / CoverInitializedName `{ a = default }`
                // (latter is only valid as assignment pattern; checker rejects as value).
                // Keywords as IdentifierName keys require `: value` (not bare shorthand),
                // except non-strict non-generator `yield` (IdentifierReference, E19.37)
                // and [~Await] `await` (E19.52).
                // Escaped reserved words are TokenKind::Ident but still invalid IdentifierReference
                // (E19.39 assignment dstr / object shorthand).
                let is_keyword_key = !matches!(key_tok.kind, TokenKind::Ident(_))
                    && !(matches!(key_tok.kind, TokenKind::Yield) && self.yield_is_ident())
                    && !(matches!(key_tok.kind, TokenKind::Await) && self.await_is_ident())
                    && !matches!(key_tok.kind, TokenKind::As | TokenKind::From);
                if matches!(&key_tok.kind, TokenKind::Ident(n) if self.is_invalid_ident_name(n))
                    && (self.check(&TokenKind::Comma)
                        || self.check(&TokenKind::RBrace)
                        || self.check(&TokenKind::Eq))
                {
                    return Err(Diagnostic::new(
                        format!(
                            "'{name}' is a reserved word and cannot be used as an identifier",
                            name = name
                        ),
                        key_span,
                    ));
                }
                if !is_keyword_key
                    && (self.check(&TokenKind::Comma)
                        || self.check(&TokenKind::RBrace)
                        || self.check(&TokenKind::Eq))
                {
                    if self.check(&TokenKind::Eq) {
                        self.bump();
                        let default = self.parse_assignment()?;
                        let end = expr_span(&default).end.0;
                        // Encode CoverInitializedName as `a = default` assign value.
                        let value = Expr::Assign {
                            target: Box::new(Expr::Ident(Ident {
                                name: name.clone(),
                                span: key_span,
                            })),
                            op: AssignOp::Eq,
                            value: Box::new(default),
                            span: Span::new(key_span.start.0, end),
                        };
                        return Ok(ObjectProp::Property {
                            key,
                            value,
                            shorthand: true,
                            span: Span::new(key_span.start.0, end),
                        });
                    }
                    let value = Expr::Ident(Ident {
                        name,
                        span: key_span,
                    });
                    return Ok(ObjectProp::Property {
                        key,
                        value,
                        shorthand: true,
                        span: key_span,
                    });
                }
                self.expect(&TokenKind::Colon)?;
                let value = self.parse_assignment()?;
                let end = expr_span(&value).end.0;
                Ok(ObjectProp::Property {
                    key,
                    value,
                    shorthand: false,
                    span: Span::new(key_span.start.0, end),
                })
            }
            TokenKind::String(value) => {
                self.reject_legacy_octal_token(&key_tok)?;
                let value_s = value.clone();
                let key_span = key_tok.span;
                let span_start = if is_async || is_generator {
                    prop_start
                } else {
                    key_span.start.0
                };
                self.bump();
                let key = ObjectKey::String(StringLit {
                    value: value_s,
                    span: key_span,
                });
                if self.check(&TokenKind::LParen) {
                    let method = self.parse_method_function(span_start, is_async, is_generator)?;
                    let end = expr_span(&method).end.0;
                    return Ok(ObjectProp::Property {
                        key,
                        value: method,
                        shorthand: false,
                        span: Span::new(span_start, end),
                    });
                }
                if is_async || is_generator {
                    return Err(Diagnostic::new(
                        "async/generator method requires `(params) { body }`".to_string(),
                        self.current_span(),
                    ));
                }
                self.expect(&TokenKind::Colon)?;
                let value = self.parse_assignment()?;
                let end = expr_span(&value).end.0;
                Ok(ObjectProp::Property {
                    key,
                    value,
                    shorthand: false,
                    span: Span::new(key_span.start.0, end),
                })
            }
            TokenKind::Number(raw) => {
                self.reject_legacy_octal_token(&key_tok)?;
                let name = numeric_literal_property_name(raw);
                let key_span = key_tok.span;
                let span_start = if is_async || is_generator {
                    prop_start
                } else {
                    key_span.start.0
                };
                self.bump();
                let key = ObjectKey::String(StringLit {
                    value: name.into(),
                    span: key_span,
                });
                if self.check(&TokenKind::LParen) {
                    let method = self.parse_method_function(span_start, is_async, is_generator)?;
                    let end = expr_span(&method).end.0;
                    return Ok(ObjectProp::Property {
                        key,
                        value: method,
                        shorthand: false,
                        span: Span::new(span_start, end),
                    });
                }
                if is_async || is_generator {
                    return Err(Diagnostic::new(
                        "async/generator method requires `(params) { body }`".to_string(),
                        self.current_span(),
                    ));
                }
                self.expect(&TokenKind::Colon)?;
                let value = self.parse_assignment()?;
                let end = expr_span(&value).end.0;
                Ok(ObjectProp::Property {
                    key,
                    value,
                    shorthand: false,
                    span: Span::new(key_span.start.0, end),
                })
            }
            _ => Err(Diagnostic::new(
                format!("expected property name, found {:?}", key_tok.kind),
                key_tok.span,
            )),
        }
    }

    /// Method body after a property key: `(params) { body }` → anonymous FunctionExpression.
    pub(crate) fn parse_method_function(
        &mut self,
        start: u32,
        is_async: bool,
        is_generator: bool,
    ) -> Result<Expr, Diagnostic> {
        self.expect(&TokenKind::LParen)?;
        self.with_ctx(
            |c| {
                c.in_generator = is_generator;
                c.in_await_context = is_async;
            },
            |p| {
                let params = p.parse_param_list()?;
                p.expect(&TokenKind::RParen)?;
                // E19.39: FormalParameters of a generator must not contain YieldExpression.
                if is_generator && params_contain_yield_expr(&params) {
                    return Err(Diagnostic::new(
                        "generator parameters cannot contain yield".to_string(),
                        Span::new(start, p.current_span().end.0),
                    ));
                }
                // E19.67: async method FormalParameters cannot contain AwaitExpression.
                if is_async && params_contain_await_expr(&params) {
                    return Err(Diagnostic::new(
                        "async function parameters cannot contain await".to_string(),
                        Span::new(start, p.current_span().end.0),
                    ));
                }
                let return_type = p.parse_optional_type_ann()?;
                // E19.67: methods introduce NewTarget and allow SuperProperty.
                p.ctx.new_target_depth += 1;
                p.ctx.super_property_depth += 1;
                let body = Box::new(p.parse_function_body_block()?);
                let end = stmt_span(&body).end.0;
                Ok(Expr::FunctionExpression {
                    name: None,
                    params,
                    return_type,
                    body,
                    is_async,
                    is_generator,
                    is_method: true,
                    span: Span::new(start, end),
                })
            },
        )
    }

    /// True when the next token can start a method name after `async` (`m`, keywords, `"m"`, `0`, `[`, `*`).
    pub(crate) fn peek_starts_method_name(&self) -> bool {
        let next = match self.tokens.get(self.pos + 1) {
            Some(t) => t,
            None => return false,
        };
        matches!(
            next.kind,
            TokenKind::Ident(_)
                | TokenKind::PrivateIdent(_)
                | TokenKind::String(_)
                | TokenKind::Number(_)
                | TokenKind::LBracket
                | TokenKind::Star
        ) || next.ident_name_opt().is_some()
    }

    /// True when next token starts an auto-accessor field name after `accessor` (no LineTerminator).
    pub(crate) fn peek_starts_accessor_field_name(&self) -> bool {
        let next = match self.tokens.get(self.pos + 1) {
            Some(t) => t,
            None => return false,
        };
        if next.preceded_by_line_terminator {
            return false;
        }
        matches!(
            next.kind,
            TokenKind::Ident(_)
                | TokenKind::PrivateIdent(_)
                | TokenKind::String(_)
                | TokenKind::Number(_)
                | TokenKind::LBracket
        ) || next.ident_name_opt().is_some()
    }

    /// True when current token is unescaped `get`/`set` and the next token starts an accessor name.
    /// Escaped `\u0067et` / `\u0073et` are IdentifierName, not the `get`/`set` terminal (E19.39).
    pub(crate) fn peek_accessor_kind(&self) -> Option<AccessorKind> {
        let cur = self.current();
        if cur.escaped {
            return None;
        }
        let kind = match &cur.kind {
            TokenKind::Ident(name) if name == "get" => AccessorKind::Get,
            TokenKind::Ident(name) if name == "set" => AccessorKind::Set,
            _ => return None,
        };
        let next = self.tokens.get(self.pos + 1)?;
        match &next.kind {
            TokenKind::Ident(_)
            | TokenKind::PrivateIdent(_)
            | TokenKind::String(_)
            | TokenKind::Number(_)
            | TokenKind::LBracket => Some(kind),
            _ if next.ident_name_opt().is_some() => Some(kind),
            _ => None,
        }
    }

    /// Object literal / accessor property key: IdentifierName (incl. keywords), string, number, or `[expr]`.
    pub(crate) fn parse_object_key(&mut self) -> Result<ObjectKey, Diagnostic> {
        let tok = self.current().clone();
        if let Some(name) = tok.ident_name_opt() {
            self.bump();
            return Ok(ObjectKey::Ident(Ident {
                name,
                span: tok.span,
            }));
        }
        match &tok.kind {
            TokenKind::String(value) => {
                self.reject_legacy_octal_token(&tok)?;
                let value = value.clone();
                self.bump();
                Ok(ObjectKey::String(StringLit {
                    value,
                    span: tok.span,
                }))
            }
            TokenKind::Number(raw) => {
                self.reject_legacy_octal_token(&tok)?;
                let name = numeric_literal_property_name(raw);
                self.bump();
                Ok(ObjectKey::String(StringLit {
                    value: name.into(),
                    span: tok.span,
                }))
            }
            TokenKind::LBracket => {
                self.bump();
                // ComputedPropertyName AssignmentExpression always allows `in`
                // (even when the surrounding cover is for-in `allow_in = false`) (E19.78).
                let expr = self.with_ctx(|c| c.allow_in = true, Self::parse_assignment)?;
                self.expect(&TokenKind::RBracket)?;
                Ok(ObjectKey::Computed(Box::new(expr)))
            }
            _ => Err(Diagnostic::new(
                format!("expected property name, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }
}

pub(crate) fn object_prop_is_proto_data(prop: &ObjectProp) -> bool {
    match prop {
        ObjectProp::Property {
            key,
            shorthand: false,
            value,
            ..
        } => {
            // Only `PropertyName : AssignmentExpression` form — not methods/shorthand.
            if matches!(
                value,
                Expr::FunctionExpression {
                    is_method: true,
                    ..
                }
            ) {
                return false;
            }
            match key {
                ObjectKey::Ident(id) => id.name == "__proto__",
                ObjectKey::String(s) => s.value.to_string_lossy() == "__proto__",
                ObjectKey::Computed(_) => false,
            }
        }
        _ => false,
    }
}

pub(crate) fn object_prop_span(prop: &ObjectProp) -> Span {
    match prop {
        ObjectProp::Property { span, .. }
        | ObjectProp::Spread { span, .. }
        | ObjectProp::Accessor { span, .. } => *span,
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn parse_object_literal_and_member() {
        let dump =
            parse_and_dump(r#"let o = { a: 1, "b": 2 }; let x = o.a; let y = o["b"];"#).unwrap();
        assert!(dump.contains("ObjectExpression"));
        assert!(dump.contains("key: Ident a"));
        assert!(dump.contains("key: String \"b\""));
        assert!(dump.contains("MemberExpression\n"));
        assert!(dump.contains("MemberExpression computed"));
    }

    #[test]
    fn parse_object_literal_sugar() {
        let dump =
            parse_and_dump("let a = 1; let k = \"z\"; let o = { a, m() { return 1; }, [k]: 2 };")
                .unwrap();
        assert!(dump.contains("prop shorthand:"));
        assert!(dump.contains("key: Ident a"));
        assert!(dump.contains("FunctionExpression"));
        assert!(dump.contains("key: Computed"));
        assert!(dump.contains("Ident k"));
    }

    #[test]
    fn parse_object_spread() {
        let dump = parse_and_dump("let a = { x: 1 }; let b = { ...a, y: 2, ...a };").unwrap();
        assert!(dump.contains("ObjectExpression"), "got:\n{dump}");
        assert!(dump.contains("spread:"), "got:\n{dump}");
        assert!(dump.contains("key: Ident y"), "got:\n{dump}");
    }

    #[test]
    fn parse_object_and_class_accessors() {
        let dump = parse_and_dump(
            "let o = { get x() { return 1; }, set x(v) { }, get [k]() { return 2; } }; class C { get n() { return 0; } set n(v) {} static get t() { return 1; } }",
        )
        .unwrap();
        assert!(dump.contains("accessor get:"), "{dump}");
        assert!(dump.contains("accessor set:"), "{dump}");
        assert!(dump.contains("key: Computed"), "{dump}");
        assert!(dump.contains("Accessor get"), "{dump}");
        assert!(dump.contains("Accessor set"), "{dump}");
        assert!(dump.contains("StaticAccessor get"), "{dump}");
    }

    /// E19.47: reserved words are valid IdentifierName method/accessor keys.
    #[test]
    fn parse_reserved_word_method_and_accessor_names() {
        let dump = parse_and_dump(
            r#"
            let o = {
              return() { return 1; },
              throw() {},
              await() {},
              get return() { return 1; },
              set throw(v) {},
              get await() { return 2; },
              async return() {},
              *throw() { yield 1; },
              async *await() { yield 1; }
            };
            class C {
              return() {}
              throw() {}
              get return() { return 1; }
              set throw(v) {}
              async await() {}
              *break() { yield 1; }
              async *continue() { yield 1; }
            }
            "#,
        )
        .unwrap();
        assert!(dump.contains("key: Ident return"), "{dump}");
        assert!(dump.contains("key: Ident throw"), "{dump}");
        assert!(dump.contains("key: Ident await"), "{dump}");
        assert!(dump.contains("name: break"), "{dump}");
        assert!(dump.contains("name: continue"), "{dump}");
        assert!(dump.contains("accessor get:"), "{dump}");
        assert!(dump.contains("accessor set:"), "{dump}");
        assert!(dump.contains("Accessor get"), "{dump}");
        assert!(dump.contains("Accessor set"), "{dump}");
        assert!(dump.contains("async: true"), "{dump}");
        assert!(dump.contains("generator: true"), "{dump}");
    }

    #[test]
    fn parse_async_methods() {
        let dump = parse_and_dump(
            "let o = { async m(x) { return await x; } }; class C { async n() { return 1; } static async s() { return 2; } }",
        )
        .unwrap();
        assert!(dump.contains("async: true"), "got:\n{dump}");
        assert!(dump.contains("FunctionExpression"), "got:\n{dump}");
        assert!(dump.contains("Method\n"), "got:\n{dump}");
        assert!(dump.contains("StaticMethod\n"), "got:\n{dump}");
        assert!(dump.contains("Unary await"), "got:\n{dump}");
    }
}
