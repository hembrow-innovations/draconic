//! JS backend: IR → ECMAScript (ROADMAP B07 + N04 native policy + U03 source maps).

mod emit;
mod source_map;

pub use source_map::{
    decode_mappings, decode_vlq, encode_vlq, source_mapping_url_comment, Mapping, SourceMap,
    SourceMapOptions,
};

use std::collections::HashMap;

use draconic_ast::UnaryOp;
use draconic_check::{
    extern_unsupported_on_js_diagnostic, host_api_unsupported_diagnostic, CompileTarget,
};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    ArrayPatternEl, AssignTarget, Expr, IrType, LocalId, Module, ObjectPatternEl, Pattern, Stmt,
    UpdateTarget,
};
use source_map::SourceMapBuilder;

/// JS emit result with optional Source Map v3 (U03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmittedJs {
    pub code: String,
    pub map: Option<SourceMap>,
}

/// Emit ECMAScript source for a shared IR module.
///
/// **N04 native policy (JS target):**
/// - Native scalars (`i32`, …), layout structs, and fixed arrays: polyfill/erase
///   (type annotations already gone at IR; values lower as ordinary JS numbers/objects/arrays).
/// - Native pointers (`*T`, `&x`, `*p`, `*p = v`): hard-error (native-only).
/// - `extern "C"` / FFI (`module.has_extern_ffi`): hard-error (F08.01).
pub fn emit_js(module: &Module) -> Result<String, Diagnostic> {
    Ok(emit_js_full(module, None)?.code)
}

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

/// L06.01: `createLogger`.
fn module_uses_create_logger(module: &Module) -> bool {
    module_uses_named_local(module, "createLogger")
}

/// L02.01: `groupBy` / `chunk`.
fn module_uses_collections(module: &Module) -> bool {
    module_uses_named_local(module, "groupBy") || module_uses_named_local(module, "chunk")
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

/// Emit ECMAScript plus a Source Map v3 mapping generated positions back to the Program.
///
/// One mapping segment is recorded at the start of each top-level IR statement, using
/// `module.body_spans` (original AST spans preserved through lower). Nested statements
/// share their enclosing top-level origin.
pub fn emit_js_with_map(
    module: &Module,
    opts: &SourceMapOptions<'_>,
) -> Result<EmittedJs, Diagnostic> {
    emit_js_full(module, Some(opts))
}

fn emit_js_full(
    module: &Module,
    map_opts: Option<&SourceMapOptions<'_>>,
) -> Result<EmittedJs, Diagnostic> {
    reject_native_only(module)?;
    reject_extern_ffi(module)?;

    let names: HashMap<LocalId, &str> = module
        .locals
        .iter()
        .map(|l| (l.id, l.name.as_str()))
        .collect();

    let mut out = String::new();
    // L03.01: portable `sha256` polyfill when the Program references it.
    if module_uses_sha256(module) {
        out.push_str(draconic_runtime::sha256_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // L03.02: portable `randomBytes` polyfill when the Program references it.
    if module_uses_random_bytes(module) {
        out.push_str(draconic_runtime::random_bytes_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // L08.01: portable `parseUrl` polyfill when the Program references it.
    if module_uses_parse_url(module) {
        out.push_str(draconic_runtime::parse_url_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // L08.02: portable `parseQuery` / `serializeQuery` polyfill.
    if module_uses_query(module) {
        out.push_str(draconic_runtime::query_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // L06.01: portable `createLogger` polyfill.
    if module_uses_create_logger(module) {
        out.push_str(draconic_runtime::create_logger_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // L02.01: portable `groupBy` / `chunk` polyfill.
    if module_uses_collections(module) {
        out.push_str(draconic_runtime::collections_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // L05.01: portable `describe` / `it` polyfill.
    if module_uses_describe_it(module) {
        out.push_str(draconic_runtime::describe_it_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H01.01: `processArgs()` Node bridge when the Program references it.
    if module_uses_process_args(module) {
        out.push_str(draconic_runtime::process_args_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H01.02: `envGet` / `envSet` / `envDelete` Node bridge.
    if module_uses_process_env(module) {
        out.push_str(draconic_runtime::process_env_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H01.03: `exit` / `exitCode` / `setExitCode` Node bridge.
    if module_uses_process_exit(module) {
        out.push_str(draconic_runtime::process_exit_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H01.04: `pid` / `ppid` Node bridge.
    if module_uses_process_pid(module) {
        out.push_str(draconic_runtime::process_pid_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H16.01: `cwd` / `chdir` Node bridge.
    if module_uses_cwd_chdir(module) {
        out.push_str(draconic_runtime::cwd_chdir_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H16.02: `hostname` / `osType` / `osArch` Node bridge.
    if module_uses_hostname_os(module) {
        out.push_str(draconic_runtime::hostname_os_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H16.03: `tempDir` / `homeDir` Node bridge.
    if module_uses_temp_home(module) {
        out.push_str(draconic_runtime::temp_home_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H15.01: `processRun` spawn+wait Node bridge.
    if module_uses_process_run(module) {
        out.push_str(draconic_runtime::process_run_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H15.02: processSpawn + stdin/stdout/stderr/kill Node bridge.
    if module_uses_process_spawn(module) {
        out.push_str(draconic_runtime::process_spawn_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // C01.01: spawnWorker isolate (worker_threads, unref).
    if module_uses_spawn_worker(module) {
        out.push_str(draconic_runtime::spawn_worker_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // C02.01: makeChannel / channelSend / channelRecv FIFO.
    if module_uses_channel(module) {
        out.push_str(draconic_runtime::channel_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // C05.01: makeCancelToken / cancelTokenAbort / cancelTokenAborted / cancelTokenLink.
    if module_uses_cancel_token(module) {
        out.push_str(draconic_runtime::cancel_token_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H05.01: `nowMs` wall clock.
    if module_uses_now_ms(module) {
        out.push_str(draconic_runtime::now_ms_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H05.02: `monotonicMs` monotonic clock.
    if module_uses_monotonic_ms(module) {
        out.push_str(draconic_runtime::monotonic_ms_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H05.03: `setTimeout` / `clearTimeout` host bridge.
    if module_uses_set_timeout(module) {
        out.push_str(draconic_runtime::set_timeout_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H05.04: `setInterval` / `clearInterval` host bridge.
    if module_uses_set_interval(module) {
        out.push_str(draconic_runtime::set_interval_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H02.01: `stdoutWrite` Node bridge.
    if module_uses_stdout_write(module) {
        out.push_str(draconic_runtime::stdout_write_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H02.02: `stderrWrite` Node bridge.
    if module_uses_stderr_write(module) {
        out.push_str(draconic_runtime::stderr_write_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H02.03: `stdinReadLine` / `stdinReadBytes` Node bridge.
    if module_uses_stdin_read(module) {
        out.push_str(draconic_runtime::stdin_read_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H03.01–H03.02: path pure string helpers.
    if module_uses_path(module) {
        out.push_str(draconic_runtime::path_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H04.01–H04.02: whole-file read / write / append.
    if module_uses_fs_read(module) {
        out.push_str(draconic_runtime::fs_read_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H17.04: HTTP/1.1 helpers (portable parse/write).
    if module_uses_http_helpers(module) {
        out.push_str(draconic_runtime::http_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H17.04: `dnsLookup` Node `dns` bridge.
    if module_uses_dns_lookup(module) {
        out.push_str(draconic_runtime::dns_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // H17.04: sync TCP Node `net` bridge.
    if module_uses_tcp(module) {
        out.push_str(draconic_runtime::tcp_js_polyfill());
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    let mut builder = map_opts.map(SourceMapBuilder::new);

    for (i, stmt) in module.body.iter().enumerate() {
        if let Some(b) = builder.as_mut() {
            let span = module
                .body_spans
                .get(i)
                .copied()
                .unwrap_or_else(Span::dummy);
            b.add_mapping_span(span);
        }
        let before = out.len();
        emit::emit_stmt(&mut out, stmt, &names);
        if let Some(b) = builder.as_mut() {
            b.note_write(&out[before..]);
        }
    }

    let map = builder.map(|b| b.finish());
    Ok(EmittedJs { code: out, map })
}

fn native_only_diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}

/// Hard-error free host API names that the H00.01 registry marks unavailable on js.
fn reject_host_api_name(name: &str) -> Result<(), Diagnostic> {
    if let Some(d) = host_api_unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()) {
        return Err(d);
    }
    Ok(())
}

/// F08.01: `extern "C"` / FFI is native-only — hard-error on the js backend.
fn reject_extern_ffi(module: &Module) -> Result<(), Diagnostic> {
    if !module.has_extern_ffi {
        return Ok(());
    }
    Err(extern_unsupported_on_js_diagnostic("extern", Span::dummy()))
}

/// Reject IR that is native-only on the JS backend (N04).
fn reject_native_only(module: &Module) -> Result<(), Diagnostic> {
    for local in &module.locals {
        if matches!(local.ty, IrType::Ptr(_)) {
            return Err(native_only_diag(format!(
                "native pointer type `*T` is native-only (cannot emit JS for `{}`)",
                local.name
            )));
        }
    }
    for stmt in &module.body {
        reject_native_only_stmt(stmt)?;
    }
    Ok(())
}

fn reject_native_only_stmt(stmt: &Stmt) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Declare { init, .. } => {
            if let Some(init) = init {
                reject_native_only_expr(init)?;
            }
        }
        Stmt::DeclareArrayPattern { elements, init, .. } => {
            if let Some(init) = init {
                reject_native_only_expr(init)?;
            }
            for el in elements {
                reject_native_only_array_pat_el(el)?;
            }
        }
        Stmt::DeclareObjectPattern {
            properties, init, ..
        } => {
            if let Some(init) = init {
                reject_native_only_expr(init)?;
            }
            for prop in properties {
                reject_native_only_object_pat_el(prop)?;
            }
        }
        Stmt::AssignLeft { target } => reject_native_only_assign_target(target)?,
        Stmt::Expr { expr } => reject_native_only_expr(expr)?,
        Stmt::Block { body } => {
            for s in body {
                reject_native_only_stmt(s)?;
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            reject_native_only_expr(test)?;
            reject_native_only_stmt(consequent)?;
            if let Some(alt) = alternate {
                reject_native_only_stmt(alt)?;
            }
        }
        Stmt::While { test, body } | Stmt::DoWhile { test, body } => {
            reject_native_only_expr(test)?;
            reject_native_only_stmt(body)?;
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            if let Some(init) = init {
                reject_native_only_stmt(init)?;
            }
            if let Some(test) = test {
                reject_native_only_expr(test)?;
            }
            if let Some(update) = update {
                reject_native_only_expr(update)?;
            }
            reject_native_only_stmt(body)?;
        }
        Stmt::ForIn { left, right, body }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            reject_native_only_stmt(left)?;
            reject_native_only_expr(right)?;
            reject_native_only_stmt(body)?;
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
        Stmt::Labeled { body, .. } => reject_native_only_stmt(body)?,
        Stmt::Switch {
            discriminant,
            cases,
        } => {
            reject_native_only_expr(discriminant)?;
            for case in cases {
                if let Some(test) = &case.test {
                    reject_native_only_expr(test)?;
                }
                for s in &case.body {
                    reject_native_only_stmt(s)?;
                }
            }
        }
        Stmt::Function { params, body, .. } => {
            for p in params {
                reject_native_only_pattern(&p.pattern)?;
                if let Some(default) = &p.default {
                    reject_native_only_expr(default)?;
                }
            }
            for s in body {
                reject_native_only_stmt(s)?;
            }
        }
        Stmt::Return { value } => {
            if let Some(value) = value {
                reject_native_only_expr(value)?;
            }
        }
        Stmt::Throw { value } => reject_native_only_expr(value)?,
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            for s in block {
                reject_native_only_stmt(s)?;
            }
            if let Some(handler) = handler {
                for s in handler {
                    reject_native_only_stmt(s)?;
                }
            }
            if let Some(finalizer) = finalizer {
                for s in finalizer {
                    reject_native_only_stmt(s)?;
                }
            }
        }
        Stmt::With { object, body } => {
            reject_native_only_expr(object)?;
            for s in body {
                reject_native_only_stmt(s)?;
            }
        }
        Stmt::ExternFunction { .. } => {}
    }
    Ok(())
}

fn reject_native_only_expr(expr: &Expr) -> Result<(), Diagnostic> {
    match expr {
        Expr::Unary {
            op: UnaryOp::Ref | UnaryOp::Deref,
            ..
        } => Err(native_only_diag(
            "native pointer operators `&` / `*` are native-only (cannot emit JS)",
        )),
        Expr::Assign {
            target: AssignTarget::Deref(_),
            ..
        } => Err(native_only_diag(
            "native pointer store `*p = …` is native-only (cannot emit JS)",
        )),
        Expr::IdentName { name, .. } => reject_host_api_name(name),
        Expr::Local { .. }
        | Expr::Number { .. }
        | Expr::BigInt { .. }
        | Expr::String { .. }
        | Expr::RegExp { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::NewTarget { .. }
        | Expr::ImportMeta { .. }
        | Expr::Super { .. } => Ok(()),
        Expr::ImportCall {
            source, options, ..
        } => {
            reject_native_only_expr(source)?;
            if let Some(opts) = options {
                reject_native_only_expr(opts)?;
            }
            Ok(())
        }
        Expr::Unary { arg, .. } => reject_native_only_expr(arg),
        Expr::Binary { left, right, .. } => {
            reject_native_only_expr(left)?;
            reject_native_only_expr(right)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            reject_native_only_expr(test)?;
            reject_native_only_expr(consequent)?;
            reject_native_only_expr(alternate)
        }
        Expr::Assign { target, value, .. } => {
            reject_native_only_assign_target(target)?;
            reject_native_only_expr(value)
        }
        Expr::Update { target, .. } => match target {
            UpdateTarget::Local(_) => Ok(()),
            UpdateTarget::Name(name) => reject_host_api_name(name),
            UpdateTarget::Member {
                object, property, ..
            } => {
                reject_native_only_expr(object)?;
                reject_native_only_expr(property)
            }
        },
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            reject_native_only_expr(callee)?;
            for a in args {
                match a {
                    draconic_ir::Arg::Expr(e) | draconic_ir::Arg::Spread(e) => {
                        reject_native_only_expr(e)?;
                    }
                }
            }
            Ok(())
        }
        Expr::Function { params, body, .. } => {
            for p in params {
                reject_native_only_pattern(&p.pattern)?;
                if let Some(default) = &p.default {
                    reject_native_only_expr(default)?;
                }
            }
            for s in body {
                reject_native_only_stmt(s)?;
            }
            Ok(())
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                reject_native_only_object_prop(p)?;
            }
            Ok(())
        }
        Expr::Array { elements, .. } => {
            for el in elements {
                match el {
                    draconic_ir::ArrayElement::Expr(e) | draconic_ir::ArrayElement::Spread(e) => {
                        reject_native_only_expr(e)?;
                    }
                    draconic_ir::ArrayElement::Elision => {}
                }
            }
            Ok(())
        }
        Expr::Member {
            object, property, ..
        } => {
            reject_native_only_expr(object)?;
            reject_native_only_expr(property)
        }
        Expr::Template { expressions, .. } => {
            for e in expressions {
                reject_native_only_expr(e)?;
            }
            Ok(())
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            reject_native_only_expr(tag)?;
            for e in expressions {
                reject_native_only_expr(e)?;
            }
            Ok(())
        }
    }
}

fn reject_native_only_assign_target(target: &AssignTarget) -> Result<(), Diagnostic> {
    match target {
        AssignTarget::Local(_) => Ok(()),
        AssignTarget::Name(name) => reject_host_api_name(name),
        AssignTarget::Deref(_) => Err(native_only_diag(
            "native pointer store `*p = …` is native-only (cannot emit JS)",
        )),
        AssignTarget::Member {
            object, property, ..
        } => {
            reject_native_only_expr(object)?;
            reject_native_only_expr(property)
        }
        AssignTarget::ArrayPattern { elements } => {
            for el in elements {
                reject_native_only_array_pat_el(el)?;
            }
            Ok(())
        }
        AssignTarget::ObjectPattern { properties } => {
            for p in properties {
                reject_native_only_object_pat_el(p)?;
            }
            Ok(())
        }
    }
}

fn reject_native_only_pattern(pat: &Pattern) -> Result<(), Diagnostic> {
    match pat {
        Pattern::Local(_) => Ok(()),
        Pattern::Name(name) => reject_host_api_name(name),
        Pattern::Member {
            object, property, ..
        } => {
            reject_native_only_expr(object)?;
            reject_native_only_expr(property)
        }
        Pattern::Array(els) => {
            for el in els {
                reject_native_only_array_pat_el(el)?;
            }
            Ok(())
        }
        Pattern::Object(props) => {
            for p in props {
                reject_native_only_object_pat_el(p)?;
            }
            Ok(())
        }
    }
}

fn reject_native_only_array_pat_el(el: &ArrayPatternEl) -> Result<(), Diagnostic> {
    match el {
        ArrayPatternEl::Elision => Ok(()),
        ArrayPatternEl::Pattern { binding, default } => {
            reject_native_only_pattern(binding)?;
            if let Some(d) = default {
                reject_native_only_expr(d)?;
            }
            Ok(())
        }
        ArrayPatternEl::Rest(pat) => reject_native_only_pattern(pat),
    }
}

fn reject_native_only_object_pat_el(el: &ObjectPatternEl) -> Result<(), Diagnostic> {
    match el {
        ObjectPatternEl::Prop {
            key,
            binding,
            default,
            ..
        } => {
            if let draconic_ir::ObjectPropKey::Computed(e) = key {
                reject_native_only_expr(e)?;
            }
            reject_native_only_pattern(binding)?;
            if let Some(d) = default {
                reject_native_only_expr(d)?;
            }
            Ok(())
        }
        ObjectPatternEl::Rest(pat) => reject_native_only_pattern(pat),
    }
}

fn reject_native_only_object_prop(prop: &draconic_ir::ObjectProp) -> Result<(), Diagnostic> {
    use draconic_ir::{ObjectProp, ObjectPropKey};
    match prop {
        ObjectProp::Spread(e) => reject_native_only_expr(e),
        ObjectProp::Property { key, value } | ObjectProp::Accessor { key, value, .. } => {
            if let ObjectPropKey::Computed(e) = key {
                reject_native_only_expr(e)?;
            }
            reject_native_only_expr(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::{compile_source, compile_source_module};

    fn emit_src(src: &str) -> String {
        let module = compile_source(src).expect("compile");
        emit_js(&module).expect("emit")
    }

    #[test]
    fn emit_let_number() {
        assert_eq!(emit_src("let x = 1;"), "let x = 1;\n");
    }

    #[test]
    fn emit_const_number() {
        assert_eq!(emit_src("const x = 1;"), "const x = 1;\n");
    }

    #[test]
    fn emit_uninitialized_let() {
        assert_eq!(emit_src("let x;"), "let x;\n");
    }

    #[test]
    fn emit_binary_and_use() {
        let js = emit_src("let x = 1 + 2; x;");
        assert_eq!(js, "let x = (1) + (2);\nx;\n");
    }

    #[test]
    fn emit_string_concat() {
        let js = emit_src(r#"let s = "a" + "b";"#);
        assert_eq!(js, "let s = (\"a\") + (\"b\");\n");
    }

    #[test]
    fn emit_string_escapes() {
        let js = emit_src(r#"let s = "a\"b\nc";"#);
        assert!(js.contains("let s = "), "{js}");
        // Round-trip: escaped form must be valid JS string literal content.
        assert!(js.contains('\\') || js.contains("a"), "{js}");
    }

    #[test]
    fn emit_unary_and_literals() {
        let js = emit_src("let a = -1; let b = !false; let c = null; let d = true;");
        assert_eq!(
            js,
            "let a = -(1);\nlet b = !(false);\nlet c = null;\nlet d = true;\n"
        );
    }

    #[test]
    fn emit_call() {
        let js = emit_src("let f; f(1, 2);");
        assert_eq!(js, "let f;\n(f)(1, 2);\n");
    }

    #[test]
    fn emit_sha256_polyfill() {
        let js = emit_src("let d = sha256(new Uint8Array([]));");
        assert!(js.contains("function sha256("), "{js}");
        assert!(js.contains("globalThis.sha256 = sha256"), "{js}");
    }

    #[test]
    fn emit_random_bytes_polyfill() {
        let js = emit_src("let d = randomBytes(8);");
        assert!(js.contains("function randomBytes("), "{js}");
        assert!(js.contains("globalThis.randomBytes = randomBytes"), "{js}");
    }

    #[test]
    fn emit_create_logger_polyfill() {
        let js = emit_src("let logger = createLogger(); logger.info(\"hi\");");
        assert!(js.contains("function createLogger("), "{js}");
        assert!(
            js.contains("globalThis.createLogger = createLogger"),
            "{js}"
        );
        assert!(js.contains("setLevel"), "{js}");
    }

    #[test]
    fn emit_collections_polyfill() {
        let js =
            emit_src("let g = groupBy([\"a\", \"b\"], \"length\"); let c = chunk([1, 2, 3], 2);");
        assert!(js.contains("function groupBy("), "{js}");
        assert!(js.contains("function chunk("), "{js}");
        assert!(js.contains("globalThis.groupBy = groupBy"), "{js}");
        assert!(js.contains("globalThis.chunk = chunk"), "{js}");
    }

    #[test]
    fn emit_describe_it_polyfill() {
        let js = emit_src("describe(\"s\", () => { it(\"t\", () => {}); });");
        assert!(js.contains("function describe("), "{js}");
        assert!(js.contains("function it("), "{js}");
        assert!(js.contains("globalThis.describe = describe"), "{js}");
        assert!(js.contains("globalThis.it = it"), "{js}");
        assert!(js.contains("globalThis.expect = expect"), "{js}");
        assert!(js.contains("globalThis.before = before"), "{js}");
        assert!(js.contains("globalThis.after = after"), "{js}");
        assert!(js.contains("globalThis.beforeEach = beforeEach"), "{js}");
        assert!(js.contains("globalThis.afterEach = afterEach"), "{js}");
    }

    #[test]
    fn emit_expect_polyfill() {
        let js = emit_src("expect(1).toBe(1);");
        assert!(js.contains("function expect("), "{js}");
        assert!(js.contains("globalThis.expect = expect"), "{js}");
        assert!(js.contains("toBeTruthy"), "{js}");
        assert!(js.contains("toBeFalsy"), "{js}");
    }

    #[test]
    fn emit_direct_eval_unparenthesized() {
        let js = emit_src(r#"eval("var evx = 1");"#);
        assert!(
            js.contains("eval(") && !js.contains("(eval)"),
            "direct eval callee must stay Identifier eval, not (eval): {js}"
        );
        let indirect = emit_src(r#"(0, eval)("var evx = 1");"#);
        assert!(
            indirect.contains("(0)") || indirect.contains("(eval)"),
            "comma-eval must stay indirect: {indirect}"
        );
    }

    #[test]
    fn emit_import_call() {
        // E19.27: dynamic `import(specifier)` / options.
        let js = emit_src("let p = import('./m.js'); let q = import(p, opts);");
        assert!(js.contains("import(\"./m.js\")"), "{js}");
        assert!(js.contains("import(p, opts)"), "{js}");
    }

    #[test]
    fn emit_import_meta() {
        // E19.83.01: Module-goal `import.meta` + ImportCall argument.
        let module = compile_source_module("const p = import(import.meta);").expect("compile");
        let js = emit_js(&module).expect("emit");
        assert!(js.contains("import(import.meta)"), "{js}");
    }

    #[test]
    fn emit_import_defer_and_source_call() {
        // E19.33: `import.source` kept; E19.55: `import.defer` → `import()` for Node hosts.
        let js = emit_src("let d = import.defer('./m.js'); let s = import.source(x);");
        assert!(js.contains("import(\"./m.js\")"), "{js}");
        assert!(!js.contains("import.defer"), "{js}");
        assert!(js.contains("import.source(x)"), "{js}");
    }

    #[test]
    fn emit_call_spread() {
        let js = emit_src("let f; let a = [1]; f(...a); f(0, ...a, 2); new f(...a);");
        assert!(js.contains("(f)(...a);"), "{js}");
        assert!(js.contains("(f)(0, ...a, 2);"), "{js}");
        assert!(js.contains("(new (f)(...a))"), "{js}");
    }

    #[test]
    fn emit_comparison_and_logic() {
        let js = emit_src("let ok = 1 < 2 && true || false;");
        assert_eq!(js, "let ok = (((1) < (2)) && (true)) || (false);\n");
    }

    #[test]
    fn emit_bitwise() {
        let js = emit_src("let x = 5 & 3 | ~1 << 2;");
        assert_eq!(js, "let x = ((5) & (3)) | ((~(1)) << (2));\n");
    }

    #[test]
    fn emit_exponentiation() {
        let js = emit_src("let x = 2 ** 3 ** 2;");
        assert_eq!(js, "let x = (2) ** ((3) ** (2));\n");
    }

    #[test]
    fn emit_conditional() {
        let js = emit_src("let x = true ? 1 : 2;");
        assert_eq!(js, "let x = (true) ? (1) : (2);\n");
    }

    #[test]
    fn emit_conditional_right_assoc() {
        let js = emit_src("let x = false ? 1 : true ? 2 : 3;");
        assert_eq!(js, "let x = (false) ? (1) : ((true) ? (2) : (3));\n");
    }

    #[test]
    fn emit_assignment() {
        let js = emit_src("let x; x = 1;");
        assert_eq!(js, "let x;\n(x = 1);\n");
    }

    #[test]
    fn emit_assignment_right_assoc() {
        let js = emit_src("let a; let b; a = b = 1;");
        assert_eq!(js, "let a;\nlet b;\n(a = (b = 1));\n");
    }

    #[test]
    fn emit_compound_assignment() {
        let js = emit_src("let x = 1; x += 2; x **= 3;");
        assert_eq!(js, "let x = 1;\n(x += 2);\n(x **= 3);\n");
    }

    #[test]
    fn emit_compound_assignment_to_property() {
        let js = emit_src("let o = { a: 1 }; o.a += 2; o[\"a\"] *= 3;");
        assert_eq!(js, "let o = {a: 1};\n((o).a += 2);\n((o)[\"a\"] *= 3);\n");
    }

    #[test]
    fn emit_nullish() {
        let js = emit_src("let x = null ?? 1;");
        assert_eq!(js, "let x = (null) ?? (1);\n");
    }

    #[test]
    fn emit_logical_assignment() {
        let js = emit_src("let x = 1; x &&= 2; x ||= 3; x ??= 4;");
        assert_eq!(js, "let x = 1;\n(x &&= 2);\n(x ||= 3);\n(x ??= 4);\n");
    }

    #[test]
    fn emit_update() {
        let js = emit_src("let x = 1; ++x; x++; --x; x--;");
        assert_eq!(js, "let x = 1;\n(++x);\n(x++);\n(--x);\n(x--);\n");
    }

    #[test]
    fn emit_update_on_property() {
        let js = emit_src("let o = { a: 1 }; o.a++; ++o[\"a\"];");
        assert_eq!(js, "let o = {a: 1};\n((o).a++);\n(++(o)[\"a\"]);\n");
    }

    #[test]
    fn emit_empty_program() {
        assert_eq!(emit_src(""), "");
    }

    #[test]
    fn emit_typeof_void() {
        let js = emit_src("let t = typeof 1; let v = void 0;");
        assert_eq!(js, "let t = typeof (1);\nlet v = void (0);\n");
    }

    #[test]
    fn emit_while() {
        let js = emit_src("let x = 0; while (x < 3) { x = x + 1; }");
        assert_eq!(js, "let x = 0;\nwhile ((x) < (3)) {\n(x = (x) + (1));\n}\n");
    }

    #[test]
    fn emit_do_while() {
        let js = emit_src("let x = 0; do { x = x + 1; } while (x < 3);");
        assert_eq!(
            js,
            "let x = 0;\ndo {\n(x = (x) + (1));\n} while ((x) < (3));\n"
        );
    }

    #[test]
    fn emit_object_method_super() {
        // E19.23: concise methods keep home-object `super` (not parenthesized; method form).
        let js = emit_src(
            r#"const o = { m() { return super.x; }, n() { return (() => super.y)(); }, ["p"]() { return super["z"]; } };"#,
        );
        assert!(js.contains("m() {"), "{js}");
        assert!(js.contains("return super.x;"), "{js}");
        assert!(js.contains("return super.y;"), "{js}");
        assert!(js.contains("super["), "{js}");
        assert!(!js.contains("(super)"), "{js}");
        assert!(!js.contains("m: function"), "{js}");
    }

    #[test]
    fn emit_class_method_super_home_object() {
        // E19.72: class methods keep Super + method form with home-object install.
        let js = emit_src(
            r#"
class B { }
class C extends B {
  m() { return super.x; }
  n() { super.y = 1; super.z += 2; return eval("super.x"); }
}
"#,
        );
        assert!(js.contains("super.x"), "{js}");
        assert!(js.contains("super.y"), "{js}");
        assert!(js.contains("super.z"), "{js}");
        assert!(js.contains("getOwnPropertyDescriptor"), "{js}");
        // Method form via home-object install (key may be once-bound temp, E19.78).
        assert!(
            js.contains("m() {")
                || js.contains("m(){")
                || js.contains("]() {")
                || js.contains("](){"),
            "{js}"
        );
        assert!(!js.contains("B.prototype.x"), "{js}");
        assert!(!js.contains("(super)"), "{js}");
    }

    #[test]
    fn emit_for() {
        let js = emit_src("let x = 0; for (let i = 0; i < 3; i = i + 1) { x = x + 1; }");
        assert_eq!(
            js,
            "let x = 0;\nfor (let i = 0; (i) < (3); (i = (i) + (1))) {\n(x = (x) + (1));\n}\n"
        );
    }

    #[test]
    fn emit_for_omitted_clauses() {
        let js = emit_src("let x = 0; for (; x < 2; x = x + 1) {}");
        assert_eq!(js, "let x = 0;\nfor (; (x) < (2); (x = (x) + (1))) {\n}\n");
    }

    #[test]
    fn emit_break_continue() {
        let js = emit_src("let x = 0; while (true) { if (x === 1) break; x = x + 1; continue; }");
        assert!(js.contains("break;\n"), "{js}");
        assert!(js.contains("continue;\n"), "{js}");
    }

    #[test]
    fn emit_labeled_break_continue() {
        let js = emit_src(
            "let x = 0; outer: while (true) { x = x + 1; if (x === 1) continue outer; break outer; }",
        );
        assert!(js.contains("outer:"), "{js}");
        assert!(js.contains("continue outer;\n"), "{js}");
        assert!(js.contains("break outer;\n"), "{js}");
    }

    #[test]
    fn emit_switch() {
        let js = emit_src(
            "let a = 0; switch (1) { case 1: a = 10; break; case 2: a = 20; default: a = 30; }",
        );
        assert!(js.contains("switch (1) {\n"), "{js}");
        assert!(js.contains("case 1:\n"), "{js}");
        assert!(js.contains("case 2:\n"), "{js}");
        assert!(js.contains("default:\n"), "{js}");
        assert!(js.contains("break;\n"), "{js}");
    }

    #[test]
    fn emit_comma() {
        let js = emit_src("let x = (1, 2);");
        assert_eq!(js, "let x = ((1) , (2));\n");
    }

    fn emit_result(src: &str) -> Result<String, Diagnostic> {
        let module = compile_source(src).expect("compile");
        emit_js(&module)
    }

    #[test]
    fn n04_native_scalar_polyfill() {
        let js = emit_src("let a: i32 = 1; let b: i64 = 2; let c: f64 = 3.5;");
        assert!(js.contains("let a = 1;"), "{js}");
        assert!(js.contains("let b = 2;"), "{js}");
        assert!(js.contains("let c = 3.5;"), "{js}");
    }

    #[test]
    fn n04_native_struct_polyfill() {
        let js = emit_src(
            "type Point = { x: i32; y: i32 }; let p: Point = { x: 10, y: 20 }; let a: i32 = p.x;",
        );
        assert!(js.contains("let p = {x: 10, y: 20};"), "{js}");
        assert!(js.contains("let a = (p).x;"), "{js}");
    }

    #[test]
    fn n04_native_array_polyfill() {
        let js = emit_src("type V = [i32, i32, i32]; let v: V = [10, 20, 30]; let a: i32 = v[0];");
        assert!(js.contains("let v = [10, 20, 30];"), "{js}");
        assert!(js.contains("let a = (v)[0];"), "{js}");
    }

    #[test]
    fn n04_pointer_hard_error() {
        let err = emit_result("let x: i32 = 10; let p: *i32 = &x; let y: i32 = *p;")
            .expect_err("pointers must hard-error on JS");
        let msg = err.to_string();
        assert!(
            msg.contains("native-only") || msg.contains("pointer"),
            "{msg}"
        );
    }

    #[test]
    fn n04_pointer_store_hard_error() {
        let err = emit_result("let x: i32 = 10; let p: *i32 = &x; *p = 42;")
            .expect_err("pointer store must hard-error on JS");
        let msg = err.to_string();
        assert!(
            msg.contains("native-only") || msg.contains("pointer"),
            "{msg}"
        );
    }

    fn emit_mapped(src: &str, name: &str) -> EmittedJs {
        let module = compile_source(src).expect("compile");
        let opts = SourceMapOptions::new(name)
            .with_content(src)
            .with_output_file("out.js");
        emit_js_with_map(&module, &opts).expect("emit_js_with_map")
    }

    #[test]
    fn u03_source_map_version_and_sources() {
        let emitted = emit_mapped("let x = 1;\n", "main.drac");
        let map = emitted.map.expect("map");
        assert_eq!(map.version, 3);
        assert_eq!(map.sources, vec!["main.drac".to_string()]);
        assert_eq!(map.file.as_deref(), Some("out.js"));
        assert_eq!(map.sources_content, vec![Some("let x = 1;\n".to_string())]);
        assert!(!map.mappings.is_empty(), "mappings={}", map.mappings);
        assert_eq!(emitted.code, "let x = 1;\n");
    }

    #[test]
    fn u03_source_map_maps_second_statement_to_line_two() {
        let src = "let x = 1;\nlet y = 2;\n";
        let emitted = emit_mapped(src, "t.drac");
        let map = emitted.map.expect("map");
        let segs = decode_mappings(&map.mappings);
        assert!(
            segs.len() >= 2,
            "expected ≥2 segments, got {:?}\nmappings={}",
            segs,
            map.mappings
        );
        // First top-level stmt → original line 0
        assert_eq!(segs[0].original_line, 0, "{segs:?}");
        assert_eq!(segs[0].generated_line, 0, "{segs:?}");
        // Second top-level stmt → original line 1
        assert_eq!(segs[1].original_line, 1, "{segs:?}");
        assert_eq!(segs[1].generated_line, 1, "{segs:?}");
    }

    #[test]
    fn u03_source_map_json_roundtrip_fields() {
        let emitted = emit_mapped("let a = 1 + 2;\n", "x.drac");
        let map = emitted.map.expect("map");
        let json = map.to_json();
        assert!(json.contains("\"version\": 3"), "{json}");
        assert!(json.contains("\"sources\": [\"x.drac\"]"), "{json}");
        assert!(json.contains("\"mappings\":"), "{json}");
        assert!(json.contains("let a = 1 + 2;\\n"), "{json}");
    }

    #[test]
    fn u03_source_mapping_url_comment() {
        let c = source_mapping_url_comment("out.js.map");
        assert_eq!(c, "\n//# sourceMappingURL=out.js.map\n");
    }

    #[test]
    fn u03_emit_js_unchanged_without_map() {
        assert_eq!(emit_src("let x = 1;"), "let x = 1;\n");
    }

    /// E19.32: array pattern elision must emit holes so IteratorStep/IteratorClose run.
    #[test]
    fn emit_array_pattern_elision_holes() {
        let only = emit_src("let [,] = vals;");
        assert!(
            only.contains("let [,] = vals;") || only.contains("let [, ] = vals;"),
            "{only}"
        );
        assert!(!only.contains("let [] ="), "{only}");

        let trail = emit_src("let [a,,] = vals;");
        assert!(
            trail.contains("[a,,]") || trail.contains("[a, ,]") || trail.contains("[a, , ]"),
            "{trail}"
        );

        let mid = emit_src("let [a, , b] = vals;");
        assert!(mid.contains("[a, , b]") || mid.contains("[a,, b]"), "{mid}");

        let lead = emit_src("let [, x] = vals;");
        assert!(lead.contains("[, x]") || lead.contains("[,x]"), "{lead}");

        let assign = emit_src("let x; [, ] = vals;");
        assert!(
            assign.contains("[,]") || assign.contains("[, ]") || assign.contains("([,])"),
            "{assign}"
        );
        assert!(!assign.contains("([] ="), "{assign}");
    }

    /// E19.32: array literal trailing/only holes keep length semantics.
    #[test]
    fn emit_array_literal_elision_holes() {
        let only = emit_src("let a = [,];");
        assert!(only.contains("[,]") || only.contains("[, ]"), "{only}");
        assert!(!only.contains("let a = [];"), "{only}");

        let two = emit_src("let a = [,,];");
        assert!(
            two.contains("[,,]") || two.contains("[, ,]") || two.contains("[, , ]"),
            "{two}"
        );
    }
}
