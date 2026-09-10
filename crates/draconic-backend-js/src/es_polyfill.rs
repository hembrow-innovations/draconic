//! Stdlib and host JS polyfill injection for IR that references those APIs.

use draconic_ir::{AssignTarget, Expr, LocalId, Module, Stmt};

/// L03.01: true when the Program body references the stdlib `sha256` global.
fn module_uses_sha256(module: &Module) -> bool {
    module_uses_named_local(module, "sha256")
}

/// L10.01: true when the Program body references the stdlib `hmacSha256` global.
fn module_uses_hmac_sha256(module: &Module) -> bool {
    module_uses_named_local(module, "hmacSha256")
}

/// L10.02: true when the Program body references AEAD encrypt/decrypt.
fn module_uses_aead(module: &Module) -> bool {
    module_uses_named_local(module, "aeadEncrypt") || module_uses_named_local(module, "aeadDecrypt")
}

/// L03.02: true when the Program body references the stdlib `randomBytes` global.
fn module_uses_random_bytes(module: &Module) -> bool {
    module_uses_named_local(module, "randomBytes")
}

/// L04: gzip / gunzip / deflate / inflate.
fn module_uses_compression(module: &Module) -> bool {
    module_uses_named_local(module, "gzip")
        || module_uses_named_local(module, "gunzip")
        || module_uses_named_local(module, "deflate")
        || module_uses_named_local(module, "inflate")
}

/// L07 / L07.01 / L07.02: `parseFlags` / `flagHelp`.
fn module_uses_parse_flags(module: &Module) -> bool {
    module_uses_named_local(module, "parseFlags") || module_uses_named_local(module, "flagHelp")
}

/// L08.01: true when the Program body references the stdlib `parseUrl` global.
///
/// IR locals include every binder symbol (all builtins), so presence in
/// `module.locals` is not enough — walk the body for a use of the parseUrl local.
fn module_uses_parse_url(module: &Module) -> bool {
    module_uses_named_local(module, "parseUrl")
}

/// L08.02: `parseQuery` / `serializeQuery`.
fn module_uses_query(module: &Module) -> bool {
    module_uses_named_local(module, "parseQuery")
        || module_uses_named_local(module, "serializeQuery")
}

/// L09: `parseMultipart` / `serializeMultipart`.
fn module_uses_mime(module: &Module) -> bool {
    module_uses_named_local(module, "parseMultipart")
        || module_uses_named_local(module, "serializeMultipart")
}

/// L06.01: `createLogger`.
fn module_uses_create_logger(module: &Module) -> bool {
    module_uses_named_local(module, "createLogger")
}

/// L02.01 / L02.02: `groupBy` / `chunk` / `Deque`.
fn module_uses_collections(module: &Module) -> bool {
    module_uses_named_local(module, "groupBy")
        || module_uses_named_local(module, "chunk")
        || module_uses_named_local(module, "Deque")
}

/// L05.01 / L05.02 / L05.03: free `describe` / `it` / `expect` / hooks (IdentName so user `let it` does not collide).
fn module_uses_describe_it(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "describe")
            || stmt_uses_ident_name(s, "it")
            || stmt_uses_ident_name(s, "expect")
            || stmt_uses_ident_name(s, "before")
            || stmt_uses_ident_name(s, "after")
            || stmt_uses_ident_name(s, "beforeEach")
            || stmt_uses_ident_name(s, "afterEach")
    })
}

fn module_uses_named_local(module: &Module, name: &str) -> bool {
    let ids: Vec<LocalId> = module
        .locals
        .iter()
        .filter(|l| l.name == name)
        .map(|l| l.id)
        .collect();
    if !ids.is_empty() && module.body.iter().any(|s| stmt_uses_local(s, &ids)) {
        return true;
    }
    module_uses_ident(module, name)
}

fn module_uses_ident(module: &Module, name: &str) -> bool {
    module.body.iter().any(|s| stmt_uses_ident_name(s, name))
}

/// Inject JS host polyfills for catalog names used as free identifiers.
fn prepend_host_polyfills(module: &Module, out: &mut String) {
    let mut injected: Vec<&'static str> = Vec::new();
    for entry in draconic_check::host_apis() {
        if !module_uses_ident(module, entry.name) {
            continue;
        }
        let Some(src) = draconic_runtime::host_js_polyfill(entry.name) else {
            continue;
        };
        if injected.iter().any(|seen| std::ptr::eq(*seen, src)) {
            continue;
        }
        injected.push(src);
        out.push_str(src);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
}

fn stmt_uses_ident_name(stmt: &Stmt, name: &str) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. }
        | Stmt::DeclareArrayPattern { init: Some(e), .. }
        | Stmt::DeclareObjectPattern { init: Some(e), .. }
        | Stmt::Expr { expr: e }
        | Stmt::Throw { value: e } => expr_uses_ident_name(e, name),
        Stmt::Return { value: Some(e) } => expr_uses_ident_name(e, name),
        Stmt::Block { body } => body.iter().any(|s| stmt_uses_ident_name(s, name)),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_uses_ident_name(test, name)
                || stmt_uses_ident_name(consequent, name)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_uses_ident_name(a, name))
        }
        Stmt::While { test, body } | Stmt::DoWhile { test, body } => {
            expr_uses_ident_name(test, name) || stmt_uses_ident_name(body, name)
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            init.as_ref().is_some_and(|s| stmt_uses_ident_name(s, name))
                || test.as_ref().is_some_and(|e| expr_uses_ident_name(e, name))
                || update
                    .as_ref()
                    .is_some_and(|e| expr_uses_ident_name(e, name))
                || stmt_uses_ident_name(body, name)
        }
        Stmt::ForIn { left, right, body }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            stmt_uses_ident_name(left, name)
                || expr_uses_ident_name(right, name)
                || stmt_uses_ident_name(body, name)
        }
        Stmt::Labeled { body, .. } => stmt_uses_ident_name(body, name),
        Stmt::Switch {
            discriminant,
            cases,
        } => {
            expr_uses_ident_name(discriminant, name)
                || cases.iter().any(|c| {
                    c.test
                        .as_ref()
                        .is_some_and(|e| expr_uses_ident_name(e, name))
                        || c.body.iter().any(|s| stmt_uses_ident_name(s, name))
                })
        }
        Stmt::Function { body, .. } => body.iter().any(|s| stmt_uses_ident_name(s, name)),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(|s| stmt_uses_ident_name(s, name))
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(|s| stmt_uses_ident_name(s, name)))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(|s| stmt_uses_ident_name(s, name)))
        }
        Stmt::With { object, body } => {
            expr_uses_ident_name(object, name) || body.iter().any(|s| stmt_uses_ident_name(s, name))
        }
        _ => false,
    }
}

fn expr_uses_ident_name(expr: &Expr, name: &str) -> bool {
    use draconic_ir::{Arg, ArrayElement, ObjectProp, ObjectPropKey};
    match expr {
        Expr::IdentName { name: n, .. } => n == name,
        Expr::Unary { arg, .. } => expr_uses_ident_name(arg, name),
        Expr::Binary { left, right, .. } => {
            expr_uses_ident_name(left, name) || expr_uses_ident_name(right, name)
        }
        Expr::Assign { target, value, .. } => {
            let t = match target {
                AssignTarget::Member {
                    object, property, ..
                } => expr_uses_ident_name(object, name) || expr_uses_ident_name(property, name),
                _ => false,
            };
            t || expr_uses_ident_name(value, name)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_uses_ident_name(test, name)
                || expr_uses_ident_name(consequent, name)
                || expr_uses_ident_name(alternate, name)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_uses_ident_name(callee, name)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_uses_ident_name(e, name),
                })
        }
        Expr::Member {
            object, property, ..
        } => expr_uses_ident_name(object, name) || expr_uses_ident_name(property, name),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_uses_ident_name(e, name),
            ArrayElement::Elision => false,
        }),
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Spread(e) => expr_uses_ident_name(e, name),
            ObjectProp::Property { key, value } | ObjectProp::Accessor { key, value, .. } => {
                let key_hit = match key {
                    ObjectPropKey::Computed(e) => expr_uses_ident_name(e, name),
                    _ => false,
                };
                key_hit || expr_uses_ident_name(value, name)
            }
        }),
        Expr::Function { body, .. } => body.iter().any(|s| stmt_uses_ident_name(s, name)),
        _ => false,
    }
}

fn stmt_uses_local(stmt: &Stmt, ids: &[LocalId]) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. }
        | Stmt::DeclareArrayPattern { init: Some(e), .. }
        | Stmt::DeclareObjectPattern { init: Some(e), .. }
        | Stmt::Expr { expr: e }
        | Stmt::Throw { value: e } => expr_uses_local(e, ids),
        Stmt::Return { value: Some(e) } => expr_uses_local(e, ids),
        Stmt::Block { body } => body.iter().any(|s| stmt_uses_local(s, ids)),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_uses_local(test, ids)
                || stmt_uses_local(consequent, ids)
                || alternate.as_ref().is_some_and(|a| stmt_uses_local(a, ids))
        }
        Stmt::While { test, body } | Stmt::DoWhile { test, body } => {
            expr_uses_local(test, ids) || stmt_uses_local(body, ids)
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            init.as_ref().is_some_and(|s| stmt_uses_local(s, ids))
                || test.as_ref().is_some_and(|e| expr_uses_local(e, ids))
                || update.as_ref().is_some_and(|e| expr_uses_local(e, ids))
                || stmt_uses_local(body, ids)
        }
        Stmt::ForIn { left, right, body }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            stmt_uses_local(left, ids) || expr_uses_local(right, ids) || stmt_uses_local(body, ids)
        }
        Stmt::Labeled { body, .. } => stmt_uses_local(body, ids),
        Stmt::Switch {
            discriminant,
            cases,
        } => {
            expr_uses_local(discriminant, ids)
                || cases.iter().any(|c| {
                    c.test.as_ref().is_some_and(|e| expr_uses_local(e, ids))
                        || c.body.iter().any(|s| stmt_uses_local(s, ids))
                })
        }
        Stmt::Function { body, .. } => body.iter().any(|s| stmt_uses_local(s, ids)),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(|s| stmt_uses_local(s, ids))
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(|s| stmt_uses_local(s, ids)))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(|s| stmt_uses_local(s, ids)))
        }
        Stmt::With { object, body } => {
            expr_uses_local(object, ids) || body.iter().any(|s| stmt_uses_local(s, ids))
        }
        _ => false,
    }
}

fn expr_uses_local(expr: &Expr, ids: &[LocalId]) -> bool {
    use draconic_ir::{Arg, ArrayElement, ObjectProp, ObjectPropKey};
    match expr {
        Expr::Local { id, .. } => ids.contains(id),
        Expr::Unary { arg, .. } => expr_uses_local(arg, ids),
        Expr::Binary { left, right, .. } => {
            expr_uses_local(left, ids) || expr_uses_local(right, ids)
        }
        Expr::Assign { target, value, .. } => {
            let t = match target {
                AssignTarget::Local(id) => ids.contains(id),
                AssignTarget::Member {
                    object, property, ..
                } => expr_uses_local(object, ids) || expr_uses_local(property, ids),
                _ => false,
            };
            t || expr_uses_local(value, ids)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_uses_local(test, ids)
                || expr_uses_local(consequent, ids)
                || expr_uses_local(alternate, ids)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_uses_local(callee, ids)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_uses_local(e, ids),
                })
        }
        Expr::Member {
            object, property, ..
        } => expr_uses_local(object, ids) || expr_uses_local(property, ids),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_uses_local(e, ids),
            ArrayElement::Elision => false,
        }),
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Spread(e) => expr_uses_local(e, ids),
            ObjectProp::Property { key, value } | ObjectProp::Accessor { key, value, .. } => {
                let key_hit = match key {
                    ObjectPropKey::Computed(e) => expr_uses_local(e, ids),
                    _ => false,
                };
                key_hit || expr_uses_local(value, ids)
            }
        }),
        Expr::Function { body, .. } => body.iter().any(|s| stmt_uses_local(s, ids)),
        _ => false,
    }
}

pub(crate) fn prepend_polyfills(module: &Module, out: &mut String) {
    fn push(out: &mut String, src: &str) {
        out.push_str(src);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    if module_uses_sha256(module) {
        push(out, draconic_runtime::sha256_js_polyfill());
    }
    if module_uses_random_bytes(module) {
        push(out, draconic_runtime::random_bytes_js_polyfill());
    }
    if module_uses_hmac_sha256(module) {
        push(out, draconic_runtime::hmac_sha256_js_polyfill());
    }
    if module_uses_aead(module) {
        push(out, draconic_runtime::aead_js_polyfill());
    }
    if module_uses_compression(module) {
        push(out, draconic_runtime::compression_js_polyfill());
    }
    if module_uses_parse_flags(module) {
        push(out, draconic_runtime::parse_flags_js_polyfill());
    }
    if module_uses_parse_url(module) {
        push(out, draconic_runtime::parse_url_js_polyfill());
    }
    if module_uses_query(module) {
        push(out, draconic_runtime::query_js_polyfill());
    }
    if module_uses_mime(module) {
        push(out, draconic_runtime::mime_js_polyfill());
    }
    if module_uses_create_logger(module) {
        push(out, draconic_runtime::create_logger_js_polyfill());
    }
    if module_uses_collections(module) {
        push(out, draconic_runtime::collections_js_polyfill());
    }
    if module_uses_describe_it(module) {
        push(out, draconic_runtime::describe_it_js_polyfill());
    }
    prepend_host_polyfills(module, out);
}
