use super::*;

impl Parser {
    /// ExportDeclaration terminator: explicit `;` or ASI (LineTerminator / `}` / EOF).
    fn expect_export_semi(&mut self, end: u32) -> Result<u32, Diagnostic> {
        if self.check(&TokenKind::Semi) {
            return Ok(self.bump().span.end.0);
        }
        if self.can_asi_before_current() {
            return Ok(end);
        }
        Err(Diagnostic::new(
            "expected `;` after export declaration".to_string(),
            self.current_span(),
        ))
    }

    /// `import type { … }` / `import type foo from` / `import type * as ns from`.
    /// Not `import type from "mod"` (default binding named `type`).
    fn at_type_only_import_modifier(&self) -> bool {
        matches!(&self.current().kind, TokenKind::Ident(n) if n == "type")
            && (self.peek_is(&TokenKind::LBrace)
                || self.peek_is(&TokenKind::Star)
                || matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Ident(_))
                ))
    }

    /// `{ type foo }` / `{ type foo as bar }`, not `{ type }` / `{ type as bar }`.
    fn at_inline_type_import_specifier(&self) -> bool {
        matches!(&self.current().kind, TokenKind::Ident(n) if n == "type")
            && self
                .tokens
                .get(self.pos + 1)
                .and_then(|t| t.ident_name_opt())
                .is_some()
            && !self.peek_is(&TokenKind::As)
            && !self.peek_is(&TokenKind::Comma)
            && !self.peek_is(&TokenKind::RBrace)
    }

    /// `import { a, b as c } from "mod";`
    /// `import d from "mod";`
    /// `import d, { a } from "mod";`
    /// `import * as ns from "mod";`
    /// `import d, * as ns from "mod";`
    /// `import "mod";` / `import "mod" with {…};`
    pub(crate) fn parse_import(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect(&TokenKind::Import)?.span.start.0;
        let mut specifiers = Vec::new();
        let mut namespace = None;
        let mut phase = ImportPhase::Evaluation;
        let mut type_only = false;

        // Side-effect import: `import "mod"` (optional WithClause).
        let source = if matches!(self.current().kind, TokenKind::String(_)) {
            self.expect_string_lit()?
        } else if matches!(&self.current().kind, TokenKind::Ident(n) if n == "defer")
            && self.peek_is(&TokenKind::Star)
        {
            // E19.42: `import defer * as ns from "mod"` (only NameSpaceImport).
            self.bump();
            phase = ImportPhase::Defer;
            namespace = Some(self.parse_namespace_import()?);
            self.expect(&TokenKind::From)?;
            self.expect_string_lit()?
        } else {
            if self.at_type_only_import_modifier() {
                self.bump();
                type_only = true;
            }
            if self.check(&TokenKind::Star) {
                namespace = Some(self.parse_namespace_import()?);
            } else if matches!(self.current().kind, TokenKind::Ident(_)) {
                let local_tok = self.expect_ident()?;
                let local = Ident {
                    name: local_tok.ident_name(),
                    span: local_tok.span,
                };
                // E19.49: ImportedBinding is BindingIdentifier (modules always strict).
                if is_strict_forbidden_binding_name(&local.name) {
                    return Err(Diagnostic::new(
                        format!("binding `{}` is invalid in strict mode", local.name),
                        local.span,
                    ));
                }
                let def_span = local.span;
                specifiers.push(ImportSpecifier {
                    imported: Ident {
                        name: "default".into(),
                        span: def_span,
                    },
                    local,
                    is_type: false,
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
            self.expect_string_lit()?
        };

        let (attributes, clause_end) = self.parse_with_clause_opt()?;
        let mut end = clause_end.unwrap_or(source.span.end.0);
        if self.check(&TokenKind::Semi) {
            end = self.bump().span.end.0;
        }
        Ok(Stmt::ImportDeclaration {
            specifiers,
            namespace,
            source,
            attributes,
            phase,
            type_only,
            span: Span::new(start, end),
        })
    }

    /// Optional `with {…}` or legacy `assert {…}` (no LineTerminator before `assert`).
    /// Returns attributes and the end offset of the clause when present.
    fn parse_with_clause_opt(&mut self) -> Result<(Vec<ImportAttribute>, Option<u32>), Diagnostic> {
        let keyword_start = self.current_span().start.0;
        if self.check(&TokenKind::With) {
            self.bump();
        } else if matches!(&self.current().kind, TokenKind::Ident(n) if n == "assert") {
            if self.current().preceded_by_line_terminator {
                return Ok((Vec::new(), None));
            }
            self.bump();
        } else {
            return Ok((Vec::new(), None));
        }

        self.expect(&TokenKind::LBrace)?;
        let mut attributes = Vec::new();
        let mut seen_keys: Vec<JsString> = Vec::new();
        if !self.check(&TokenKind::RBrace) {
            loop {
                let key_start = self.current_span().start.0;
                let (key, key_units, key_span) =
                    if matches!(self.current().kind, TokenKind::String(_)) {
                        let s = self.expect_string_lit()?;
                        let units = s.value.clone();
                        let span = s.span;
                        (ImportAttributeKey::String(s), units, span)
                    } else {
                        let (name, span) = self.expect_ident_name()?;
                        let units = JsString::from(name.as_str());
                        (ImportAttributeKey::Ident(Ident { name, span }), units, span)
                    };
                if seen_keys.iter().any(|k| k == &key_units) {
                    return Err(Diagnostic::new(
                        "duplicate import attribute key".to_string(),
                        key_span,
                    ));
                }
                seen_keys.push(key_units);
                self.expect(&TokenKind::Colon)?;
                let value = self.expect_string_lit()?;
                let end = value.span.end.0;
                attributes.push(ImportAttribute {
                    key,
                    value,
                    span: Span::new(key_start, end),
                });
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
        let rbrace = self.expect(&TokenKind::RBrace)?;
        let end = rbrace.span.end.0;
        let _ = keyword_start;
        Ok((attributes, Some(end)))
    }

    /// `* as ImportedBinding`
    fn parse_namespace_import(&mut self) -> Result<Ident, Diagnostic> {
        self.expect(&TokenKind::Star)?;
        self.expect(&TokenKind::As)?;
        let local_tok = self.expect_ident()?;
        let local = Ident {
            name: local_tok.ident_name(),
            span: local_tok.span,
        };
        // E19.49: ImportedBinding is BindingIdentifier (modules always strict).
        if is_strict_forbidden_binding_name(&local.name) {
            return Err(Diagnostic::new(
                format!("binding `{}` is invalid in strict mode", local.name),
                local.span,
            ));
        }
        Ok(local)
    }

    fn parse_named_import_specifiers(
        &mut self,
        specifiers: &mut Vec<ImportSpecifier>,
    ) -> Result<(), Diagnostic> {
        if self.check(&TokenKind::RBrace) {
            return Ok(());
        }
        loop {
            let is_type = if self.at_inline_type_import_specifier() {
                self.bump();
                true
            } else {
                false
            };
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
            // E19.49: ImportedBinding is BindingIdentifier (modules always strict).
            if is_strict_forbidden_binding_name(&local.name) {
                return Err(Diagnostic::new(
                    format!("binding `{}` is invalid in strict mode", local.name),
                    local.span,
                ));
            }
            specifiers.push(ImportSpecifier {
                imported,
                local,
                is_type,
            });
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
    pub(crate) fn parse_export(&mut self) -> Result<Stmt, Diagnostic> {
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
            let (attributes, clause_end) = self.parse_with_clause_opt()?;
            let mut end = clause_end.unwrap_or(source.span.end.0);
            // E19.69: ExportDeclaration requires `;` or LineTerminator (ASI) after FromClause.
            end = self.expect_export_semi(end)?;
            return Ok(Stmt::ExportAllDeclaration {
                exported,
                source,
                attributes,
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
            let (source, attributes) = if self.check(&TokenKind::From) {
                self.bump();
                let src = self.expect_string_lit()?;
                end = src.span.end.0;
                let (attributes, clause_end) = self.parse_with_clause_opt()?;
                if let Some(e) = clause_end {
                    end = e;
                }
                (Some(src), attributes)
            } else {
                (None, Vec::new())
            };
            // E19.69: trailing `;` or ASI after named export / export-from.
            end = self.expect_export_semi(end)?;
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: None,
                specifiers,
                source,
                attributes,
                span: Span::new(start, end),
            });
        }
        if self.check(&TokenKind::Let)
            || self.check(&TokenKind::Const)
            || self.check(&TokenKind::Var)
        {
            // `export VariableStatement` / `export LexicalDeclaration` (E19.54: `export var`).
            let decl = self.parse_lexical_decl()?;
            let end = stmt_span(&decl).end.0;
            return Ok(Stmt::ExportNamedDeclaration {
                declaration: Some(Box::new(decl)),
                specifiers: Vec::new(),
                source: None,
                attributes: Vec::new(),
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
                attributes: Vec::new(),
                span: Span::new(start, end),
            });
        }
        // E19.78: `export` DecoratorList_opt `class` …
        if self.check(&TokenKind::At) || self.check(&TokenKind::Class) {
            if self.check(&TokenKind::At) {
                self.parse_decorator_list()?;
            }
            if self.check(&TokenKind::Class) {
                let decl = self.parse_class_decl()?;
                let end = stmt_span(&decl).end.0;
                return Ok(Stmt::ExportNamedDeclaration {
                    declaration: Some(Box::new(decl)),
                    specifiers: Vec::new(),
                    source: None,
                    attributes: Vec::new(),
                    span: Span::new(start, end),
                });
            }
            return Err(Diagnostic::new(
                "decorators must precede a class declaration".to_string(),
                self.current_span(),
            ));
        }
        Err(Diagnostic::new(
            "expected `default`, `*`, `let`, `const`, `var`, `function`, `class`, or `{` after `export`"
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
            return self.with_ctx(
                |c| c.in_generator = is_generator,
                |p| {
                    // Default export function declaration name inherits outer [Await]; body uses is_async.
                    let (name, is_synthetic) = if p.at_binding_ident() {
                        let name_tok = p.expect_ident()?;
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
                    p.ctx.in_await_context = is_async;
                    p.expect(&TokenKind::LParen)?;
                    let params = p.parse_param_list()?;
                    p.expect(&TokenKind::RParen)?;
                    // E19.58: FormalParameters of a generator must not contain YieldExpression.
                    if is_generator && params_contain_yield_expr(&params) {
                        return Err(Diagnostic::new(
                            "generator parameters cannot contain yield".to_string(),
                            Span::new(fn_start, p.current_span().end.0),
                        ));
                    }
                    let return_type = p.parse_optional_type_ann()?;
                    let body = Box::new(p.parse_function_body_block()?);
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
                    Ok(Stmt::ExportDefaultDeclaration {
                        declaration: Box::new(declaration),
                        local,
                        span: Span::new(start, end),
                    })
                },
            );
        }
        // E19.78: `export default` DecoratorList_opt `class` …
        if self.check(&TokenKind::At) || self.check(&TokenKind::Class) {
            if self.check(&TokenKind::At) {
                self.parse_decorator_list()?;
            }
            if self.check(&TokenKind::Class) {
                // E19.54: `export default class extends …` / anonymous `export default class {…}`.
                let decl = self.parse_class_decl_inner(true)?;
                let end = stmt_span(&decl).end.0;
                let local = match &decl {
                    Stmt::ClassDeclaration { name, .. } => name.clone(),
                    _ => unreachable!("parse_class_decl_inner returns ClassDeclaration"),
                };
                return Ok(Stmt::ExportDefaultDeclaration {
                    declaration: Box::new(decl),
                    local,
                    span: Span::new(start, end),
                });
            }
            // Fall through: `@dec` alone is not export default (decorator applies to following).
        }
        // LexicalDeclaration is not a valid export default (E19.41 / Test262 parse-err-export-dflt-let).
        if self.check(&TokenKind::Const)
            || (self.check(&TokenKind::Let) && self.let_starts_lexical_declaration())
            || self.await_using_starts_declaration()
            || self.using_starts_declaration()
        {
            return Err(Diagnostic::new(
                "lexical declaration not allowed in export default".to_string(),
                self.current().span,
            ));
        }

        // E19.69: `export default` AssignmentExpression (not Expression/comma).
        let expr = self.parse_assignment()?;
        let mut end = expr_span(&expr).end.0;
        end = self.expect_export_semi(end)?;
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
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::super::dump_program;

    #[test]
    fn parse_import_defer_namespace() {
        // E19.42: static deferred namespace import.
        let dump = parse_and_dump("import defer * as ns from './m.js';").unwrap();
        assert!(
            dump.contains("ImportDeclaration")
                && dump.contains("phase: defer")
                && dump.contains("namespace: ns")
                && dump.contains("source: ./m.js"),
            "expected import defer namespace, got:\n{dump}"
        );
        assert!(!dump.contains("ImportSpecifier"), "{dump}");
    }

    #[test]
    fn parse_import_defer_namespace_with_attributes() {
        let dump =
            parse_and_dump("import defer * as ns from './m.js' with { type: \"json\" };").unwrap();
        assert!(
            dump.contains("phase: defer")
                && dump.contains("namespace: ns")
                && dump.contains("ImportAttribute"),
            "expected import defer + attributes, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_default_binding_named_defer() {
        // `defer` remains a valid default import binding name.
        let dump = parse_and_dump("import defer from './m.js';").unwrap();
        assert!(
            dump.contains("ImportDeclaration")
                && dump.contains("local: defer")
                && !dump.contains("phase: defer"),
            "expected default import named defer, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_defer_named_fails() {
        assert!(parse("import defer { x } from './m.js';").is_err());
    }

    #[test]
    fn parse_import_defer_default_fails() {
        assert!(parse("import defer x from './m.js';").is_err());
    }

    #[test]
    fn parse_import_with_attributes() {
        let dump = parse_and_dump("import x from \"./m.js\" with { type: \"json\" };").unwrap();
        assert!(
            dump.contains("ImportDeclaration")
                && dump.contains("source: ./m.js")
                && dump.contains("ImportAttribute")
                && dump.contains("key: type")
                && dump.contains("value: \"json\""),
            "expected import with attributes, got:\n{dump}"
        );
    }

    #[test]
    fn parse_side_effect_import_with_empty_attributes() {
        let dump = parse_and_dump("import \"./m.js\" with {};").unwrap();
        assert!(
            dump.contains("ImportDeclaration") && dump.contains("source: ./m.js"),
            "expected side-effect import, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_assert_attributes() {
        let dump = parse_and_dump("import x from \"./m.js\" assert { type: \"json\" };").unwrap();
        assert!(
            dump.contains("ImportAttribute") && dump.contains("key: type"),
            "expected assert attributes, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_with_attributes_allows_nlt_before_with() {
        let dump = parse_and_dump("import x from \"./m.js\"\nwith { type: \"json\" };").unwrap();
        assert!(
            dump.contains("ImportAttribute") && dump.contains("key: type"),
            "expected with after newline, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_assert_rejects_nlt_before_assert() {
        // LineTerminator before `assert` ends the import; `assert` is not a WithClause.
        let dump = parse_and_dump("import x from \"./m.js\"\nassert { type: \"json\" };");
        match dump {
            Ok(d) => assert!(
                !d.contains("ImportAttribute"),
                "assert after newline must not be WithClause, got:\n{d}"
            ),
            Err(_) => {}
        }
    }

    #[test]
    fn parse_export_all_with_attributes() {
        let dump = parse_and_dump("export * from \"./m.js\" with { type: \"json\" };").unwrap();
        assert!(
            dump.contains("ExportAllDeclaration")
                && dump.contains("ImportAttribute")
                && dump.contains("key: type"),
            "expected export * with attributes, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_attribute_keyword_key() {
        let dump = parse_and_dump("import \"./m.js\" with { if: \"\" };").unwrap();
        assert!(
            dump.contains("key: if"),
            "expected IdentifierName key, got:\n{dump}"
        );
    }

    #[test]
    fn parse_import_attribute_duplicate_key_fails() {
        assert!(
            parse("import x from \"./m.js\" with { type: \"json\", \"typ\\u0065\": \"\" };")
                .is_err()
        );
    }

    #[test]
    fn parse_import_attribute_trailing_comma() {
        let dump = parse_and_dump("import \"./m.js\" with { type: \"json\", };").unwrap();
        assert!(
            dump.contains("ImportAttribute") && dump.contains("key: type"),
            "expected trailing comma with clause, got:\n{dump}"
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
        let dump =
            parse_and_dump("export class Point { constructor(x) { this.x = x; } }\n").unwrap();
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
    fn parse_export_var() {
        let dump = parse_and_dump("export var name1 = 1;\n").unwrap();
        assert!(
            dump.contains("ExportNamedDeclaration")
                && dump.contains("Var")
                && dump.contains("name: name1"),
            "expected export var, got:\n{dump}"
        );
        let dstr = parse_and_dump("export var { x = 1 } = {};\n").unwrap();
        assert!(
            dstr.contains("ExportNamedDeclaration") && dstr.contains("ObjectPattern"),
            "expected export var destructuring, got:\n{dstr}"
        );
    }

    #[test]
    fn parse_export_var_await_module() {
        // E19.54: `export var x = await expr` under Module [+Await].
        let prog =
            parse_module("export var name1 = await foo;\nexport var { x = await foo } = {};\n")
                .expect("export var await");
        let dump = dump_program(&prog);
        assert!(
            dump.contains("ExportNamedDeclaration")
                && dump.contains("Unary await")
                && dump.matches("ExportNamedDeclaration").count() >= 2,
            "expected export var await, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_default_class_anonymous_await_extends() {
        // E19.54: `export default class extends fn(await foo) {}`
        let prog = parse_module(
            "function fn() { return function() {}; }\n\
             export default class extends fn(await foo) {}\n",
        )
        .expect("export default class anonymous + await extends");
        let dump = dump_program(&prog);
        assert!(
            dump.contains("ExportDefaultDeclaration")
                && dump.contains("ClassDeclaration")
                && dump.contains("name: __class")
                && dump.contains("Unary await"),
            "expected anonymous default class with await extends, got:\n{dump}"
        );
    }

    #[test]
    fn parse_export_class_await_extends() {
        let prog = parse_module(
            "function fn() { return function() {}; }\n\
             export class C extends fn(await foo) {}\n",
        )
        .expect("export class await extends");
        let dump = dump_program(&prog);
        assert!(
            dump.contains("ExportNamedDeclaration")
                && dump.contains("name: C")
                && dump.contains("Unary await"),
            "expected export class await extends, got:\n{dump}"
        );
    }
}
