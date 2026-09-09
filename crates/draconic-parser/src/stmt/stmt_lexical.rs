use crate::*;

impl Parser {
    /// One or more lexical declarators (`let a, b = 1;`), each as its own `Stmt::Let`.
    pub(crate) fn parse_lexical_decls(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        let kind_tok = self.bump();
        let kind = match kind_tok.kind {
            TokenKind::Const => BindingKind::Const,
            TokenKind::Var => BindingKind::Var,
            _ => BindingKind::Let,
        };
        let kw_start = kind_tok.span.start.0;
        let mut decls = Vec::new();
        loop {
            let binding = if kind == BindingKind::Var {
                // `var` allows simple idents and destructuring patterns.
                self.parse_binding_pattern()?
            } else {
                self.parse_binding_pattern()?
            };
            // LexicalBinding BoundNames must not include "let" (always; not only strict).
            if kind != BindingKind::Var && binding_pattern_bound_names_contain_let(&binding) {
                return Err(Diagnostic::new(
                    "'let' is not allowed as a lexical binding name".to_string(),
                    binding.span(),
                ));
            }
            let type_ann = if matches!(binding, BindingPattern::Ident(_)) {
                self.parse_optional_type_ann()?
            } else {
                None
            };
            let init = if self.check(&TokenKind::Eq) {
                self.bump();
                Some(self.parse_assignment()?)
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
            let decl_end = init
                .as_ref()
                .map(expr_span)
                .map(|s| s.end.0)
                .or_else(|| type_ann.as_ref().map(|a| a.span().end.0))
                .unwrap_or_else(|| binding.span().end.0);
            let start = if decls.is_empty() {
                kw_start
            } else {
                binding.span().start.0
            };
            decls.push(Stmt::Let {
                kind,
                binding,
                type_ann,
                init,
                span: Span::new(start, decl_end),
            });
            if self.check(&TokenKind::Comma) {
                self.bump();
                continue;
            }
            break;
        }
        if self.check(&TokenKind::Semi) {
            let semi_end = self.bump().span.end.0;
            if let Some(Stmt::Let { span, .. }) = decls.last_mut() {
                *span = Span::new(span.start.0, semi_end);
            }
        } else if !self.can_asi_before_current() {
            // `let yield 0` — no ASI between BindingIdentifier and `0` (E19.37).
            return Err(Diagnostic::new(
                "expected ';' after declaration".to_string(),
                self.current().span,
            ));
        }
        Ok(decls)
    }

    pub(crate) fn parse_lexical_decl(&mut self) -> Result<Stmt, Diagnostic> {
        let mut decls = self.parse_lexical_decls()?;
        if decls.len() == 1 {
            return Ok(decls.pop().unwrap());
        }
        // Multi-declarator as a single statement only via stmt-list expansion.
        // Callers that need one Stmt (for-init) take the first; remainder is rare.
        let start = stmt_span(&decls[0]).start.0;
        let end = stmt_span(decls.last().unwrap()).end.0;
        Ok(Stmt::Block {
            body: decls,
            span: Span::new(start, end),
        })
    }
}

/// True if BoundNames of a BindingPattern includes `let` (LexicalBinding early error).
pub(crate) fn binding_pattern_bound_names_contain_let(binding: &BindingPattern) -> bool {
    let mut has_let = false;
    binding.for_each_ident(&mut |id| {
        if id.name == "let" {
            has_let = true;
        }
    });
    has_let
}


#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn parse_let_and_expr() {
        let dump = parse_and_dump("let x = 1 + 2 * 3;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary +
        Number 1
        Binary *
          Number 2
          Number 3
"
        );
    }

    #[test]
    fn parse_const_decl() {
        let dump = parse_and_dump("const x = 1 + 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Const
    name: x
    init:
      Binary +
        Number 1
        Number 2
"
        );
    }

    #[test]
    fn parse_const_requires_initializer() {
        let err = parse("const x;").unwrap_err();
        assert!(
            err.message
                .contains("const declaration requires an initializer"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn parse_future_reserved_as_identifier_non_strict() {
        // E17.02.08: strict FutureReservedWord tokens as BindingIdentifier / IdentifierReference.
        let dump = parse_and_dump(
            "var static = 1; var let = 2; let implements = 3; function public() { return static + let; }",
        )
        .unwrap();
        assert!(
            dump.contains("name: static")
                && dump.contains("name: let")
                && dump.contains("name: implements")
                && dump.contains("name: public"),
            "future reserved as bindings, got:\n{dump}"
        );
        assert!(
            parse_and_dump("function f(static, let) { return static + let; }").is_ok(),
            "params static/let non-strict"
        );
        assert!(
            parse_and_dump("let static = 1; const public = 2;").is_ok(),
            "lexical static/public non-strict"
        );
        // LexicalBinding BoundNames must not include "let".
        assert!(parse_and_dump("let let = 1;").is_err(), "let let must fail");
        assert!(
            parse_and_dump("const let = 1;").is_err(),
            "const let must fail"
        );
        assert!(
            parse_and_dump("for (let let of []) {}").is_err(),
            "for-of let let must fail"
        );
        // Strict: FutureReservedWord reserved.
        assert!(
            parse_and_dump("\"use strict\"; var static = 1;").is_err(),
            "strict BindingIdentifier static must fail"
        );
        assert!(
            parse_and_dump("\"use strict\"; var let = 1;").is_err(),
            "strict BindingIdentifier let must fail"
        );
        assert!(
            parse_and_dump("\"use strict\"; var implements = 1;").is_err(),
            "strict BindingIdentifier implements must fail"
        );
    }
}
