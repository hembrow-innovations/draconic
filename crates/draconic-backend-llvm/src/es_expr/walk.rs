use std::collections::HashSet;

use draconic_ast::{AssignOp, BinaryOp, BindingKind};
use draconic_diagnostics::Diagnostic;
use draconic_ir::{
    Arg, ArrayElement, ArrayPatternEl, AssignTarget, Expr, Module, ObjectPatternEl, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt, UpdateTarget,
};

use super::{classify_body, diag, emit_es_expr_with};

pub(super) fn emit_walk(module: &Module) -> Result<String, Diagnostic> {
    let seen = Seen::walk(module);
    seen.emit(module)
}

#[derive(Default)]
struct Seen {
    host: HashSet<&'static str>,
    idents: HashSet<String>,
    has_script: bool,
    has_async_fn: bool,
    has_generator: bool,
    has_function: bool,
    has_try: bool,
    has_with: bool,
    has_array: bool,
    has_tagged: bool,
    has_new_target: bool,
    has_optional: bool,
    has_nullish: bool,
    has_spread_arg: bool,
    has_instanceof: bool,
    has_obj_pattern: bool,
    has_arr_pattern: bool,
    has_var: bool,
    has_for_in_of: bool,
    has_for_await: bool,
    has_regexp: bool,
    has_date_now: bool,
    has_value_of: bool,
}

impl Seen {
    fn walk(module: &Module) -> Self {
        let mut seen = Self::default();
        for stmt in &module.body {
            walk_stmt(stmt, module, &mut seen);
        }
        seen
    }

    fn ident(&self, name: &str) -> bool {
        self.idents.contains(name)
    }

    fn host_has(&self, names: &[&str]) -> bool {
        names.iter().any(|n| self.host.contains(n))
    }

    fn emit(self, module: &Module) -> Result<String, Diagnostic> {
        if !self.host.is_empty() || self.has_date_now {
            return emit_host(module, &self);
        }
        emit_es(module, &self)
    }
}

fn claimed(opt: Option<Result<String, Diagnostic>>) -> Result<String, Diagnostic> {
    opt.ok_or_else(|| diag("unsupported IR node"))?
}

fn emit_host(module: &Module, seen: &Seen) -> Result<String, Diagnostic> {
    if seen.host_has(&["processWaitAsync"]) {
        return crate::host_process_async::emit_host_process_async(module);
    }
    if seen.host_has(&[
        "processRun",
        "processWait",
        "processSpawn",
        "processClose",
        "processStdinWrite",
        "processStdout",
        "processStderr",
        "processKill",
    ]) {
        return crate::host_subprocess::emit_host_subprocess(module);
    }
    if seen.host_has(&["onSignal", "raiseSignal", "ignoreSignal", "restoreSignal"]) {
        return crate::host_signals::emit_host_signals(module);
    }
    if seen.host_has(&["stdinReadLine", "stdinReadBytes"]) {
        return crate::host_stdio::emit_host_stdio(module);
    }
    if seen.host_has(&[
        "pathJoin",
        "pathNormalize",
        "pathDirname",
        "pathBasename",
        "pathExtname",
        "pathIsAbsolute",
        "pathResolve",
    ]) {
        return crate::host_path::emit_host_path(module);
    }
    if seen.host_has(&[
        "tcpAcceptAsync",
        "tcpConnectAsync",
        "tcpReadAsync",
        "tcpWriteAsync",
    ]) {
        return crate::host_tcp_async::emit_host_tcp_async(module);
    }
    if seen.host_has(&[
        "udpBind",
        "udpLocalPort",
        "udpSendTo",
        "udpRecvFrom",
        "closeUdp",
    ]) {
        return crate::host_udp::emit_host_udp(module);
    }
    if seen.host_has(&["dnsLookup"]) {
        return crate::host_dns::emit_host_dns(module);
    }
    if seen.host_has(&[
        "wsClientHandshakeRequest",
        "wsClientCheckAccept",
        "wsEncodeTextClient",
    ]) {
        return crate::host_ws_e2e::emit_host_ws_e2e(module);
    }
    if seen.host_has(&[
        "http2ClientPreface",
        "http2ServerPreface",
        "http2SettingsAck",
        "http2EncodeRequest",
        "http2EncodeResponse",
        "http2ParseRequest",
        "http2ParseResponse",
        "http2ClientOpen",
        "http2ServerReply",
    ]) {
        return crate::host_http2::emit_host_http2(module);
    }
    if seen.host_has(&["httpServeStatic"])
        || (seen.host_has(&[
            "httpParseRequest",
            "httpWriteResponse",
            "httpWriteRequest",
            "httpParseResponse",
            "wsHandshakeResponse",
        ]) && seen.host_has(&[
            "tcpListen",
            "tcpAccept",
            "tcpConnect",
            "tcpRead",
            "tcpWrite",
            "tlsClientWrap",
            "tlsServerWrap",
        ]))
    {
        return crate::host_http_server::emit_host_http_server(module);
    }
    if seen.host_has(&[
        "wsEncodeText",
        "wsEncodeBinary",
        "wsEncodeClose",
        "wsEncodePing",
        "wsEncodePong",
        "wsDecodeFrame",
    ]) {
        return crate::host_ws::emit_host_ws(module);
    }
    if seen.host_has(&[
        "httpParseRequest",
        "httpRequestHeader",
        "httpWriteResponse",
        "httpWriteRequest",
        "httpParseResponse",
        "httpResponseHeader",
        "wsHandshakeResponse",
    ]) {
        return crate::host_http::emit_host_http(module);
    }
    if seen.host_has(&[
        "tcpListen",
        "tcpLocalPort",
        "closeTcp",
        "tcpAccept",
        "tcpConnect",
        "tcpPeerAddress",
        "tcpPeerPort",
        "tcpRead",
        "tcpWrite",
        "tcpShutdown",
        "tlsClientWrap",
        "tlsServerWrap",
        "tlsRead",
        "tlsWrite",
        "closeTls",
    ]) {
        return crate::host_tcp::emit_host_tcp(module);
    }
    if seen.host_has(&["setTimeout", "clearTimeout", "setInterval", "clearInterval"]) {
        return crate::host_timers::emit_host_timers(module);
    }
    if seen.host_has(&["nowMs", "monotonicMs"]) || seen.has_date_now {
        return crate::host_time::emit_host_time(module);
    }
    if seen.host_has(&[
        "makeSharedMemory",
        "sharedLoad",
        "sharedStore",
        "sharedAdd",
        "sharedCompareExchange",
        "sharedWait",
        "sharedNotify",
    ]) {
        return crate::host_atomics::emit_host_atomics(module);
    }
    if seen.host_has(&["spawnWorker"])
        && seen.host_has(&["makeChannel", "channelSend", "channelRecv"])
    {
        return crate::host_worker_channels::emit_host_worker_channels(module);
    }
    if seen.host_has(&["makeOnce", "onceRun"]) {
        return crate::host_once::emit_host_once(module);
    }
    if seen.host_has(&[
        "makeCancelToken",
        "cancelTokenAbort",
        "cancelTokenAborted",
        "cancelTokenLink",
        "withTimeout",
        "clearWithTimeout",
    ]) {
        return crate::host_cancel::emit_host_cancel(module);
    }
    if seen.host_has(&[
        "spawnWorker",
        "joinWorker",
        "terminateWorker",
        "workerOsThread",
    ]) {
        return crate::host_workers::emit_host_workers(module);
    }
    if seen.host_has(&["makeChannel", "channelSend", "channelRecv"]) {
        return crate::host_channels::emit_host_channels(module);
    }
    if seen.host_has(&[
        "processArgs",
        "envGet",
        "envSet",
        "envDelete",
        "exit",
        "exitCode",
        "setExitCode",
        "pid",
        "ppid",
    ]) {
        return crate::host_process::emit_host_process(module);
    }
    if seen.host_has(&[
        "cwd", "chdir", "hostname", "osType", "osArch", "tempDir", "homeDir",
    ]) {
        return crate::host_os::emit_host_os(module);
    }
    if seen.host_has(&["stdoutWrite", "stderrWrite"])
        && seen
            .host
            .iter()
            .all(|n| matches!(*n, "stdoutWrite" | "stderrWrite"))
    {
        return crate::host_stdio::emit_host_stdio(module);
    }
    if seen.host_has(&[
        "readFileText",
        "readFileBytes",
        "writeFileText",
        "writeFileBytes",
        "appendFileText",
        "appendFileBytes",
        "exists",
        "stat",
        "mkdir",
        "mkdirAll",
        "readdir",
        "rmdir",
        "removeFile",
        "renameFile",
        "copyFile",
        "openFile",
        "fileRead",
        "fileWrite",
        "fileSeek",
        "closeFile",
    ]) {
        if seen.has_script {
            return crate::host_docs::emit_host_docs(module);
        }
        return crate::host_fs::emit_host_fs(module);
    }
    Err(diag("unsupported IR node"))
}

fn emit_es(module: &Module, seen: &Seen) -> Result<String, Diagnostic> {
    match es_kind(module, seen) {
        EsKind::Promise => crate::es_promise::emit_es_promise(module),
        EsKind::Eval => crate::es_eval::emit_es_eval(module),
        EsKind::PrivateIn => crate::es_private_in::emit_es_private_in(module),
        EsKind::Proxies => crate::es_proxies::emit_es_proxies(module),
        EsKind::Testing => crate::es_testing::emit_es_testing(module),
        EsKind::Logging => crate::es_logging::emit_es_logging(module),
        EsKind::Mime => crate::es_mime::emit_es_mime(module),
        EsKind::Collections => crate::es_collections::emit_es_collections(module),
        EsKind::Encoding => crate::es_encoding::emit_es_encoding(module),
        EsKind::NewTarget => crate::es_new_target::emit_es_new_target(module),
        EsKind::PrivateAccessors => crate::es_private_accessors::emit_es_private_accessors(module),
        EsKind::Instanceof => crate::es_instanceof::emit_es_instanceof(module),
        EsKind::Generators => crate::es_generators::emit_es_generators(module),
        EsKind::Modules => crate::es_modules::emit_es_modules(module),
        EsKind::Exceptions => crate::es_exceptions::emit_es_exceptions(module),
        EsKind::Legacy => crate::es_legacy::emit_es_legacy(module),
        EsKind::OptionalChain => crate::es_optional_chain::emit_es_optional_chain(module),
        EsKind::StaticBlocks => crate::es_static_blocks::emit_es_static_blocks(module),
        EsKind::Nullish => crate::es_nullish::emit_es_nullish(module),
        EsKind::ToPrimitive => crate::es_to_primitive::emit_es_to_primitive(module),
        EsKind::Coercion => crate::es_coercion::emit_es_coercion(module),
        EsKind::Values => crate::es_values::emit_es_values(module),
        EsKind::CallSpread => crate::es_call_spread::emit_es_call_spread(module),
        EsKind::TaggedTemplate => crate::es_tagged_template::emit_es_tagged_template(module),
        EsKind::ParamDstr => crate::es_param_dstr::emit_es_param_dstr(module),
        EsKind::VarFor => crate::es_var_for::emit_es_var_for(module),
        EsKind::ClassExprName => crate::es_class_expr_name::emit_es_class_expr_name(module),
        EsKind::StaticPrivateMethods => {
            crate::es_static_private_methods::emit_es_static_private_methods(module)
        }
        EsKind::ObjectDestructure => {
            crate::es_object_destructure::emit_es_object_destructure(module)
        }
        EsKind::DestructureDefaults => {
            crate::es_destructure_defaults::emit_es_destructure_defaults(module)
        }
        EsKind::Builtins => crate::es_builtins::emit_es_builtins(module),
        EsKind::Objects => crate::es_objects::emit_es_objects(module),
        EsKind::Arrays => crate::es_arrays::emit_es_arrays(module),
        EsKind::Classes => claimed(crate::es_classes::walk_es_classes(module)),
        EsKind::Functions => claimed(crate::es_functions::walk_es_functions(module)),
        EsKind::Expr => {
            let info = classify_body(module).ok_or_else(|| diag("unsupported IR node"))?;
            emit_es_expr_with(module, &info)
        }
    }
}

enum EsKind {
    Promise,
    Eval,
    PrivateIn,
    Proxies,
    Testing,
    Logging,
    Mime,
    Collections,
    Encoding,
    NewTarget,
    PrivateAccessors,
    Instanceof,
    Generators,
    Modules,
    Exceptions,
    Legacy,
    OptionalChain,
    StaticBlocks,
    Nullish,
    ToPrimitive,
    Coercion,
    Values,
    CallSpread,
    TaggedTemplate,
    ParamDstr,
    VarFor,
    ClassExprName,
    StaticPrivateMethods,
    ObjectDestructure,
    DestructureDefaults,
    Builtins,
    Objects,
    Arrays,
    Classes,
    Functions,
    Expr,
}

fn es_kind(module: &Module, seen: &Seen) -> EsKind {
    if (seen.ident("Promise") || seen.has_async_fn)
        && crate::es_promise::is_es_promise_module(module)
    {
        return EsKind::Promise;
    }
    if (seen.ident("eval") || seen.ident("Function")) && crate::es_eval::is_es_eval_module(module) {
        return EsKind::Eval;
    }
    if crate::es_private_in::is_es_private_in_module(module) {
        return EsKind::PrivateIn;
    }
    if (seen.ident("Proxy") || seen.ident("Reflect"))
        && crate::es_proxies::is_es_proxies_module(module)
    {
        return EsKind::Proxies;
    }
    if (seen.ident("describe") || seen.ident("it") || seen.ident("expect"))
        && crate::es_testing::is_es_testing_module(module)
    {
        return EsKind::Testing;
    }
    if seen.ident("createLogger") && crate::es_logging::is_es_logging_module(module) {
        return EsKind::Logging;
    }
    if (seen.ident("parseMultipart") || seen.ident("serializeMultipart"))
        && crate::es_mime::is_es_mime_module(module)
    {
        return EsKind::Mime;
    }
    if (seen.ident("groupBy") || seen.ident("chunk") || seen.ident("Deque"))
        && crate::es_collections::is_es_collections_module(module)
    {
        return EsKind::Collections;
    }
    if (seen.ident("TextEncoder")
        || seen.ident("TextDecoder")
        || seen.ident("toBase64")
        || seen.ident("fromBase64")
        || seen.ident("toHex")
        || seen.ident("fromHex")
        || seen.ident("sha256")
        || seen.ident("hmacSha256")
        || seen.ident("aeadEncrypt")
        || seen.ident("aeadDecrypt")
        || seen.ident("randomBytes"))
        && crate::es_encoding::is_es_encoding_module(module)
    {
        return EsKind::Encoding;
    }
    if seen.has_new_target && crate::es_new_target::is_es_new_target_module(module) {
        return EsKind::NewTarget;
    }
    if crate::es_private_accessors::is_es_private_accessors_module(module) {
        return EsKind::PrivateAccessors;
    }
    if seen.has_instanceof && crate::es_instanceof::is_es_instanceof_module(module) {
        return EsKind::Instanceof;
    }
    if (seen.has_generator || seen.has_for_await || seen.has_async_fn)
        && crate::es_generators::is_es_generators_module(module)
    {
        return EsKind::Generators;
    }
    if seen.idents.iter().any(|n| n.starts_with("__m"))
        && crate::es_modules::is_es_modules_module(module)
    {
        return EsKind::Modules;
    }
    if seen.has_try && crate::es_exceptions::is_es_exceptions_module(module) {
        return EsKind::Exceptions;
    }
    if seen.has_with && crate::es_legacy::is_es_legacy_module(module) {
        return EsKind::Legacy;
    }
    if seen.has_optional && crate::es_optional_chain::is_es_optional_chain_module(module) {
        return EsKind::OptionalChain;
    }
    if crate::es_static_blocks::is_es_static_blocks_module(module) {
        return EsKind::StaticBlocks;
    }
    if seen.has_nullish && crate::es_nullish::is_es_nullish_module(module) {
        return EsKind::Nullish;
    }
    if seen.has_value_of && crate::es_to_primitive::is_es_to_primitive_module(module) {
        return EsKind::ToPrimitive;
    }
    if crate::es_coercion::is_es_coercion_module(module) {
        return EsKind::Coercion;
    }
    if seen.ident("Symbol") && crate::es_values::is_es_values_module(module) {
        return EsKind::Values;
    }
    if seen.has_spread_arg && crate::es_call_spread::is_es_call_spread_module(module) {
        return EsKind::CallSpread;
    }
    if seen.has_tagged && crate::es_tagged_template::is_es_tagged_template_module(module) {
        return EsKind::TaggedTemplate;
    }
    if crate::es_param_dstr::is_es_param_dstr_module(module) {
        return EsKind::ParamDstr;
    }
    if seen.has_var && seen.has_for_in_of && crate::es_var_for::is_es_var_for_module(module) {
        return EsKind::VarFor;
    }
    if crate::es_class_expr_name::is_es_class_expr_name_module(module) {
        return EsKind::ClassExprName;
    }
    if crate::es_static_private_methods::is_es_static_private_methods_module(module) {
        return EsKind::StaticPrivateMethods;
    }
    if seen.has_obj_pattern && crate::es_object_destructure::is_es_object_destructure_module(module)
    {
        return EsKind::ObjectDestructure;
    }
    if crate::es_destructure_defaults::is_es_destructure_defaults_module(module) {
        return EsKind::DestructureDefaults;
    }
    if (is_builtins_ident(seen) || seen.has_regexp)
        && crate::es_builtins::is_es_builtins_module(module)
    {
        return EsKind::Builtins;
    }
    if crate::es_objects::is_es_objects_module(module) {
        return EsKind::Objects;
    }
    if crate::es_classes::walk_es_classes_applies(module) {
        return EsKind::Classes;
    }
    if (seen.has_array || seen.has_arr_pattern) && crate::es_arrays::is_es_arrays_module(module) {
        return EsKind::Arrays;
    }
    if classify_body(module).is_none()
        && (seen.ident("Object") || seen.ident("String"))
        && crate::es_builtins::is_es_builtins_module(module)
    {
        return EsKind::Builtins;
    }
    if seen.has_function {
        return EsKind::Functions;
    }
    EsKind::Expr
}

fn is_builtins_ident(seen: &Seen) -> bool {
    seen.ident("JSON")
        || seen.ident("Date")
        || seen.ident("RegExp")
        || seen.ident("Map")
        || seen.ident("Set")
        || seen.ident("WeakMap")
        || seen.ident("WeakSet")
        || seen.ident("Error")
        || seen.ident("TypeError")
        || seen.ident("parseInt")
        || seen.ident("parseFloat")
        || seen.ident("encodeURI")
        || seen.ident("encodeURIComponent")
        || seen.ident("decodeURI")
        || seen.ident("decodeURIComponent")
        || seen.ident("escape")
        || seen.ident("unescape")
        || seen.ident("ArrayBuffer")
        || seen.ident("DataView")
        || seen.ident("Uint8Array")
        || seen.ident("Int32Array")
        || seen.ident("Float64Array")
        || seen.ident("isNaN")
        || seen.ident("isFinite")
}

fn walk_stmt(stmt: &Stmt, module: &Module, seen: &mut Seen) {
    match stmt {
        Stmt::Declare { init, kind, .. } => {
            if *kind == BindingKind::Var {
                seen.has_var = true;
            }
            if let Some(expr) = init {
                walk_expr(expr, module, seen);
            }
        }
        Stmt::DeclareArrayPattern {
            init,
            kind,
            elements,
            ..
        } => {
            if *kind == BindingKind::Var {
                seen.has_var = true;
            }
            seen.has_arr_pattern = true;
            walk_array_pattern(elements, module, seen);
            if let Some(expr) = init {
                walk_expr(expr, module, seen);
            }
        }
        Stmt::DeclareObjectPattern {
            init,
            kind,
            properties,
            ..
        } => {
            if *kind == BindingKind::Var {
                seen.has_var = true;
            }
            seen.has_obj_pattern = true;
            walk_object_pattern(properties, module, seen);
            if let Some(expr) = init {
                walk_expr(expr, module, seen);
            }
        }
        Stmt::AssignLeft { target } => walk_assign_target(target, module, seen),
        Stmt::Expr { expr } => walk_expr(expr, module, seen),
        Stmt::Block { body } => {
            seen.has_script = true;
            for s in body {
                walk_stmt(s, module, seen);
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            seen.has_script = true;
            walk_expr(test, module, seen);
            walk_stmt(consequent, module, seen);
            if let Some(alt) = alternate {
                walk_stmt(alt, module, seen);
            }
        }
        Stmt::While { test, body } | Stmt::DoWhile { body, test } => {
            seen.has_script = true;
            walk_expr(test, module, seen);
            walk_stmt(body, module, seen);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            seen.has_script = true;
            if let Some(i) = init {
                walk_stmt(i, module, seen);
            }
            if let Some(t) = test {
                walk_expr(t, module, seen);
            }
            if let Some(u) = update {
                walk_expr(u, module, seen);
            }
            walk_stmt(body, module, seen);
        }
        Stmt::ForIn { left, right, body } => {
            seen.has_script = true;
            seen.has_for_in_of = true;
            walk_stmt(left, module, seen);
            walk_expr(right, module, seen);
            walk_stmt(body, module, seen);
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
        } => {
            seen.has_script = true;
            seen.has_for_in_of = true;
            if *is_await {
                seen.has_for_await = true;
            }
            walk_stmt(left, module, seen);
            walk_expr(right, module, seen);
            walk_stmt(body, module, seen);
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
        Stmt::Labeled { body, .. } => walk_stmt(body, module, seen),
        Stmt::Switch {
            discriminant,
            cases,
        } => {
            walk_expr(discriminant, module, seen);
            for c in cases {
                if let Some(t) = &c.test {
                    walk_expr(t, module, seen);
                }
                for s in &c.body {
                    walk_stmt(s, module, seen);
                }
            }
        }
        Stmt::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            seen.has_function = true;
            if *is_async {
                seen.has_async_fn = true;
            }
            if *is_generator {
                seen.has_generator = true;
            }
            walk_params(params, module, seen);
            for s in body {
                walk_stmt(s, module, seen);
            }
        }
        Stmt::ExternFunction { .. } => {}
        Stmt::Return { value } => {
            if let Some(v) = value {
                walk_expr(v, module, seen);
            }
        }
        Stmt::Throw { value } => {
            seen.has_try = true;
            walk_expr(value, module, seen);
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            seen.has_try = true;
            for s in block {
                walk_stmt(s, module, seen);
            }
            if let Some(p) = handler_param {
                walk_pattern(p, module, seen);
            }
            if let Some(h) = handler {
                for s in h {
                    walk_stmt(s, module, seen);
                }
            }
            if let Some(f) = finalizer {
                for s in f {
                    walk_stmt(s, module, seen);
                }
            }
        }
        Stmt::With { object, body } => {
            seen.has_with = true;
            walk_expr(object, module, seen);
            for s in body {
                walk_stmt(s, module, seen);
            }
        }
    }
}

fn walk_expr(expr: &Expr, module: &Module, seen: &mut Seen) {
    if let Some(entry) = crate::host_catalog::catalog_callee_in_module(expr, module) {
        seen.host.insert(entry.name);
    }
    match expr {
        Expr::Local { id, .. } => {
            if let Some(local) = module.locals.iter().find(|l| l.id == *id) {
                seen.idents.insert(local.name.clone());
            }
        }
        Expr::IdentName { name, .. } => {
            seen.idents.insert(name.clone());
        }
        Expr::Number { .. }
        | Expr::BigInt { .. }
        | Expr::String { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::Super { .. }
        | Expr::ImportMeta { .. } => {}
        Expr::RegExp { .. } => seen.has_regexp = true,
        Expr::NewTarget { .. } => seen.has_new_target = true,
        Expr::Template { expressions, .. } => {
            for e in expressions {
                walk_expr(e, module, seen);
            }
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            seen.has_tagged = true;
            walk_expr(tag, module, seen);
            for e in expressions {
                walk_expr(e, module, seen);
            }
        }
        Expr::ImportCall {
            source, options, ..
        } => {
            walk_expr(source, module, seen);
            if let Some(o) = options {
                walk_expr(o, module, seen);
            }
        }
        Expr::Unary { arg, .. } => walk_expr(arg, module, seen),
        Expr::Binary {
            left, op, right, ..
        } => {
            if matches!(*op, BinaryOp::Nullish) {
                seen.has_nullish = true;
            }
            if matches!(*op, BinaryOp::InstanceOf) {
                seen.has_instanceof = true;
            }
            walk_expr(left, module, seen);
            walk_expr(right, module, seen);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            walk_expr(test, module, seen);
            walk_expr(consequent, module, seen);
            walk_expr(alternate, module, seen);
        }
        Expr::Assign {
            target, op, value, ..
        } => {
            if matches!(
                *op,
                AssignOp::NullishEq | AssignOp::AndAndEq | AssignOp::OrOrEq
            ) {
                seen.has_nullish = true;
            }
            walk_assign_target(target, module, seen);
            walk_expr(value, module, seen);
        }
        Expr::Update { target, .. } => walk_update_target(target, module, seen),
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                seen.has_optional = true;
            }
            note_date_now(callee, args, module, seen);
            walk_expr(callee, module, seen);
            walk_args(args, module, seen);
        }
        Expr::New { callee, args, .. } => {
            walk_expr(callee, module, seen);
            walk_args(args, module, seen);
        }
        Expr::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            seen.has_function = true;
            if *is_async {
                seen.has_async_fn = true;
            }
            if *is_generator {
                seen.has_generator = true;
            }
            walk_params(params, module, seen);
            for s in body {
                walk_stmt(s, module, seen);
            }
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                walk_object_prop(p, module, seen);
            }
        }
        Expr::Array { elements, .. } => {
            seen.has_array = true;
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        walk_expr(e, module, seen);
                    }
                    ArrayElement::Elision => {}
                }
            }
        }
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            if *optional {
                seen.has_optional = true;
            }
            if let Expr::String { value, .. } = property.as_ref() {
                let key = value.to_string_lossy();
                if key == "valueOf" || key == "toString" {
                    seen.has_value_of = true;
                }
            }
            walk_expr(object, module, seen);
            walk_expr(property, module, seen);
        }
    }
}

fn note_date_now(callee: &Expr, args: &[Arg], module: &Module, seen: &mut Seen) {
    if !args.is_empty() {
        return;
    }
    let Expr::Member {
        object, property, ..
    } = callee
    else {
        return;
    };
    let Expr::String { value, .. } = property.as_ref() else {
        return;
    };
    if value.to_string_lossy() != "now" {
        return;
    }
    if crate::host_catalog::is_named_callee_in(object, "Date", Some(module))
        || ident_is(object, module, "Date")
    {
        seen.has_date_now = true;
    }
}

fn ident_is(expr: &Expr, module: &Module, want: &str) -> bool {
    match expr {
        Expr::IdentName { name, .. } => name == want,
        Expr::Local { id, .. } => module
            .locals
            .iter()
            .find(|l| l.id == *id)
            .is_some_and(|l| l.name == want),
        _ => false,
    }
}

fn walk_args(args: &[Arg], module: &Module, seen: &mut Seen) {
    for a in args {
        match a {
            Arg::Expr(e) => walk_expr(e, module, seen),
            Arg::Spread(e) => {
                seen.has_spread_arg = true;
                walk_expr(e, module, seen);
            }
        }
    }
}

fn walk_object_prop(prop: &ObjectProp, module: &Module, seen: &mut Seen) {
    match prop {
        ObjectProp::Property { key, value } | ObjectProp::Accessor { key, value, .. } => {
            if let ObjectPropKey::Static(name) = key {
                let n = name.to_string_lossy();
                if n == "valueOf" || n == "toString" {
                    seen.has_value_of = true;
                }
            }
            walk_prop_key(key, module, seen);
            walk_expr(value, module, seen);
        }
        ObjectProp::Spread(e) => walk_expr(e, module, seen),
    }
}

fn walk_prop_key(key: &ObjectPropKey, module: &Module, seen: &mut Seen) {
    if let ObjectPropKey::Computed(e) = key {
        walk_expr(e, module, seen);
    }
}

fn walk_assign_target(target: &AssignTarget, module: &Module, seen: &mut Seen) {
    match target {
        AssignTarget::Local(_) | AssignTarget::Name(_) => {}
        AssignTarget::Member {
            object, property, ..
        } => {
            walk_expr(object, module, seen);
            walk_expr(property, module, seen);
        }
        AssignTarget::Deref(e) => walk_expr(e, module, seen),
        AssignTarget::ArrayPattern { elements } => {
            seen.has_arr_pattern = true;
            walk_array_pattern(elements, module, seen);
        }
        AssignTarget::ObjectPattern { properties } => {
            seen.has_obj_pattern = true;
            walk_object_pattern(properties, module, seen);
        }
    }
}

fn walk_update_target(target: &UpdateTarget, module: &Module, seen: &mut Seen) {
    match target {
        UpdateTarget::Local(_) | UpdateTarget::Name(_) => {}
        UpdateTarget::Member {
            object, property, ..
        } => {
            walk_expr(object, module, seen);
            walk_expr(property, module, seen);
        }
    }
}

fn walk_params(params: &[Param], module: &Module, seen: &mut Seen) {
    for p in params {
        walk_pattern(&p.pattern, module, seen);
        if let Some(d) = &p.default {
            walk_expr(d, module, seen);
        }
    }
}

fn walk_pattern(pat: &Pattern, module: &Module, seen: &mut Seen) {
    match pat {
        Pattern::Local(_) | Pattern::Name(_) => {}
        Pattern::Member {
            object, property, ..
        } => {
            walk_expr(object, module, seen);
            walk_expr(property, module, seen);
        }
        Pattern::Array(els) => {
            seen.has_arr_pattern = true;
            walk_array_pattern(els, module, seen);
        }
        Pattern::Object(els) => {
            seen.has_obj_pattern = true;
            walk_object_pattern(els, module, seen);
        }
    }
}

fn walk_array_pattern(els: &[ArrayPatternEl], module: &Module, seen: &mut Seen) {
    for el in els {
        match el {
            ArrayPatternEl::Elision => {}
            ArrayPatternEl::Pattern { binding, default } => {
                walk_pattern(binding, module, seen);
                if let Some(d) = default {
                    walk_expr(d, module, seen);
                }
            }
            ArrayPatternEl::Rest(p) => walk_pattern(p, module, seen),
        }
    }
}

fn walk_object_pattern(els: &[ObjectPatternEl], module: &Module, seen: &mut Seen) {
    for el in els {
        match el {
            ObjectPatternEl::Prop {
                key,
                binding,
                default,
                ..
            } => {
                walk_prop_key(key, module, seen);
                walk_pattern(binding, module, seen);
                if let Some(d) = default {
                    walk_expr(d, module, seen);
                }
            }
            ObjectPatternEl::Rest(p) => walk_pattern(p, module, seen),
        }
    }
}
