use super::*;

impl Parser {
    pub(crate) fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let tok = self.current().clone();
        match &tok.kind {
            TokenKind::Number(raw) => {
                self.reject_legacy_octal_token(&tok)?;
                self.bump();
                Ok(Expr::Number(NumberLit {
                    raw: raw.clone(),
                    span: tok.span,
                }))
            }
            TokenKind::BigInt(raw) => {
                self.bump();
                Ok(Expr::BigInt(BigIntLit {
                    raw: raw.clone(),
                    span: tok.span,
                }))
            }
            TokenKind::String(value) => {
                self.reject_legacy_octal_token(&tok)?;
                self.bump();
                Ok(Expr::String(StringLit {
                    value: value.clone(),
                    span: tok.span,
                }))
            }
            TokenKind::RegExp { pattern, flags } => {
                self.bump();
                Ok(Expr::RegExp {
                    pattern: pattern.clone(),
                    flags: flags.clone(),
                    span: tok.span,
                })
            }
            TokenKind::TemplateNoSubstitution(_) | TokenKind::TemplateHead(_) => {
                self.parse_template_literal()
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
            TokenKind::This => {
                self.bump();
                Ok(Expr::This { span: tok.span })
            }
            TokenKind::Super => {
                // E19.67: SuperProperty / SuperCall only in method/constructor/static-block code.
                if self.ctx.super_property_depth == 0 {
                    return Err(Diagnostic::new(
                        "'super' is only valid inside methods".to_string(),
                        tok.span,
                    ));
                }
                self.bump();
                Ok(Expr::Super { span: tok.span })
            }
            TokenKind::Yield if self.yield_is_ident() => {
                self.bump();
                Ok(Expr::Ident(Ident {
                    name: "yield".into(),
                    span: tok.span,
                }))
            }
            TokenKind::Yield => Err(Diagnostic::new(
                "'yield' is a reserved word and cannot be used as an identifier".to_string(),
                tok.span,
            )),
            // E19.52: [~Await] IdentifierReference `await` (scripts, non-async functions).
            TokenKind::Await if self.await_is_ident() => {
                self.bump();
                Ok(Expr::Ident(Ident {
                    name: "await".into(),
                    span: tok.span,
                }))
            }
            TokenKind::Await => Err(Diagnostic::new(
                "'await' is a reserved word and cannot be used as an identifier".to_string(),
                tok.span,
            )),
            // Non-strict IdentifierReference `let` (E19.41 statement-position ASI / bare `let`).
            TokenKind::Let if !self.ctx.in_strict => {
                self.bump();
                Ok(Expr::Ident(Ident {
                    name: "let".into(),
                    span: tok.span,
                }))
            }
            TokenKind::Let => Err(Diagnostic::new(
                "'let' is a reserved word and cannot be used as an identifier".to_string(),
                tok.span,
            )),
            // E17.02.08: non-strict IdentifierReference `static` (strict FutureReservedWord).
            TokenKind::Static if !self.ctx.in_strict => {
                self.bump();
                Ok(Expr::Ident(Ident {
                    name: "static".into(),
                    span: tok.span,
                }))
            }
            TokenKind::Static => Err(Diagnostic::new(
                "'static' is a reserved word and cannot be used as an identifier".to_string(),
                tok.span,
            )),
            // Contextual keywords: always IdentifierReference (E17.02.159).
            TokenKind::As => {
                self.bump();
                Ok(Expr::Ident(Ident {
                    name: "as".into(),
                    span: tok.span,
                }))
            }
            TokenKind::From => {
                self.bump();
                Ok(Expr::Ident(Ident {
                    name: "from".into(),
                    span: tok.span,
                }))
            }
            TokenKind::Ident(name) if self.is_invalid_ident_name(name) => Err(Diagnostic::new(
                format!("'{name}' is a reserved word and cannot be used as an identifier"),
                tok.span,
            )),
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
            TokenKind::LBrace => self.parse_object_expression(),
            TokenKind::LBracket => self.parse_array_expression(),
            TokenKind::Function => self.parse_function_expression(),
            TokenKind::Async if self.peek_is(&TokenKind::Function) => {
                self.parse_function_expression()
            }
            TokenKind::Class => self.parse_class_expression(),
            // E19.78: `@dec class …` class expression with DecoratorList.
            TokenKind::At => {
                self.parse_decorator_list()?;
                self.parse_class_expression()
            }
            _ => Err(Diagnostic::new(
                format!("expected expression, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    /// `` `…` `` / `` `a${x}b` `` untagged template literal.
    pub(crate) fn parse_template_literal(&mut self) -> Result<Expr, Diagnostic> {
        let tok = self.bump().clone();
        let start = tok.span.start.0;
        let (quasis, expressions, end) = self.parse_template_contents(tok)?;
        Ok(Expr::TemplateLiteral {
            quasis,
            expressions,
            span: Span::new(start, end),
        })
    }

    /// `` tag`…` `` / `` tag`a${x}b` `` tagged template.
    pub(crate) fn parse_tagged_template(&mut self, tag: Expr) -> Result<Expr, Diagnostic> {
        let start = expr_span(&tag).start.0;
        let tok = self.bump().clone();
        let (quasis, expressions, end) = self.parse_template_contents(tok)?;
        Ok(Expr::TaggedTemplate {
            tag: Box::new(tag),
            quasis,
            expressions,
            span: Span::new(start, end),
        })
    }

    /// Shared body for tagged/untagged templates after the opening template token.
    pub(crate) fn parse_template_contents(
        &mut self,
        first: Token,
    ) -> Result<(Vec<TemplateElement>, Vec<Expr>, u32), Diagnostic> {
        match &first.kind {
            TokenKind::TemplateNoSubstitution(cooked) => Ok((
                vec![TemplateElement {
                    cooked: cooked.clone(),
                    tail: true,
                    span: first.span,
                }],
                vec![],
                first.span.end.0,
            )),
            TokenKind::TemplateHead(head) => {
                let mut quasis = vec![TemplateElement {
                    cooked: head.clone(),
                    tail: false,
                    span: first.span,
                }];
                let mut expressions = Vec::new();
                loop {
                    expressions.push(self.parse_expr()?);
                    let cont = self.current().clone();
                    match &cont.kind {
                        TokenKind::TemplateMiddle(cooked) => {
                            let span = self.bump().span;
                            quasis.push(TemplateElement {
                                cooked: cooked.clone(),
                                tail: false,
                                span,
                            });
                        }
                        TokenKind::TemplateTail(cooked) => {
                            let span = self.bump().span;
                            quasis.push(TemplateElement {
                                cooked: cooked.clone(),
                                tail: true,
                                span,
                            });
                            return Ok((quasis, expressions, span.end.0));
                        }
                        _ => {
                            return Err(Diagnostic::new(
                                format!("expected template continuation, found {:?}", cont.kind),
                                cont.span,
                            ));
                        }
                    }
                }
            }
            _ => unreachable!("parse_template_contents on non-template token"),
        }
    }

    /// `[elem, …]` — trailing comma, holes/elision, and `...spread` allowed.
    pub(crate) fn parse_array_expression(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::LBracket)?.span.start.0;
        let mut elements = Vec::new();
        let mut trailing_comma = false;
        if !self.check(&TokenKind::RBracket) {
            loop {
                if self.check(&TokenKind::RBracket) {
                    break;
                }
                trailing_comma = false;
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    elements.push(ArrayElement::Elision);
                    continue;
                }
                if self.check(&TokenKind::DotDotDot) {
                    self.bump();
                    elements.push(ArrayElement::Spread(self.parse_assignment()?));
                } else {
                    elements.push(ArrayElement::Expr(self.parse_assignment()?));
                }
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    trailing_comma = true;
                    continue;
                }
                break;
            }
        }
        let end = self.expect(&TokenKind::RBracket)?.span.end.0;
        Ok(Expr::ArrayExpression {
            elements,
            trailing_comma,
            span: Span::new(start, end),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn parse_array_literal() {
        let dump = parse_and_dump("let a = [1, 2,]; let x = a[0]; let n = a.length;").unwrap();
        assert!(dump.contains("ArrayExpression"));
        assert!(dump.contains("element[0]:"));
        assert!(dump.contains("element[1]:"));
        assert!(dump.contains("MemberExpression computed"));
        assert!(dump.contains("MemberExpression\n"));
    }

    #[test]
    fn parse_array_spread() {
        let dump = parse_and_dump("let a = [1]; let b = [...a, 2, ...a];").unwrap();
        assert!(dump.contains("ArrayExpression"));
        assert!(dump.contains("element[0] spread:"));
        assert!(dump.contains("element[1]:"));
        assert!(dump.contains("element[2] spread:"));
    }

    #[test]
    fn parse_string_and_null() {
        let dump = parse_and_dump(r#"let s = "hi"; let n = null;"#).unwrap();
        assert!(dump.contains("String \"hi\""));
        assert!(dump.contains("Null"));
    }
}
