use super::*;

mod stmt_class;
mod stmt_for;
mod stmt_function;
pub(crate) mod stmt_lexical;
mod stmt_using;

impl Parser {
    /// One statement-list item; multi-declarator `let`/`const`/`var` expands to multiple Lets.
    pub(crate) fn parse_stmt_list_item_into(&mut self, body: &mut Vec<Stmt>) -> Result<(), Diagnostic> {
        // Declaration forms are StatementListItem only (not bare Statement).
        if self.check(&TokenKind::Const) || self.check(&TokenKind::Var) {
            body.extend(self.parse_lexical_decls()?);
            return Ok(());
        }
        if self.check(&TokenKind::Let) && self.let_starts_lexical_declaration() {
            body.extend(self.parse_lexical_decls()?);
            return Ok(());
        }
        // E19.44: `await using` / `using` declarations (contextual).
        if self.await_using_starts_declaration() {
            body.extend(self.parse_using_decls(true)?);
            return Ok(());
        }
        if self.using_starts_declaration() {
            body.extend(self.parse_using_decls(false)?);
            return Ok(());
        }
        // E19.78: DecoratorList before class declaration.
        if self.check(&TokenKind::At) || self.check(&TokenKind::Class) {
            if self.check(&TokenKind::At) {
                self.parse_decorator_list()?;
            }
            if self.check(&TokenKind::Class) {
                body.push(self.parse_class_decl()?);
                return Ok(());
            }
            return Err(Diagnostic::new(
                "decorators must precede a class declaration".to_string(),
                self.current_span(),
            ));
        }
        // HoistableDeclaration is StatementListItem (E19.49); bare Statement is Annex B only.
        if self.check(&TokenKind::Function)
            || (self.check(&TokenKind::Async) && self.peek_is(&TokenKind::Function))
        {
            body.push(self.parse_function_decl()?);
            return Ok(());
        }
        // F06.01: `extern "C" function name(…): T;` (declaration form only).
        if self.is_extern_function_start() {
            body.push(self.parse_extern_function_decl()?);
            return Ok(());
        }
        body.push(self.parse_stmt()?);
        Ok(())
    }

    /// `let` starts LexicalDeclaration when followed by BindingPattern / BindingIdentifier.
    /// Otherwise (e.g. `let;`, `let = 1`, `let.x`) it is IdentifierReference.
    ///
    /// `yield` / `await` / `static` / `let` are BindingIdentifier in the grammar even when
    /// static semantics later reject them (e.g. `let\\nyield 0` in a generator — no ASI).
    pub(crate) fn let_starts_lexical_declaration(&self) -> bool {
        if !self.check(&TokenKind::Let) {
            return false;
        }
        match self.tokens.get(self.pos + 1).map(|t| &t.kind) {
            Some(TokenKind::LBracket | TokenKind::LBrace) => true,
            Some(TokenKind::Ident(name)) if !self.is_invalid_ident_name(name) => true,
            Some(
                TokenKind::Yield
                | TokenKind::Await
                | TokenKind::Let
                | TokenKind::Static
                | TokenKind::Const
                | TokenKind::As
                | TokenKind::From,
            ) => true,
            _ => false,
        }
    }

    pub(crate) fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        if self.check(&TokenKind::Semi) {
            let span = self.bump().span;
            return Ok(Stmt::Empty { span });
        }
        if self.check(&TokenKind::LBrace) {
            return self.parse_block();
        }
        if self.check(&TokenKind::If) {
            return self.parse_if();
        }
        if self.check(&TokenKind::While) {
            return self.parse_while();
        }
        if self.check(&TokenKind::Do) {
            return self.parse_do_while();
        }
        if self.check(&TokenKind::For) {
            return self.parse_for();
        }
        if self.check(&TokenKind::Break) {
            return self.parse_break();
        }
        if self.check(&TokenKind::Continue) {
            return self.parse_continue();
        }
        if self.check(&TokenKind::Switch) {
            return self.parse_switch();
        }
        // FunctionDeclaration is StatementListItem only. Annex B.3.4 allows it solely as
        // IfStatement clause (and B.3.2 labelled) in non-strict — handled in parse_if /
        // parse_labeled. while/do/for/with always reject (E19.49 / Node parity).
        if self.check(&TokenKind::Function)
            || (self.check(&TokenKind::Async) && self.peek_is(&TokenKind::Function))
        {
            return Err(Diagnostic::new(
                "function declaration not allowed in statement position".to_string(),
                self.current().span,
            ));
        }
        // ClassDeclaration is StatementListItem only (E19.41).
        if self.check(&TokenKind::Class) {
            return Err(Diagnostic::new(
                "class declaration not allowed in statement position".to_string(),
                self.current().span,
            ));
        }
        if self.check(&TokenKind::Return) {
            return self.parse_return();
        }
        if self.check(&TokenKind::Throw) {
            return self.parse_throw();
        }
        if self.check(&TokenKind::Try) {
            return self.parse_try();
        }
        if self.check(&TokenKind::With) {
            return self.parse_with();
        }
        if self.check(&TokenKind::Import) {
            // `import(…)`, `import.defer(…)`, `import.source(…)`, `import.meta` are expressions.
            if !self.is_import_call_start() && !self.is_import_meta_start() {
                return self.parse_import();
            }
        }
        if self.check(&TokenKind::Export) {
            return self.parse_export();
        }
        // VariableStatement (`var`) is a Statement; LexicalDeclaration is not (E19.41).
        if self.check(&TokenKind::Var) {
            return self.parse_lexical_decl();
        }
        if self.check(&TokenKind::Const) {
            return Err(Diagnostic::new(
                "const declaration not allowed in statement position".to_string(),
                self.current().span,
            ));
        }
        // UsingDeclaration / AwaitUsingDeclaration are Declaration only (E19.44).
        if self.await_using_starts_declaration() || self.using_starts_declaration() {
            return Err(Diagnostic::new(
                "using declaration not allowed in statement position".to_string(),
                self.current().span,
            ));
        }
        if self.check(&TokenKind::Let) {
            // ExpressionStatement lookahead ∉ { let [ } — reject here (also not LexicalDeclaration).
            if self.peek_is(&TokenKind::LBracket) {
                return Err(Diagnostic::new(
                    "lexical declaration not allowed in statement position".to_string(),
                    self.current().span,
                ));
            }
            // `let` + BindingIdentifier / `{` same line → cannot be ExpressionStatement either
            // (no ASI); fall through and fail with expected ';' — SyntaxError either way.
            // `let` + LineTerminator → ASI IdentifierReference `let`.
            // Do not parse as LexicalDeclaration in Statement position.
        }
        // `type Name = Type;` (contextual keyword; T02)
        if self.is_type_alias_start() {
            return self.parse_type_alias();
        }
        // `label: statement` (incl. non-strict `yield:` when yield_is_ident)
        if self.at_binding_ident() && self.peek_is(&TokenKind::Colon) {
            return self.parse_labeled();
        }
        // expression statement
        let expr = self.parse_expr()?;
        let expr_span_v = expr_span(&expr);
        let end = if self.check(&TokenKind::Semi) {
            self.bump().span.end.0
        } else if self.can_asi_before_current() {
            expr_span_v.end.0
        } else {
            return Err(Diagnostic::new(
                "expected ';' after expression".to_string(),
                self.current().span,
            ));
        };
        Ok(Stmt::Expression {
            expr,
            span: Span::new(expr_span_v.start.0, end),
        })
    }

    pub(crate) fn parse_labeled(&mut self) -> Result<Stmt, Diagnostic> {
        let name_tok = self.expect_ident()?;
        let label = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        let start = name_tok.span.start.0;
        self.expect(&TokenKind::Colon)?;
        // Annex B.3.2: non-strict `label: function f() {}` (plain FunctionDeclaration only).
        let body = Box::new(self.parse_stmt_or_annex_b_function()?);
        // E19.67: LabelledItem FunctionDeclaration cannot be async/generator.
        if let Stmt::FunctionDeclaration {
            is_async,
            is_generator,
            span,
            ..
        } = body.as_ref()
        {
            if *is_async || *is_generator {
                return Err(Diagnostic::new(
                    "labelled function declaration cannot be async or generator".to_string(),
                    *span,
                ));
            }
        }
        let end = stmt_span(&body).end.0;
        Ok(Stmt::Labeled {
            label,
            body,
            span: Span::new(start, end),
        })
    }

    /// Statement, or Annex B plain FunctionDeclaration in non-strict (if / label only).
    /// E19.67: Annex B does not allow `async function` / `function*` here.
    pub(crate) fn parse_stmt_or_annex_b_function(&mut self) -> Result<Stmt, Diagnostic> {
        if !self.ctx.in_strict
            && self.check(&TokenKind::Function)
            && !self.peek_is(&TokenKind::Star)
        {
            return self.parse_function_decl();
        }
        // Async / generator declarations are never valid Annex B statement forms.
        if self.check(&TokenKind::Function)
            || (self.check(&TokenKind::Async) && self.peek_is(&TokenKind::Function))
        {
            return Err(Diagnostic::new(
                "function declaration not allowed in statement position".to_string(),
                self.current().span,
            ));
        }
        self.parse_stmt()
    }

    /// E19.67: IterationStatement / IfStatement early error — IsLabelledFunction(Statement).
    pub(crate) fn reject_labelled_function(stmt: &Stmt) -> Result<(), Diagnostic> {
        if stmt_is_labelled_function(stmt) {
            return Err(Diagnostic::new(
                "labelled function declaration not allowed here".to_string(),
                stmt_span(stmt),
            ));
        }
        Ok(())
    }

    pub(crate) fn parse_block(&mut self) -> Result<Stmt, Diagnostic> {
        self.parse_block_inner(false)
    }

    /// FunctionBody block: may contain a Directive Prologue (`"use strict"`).
    pub(crate) fn parse_function_body_block(&mut self) -> Result<Stmt, Diagnostic> {
        self.parse_block_inner(true)
    }

    pub(crate) fn parse_block_inner(&mut self, allow_directives: bool) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::LBrace)?.span.start.0;
        self.with_ctx(
            |c| {
                if allow_directives {
                    c.prologue_had_legacy_escape = false;
                }
                c.using_container_depth += 1;
                c.forbid_direct_using = false;
            },
            |p| {
                let mut body = Vec::new();
                let mut directive_prologue = allow_directives;
                while !p.check(&TokenKind::RBrace) && !p.check(&TokenKind::Eof) {
                    let upcoming_legacy_string = p.current().legacy_octal
                        && matches!(p.current().kind, TokenKind::String(_));
                    p.parse_stmt_list_item_into(&mut body)?;
                    if directive_prologue {
                        match body.last() {
                            Some(stmt) if stmt_is_directive(stmt) => {
                                if upcoming_legacy_string {
                                    p.ctx.prologue_had_legacy_escape = true;
                                }
                                if stmt_is_use_strict_directive(stmt) {
                                    p.activate_strict_from_directive()?;
                                }
                            }
                            _ => directive_prologue = false,
                        }
                    }
                }
                let end = p.expect(&TokenKind::RBrace)?.span.end.0;
                Ok(Stmt::Block {
                    body,
                    span: Span::new(start, end),
                })
            },
        )
    }

    pub(crate) fn parse_if(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::If)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let test = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        // Annex B.3.4: non-strict `if (c) function f() {}` / `else function g() {}`.
        // Plain FunctionDeclaration only; IsLabelledFunction is always SyntaxError (E19.67).
        let consequent = Box::new(self.parse_stmt_or_annex_b_function()?);
        Self::reject_labelled_function(&consequent)?;
        let alternate = if self.check(&TokenKind::Else) {
            self.bump();
            let alt = Box::new(self.parse_stmt_or_annex_b_function()?);
            Self::reject_labelled_function(&alt)?;
            Some(alt)
        } else {
            None
        };
        let end = alternate
            .as_ref()
            .map(|s| stmt_span(s).end.0)
            .unwrap_or_else(|| stmt_span(&consequent).end.0);
        Ok(Stmt::If {
            test,
            consequent,
            alternate,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_while(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::While)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let test = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        // E19.67: IterationStatement body must not be IsLabelledFunction.
        Self::reject_labelled_function(&body)?;
        let end = stmt_span(&body).end.0;
        Ok(Stmt::While {
            test,
            body,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_do_while(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Do)?.span.start.0;
        let body = Box::new(self.parse_stmt()?);
        Self::reject_labelled_function(&body)?;
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LParen)?;
        let test = self.parse_expr()?;
        let end = self.expect(&TokenKind::RParen)?.span.end.0;
        let end = if self.check(&TokenKind::Semi) {
            self.bump().span.end.0
        } else {
            end
        };
        Ok(Stmt::DoWhile {
            body,
            test,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_break(&mut self) -> Result<Stmt, Diagnostic> {
        let tok = self.expect(&TokenKind::Break)?;
        let start = tok.span.start.0;
        let mut end = tok.span.end.0;
        let label = if self.at_binding_ident() {
            let name_tok = self.expect_ident()?;
            end = name_tok.span.end.0;
            Some(Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            })
        } else {
            None
        };
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::Break {
            label,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_continue(&mut self) -> Result<Stmt, Diagnostic> {
        let tok = self.expect(&TokenKind::Continue)?;
        let start = tok.span.start.0;
        let mut end = tok.span.end.0;
        let label = if self.at_binding_ident() {
            let name_tok = self.expect_ident()?;
            end = name_tok.span.end.0;
            Some(Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            })
        } else {
            None
        };
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::Continue {
            label,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_switch(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Switch)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let discriminant = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::LBrace)?;
        let mut cases = Vec::new();
        let mut saw_default = false;
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let case = self.parse_switch_case()?;
            // E19.67: at most one DefaultClause.
            if case.test.is_none() {
                if saw_default {
                    return Err(Diagnostic::new(
                        "multiple default clauses in switch".to_string(),
                        case.span,
                    ));
                }
                saw_default = true;
            }
            cases.push(case);
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(Stmt::Switch {
            discriminant,
            cases,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_return(&mut self) -> Result<Stmt, Diagnostic> {
        let tok = self.expect(&TokenKind::Return)?;
        let start = tok.span.start.0;
        let mut end = tok.span.end.0;
        let argument = if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
            None
        } else if self.check(&TokenKind::RBrace) || self.check(&TokenKind::Eof) {
            None
        } else {
            let expr = self.parse_expr()?;
            end = expr_span(&expr).end.0;
            if self.check(&TokenKind::Semi) {
                end = self.bump().span.end.0;
            }
            Some(expr)
        };
        Ok(Stmt::Return {
            argument,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_throw(&mut self) -> Result<Stmt, Diagnostic> {
        let tok = self.expect(&TokenKind::Throw)?;
        let start = tok.span.start.0;
        // ECMA-262 / E19.67: no LineTerminator between `throw` and Expression.
        if self.current().preceded_by_line_terminator {
            return Err(Diagnostic::new(
                "line terminator not allowed after 'throw'".to_string(),
                self.current_span(),
            ));
        }
        let argument = self.parse_expr()?;
        let mut end = expr_span(&argument).end.0;
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::Throw {
            argument,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_try(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Try)?.span.start.0;
        if !self.check(&TokenKind::LBrace) {
            return Err(Diagnostic::new(
                "expected `{` after `try`".to_string(),
                self.current_span(),
            ));
        }
        let block = Box::new(self.parse_block()?);
        let mut handler_param = None;
        let mut handler = None;
        if self.check(&TokenKind::Catch) {
            self.bump(); // catch
                         // Optional catch binding (ES2019): `catch { … }` or `catch (CatchParameter) { … }`.
                         // CatchParameter is BindingIdentifier | BindingPattern (array/object).
            if self.check(&TokenKind::LParen) {
                self.bump();
                handler_param = Some(self.parse_binding_pattern()?);
                self.expect(&TokenKind::RParen)?;
            }
            if !self.check(&TokenKind::LBrace) {
                return Err(Diagnostic::new(
                    "expected `{` after catch clause".to_string(),
                    self.current_span(),
                ));
            }
            handler = Some(Box::new(self.parse_block()?));
        }
        let mut finalizer = None;
        if self.check(&TokenKind::Finally) {
            self.bump(); // finally
            if !self.check(&TokenKind::LBrace) {
                return Err(Diagnostic::new(
                    "expected `{` after `finally`".to_string(),
                    self.current_span(),
                ));
            }
            finalizer = Some(Box::new(self.parse_block()?));
        }
        if handler.is_none() && finalizer.is_none() {
            return Err(Diagnostic::new(
                "expected `catch` or `finally` after `try` block".to_string(),
                self.current_span(),
            ));
        }
        let end = if let Some(ref f) = finalizer {
            stmt_span(f).end.0
        } else if let Some(ref h) = handler {
            stmt_span(h).end.0
        } else {
            stmt_span(&block).end.0
        };
        Ok(Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_with(&mut self) -> Result<Stmt, Diagnostic> {
        // E19.39: `with` is early SyntaxError in strict mode (incl. class extends / body).
        if self.ctx.in_strict {
            return Err(Diagnostic::new(
                "'with' statements are not allowed in strict mode".to_string(),
                self.current_span(),
            ));
        }
        let start = self.expect(&TokenKind::With)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let object = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        // E19.49: WithStatement body is Statement — never FunctionDeclaration, and
        // IsLabelledFunction(Statement) is always a SyntaxError (Annex B does not extend with).
        if stmt_is_function_or_labelled_function(&body) {
            return Err(Diagnostic::new(
                "function declaration not allowed in with statement body".to_string(),
                stmt_span(&body),
            ));
        }
        let end = stmt_span(&body).end.0;
        Ok(Stmt::With {
            object,
            body,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_switch_case(&mut self) -> Result<SwitchCase, Diagnostic> {
        if self.check(&TokenKind::Case) {
            let start = self.bump().span.start.0;
            let test = self.parse_expr()?;
            let colon_end = self.expect(&TokenKind::Colon)?.span.end.0;
            let body = self.parse_switch_case_body()?;
            let end = body.last().map(|s| stmt_span(s).end.0).unwrap_or(colon_end);
            Ok(SwitchCase {
                test: Some(test),
                body,
                span: Span::new(start, end),
            })
        } else if self.check(&TokenKind::Default) {
            let start = self.bump().span.start.0;
            let colon_end = self.expect(&TokenKind::Colon)?.span.end.0;
            let body = self.parse_switch_case_body()?;
            let end = body.last().map(|s| stmt_span(s).end.0).unwrap_or(colon_end);
            Ok(SwitchCase {
                test: None,
                body,
                span: Span::new(start, end),
            })
        } else {
            Err(Diagnostic::new(
                format!("expected case or default, found {:?}", self.current().kind),
                self.current().span,
            ))
        }
    }

    pub(crate) fn parse_switch_case_body(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.with_ctx(
            |c| c.forbid_direct_using = true,
            |p| {
                let mut body = Vec::new();
                while !p.check(&TokenKind::Case)
                    && !p.check(&TokenKind::Default)
                    && !p.check(&TokenKind::RBrace)
                    && !p.check(&TokenKind::Eof)
                {
                    // Case clause body is StatementList (allows LexicalDeclaration / class).
                    // Direct `using` in the clause list is a SyntaxError (E19.44).
                    p.parse_stmt_list_item_into(&mut body)?;
                }
                Ok(body)
            },
        )
    }
}

/// Bare FunctionDeclaration or `label: … function` (IsLabelledFunction). E19.49.
fn stmt_is_function_or_labelled_function(stmt: &Stmt) -> bool {
    let mut s = stmt;
    loop {
        match s {
            Stmt::FunctionDeclaration { .. } => return true,
            Stmt::Labeled { body, .. } => s = body,
            _ => return false,
        }
    }
}

/// ECMA-262 IsLabelledFunction: LabelledStatement whose innermost item is FunctionDeclaration.
/// E19.67: IterationStatement / IfStatement early error.
fn stmt_is_labelled_function(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Labeled { body, .. } => stmt_is_function_or_labelled_function(body),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn parse_if_else_with_block() {
        let dump = parse_and_dump("if (true) { x = 1; } else { x = 2; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  If
    test:
      Boolean true
    consequent:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Number 1
    alternate:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Number 2
"
        );
    }

    #[test]
    fn parse_while_with_block() {
        let dump = parse_and_dump("while (x < 3) { x = x + 1; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  While
    test:
      Binary <
        Ident x
        Number 3
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
    fn parse_do_while_with_block() {
        let dump = parse_and_dump("do { x = x + 1; } while (x < 3);").unwrap();
        assert_eq!(
            dump,
            "\
Program
  DoWhile
    body:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Binary +
              Ident x
              Number 1
    test:
      Binary <
        Ident x
        Number 3
"
        );
    }

    #[test]
    fn parse_break_continue() {
        let dump = parse_and_dump("while (true) { break; continue; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  While
    test:
      Boolean true
    body:
      Block
        Break
        Continue
"
        );
    }

    #[test]
    fn parse_labeled_break_continue() {
        let dump = parse_and_dump("outer: while (true) { break outer; continue outer; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Labeled outer
    While
      test:
        Boolean true
      body:
        Block
          Break outer
          Continue outer
"
        );
    }

    #[test]
    fn parse_switch() {
        let dump =
            parse_and_dump("switch (x) { case 1: a = 1; break; case 2: a = 2; default: a = 0; }")
                .unwrap();
        assert_eq!(
            dump,
            "\
Program
  Switch
    discriminant:
      Ident x
    Case
      test:
        Number 1
      ExpressionStatement
        Assign =
          Ident a
          Number 1
      Break
    Case
      test:
        Number 2
      ExpressionStatement
        Assign =
          Ident a
          Number 2
    Default
      ExpressionStatement
        Assign =
          Ident a
          Number 0
"
        );
    }

    #[test]
    fn parse_optional_catch_binding() {
        let dump = parse_and_dump("try { throw 1; } catch { x = 2; }").unwrap();
        assert!(dump.contains("Try"), "got:\n{dump}");
        assert!(dump.contains("catch:"), "got:\n{dump}");
        assert!(
            !dump.contains("catch ("),
            "optional catch must not bind a param, got:\n{dump}"
        );
        let with_finally =
            parse_and_dump("try { throw 1; } catch { x = 1; } finally { y = 2; }").unwrap();
        assert!(with_finally.contains("catch:"), "got:\n{with_finally}");
        assert!(with_finally.contains("finally:"), "got:\n{with_finally}");
    }

    #[test]
    fn parse_catch_binding_destructure() {
        let ary = parse_and_dump("try { throw [1]; } catch ([a]) { x = a; }").unwrap();
        assert!(ary.contains("catch ([a]):"), "got:\n{ary}");
        let obj = parse_and_dump("try { throw {x: 1}; } catch ({x}) { y = x; }").unwrap();
        assert!(obj.contains("catch ({x}):"), "got:\n{obj}");
        let nested = parse_and_dump("try { throw [[1]]; } catch ([[a]]) { z = a; }").unwrap();
        assert!(nested.contains("catch ([[a]]):"), "got:\n{nested}");
        let rest = parse_and_dump("try { throw [1, 2]; } catch ([a, ...r]) { z = r; }").unwrap();
        assert!(rest.contains("catch ([a, ...r]):"), "got:\n{rest}");
    }

    #[test]
    fn parse_with_statement() {
        let dump = parse_and_dump("with (obj) { a = x; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  With
    object:
      Ident obj
    body:
      Block
        ExpressionStatement
          Assign =
            Ident a
            Ident x
"
        );
    }

    /// E19.41: lexical `let`/`const`/`class` are StatementListItem only, not Statement.
    #[test]
    fn parse_e19_41_statement_position_lexical_decls() {
        assert!(
            parse("if (true) let x = 1;").is_err(),
            "let decl in if consequent must fail"
        );
        assert!(
            parse("if (true) const x = 1;").is_err(),
            "const decl in if consequent must fail"
        );
        assert!(
            parse("if (true) class C {}").is_err(),
            "class decl in if consequent must fail"
        );
        assert!(
            parse("while (false) let x;").is_err(),
            "let decl in while body must fail"
        );
        assert!(
            parse("do let x; while (false);").is_err(),
            "let decl in do body must fail"
        );
        assert!(
            parse("for (;;) let x = 1;").is_err(),
            "let decl in for body must fail"
        );
        assert!(
            parse("label: let x = 1;").is_err(),
            "let decl after label must fail"
        );
        assert!(
            parse("with ({}) let x = 1;").is_err(),
            "let decl in with body must fail"
        );
        assert!(
            parse("if (true) let [x] = [];").is_err(),
            "let [ pattern in statement position must fail"
        );
        // Non-strict: `let` as IdentifierReference is a valid Statement.
        let dump = parse_and_dump("if (false) let\nx = 1;\n").unwrap();
        assert!(
            dump.contains("ExpressionStatement") && dump.contains("Ident let"),
            "let + ASI in if body must be expression; got:\n{dump}"
        );
        assert!(
            parse_and_dump("if (true) let;\n")
                .unwrap()
                .contains("Ident let"),
            "bare let; in if body must parse as identifier"
        );
        // StatementListItem still allows lexical decls (incl. case/default lists).
        assert!(
            parse_and_dump("switch (true) { case true: let x = 1; }\n").is_ok(),
            "let in case StatementList must remain valid"
        );
        assert!(
            parse_and_dump("class C {}\n").is_ok(),
            "top-level class declaration must remain valid"
        );
        assert!(
            parse_and_dump("if (true) { let x = 1; }\n").is_ok(),
            "let inside block StatementList must remain valid"
        );
        assert!(
            parse_and_dump("if (true) var x = 1;\n").is_ok(),
            "var VariableStatement is allowed in Statement position"
        );
    }
}
