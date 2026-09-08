use super::*;

impl Parser {
    /// Binding pattern: identifier, `[a, b, ...rest]`, or `{ a, b: c, ...rest }`.
    pub(crate) fn parse_binding_pattern(&mut self) -> Result<BindingPattern, Diagnostic> {
        if self.check(&TokenKind::LBracket) {
            self.parse_array_binding_pattern()
        } else if self.check(&TokenKind::LBrace) {
            self.parse_object_binding_pattern()
        } else {
            let name_tok = self.expect_ident()?;
            Ok(BindingPattern::Ident(Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            }))
        }
    }

    fn parse_array_binding_pattern(&mut self) -> Result<BindingPattern, Diagnostic> {
        let start = self.expect(&TokenKind::LBracket)?.span.start.0;
        let mut elements = Vec::new();
        let mut saw_rest = false;
        if !self.check(&TokenKind::RBracket) {
            loop {
                if self.check(&TokenKind::RBracket) {
                    break;
                }
                if saw_rest {
                    return Err(Diagnostic::new(
                        "rest element must be last in array pattern".to_string(),
                        self.current().span,
                    ));
                }
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    elements.push(ArrayPatternElement::Elision);
                    continue;
                }
                if self.check(&TokenKind::DotDotDot) {
                    self.bump();
                    let binding = self.parse_binding_pattern()?;
                    elements.push(ArrayPatternElement::Rest(binding));
                    saw_rest = true;
                } else {
                    let binding = self.parse_binding_pattern()?;
                    let default = if self.check(&TokenKind::Eq) {
                        self.bump();
                        Some(self.parse_assignment()?)
                    } else {
                        None
                    };
                    elements.push(ArrayPatternElement::Pattern { binding, default });
                }
                if self.check(&TokenKind::Comma) {
                    if saw_rest {
                        return Err(Diagnostic::new(
                            "rest element must be last in array pattern".to_string(),
                            self.current().span,
                        ));
                    }
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let end = self.expect(&TokenKind::RBracket)?.span.end.0;
        Ok(BindingPattern::Array {
            elements,
            span: Span::new(start, end),
        })
    }

    fn parse_object_binding_pattern(&mut self) -> Result<BindingPattern, Diagnostic> {
        let start = self.expect(&TokenKind::LBrace)?.span.start.0;
        let mut properties = Vec::new();
        let mut saw_rest = false;
        if !self.check(&TokenKind::RBrace) {
            loop {
                if self.check(&TokenKind::RBrace) {
                    break;
                }
                if saw_rest {
                    return Err(Diagnostic::new(
                        "rest element must be last in object pattern".to_string(),
                        self.current().span,
                    ));
                }
                if self.check(&TokenKind::DotDotDot) {
                    self.bump();
                    let binding = self.parse_binding_pattern()?;
                    properties.push(ObjectPatternProp::Rest(binding));
                    saw_rest = true;
                } else {
                    // PropertyName: IdentifierName | StringLiteral | NumericLiteral | [AssignmentExpression]
                    // Shorthand only for BindingIdentifier (not string/number/computed).
                    let (key, can_shorthand) = self.parse_binding_property_name()?;
                    let key_span = object_key_span(&key);
                    if self.check(&TokenKind::Colon) {
                        self.bump();
                        let binding = self.parse_binding_pattern()?;
                        let default = if self.check(&TokenKind::Eq) {
                            self.bump();
                            Some(self.parse_assignment()?)
                        } else {
                            None
                        };
                        let end = default
                            .as_ref()
                            .map(|d| expr_span(d).end.0)
                            .unwrap_or_else(|| binding.span().end.0);
                        properties.push(ObjectPatternProp::Prop {
                            key,
                            binding,
                            shorthand: false,
                            default,
                            span: Span::new(key_span.start.0, end),
                        });
                    } else {
                        if !can_shorthand {
                            return Err(Diagnostic::new(
                                "expected ':' after property name in object pattern".to_string(),
                                self.current().span,
                            ));
                        }
                        let ObjectKey::Ident(key_id) = &key else {
                            return Err(Diagnostic::new(
                                "expected ':' after property name in object pattern".to_string(),
                                self.current().span,
                            ));
                        };
                        // Shorthand `{ a }` — BindingIdentifier (yield only when yield_is_ident).
                        if self.is_invalid_ident_name(&key_id.name) {
                            return Err(Diagnostic::new(
                                format!(
                                    "'{}' is a reserved word and cannot be used as an identifier",
                                    key_id.name
                                ),
                                key_span,
                            ));
                        }
                        let default = if self.check(&TokenKind::Eq) {
                            self.bump();
                            Some(self.parse_assignment()?)
                        } else {
                            None
                        };
                        let end = default
                            .as_ref()
                            .map(|d| expr_span(d).end.0)
                            .unwrap_or(key_id.span.end.0);
                        properties.push(ObjectPatternProp::Prop {
                            key: key.clone(),
                            binding: BindingPattern::Ident(key_id.clone()),
                            shorthand: true,
                            default,
                            span: Span::new(key_id.span.start.0, end),
                        });
                    }
                }
                if self.check(&TokenKind::Comma) {
                    if saw_rest {
                        return Err(Diagnostic::new(
                            "rest element must be last in object pattern".to_string(),
                            self.current().span,
                        ));
                    }
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(BindingPattern::Object {
            properties,
            span: Span::new(start, end),
        })
    }

    /// Object-pattern PropertyName → ObjectKey + whether shorthand is allowed.
    /// Shorthand only for IdentifierName; string/number/computed require `:`.
    fn parse_binding_property_name(&mut self) -> Result<(ObjectKey, bool), Diagnostic> {
        let tok = self.current().clone();
        if let Some(name) = tok.ident_name_opt() {
            self.bump();
            return Ok((
                ObjectKey::Ident(Ident {
                    name,
                    span: tok.span,
                }),
                true,
            ));
        }
        match &tok.kind {
            TokenKind::String(value) => {
                self.reject_legacy_octal_token(&tok)?;
                let value = value.clone();
                self.bump();
                Ok((
                    ObjectKey::String(StringLit {
                        value,
                        span: tok.span,
                    }),
                    false,
                ))
            }
            TokenKind::Number(raw) => {
                self.reject_legacy_octal_token(&tok)?;
                let name = numeric_literal_property_name(raw);
                self.bump();
                Ok((
                    ObjectKey::String(StringLit {
                        value: name.into(),
                        span: tok.span,
                    }),
                    false,
                ))
            }
            TokenKind::LBracket => {
                self.bump();
                let expr = self.with_ctx(|c| c.allow_in = true, Self::parse_assignment)?;
                self.expect(&TokenKind::RBracket)?;
                Ok((ObjectKey::Computed(Box::new(expr)), false))
            }
            _ => Err(Diagnostic::new(
                format!("expected property name, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    /// E19.32: binding elision is preserved; rest+trailing comma is a SyntaxError.
    #[test]
    fn parse_array_pattern_elision_and_rest_trailing_comma() {
        let elision = parse_and_dump("let [,] = x;\n").unwrap();
        assert!(
            elision.contains("elision"),
            "expected elision in binding pattern, got:\n{elision}"
        );
        let trail = parse_and_dump("let [a,,] = x;\n").unwrap();
        assert!(
            trail.contains("elision") && trail.contains("name: a"),
            "expected trailing elision after a, got:\n{trail}"
        );
        assert!(
            parse_and_dump("let [...x,] = [];\n").is_err(),
            "binding rest+trailing comma must fail"
        );
        // Assignment `[...x,]` stays an array literal (trailing_comma+rest → not a pattern);
        // checker rejects invalid LHS. Bare rest still becomes ArrayPattern.
        let bad_assign = parse_and_dump("[...x,] = [];\n").unwrap();
        assert!(
            bad_assign.contains("ArrayExpression") && !bad_assign.contains("ArrayPattern"),
            "rest+trailing comma must not become assignment pattern, got:\n{bad_assign}"
        );
        let ok_rest = parse_and_dump("[...x] = [];\n").unwrap();
        assert!(
            ok_rest.contains("ArrayPattern") && ok_rest.contains("rest:"),
            "bare rest assignment ok, got:\n{ok_rest}"
        );
    }

    /// E19.51: trailing-dot number + object rest with non-string computed key.
    #[test]
    fn parse_e19_51_obj_rest_non_string_computed_trailing_dot() {
        let dump = parse_and_dump("var a = 1.;\nvar b, rest;\n({[a]:b, ...rest} = vals);\n")
            .expect("var a = 1.; and assignment pattern must parse");
        assert!(
            dump.contains("Number 1.")
                && dump.contains("ObjectPattern")
                && dump.contains("rest:")
                && dump.contains("key: Computed"),
            "trailing-dot number + computed key + rest; got:\n{dump}"
        );
        let dump = parse_and_dump("for (var {[a]:b, ...rest} of vals) {}\n")
            .expect("for-of object rest computed key must parse");
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: Computed"),
            "for-of dstr; got:\n{dump}"
        );
        let dump = parse_and_dump("let {...{ [k]: v }} = obj;\n").expect("nested rest object");
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: Computed"),
            "nested rest object computed; got:\n{dump}"
        );
    }

    /// E19.46: computed property names in object binding / assignment patterns.
    #[test]
    fn parse_e19_46_object_binding_computed_keys() {
        let dump = parse_and_dump("let { [k]: v } = a;\n").unwrap();
        assert!(
            dump.contains("ObjectPattern")
                && dump.contains("key: Computed")
                && dump.contains("name: v"),
            "computed key in let object pattern; got:\n{dump}"
        );
        let dump = parse_and_dump("function f({ [k]: v }) {}\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: Computed"),
            "computed key in params; got:\n{dump}"
        );
        let dump = parse_and_dump("({ [k]: x } = a);\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: Computed"),
            "assignment object pattern computed key; got:\n{dump}"
        );
        let dump = parse_and_dump("let { [k + 1]: v = 0 } = a;\n").unwrap();
        assert!(
            dump.contains("key: Computed") && dump.contains("default:"),
            "computed key with default; got:\n{dump}"
        );
        let dump = parse_and_dump("const { [\"x\"]: n } = a;\n").unwrap();
        assert!(
            dump.contains("key: Computed"),
            "string expr computed key; got:\n{dump}"
        );
        assert!(
            parse("let { [k] } = a;\n").is_err(),
            "computed key without ':' must fail (no shorthand)"
        );
    }

    /// E19.43: object binding pattern numeric (and string) PropertyName keys.
    #[test]
    fn parse_e19_43_object_binding_numeric_keys() {
        let dump = parse_and_dump("let { 0: v, 1: w } = a;\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: 0") && dump.contains("key: 1"),
            "numeric keys in let object pattern; got:\n{dump}"
        );
        let dump = parse_and_dump("function f([...{ 0: v, 1: w, length: z }]) {}\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: 0") && dump.contains("name: v"),
            "array rest nested object numeric keys in params; got:\n{dump}"
        );
        let dump = parse_and_dump("({ 0: x } = a);\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: 0"),
            "assignment object pattern numeric key; got:\n{dump}"
        );
        let dump = parse_and_dump("let { \"0\": v } = a;\n").unwrap();
        assert!(
            dump.contains("ObjectPattern") && dump.contains("key: 0"),
            "string key \"0\" in object pattern; got:\n{dump}"
        );
        let dump = parse_and_dump("const { 0x10: n } = a;\n").unwrap();
        assert!(
            dump.contains("key: 16"),
            "hex numeric property name → ToString key; got:\n{dump}"
        );
        assert!(
            parse("let { 0 } = a;\n").is_err(),
            "numeric key without ':' must fail (no shorthand)"
        );
    }
}
