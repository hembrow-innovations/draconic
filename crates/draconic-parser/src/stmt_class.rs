use super::*;

impl Parser {
    /// E19.78: parse and discard DecoratorList (`@dec …`). Syntax only for now.
    pub(crate) fn parse_decorator_list(&mut self) -> Result<(), Diagnostic> {
        while self.check(&TokenKind::At) {
            self.parse_decorator()?;
        }
        Ok(())
    }

    /// `@ DecoratorMemberExpression | @ DecoratorParenthesizedExpression | @ DecoratorCallExpression`
    pub(crate) fn parse_decorator(&mut self) -> Result<(), Diagnostic> {
        self.expect(&TokenKind::At)?;
        if self.check(&TokenKind::LParen) {
            // DecoratorParenthesizedExpression: `( Expression )`
            self.bump();
            let _ = self.parse_expr()?;
            self.expect(&TokenKind::RParen)?;
            return Ok(());
        }
        // DecoratorMemberExpression (+ optional Arguments for CallExpression).
        let _ = self.parse_decorator_member_expression()?;
        if self.check(&TokenKind::LParen) {
            self.bump();
            let _ = self.parse_arg_list()?;
            self.expect(&TokenKind::RParen)?;
        }
        Ok(())
    }

    /// IdentifierReference / `. IdentifierName` / `. PrivateIdentifier` chain.
    pub(crate) fn parse_decorator_member_expression(&mut self) -> Result<Expr, Diagnostic> {
        // Start with IdentifierReference (incl. yield/await when allowed as idents).
        let start_tok = self.current().clone();
        let mut expr = if let Some(name) = start_tok.ident_name_opt() {
            self.bump();
            Expr::Ident(Ident {
                name,
                span: start_tok.span,
            })
        } else if self.check(&TokenKind::Yield) && !self.ctx.in_generator {
            let sp = self.bump().span;
            Expr::Ident(Ident {
                name: "yield".into(),
                span: sp,
            })
        } else if self.check(&TokenKind::Await) && !self.ctx.in_await_context {
            let sp = self.bump().span;
            Expr::Ident(Ident {
                name: "await".into(),
                span: sp,
            })
        } else {
            return Err(Diagnostic::new(
                format!("expected decorator expression, found {:?}", start_tok.kind),
                start_tok.span,
            ));
        };
        while self.check(&TokenKind::Dot) {
            self.bump();
            let obj_start = expr_span(&expr).start.0;
            if let TokenKind::PrivateIdent(name) = &self.current().kind {
                let name = name.clone();
                let prop_span = self.bump().span;
                expr = Expr::MemberExpression {
                    object: Box::new(expr),
                    property: Box::new(Expr::Ident(Ident {
                        name,
                        span: prop_span,
                    })),
                    computed: false,
                    optional: false,
                    private: true,
                    span: Span::new(obj_start, prop_span.end.0),
                };
            } else {
                let (name, prop_span) = self.expect_ident_name()?;
                expr = Expr::MemberExpression {
                    object: Box::new(expr),
                    property: Box::new(Expr::Ident(Ident {
                        name,
                        span: prop_span,
                    })),
                    computed: false,
                    optional: false,
                    private: false,
                    span: Span::new(obj_start, prop_span.end.0),
                };
            }
        }
        Ok(expr)
    }

    /// `class Name extends Super? { constructor?(…) {…} method(…) {…} … }`
    pub(crate) fn parse_class_decl(&mut self) -> Result<Stmt, Diagnostic> {
        self.parse_class_decl_inner(false)
    }

    /// `class` declaration. When `default_export`, name may be omitted
    /// (`export default class extends …` / `export default class {…}`) (E19.54).
    pub(crate) fn parse_class_decl_inner(&mut self, default_export: bool) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Class)?.span.start.0;
        // Entire class is strict mode code (incl. BindingIdentifier name).
        self.with_ctx(
            |c| c.in_strict = true,
            |p| {
                let name = if default_export
                    && (p.check(&TokenKind::Extends) || p.check(&TokenKind::LBrace))
                {
                    // [+Default] class ClassTail — synthetic binding for ExportDefault local.
                    Ident {
                        name: "__class".into(),
                        span: Span::new(start, start),
                    }
                } else {
                    let name_tok = p.expect_ident()?;
                    let name = Ident {
                        name: name_tok.ident_name(),
                        span: name_tok.span,
                    };
                    // E19.49: class BindingIdentifier cannot be `eval`/`arguments`.
                    if is_strict_forbidden_binding_name(&name.name) {
                        return Err(Diagnostic::new(
                            format!("binding `{}` is invalid in strict mode", name.name),
                            name.span,
                        ));
                    }
                    name
                };
                let (super_class, body, end) = p.parse_class_tail()?;
                Ok(Stmt::ClassDeclaration {
                    name,
                    super_class,
                    body,
                    span: Span::new(start, end),
                })
            },
        )
    }

    /// `class Name? extends Super? { … }` in expression position (E18.33).
    pub(crate) fn parse_class_expression(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::Class)?.span.start.0;
        self.with_ctx(
            |c| c.in_strict = true,
            |p| {
                let name = if p.check(&TokenKind::Extends) || p.check(&TokenKind::LBrace) {
                    None
                } else {
                    let name_tok = p.expect_ident()?;
                    let id = Ident {
                        name: name_tok.ident_name(),
                        span: name_tok.span,
                    };
                    // E19.49: class BindingIdentifier cannot be `eval`/`arguments`.
                    if is_strict_forbidden_binding_name(&id.name) {
                        return Err(Diagnostic::new(
                            format!("binding `{}` is invalid in strict mode", id.name),
                            id.span,
                        ));
                    }
                    Some(id)
                };
                let (super_class, body, end) = p.parse_class_tail()?;
                Ok(Expr::ClassExpression {
                    name,
                    super_class,
                    body,
                    span: Span::new(start, end),
                })
            },
        )
    }

    /// `extends Super? { elements… }` shared by class declaration and expression.
    pub(crate) fn parse_class_tail(
        &mut self,
    ) -> Result<(Option<Box<Expr>>, Vec<ClassElement>, u32), Diagnostic> {
        let super_class = if self.check(&TokenKind::Extends) {
            self.bump();
            Some(Box::new(self.parse_lhs()?))
        } else {
            None
        };
        self.expect(&TokenKind::LBrace)?;
        let (body, end) = self.with_ctx(
            |c| {
                // Class bodies are always strict (ECMA-262).
                c.in_strict = true;
                // Nested classes inherit outer private names (E19.36). Push a frame so
                // nested class validation sees names declared so far in this class.
                c.class_private_stack.push(Vec::new());
            },
            |p| {
                let mut body = Vec::new();
                while !p.check(&TokenKind::RBrace) && !p.check(&TokenKind::Eof) {
                    // Empty ClassElement: lone `;` (ECMA-262 ClassElement → `;`).
                    if p.check(&TokenKind::Semi) {
                        p.bump();
                        continue;
                    }
                    let el = p.parse_class_element()?;
                    // Register private names immediately so nested classes in later
                    // initializers can reference them.
                    if let Some(frame) = p.ctx.class_private_stack.last_mut() {
                        register_private_names_from_element(&el, frame);
                    }
                    let needs_field_semi = matches!(&el, ClassElement::Field { .. });
                    body.push(el);
                    // FieldDefinition requires `;` (explicit or ASI). Methods end at `}`.
                    if needs_field_semi {
                        if p.check(&TokenKind::Semi) {
                            p.bump();
                        } else if p.check(&TokenKind::RBrace) || p.check(&TokenKind::Eof) {
                            // ASI before `}` / EOF
                        } else if p.current().preceded_by_line_terminator {
                            // ASI across LineTerminator
                        } else {
                            return Err(Diagnostic::new(
                                "expected ';' after class field".to_string(),
                                p.current_span(),
                            ));
                        }
                    } else if p.check(&TokenKind::Semi) {
                        p.bump();
                    }
                }
                let end = p.expect(&TokenKind::RBrace)?.span.end.0;
                let has_heritage = super_class.is_some();
                let inherited = p.all_class_private_names();
                validate_class_body(&body, has_heritage, &inherited)?;
                Ok((body, end))
            },
        )?;
        Ok((super_class, body, end))
    }

    /// Class field Initializer is always [~Await] (E19.52 / FieldDefinition).
    /// SuperProperty is allowed (E19.82.05); SuperCall still rejected by ClassBody early errors.
    pub(crate) fn parse_optional_class_field_init(&mut self) -> Result<Option<Expr>, Diagnostic> {
        if !self.check(&TokenKind::Eq) {
            return Ok(None);
        }
        self.bump();
        self.with_ctx(
            |c| {
                c.in_await_context = false;
                c.super_property_depth += 1;
            },
            |p| Ok(Some(p.parse_assignment()?)),
        )
    }

    pub(crate) fn parse_class_element(&mut self) -> Result<ClassElement, Diagnostic> {
        // E19.78: optional DecoratorList before class element (discarded; syntax only).
        if self.check(&TokenKind::At) {
            self.parse_decorator_list()?;
        }
        let start = self.current_span().start.0;
        // `static` is a keyword only when followed by a ClassElement; `static;` / `static =`
        // is an instance field named "static" (E19.82.04 / IdentifierName).
        let is_static = if self.check(&TokenKind::Static) {
            let next = self.tokens.get(self.pos + 1).map(|t| &t.kind);
            match next {
                Some(TokenKind::Semi) | Some(TokenKind::Eq) => false,
                _ => {
                    self.bump();
                    true
                }
            }
        } else {
            false
        };
        // `static { … }` static initialization block (E18.41). Body is [+Await] (E19.52).
        if is_static && self.check(&TokenKind::LBrace) {
            let body = Box::new(self.with_ctx(
                |c| {
                    c.in_await_context = true;
                    // E19.67: static blocks introduce NewTarget and allow SuperProperty.
                    c.new_target_depth += 1;
                    c.super_property_depth += 1;
                },
                Self::parse_block,
            )?);
            let end = stmt_span(&body).end.0;
            return Ok(ClassElement::StaticBlock {
                body,
                span: Span::new(start, end),
            });
        }
        // Auto-accessor field: `accessor name;` / `accessor name = expr;` (no LineTerminator after `accessor`).
        if matches!(self.current().kind, TokenKind::Ident(ref n) if n == "accessor")
            && self.peek_starts_accessor_field_name()
        {
            self.bump(); // consume `accessor`
            let (key, is_private) = if let TokenKind::PrivateIdent(pname) = &self.current().kind {
                let pname = pname.clone();
                let name_tok = self.bump();
                (
                    ObjectKey::Ident(Ident {
                        name: pname,
                        span: name_tok.span,
                    }),
                    true,
                )
            } else {
                (self.parse_object_key()?, false)
            };
            let key_span = object_key_span(&key);
            let value = self.parse_optional_class_field_init()?;
            let end = value
                .as_ref()
                .map(|v| expr_span(v).end.0)
                .unwrap_or(key_span.end.0);
            let span = Span::new(start, end);
            if !is_private && class_key_is_literal_constructor(&key) {
                return Err(Diagnostic::new(
                    "class field cannot be named constructor".to_string(),
                    span,
                ));
            }
            // Lower as a public/private field for now (auto-accessor semantics deferred).
            return Ok(ClassElement::Field {
                key,
                value,
                is_static,
                is_private,
                span,
            });
        }
        // `get name()` / `set name(v)` / `get #name()` / `set #name(v)` / `get [expr]()` (not `get()` method)
        if let Some(kind) = self.peek_accessor_kind() {
            self.bump(); // consume get/set
            let (key, is_private) = if let TokenKind::PrivateIdent(pname) = &self.current().kind {
                let pname = pname.clone();
                let name_tok = self.bump();
                (
                    ObjectKey::Ident(Ident {
                        name: pname,
                        span: name_tok.span,
                    }),
                    true,
                )
            } else {
                (self.parse_object_key()?, false)
            };
            let key_span = object_key_span(&key);
            self.expect(&TokenKind::LParen)?;
            // Accessors are ordinary methods: [~Await] params/body (E19.52).
            return self.with_ctx(
                |c| c.in_await_context = false,
                |p| {
                    let params = p.parse_param_list()?;
                    p.expect(&TokenKind::RParen)?;
                    if kind == AccessorKind::Get && !params.is_empty() {
                        return Err(Diagnostic::new(
                            "getter must have zero parameters".to_string(),
                            key_span,
                        ));
                    }
                    if kind == AccessorKind::Set && params.len() != 1 {
                        return Err(Diagnostic::new(
                            "setter must have exactly one parameter".to_string(),
                            key_span,
                        ));
                    }
                    // E19.67: accessors introduce NewTarget and allow SuperProperty.
                    p.ctx.new_target_depth += 1;
                    p.ctx.super_property_depth += 1;
                    let body = Box::new(p.parse_function_body_block()?);
                    let end = stmt_span(&body).end.0;
                    Ok(ClassElement::Accessor {
                        kind,
                        key,
                        params,
                        body,
                        is_static,
                        is_private,
                        span: Span::new(start, end),
                    })
                },
            );
        }
        // `async m()` / `async *m()` / `async #m()` / `async [e]()` — not method/field named `async`.
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
        // Private field/method: `#name;` / `#name = expr;` / `static? #name(…){…}` (E18.35 / E18.37 / E18.38).
        if let TokenKind::PrivateIdent(pname) = &self.current().kind {
            let pname = pname.clone();
            let name_tok = self.bump();
            let name = Ident {
                name: pname,
                span: name_tok.span,
            };
            if is_async || is_generator || self.check(&TokenKind::LParen) {
                self.expect(&TokenKind::LParen)?;
                return self.with_ctx(
                    |c| {
                        c.in_generator = is_generator;
                        c.in_await_context = is_async;
                    },
                    |p| {
                        let params = p.parse_param_list()?;
                        p.expect(&TokenKind::RParen)?;
                        if is_generator && params_contain_yield_expr(&params) {
                            return Err(Diagnostic::new(
                                "generator parameters cannot contain yield".to_string(),
                                Span::new(start, p.current_span().end.0),
                            ));
                        }
                        if is_async && params_contain_await_expr(&params) {
                            return Err(Diagnostic::new(
                                "async function parameters cannot contain await".to_string(),
                                Span::new(start, p.current_span().end.0),
                            ));
                        }
                        p.ctx.new_target_depth += 1;
                        p.ctx.super_property_depth += 1;
                        let body = Box::new(p.parse_function_body_block()?);
                        let end = stmt_span(&body).end.0;
                        Ok(ClassElement::Method {
                            key: ObjectKey::Ident(name),
                            params,
                            body,
                            is_static,
                            is_async,
                            is_generator,
                            is_private: true,
                            span: Span::new(start, end),
                        })
                    },
                );
            }
            let value = self.parse_optional_class_field_init()?;
            let end = value
                .as_ref()
                .map(|v| expr_span(v).end.0)
                .unwrap_or(name.span.end.0);
            let span = Span::new(start, end);
            return Ok(ClassElement::Field {
                key: ObjectKey::Ident(name),
                value,
                is_static,
                is_private: true,
                span,
            });
        }
        let key = self.parse_object_key()?;
        let key_span = object_key_span(&key);
        // Public field: `name;` / `name = expr;` / `[e];` / `[e] = expr;` (not a method/constructor).
        if !is_async && !is_generator && !self.check(&TokenKind::LParen) {
            let value = self.parse_optional_class_field_init()?;
            let end = value
                .as_ref()
                .map(|v| expr_span(v).end.0)
                .unwrap_or(key_span.end.0);
            let span = Span::new(start, end);
            if class_key_is_literal_constructor(&key) {
                return Err(Diagnostic::new(
                    "class field cannot be named constructor".to_string(),
                    span,
                ));
            }
            return Ok(ClassElement::Field {
                key,
                value,
                is_static,
                is_private: false,
                span,
            });
        }
        self.expect(&TokenKind::LParen)?;
        self.with_ctx(
            |c| {
                c.in_generator = is_generator;
                c.in_await_context = is_async;
            },
            |p| {
                let params = p.parse_param_list()?;
                p.expect(&TokenKind::RParen)?;
                // E19.39: generator FormalParameters cannot contain YieldExpression.
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
                p.ctx.new_target_depth += 1;
                p.ctx.super_property_depth += 1;
                let body = Box::new(p.parse_function_body_block()?);
                let end = stmt_span(&body).end.0;
                let span = Span::new(start, end);
                // Only non-static literal IdentifierName `constructor` is the constructor.
                // `static constructor` / `static *constructor` / `static async constructor` are ordinary methods (E19.53).
                // computed/`"constructor"` are always methods.
                if class_key_is_literal_constructor(&key) && !is_static {
                    if is_async {
                        return Err(Diagnostic::new(
                            "class constructor cannot be async".to_string(),
                            span,
                        ));
                    }
                    if is_generator {
                        return Err(Diagnostic::new(
                            "class constructor cannot be a generator".to_string(),
                            span,
                        ));
                    }
                    Ok(ClassElement::Constructor { params, body, span })
                } else {
                    Ok(ClassElement::Method {
                        key,
                        params,
                        body,
                        is_static,
                        is_async,
                        is_generator,
                        is_private: false,
                        span,
                    })
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn parse_class_static_block() {
        let dump = parse_and_dump("class C { static { this.x = 1; } static y = 2; }\n").unwrap();
        assert!(
            dump.contains("StaticBlock") && dump.contains("StaticField"),
            "expected static block + field, got:\n{dump}"
        );
        let multi =
            parse_and_dump("class C { static { let a = 1; } static { let b = 2; } }\n").unwrap();
        assert_eq!(
            multi.matches("StaticBlock").count(),
            2,
            "two static blocks, got:\n{multi}"
        );
    }

    #[test]
    fn parse_class_computed_property_names() {
        let dump = parse_and_dump(
            "class C {\n\
               ['m']() { return 1; }\n\
               *[g]() { yield 2; }\n\
               async [a]() { return 3; }\n\
               get [x]() { return 4; }\n\
               set [y](v) { this._ = v; }\n\
               static ['s']() { return 5; }\n\
               static get [sg]() { return 6; }\n\
               static async *[sag]() { yield 7; }\n\
             }\n",
        )
        .unwrap();
        assert!(
            dump.contains("key: Computed")
                && dump.contains("Method")
                && dump.contains("generator: true")
                && dump.contains("async: true")
                && dump.contains("Accessor get")
                && dump.contains("Accessor set")
                && dump.contains("StaticMethod")
                && dump.contains("StaticAccessor get"),
            "expected class computed methods/accessors, got:\n{dump}"
        );
        assert!(
            dump.matches("key: Computed").count() >= 7,
            "expected multiple computed keys, got:\n{dump}"
        );
    }

    /// E19.34: numeric LiteralPropertyName on class methods/fields/accessors.
    #[test]
    fn parse_class_numeric_property_names() {
        let dump = parse_and_dump(
            "class C {\n\
               0 = 'bar';\n\
               1() { return 1; }\n\
               get 2() { return 2; }\n\
               set 3(_) {}\n\
               static 0x10() { return 16; }\n\
             }\n",
        )
        .unwrap();
        assert!(
            dump.contains("key: String \"0\"")
                && dump.contains("key: String \"1\"")
                && dump.contains("key: String \"2\"")
                && dump.contains("key: String \"3\"")
                && dump.contains("key: String \"16\""),
            "expected numeric PropNames as strings, got:\n{dump}"
        );
    }

    /// E19.78: non-canonical numeric accessor names use ToString(MV) (`0.0000001` → `1e-7`).
    #[test]
    fn parse_class_numeric_accessor_non_canonical() {
        let dump = parse_and_dump("class C { get 0.0000001() { return 1; } }\n").unwrap();
        assert!(
            dump.contains("key: String \"1e-7\""),
            "expected ToString key 1e-7, got:\n{dump}"
        );
    }

    /// E19.78: decorator `@` syntax on class declarations (parsed, discarded).
    #[test]
    fn parse_class_decorators() {
        let dump = parse_and_dump("function dec() {}\n@dec class C {}\n").unwrap();
        assert!(
            dump.contains("ClassDeclaration") && dump.contains("name: C"),
            "expected decorated class decl, got:\n{dump}"
        );
        let expr = parse_and_dump("function dec() {}\nvar C = @dec class {};\n").unwrap();
        assert!(
            expr.contains("ClassExpression"),
            "expected decorated class expr, got:\n{expr}"
        );
    }

    /// E19.34: field ASI required; SuperCall/arguments/dups/constructor/prototype early errors.
    #[test]
    fn parse_class_element_early_errors() {
        assert!(
            parse_and_dump("class C { field method(){} }\n").is_err(),
            "same-line field then method without ';' must fail"
        );
        assert!(
            parse_and_dump("class C { x = super(); }\n").is_err(),
            "SuperCall in field init must fail"
        );
        assert!(
            parse_and_dump("class C { x = () => super(); }\n").is_err(),
            "SuperCall in arrow field init must fail"
        );
        assert!(
            parse_and_dump("class C { x = () => arguments; }\n").is_err(),
            "arguments in arrow field init must fail"
        );
        assert!(
            parse_and_dump("class C { #x; #x; }\n").is_err(),
            "duplicate private field must fail"
        );
        assert!(
            parse_and_dump("class C { 'constructor'; }\n").is_err(),
            "string field named constructor must fail"
        );
        assert!(
            parse_and_dump("class C { static prototype; }\n").is_err(),
            "static field named prototype must fail"
        );
        // ASI with newline is OK
        let ok = parse_and_dump("class C { field\nmethod(){} }\n").unwrap();
        assert!(
            ok.contains("name: field") && ok.contains("name: method"),
            "newline ASI field then method ok, got:\n{ok}"
        );
    }

    /// E19.29: empty ClassElement `;` and same-line fields after methods (ASI / explicit).
    #[test]
    fn parse_static_as_instance_field_name() {
        // E19.82.04: `static;` / `static = expr` is a field named "static".
        let bare = parse_and_dump("class C { static; }\n").unwrap();
        assert!(
            bare.contains("Field") && bare.contains("static") && !bare.contains("StaticField"),
            "expected instance field named static, got:\n{bare}"
        );
        let assigned = parse_and_dump("class C { static = \"foo\"; }\n").unwrap();
        assert!(
            assigned.contains("Field") && !assigned.contains("StaticField"),
            "expected instance field static=, got:\n{assigned}"
        );
        let both = parse_and_dump("class C { static static; }\n").unwrap();
        assert!(
            both.contains("StaticField"),
            "expected static field named static, got:\n{both}"
        );
    }

    #[test]
    fn parse_class_empty_element_and_same_line_fields() {
        let empty = parse_and_dump("class C { ; }\n").unwrap();
        assert!(
            empty.contains("ClassDeclaration") && !empty.contains("Field"),
            "lone `;` is empty ClassElement, got:\n{empty}"
        );
        let double = parse_and_dump("class C { a;; }\n").unwrap();
        assert!(
            double.contains("Field") && double.contains("name: a"),
            "field then empty `;`, got:\n{double}"
        );
        let same_line = parse_and_dump(
            "class C {\n\
               *m() { return 42; } a; b = 42;\n\
               c = 1;\n\
             }\n",
        )
        .unwrap();
        assert!(
            same_line.contains("generator: true")
                && same_line.contains("name: a")
                && same_line.contains("name: b")
                && same_line.contains("name: c"),
            "fields after same-line generator, got:\n{same_line}"
        );
        let asi = parse_and_dump(
            "class C {\n\
               *m() { return 42; } a\n\
               b = 42;;\n\
             }\n",
        )
        .unwrap();
        assert!(
            asi.contains("name: a") && asi.contains("name: b") && asi.contains("Number 42"),
            "ASI field after method + trailing empty `;`, got:\n{asi}"
        );
        let privates = parse_and_dump(
            "class C {\n\
               *m() { return 42; } #x; #y;\n\
             }\n",
        )
        .unwrap();
        assert!(
            privates.contains("PrivateField")
                && privates.contains("name: #x")
                && privates.contains("name: #y"),
            "private fields after same-line generator, got:\n{privates}"
        );
    }

    /// E19.36: `delete` of private member reference is early SyntaxError.
    #[test]
    fn parse_delete_private_member_early_error() {
        assert!(
            parse_and_dump("class C { #x; m() { delete this.#x; } }\n").is_err(),
            "delete this.#x must fail"
        );
        assert!(
            parse_and_dump("class C { #x; m() { delete (this.#x); } }\n").is_err(),
            "delete (this.#x) must fail"
        );
        assert!(
            parse_and_dump("class C { #x; m() { delete ((this.#x)); } }\n").is_err(),
            "delete ((this.#x)) must fail"
        );
        // Public delete still parses.
        assert!(
            parse_and_dump("class C { m() { delete this.x; } }\n").is_ok(),
            "delete this.x must still parse"
        );
    }
}
