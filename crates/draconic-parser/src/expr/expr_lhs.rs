use crate::*;

impl Parser {
    pub(crate) fn parse_lhs(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = if self.check(&TokenKind::New) {
            self.parse_new()?
        } else if self.check(&TokenKind::Import) && self.is_import_call_start() {
            self.parse_import_call()?
        } else if self.check(&TokenKind::Import) && self.is_import_meta_start() {
            self.parse_import_meta()?
        } else {
            self.parse_primary()?
        };
        loop {
            if self.check(&TokenKind::LParen) {
                self.bump();
                let args = self.parse_arg_list()?;
                let end = self.expect(&TokenKind::RParen)?.span.end.0;
                let start = expr_span(&expr).start.0;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    optional: false,
                    span: Span::new(start, end),
                };
            } else if self.check(&TokenKind::QuestionDot) {
                self.bump();
                let start = expr_span(&expr).start.0;
                // E19.67: optional chain cannot be followed by tagged template.
                if matches!(
                    &self.current().kind,
                    TokenKind::TemplateNoSubstitution(_) | TokenKind::TemplateHead(_)
                ) {
                    return Err(Diagnostic::new(
                        "tagged template not allowed after optional chain".to_string(),
                        self.current_span(),
                    ));
                }
                if self.check(&TokenKind::LParen) {
                    self.bump();
                    let args = self.parse_arg_list()?;
                    let end = self.expect(&TokenKind::RParen)?.span.end.0;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                        optional: true,
                        span: Span::new(start, end),
                    };
                } else if self.check(&TokenKind::LBracket) {
                    self.bump();
                    let property = self.parse_expr()?;
                    let end = self.expect(&TokenKind::RBracket)?.span.end.0;
                    expr = Expr::MemberExpression {
                        object: Box::new(expr),
                        property: Box::new(property),
                        computed: true,
                        optional: true,
                        private: false,
                        span: Span::new(start, end),
                    };
                } else if let TokenKind::PrivateIdent(name) = &self.current().kind {
                    let name = name.clone();
                    let prop_span = self.bump().span;
                    let end = prop_span.end.0;
                    let property = Expr::Ident(Ident {
                        name,
                        span: prop_span,
                    });
                    expr = Expr::MemberExpression {
                        object: Box::new(expr),
                        property: Box::new(property),
                        computed: false,
                        optional: true,
                        private: true,
                        span: Span::new(start, end),
                    };
                } else {
                    let (name, prop_span) = self.expect_ident_name()?;
                    let end = prop_span.end.0;
                    let property = Expr::Ident(Ident {
                        name,
                        span: prop_span,
                    });
                    expr = Expr::MemberExpression {
                        object: Box::new(expr),
                        property: Box::new(property),
                        computed: false,
                        optional: true,
                        private: false,
                        span: Span::new(start, end),
                    };
                }
            } else if self.check(&TokenKind::Dot) {
                self.bump();
                let start = expr_span(&expr).start.0;
                if let TokenKind::PrivateIdent(name) = &self.current().kind {
                    let name = name.clone();
                    let prop_span = self.bump().span;
                    let end = prop_span.end.0;
                    let property = Expr::Ident(Ident {
                        name,
                        span: prop_span,
                    });
                    expr = Expr::MemberExpression {
                        object: Box::new(expr),
                        property: Box::new(property),
                        computed: false,
                        optional: false,
                        private: true,
                        span: Span::new(start, end),
                    };
                } else {
                    let (name, prop_span) = self.expect_ident_name()?;
                    let end = prop_span.end.0;
                    let property = Expr::Ident(Ident {
                        name,
                        span: prop_span,
                    });
                    expr = Expr::MemberExpression {
                        object: Box::new(expr),
                        property: Box::new(property),
                        computed: false,
                        optional: false,
                        private: false,
                        span: Span::new(start, end),
                    };
                }
            } else if self.check(&TokenKind::LBracket) {
                self.bump();
                let property = self.parse_expr()?;
                let end = self.expect(&TokenKind::RBracket)?.span.end.0;
                let start = expr_span(&expr).start.0;
                expr = Expr::MemberExpression {
                    object: Box::new(expr),
                    property: Box::new(property),
                    computed: true,
                    optional: false,
                    private: false,
                    span: Span::new(start, end),
                };
            } else if matches!(
                &self.current().kind,
                TokenKind::TemplateNoSubstitution(_) | TokenKind::TemplateHead(_)
            ) {
                // E19.67: OptionalExpression cannot be the tag of a tagged template.
                if expr_is_optional_chain(&expr) {
                    return Err(Diagnostic::new(
                        "tagged template not allowed after optional chain".to_string(),
                        self.current_span(),
                    ));
                }
                expr = self.parse_tagged_template(expr)?;
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// `import(…)`, `import.defer(…)`, or `import.source(…)` lookahead from `import`.
    pub(crate) fn is_import_call_start(&self) -> bool {
        if self.peek_is(&TokenKind::LParen) {
            return true;
        }
        // `import . (defer|source) (`
        if !self.peek_is(&TokenKind::Dot) {
            return false;
        }
        let Some(phase_tok) = self.tokens.get(self.pos + 2) else {
            return false;
        };
        let phase_ok = match &phase_tok.kind {
            TokenKind::Ident(name) => name == "defer" || name == "source",
            _ => false,
        };
        if !phase_ok {
            return false;
        }
        matches!(
            self.tokens.get(self.pos + 3).map(|t| &t.kind),
            Some(TokenKind::LParen)
        )
    }

    /// `import.meta` lookahead from `import`.
    pub(crate) fn is_import_meta_start(&self) -> bool {
        if !self.peek_is(&TokenKind::Dot) {
            return false;
        }
        matches!(
            self.tokens.get(self.pos + 2).map(|t| &t.kind),
            Some(TokenKind::Ident(name)) if name == "meta"
        )
    }

    /// `import.meta` meta-property (Module goal only; E19.83.01).
    pub(crate) fn parse_import_meta(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::Import)?.span.start.0;
        self.expect(&TokenKind::Dot)?;
        let meta_escaped = self.current().escaped;
        let (name, prop_span) = self.expect_ident_name()?;
        if meta_escaped {
            return Err(Diagnostic::new(
                "escaped 'meta' is not allowed in import.meta".to_string(),
                prop_span,
            ));
        }
        if name != "meta" {
            return Err(Diagnostic::new(
                format!("expected `meta` after `import.`, found `{name}`"),
                prop_span,
            ));
        }
        if !self.ctx.is_module {
            return Err(Diagnostic::new(
                "'import.meta' is only valid in modules".to_string(),
                Span::new(start, prop_span.end.0),
            ));
        }
        Ok(Expr::ImportMeta {
            span: Span::new(start, prop_span.end.0),
        })
    }

    /// `import(AssignmentExpression)` / `import(AssignmentExpression, options)` /
    /// `import.defer(AssignmentExpression)` / `import.source(AssignmentExpression)`.
    /// Rest args and empty argument lists are early SyntaxErrors.
    /// Phase forms (`defer` / `source`) accept only one argument (no options).
    pub(crate) fn parse_import_call(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::Import)?.span.start.0;
        let phase = if self.check(&TokenKind::Dot) {
            self.bump();
            let (name, prop_span) = self.expect_ident_name()?;
            match name.as_str() {
                "defer" => ImportPhase::Defer,
                "source" => ImportPhase::Source,
                _ => {
                    return Err(Diagnostic::new(
                        format!("expected `defer` or `source` after `import.`, found `{name}`"),
                        prop_span,
                    ));
                }
            }
        } else {
            ImportPhase::Evaluation
        };
        self.expect(&TokenKind::LParen)?;
        if self.check(&TokenKind::RParen) {
            return Err(Diagnostic::new(
                "ImportCall requires a module specifier argument",
                self.current_span(),
            ));
        }
        if self.check(&TokenKind::DotDotDot) {
            return Err(Diagnostic::new(
                "ImportCall does not allow rest arguments",
                self.current_span(),
            ));
        }
        let source = self.parse_assignment()?;
        let mut options = None;
        if self.check(&TokenKind::Comma) {
            self.bump();
            if !self.check(&TokenKind::RParen) {
                if phase != ImportPhase::Evaluation {
                    return Err(Diagnostic::new(
                        "ImportCall with defer/source accepts at most one argument",
                        self.current_span(),
                    ));
                }
                if self.check(&TokenKind::DotDotDot) {
                    return Err(Diagnostic::new(
                        "ImportCall does not allow rest arguments",
                        self.current_span(),
                    ));
                }
                options = Some(Box::new(self.parse_assignment()?));
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    if !self.check(&TokenKind::RParen) {
                        return Err(Diagnostic::new(
                            "ImportCall accepts at most two arguments",
                            self.current_span(),
                        ));
                    }
                }
            }
        }
        let end = self.expect(&TokenKind::RParen)?.span.end.0;
        Ok(Expr::ImportCall {
            phase,
            source: Box::new(source),
            options,
            span: Span::new(start, end),
        })
    }

    /// `new.target` meta-property, or `new callee` / `new callee(args)`.
    pub(crate) fn parse_new(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::New)?.span.start.0;
        // `new.target` — meta-property, not a construct expression.
        if self.check(&TokenKind::Dot) {
            self.bump();
            // E19.67: `new.t\u0061rget` — IdentifierName must be unescaped `target`.
            let target_escaped = self.current().escaped;
            let (name, prop_span) = self.expect_ident_name()?;
            if target_escaped {
                return Err(Diagnostic::new(
                    "escaped 'target' is not allowed in new.target".to_string(),
                    prop_span,
                ));
            }
            if name != "target" {
                return Err(Diagnostic::new(
                    format!("expected `target` after `new.`, found `{name}`"),
                    prop_span,
                ));
            }
            // E19.67: NewTarget only in non-arrow function / method / static block code.
            if self.ctx.new_target_depth == 0 {
                return Err(Diagnostic::new(
                    "'new.target' is only valid inside functions".to_string(),
                    Span::new(start, prop_span.end.0),
                ));
            }
            return Ok(Expr::NewTarget {
                span: Span::new(start, prop_span.end.0),
            });
        }
        let mut callee = if self.check(&TokenKind::New) {
            self.parse_new()?
        } else {
            self.parse_primary()?
        };
        // Member chain on the constructed callee (not calls — those bind to outer `new` args).
        loop {
            if self.check(&TokenKind::Dot) {
                self.bump();
                let cstart = expr_span(&callee).start.0;
                if let TokenKind::PrivateIdent(name) = &self.current().kind {
                    let name = name.clone();
                    let prop_span = self.bump().span;
                    let end = prop_span.end.0;
                    let property = Expr::Ident(Ident {
                        name,
                        span: prop_span,
                    });
                    callee = Expr::MemberExpression {
                        object: Box::new(callee),
                        property: Box::new(property),
                        computed: false,
                        optional: false,
                        private: true,
                        span: Span::new(cstart, end),
                    };
                } else {
                    let (name, prop_span) = self.expect_ident_name()?;
                    let end = prop_span.end.0;
                    let property = Expr::Ident(Ident {
                        name,
                        span: prop_span,
                    });
                    callee = Expr::MemberExpression {
                        object: Box::new(callee),
                        property: Box::new(property),
                        computed: false,
                        optional: false,
                        private: false,
                        span: Span::new(cstart, end),
                    };
                }
            } else if self.check(&TokenKind::LBracket) {
                self.bump();
                let property = self.parse_expr()?;
                let end = self.expect(&TokenKind::RBracket)?.span.end.0;
                let cstart = expr_span(&callee).start.0;
                callee = Expr::MemberExpression {
                    object: Box::new(callee),
                    property: Box::new(property),
                    computed: true,
                    optional: false,
                    private: false,
                    span: Span::new(cstart, end),
                };
            } else {
                break;
            }
        }
        let (args, end) = if self.check(&TokenKind::LParen) {
            self.bump();
            let args = self.parse_arg_list()?;
            let end = self.expect(&TokenKind::RParen)?.span.end.0;
            (args, end)
        } else {
            (Vec::new(), expr_span(&callee).end.0)
        };
        Ok(Expr::New {
            callee: Box::new(callee),
            args,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_arg_list(&mut self) -> Result<Vec<Arg>, Diagnostic> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                if self.check(&TokenKind::DotDotDot) {
                    self.bump();
                    args.push(Arg::Spread(self.parse_assignment()?));
                } else {
                    args.push(Arg::Expr(self.parse_assignment()?));
                }
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    if self.check(&TokenKind::RParen) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        Ok(args)
    }
}

/// True when `expr` is (or chains through) an optional chain (`?.`).
pub(crate) fn expr_is_optional_chain(expr: &Expr) -> bool {
    match expr {
        Expr::MemberExpression { optional: true, .. } | Expr::Call { optional: true, .. } => true,
        Expr::MemberExpression {
            optional: false,
            object,
            ..
        } => expr_is_optional_chain(object),
        Expr::Call {
            optional: false,
            callee,
            ..
        } => expr_is_optional_chain(callee),
        Expr::Paren { expr: inner, .. } => expr_is_optional_chain(inner),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

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
    fn parse_member_keyword_ident_name() {
        // IdentifierName after `.` may be a reserved word (e.g. Symbol.for).
        let dump = parse_and_dump("Symbol.for; obj.default;").unwrap();
        assert!(dump.contains("Ident for"), "got:\n{dump}");
        assert!(dump.contains("Ident default"), "got:\n{dump}");
    }

    #[test]
    fn parse_call_spread() {
        let dump = parse_and_dump("f(...a); g(1, ...b, 2); new C(...a);").unwrap();
        assert!(dump.contains("arg[0] spread:"));
        assert!(dump.contains("arg[0]:\n        Number 1"));
        assert!(dump.contains("arg[1] spread:"));
        assert!(dump.contains("New\n"));
    }

    #[test]
    fn parse_call_args_trailing_comma() {
        // E19.21: trailing comma in Arguments (call and new).
        let dump =
            parse_and_dump("f(a,); g(a, b,); h(...a,); i(1, ...b,); new C(x,); new D(...y,);")
                .unwrap();
        assert!(dump.contains("Call\n"), "got:\n{dump}");
        assert!(dump.contains("New\n"), "got:\n{dump}");
        assert!(dump.contains("arg[0]:\n        Ident a"), "got:\n{dump}");
        assert!(dump.contains("arg[0] spread:"), "got:\n{dump}");
        assert!(dump.contains("arg[1]:\n        Ident b"), "got:\n{dump}");
        // Trailing comma must not invent an extra empty arg.
        assert!(
            !dump.contains("arg[2]"),
            "trailing comma must not add args, got:\n{dump}"
        );
    }

    #[test]
    fn parse_property_assignment() {
        let dump = parse_and_dump(r#"o.a = 1; o["b"] = 2;"#).unwrap();
        assert!(dump.contains("Assign ="));
        assert!(dump.contains("MemberExpression\n"));
        assert!(dump.contains("MemberExpression computed"));
        assert!(dump.contains("Number 1"));
        assert!(dump.contains("Number 2"));
    }

    #[test]
    fn parse_this_and_method_call() {
        let dump = parse_and_dump("let o = { m: function () { return this.x; } }; o.m();").unwrap();
        assert!(dump.contains("This"));
        assert!(dump.contains("MemberExpression"));
        assert!(dump.contains("Call"));
    }

    #[test]
    fn parse_new_expression() {
        let dump =
            parse_and_dump("let p = new Point(1, 2); let q = new Foo; let x = new A().b;").unwrap();
        assert!(dump.contains("New\n"));
        assert!(dump.contains("arg[0]:"));
        assert!(dump.contains("MemberExpression"));
    }

    #[test]
    fn parse_new_target() {
        let dump = parse_and_dump("function f() { return new.target; }").unwrap();
        assert!(dump.contains("NewTarget\n"), "{dump}");
        assert!(!dump.contains("MemberExpression"), "{dump}");
    }

    #[test]
    fn parse_import_call() {
        let dump = parse_and_dump("let p = import('./m.js');").unwrap();
        assert!(dump.contains("ImportCall\n"), "{dump}");
        assert!(dump.contains("String \"./m.js\""), "{dump}");
        assert!(!dump.contains("ImportCall defer"), "{dump}");
        assert!(!dump.contains("ImportCall source"), "{dump}");
    }

    #[test]
    fn parse_import_meta() {
        // E19.83.01: Module-goal `import.meta` meta-property.
        let dump = parse_module_and_dump("const u = import.meta;").unwrap();
        assert!(dump.contains("ImportMeta\n"), "{dump}");
        assert!(!dump.contains("ImportCall"), "{dump}");
        assert!(parse("const u = import.meta;").is_err());
        let dump2 = parse_module_and_dump("const p = import(import.meta);").unwrap();
        assert!(
            dump2.contains("ImportCall\n") && dump2.contains("ImportMeta\n"),
            "{dump2}"
        );
    }

    #[test]
    fn parse_import_call_options_and_trailing_comma() {
        let dump = parse_and_dump("import('./m.js',); import('./m.js', opts);").unwrap();
        assert!(dump.contains("ImportCall\n"), "{dump}");
    }

    #[test]
    fn parse_import_call_empty_args_fails() {
        assert!(parse("import();").is_err());
    }

    #[test]
    fn parse_import_call_rest_fails() {
        assert!(parse("import(...a);").is_err());
    }

    #[test]
    fn parse_import_defer_call() {
        let dump = parse_and_dump("let p = import.defer('./m.js');").unwrap();
        assert!(dump.contains("ImportCall defer\n"), "{dump}");
        assert!(dump.contains("String \"./m.js\""), "{dump}");
        // Expression-statement form (not static ImportDeclaration).
        let dump2 = parse_and_dump("import.defer('./m.js');").unwrap();
        assert!(dump2.contains("ImportCall defer\n"), "{dump2}");
    }

    #[test]
    fn parse_import_source_call() {
        let dump = parse_and_dump("let p = import.source('./m.js');").unwrap();
        assert!(dump.contains("ImportCall source\n"), "{dump}");
        assert!(dump.contains("String \"./m.js\""), "{dump}");
        let dump2 = parse_and_dump("import.source('./m.js');").unwrap();
        assert!(dump2.contains("ImportCall source\n"), "{dump2}");
    }

    #[test]
    fn parse_import_defer_empty_args_fails() {
        assert!(parse("import.defer();").is_err());
    }

    #[test]
    fn parse_import_source_empty_args_fails() {
        assert!(parse("import.source();").is_err());
    }

    #[test]
    fn parse_import_defer_rest_fails() {
        assert!(parse("import.defer(...a);").is_err());
    }

    #[test]
    fn parse_import_defer_options_fails() {
        assert!(parse("import.defer('./m.js', opts);").is_err());
    }

    #[test]
    fn parse_import_source_options_fails() {
        assert!(parse("import.source('./m.js', opts);").is_err());
    }

    #[test]
    fn parse_new_import_defer_fails() {
        assert!(parse("new import.defer('./m.js');").is_err());
    }

    #[test]
    fn parse_typeof_import_source_without_call_fails() {
        assert!(parse("typeof import.source;").is_err());
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
