use draconic_ast::{
    dump_program, AccessorKind, Arg, ArrayElement, ArrayPatternElement, ArrowBody, AssignOp,
    BinaryOp, BigIntLit, BindingKind, BindingPattern, ClassElement, ExportSpecifier, Expr, Ident,
    ImportPhase, ImportSpecifier, NumberLit, ObjectKey, ObjectPatternProp, ObjectProp, Param,
    Program, Stmt, StringLit, SwitchCase, TemplateElement, UnaryOp, UpdateOp,
};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_lexer::{Lexer, Token, TokenKind};

pub use draconic_ast::dump_program as dump_ast;

pub fn parse(source: &str) -> Result<Program, Diagnostic> {
    let tokens = Lexer::new(source).tokenize()?;
    Parser::new(tokens).parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// When false, relational `in` is not parsed (for-header left-hand side).
    allow_in: bool,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            allow_in: true,
        }
    }

    fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let start = self.current_span().start.0;
        let mut body = Vec::new();
        while !self.check(&TokenKind::Eof) {
            self.parse_stmt_list_item_into(&mut body)?;
        }
        let end = self.current_span().end.0;
        Ok(Program {
            body,
            span: Span::new(start, end),
        })
    }

    /// One statement-list item; multi-declarator `let`/`const`/`var` expands to multiple Lets.
    fn parse_stmt_list_item_into(&mut self, body: &mut Vec<Stmt>) -> Result<(), Diagnostic> {
        if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) || self.check(&TokenKind::Var)
        {
            body.extend(self.parse_lexical_decls()?);
            return Ok(());
        }
        body.push(self.parse_stmt()?);
        Ok(())
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
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
        if self.check(&TokenKind::Function)
            || (self.check(&TokenKind::Async) && self.peek_is(&TokenKind::Function))
        {
            return self.parse_function_decl();
        }
        if self.check(&TokenKind::Class) {
            return self.parse_class_decl();
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
            // `import(…)`, `import.defer(…)`, `import.source(…)` are ImportCall expressions.
            if !self.is_import_call_start() {
                return self.parse_import();
            }
        }
        if self.check(&TokenKind::Export) {
            return self.parse_export();
        }
        if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) || self.check(&TokenKind::Var)
        {
            return self.parse_lexical_decl();
        }
        // `type Name = Type;` (contextual keyword; T02)
        if self.is_type_alias_start() {
            return self.parse_type_alias();
        }
        // `label: statement`
        if matches!(self.current().kind, TokenKind::Ident(_)) && self.peek_is(&TokenKind::Colon) {
            return self.parse_labeled();
        }
        // expression statement
        let expr = self.parse_expr()?;
        let expr_span = expr_span(&expr);
        let end = if self.check(&TokenKind::Semi) {
            self.bump().span.end.0
        } else if self.check(&TokenKind::Eof) {
            expr_span.end.0
        } else {
            // ASI: allow newline-terminated; for bootstrap require ; or eof/next stmt boundary
            expr_span.end.0
        };
        Ok(Stmt::Expression {
            expr,
            span: Span::new(expr_span.start.0, end),
        })
    }

    fn parse_labeled(&mut self) -> Result<Stmt, Diagnostic> {
        let name_tok = self.expect_ident()?;
        let label = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        let start = name_tok.span.start.0;
        self.expect(&TokenKind::Colon)?;
        let body = Box::new(self.parse_stmt()?);
        let end = stmt_span(&body).end.0;
        Ok(Stmt::Labeled {
            label,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_block(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::LBrace)?.span.start.0;
        let mut body = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.parse_stmt_list_item_into(&mut body)?;
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(Stmt::Block {
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::If)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let test = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        let consequent = Box::new(self.parse_stmt()?);
        let alternate = if self.check(&TokenKind::Else) {
            self.bump();
            Some(Box::new(self.parse_stmt()?))
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

    fn parse_while(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::While)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let test = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        let end = stmt_span(&body).end.0;
        Ok(Stmt::While {
            test,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_do_while(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Do)?.span.start.0;
        let body = Box::new(self.parse_stmt()?);
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

    fn parse_for(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::For)?.span.start.0;
        // `for await (… of …)` — async iteration (E18.42).
        let is_await = if self.check(&TokenKind::Await) {
            self.bump();
            true
        } else {
            false
        };
        self.expect(&TokenKind::LParen)?;

        // `for (let/const/var binding in/of right)` and classic `for (let/const/var …; …; …)`.
        // Annex B.3.5: `for (var name = init in right)` only (ident binding).
        if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) || self.check(&TokenKind::Var)
        {
            let kind = if self.check(&TokenKind::Const) {
                BindingKind::Const
            } else if self.check(&TokenKind::Var) {
                BindingKind::Var
            } else {
                BindingKind::Let
            };
            let let_start = self.bump().span.start.0;
            let binding = self.parse_binding_pattern()?;
            let binding_end = binding.span().end.0;
            if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
                let is_in = self.check(&TokenKind::In);
                self.bump();
                let right = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                let end = stmt_span(&body).end.0;
                let left = Box::new(Stmt::Let {
                    kind,
                    binding,
                    type_ann: None,
                    init: None,
                    span: Span::new(let_start, binding_end),
                });
                return if is_in {
                    if is_await {
                        return Err(Diagnostic::new(
                            "for await…in is not allowed".to_string(),
                            Span::new(start, end),
                        ));
                    }
                    Ok(Stmt::ForIn {
                        left,
                        right,
                        body,
                        span: Span::new(start, end),
                    })
                } else {
                    Ok(Stmt::ForOf {
                        left,
                        right,
                        body,
                        is_await,
                        span: Span::new(start, end),
                    })
                };
            }
            if is_await {
                return Err(Diagnostic::new(
                    "for await requires `of`".to_string(),
                    Span::new(start, binding_end),
                ));
            }
            // Classic `for (let/const/var binding: T? = init; …)` / Annex B `for (var name = init in …)`.
            // Disable relational `in` while parsing the initializer so
            // `for (var k = 1 in obj)` is Annex B, not `k = (1 in obj)`.
            let type_ann = if matches!(binding, BindingPattern::Ident(_)) {
                self.parse_optional_type_ann()?
            } else {
                None
            };
            let init_expr = if self.check(&TokenKind::Eq) {
                self.bump();
                let prev_allow_in = self.allow_in;
                self.allow_in = false;
                let e = self.parse_assignment();
                self.allow_in = prev_allow_in;
                Some(e?)
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
            // Annex B.3.5 / for-of reject: initializer then `in`/`of`.
            if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
                let is_in = self.check(&TokenKind::In);
                if !is_in {
                    return Err(Diagnostic::new(
                        "for-of binding cannot have an initializer".to_string(),
                        binding.span(),
                    ));
                }
                if kind != BindingKind::Var
                    || type_ann.is_some()
                    || !matches!(binding, BindingPattern::Ident(_))
                {
                    return Err(Diagnostic::new(
                        "for-in binding cannot have an initializer".to_string(),
                        binding.span(),
                    ));
                }
                self.bump();
                let right = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                let end = stmt_span(&body).end.0;
                let let_end = if let Some(ref e) = init_expr {
                    expr_span(e).end.0
                } else {
                    binding_end
                };
                let left = Box::new(Stmt::Let {
                    kind,
                    binding,
                    type_ann: None,
                    init: init_expr,
                    span: Span::new(let_start, let_end),
                });
                return Ok(Stmt::ForIn {
                    left,
                    right,
                    body,
                    span: Span::new(start, end),
                });
            }
            let let_end = if let Some(ref e) = init_expr {
                expr_span(e).end.0
            } else if let Some(ref ann) = type_ann {
                ann.span().end.0
            } else {
                binding_end
            };
            self.expect(&TokenKind::Semi)?;
            let left_init = Some(Box::new(Stmt::Let {
                kind,
                binding,
                type_ann,
                init: init_expr,
                span: Span::new(let_start, let_end),
            }));
            return self.finish_classic_for(start, left_init);
        }

        if self.check(&TokenKind::Semi) {
            if is_await {
                return Err(Diagnostic::new(
                    "for await requires `of`".to_string(),
                    Span::new(start, self.current_span().start.0),
                ));
            }
            self.bump();
            return self.finish_classic_for(start, None);
        }

        // Expression left: `for (lhs in/of right)` or classic `for (expr; …)`.
        // Disable relational `in` so `for (z in obj)` does not consume `in` here.
        let prev_allow_in = self.allow_in;
        self.allow_in = false;
        let expr = self.parse_expr();
        self.allow_in = prev_allow_in;
        let expr = expr?;
        let mut left_span = expr_span(&expr);
        if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
            let is_in = self.check(&TokenKind::In);
            self.bump();
            let right = self.parse_expr()?;
            self.expect(&TokenKind::RParen)?;
            let body = Box::new(self.parse_stmt()?);
            let end = stmt_span(&body).end.0;
            // Reinterpret array/object literals as assignment patterns for for-in/of LHS.
            let expr = array_expr_to_pattern(&expr)
                .or_else(|| object_expr_to_pattern(&expr))
                .unwrap_or(expr);
            left_span = expr_span(&expr);
            let left = Box::new(Stmt::Expression {
                expr,
                span: left_span,
            });
            return if is_in {
                if is_await {
                    return Err(Diagnostic::new(
                        "for await…in is not allowed".to_string(),
                        Span::new(start, end),
                    ));
                }
                Ok(Stmt::ForIn {
                    left,
                    right,
                    body,
                    span: Span::new(start, end),
                })
            } else {
                Ok(Stmt::ForOf {
                    left,
                    right,
                    body,
                    is_await,
                    span: Span::new(start, end),
                })
            };
        }

        if is_await {
            return Err(Diagnostic::new(
                "for await requires `of`".to_string(),
                Span::new(start, left_span.end.0),
            ));
        }
        self.expect(&TokenKind::Semi)?;
        let init = Some(Box::new(Stmt::Expression {
            expr,
            span: left_span,
        }));
        self.finish_classic_for(start, init)
    }

    fn finish_classic_for(
        &mut self,
        start: u32,
        init: Option<Box<Stmt>>,
    ) -> Result<Stmt, Diagnostic> {
        let test = if self.check(&TokenKind::Semi) {
            self.bump();
            None
        } else {
            let expr = self.parse_expr()?;
            self.expect(&TokenKind::Semi)?;
            Some(expr)
        };
        let update = if self.check(&TokenKind::RParen) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        let end = stmt_span(&body).end.0;
        Ok(Stmt::For {
            init,
            test,
            update,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_break(&mut self) -> Result<Stmt, Diagnostic> {
        let tok = self.expect(&TokenKind::Break)?;
        let start = tok.span.start.0;
        let mut end = tok.span.end.0;
        let label = if matches!(self.current().kind, TokenKind::Ident(_)) {
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

    fn parse_continue(&mut self) -> Result<Stmt, Diagnostic> {
        let tok = self.expect(&TokenKind::Continue)?;
        let start = tok.span.start.0;
        let mut end = tok.span.end.0;
        let label = if matches!(self.current().kind, TokenKind::Ident(_)) {
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

    fn parse_switch(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Switch)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let discriminant = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::LBrace)?;
        let mut cases = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            cases.push(self.parse_switch_case()?);
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(Stmt::Switch {
            discriminant,
            cases,
            span: Span::new(start, end),
        })
    }

    fn parse_function_decl(&mut self) -> Result<Stmt, Diagnostic> {
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
        let name_tok = self.expect_ident()?;
        let name = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        let type_params = self.parse_optional_type_params()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        let return_type = self.parse_optional_type_ann()?;
        let body = Box::new(self.parse_block()?);
        let end = stmt_span(&body).end.0;
        Ok(Stmt::FunctionDeclaration {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            is_generator,
            span: Span::new(start, end),
        })
    }

    /// `class Name extends Super? { constructor?(…) {…} method(…) {…} … }`
    fn parse_class_decl(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Class)?.span.start.0;
        let name_tok = self.expect_ident()?;
        let name = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        let (super_class, body, end) = self.parse_class_tail()?;
        Ok(Stmt::ClassDeclaration {
            name,
            super_class,
            body,
            span: Span::new(start, end),
        })
    }

    /// `class Name? extends Super? { … }` in expression position (E18.33).
    fn parse_class_expression(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::Class)?.span.start.0;
        let name = if matches!(self.current().kind, TokenKind::Ident(_)) {
            let name_tok = self.expect_ident()?;
            Some(Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            })
        } else {
            None
        };
        let (super_class, body, end) = self.parse_class_tail()?;
        Ok(Expr::ClassExpression {
            name,
            super_class,
            body,
            span: Span::new(start, end),
        })
    }

    /// `extends Super? { elements… }` shared by class declaration and expression.
    fn parse_class_tail(
        &mut self,
    ) -> Result<(Option<Box<Expr>>, Vec<ClassElement>, u32), Diagnostic> {
        let super_class = if self.check(&TokenKind::Extends) {
            self.bump();
            Some(Box::new(self.parse_lhs()?))
        } else {
            None
        };
        self.expect(&TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            // Empty ClassElement: lone `;` (ECMA-262 ClassElement → `;`).
            if self.check(&TokenKind::Semi) {
                self.bump();
                continue;
            }
            let el = self.parse_class_element()?;
            let needs_field_semi = matches!(
                &el,
                ClassElement::Field { .. }
            );
            body.push(el);
            // FieldDefinition requires `;` (explicit or ASI). Methods end at `}`.
            if needs_field_semi {
                if self.check(&TokenKind::Semi) {
                    self.bump();
                } else if self.check(&TokenKind::RBrace) || self.check(&TokenKind::Eof) {
                    // ASI before `}` / EOF
                } else if self.current().preceded_by_line_terminator {
                    // ASI across LineTerminator
                } else {
                    return Err(Diagnostic::new(
                        "expected ';' after class field".to_string(),
                        self.current_span(),
                    ));
                }
            } else if self.check(&TokenKind::Semi) {
                self.bump();
            }
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        validate_class_body(&body)?;
        Ok((super_class, body, end))
    }

    fn parse_class_element(&mut self) -> Result<ClassElement, Diagnostic> {
        let start = self.current_span().start.0;
        let is_static = if self.check(&TokenKind::Static) {
            self.bump();
            true
        } else {
            false
        };
        // `static { … }` static initialization block (E18.41).
        if is_static && self.check(&TokenKind::LBrace) {
            let body = Box::new(self.parse_block()?);
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
            let value = if self.check(&TokenKind::Eq) {
                self.bump();
                Some(self.parse_assignment()?)
            } else {
                None
            };
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
            let params = self.parse_param_list()?;
            self.expect(&TokenKind::RParen)?;
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
            let body = Box::new(self.parse_block()?);
            let end = stmt_span(&body).end.0;
            return Ok(ClassElement::Accessor {
                kind,
                key,
                params,
                body,
                is_static,
                is_private,
                span: Span::new(start, end),
            });
        }
        // `async m()` / `async *m()` / `async #m()` / `async [e]()` — not method/field named `async`.
        let is_async = if self.check(&TokenKind::Async) && self.peek_starts_method_name() {
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
                let params = self.parse_param_list()?;
                self.expect(&TokenKind::RParen)?;
                let body = Box::new(self.parse_block()?);
                let end = stmt_span(&body).end.0;
                return Ok(ClassElement::Method {
                    key: ObjectKey::Ident(name),
                    params,
                    body,
                    is_static,
                    is_async,
                    is_generator,
                    is_private: true,
                    span: Span::new(start, end),
                });
            }
            let value = if self.check(&TokenKind::Eq) {
                self.bump();
                Some(self.parse_assignment()?)
            } else {
                None
            };
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
            let value = if self.check(&TokenKind::Eq) {
                self.bump();
                Some(self.parse_assignment()?)
            } else {
                None
            };
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
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_block()?);
        let end = stmt_span(&body).end.0;
        let span = Span::new(start, end);
        // Only literal IdentifierName `constructor` is the constructor; computed/`"constructor"` are methods.
        if class_key_is_literal_constructor(&key) {
            if is_static {
                return Err(Diagnostic::new(
                    "class constructor cannot be static".to_string(),
                    span,
                ));
            }
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
            Ok(ClassElement::Constructor {
                params,
                body,
                span,
            })
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
    }

    /// `async? function *? name? (params) { body }` in expression position.
    fn parse_function_expression(&mut self) -> Result<Expr, Diagnostic> {
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
        let name = if matches!(self.current().kind, TokenKind::Ident(_)) {
            let name_tok = self.expect_ident()?;
            Some(Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            })
        } else {
            None
        };
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        let return_type = self.parse_optional_type_ann()?;
        let body = Box::new(self.parse_block()?);
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
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, Diagnostic> {
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

    /// Optional `: Type` type annotation (T01 named / T02 object).
    fn parse_optional_type_ann(&mut self) -> Result<Option<draconic_ast::TypeAnn>, Diagnostic> {
        if !self.check(&TokenKind::Colon) {
            return Ok(None);
        }
        self.bump();
        Ok(Some(self.parse_type()?))
    }

    /// `type Name = Type;` / `type Name<T> = Type;`
    fn is_type_alias_start(&self) -> bool {
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

    fn parse_type_alias(&mut self) -> Result<Stmt, Diagnostic> {
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
    fn parse_optional_type_params(&mut self) -> Result<Vec<draconic_ast::TypeParam>, Diagnostic> {
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
    fn parse_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
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
        let err_span = self.current().span;
        let name_tok = self.expect_ident().map_err(|_| {
            Diagnostic::new("expected type name after `:`".to_string(), err_span)
        })?;
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

    fn parse_return(&mut self) -> Result<Stmt, Diagnostic> {
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

    fn parse_throw(&mut self) -> Result<Stmt, Diagnostic> {
        let tok = self.expect(&TokenKind::Throw)?;
        let start = tok.span.start.0;
        // ECMA-262: no LineTerminator between `throw` and Expression.
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

    fn parse_try(&mut self) -> Result<Stmt, Diagnostic> {
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

    fn parse_with(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::With)?.span.start.0;
        self.expect(&TokenKind::LParen)?;
        let object = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        let end = stmt_span(&body).end.0;
        Ok(Stmt::With {
            object,
            body,
            span: Span::new(start, end),
        })
    }

    /// `import { a, b as c } from "mod";`
    /// `import d from "mod";`
    /// `import d, { a } from "mod";`
    /// `import * as ns from "mod";`
    /// `import d, * as ns from "mod";`
    fn parse_import(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Import)?.span.start.0;
        let mut specifiers = Vec::new();
        let mut namespace = None;

        if self.check(&TokenKind::Star) {
            namespace = Some(self.parse_namespace_import()?);
        } else if matches!(self.current().kind, TokenKind::Ident(_)) {
            let local_tok = self.expect_ident()?;
            let local = Ident {
                name: local_tok.ident_name(),
                span: local_tok.span,
            };
            let def_span = local.span;
            specifiers.push(ImportSpecifier {
                imported: Ident {
                    name: "default".into(),
                    span: def_span,
                },
                local,
            });
            if self.check(&TokenKind::Comma) {
                self.bump();
                if self.check(&TokenKind::Star) {
                    namespace = Some(self.parse_namespace_import()?);
                } else {
                    self.expect(&TokenKind::LBrace)?;
                    self.parse_named_import_specifiers(&mut specifiers)?;
                    self.expect(&TokenKind::RBrace)?;
                }
            }
        } else {
            self.expect(&TokenKind::LBrace)?;
            self.parse_named_import_specifiers(&mut specifiers)?;
            self.expect(&TokenKind::RBrace)?;
        }

        self.expect(&TokenKind::From)?;
        let source = self.expect_string_lit()?;
        let mut end = source.span.end.0;
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::ImportDeclaration {
            specifiers,
            namespace,
            source,
            span: Span::new(start, end),
        })
    }

    /// `* as ImportedBinding`
    fn parse_namespace_import(&mut self) -> Result<Ident, Diagnostic> {
        self.expect(&TokenKind::Star)?;
        self.expect(&TokenKind::As)?;
        let local_tok = self.expect_ident()?;
        Ok(Ident {
            name: local_tok.ident_name(),
            span: local_tok.span,
        })
    }

    fn parse_named_import_specifiers(
        &mut self,
        specifiers: &mut Vec<ImportSpecifier>,
    ) -> Result<(), Diagnostic> {
        if self.check(&TokenKind::RBrace) {
            return Ok(());
        }
        loop {
            // `default` is a keyword but valid as ImportedBinding name: `{ default as x }`.
            let (imported_name, imported_span) = self.expect_ident_name()?;
            let imported = Ident {
                name: imported_name,
                span: imported_span,
            };
            let local = if self.check(&TokenKind::As) {
                self.bump();
                let local_tok = self.expect_ident()?;
                Ident {
                    name: local_tok.ident_name(),
                    span: local_tok.span,
                }
            } else if imported.name == "default" {
                return Err(Diagnostic::new(
                    "default import in named list requires `as` binding".to_string(),
                    imported.span,
                ));
            } else {
                imported.clone()
            };
            specifiers.push(ImportSpecifier { imported, local });
            if self.check(&TokenKind::Comma) {
                self.bump();
                if self.check(&TokenKind::RBrace) {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(())
    }

    /// `export let/const/function …` or `export { a, b as c };` or `export * from` or `export default …`
    fn parse_export(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Export)?.span.start.0;
        if self.check(&TokenKind::Default) {
            return self.parse_export_default(start);
        }
        // `export * from "mod"` / `export * as ns from "mod"`
        if self.check(&TokenKind::Star) {
            self.bump();
            let exported = if self.check(&TokenKind::As) {
                self.bump();
                let tok = self.expect_ident()?;
                Some(Ident {
                    name: tok.ident_name(),
                    span: tok.span,
                })
            } else {
                None
            };
            self.expect(&TokenKind::From)?;
            let source = self.expect_string_lit()?;
            let mut end = source.span.end.0;
            if self.check(&TokenKind::Semi) {
                end = self.bump().span.end.0;
            }
            return Ok(Stmt::ExportAllDeclaration {
                exported,
                source,
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::LBrace) {
            self.bump();
            let mut specifiers = Vec::new();
            if !self.check(&TokenKind::RBrace) {
                loop {
                    // IdentifierName: `default` is valid in `{ default as x }` / `{ x as default }`.
                    let (local_name, local_span) = self.expect_ident_name()?;
                    let local = Ident {
                        name: local_name,
                        span: local_span,
                    };
                    let exported = if self.check(&TokenKind::As) {
                        self.bump();
                        let (name, span) = self.expect_ident_name()?;
                        Ident { name, span }
                    } else {
                        local.clone()
                    };
                    specifiers.push(ExportSpecifier { local, exported });
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
            let end_brace = self.expect(&TokenKind::RBrace)?.span.end.0;
            let mut end = end_brace;
            // `export { a, b as c } from "mod"`
            let source = if self.check(&TokenKind::From) {
                self.bump();
                let src = self.expect_string_lit()?;
                end = src.span.end.0;
                Some(src)
            } else {
                None
            };
            if self.check(&TokenKind::Semi) {
                end = self.bump().span.end.0;
            }
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: None,
                specifiers,
                source,
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
            let decl = self.parse_lexical_decl()?;
            let end = stmt_span(&decl).end.0;
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: Some(Box::new(decl)),
                specifiers: Vec::new(),
                source: None,
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::Function)
            || (self.check(&TokenKind::Async) && self.peek_is(&TokenKind::Function))
        {
            let decl = self.parse_function_decl()?;
            let end = stmt_span(&decl).end.0;
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: Some(Box::new(decl)),
                specifiers: Vec::new(),
                source: None,
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::Class) {
            let decl = self.parse_class_decl()?;
            let end = stmt_span(&decl).end.0;
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: Some(Box::new(decl)),
                specifiers: Vec::new(),
                source: None,
                span: Span::new(start, end),
            });
        }
        Err(Diagnostic::new(
            "expected `default`, `*`, `let`, `const`, `function`, `class`, or `{` after `export`"
                .to_string(),
            self.current_span(),
        ))
    }

    /// `export default async? function name? (…) {…}` or `export default class Name {…}` or `export default expr;`
    fn parse_export_default(&mut self, start: u32) -> Result<Stmt, Diagnostic> {
        self.expect(&TokenKind::Default)?;
        if self.check(&TokenKind::Function)
            || (self.check(&TokenKind::Async) && self.peek_is(&TokenKind::Function))
        {
            let (is_async, fn_start) = if self.check(&TokenKind::Async) {
                let s = self.bump().span.start.0;
                (true, s)
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
            let (name, is_synthetic) = if matches!(self.current().kind, TokenKind::Ident(_)) {
                let name_tok = self.expect_ident()?;
                (
                    Ident {
                        name: name_tok.ident_name(),
                        span: name_tok.span,
                    },
                    false,
                )
            } else {
                (
                    Ident {
                        name: "__default".into(),
                        span: Span::new(fn_start, fn_start),
                    },
                    true,
                )
            };
            self.expect(&TokenKind::LParen)?;
            let params = self.parse_param_list()?;
            self.expect(&TokenKind::RParen)?;
            let return_type = self.parse_optional_type_ann()?;
            let body = Box::new(self.parse_block()?);
            let end = stmt_span(&body).end.0;
            let local = name.clone();
            let declaration = if is_synthetic {
                // Anonymous default function → `let __default = async? function *? (…) {…}`
                Stmt::Let {
                    kind: BindingKind::Let,
                    binding: BindingPattern::Ident(local.clone()),
                    type_ann: None,
                    init: Some(Expr::FunctionExpression {
                        name: None,
                        params,
                        return_type,
                        body,
                        is_async,
                        is_generator,
                        is_method: false,
                        span: Span::new(fn_start, end),
                    }),
                    span: Span::new(fn_start, end),
                }
            } else {
                Stmt::FunctionDeclaration {
                    name,
                    type_params: Vec::new(),
                    params,
                    return_type,
                    body,
                    is_async,
                    is_generator,
                    span: Span::new(fn_start, end),
                }
            };
            return Ok(Stmt::ExportDefaultDeclaration {
                declaration: Box::new(declaration),
                local,
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::Class) {
            let decl = self.parse_class_decl()?;
            let end = stmt_span(&decl).end.0;
            let local = match &decl {
                Stmt::ClassDeclaration { name, .. } => name.clone(),
                _ => unreachable!("parse_class_decl returns ClassDeclaration"),
            };
            return Ok(Stmt::ExportDefaultDeclaration {
                declaration: Box::new(decl),
                local,
                span: Span::new(start, end),
            });
        }

        let expr = self.parse_expr()?;
        let mut end = expr_span(&expr).end.0;
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        let local = Ident {
            name: "__default".into(),
            span: Span::new(start, end),
        };
        let declaration = Stmt::Let {
            kind: BindingKind::Let,
            binding: BindingPattern::Ident(local.clone()),
            type_ann: None,
            init: Some(expr),
            span: Span::new(start, end),
        };
        Ok(Stmt::ExportDefaultDeclaration {
            declaration: Box::new(declaration),
            local,
            span: Span::new(start, end),
        })
    }

    fn expect_string_lit(&mut self) -> Result<StringLit, Diagnostic> {
        let tok = self.current().clone();
        match tok.kind {
            TokenKind::String(value) => {
                self.bump();
                Ok(StringLit {
                    value,
                    span: tok.span,
                })
            }
            _ => Err(Diagnostic::new(
                format!("expected string literal, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    fn parse_switch_case(&mut self) -> Result<SwitchCase, Diagnostic> {
        if self.check(&TokenKind::Case) {
            let start = self.bump().span.start.0;
            let test = self.parse_expr()?;
            let colon_end = self.expect(&TokenKind::Colon)?.span.end.0;
            let mut body = Vec::new();
            while !self.check(&TokenKind::Case)
                && !self.check(&TokenKind::Default)
                && !self.check(&TokenKind::RBrace)
                && !self.check(&TokenKind::Eof)
            {
                body.push(self.parse_stmt()?);
            }
            let end = body
                .last()
                .map(|s| stmt_span(s).end.0)
                .unwrap_or(colon_end);
            Ok(SwitchCase {
                test: Some(test),
                body,
                span: Span::new(start, end),
            })
        } else if self.check(&TokenKind::Default) {
            let start = self.bump().span.start.0;
            let colon_end = self.expect(&TokenKind::Colon)?.span.end.0;
            let mut body = Vec::new();
            while !self.check(&TokenKind::Case)
                && !self.check(&TokenKind::Default)
                && !self.check(&TokenKind::RBrace)
                && !self.check(&TokenKind::Eof)
            {
                body.push(self.parse_stmt()?);
            }
            let end = body
                .last()
                .map(|s| stmt_span(s).end.0)
                .unwrap_or(colon_end);
            Ok(SwitchCase {
                test: None,
                body,
                span: Span::new(start, end),
            })
        } else {
            Err(Diagnostic::new(
                format!(
                    "expected case or default, found {:?}",
                    self.current().kind
                ),
                self.current().span,
            ))
        }
    }

    /// One or more lexical declarators (`let a, b = 1;`), each as its own `Stmt::Let`.
    fn parse_lexical_decls(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
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
        }
        Ok(decls)
    }

    fn parse_lexical_decl(&mut self) -> Result<Stmt, Diagnostic> {
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

    /// Binding pattern: identifier, `[a, b, ...rest]`, or `{ a, b: c, ...rest }`.
    fn parse_binding_pattern(&mut self) -> Result<BindingPattern, Diagnostic> {
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
                    let key_tok = self.expect_ident()?;
                    let key = Ident {
                        name: key_tok.ident_name(),
                        span: key_tok.span,
                    };
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
                            span: Span::new(key_tok.span.start.0, end),
                        });
                    } else {
                        // Shorthand `{ a }` or CoverInitializedName `{ a = default }`
                        let default = if self.check(&TokenKind::Eq) {
                            self.bump();
                            Some(self.parse_assignment()?)
                        } else {
                            None
                        };
                        let end = default
                            .as_ref()
                            .map(|d| expr_span(d).end.0)
                            .unwrap_or(key.span.end.0);
                        properties.push(ObjectPatternProp::Prop {
                            key: key.clone(),
                            binding: BindingPattern::Ident(key.clone()),
                            shorthand: true,
                            default,
                            span: Span::new(key.span.start.0, end),
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

    /// Expression: `AssignmentExpression` (`,` `AssignmentExpression`)* left-assoc.
    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_assignment()?;
        while self.check(&TokenKind::Comma) {
            self.bump();
            let right = self.parse_assignment()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Comma,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    /// Right-associative assignment: `target = value` or compound `op=`.
    /// Also covers arrow functions (`params => body`) and `yield` (YieldExpression),
    /// which are AssignmentExpressions in ECMA-262.
    fn parse_assignment(&mut self) -> Result<Expr, Diagnostic> {
        if self.is_arrow_start() {
            return self.parse_arrow_function();
        }
        if self.check(&TokenKind::Yield) {
            return self.parse_yield();
        }
        let left = self.parse_conditional()?;
        let Some(op) = self.peek_assign_op() else {
            return Ok(left);
        };
        self.bump();
        let value = self.parse_assignment()?;
        let span = span_merge(expr_span(&left), expr_span(&value));
        let target = if op == AssignOp::Eq {
            if let Some(pat) = array_expr_to_pattern(&left) {
                pat
            } else if let Some(pat) = object_expr_to_pattern(&left) {
                pat
            } else {
                left
            }
        } else {
            left
        };
        Ok(Expr::Assign {
            target: Box::new(target),
            op,
            value: Box::new(value),
            span,
        })
    }

    /// `yield` / `yield AssignmentExpression` / `yield* AssignmentExpression`.
    /// Bare `yield` / `yield;` → yield undefined (`void 0`).
    fn parse_yield(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::Yield)?.span.start.0;
        let delegate = if self.check(&TokenKind::Star) {
            self.bump();
            true
        } else {
            false
        };
        let arg = if !delegate
            && (self.check(&TokenKind::Semi)
                || self.check(&TokenKind::RBrace)
                || self.check(&TokenKind::RParen)
                || self.check(&TokenKind::RBracket)
                || self.check(&TokenKind::Comma)
                || self.check(&TokenKind::Colon)
                || self.check(&TokenKind::Eof))
        {
            Expr::Unary {
                op: UnaryOp::Void,
                arg: Box::new(Expr::Number(NumberLit {
                    raw: "0".into(),
                    span: Span::new(start, start),
                })),
                span: Span::new(start, start),
            }
        } else {
            // Right-associative: `yield yield 1` and `yield 1 + 2` / `yield x = 1`.
            // `yield*` requires an AssignmentExpression operand.
            self.parse_assignment()?
        };
        let end = expr_span(&arg).end.0;
        Ok(Expr::Unary {
            op: if delegate {
                UnaryOp::YieldStar
            } else {
                UnaryOp::Yield
            },
            arg: Box::new(arg),
            span: Span::new(start, end),
        })
    }

    /// `async? ident =>` or `async? (params) =>` with simple ident params only.
    fn is_arrow_start(&self) -> bool {
        if self.check(&TokenKind::Async) && !self.peek_is(&TokenKind::Function) {
            let next = self.tokens.get(self.pos + 1).map(|t| &t.kind);
            let after = self.tokens.get(self.pos + 2).map(|t| &t.kind);
            if matches!(next, Some(TokenKind::Ident(_))) && matches!(after, Some(TokenKind::Arrow))
            {
                return true;
            }
            if self.peek_is(&TokenKind::LParen) {
                return self.lookahead_paren_arrow_from(self.pos + 1);
            }
            return false;
        }
        if matches!(self.current().kind, TokenKind::Ident(_)) && self.peek_is(&TokenKind::Arrow) {
            return true;
        }
        if self.check(&TokenKind::LParen) {
            return self.lookahead_paren_arrow_from(self.pos);
        }
        false
    }

    fn lookahead_paren_arrow_from(&self, mut i: usize) -> bool {
        if !matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::LParen)
        ) {
            return false;
        }
        i += 1;
        let mut depth = 1usize;
        while i < self.tokens.len() && depth > 0 {
            match &self.tokens[i].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => depth -= 1,
                TokenKind::Eof => return false,
                _ => {}
            }
            i += 1;
        }
        if depth != 0 {
            return false;
        }
        // Optional return type `: Ident` between `)` and `=>`.
        if matches!(self.tokens.get(i).map(|t| &t.kind), Some(TokenKind::Colon)) {
            i += 1;
            if !matches!(
                self.tokens.get(i).map(|t| &t.kind),
                Some(TokenKind::Ident(_))
            ) {
                return false;
            }
            i += 1;
        }
        matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::Arrow)
        )
    }

    /// `async? (params): ret? => body` or bare `async? param => body`.
    fn parse_arrow_function(&mut self) -> Result<Expr, Diagnostic> {
        let (is_async, start) = if self.check(&TokenKind::Async) {
            let start = self.bump().span.start.0;
            (true, start)
        } else {
            (false, self.current().span.start.0)
        };
        let (params, return_type) = if matches!(self.current().kind, TokenKind::Ident(_)) {
            let p = self.expect_ident()?;
            (
                vec![Param {
                    binding: BindingPattern::Ident(Ident {
                        name: p.ident_name(),
                        span: p.span,
                    }),
                    type_ann: None,
                    default: None,
                    rest: false,
                }],
                None,
            )
        } else {
            self.expect(&TokenKind::LParen)?;
            let params = self.parse_param_list()?;
            self.expect(&TokenKind::RParen)?;
            let return_type = self.parse_optional_type_ann()?;
            (params, return_type)
        };
        self.expect(&TokenKind::Arrow)?;
        let body = if self.check(&TokenKind::LBrace) {
            ArrowBody::Block(Box::new(self.parse_block()?))
        } else {
            ArrowBody::Expr(Box::new(self.parse_assignment()?))
        };
        let end = match &body {
            ArrowBody::Block(s) => stmt_span(s).end.0,
            ArrowBody::Expr(e) => expr_span(e).end.0,
        };
        Ok(Expr::ArrowFunction {
            params,
            return_type,
            body,
            is_async,
            span: Span::new(start, end),
        })
    }

    fn peek_assign_op(&self) -> Option<AssignOp> {
        match self.current().kind {
            TokenKind::Eq => Some(AssignOp::Eq),
            TokenKind::PlusEq => Some(AssignOp::AddEq),
            TokenKind::MinusEq => Some(AssignOp::SubEq),
            TokenKind::StarEq => Some(AssignOp::MulEq),
            TokenKind::SlashEq => Some(AssignOp::DivEq),
            TokenKind::PercentEq => Some(AssignOp::RemEq),
            TokenKind::StarStarEq => Some(AssignOp::PowEq),
            TokenKind::ShlEq => Some(AssignOp::ShlEq),
            TokenKind::ShrEq => Some(AssignOp::ShrEq),
            TokenKind::UShrEq => Some(AssignOp::UShrEq),
            TokenKind::BitAndEq => Some(AssignOp::BitAndEq),
            TokenKind::BitOrEq => Some(AssignOp::BitOrEq),
            TokenKind::BitXorEq => Some(AssignOp::BitXorEq),
            TokenKind::AndAndEq => Some(AssignOp::AndAndEq),
            TokenKind::OrOrEq => Some(AssignOp::OrOrEq),
            TokenKind::QuestionQuestionEq => Some(AssignOp::NullishEq),
            _ => None,
        }
    }

    /// Conditional: `test ? AssignmentExpression : AssignmentExpression`.
    fn parse_conditional(&mut self) -> Result<Expr, Diagnostic> {
        let test = self.parse_nullish()?;
        if !self.check(&TokenKind::Question) {
            return Ok(test);
        }
        self.bump();
        let consequent = self.parse_assignment()?;
        self.expect(&TokenKind::Colon)?;
        let alternate = self.parse_assignment()?;
        let span = span_merge(expr_span(&test), expr_span(&alternate));
        Ok(Expr::Conditional {
            test: Box::new(test),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
            span,
        })
    }

    /// Nullish coalescing: left-assoc `??`. Cannot mix with `&&` / `||` without parens.
    fn parse_nullish(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_or()?;
        while self.check(&TokenKind::QuestionQuestion) {
            if is_logical_and_or(&left) {
                return Err(Diagnostic::new(
                    "cannot mix '??' with '&&' or '||' without parentheses".to_string(),
                    self.current().span,
                ));
            }
            self.bump();
            // RHS is BitwiseORExpression (not LogicalOR / LogicalAND).
            let right = self.parse_bit_or()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Nullish,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::OrOr) {
            self.bump();
            let right = self.parse_and()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_bit_or()?;
        while self.check(&TokenKind::AndAnd) {
            self.bump();
            let right = self.parse_bit_or()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_bit_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_bit_xor()?;
        while self.check(&TokenKind::BitOr) {
            self.bump();
            let right = self.parse_bit_xor()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::BitOr,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_bit_xor(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_bit_and()?;
        while self.check(&TokenKind::BitXor) {
            self.bump();
            let right = self.parse_bit_and()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::BitXor,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_bit_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_equality()?;
        while self.check(&TokenKind::BitAnd) {
            self.bump();
            let right = self.parse_equality()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::BitAnd,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_relational()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::EqEq => BinaryOp::EqEq,
                TokenKind::NotEq => BinaryOp::NotEq,
                TokenKind::EqEqEq => BinaryOp::EqEqEq,
                TokenKind::NotEqEq => BinaryOp::NotEqEq,
                _ => break,
            };
            self.bump();
            let right = self.parse_relational()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expr, Diagnostic> {
        // `#name in object` (E18.40) — PrivateIdentifier is only valid as LHS of `in`.
        if self.allow_in {
            if let TokenKind::PrivateIdent(pname) = &self.current().kind {
                let name = pname.clone();
                let name_span = self.bump().span;
                self.expect(&TokenKind::In)?;
                let object = self.parse_shift()?;
                let span = span_merge(name_span, expr_span(&object));
                return Ok(Expr::PrivateIn {
                    name: Ident {
                        name,
                        span: name_span,
                    },
                    object: Box::new(object),
                    span,
                });
            }
        }
        let mut left = self.parse_shift()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::LtEq => BinaryOp::LtEq,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::GtEq => BinaryOp::GtEq,
                TokenKind::In if self.allow_in => BinaryOp::In,
                TokenKind::InstanceOf => BinaryOp::InstanceOf,
                _ => break,
            };
            self.bump();
            let right = self.parse_shift()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_term()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Shl => BinaryOp::Shl,
                TokenKind::Shr => BinaryOp::Shr,
                TokenKind::UShr => BinaryOp::UShr,
                _ => break,
            };
            self.bump();
            let right = self.parse_term()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_factor()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.parse_factor()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_exponentiation()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Rem,
                _ => break,
            };
            self.bump();
            let right = self.parse_exponentiation()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    /// Right-associative `**` (ECMA-262 ExponentiationExpression).
    fn parse_exponentiation(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.parse_unary()?;
        if self.check(&TokenKind::StarStar) {
            self.bump();
            let right = self.parse_exponentiation()?;
            let span = span_merge(expr_span(&left), expr_span(&right));
            return Ok(Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Pow,
                right: Box::new(right),
                span,
            });
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        let update_op = match &self.current().kind {
            TokenKind::PlusPlus => Some(UpdateOp::Inc),
            TokenKind::MinusMinus => Some(UpdateOp::Dec),
            _ => None,
        };
        if let Some(op) = update_op {
            let start = self.bump().span.start.0;
            let arg = self.parse_unary()?;
            let end = expr_span(&arg).end.0;
            return Ok(Expr::Update {
                op,
                arg: Box::new(arg),
                prefix: true,
                span: Span::new(start, end),
            });
        }
        let op = match &self.current().kind {
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Minus => Some(UnaryOp::Minus),
            TokenKind::Bang => Some(UnaryOp::Not),
            TokenKind::Tilde => Some(UnaryOp::BitNot),
            TokenKind::TypeOf => Some(UnaryOp::TypeOf),
            TokenKind::Void => Some(UnaryOp::Void),
            TokenKind::Delete => Some(UnaryOp::Delete),
            TokenKind::Await => Some(UnaryOp::Await),
            // N03.03 native pointers: `&x` address-of, `*p` dereference.
            TokenKind::BitAnd => Some(UnaryOp::Ref),
            TokenKind::Star => Some(UnaryOp::Deref),
            _ => None,
        };
        if let Some(op) = op {
            let start = self.bump().span.start.0;
            let arg = self.parse_unary()?;
            let end = expr_span(&arg).end.0;
            // E19.36: `delete` of MemberExpression/CallExpression.PrivateName is early SyntaxError
            // (class bodies are strict; also covered parenthesized forms).
            if matches!(op, UnaryOp::Delete) && expr_is_private_member_reference(&arg) {
                return Err(Diagnostic::new(
                    "cannot delete private field or method".to_string(),
                    Span::new(start, end),
                ));
            }
            return Ok(Expr::Unary {
                op,
                arg: Box::new(arg),
                span: Span::new(start, end),
            });
        }
        self.parse_as()
    }

    /// Dual-worlds / type boundary: `expr as T` (postfix after update; T06).
    fn parse_as(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_update()?;
        while self.check(&TokenKind::As) {
            self.bump();
            let ty = self.parse_type()?;
            let end = ty.span().end.0;
            let start = expr_span(&expr).start.0;
            expr = Expr::As {
                expr: Box::new(expr),
                ty,
                span: Span::new(start, end),
            };
        }
        Ok(expr)
    }

    /// Postfix update (`lhs++` / `lhs--`) and call.
    fn parse_update(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_lhs()?;
        loop {
            let tok = self.current();
            let op = match &tok.kind {
                TokenKind::PlusPlus => UpdateOp::Inc,
                TokenKind::MinusMinus => UpdateOp::Dec,
                _ => break,
            };
            // ECMA-262: no LineTerminator between LeftHandSideExpression and `++`/`--`.
            if tok.preceded_by_line_terminator {
                break;
            }
            let end = self.bump().span.end.0;
            let start = expr_span(&expr).start.0;
            expr = Expr::Update {
                op,
                arg: Box::new(expr),
                prefix: false,
                span: Span::new(start, end),
            };
        }
        Ok(expr)
    }

    fn parse_lhs(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = if self.check(&TokenKind::New) {
            self.parse_new()?
        } else if self.check(&TokenKind::Import) && self.is_import_call_start() {
            self.parse_import_call()?
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
                expr = self.parse_tagged_template(expr)?;
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// `import(…)`, `import.defer(…)`, or `import.source(…)` lookahead from `import`.
    fn is_import_call_start(&self) -> bool {
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

    /// `import(AssignmentExpression)` / `import(AssignmentExpression, options)` /
    /// `import.defer(AssignmentExpression)` / `import.source(AssignmentExpression)`.
    /// Rest args and empty argument lists are early SyntaxErrors.
    /// Phase forms (`defer` / `source`) accept only one argument (no options).
    fn parse_import_call(&mut self) -> Result<Expr, Diagnostic> {
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
    fn parse_new(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::New)?.span.start.0;
        // `new.target` — meta-property, not a construct expression.
        if self.check(&TokenKind::Dot) {
            self.bump();
            let (name, prop_span) = self.expect_ident_name()?;
            if name != "target" {
                return Err(Diagnostic::new(
                    format!("expected `target` after `new.`, found `{name}`"),
                    prop_span,
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

    fn parse_arg_list(&mut self) -> Result<Vec<Arg>, Diagnostic> {
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

    fn parse_object_expression(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::LBrace)?.span.start.0;
        let mut properties = Vec::new();
        if !self.check(&TokenKind::RBrace) {
            loop {
                properties.push(self.parse_object_prop()?);
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
    fn parse_object_prop(&mut self) -> Result<ObjectProp, Diagnostic> {
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
            let params = self.parse_param_list()?;
            self.expect(&TokenKind::RParen)?;
            if kind == AccessorKind::Get && !params.is_empty() {
                return Err(Diagnostic::new(
                    "getter must have zero parameters".to_string(),
                    self.current_span(),
                ));
            }
            if kind == AccessorKind::Set && params.len() != 1 {
                return Err(Diagnostic::new(
                    "setter must have exactly one parameter".to_string(),
                    self.current_span(),
                ));
            }
            let body = Box::new(self.parse_block()?);
            let end = stmt_span(&body).end.0;
            return Ok(ObjectProp::Accessor {
                kind,
                key,
                params,
                body,
                span: Span::new(prop_start, end),
            });
        }
        // `async m()` / `async *m()` — not property/method named `async`.
        let is_async = if self.check(&TokenKind::Async) && self.peek_starts_method_name() {
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
                let key_expr = self.parse_assignment()?;
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
                // Keywords as IdentifierName keys require `: value` (not bare shorthand).
                let is_keyword_key = !matches!(key_tok.kind, TokenKind::Ident(_));
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
    fn parse_method_function(
        &mut self,
        start: u32,
        is_async: bool,
        is_generator: bool,
    ) -> Result<Expr, Diagnostic> {
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        let return_type = self.parse_optional_type_ann()?;
        let body = Box::new(self.parse_block()?);
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
    }

    /// True when the next token can start a method name after `async` (`m`, `"m"`, `0`, `[`, `*`).
    fn peek_starts_method_name(&self) -> bool {
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
        )
    }

    /// True when next token starts an auto-accessor field name after `accessor` (no LineTerminator).
    fn peek_starts_accessor_field_name(&self) -> bool {
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

    /// True when current token is `get`/`set` and the next token starts an accessor name.
    fn peek_accessor_kind(&self) -> Option<AccessorKind> {
        let kind = match &self.current().kind {
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
            _ => None,
        }
    }

    /// Object literal / accessor property key: IdentifierName (incl. keywords), string, number, or `[expr]`.
    fn parse_object_key(&mut self) -> Result<ObjectKey, Diagnostic> {
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
                let value = value.clone();
                self.bump();
                Ok(ObjectKey::String(StringLit {
                    value,
                    span: tok.span,
                }))
            }
            TokenKind::Number(raw) => {
                let name = numeric_literal_property_name(raw);
                self.bump();
                Ok(ObjectKey::String(StringLit {
                    value: name.into(),
                    span: tok.span,
                }))
            }
            TokenKind::LBracket => {
                self.bump();
                let expr = self.parse_assignment()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(ObjectKey::Computed(Box::new(expr)))
            }
            _ => Err(Diagnostic::new(
                format!("expected property name, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let tok = self.current().clone();
        match &tok.kind {
            TokenKind::Number(raw) => {
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
            TokenKind::TemplateNoSubstitution(_)
            | TokenKind::TemplateHead(_) => self.parse_template_literal(),
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
                self.bump();
                Ok(Expr::Super { span: tok.span })
            }
            TokenKind::Ident(name) if is_reserved_word(name) => Err(Diagnostic::new(
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
            _ => Err(Diagnostic::new(
                format!("expected expression, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    /// `` `…` `` / `` `a${x}b` `` untagged template literal.
    fn parse_template_literal(&mut self) -> Result<Expr, Diagnostic> {
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
    fn parse_tagged_template(&mut self, tag: Expr) -> Result<Expr, Diagnostic> {
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
    fn parse_template_contents(
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
                                format!(
                                    "expected template continuation, found {:?}",
                                    cont.kind
                                ),
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
    fn parse_array_expression(&mut self) -> Result<Expr, Diagnostic> {
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

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn current_span(&self) -> Span {
        self.current().span
    }

    fn check(&self, kind: &TokenKind) -> bool {
        &self.current().kind == kind
    }

    fn peek_is(&self, kind: &TokenKind) -> bool {
        self.tokens
            .get(self.pos + 1)
            .map(|t| &t.kind == kind)
            .unwrap_or(false)
    }

    fn bump(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token, Diagnostic> {
        if self.check(kind) {
            Ok(self.bump())
        } else {
            Err(Diagnostic::new(
                format!("expected {:?}, found {:?}", kind, self.current().kind),
                self.current().span,
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<Token, Diagnostic> {
        let tok = self.current().clone();
        match &tok.kind {
            TokenKind::Ident(name) if is_reserved_word(name) => Err(Diagnostic::new(
                format!("'{name}' is a reserved word and cannot be used as an identifier"),
                tok.span,
            )),
            TokenKind::Ident(_) => {
                self.bump();
                Ok(tok)
            }
            _ => Err(Diagnostic::new(
                format!("expected identifier, found {:?}", tok.kind),
                tok.span,
            )),
        }
    }

    /// IdentifierName after `.` (ECMA-262): Ident or reserved-word keyword.
    fn expect_ident_name(&mut self) -> Result<(String, Span), Diagnostic> {
        let tok = self.current().clone();
        if let Some(name) = tok.ident_name_opt() {
            self.bump();
            Ok((name, tok.span))
        } else {
            Err(Diagnostic::new(
                format!("expected identifier, found {:?}", tok.kind),
                tok.span,
            ))
        }
    }
}

trait IdentName {
    fn ident_name(&self) -> String;
    fn ident_name_opt(&self) -> Option<String>;
}

impl IdentName for Token {
    fn ident_name(&self) -> String {
        self.ident_name_opt().expect("ident name")
    }

    fn ident_name_opt(&self) -> Option<String> {
        match &self.kind {
            TokenKind::Ident(n) => Some(n.clone()),
            TokenKind::True => Some("true".into()),
            TokenKind::False => Some("false".into()),
            TokenKind::Null => Some("null".into()),
            TokenKind::Let => Some("let".into()),
            TokenKind::Const => Some("const".into()),
            TokenKind::Var => Some("var".into()),
            TokenKind::TypeOf => Some("typeof".into()),
            TokenKind::Void => Some("void".into()),
            TokenKind::Delete => Some("delete".into()),
            TokenKind::If => Some("if".into()),
            TokenKind::Else => Some("else".into()),
            TokenKind::While => Some("while".into()),
            TokenKind::Do => Some("do".into()),
            TokenKind::For => Some("for".into()),
            TokenKind::Break => Some("break".into()),
            TokenKind::Continue => Some("continue".into()),
            TokenKind::Switch => Some("switch".into()),
            TokenKind::Case => Some("case".into()),
            TokenKind::Default => Some("default".into()),
            TokenKind::In => Some("in".into()),
            TokenKind::InstanceOf => Some("instanceof".into()),
            TokenKind::Of => Some("of".into()),
            TokenKind::Function => Some("function".into()),
            TokenKind::Async => Some("async".into()),
            TokenKind::Await => Some("await".into()),
            TokenKind::Yield => Some("yield".into()),
            TokenKind::Return => Some("return".into()),
            TokenKind::This => Some("this".into()),
            TokenKind::New => Some("new".into()),
            TokenKind::Class => Some("class".into()),
            TokenKind::Extends => Some("extends".into()),
            TokenKind::Super => Some("super".into()),
            TokenKind::Static => Some("static".into()),
            TokenKind::Throw => Some("throw".into()),
            TokenKind::Try => Some("try".into()),
            TokenKind::Catch => Some("catch".into()),
            TokenKind::Finally => Some("finally".into()),
            TokenKind::With => Some("with".into()),
            TokenKind::Import => Some("import".into()),
            TokenKind::Export => Some("export".into()),
            TokenKind::From => Some("from".into()),
            TokenKind::As => Some("as".into()),
            _ => None,
        }
    }
}

/// ECMA-262 ReservedWord (always reserved; not strict-only FutureReservedWord).
fn is_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

fn object_key_span(key: &ObjectKey) -> Span {
    match key {
        ObjectKey::Ident(id) => id.span,
        ObjectKey::String(s) => s.span,
        ObjectKey::Computed(expr) => expr_span(expr),
    }
}

/// Literal IdentifierName `constructor` only — not `"constructor"` or `['constructor']`.
fn class_key_is_literal_constructor(key: &ObjectKey) -> bool {
    matches!(key, ObjectKey::Ident(id) if id.name == "constructor")
}

/// PropName of a non-computed class element key (Ident / String / numeric→String).
fn class_element_prop_name(key: &ObjectKey) -> Option<String> {
    match key {
        ObjectKey::Ident(id) => Some(id.name.clone()),
        ObjectKey::String(s) => Some(s.value.to_string_lossy()),
        ObjectKey::Computed(_) => None,
    }
}

/// NumericLiteral property name → ToString(MV) string key (ECMA-262 LiteralPropertyName).
fn numeric_literal_property_name(raw: &str) -> String {
    let s: String = raw.chars().filter(|&c| c != '_').collect();
    let lower = s.to_ascii_lowercase();
    if let Some(hex) = lower.strip_prefix("0x") {
        if let Ok(n) = u64::from_str_radix(hex, 16) {
            return n.to_string();
        }
    } else if let Some(bin) = lower.strip_prefix("0b") {
        if let Ok(n) = u64::from_str_radix(bin, 2) {
            return n.to_string();
        }
    } else if let Some(oct) = lower.strip_prefix("0o") {
        if let Ok(n) = u64::from_str_radix(oct, 8) {
            return n.to_string();
        }
    } else if let Ok(n) = s.parse::<f64>() {
        return js_number_to_property_key(n);
    }
    s
}

fn js_number_to_property_key(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n.is_sign_positive() {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    if n == 0.0 {
        return "0".into();
    }
    if n.fract() == 0.0 && n.abs() <= 9007199254740991.0 {
        if n < 0.0 {
            return format!("-{}", (-n) as u64);
        }
        return format!("{}", n as u64);
    }
    let mut s = format!("{n}");
    if let Some(stripped) = s.strip_suffix(".0") {
        s = stripped.to_string();
    }
    s
}

/// True when `expr` is (possibly parenthesized) `….#private` (E19.36 delete early error).
fn expr_is_private_member_reference(expr: &Expr) -> bool {
    match expr {
        Expr::Paren { expr: inner, .. } => expr_is_private_member_reference(inner),
        Expr::MemberExpression { private: true, .. } => true,
        _ => false,
    }
}

/// ClassBody early errors: duplicate privates, field PropName, SuperCall/arguments in field init.
fn validate_class_body(body: &[ClassElement]) -> Result<(), Diagnostic> {
    let mut private_names: Vec<String> = Vec::new();
    for el in body {
        match el {
            ClassElement::Field {
                key,
                value,
                is_static,
                is_private,
                span,
            } => {
                if *is_private {
                    if let ObjectKey::Ident(id) = key {
                        if private_names.iter().any(|n| n == &id.name) {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        private_names.push(id.name.clone());
                    }
                } else if let Some(name) = class_element_prop_name(key) {
                    if name == "constructor" {
                        return Err(Diagnostic::new(
                            "class field cannot be named constructor".to_string(),
                            *span,
                        ));
                    }
                    if *is_static && name == "prototype" {
                        return Err(Diagnostic::new(
                            "static class field cannot be named prototype".to_string(),
                            *span,
                        ));
                    }
                }
                if let Some(v) = value {
                    if expr_contains_super_call(v) {
                        return Err(Diagnostic::new(
                            "class field initializer cannot contain super call".to_string(),
                            *span,
                        ));
                    }
                    if expr_contains_arguments_ref(v) {
                        return Err(Diagnostic::new(
                            "class field initializer cannot contain arguments".to_string(),
                            *span,
                        ));
                    }
                }
            }
            ClassElement::Method {
                key,
                is_static,
                is_private,
                span,
                ..
            } => {
                if *is_private {
                    if let ObjectKey::Ident(id) = key {
                        if private_names.iter().any(|n| n == &id.name) {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        private_names.push(id.name.clone());
                    }
                } else if *is_static {
                    if let Some(name) = class_element_prop_name(key) {
                        if name == "prototype" {
                            return Err(Diagnostic::new(
                                "static class method cannot be named prototype".to_string(),
                                *span,
                            ));
                        }
                    }
                }
            }
            ClassElement::Accessor {
                key,
                is_static,
                is_private,
                span,
                ..
            } => {
                if *is_private {
                    if let ObjectKey::Ident(id) = key {
                        // get/set pair may share one PrivateBoundName (allow up to 2).
                        if private_names.iter().filter(|n| *n == &id.name).count() >= 2 {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        private_names.push(id.name.clone());
                    }
                } else if *is_static {
                    if let Some(name) = class_element_prop_name(key) {
                        if name == "prototype" {
                            return Err(Diagnostic::new(
                                "static class accessor cannot be named prototype".to_string(),
                                *span,
                            ));
                        }
                    }
                }
            }
            ClassElement::Constructor { .. } | ClassElement::StaticBlock { .. } => {}
        }
    }
    Ok(())
}

/// `Contains SuperCall` for field initializers: recurse into arrows; skip nested functions/classes.
fn expr_contains_super_call(expr: &Expr) -> bool {
    match expr {
        Expr::Call { callee, args, .. } => {
            matches!(callee.as_ref(), Expr::Super { .. })
                || expr_contains_super_call(callee)
                || args.iter().any(arg_contains_super_call)
        }
        Expr::ArrowFunction { body, params, .. } => {
            params
                .iter()
                .any(|p| p.default.as_ref().is_some_and(expr_contains_super_call))
                || match body {
                    ArrowBody::Expr(e) => expr_contains_super_call(e),
                    ArrowBody::Block(s) => stmt_contains_super_call(s),
                }
        }
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_super_call(inner),
        Expr::Binary { left, right, .. } | Expr::Assign { target: left, value: right, .. } => {
            expr_contains_super_call(left) || expr_contains_super_call(right)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super_call(test)
                || expr_contains_super_call(consequent)
                || expr_contains_super_call(alternate)
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_super_call(object) || expr_contains_super_call(property),
        Expr::New { callee, args, .. } => {
            expr_contains_super_call(callee) || args.iter().any(arg_contains_super_call)
        }
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_super_call(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value, .. } => {
                object_key_contains_super_call(key) || expr_contains_super_call(value)
            }
            ObjectProp::Spread { expr, .. } => expr_contains_super_call(expr),
            ObjectProp::Accessor { key, .. } => object_key_contains_super_call(key),
        }),
        Expr::TemplateLiteral { expressions, .. } => {
            expressions.iter().any(expr_contains_super_call)
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => expr_contains_super_call(tag) || expressions.iter().any(expr_contains_super_call),
        Expr::ImportCall {
            source, options, ..
        } => {
            expr_contains_super_call(source)
                || options.as_ref().is_some_and(|o| expr_contains_super_call(o))
        }
        Expr::PrivateIn { object, .. } => expr_contains_super_call(object),
        Expr::ArrayPattern { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Pattern { default, .. } => {
                default.as_ref().is_some_and(expr_contains_super_call)
            }
            _ => false,
        }),
        Expr::ObjectPattern { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop { default, .. } => {
                default.as_ref().is_some_and(expr_contains_super_call)
            }
            _ => false,
        }),
        _ => false,
    }
}

fn arg_contains_super_call(a: &Arg) -> bool {
    match a {
        Arg::Expr(e) | Arg::Spread(e) => expr_contains_super_call(e),
    }
}

fn object_key_contains_super_call(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_super_call(e),
        ObjectKey::Ident(_) | ObjectKey::String(_) => false,
    }
}

fn stmt_contains_super_call(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_super_call),
        Stmt::Expression { expr, .. } => expr_contains_super_call(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_super_call),
        Stmt::Throw { argument, .. } => expr_contains_super_call(argument),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super_call(test)
                || stmt_contains_super_call(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_contains_super_call(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_contains_super_call(test) || stmt_contains_super_call(body)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_super_call),
        _ => false,
    }
}

/// `ContainsArguments` for field initializers: recurse into arrows; skip nested functions/classes.
fn expr_contains_arguments_ref(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(id) if id.name == "arguments" => true,
        Expr::ArrowFunction { body, params, .. } => {
            params.iter().any(|p| {
                p.default.as_ref().is_some_and(expr_contains_arguments_ref)
                    || binding_contains_arguments(&p.binding)
            }) || match body {
                ArrowBody::Expr(e) => expr_contains_arguments_ref(e),
                ArrowBody::Block(s) => stmt_contains_arguments_ref(s),
            }
        }
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_arguments_ref(inner),
        Expr::Binary { left, right, .. } | Expr::Assign { target: left, value: right, .. } => {
            expr_contains_arguments_ref(left) || expr_contains_arguments_ref(right)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_arguments_ref(test)
                || expr_contains_arguments_ref(consequent)
                || expr_contains_arguments_ref(alternate)
        }
        Expr::MemberExpression {
            object,
            property,
            computed,
            ..
        } => {
            expr_contains_arguments_ref(object)
                || (*computed && expr_contains_arguments_ref(property))
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_arguments_ref(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_arguments_ref(e),
                })
        }
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_arguments_ref(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value, .. } => {
                object_key_contains_arguments(key) || expr_contains_arguments_ref(value)
            }
            ObjectProp::Spread { expr, .. } => expr_contains_arguments_ref(expr),
            ObjectProp::Accessor { key, .. } => object_key_contains_arguments(key),
        }),
        Expr::TemplateLiteral { expressions, .. } => {
            expressions.iter().any(expr_contains_arguments_ref)
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            expr_contains_arguments_ref(tag) || expressions.iter().any(expr_contains_arguments_ref)
        }
        Expr::ImportCall {
            source, options, ..
        } => {
            expr_contains_arguments_ref(source)
                || options
                    .as_ref()
                    .is_some_and(|o| expr_contains_arguments_ref(o))
        }
        Expr::PrivateIn { object, .. } => expr_contains_arguments_ref(object),
        Expr::ArrayPattern { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Pattern { binding, default } => {
                binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ArrayPatternElement::Rest(b) => binding_contains_arguments(b),
            ArrayPatternElement::Elision => false,
        }),
        Expr::ObjectPattern { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                binding, default, ..
            } => {
                binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ObjectPatternProp::Rest(b) => binding_contains_arguments(b),
        }),
        _ => false,
    }
}

fn object_key_contains_arguments(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_arguments_ref(e),
        ObjectKey::Ident(_) | ObjectKey::String(_) => false,
    }
}

fn binding_contains_arguments(b: &BindingPattern) -> bool {
    match b {
        BindingPattern::Ident(id) => id.name == "arguments",
        BindingPattern::Member(e) => expr_contains_arguments_ref(e),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                binding, default, ..
            } => {
                binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ObjectPatternProp::Rest(inner) => binding_contains_arguments(inner),
        }),
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Pattern { binding, default } => {
                binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ArrayPatternElement::Rest(inner) => binding_contains_arguments(inner),
            ArrayPatternElement::Elision => false,
        }),
    }
}

fn stmt_contains_arguments_ref(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_arguments_ref),
        Stmt::Expression { expr, .. } => expr_contains_arguments_ref(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_arguments_ref),
        Stmt::Throw { argument, .. } => expr_contains_arguments_ref(argument),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_arguments_ref(test)
                || stmt_contains_arguments_ref(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_contains_arguments_ref(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_contains_arguments_ref(test) || stmt_contains_arguments_ref(body)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_arguments_ref),
        _ => false,
    }
}

fn expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Ident(i) => i.span,
        Expr::Number(n) => n.span,
        Expr::BigInt(n) => n.span,
        Expr::String(s) => s.span,
        Expr::RegExp { span, .. } => *span,
        Expr::Boolean { span, .. }
        | Expr::Null { span }
        | Expr::This { span }
        | Expr::Super { span }
        | Expr::NewTarget { span }
        | Expr::ImportCall { span, .. }
        | Expr::TemplateLiteral { span, .. }
        | Expr::TaggedTemplate { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Conditional { span, .. }
        | Expr::Assign { span, .. }
        | Expr::Update { span, .. }
        | Expr::Call { span, .. }
        | Expr::New { span, .. }
        | Expr::FunctionExpression { span, .. }
        | Expr::ClassExpression { span, .. }
        | Expr::ArrowFunction { span, .. }
        | Expr::ObjectExpression { span, .. }
        | Expr::ArrayExpression { span, .. }
        | Expr::ArrayPattern { span, .. }
        | Expr::ObjectPattern { span, .. }
        | Expr::MemberExpression { span, .. }
        | Expr::PrivateIn { span, .. }
        | Expr::Paren { span, .. }
        | Expr::As { span, .. } => *span,
    }
}

/// Split `pat = default` assignment into binding + default for pattern elements.
fn expr_to_pattern_element(expr: &Expr) -> Option<(BindingPattern, Option<Expr>)> {
    if let Expr::Assign {
        target,
        op: AssignOp::Eq,
        value,
        ..
    } = expr
    {
        let binding = expr_to_binding_pattern(target)?;
        return Some((binding, Some((**value).clone())));
    }
    let binding = expr_to_binding_pattern(expr)?;
    Some((binding, None))
}

/// Reinterpret an array literal as an assignment pattern when every element is
/// a binding/LHS target (`ident`, member, `pat = default`, nested pattern, elision, or trailing rest).
fn array_expr_to_pattern(expr: &Expr) -> Option<Expr> {
    let Expr::ArrayExpression {
        elements,
        trailing_comma,
        span,
    } = expr
    else {
        return None;
    };
    let mut pat_els = Vec::with_capacity(elements.len());
    let mut saw_rest = false;
    for el in elements {
        if saw_rest {
            return None;
        }
        match el {
            ArrayElement::Elision => {
                pat_els.push(ArrayPatternElement::Elision);
            }
            ArrayElement::Expr(inner) => {
                let (binding, default) = expr_to_pattern_element(inner)?;
                pat_els.push(ArrayPatternElement::Pattern { binding, default });
            }
            ArrayElement::Spread(inner) => {
                let binding = expr_to_binding_pattern(inner)?;
                pat_els.push(ArrayPatternElement::Rest(binding));
                saw_rest = true;
            }
        }
    }
    // `[...x,]` — trailing comma after rest is a SyntaxError in assignment patterns.
    if *trailing_comma && saw_rest {
        return None;
    }
    Some(Expr::ArrayPattern {
        elements: pat_els,
        span: *span,
    })
}

/// Reinterpret an object literal as an assignment pattern when every property is
/// a binding target (shorthand, CoverInitializedName, `key: pattern`, or trailing `...ident`).
fn object_expr_to_pattern(expr: &Expr) -> Option<Expr> {
    let Expr::ObjectExpression { properties, span } = expr else {
        return None;
    };
    let mut props = Vec::with_capacity(properties.len());
    let mut saw_rest = false;
    for prop in properties {
        if saw_rest {
            return None;
        }
        match prop {
            ObjectProp::Property {
                key,
                value,
                shorthand,
                span: prop_span,
            } => {
                let ObjectKey::Ident(key_id) = key else {
                    return None;
                };
                // CoverInitializedName: `{ a = default }` encoded as shorthand Assign.
                if *shorthand {
                    if let Expr::Assign {
                        target,
                        op: AssignOp::Eq,
                        value: def,
                        ..
                    } = value
                    {
                        let Expr::Ident(id) = target.as_ref() else {
                            return None;
                        };
                        if id.name != key_id.name {
                            return None;
                        }
                        props.push(ObjectPatternProp::Prop {
                            key: key_id.clone(),
                            binding: BindingPattern::Ident(id.clone()),
                            shorthand: true,
                            default: Some((**def).clone()),
                            span: *prop_span,
                        });
                        continue;
                    }
                }
                let (binding, default) = expr_to_pattern_element(value)?;
                props.push(ObjectPatternProp::Prop {
                    key: key_id.clone(),
                    binding,
                    shorthand: *shorthand,
                    default,
                    span: *prop_span,
                });
            }
            ObjectProp::Spread { expr: inner, .. } => {
                let binding = expr_to_binding_pattern(inner)?;
                props.push(ObjectPatternProp::Rest(binding));
                saw_rest = true;
            }
            ObjectProp::Accessor { .. } => return None,
        }
    }
    Some(Expr::ObjectPattern {
        properties: props,
        span: *span,
    })
}

fn expr_to_binding_pattern(expr: &Expr) -> Option<BindingPattern> {
    match expr {
        Expr::Ident(id) => Some(BindingPattern::Ident(id.clone())),
        Expr::MemberExpression {
            optional: false,
            private: false,
            ..
        } => Some(BindingPattern::Member(Box::new(expr.clone()))),
        Expr::ArrayExpression {
            elements,
            trailing_comma,
            span,
        } => {
            let mut pat_els = Vec::with_capacity(elements.len());
            let mut saw_rest = false;
            for el in elements {
                if saw_rest {
                    return None;
                }
                match el {
                    ArrayElement::Elision => {
                        pat_els.push(ArrayPatternElement::Elision);
                    }
                    ArrayElement::Expr(inner) => {
                        let (binding, default) = expr_to_pattern_element(inner)?;
                        pat_els.push(ArrayPatternElement::Pattern { binding, default });
                    }
                    ArrayElement::Spread(inner) => {
                        let binding = expr_to_binding_pattern(inner)?;
                        pat_els.push(ArrayPatternElement::Rest(binding));
                        saw_rest = true;
                    }
                }
            }
            if *trailing_comma && saw_rest {
                return None;
            }
            Some(BindingPattern::Array {
                elements: pat_els,
                span: *span,
            })
        }
        Expr::ArrayPattern { elements, span } => Some(BindingPattern::Array {
            elements: elements.clone(),
            span: *span,
        }),
        Expr::ObjectExpression { properties, span } => {
            let mut props = Vec::with_capacity(properties.len());
            let mut saw_rest = false;
            for prop in properties {
                if saw_rest {
                    return None;
                }
                match prop {
                    ObjectProp::Property {
                        key,
                        value,
                        shorthand,
                        span: prop_span,
                    } => {
                        let ObjectKey::Ident(key_id) = key else {
                            return None;
                        };
                        if *shorthand {
                            if let Expr::Assign {
                                target,
                                op: AssignOp::Eq,
                                value: def,
                                ..
                            } = value
                            {
                                let Expr::Ident(id) = target.as_ref() else {
                                    return None;
                                };
                                if id.name != key_id.name {
                                    return None;
                                }
                                props.push(ObjectPatternProp::Prop {
                                    key: key_id.clone(),
                                    binding: BindingPattern::Ident(id.clone()),
                                    shorthand: true,
                                    default: Some((**def).clone()),
                                    span: *prop_span,
                                });
                                continue;
                            }
                        }
                        let (binding, default) = expr_to_pattern_element(value)?;
                        props.push(ObjectPatternProp::Prop {
                            key: key_id.clone(),
                            binding,
                            shorthand: *shorthand,
                            default,
                            span: *prop_span,
                        });
                    }
                    ObjectProp::Spread { expr: inner, .. } => {
                        let binding = expr_to_binding_pattern(inner)?;
                        props.push(ObjectPatternProp::Rest(binding));
                        saw_rest = true;
                    }
                    ObjectProp::Accessor { .. } => return None,
                }
            }
            Some(BindingPattern::Object {
                properties: props,
                span: *span,
            })
        }
        Expr::ObjectPattern { properties, span } => Some(BindingPattern::Object {
            properties: properties.clone(),
            span: *span,
        }),
        _ => None,
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Expression { span, .. }
        | Stmt::Let { span, .. }
        | Stmt::Empty { span }
        | Stmt::Block { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoWhile { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForIn { span, .. }
        | Stmt::ForOf { span, .. }
        | Stmt::Break { span, .. }
        | Stmt::Continue { span, .. }
        | Stmt::Labeled { span, .. }
        | Stmt::Switch { span, .. }
        | Stmt::FunctionDeclaration { span, .. }
        | Stmt::ClassDeclaration { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Throw { span, .. }
        | Stmt::Try { span, .. }
        | Stmt::With { span, .. }
        | Stmt::ImportDeclaration { span, .. }
        | Stmt::ExportNamedDeclaration { span, .. }
        | Stmt::ExportDefaultDeclaration { span, .. }
        | Stmt::ExportAllDeclaration { span, .. }
        | Stmt::TypeAlias { span, .. } => *span,
    }
}

fn is_logical_and_or(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Binary {
            op: BinaryOp::And | BinaryOp::Or,
            ..
        }
    )
}

fn span_merge(a: Span, b: Span) -> Span {
    Span::new(a.start.0, b.end.0)
}

/// Helper for tests and CLI.
pub fn parse_and_dump(source: &str) -> Result<String, Diagnostic> {
    let program = parse(source)?;
    Ok(dump_program(&program))
}

#[cfg(test)]
mod tests {
    use super::*;

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
            err.message.contains("const declaration requires an initializer"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn parse_for_const_of() {
        let dump = parse_and_dump(r#"for (const c of "ab") c;"#).unwrap();
        assert!(dump.contains("ForOf"), "got:\n{dump}");
        assert!(dump.contains("Const"), "got:\n{dump}");
    }

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
    fn parse_for_with_let_init() {
        let dump = parse_and_dump("for (let i = 0; i < 3; i = i + 1) { x = x + 1; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  For
    init:
      Let
        name: i
        init:
          Number 0
    test:
      Binary <
        Ident i
        Number 3
    update:
      Assign =
        Ident i
        Binary +
          Ident i
          Number 1
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
    fn parse_for_omitted_clauses() {
        let dump = parse_and_dump("for (;;) x = 1;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  For
    body:
      ExpressionStatement
        Assign =
          Ident x
          Number 1
"
        );
    }

    #[test]
    fn parse_for_in_let() {
        let dump = parse_and_dump("for (let k in s) { x = k; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForIn
    left:
      Let
        name: k
    right:
      Ident s
    body:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Ident k
"
        );
    }

    #[test]
    fn parse_for_in_var() {
        let dump = parse_and_dump("for (var k in s) { x = k; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForIn
    left:
      Var
        name: k
    right:
      Ident s
    body:
      Block
        ExpressionStatement
          Assign =
            Ident x
            Ident k
"
        );
    }

    #[test]
    fn parse_for_var_classic() {
        let dump = parse_and_dump("for (var i = 0; i < 3; i = i + 1) sum = i;").unwrap();
        assert!(dump.contains("Var"), "{dump}");
        assert!(dump.contains("For\n"), "{dump}");
    }

    #[test]
    fn parse_for_in_var_init_annex_b() {
        let dump = parse_and_dump("for (var k = 1 in s) x = k;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForIn
    left:
      Var
        name: k
        init:
          Number 1
    right:
      Ident s
    body:
      ExpressionStatement
        Assign =
          Ident x
          Ident k
"
        );
    }

    #[test]
    fn parse_for_of_var_init_rejected() {
        let err = parse_and_dump("for (var k = 1 of s) x = k;").unwrap_err();
        assert!(
            err.message.contains("for-of binding cannot have an initializer"),
            "{err:?}"
        );
    }

    #[test]
    fn parse_for_of_let() {
        let dump = parse_and_dump("for (let c of s) x = c;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForOf
    left:
      Let
        name: c
    right:
      Ident s
    body:
      ExpressionStatement
        Assign =
          Ident x
          Ident c
"
        );
    }

    #[test]
    fn parse_for_of_const_array_pattern() {
        let dump = parse_and_dump("for (const [a] of s) x = a;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForOf
    left:
      Const
        ArrayPattern
          name: a
    right:
      Ident s
    body:
      ExpressionStatement
        Assign =
          Ident x
          Ident a
"
        );
    }

    #[test]
    fn parse_for_of_let_object_pattern() {
        let dump = parse_and_dump("for (let {x} of s) y = x;").unwrap();
        assert!(dump.contains("ForOf"), "got:\n{dump}");
        assert!(dump.contains("ObjectPattern"), "got:\n{dump}");
        assert!(dump.contains("name: x"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_in_var_array_pattern() {
        let dump = parse_and_dump("for (var [a] in s) x = a;").unwrap();
        assert!(dump.contains("ForIn"), "got:\n{dump}");
        assert!(dump.contains("ArrayPattern"), "got:\n{dump}");
        assert!(dump.contains("Var"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_of_assign_array_pattern() {
        let dump = parse_and_dump("for ([a] of s) {}").unwrap();
        assert!(dump.contains("ForOf"), "got:\n{dump}");
        assert!(dump.contains("ArrayPattern"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_classic_let_array_pattern() {
        let dump = parse_and_dump("for (let [a] = arr; a; ) x = a;").unwrap();
        assert!(dump.contains("For\n"), "got:\n{dump}");
        assert!(dump.contains("ArrayPattern"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_of_assign_target() {
        let dump = parse_and_dump("for (x of s) {}").unwrap();
        assert_eq!(
            dump,
            "\
Program
  ForOf
    left:
      ExpressionStatement
        Ident x
    right:
      Ident s
    body:
      Block
"
        );
    }

    #[test]
    fn parse_for_await_of_let() {
        let dump = parse_and_dump("async function f() { for await (let x of a) { y = x; } }").unwrap();
        assert!(dump.contains("ForOf await"), "got:\n{dump}");
        assert!(dump.contains("name: x"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_await_of_assign() {
        let dump = parse_and_dump("async function f() { for await (x of a) {} }").unwrap();
        assert!(dump.contains("ForOf await"), "got:\n{dump}");
    }

    #[test]
    fn parse_for_await_in_rejected() {
        let err = parse_and_dump("async function f() { for await (let x in a) {} }").unwrap_err();
        assert!(
            err.message.contains("for await") && err.message.contains("in"),
            "{err:?}"
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
        let dump =
            parse_and_dump("outer: while (true) { break outer; continue outer; }").unwrap();
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
        let dump = parse_and_dump(
            "switch (x) { case 1: a = 1; break; case 2: a = 2; default: a = 0; }",
        )
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
    fn parse_function_decl_return() {
        let dump = parse_and_dump("function f() { return 1; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  FunctionDeclaration
    name: f
    body:
      Block
        Return
          Number 1
"
        );
    }

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
    fn parse_arrow_expression_body() {
        let dump = parse_and_dump("let f = (a) => a;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: f
    init:
      ArrowFunction
        params:
          name: a
        body:
          Ident a
"
        );
    }

    #[test]
    fn parse_async_arrow() {
        let dump = parse_and_dump(
            "let f = async () => 1; let g = async (x) => { return await x; }; let h = async y => y;",
        )
        .unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: f
    init:
      ArrowFunction
        async: true
        body:
          Number 1
  Let
    name: g
    init:
      ArrowFunction
        async: true
        params:
          name: x
        body:
          Block
            Return
              Unary await
                Ident x
  Let
    name: h
    init:
      ArrowFunction
        async: true
        params:
          name: y
        body:
          Ident y
"
        );
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
    fn parse_arrow_block_body_and_bare_param() {
        let dump = parse_and_dump("let f = x => { return x; }; let g = () => 1;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: f
    init:
      ArrowFunction
        params:
          name: x
        body:
          Block
            Return
              Ident x
  Let
    name: g
    init:
      ArrowFunction
        body:
          Number 1
"
        );
    }

    #[test]
    fn parse_default_params() {
        let dump = parse_and_dump("function f(a = 1, b) { return a + b; } let g = (x = 2) => x;").unwrap();
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
    fn parse_object_literal_and_member() {
        let dump = parse_and_dump(r#"let o = { a: 1, "b": 2 }; let x = o.a; let y = o["b"];"#)
            .unwrap();
        assert!(dump.contains("ObjectExpression"));
        assert!(dump.contains("key: Ident a"));
        assert!(dump.contains("key: String \"b\""));
        assert!(dump.contains("MemberExpression\n"));
        assert!(dump.contains("MemberExpression computed"));
    }

    #[test]
    fn parse_member_keyword_ident_name() {
        // IdentifierName after `.` may be a reserved word (e.g. Symbol.for).
        let dump = parse_and_dump("Symbol.for; obj.default;").unwrap();
        assert!(dump.contains("Ident for"), "got:\n{dump}");
        assert!(dump.contains("Ident default"), "got:\n{dump}");
    }

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
        let dump = parse_and_dump("f(a,); g(a, b,); h(...a,); i(1, ...b,); new C(x,); new D(...y,);")
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
    fn parse_object_literal_sugar() {
        let dump = parse_and_dump("let a = 1; let k = \"z\"; let o = { a, m() { return 1; }, [k]: 2 };")
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

    #[test]
    fn parse_private_accessors() {
        let dump = parse_and_dump(
            "class C { get #x() { return 1; } set #x(v) {} static get #y() { return 2; } static set #y(v) {} }",
        )
        .unwrap();
        assert!(dump.contains("PrivateAccessor get"), "{dump}");
        assert!(dump.contains("PrivateAccessor set"), "{dump}");
        assert!(dump.contains("StaticPrivateAccessor get"), "{dump}");
        assert!(dump.contains("StaticPrivateAccessor set"), "{dump}");
        assert!(dump.contains("name: #x"), "{dump}");
        assert!(dump.contains("name: #y"), "{dump}");
    }

    #[test]
    fn parse_private_in() {
        let dump = parse_and_dump(
            "class C { #x = 1; m(o) { return #x in o; } }",
        )
        .unwrap();
        assert!(dump.contains("PrivateIn"), "{dump}");
        assert!(dump.contains("name: #x"), "{dump}");
        assert!(dump.contains("Ident o"), "{dump}");
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
        let dump = parse_and_dump("let p = new Point(1, 2); let q = new Foo; let x = new A().b;").unwrap();
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
    fn parse_unary_and_bool() {
        let dump = parse_and_dump("let ok = !false;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: ok
    init:
      Unary !
        Boolean false
"
        );
    }

    #[test]
    fn parse_string_and_null() {
        let dump = parse_and_dump(r#"let s = "hi"; let n = null;"#).unwrap();
        assert!(dump.contains("String \"hi\""));
        assert!(dump.contains("Null"));
    }

    #[test]
    fn parse_comparison() {
        let dump = parse_and_dump("a === b && c !== d;").unwrap();
        assert!(dump.contains("Binary ==="));
        assert!(dump.contains("Binary &&"));
        assert!(dump.contains("Binary !=="));
    }

    #[test]
    fn parse_in_operator() {
        let dump = parse_and_dump(r#""a" in obj;"#).unwrap();
        assert!(dump.contains("Binary in"), "dump={dump}");
        assert!(dump.contains("String \"a\""));
        assert!(dump.contains("Ident obj"));
    }

    #[test]
    fn parse_bitwise_precedence() {
        let dump = parse_and_dump("let x = 1 | 2 & 4;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary |
        Number 1
        Binary &
          Number 2
          Number 4
"
        );
    }

    #[test]
    fn parse_bitwise_not_and_shift() {
        let dump = parse_and_dump("let x = ~1 << 2;").unwrap();
        assert!(dump.contains("Binary <<"));
        assert!(dump.contains("Unary ~"));
    }

    #[test]
    fn parse_exponentiation_right_assoc() {
        let dump = parse_and_dump("let x = 2 ** 3 ** 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary **
        Number 2
        Binary **
          Number 3
          Number 2
"
        );
    }

    #[test]
    fn parse_exponentiation_precedence() {
        let dump = parse_and_dump("let x = 2 * 3 ** 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary *
        Number 2
        Binary **
          Number 3
          Number 2
"
        );
    }

    #[test]
    fn parse_conditional() {
        let dump = parse_and_dump("let x = true ? 1 : 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Conditional
        Boolean true
        Number 1
        Number 2
"
        );
    }

    #[test]
    fn parse_conditional_right_assoc() {
        let dump = parse_and_dump("let x = a ? b : c ? d : e;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Conditional
        Ident a
        Ident b
        Conditional
          Ident c
          Ident d
          Ident e
"
        );
    }

    #[test]
    fn parse_conditional_nested_consequent() {
        let dump = parse_and_dump("let x = a ? b ? c : d : e;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Conditional
        Ident a
        Conditional
          Ident b
          Ident c
          Ident d
        Ident e
"
        );
    }

    #[test]
    fn parse_assignment() {
        let dump = parse_and_dump("let x; x = 1;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
  ExpressionStatement
    Assign =
      Ident x
      Number 1
"
        );
    }

    #[test]
    fn parse_assignment_right_assoc() {
        let dump = parse_and_dump("let a; let b; a = b = 1;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: a
  Let
    name: b
  ExpressionStatement
    Assign =
      Ident a
      Assign =
        Ident b
        Number 1
"
        );
    }

    #[test]
    fn parse_assignment_in_conditional_alternate() {
        let dump = parse_and_dump("let a; let x = true ? 1 : a = 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: a
  Let
    name: x
    init:
      Conditional
        Boolean true
        Number 1
        Assign =
          Ident a
          Number 2
"
        );
    }

    #[test]
    fn parse_nullish() {
        let dump = parse_and_dump("let x = null ?? 1;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary ??
        Null
        Number 1
"
        );
    }

    #[test]
    fn parse_nullish_left_assoc() {
        let dump = parse_and_dump("let x = a ?? b ?? c;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Binary ??
        Binary ??
          Ident a
          Ident b
        Ident c
"
        );
    }

    #[test]
    fn parse_nullish_rejects_mix_with_or() {
        let err = parse_and_dump("let x = a || b ?? c;").unwrap_err();
        assert!(
            err.message.contains("??") && err.message.contains("||"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn parse_nullish_allows_paren_mix() {
        let dump = parse_and_dump("let x = (a || b) ?? c;").unwrap();
        assert!(dump.contains("Binary ??"));
        assert!(dump.contains("Paren"));
        assert!(dump.contains("Binary ||"));
    }

    #[test]
    fn parse_logical_assignment() {
        let dump = parse_and_dump("let x = 1; x &&= 2; x ||= 3; x ??= 4;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Number 1
  ExpressionStatement
    Assign &&=
      Ident x
      Number 2
  ExpressionStatement
    Assign ||=
      Ident x
      Number 3
  ExpressionStatement
    Assign ??=
      Ident x
      Number 4
"
        );
    }

    #[test]
    fn parse_compound_assignment() {
        let dump = parse_and_dump("let x = 1; x += 2;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Number 1
  ExpressionStatement
    Assign +=
      Ident x
      Number 2
"
        );
    }

    #[test]
    fn parse_compound_assignment_right_assoc() {
        let dump = parse_and_dump("let a = 1; let b = 2; a += b += 3;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: a
    init:
      Number 1
  Let
    name: b
    init:
      Number 2
  ExpressionStatement
    Assign +=
      Ident a
      Assign +=
        Ident b
        Number 3
"
        );
    }

    #[test]
    fn parse_update_prefix() {
        let dump = parse_and_dump("let x = 1; ++x; --x;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Number 1
  ExpressionStatement
    Update prefix ++
      Ident x
  ExpressionStatement
    Update prefix --
      Ident x
"
        );
    }

    #[test]
    fn parse_update_postfix() {
        let dump = parse_and_dump("let x = 1; x++; x--;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Number 1
  ExpressionStatement
    Update postfix ++
      Ident x
  ExpressionStatement
    Update postfix --
      Ident x
"
        );
    }

    #[test]
    fn parse_update_in_init() {
        let dump = parse_and_dump("let a = 1; let b = a++; let c = ++a;").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: a
    init:
      Number 1
  Let
    name: b
    init:
      Update postfix ++
        Ident a
  Let
    name: c
    init:
      Update prefix ++
        Ident a
"
        );
    }

    #[test]
    fn parse_comma() {
        let dump = parse_and_dump("let x = (1, 2);").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Paren
        Binary ,
          Number 1
          Number 2
"
        );
    }

    #[test]
    fn parse_comma_left_assoc() {
        let dump = parse_and_dump("let x = (1, 2, 3);").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Paren
        Binary ,
          Binary ,
            Number 1
            Number 2
          Number 3
"
        );
    }

    #[test]
    fn parse_comma_with_assignment() {
        let dump = parse_and_dump("let a; let x = (a = 1, 2);").unwrap();
        assert_eq!(
            dump,
            "\
Program
  Let
    name: a
  Let
    name: x
    init:
      Paren
        Binary ,
          Assign =
            Ident a
            Number 1
          Number 2
"
        );
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
        let nested =
            parse_and_dump("try { throw [[1]]; } catch ([[a]]) { z = a; }").unwrap();
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

    #[test]
    fn parse_yield_assignment_expr_rhs() {
        // `yield` is AssignmentExpression-level: `yield 1 + 2` → yield (1 + 2), not (yield 1) + 2.
        let dump = parse_and_dump("function* g() { yield 1 + 2; }").unwrap();
        assert_eq!(
            dump,
            "\
Program
  FunctionDeclaration
    generator: true
    name: g
    body:
      Block
        ExpressionStatement
          Unary yield
            Binary +
              Number 1
              Number 2
"
        );
    }

    #[test]
    fn parse_yield_bare_and_conditional_rhs() {
        let bare = parse_and_dump("function* g() { yield; }").unwrap();
        assert!(bare.contains("Unary yield"), "got:\n{bare}");
        assert!(bare.contains("Unary void"), "bare yield → void 0, got:\n{bare}");

        let cond = parse_and_dump("function* g() { yield 1 ? 2 : 3; }").unwrap();
        assert!(
            cond.contains("Unary yield") && cond.contains("Conditional"),
            "yield RHS includes conditional, got:\n{cond}"
        );
        // Conditional must be under yield, not yield under binary/conditional left.
        let yield_idx = cond.find("Unary yield").expect("yield");
        let cond_idx = cond.find("Conditional").expect("conditional");
        assert!(
            yield_idx < cond_idx,
            "expected yield to wrap conditional, got:\n{cond}"
        );
    }

    #[test]
    fn parse_yield_star_delegate() {
        let dump = parse_and_dump("function* g() { yield* inner(); }").unwrap();
        assert!(
            dump.contains("Unary yield*"),
            "expected yield* unary, got:\n{dump}"
        );
        assert!(
            dump.contains("Call"),
            "yield* operand should parse as call, got:\n{dump}"
        );
        let star = parse_and_dump("function* g() { yield* [1, 2]; }").unwrap();
        assert!(
            star.contains("Unary yield*") && star.contains("Array"),
            "yield* array iterable, got:\n{star}"
        );
    }

    #[test]
    fn parse_as_type_assertion() {
        let dump = parse_and_dump("let x = n as i32;").unwrap();
        assert!(
            dump.contains("As\n")
                && dump.contains("Ident n")
                && dump.contains("NamedType i32"),
            "expected As node, got:\n{dump}"
        );
        let chain = parse_and_dump("let y = (n + 1) as number as i32;").unwrap();
        assert!(
            chain.matches("As\n").count() >= 2,
            "chained as, got:\n{chain}"
        );
    }

    #[test]
    fn parse_export_all_from() {
        let dump = parse_and_dump("export * from \"./lib.drac\";").unwrap();
        assert!(
            dump.contains("ExportAllDeclaration") && dump.contains("source: ./lib.drac"),
            "expected export * from, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_named_from() {
        let dump =
            parse_and_dump("export { value, inc as bump, default as d } from \"./lib.drac\";")
                .unwrap();
        assert!(
            dump.contains("ExportNamedDeclaration")
                && dump.contains("source: ./lib.drac")
                && dump.contains("local: value")
                && dump.contains("exported: bump")
                && dump.contains("local: default")
                && dump.contains("exported: d"),
            "expected export {{…}} from, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_star_as_ns_from() {
        let dump = parse_and_dump("export * as ns from \"./lib.drac\";").unwrap();
        assert!(
            dump.contains("ExportAllDeclaration")
                && dump.contains("exported: ns")
                && dump.contains("source: ./lib.drac"),
            "expected export * as ns from, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_class() {
        let dump = parse_and_dump("export class Point { constructor(x) { this.x = x; } }\n")
            .unwrap();
        assert!(
            dump.contains("ExportNamedDeclaration")
                && dump.contains("ClassDeclaration")
                && dump.contains("name: Point"),
            "expected export class, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_default_class() {
        let dump =
            parse_and_dump("export default class Counter { constructor() { this.n = 0; } }\n")
                .unwrap();
        assert!(
            dump.contains("ExportDefaultDeclaration")
                && dump.contains("ClassDeclaration")
                && dump.contains("name: Counter"),
            "expected export default class, got:\n{dump}"
        );
    }

    #[test]
    fn parse_class_static_block() {
        let dump = parse_and_dump("class C { static { this.x = 1; } static y = 2; }\n").unwrap();
        assert!(
            dump.contains("StaticBlock") && dump.contains("StaticField"),
            "expected static block + field, got:\n{dump}"
        );
        let multi = parse_and_dump("class C { static { let a = 1; } static { let b = 2; } }\n")
            .unwrap();
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
