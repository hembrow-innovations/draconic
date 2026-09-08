//! Stdlib and host JS polyfill injection for IR that references those APIs.

use draconic_ir::{AssignTarget, Expr, LocalId, Module, Stmt};

/// L03.01: true when the Program body references the stdlib `sha256` global.
fn module_uses_sha256(module: &Module) -> bool {
    let ids: Vec<LocalId> = module
        .locals
        .iter()
        .filter(|l| l.name == "sha256")
        .map(|l| l.id)
        .collect();
    if ids.is_empty() {
        return false;
    }
    module.body.iter().any(|s| stmt_uses_local(s, &ids))
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
    let ids: Vec<LocalId> = module
        .locals
        .iter()
        .filter(|l| l.name == "randomBytes")
        .map(|l| l.id)
        .collect();
    if ids.is_empty() {
        return false;
    }
    module.body.iter().any(|s| stmt_uses_local(s, &ids))
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
    if ids.is_empty() {
        return false;
    }
    module.body.iter().any(|s| stmt_uses_local(s, &ids))
}

/// H01.01: free host API `processArgs` lowers as `IdentName` (not a builtin local).
fn module_uses_process_args(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "processArgs"))
}

/// H01.02: free host APIs `envGet` / `envSet` / `envDelete`.
fn module_uses_process_env(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "envGet")
            || stmt_uses_ident_name(s, "envSet")
            || stmt_uses_ident_name(s, "envDelete")
    })
}

/// H01.03: free host APIs `exit` / `exitCode` / `setExitCode`.
fn module_uses_process_exit(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "exit")
            || stmt_uses_ident_name(s, "exitCode")
            || stmt_uses_ident_name(s, "setExitCode")
    })
}

/// H01.04: free host APIs `pid` / `ppid`.
fn module_uses_process_pid(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "pid") || stmt_uses_ident_name(s, "ppid"))
}

/// H16.01: free host APIs `cwd` / `chdir`.
fn module_uses_cwd_chdir(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "cwd") || stmt_uses_ident_name(s, "chdir"))
}

/// H16.02: free host APIs `hostname` / `osType` / `osArch`.
fn module_uses_hostname_os(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "hostname")
            || stmt_uses_ident_name(s, "osType")
            || stmt_uses_ident_name(s, "osArch")
    })
}

/// H16.03: free host APIs `tempDir` / `homeDir`.
fn module_uses_temp_home(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "tempDir") || stmt_uses_ident_name(s, "homeDir"))
}

/// H15.01: free host API `processRun`.
fn module_uses_process_run(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "processRun"))
}

/// H15.02: process spawn + pipes + kill.
fn module_uses_process_spawn(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "processSpawn")
            || stmt_uses_ident_name(s, "processStdinWrite")
            || stmt_uses_ident_name(s, "processWait")
            || stmt_uses_ident_name(s, "processStdout")
            || stmt_uses_ident_name(s, "processStderr")
            || stmt_uses_ident_name(s, "processKill")
            || stmt_uses_ident_name(s, "processClose")
    })
}

/// C01.01 / C01.02 / C01.03: free host APIs `spawnWorker` / `joinWorker` / `terminateWorker`.
fn module_uses_spawn_worker(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "spawnWorker")
            || stmt_uses_ident_name(s, "joinWorker")
            || stmt_uses_ident_name(s, "terminateWorker")
    })
}

/// C02.01–C02.03: free host APIs `makeChannel` / `channelSend` / `channelRecv`.
fn module_uses_channel(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "makeChannel")
            || stmt_uses_ident_name(s, "channelSend")
            || stmt_uses_ident_name(s, "channelRecv")
    })
}

/// C05.01 / C05.02: free host APIs `makeCancelToken` / `cancelTokenAbort` /
/// `cancelTokenAborted` / `cancelTokenLink` / `withTimeout` / `clearWithTimeout`.
fn module_uses_cancel_token(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "makeCancelToken")
            || stmt_uses_ident_name(s, "cancelTokenAbort")
            || stmt_uses_ident_name(s, "cancelTokenAborted")
            || stmt_uses_ident_name(s, "cancelTokenLink")
            || stmt_uses_ident_name(s, "withTimeout")
            || stmt_uses_ident_name(s, "clearWithTimeout")
    })
}

/// H05.01: free host API `nowMs`.
fn module_uses_now_ms(module: &Module) -> bool {
    module.body.iter().any(|s| stmt_uses_ident_name(s, "nowMs"))
}

/// H05.02: free host API `monotonicMs`.
fn module_uses_monotonic_ms(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "monotonicMs"))
}

/// H05.03: free host APIs `setTimeout` / `clearTimeout`.
fn module_uses_set_timeout(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "setTimeout") || stmt_uses_ident_name(s, "clearTimeout"))
}

/// H05.04: free host APIs `setInterval` / `clearInterval`.
fn module_uses_set_interval(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "setInterval") || stmt_uses_ident_name(s, "clearInterval"))
}

/// H02.01: free host API `stdoutWrite`.
fn module_uses_stdout_write(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "stdoutWrite"))
}

/// H02.02: free host API `stderrWrite`.
fn module_uses_stderr_write(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "stderrWrite"))
}

/// H02.03: free host APIs `stdinReadLine` / `stdinReadBytes`.
fn module_uses_stdin_read(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "stdinReadLine") || stmt_uses_ident_name(s, "stdinReadBytes")
    })
}

/// H03.01–H03.03: free host path APIs.
fn module_uses_path(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "pathJoin")
            || stmt_uses_ident_name(s, "pathNormalize")
            || stmt_uses_ident_name(s, "pathDirname")
            || stmt_uses_ident_name(s, "pathBasename")
            || stmt_uses_ident_name(s, "pathExtname")
            || stmt_uses_ident_name(s, "pathIsAbsolute")
            || stmt_uses_ident_name(s, "pathResolve")
    })
}

/// H04.01–H04.05: free host file-read / write / append / exists / stat / dir / rename / copy APIs.
fn module_uses_fs_read(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "readFileText")
            || stmt_uses_ident_name(s, "readFileBytes")
            || stmt_uses_ident_name(s, "writeFileText")
            || stmt_uses_ident_name(s, "writeFileBytes")
            || stmt_uses_ident_name(s, "appendFileText")
            || stmt_uses_ident_name(s, "appendFileBytes")
            || stmt_uses_ident_name(s, "exists")
            || stmt_uses_ident_name(s, "stat")
            || stmt_uses_ident_name(s, "mkdir")
            || stmt_uses_ident_name(s, "mkdirAll")
            || stmt_uses_ident_name(s, "readdir")
            || stmt_uses_ident_name(s, "rmdir")
            || stmt_uses_ident_name(s, "removeFile")
            || stmt_uses_ident_name(s, "renameFile")
            || stmt_uses_ident_name(s, "copyFile")
    })
}

/// H17.04: HTTP/1.1 helpers (parse/write request/response).
fn module_uses_http_helpers(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "httpParseRequest")
            || stmt_uses_ident_name(s, "httpRequestHeader")
            || stmt_uses_ident_name(s, "httpWriteResponse")
            || stmt_uses_ident_name(s, "httpWriteRequest")
            || stmt_uses_ident_name(s, "httpParseResponse")
            || stmt_uses_ident_name(s, "httpResponseHeader")
    })
}

/// H17.04: `dnsLookup` Node bridge.
fn module_uses_dns_lookup(module: &Module) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_uses_ident_name(s, "dnsLookup"))
}

/// H17.04: sync TCP Node `net` bridge.
fn module_uses_tcp(module: &Module) -> bool {
    module.body.iter().any(|s| {
        stmt_uses_ident_name(s, "tcpListen")
            || stmt_uses_ident_name(s, "tcpLocalPort")
            || stmt_uses_ident_name(s, "closeTcp")
            || stmt_uses_ident_name(s, "tcpAccept")
            || stmt_uses_ident_name(s, "tcpConnect")
            || stmt_uses_ident_name(s, "tcpPeerAddress")
            || stmt_uses_ident_name(s, "tcpPeerPort")
            || stmt_uses_ident_name(s, "tcpRead")
            || stmt_uses_ident_name(s, "tcpWrite")
            || stmt_uses_ident_name(s, "tcpShutdown")
    })
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
    if module_uses_process_args(module) {
        push(out, draconic_runtime::process_args_js_polyfill());
    }
    if module_uses_process_env(module) {
        push(out, draconic_runtime::process_env_js_polyfill());
    }
    if module_uses_process_exit(module) {
        push(out, draconic_runtime::process_exit_js_polyfill());
    }
    if module_uses_process_pid(module) {
        push(out, draconic_runtime::process_pid_js_polyfill());
    }
    if module_uses_cwd_chdir(module) {
        push(out, draconic_runtime::cwd_chdir_js_polyfill());
    }
    if module_uses_hostname_os(module) {
        push(out, draconic_runtime::hostname_os_js_polyfill());
    }
    if module_uses_temp_home(module) {
        push(out, draconic_runtime::temp_home_js_polyfill());
    }
    if module_uses_process_run(module) {
        push(out, draconic_runtime::process_run_js_polyfill());
    }
    if module_uses_process_spawn(module) {
        push(out, draconic_runtime::process_spawn_js_polyfill());
    }
    if module_uses_spawn_worker(module) {
        push(out, draconic_runtime::spawn_worker_js_polyfill());
    }
    if module_uses_channel(module) {
        push(out, draconic_runtime::channel_js_polyfill());
    }
    if module_uses_cancel_token(module) {
        push(out, draconic_runtime::cancel_token_js_polyfill());
    }
    if module_uses_now_ms(module) {
        push(out, draconic_runtime::now_ms_js_polyfill());
    }
    if module_uses_monotonic_ms(module) {
        push(out, draconic_runtime::monotonic_ms_js_polyfill());
    }
    if module_uses_set_timeout(module) {
        push(out, draconic_runtime::set_timeout_js_polyfill());
    }
    if module_uses_set_interval(module) {
        push(out, draconic_runtime::set_interval_js_polyfill());
    }
    if module_uses_stdout_write(module) {
        push(out, draconic_runtime::stdout_write_js_polyfill());
    }
    if module_uses_stderr_write(module) {
        push(out, draconic_runtime::stderr_write_js_polyfill());
    }
    if module_uses_stdin_read(module) {
        push(out, draconic_runtime::stdin_read_js_polyfill());
    }
    if module_uses_path(module) {
        push(out, draconic_runtime::path_js_polyfill());
    }
    if module_uses_fs_read(module) {
        push(out, draconic_runtime::fs_read_js_polyfill());
    }
    if module_uses_http_helpers(module) {
        push(out, draconic_runtime::http_js_polyfill());
    }
    if module_uses_dns_lookup(module) {
        push(out, draconic_runtime::dns_js_polyfill());
    }
    if module_uses_tcp(module) {
        push(out, draconic_runtime::tcp_js_polyfill());
    }
}
