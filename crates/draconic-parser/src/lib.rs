use draconic_ast::{
    dump_program, Arg, ArrayElement, ArrayPatternElement, ArrowBody, AssignOp, BinaryOp,
    BigIntLit, BindingKind, BindingPattern, ClassElement, ExportSpecifier, Expr, Ident,
    ImportSpecifier, NumberLit, ObjectKey, ObjectProp, Param, Program, Stmt, StringLit, SwitchCase,
    TemplateElement, UnaryOp, UpdateOp,
};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_lexer::{Lexer, Token, TokenKind};

pub use draconic_ast::dump_program as dump_ast;

pub fn parse(source: &str) -> Result<Program, Diagnostic> {
    let tokens = Lexer::new(source).tokenize()?;
    Parser::new(tokens).parse_program()
}

mod link;
pub use link::link_entry;

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
            body.push(self.parse_stmt()?);
        }
        let end = self.current_span().end.0;
        Ok(Program {
            body,
            span: Span::new(start, end),
        })
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
            return self.parse_import();
        }
        if self.check(&TokenKind::Export) {
            return self.parse_export();
        }
        if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
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
            body.push(self.parse_stmt()?);
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
        self.expect(&TokenKind::LParen)?;

        // `for (let/const name in/of right)` — binding without initializer.
        if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
            let kind = if self.check(&TokenKind::Const) {
                BindingKind::Const
            } else {
                BindingKind::Let
            };
            let let_start = self.bump().span.start.0;
            let name_tok = self.expect_ident()?;
            let name_end = name_tok.span.end.0;
            let name = Ident {
                name: name_tok.ident_name(),
                span: name_tok.span,
            };
            if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
                let is_in = self.check(&TokenKind::In);
                self.bump();
                let right = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                let end = stmt_span(&body).end.0;
                let left = Box::new(Stmt::Let {
                    kind,
                    binding: BindingPattern::Ident(name),
                    type_ann: None,
                    init: None,
                    span: Span::new(let_start, name_end),
                });
                return if is_in {
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
                        span: Span::new(start, end),
                    })
                };
            }
            // Classic `for (let/const name: T? = init; …)` / `for (let name; …)`.
            let type_ann = self.parse_optional_type_ann()?;
            let init_expr = if self.check(&TokenKind::Eq) {
                self.bump();
                Some(self.parse_assignment()?)
            } else if kind == BindingKind::Const {
                return Err(Diagnostic::new(
                    "const declaration requires an initializer".to_string(),
                    name.span,
                ));
            } else {
                None
            };
            let let_end = if let Some(ref e) = init_expr {
                expr_span(e).end.0
            } else if let Some(ref ann) = type_ann {
                ann.span().end.0
            } else {
                name.span.end.0
            };
            self.expect(&TokenKind::Semi)?;
            let left_init = Some(Box::new(Stmt::Let {
                kind,
                binding: BindingPattern::Ident(name),
                type_ann,
                init: init_expr,
                span: Span::new(let_start, let_end),
            }));
            return self.finish_classic_for(start, left_init);
        }

        if self.check(&TokenKind::Semi) {
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
        let expr_span = expr_span(&expr);
        if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
            let is_in = self.check(&TokenKind::In);
            self.bump();
            let right = self.parse_expr()?;
            self.expect(&TokenKind::RParen)?;
            let body = Box::new(self.parse_stmt()?);
            let end = stmt_span(&body).end.0;
            let left = Box::new(Stmt::Expression {
                expr,
                span: expr_span,
            });
            return if is_in {
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
                    span: Span::new(start, end),
                })
            };
        }

        self.expect(&TokenKind::Semi)?;
        let init = Some(Box::new(Stmt::Expression {
            expr,
            span: expr_span,
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
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        let return_type = self.parse_optional_type_ann()?;
        let body = Box::new(self.parse_block()?);
        let end = stmt_span(&body).end.0;
        Ok(Stmt::FunctionDeclaration {
            name,
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
        let super_class = if self.check(&TokenKind::Extends) {
            self.bump();
            Some(Box::new(self.parse_lhs()?))
        } else {
            None
        };
        self.expect(&TokenKind::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            body.push(self.parse_class_element()?);
            // Optional semicolon after method (ASI / explicit)
            if self.check(&TokenKind::Semi) {
                self.bump();
            }
        }
        let end = self.expect(&TokenKind::RBrace)?.span.end.0;
        Ok(Stmt::ClassDeclaration {
            name,
            super_class,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_class_element(&mut self) -> Result<ClassElement, Diagnostic> {
        let start = self.current_span().start.0;
        let is_static = if self.check(&TokenKind::Static) {
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
        let name_tok = self.expect_ident()?;
        let name = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_block()?);
        let end = stmt_span(&body).end.0;
        let span = Span::new(start, end);
        if name.name == "constructor" {
            if is_static {
                return Err(Diagnostic::new(
                    "class constructor cannot be static".to_string(),
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
                name,
                params,
                body,
                is_static,
                is_generator,
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

    /// `name`, `name: T`, `name = AssignmentExpression`, `name: T = …`, or `...name` / `...name: T`.
    fn parse_param(&mut self) -> Result<Param, Diagnostic> {
        if self.check(&TokenKind::DotDotDot) {
            let dots_start = self.current().span.start.0;
            self.bump();
            let p = self.expect_ident()?;
            let name = Ident {
                name: p.ident_name(),
                span: Span::new(dots_start, p.span.end.0),
            };
            let type_ann = self.parse_optional_type_ann()?;
            if self.check(&TokenKind::Eq) {
                return Err(Diagnostic::new(
                    "rest parameter cannot have a default",
                    self.current().span,
                ));
            }
            return Ok(Param {
                name,
                type_ann,
                default: None,
                rest: true,
            });
        }
        let p = self.expect_ident()?;
        let name = Ident {
            name: p.ident_name(),
            span: p.span,
        };
        let type_ann = self.parse_optional_type_ann()?;
        let default = if self.check(&TokenKind::Eq) {
            self.bump();
            Some(self.parse_assignment()?)
        } else {
            None
        };
        Ok(Param {
            name,
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

    /// `type Name = Type;`
    fn is_type_alias_start(&self) -> bool {
        matches!(self.current().kind, TokenKind::Ident(ref n) if n == "type")
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Ident(_))
            )
            && self
                .tokens
                .get(self.pos + 2)
                .map(|t| &t.kind == &TokenKind::Eq)
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
        self.expect(&TokenKind::Eq)?;
        let ty = self.parse_type()?;
        let mut end = ty.span().end.0;
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::TypeAlias {
            name,
            ty,
            span: Span::new(start, end),
        })
    }

    /// Type: named (`number`) or object (`{ a: T; b: U }`).
    fn parse_type(&mut self) -> Result<draconic_ast::TypeAnn, Diagnostic> {
        if self.check(&TokenKind::LBrace) {
            return self.parse_object_type();
        }
        let err_span = self.current().span;
        let name_tok = self.expect_ident().map_err(|_| {
            Diagnostic::new("expected type name after `:`".to_string(), err_span)
        })?;
        Ok(draconic_ast::TypeAnn::Named {
            name: name_tok.ident_name(),
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
            // Optional catch binding (ES2019): `catch { … }` or `catch (e) { … }`.
            if self.check(&TokenKind::LParen) {
                self.bump();
                let param_tok = self.expect_ident()?;
                handler_param = Some(Ident {
                    name: param_tok.ident_name(),
                    span: param_tok.span,
                });
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

    /// `export let/const/function …` or `export { a, b as c };` or `export default …`
    fn parse_export(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Export)?.span.start.0;
        if self.check(&TokenKind::Default) {
            return self.parse_export_default(start);
        }
        if self.check(&TokenKind::LBrace) {
            self.bump();
            let mut specifiers = Vec::new();
            if !self.check(&TokenKind::RBrace) {
                loop {
                    let local_tok = self.expect_ident()?;
                    let local = Ident {
                        name: local_tok.ident_name(),
                        span: local_tok.span,
                    };
                    let exported = if self.check(&TokenKind::As) {
                        self.bump();
                        // `default` is valid as ExportedBinding: `{ x as default }`.
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
            if self.check(&TokenKind::Semi) {
                end = self.bump().span.end.0;
            }
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: None,
                specifiers,
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
            let decl = self.parse_lexical_decl()?;
            let end = stmt_span(&decl).end.0;
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: Some(Box::new(decl)),
                specifiers: Vec::new(),
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
                span: Span::new(start, end),
            });
        }
        Err(Diagnostic::new(
            "expected `default`, `let`, `const`, `function`, or `{` after `export`".to_string(),
            self.current_span(),
        ))
    }

    /// `export default async? function name? (…) {…}` or `export default expr;`
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
                        span: Span::new(fn_start, end),
                    }),
                    span: Span::new(fn_start, end),
                }
            } else {
                Stmt::FunctionDeclaration {
                    name,
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

    fn parse_lexical_decl(&mut self) -> Result<Stmt, Diagnostic> {
        let kind_tok = self.bump();
        let kind = match kind_tok.kind {
            TokenKind::Const => BindingKind::Const,
            _ => BindingKind::Let,
        };
        let start = kind_tok.span.start.0;
        let binding = self.parse_binding_pattern()?;
        // Type annotations only on simple identifier bindings (`let x: T`).
        let type_ann = if matches!(binding, BindingPattern::Ident(_)) {
            self.parse_optional_type_ann()?
        } else {
            None
        };
        let init = if self.check(&TokenKind::Eq) {
            self.bump();
            // Initializer is AssignmentExpression (not Expression), so `,` is not
            // consumed here — multi-declarator lexical binding is a later feature.
            Some(self.parse_assignment()?)
        } else if matches!(binding, BindingPattern::Array { .. }) {
            return Err(Diagnostic::new(
                "array destructuring declaration requires an initializer".to_string(),
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
        let end = if self.check(&TokenKind::Semi) {
            self.bump().span.end.0
        } else {
            init.as_ref()
                .map(expr_span)
                .map(|s| s.end.0)
                .or_else(|| type_ann.as_ref().map(|a| a.span().end.0))
                .unwrap_or_else(|| binding.span().end.0)
        };
        Ok(Stmt::Let {
            kind,
            binding,
            type_ann,
            init,
            span: Span::new(start, end),
        })
    }

    /// Binding pattern: identifier or `[a, b, ...rest]` (nested arrays allowed).
    fn parse_binding_pattern(&mut self) -> Result<BindingPattern, Diagnostic> {
        if self.check(&TokenKind::LBracket) {
            self.parse_array_binding_pattern()
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
                if self.check(&TokenKind::DotDotDot) {
                    self.bump();
                    let name_tok = self.expect_ident()?;
                    elements.push(ArrayPatternElement::Rest(Ident {
                        name: name_tok.ident_name(),
                        span: name_tok.span,
                    }));
                    saw_rest = true;
                } else {
                    let inner = self.parse_binding_pattern()?;
                    elements.push(ArrayPatternElement::Pattern(inner));
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
                    name: Ident {
                        name: p.ident_name(),
                        span: p.span,
                    },
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
        let mut left = self.parse_shift()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::LtEq => BinaryOp::LtEq,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::GtEq => BinaryOp::GtEq,
                TokenKind::In if self.allow_in => BinaryOp::In,
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
            _ => None,
        };
        if let Some(op) = op {
            let start = self.bump().span.start.0;
            let arg = self.parse_unary()?;
            let end = expr_span(&arg).end.0;
            return Ok(Expr::Unary {
                op,
                arg: Box::new(arg),
                span: Span::new(start, end),
            });
        }
        self.parse_update()
    }

    /// Postfix update (`lhs++` / `lhs--`) and call.
    fn parse_update(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_lhs()?;
        loop {
            let op = match &self.current().kind {
                TokenKind::PlusPlus => UpdateOp::Inc,
                TokenKind::MinusMinus => UpdateOp::Dec,
                _ => break,
            };
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
                    span: Span::new(start, end),
                };
            } else if self.check(&TokenKind::Dot) {
                self.bump();
                let (name, prop_span) = self.expect_ident_name()?;
                let end = prop_span.end.0;
                let start = expr_span(&expr).start.0;
                let property = Expr::Ident(Ident {
                    name,
                    span: prop_span,
                });
                expr = Expr::MemberExpression {
                    object: Box::new(expr),
                    property: Box::new(property),
                    computed: false,
                    span: Span::new(start, end),
                };
            } else if self.check(&TokenKind::LBracket) {
                self.bump();
                let property = self.parse_expr()?;
                let end = self.expect(&TokenKind::RBracket)?.span.end.0;
                let start = expr_span(&expr).start.0;
                expr = Expr::MemberExpression {
                    object: Box::new(expr),
                    property: Box::new(property),
                    computed: true,
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

    /// `new callee` or `new callee(args)` — callee may include nested `new` and members.
    fn parse_new(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::New)?.span.start.0;
        let mut callee = if self.check(&TokenKind::New) {
            self.parse_new()?
        } else {
            self.parse_primary()?
        };
        // Member chain on the constructed callee (not calls — those bind to outer `new` args).
        loop {
            if self.check(&TokenKind::Dot) {
                self.bump();
                let (name, prop_span) = self.expect_ident_name()?;
                let end = prop_span.end.0;
                let cstart = expr_span(&callee).start.0;
                let property = Expr::Ident(Ident {
                    name,
                    span: prop_span,
                });
                callee = Expr::MemberExpression {
                    object: Box::new(callee),
                    property: Box::new(property),
                    computed: false,
                    span: Span::new(cstart, end),
                };
            } else if self.check(&TokenKind::LBracket) {
                self.bump();
                let property = self.parse_expr()?;
                let end = self.expect(&TokenKind::RBracket)?.span.end.0;
                let cstart = expr_span(&callee).start.0;
                callee = Expr::MemberExpression {
                    object: Box::new(callee),
                    property: Box::new(property),
                    computed: true,
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

    /// `key: value`, shorthand `{ a }`, method `{ m() {} }` / `{ *m() {} }`,
    /// or computed `{ [e]: v }` / `{ [e]() {} }` / `{ *[e]() {} }`.
    fn parse_object_prop(&mut self) -> Result<ObjectProp, Diagnostic> {
        let prop_start = self.current_span().start.0;
        let is_generator = if self.check(&TokenKind::Star) {
            self.bump();
            true
        } else {
            false
        };
        let key_tok = self.current().clone();
        match &key_tok.kind {
            TokenKind::LBracket => {
                let key_start = if is_generator {
                    prop_start
                } else {
                    key_tok.span.start.0
                };
                self.bump();
                let key_expr = self.parse_assignment()?;
                self.expect(&TokenKind::RBracket)?;
                let key = ObjectKey::Computed(Box::new(key_expr));
                if self.check(&TokenKind::LParen) {
                    let value = self.parse_method_function(key_start, is_generator)?;
                    let end = expr_span(&value).end.0;
                    return Ok(ObjectProp {
                        key,
                        value,
                        shorthand: false,
                        span: Span::new(key_start, end),
                    });
                }
                if is_generator {
                    return Err(Diagnostic::new(
                        "generator method requires `(params) { body }`".to_string(),
                        self.current_span(),
                    ));
                }
                self.expect(&TokenKind::Colon)?;
                let value = self.parse_assignment()?;
                let end = expr_span(&value).end.0;
                Ok(ObjectProp {
                    key,
                    value,
                    shorthand: false,
                    span: Span::new(key_start, end),
                })
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                let key_span = key_tok.span;
                let span_start = if is_generator {
                    prop_start
                } else {
                    key_span.start.0
                };
                self.bump();
                let key = ObjectKey::Ident(Ident {
                    name: name.clone(),
                    span: key_span,
                });
                // Method shorthand: `m(params) { body }` / `*m(params) { body }`
                if self.check(&TokenKind::LParen) {
                    let value = self.parse_method_function(span_start, is_generator)?;
                    let end = expr_span(&value).end.0;
                    return Ok(ObjectProp {
                        key,
                        value,
                        shorthand: false,
                        span: Span::new(span_start, end),
                    });
                }
                if is_generator {
                    return Err(Diagnostic::new(
                        "generator method requires `(params) { body }`".to_string(),
                        self.current_span(),
                    ));
                }
                // Property shorthand: `{ a }` or `{ a, … }`
                if self.check(&TokenKind::Comma) || self.check(&TokenKind::RBrace) {
                    let value = Expr::Ident(Ident {
                        name,
                        span: key_span,
                    });
                    return Ok(ObjectProp {
                        key,
                        value,
                        shorthand: true,
                        span: key_span,
                    });
                }
                self.expect(&TokenKind::Colon)?;
                let value = self.parse_assignment()?;
                let end = expr_span(&value).end.0;
                Ok(ObjectProp {
                    key,
                    value,
                    shorthand: false,
                    span: Span::new(key_span.start.0, end),
                })
            }
            TokenKind::String(value) => {
                let value_s = value.clone();
                let key_span = key_tok.span;
                let span_start = if is_generator {
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
                    let method = self.parse_method_function(span_start, is_generator)?;
                    let end = expr_span(&method).end.0;
                    return Ok(ObjectProp {
                        key,
                        value: method,
                        shorthand: false,
                        span: Span::new(span_start, end),
                    });
                }
                if is_generator {
                    return Err(Diagnostic::new(
                        "generator method requires `(params) { body }`".to_string(),
                        self.current_span(),
                    ));
                }
                self.expect(&TokenKind::Colon)?;
                let value = self.parse_assignment()?;
                let end = expr_span(&value).end.0;
                Ok(ObjectProp {
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
            is_async: false,
            is_generator,
            span: Span::new(start, end),
        })
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

    /// `[elem, …]` — trailing comma and `...spread` allowed; holes not in this surface.
    fn parse_array_expression(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(&TokenKind::LBracket)?.span.start.0;
        let mut elements = Vec::new();
        if !self.check(&TokenKind::RBracket) {
            loop {
                if self.check(&TokenKind::RBracket) {
                    break;
                }
                if self.check(&TokenKind::DotDotDot) {
                    self.bump();
                    elements.push(ArrayElement::Spread(self.parse_assignment()?));
                } else {
                    elements.push(ArrayElement::Expr(self.parse_assignment()?));
                }
                if self.check(&TokenKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let end = self.expect(&TokenKind::RBracket)?.span.end.0;
        Ok(Expr::ArrayExpression {
            elements,
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

fn expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Ident(i) => i.span,
        Expr::Number(n) => n.span,
        Expr::BigInt(n) => n.span,
        Expr::String(s) => s.span,
        Expr::Boolean { span, .. }
        | Expr::Null { span }
        | Expr::This { span }
        | Expr::Super { span }
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
        | Expr::ArrowFunction { span, .. }
        | Expr::ObjectExpression { span, .. }
        | Expr::ArrayExpression { span, .. }
        | Expr::ArrayPattern { span, .. }
        | Expr::MemberExpression { span, .. }
        | Expr::Paren { span, .. } => *span,
    }
}

/// Reinterpret an array literal as an assignment pattern when every element is
/// a binding target (`ident`, nested array pattern, or trailing `...ident`).
fn array_expr_to_pattern(expr: &Expr) -> Option<Expr> {
    let Expr::ArrayExpression { elements, span } = expr else {
        return None;
    };
    let mut pat_els = Vec::with_capacity(elements.len());
    let mut saw_rest = false;
    for el in elements {
        if saw_rest {
            return None;
        }
        match el {
            ArrayElement::Expr(inner) => {
                let binding = expr_to_binding_pattern(inner)?;
                pat_els.push(ArrayPatternElement::Pattern(binding));
            }
            ArrayElement::Spread(inner) => {
                let Expr::Ident(id) = inner else {
                    return None;
                };
                pat_els.push(ArrayPatternElement::Rest(id.clone()));
                saw_rest = true;
            }
        }
    }
    Some(Expr::ArrayPattern {
        elements: pat_els,
        span: *span,
    })
}

fn expr_to_binding_pattern(expr: &Expr) -> Option<BindingPattern> {
    match expr {
        Expr::Ident(id) => Some(BindingPattern::Ident(id.clone())),
        Expr::ArrayExpression { elements, span } => {
            let mut pat_els = Vec::with_capacity(elements.len());
            let mut saw_rest = false;
            for el in elements {
                if saw_rest {
                    return None;
                }
                match el {
                    ArrayElement::Expr(inner) => {
                        let binding = expr_to_binding_pattern(inner)?;
                        pat_els.push(ArrayPatternElement::Pattern(binding));
                    }
                    ArrayElement::Spread(inner) => {
                        let Expr::Ident(id) = inner else {
                            return None;
                        };
                        pat_els.push(ArrayPatternElement::Rest(id.clone()));
                        saw_rest = true;
                    }
                }
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
}
