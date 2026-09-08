use draconic_ast::{
    Arg, ArrayElement, ArrowBody, AssignOp, BindingKind, BindingPattern, Expr, Ident, NumberLit,
    ObjectKey, ObjectProp, Stmt,
};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_parser::{parse, parse_module};

use crate::load::top_level_names;
use crate::spans::{uniqueify_stmt_spans, SyntheticSpans};

/// E19.84.09: hoist bindings and run module body as an async IIFE so TLA modules
/// can interleave (fire-and-forget kick; entry awaits its own promise).
pub(crate) fn wrap_async_eager_module_body(
    mod_id: usize,
    body: Vec<Stmt>,
    async_dep_ids: &[usize],
    track_status: bool,
    is_entry: bool,
    spans: &mut SyntheticSpans,
) -> Result<Vec<Stmt>, Diagnostic> {
    let names = top_level_names(&body);
    let mut out = Vec::new();
    let mut sorted: Vec<_> = names.into_iter().collect();
    sorted.sort();
    for name in &sorted {
        let sp = spans.next();
        out.push(Stmt::Let {
            kind: BindingKind::Let,
            binding: BindingPattern::Ident(Ident {
                name: name.clone(),
                span: sp,
            }),
            type_ann: None,
            init: None,
            span: sp,
        });
    }

    let mut await_deps = String::new();
    for &dep in async_dep_ids {
        await_deps.push_str(&format!("await __draconic_mp[{dep}];\n"));
    }
    let status_start = if track_status {
        format!("__draconic_mstatus[{mod_id}] = 1;\n")
    } else {
        String::new()
    };
    let status_ok = if track_status {
        format!("__draconic_mstatus[{mod_id}] = 3;\n")
    } else {
        String::new()
    };
    let status_err = if track_status {
        format!("__draconic_merror[{mod_id}] = e;\n__draconic_mstatus[{mod_id}] = 3;\n")
    } else {
        String::new()
    };

    // Placeholder `;` is replaced with hoisted body statements.
    let src = format!(
        r#"
__draconic_mp[{mod_id}] = (async () => {{
  {status_start}try {{
    {await_deps};
    {status_ok}}} catch (e) {{
    {status_err}throw e;
  }}
}})();
"#
    );
    let mut parsed = parse(&src)?.body;
    let mut try_body: Vec<Stmt> = Vec::new();
    // Re-parse await deps as real statements by extracting from a tiny module.
    if !async_dep_ids.is_empty() {
        let mut adeps = String::new();
        for &dep in async_dep_ids {
            adeps.push_str(&format!("await __draconic_mp[{dep}];\n"));
        }
        // parse_module allows top-level await.
        let dep_prog = parse_module(&adeps)
            .map_err(|e| Diagnostic::new(format!("async dep await parse: {e}"), Span::dummy()))?;
        try_body.extend(dep_prog.body);
    }
    for stmt in body {
        try_body.push(hoist_decl_to_assign(stmt));
    }
    if track_status {
        try_body.push(make_module_status_assign(mod_id, 3, spans.next()));
    }

    // Inject try_body into the try block of the async IIFE assignment.
    inject_try_body_into_async_mp(&mut parsed, &mut try_body);

    for mut stmt in parsed {
        uniqueify_stmt_spans(&mut stmt, spans);
        out.push(stmt);
    }

    // Entry module evaluation must complete before subsequent host code; await it.
    if is_entry {
        let await_src = format!("await __draconic_mp[{mod_id}];\n");
        let await_prog = parse_module(&await_src)
            .map_err(|e| Diagnostic::new(format!("entry await parse: {e}"), Span::dummy()))?;
        for mut stmt in await_prog.body {
            uniqueify_stmt_spans(&mut stmt, spans);
            out.push(stmt);
        }
    }

    Ok(out)
}

/// Replace the try-block body of `__draconic_mp[id] = (async () => { try {…} })()`
/// with `try_body` (leaves catch intact).
pub(crate) fn inject_try_body_into_async_mp(parsed: &mut [Stmt], try_body: &mut Vec<Stmt>) {
    for stmt in parsed.iter_mut() {
        let Stmt::Expression { expr, .. } = stmt else {
            continue;
        };
        let Expr::Assign { value, .. } = expr else {
            continue;
        };
        let arrow_body = match value.as_mut() {
            Expr::Call { callee, .. } => match callee.as_mut() {
                Expr::Paren { expr: inner, .. } => match inner.as_mut() {
                    Expr::ArrowFunction {
                        body: ArrowBody::Block(block),
                        ..
                    } => Some(block.as_mut()),
                    _ => None,
                },
                Expr::ArrowFunction {
                    body: ArrowBody::Block(block),
                    ..
                } => Some(block.as_mut()),
                _ => None,
            },
            _ => None,
        };
        let Some(Stmt::Block { body: stmts, .. }) = arrow_body else {
            continue;
        };
        for s in stmts.iter_mut() {
            // status assign may precede try
            if let Stmt::Try { block: tb, .. } = s {
                if let Stmt::Block {
                    body: try_stmts, ..
                } = tb.as_mut()
                {
                    *try_stmts = std::mem::take(try_body);
                    return;
                }
            }
        }
    }
}

/// True when module body has top-level `await` / `await using` / `for await` (HasTLA).
pub(crate) fn module_body_has_tla(body: &[Stmt]) -> bool {
    body.iter().any(stmt_has_top_level_await)
}

pub(crate) fn stmt_has_top_level_await(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expression { expr, .. } | Stmt::Throw { argument: expr, .. } => {
            expr_has_top_level_await(expr)
        }
        Stmt::Let { kind, init, .. } => {
            *kind == BindingKind::AwaitUsing || init.as_ref().is_some_and(expr_has_top_level_await)
        }
        Stmt::Return {
            argument: Some(expr),
            ..
        } => expr_has_top_level_await(expr),
        Stmt::Block { body, .. } => body.iter().any(stmt_has_top_level_await),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_top_level_await(test)
                || stmt_has_top_level_await(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_has_top_level_await(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_has_top_level_await(test) || stmt_has_top_level_await(body)
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            init.as_ref().is_some_and(|s| stmt_has_top_level_await(s))
                || test.as_ref().is_some_and(expr_has_top_level_await)
                || update.as_ref().is_some_and(expr_has_top_level_await)
                || stmt_has_top_level_await(body)
        }
        Stmt::ForIn {
            left, right, body, ..
        } => {
            stmt_has_top_level_await(left)
                || expr_has_top_level_await(right)
                || stmt_has_top_level_await(body)
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
            ..
        } => {
            *is_await
                || stmt_has_top_level_await(left)
                || expr_has_top_level_await(right)
                || stmt_has_top_level_await(body)
        }
        Stmt::Labeled { body, .. } => stmt_has_top_level_await(body),
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            expr_has_top_level_await(discriminant)
                || cases.iter().any(|c| {
                    c.test.as_ref().is_some_and(expr_has_top_level_await)
                        || c.body.iter().any(stmt_has_top_level_await)
                })
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            stmt_has_top_level_await(block)
                || handler
                    .as_ref()
                    .is_some_and(|h| stmt_has_top_level_await(h))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| stmt_has_top_level_await(f))
        }
        Stmt::With { object, body, .. } => {
            expr_has_top_level_await(object) || stmt_has_top_level_await(body)
        }
        // Nested functions/classes have their own async context — not module TLA.
        Stmt::FunctionDeclaration { .. }
        | Stmt::ClassDeclaration { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Empty { .. }
        | Stmt::ImportDeclaration { .. }
        | Stmt::ExportNamedDeclaration { .. }
        | Stmt::ExportDefaultDeclaration { .. }
        | Stmt::ExportAllDeclaration { .. }
        | Stmt::TypeAlias { .. }
        | Stmt::ExternFunctionDeclaration { .. }
        | Stmt::Return { argument: None, .. } => false,
    }
}

pub(crate) fn expr_has_top_level_await(expr: &Expr) -> bool {
    match expr {
        Expr::Unary {
            op: draconic_ast::UnaryOp::Await,
            ..
        } => true,
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } => expr_has_top_level_await(arg),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_has_top_level_await(left) || expr_has_top_level_await(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_top_level_await(test)
                || expr_has_top_level_await(consequent)
                || expr_has_top_level_await(alternate)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_has_top_level_await(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_has_top_level_await(e),
                })
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_has_top_level_await(object) || expr_has_top_level_await(property),
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_has_top_level_await(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, key, .. } => {
                expr_has_top_level_await(value)
                    || matches!(key, ObjectKey::Computed(e) if expr_has_top_level_await(e))
            }
            ObjectProp::Spread { expr, .. } => expr_has_top_level_await(expr),
            ObjectProp::Accessor { .. } => false,
        }),
        Expr::ImportCall {
            source, options, ..
        } => {
            expr_has_top_level_await(source)
                || options
                    .as_ref()
                    .is_some_and(|o| expr_has_top_level_await(o))
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => expr_has_top_level_await(tag) || expressions.iter().any(expr_has_top_level_await),
        Expr::TemplateLiteral { expressions, .. } => {
            expressions.iter().any(expr_has_top_level_await)
        }
        Expr::Paren { expr, .. } | Expr::As { expr, .. } => expr_has_top_level_await(expr),
        Expr::PrivateIn { object, .. } => expr_has_top_level_await(object),
        // Nested functions — not module TLA.
        Expr::FunctionExpression { .. }
        | Expr::ArrowFunction { .. }
        | Expr::ClassExpression { .. }
        | Expr::Ident(_)
        | Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::String(_)
        | Expr::RegExp { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::Super { .. }
        | Expr::NewTarget { .. }
        | Expr::ImportMeta { .. }
        | Expr::ArrayPattern { .. }
        | Expr::ObjectPattern { .. } => false,
    }
}

/// E19.84.05: per-module [[Status]] + ReadyForSyncExecution for deferred ns.

pub(crate) fn make_module_status_assign(mod_id: usize, status: i32, span: Span) -> Stmt {
    // __draconic_mstatus[mod_id] = status;
    Stmt::Expression {
        expr: Expr::Assign {
            target: Box::new(Expr::MemberExpression {
                object: Box::new(Expr::Ident(Ident {
                    name: "__draconic_mstatus".into(),
                    span,
                })),
                property: Box::new(Expr::Number(NumberLit {
                    raw: mod_id.to_string(),
                    span,
                })),
                computed: true,
                optional: false,
                private: false,
                span,
            }),
            op: AssignOp::Eq,
            value: Box::new(Expr::Number(NumberLit {
                raw: status.to_string(),
                span,
            })),
            span,
        },
        span,
    }
}

/// Runtime helper implementing deferred module namespace exotic object triggers
/// (E19.55) and the deferred namespace object MOP (E19.84.01).

/// Hoist top-level bindings and wrap module body in a once-eval function.
pub(crate) fn wrap_deferred_module_body(
    mod_id: usize,
    eval_name: &str,
    prelude_calls: Vec<Stmt>,
    body: Vec<Stmt>,
    spans: &mut SyntheticSpans,
) -> Vec<Stmt> {
    let names = top_level_names(&body);
    let mut out = Vec::new();
    let mut sorted: Vec<_> = names.into_iter().collect();
    sorted.sort();
    for name in &sorted {
        let sp = spans.next();
        out.push(Stmt::Let {
            kind: BindingKind::Let,
            binding: BindingPattern::Ident(Ident {
                name: name.clone(),
                span: sp,
            }),
            type_ann: None,
            init: None,
            span: sp,
        });
    }

    // E19.84.05 / E19.84.08: once-eval with [[EvaluationError]] cache.
    // ReadyForSyncExecution / TypeError is enforced by __draconic_deferred_ns.ensure before call.
    let mut try_body: Vec<Stmt> = prelude_calls;
    for stmt in body {
        try_body.push(hoist_decl_to_assign(stmt));
    }
    try_body.push(make_module_status_assign(mod_id, 3, spans.next()));
    let guard_src = format!(
        r#"
function {eval_name}() {{
  if (__draconic_mstatus[{mod_id}] === 3) {{
    if (__draconic_merror[{mod_id}] !== undefined) throw __draconic_merror[{mod_id}];
    return;
  }}
  __draconic_mstatus[{mod_id}] = 1;
  try {{
    ;
  }} catch (e) {{
    __draconic_merror[{mod_id}] = e;
    __draconic_mstatus[{mod_id}] = 3;
    throw e;
  }}
}}
"#
    );
    let mut parsed = match parse(&guard_src) {
        Ok(p) => p.body,
        Err(_) => {
            // Fallback without try (should not happen): previous shape.
            let mut eval_body = Vec::new();
            let guard_span = spans.next();
            eval_body.push(make_module_status_assign(mod_id, 1, guard_span));
            eval_body.extend(try_body);
            let fn_span = spans.next();
            out.push(Stmt::FunctionDeclaration {
                name: Ident {
                    name: eval_name.to_string(),
                    span: fn_span,
                },
                type_params: vec![],
                params: vec![],
                return_type: None,
                body: Box::new(Stmt::Block {
                    body: eval_body,
                    span: fn_span,
                }),
                is_async: false,
                is_generator: false,
                span: fn_span,
            });
            return out;
        }
    };
    // Inject real module body into the try block of the parsed skeleton.
    if let Some(Stmt::FunctionDeclaration { body: fn_body, .. }) = parsed.last_mut() {
        if let Stmt::Block { body: stmts, .. } = fn_body.as_mut() {
            for stmt in stmts.iter_mut() {
                if let Stmt::Try { block, .. } = stmt {
                    if let Stmt::Block {
                        body: try_stmts, ..
                    } = block.as_mut()
                    {
                        *try_stmts = try_body;
                        break;
                    }
                }
            }
        }
    }
    for mut stmt in parsed {
        uniqueify_stmt_spans(&mut stmt, spans);
        out.push(stmt);
    }
    out
}

/// Turn top-level `let/const x = init` / `function f` into assignments to hoisted bindings.
pub(crate) fn hoist_decl_to_assign(stmt: Stmt) -> Stmt {
    match stmt {
        Stmt::Let {
            binding: BindingPattern::Ident(id),
            init: Some(init),
            span,
            ..
        } => Stmt::Expression {
            expr: Expr::Assign {
                target: Box::new(Expr::Ident(id)),
                op: AssignOp::Eq,
                value: Box::new(init),
                span,
            },
            span,
        },
        Stmt::Let {
            binding: BindingPattern::Ident(_),
            init: None,
            span,
            ..
        } => Stmt::Empty { span },
        Stmt::FunctionDeclaration {
            name,
            params,
            return_type,
            body,
            is_async,
            is_generator,
            span,
            ..
        } => {
            let fn_expr = Expr::FunctionExpression {
                name: Some(name.clone()),
                params,
                return_type,
                body,
                is_async,
                is_generator,
                is_method: false,
                span,
            };
            Stmt::Expression {
                expr: Expr::Assign {
                    target: Box::new(Expr::Ident(name)),
                    op: AssignOp::Eq,
                    value: Box::new(fn_expr),
                    span,
                },
                span,
            }
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use crate::{link_entry, temp_link_dir};
    use std::fs;

    #[test]
    fn link_defer_and_eager_same_module_eval_order() {
        // E19.84.09: when a module is both `import defer` and later evaluation-phase,
        // evaluation order follows the non-deferred import site (not first load).
        let dir = temp_link_dir("defer-and-eager-order");
        fs::write(dir.join("setup.drac"), "globalThis.evaluations = [];\n").unwrap();
        fs::write(
            dir.join("dep1.drac"),
            "import \"./dep1a.drac\";\nglobalThis.evaluations.push(1);\n",
        )
        .unwrap();
        fs::write(
            dir.join("dep1a.drac"),
            "globalThis.evaluations.push(1.1);\n",
        )
        .unwrap();
        fs::write(dir.join("dep2.drac"), "globalThis.evaluations.push(2);\n").unwrap();
        let main = dir.join("main.drac");
        fs::write(
            &main,
            r#"
import "./setup.drac";
import defer * as ns1 from "./dep1.drac";
import "./dep2.drac";
import "./dep1.drac";
let order = globalThis.evaluations;
"#,
        )
        .unwrap();
        let program = link_entry(&main).expect("defer+eager link");
        let dump = draconic_ast::dump_program(&program);
        // dep2 body (push 2) must appear before dep1a (push 1.1) in the linked program.
        let p2 = dump.find("Number 2").expect("dep2 push");
        let p11 = dump.find("Number 1.1").expect("dep1a push");
        assert!(
            p2 < p11,
            "expected eval order dep2 before dep1 subgraph:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_defer_tla_flattening_uses_async_mp() {
        // E19.84.09: async transitive deps of deferred imports evaluate via __draconic_mp
        // so TLA can interleave with later sync modules.
        let dir = temp_link_dir("defer-tla-flatten");
        fs::write(dir.join("setup.drac"), "globalThis.evaluations = [];\n").unwrap();
        fs::write(
            dir.join("dep1.drac"),
            "globalThis.evaluations.push(\"1\");\n",
        )
        .unwrap();
        fs::write(
            dir.join("tla.drac"),
            "globalThis.evaluations.push(\"tla start\");\nawait Promise.resolve(0);\nglobalThis.evaluations.push(\"tla end\");\n",
        )
        .unwrap();
        fs::write(
            dir.join("dep2.drac"),
            "import \"./tla.drac\";\nglobalThis.evaluations.push(\"2\");\n",
        )
        .unwrap();
        fs::write(
            dir.join("dep3.drac"),
            "globalThis.evaluations.push(\"3\");\n",
        )
        .unwrap();
        let main = dir.join("main.drac");
        fs::write(
            &main,
            r#"
import "./setup.drac";
import "./dep1.drac";
import defer * as ns from "./dep2.drac";
import "./dep3.drac";
let _ = ns;
"#,
        )
        .unwrap();
        let program = link_entry(&main).expect("defer TLA flatten link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("__draconic_mp"),
            "expected async module promise slots, got:\n{dump}"
        );
        assert!(
            dump.contains("Await") || dump.contains("await"),
            "expected top-level await of async module eval, got:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_deferred_module_throws_evaluation_error_cache() {
        // E19.84.08: throwing deferred module records [[EvaluationError]]; dynamic
        // import and deferred-ns access share the same reason; merror helpers present.
        let dir = temp_link_dir("module-throws");
        fs::write(
            dir.join("throws.drac"),
            "throw { someError: \"the error from throws\" };\n",
        )
        .unwrap();
        fs::write(
            dir.join("defer_ns.drac"),
            "import defer * as ns from \"./throws.drac\";\nexport { ns };\n",
        )
        .unwrap();
        let main = dir.join("main.drac");
        fs::write(
            &main,
            r#"
import defer * as ns from "./throws.drac";
async function run() {
  let err1;
  await import("./throws.drac").catch(function (e) { err1 = e; });
  let err2;
  try { ns.foo; } catch (e) { err2 = e; }
  let err3;
  const mod = await import("./defer_ns.drac");
  try { mod.ns.foo; } catch (e) { err3 = e; }
  return err1 === err2 && err1 === err3;
}
"#,
        )
        .unwrap();
        let program = link_entry(&main).expect("module-throws link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("__draconic_merror") && dump.contains("__draconic_deferred_ns"),
            "{dump}"
        );
        assert!(
            dump.contains("__draconic_eval_m") || dump.contains("draconic_eval"),
            "{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
