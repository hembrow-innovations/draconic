use super::*;

impl Parser {
    /// Optional `: Type` type annotation (T01 named / T02 object).
    pub(crate) fn parse_optional_type_ann(&mut self) -> Result<Option<draconic_ast::TypeAnn>, Diagnostic> {
        if !self.check(&TokenKind::Colon) {
            return Ok(None);
        }
        self.bump();
        Ok(Some(self.parse_type()?))
    }

    /// `type Name = Type;` / `type Name<T> = Type;`
    pub(crate) fn is_type_alias_start(&self) -> bool {
        matches!(self.current().kind, TokenKind::Ident(ref n) if n == "type")
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Ident(_))
            )
            && self
                .tokens
                .get(self.pos + 2)
                .map(|t| matches!(t.kind, TokenKind::Eq | TokenKind::Lt))
                .unwrap_or(false)
    }

    pub(crate) fn parse_type_alias(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current().span.start.0;
        // contextual `type`
        self.bump();
        let name_tok = self.expect_ident()?;
        let name = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        let type_params = self.parse_optional_type_params()?;
        self.expect(&TokenKind::Eq)?;
        let ty = self.parse_type()?;
        let mut end = ty.span().end.0;
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::TypeAlias {
            name,
            type_params,
            ty,
            span: Span::new(start, end),
        })
    }

    /// Optional `<T, U>` type parameter list (T04).
    pub(crate) fn parse_optional_type_params(&mut self) -> Result<Vec<draconic_ast::TypeParam>, Diagnostic> {
        if !self.check(&TokenKind::Lt) {
            return Ok(Vec::new());
        }
        self.bump();
        let mut params = Vec::new();
        if !self.check(&TokenKind::Gt) {
            loop {
                let name_tok = self.expect_ident()?;
                params.push(draconic_ast::TypeParam {
                    name: Ident {
                        name: name_tok.ident_name(),
                        span: name_tok.span,
                    },
                });
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        self.expect(&TokenKind::Gt)?;
        Ok(params)
    }

    /// `<T, U>` type argument list after a type name (T04).
    fn parse_type_args(&mut self) -> Result<Vec<draconic_ast::TypeAnn>, Diagnostic> {
        self.expect(&TokenKind::Lt)?;
        let mut args = Vec::new();
        if !self.check(&TokenKind::Gt) {
            loop {
                args.push(self.parse_type()?);
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        self.expect(&TokenKind::Gt)?;
        Ok(args)
    }

    /// Type: union (`A | B`), intersection (`A & B`), named, or object.
    pub(crate) fn parse_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        self.parse_union_type()
    }

    /// `T | U | V` — lowest precedence among type operators.
    fn parse_union_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        let first = self.parse_intersection_type()?;
        if !self.check(&TokenKind::BitOr) {
            return Ok(first);
        }
        let start = first.span().start.0;
        let mut types = vec![first];
        let mut end = types[0].span().end.0;
        while self.check(&TokenKind::BitOr) {
            self.bump();
            let next = self.parse_intersection_type()?;
            end = next.span().end.0;
            types.push(next);
        }
        Ok(draconic_ast::TypeAnn::Union {
            types,
            span: Span::new(start, end),
        })
    }

    /// `T & U & V` — binds tighter than `|`.
    fn parse_intersection_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        let first = self.parse_primary_type()?;
        if !self.check(&TokenKind::BitAnd) {
            return Ok(first);
        }
        let start = first.span().start.0;
        let mut types = vec![first];
        let mut end = types[0].span().end.0;
        while self.check(&TokenKind::BitAnd) {
            self.bump();
            let next = self.parse_primary_type()?;
            end = next.span().end.0;
            types.push(next);
        }
        Ok(draconic_ast::TypeAnn::Intersection {
            types,
            span: Span::new(start, end),
        })
    }

    /// Named (`number`), generic app (`Box<T>`), object (`{ a: T }`), tuple (`[T, U]`),
    /// or pointer (`*T`, N03.03).
    fn parse_primary_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        if self.check(&TokenKind::Star) {
            let start = self.bump().span.start.0;
            let inner = self.parse_primary_type()?;
            let end = inner.span().end.0;
            return Ok(draconic_ast::TypeAnn::Pointer {
                inner: Box::new(inner),
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::LBrace) {
            return self.parse_object_type();
        }
        if self.check(&TokenKind::LBracket) {
            return self.parse_tuple_type();
        }
        // `void` is a keyword (unary op) but also a type name (TS / C FFI returns).
        if self.check(&TokenKind::Void) {
            let sp = self.bump().span;
            return Ok(draconic_ast::TypeAnn::Named {
                name: "void".into(),
                span: sp,
            });
        }
        if self.check(&TokenKind::Function) {
            let sp = self.bump().span;
            return Ok(draconic_ast::TypeAnn::Named {
                name: "function".into(),
                span: sp,
            });
        }
        let err_span = self.current().span;
        let name_tok = self
            .expect_ident()
            .map_err(|_| Diagnostic::new("expected type name after `:`".to_string(), err_span))?;
        let name = name_tok.ident_name();
        let start = name_tok.span.start.0;
        if self.check(&TokenKind::Lt) {
            let args = self.parse_type_args()?;
            let end = args
                .last()
                .map(|a| a.span().end.0)
                .unwrap_or(name_tok.span.end.0);
            // Include trailing `>` — already consumed; use current prev end via last arg + 1 is wrong.
            // parse_type_args consumes `>`; span end is the `>` token we just passed.
            let end = self.tokens[self.pos - 1].span.end.0.max(end);
            return Ok(draconic_ast::TypeAnn::GenericApp {
                name,
                args,
                span: Span::new(start, end),
            });
        }
        Ok(draconic_ast::TypeAnn::Named {
            name,
            span: name_tok.span,
        })
    }

    fn parse_object_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        let start = self.expect(&TokenKind::LBrace)?.span.start.0;
        let mut props = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let name_tok = self.expect_ident()?;
            let prop_start = name_tok.span.start.0;
            let prop_name = name_tok.ident_name();
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            let prop_end = ty.span().end.0;
            props.push(draconic_ast::TypeProp {
                name: prop_name,
                ty,
                span: Span::new(prop_start, prop_end),
            });
            if self.check(&TokenKind::Comma) || self.check(&TokenKind::Semi) {
                self.bump();
                continue;
            }
            break;
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(draconic_ast::TypeAnn::Object {
            props,
            span: Span::new(start, end),
        })
    }

    /// `[T, U, V]` fixed-length tuple type (N03.02).
    fn parse_tuple_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        let start = self.expect(&TokenKind::LBracket)?.span.start.0;
        let mut elements = Vec::new();
        while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
            let ty = self.parse_type()?;
            elements.push(ty);
            if self.check(&TokenKind::Comma) {
                self.bump();
                continue;
            }
            break;
        }
        let end = self.expect(&TokenKind::RBracket)?.span.end.0;
        Ok(draconic_ast::TypeAnn::Tuple {
            elements,
            span: Span::new(start, end),
        })
    }
}
