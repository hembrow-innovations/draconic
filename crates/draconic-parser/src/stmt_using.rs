use super::*;

impl Parser {
    /// `using` starts UsingDeclaration when followed (no LineTerminator) by a binding.
    pub(crate) fn using_starts_declaration(&self) -> bool {
        match &self.current().kind {
            TokenKind::Ident(name) if name == "using" => {}
            _ => return false,
        }
        let Some(next) = self.tokens.get(self.pos + 1) else {
            return false;
        };
        if next.preceded_by_line_terminator {
            return false;
        }
        self.token_starts_using_binding(&next.kind)
    }

    /// `await using` starts AwaitUsingDeclaration (no LineTerminator between tokens).
    pub(crate) fn await_using_starts_declaration(&self) -> bool {
        // Only when `await` is the keyword ([+Await]), not IdentifierReference.
        if !self.ctx.in_await_context || !self.check(&TokenKind::Await) {
            return false;
        }
        let Some(using_tok) = self.tokens.get(self.pos + 1) else {
            return false;
        };
        if using_tok.preceded_by_line_terminator {
            return false;
        }
        match &using_tok.kind {
            TokenKind::Ident(name) if name == "using" => {}
            _ => return false,
        }
        let Some(next) = self.tokens.get(self.pos + 2) else {
            return false;
        };
        if next.preceded_by_line_terminator {
            return false;
        }
        self.token_starts_using_binding(&next.kind)
    }

    pub(crate) fn token_starts_using_binding(&self, kind: &TokenKind) -> bool {
        match kind {
            // Not `[`/`{`: `using[x] = …` is element access, not a declaration.
            // Invalid `using x = a, [] = b` is rejected after the comma in parse_using_decls.
            TokenKind::Ident(name) if !self.is_invalid_ident_name(name) => true,
            // BindingIdentifier may be yield/await/let/static/const tokens in some contexts.
            TokenKind::Yield
            | TokenKind::Await
            | TokenKind::Let
            | TokenKind::Static
            | TokenKind::Const
            | TokenKind::As
            | TokenKind::From => true,
            _ => false,
        }
    }

    /// `using` / `await using` BindingList (ident only; required initializer).
    pub(crate) fn parse_using_decls(&mut self, is_await: bool) -> Result<Vec<Stmt>, Diagnostic> {
        if !self.using_allowed_here() {
            return Err(Diagnostic::new(
                if is_await {
                    "await using declaration not allowed at top level of script".to_string()
                } else {
                    "using declaration not allowed at top level of script".to_string()
                },
                self.current().span,
            ));
        }
        let kw_start = self.current().span.start.0;
        let kind = if is_await {
            self.expect(&TokenKind::Await)?;
            let using_tok = self.bump(); // Ident("using")
            if using_tok.preceded_by_line_terminator {
                return Err(Diagnostic::new(
                    "LineTerminator not allowed between await and using".to_string(),
                    using_tok.span,
                ));
            }
            match &using_tok.kind {
                TokenKind::Ident(n) if n == "using" => {}
                _ => {
                    return Err(Diagnostic::new(
                        "expected using after await".to_string(),
                        using_tok.span,
                    ));
                }
            }
            BindingKind::AwaitUsing
        } else {
            let using_tok = self.bump();
            match &using_tok.kind {
                TokenKind::Ident(n) if n == "using" => {}
                _ => {
                    return Err(Diagnostic::new(
                        "expected using".to_string(),
                        using_tok.span,
                    ));
                }
            }
            BindingKind::Using
        };
        let mut decls = Vec::new();
        loop {
            // Using bindings: BindingIdentifier only (no patterns).
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
            let type_ann = self.parse_optional_type_ann()?;
            if !self.check(&TokenKind::Eq) {
                return Err(Diagnostic::new(
                    "using declaration requires an initializer".to_string(),
                    binding.span(),
                ));
            }
            self.bump();
            let init = Some(self.parse_assignment()?);
            let decl_end = init
                .as_ref()
                .map(expr_span)
                .map(|s| s.end.0)
                .unwrap_or(binding.span().end.0);
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
            return Err(Diagnostic::new(
                "expected ';' after declaration".to_string(),
                self.current().span,
            ));
        }
        Ok(decls)
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    /// E19.44: `using` / `await using` declarations.
    #[test]
    fn parse_e19_44_using_declarations() {
        let dump = parse_and_dump("{ using x = null; }\n").unwrap();
        assert!(
            dump.contains("Using") && dump.contains("name: x"),
            "block using decl: {dump}"
        );
        let multi = parse_and_dump("{ using a = null, b = null; }\n").unwrap();
        assert!(multi.matches("Using").count() >= 2, "multi using: {multi}");
        let aw = parse_and_dump("async function f() { await using x = null; }\n").unwrap();
        assert!(
            aw.contains("AwaitUsing") && aw.contains("name: x"),
            "await using: {aw}"
        );
        let for_of = parse_and_dump("{ for (using x of ys) {} }\n").unwrap();
        assert!(
            for_of.contains("ForOf") && for_of.contains("Using"),
            "for-of using: {for_of}"
        );
        let classic = parse_and_dump("{ for (using x = null; false; ) {} }\n").unwrap();
        assert!(
            classic.contains("For") && classic.contains("Using"),
            "classic for using: {classic}"
        );
        // Script top-level using is a SyntaxError.
        assert!(
            parse_and_dump("using x = null;\n").is_err(),
            "script top-level using must fail"
        );
        // Patterns forbidden.
        assert!(
            parse_and_dump("{ using [] = null; }\n").is_err(),
            "using array pattern must fail"
        );
        assert!(
            parse_and_dump("{ using {} = null; }\n").is_err(),
            "using object pattern must fail"
        );
        // Missing initializer.
        assert!(
            parse_and_dump("{ using x; }\n").is_err(),
            "using without init must fail"
        );
        // Statement position (if bare).
        assert!(
            parse_and_dump("{ if (true) using x = null; }\n").is_err(),
            "using in statement position must fail"
        );
        // for-in forbidden.
        assert!(
            parse_and_dump("{ for (using x in ys) {} }\n").is_err(),
            "using for-in must fail"
        );
        // Case clause direct list forbidden.
        assert!(
            parse_and_dump("switch (0) { case 0: using x = null; }\n").is_err(),
            "using in case clause must fail"
        );
        // Nested block in case is OK.
        assert!(
            parse_and_dump("switch (0) { case 0: { using x = null; } }\n").is_ok(),
            "using in block inside case must parse"
        );
        // `using` remains a valid identifier when not a declaration.
        let id = parse_and_dump("let using = 1; using + 2;\n").unwrap();
        assert!(id.contains("name: using"), "using as ident: {id}");
        // Module top-level using OK.
        let mod_prog = parse_module("using x = null;\n").unwrap();
        let mod_dump = dump_program(&mod_prog);
        assert!(
            mod_dump.contains("Using"),
            "module top-level using: {mod_dump}"
        );
    }
}
