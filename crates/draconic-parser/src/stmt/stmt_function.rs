use crate::*;

impl Parser {
    /// `extern "C" function …` — contextual `extern` + ABI string + `function`.
    pub(crate) fn is_extern_function_start(&self) -> bool {
        matches!(self.current().kind, TokenKind::Ident(ref n) if n == "extern")
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::String(_))
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|t| &t.kind),
                Some(TokenKind::Function)
            )
    }

    /// `extern "C" function name(params): ret?;` — FFI decl, no body (F06.01).
    pub(crate) fn parse_extern_function_decl(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current().span.start.0;
        // contextual `extern`
        self.bump();
        let abi = self.expect_string_lit()?;
        if abi.value.to_string_lossy() != "C" {
            return Err(Diagnostic::new(
                format!(
                    "unsupported extern ABI {:?}; only \"C\" is supported",
                    abi.value.to_string_lossy()
                ),
                abi.span,
            ));
        }
        self.expect(&TokenKind::Function)?;
        if self.check(&TokenKind::Star) {
            return Err(Diagnostic::new(
                "extern function declaration cannot be a generator".to_string(),
                self.current_span(),
            ));
        }
        let name_tok = self.expect_ident()?;
        let name = Ident {
            name: name_tok.ident_name(),
            span: name_tok.span,
        };
        if self.check(&TokenKind::Lt) {
            return Err(Diagnostic::new(
                "extern function declaration cannot have type parameters".to_string(),
                self.current_span(),
            ));
        }
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        let return_type = self.parse_optional_type_ann()?;
        if self.check(&TokenKind::LBrace) {
            return Err(Diagnostic::new(
                "extern function declaration cannot have a body".to_string(),
                self.current_span(),
            ));
        }
        let end = self.expect(&TokenKind::Semi)?.span.end.0;
        Ok(Stmt::ExternFunctionDeclaration {
            abi,
            name,
            params,
            return_type,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn parse_function_decl(&mut self) -> Result<Stmt, Diagnostic> {
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
        // Generator BindingIdentifier has [+Yield]: name cannot be `yield`.
        // FunctionDeclaration name inherits outer [Await]; params/body use function's [Await].
        self.with_ctx(
            |c| c.in_generator = is_generator,
            |p| {
                let name_tok = p.expect_ident()?;
                let name = Ident {
                    name: name_tok.ident_name(),
                    span: name_tok.span,
                };
                // E19.49: strict BindingIdentifier cannot be `eval`/`arguments`.
                if p.ctx.in_strict && is_strict_forbidden_binding_name(&name.name) {
                    return Err(Diagnostic::new(
                        format!("binding `{}` is invalid in strict mode", name.name),
                        name.span,
                    ));
                }
                let type_params = p.parse_optional_type_params()?;
                p.ctx.in_await_context = is_async;
                p.expect(&TokenKind::LParen)?;
                let params = p.parse_param_list()?;
                p.expect(&TokenKind::RParen)?;
                // E19.58: FormalParameters of a generator must not contain YieldExpression.
                if is_generator && params_contain_yield_expr(&params) {
                    return Err(Diagnostic::new(
                        "generator parameters cannot contain yield".to_string(),
                        Span::new(start, p.current_span().end.0),
                    ));
                }
                // E19.67: FormalParameters of an async function must not contain AwaitExpression.
                if is_async && params_contain_await_expr(&params) {
                    return Err(Diagnostic::new(
                        "async function parameters cannot contain await".to_string(),
                        Span::new(start, p.current_span().end.0),
                    ));
                }
                let return_type = p.parse_optional_type_ann()?;
                // E19.67: non-arrow functions introduce NewTarget.
                p.ctx.new_target_depth += 1;
                let body = Box::new(p.parse_function_body_block()?);
                // Name is also invalid when FunctionBody ContainsUseStrict (even if outer is sloppy).
                if is_strict_forbidden_binding_name(&name.name)
                    && block_has_use_strict_directive(&body)
                {
                    return Err(Diagnostic::new(
                        format!("binding `{}` is invalid in strict mode", name.name),
                        name.span,
                    ));
                }
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
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

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

    /// F06.01: `extern "C" function` declarations (no body).
    #[test]
    fn parse_extern_c_function_decl() {
        let dump = parse_and_dump(r#"extern "C" function add(a: i32, b: i32): i32;"#).unwrap();
        assert_eq!(
            dump,
            "\
Program
  ExternFunctionDeclaration
    abi: \"C\"
    name: add
    params:
      name: a
        type:
          NamedType i32
      name: b
        type:
          NamedType i32
    returnType:
      NamedType i32
"
        );
    }

    #[test]
    fn parse_extern_c_fnptr_type_param() {
        let dump =
            parse_and_dump(r#"extern "C" function draconic_rt_fnptr_nonnull(cb: function): i32;"#)
                .unwrap();
        assert_eq!(
            dump,
            "\
Program
  ExternFunctionDeclaration
    abi: \"C\"
    name: draconic_rt_fnptr_nonnull
    params:
      name: cb
        type:
          NamedType function
    returnType:
      NamedType i32
"
        );
    }

    #[test]
    fn parse_extern_c_function_pointer_param() {
        let dump = parse_and_dump(r#"extern "C" function puts(s: *u8): i32;"#).unwrap();
        assert_eq!(
            dump,
            "\
Program
  ExternFunctionDeclaration
    abi: \"C\"
    name: puts
    params:
      name: s
        type:
          PointerType
            NamedType u8
    returnType:
      NamedType i32
"
        );
    }

    #[test]
    fn parse_extern_c_function_no_params_no_return() {
        let dump = parse_and_dump(r#"extern "C" function quit();"#).unwrap();
        assert_eq!(
            dump,
            "\
Program
  ExternFunctionDeclaration
    abi: \"C\"
    name: quit
"
        );
    }

    #[test]
    fn parse_extern_c_function_void_return() {
        let dump = parse_and_dump(r#"extern "C" function free(p: *u8): void;"#).unwrap();
        assert!(dump.contains("ExternFunctionDeclaration"), "got:\n{dump}");
        assert!(dump.contains("returnType:"), "got:\n{dump}");
        assert!(dump.contains("NamedType void"), "got:\n{dump}");
    }

    #[test]
    fn parse_extern_c_function_rejects_body() {
        let err = parse(r#"extern "C" function f(): i32 {}"#).unwrap_err();
        assert!(
            err.message.contains("cannot have a body"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn parse_extern_c_function_rejects_generator() {
        let err = parse(r#"extern "C" function* g(): i32;"#).unwrap_err();
        assert!(err.message.contains("generator"), "got: {}", err.message);
    }

    #[test]
    fn parse_extern_c_function_rejects_non_c_abi() {
        let err = parse(r#"extern "stdcall" function f(): i32;"#).unwrap_err();
        assert!(
            err.message.contains("unsupported extern ABI"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn parse_extern_remains_identifier() {
        let dump = parse_and_dump("let extern = 1;").unwrap();
        assert!(dump.contains("name: extern"), "got:\n{dump}");
        assert!(!dump.contains("ExternFunctionDeclaration"), "got:\n{dump}");
    }

    /// E19.49: with body never allows function/generator/async/labelled-function decls.
    #[test]
    fn parse_e19_49_with_function_body_errors() {
        assert!(
            parse("with ({}) function f() {}").is_err(),
            "with + function decl must fail"
        );
        assert!(
            parse("with ({}) function* g() {}").is_err(),
            "with + generator decl must fail"
        );
        assert!(
            parse("with ({}) async function f() {}").is_err(),
            "with + async function decl must fail"
        );
        assert!(
            parse("with ({}) async function* g() {}").is_err(),
            "with + async generator decl must fail"
        );
        assert!(
            parse("with ({}) label1: label2: function f() {}").is_err(),
            "with + labelled function must fail"
        );
        // Non-strict Annex B still allows bare function in if / label only.
        assert!(
            parse("if (true) function f() {}").is_ok(),
            "non-strict if + function must remain valid (Annex B)"
        );
        assert!(
            parse("label: function f() {}").is_ok(),
            "non-strict labelled function must remain valid (Annex B)"
        );
        // while/do/for never allow bare function (even sloppy).
        assert!(
            parse("while (false) function g() {}").is_err(),
            "while + function must fail even non-strict"
        );
        assert!(
            parse("do function g() {} while (false);").is_err(),
            "do + function must fail even non-strict"
        );
        assert!(
            parse("for (;;) function g() {}").is_err(),
            "for + function must fail even non-strict"
        );
    }

    /// E19.49: strict mode rejects function declaration in statement position.
    #[test]
    fn parse_e19_49_strict_statement_position_function() {
        assert!(
            parse("\"use strict\"; if (true) function g() {}").is_err(),
            "strict if + function must fail"
        );
        assert!(
            parse("\"use strict\"; while (false) function g() {}").is_err(),
            "strict while + function must fail"
        );
        assert!(
            parse("\"use strict\"; for (;;) function g() {}").is_err(),
            "strict for + function must fail"
        );
        assert!(
            parse("\"use strict\"; do function g() {} while (false);").is_err(),
            "strict do + function must fail"
        );
        assert!(
            parse("\"use strict\"; label: function g() {}").is_err(),
            "strict labelled function must fail"
        );
        // StatementListItem still allows function decls in strict.
        assert!(
            parse("\"use strict\"; function f() {}\n").is_ok(),
            "strict top-level function declaration must remain valid"
        );
        assert!(
            parse("\"use strict\"; if (true) { function g() {} }\n").is_ok(),
            "strict block-level function declaration must remain valid"
        );
    }

    /// E19.49: strict BindingIdentifier cannot be eval/arguments (function/class names).
    #[test]
    fn parse_e19_49_strict_eval_arguments_names() {
        assert!(
            parse("\"use strict\"; function eval() {}").is_err(),
            "strict function eval name must fail"
        );
        assert!(
            parse("\"use strict\"; function arguments() {}").is_err(),
            "strict function arguments name must fail"
        );
        assert!(
            parse("function eval() { \"use strict\"; }").is_err(),
            "function eval with use-strict body must fail"
        );
        assert!(
            parse("\"use strict\"; let f = function eval() {};").is_err(),
            "strict FE eval name must fail"
        );
        assert!(parse("class eval {}").is_err(), "class eval name must fail");
        assert!(
            parse_module("import { eval } from \"./m.js\";").is_err(),
            "import eval binding must fail"
        );
        assert!(
            parse_module("import arguments from \"./m.js\";").is_err(),
            "default import arguments must fail"
        );
    }
}
