//! H07.02: async TCP → Promises (accept/connect/read/write) + cancel on close.
//!
//! Supported subset:
//! - `tcpListen` / `tcpLocalPort` / `closeTcp`
//! - `tcpAcceptAsync` / `tcpConnectAsync` / `tcpReadAsync` / `tcpWriteAsync` → Promise
//! - `p.then(onFulfilled)` / `p.then(onFulfilled, onRejected)`
//! - number locals + assigns; typeof on async APIs
//! - nested async calls / closeTcp inside then reactions
//! - end of main: `job_drain`, print number/string/bool observation locals

use std::collections::{HashMap, HashSet};

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, Local, LocalId, Module, Param, Pattern, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_HANDLE_CLOSE, HOST_TCP_ACCEPT, HOST_TCP_ACCEPT_ASYNC,
    HOST_TCP_ASYNC_DECLARES, HOST_TCP_CONNECT, HOST_TCP_CONNECT_ASYNC, HOST_TCP_LISTEN,
    HOST_TCP_LOCAL_PORT, HOST_TCP_READ_ASYNC, HOST_TCP_WRITE_ASYNC, JOB_DRAIN, PRINT_BOOL,
    PRINT_I64, PRINT_STR, PROMISE_THEN,
};

mod classify;
mod emit;

use classify::try_classify;

pub(crate) fn is_host_tcp_async_module(module: &Module) -> bool {
    match try_classify(module) {
        Ok(info) => info.uses_async,
        Err(_) => false,
    }
}

pub(crate) fn emit_host_tcp_async(module: &Module) -> Result<String, Diagnostic> {
    let info = try_classify(module).map_err(diag)?;
    if !info.uses_async {
        return Err(diag("internal: not a host_tcp_async module"));
    }
    let mut em = Emitter::new(module, info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotKind {
    Number,
    Bool,
    String,
    Promise,
    Handle,
}

struct ModuleInfo {
    uses_async: bool,
    user_locals: Vec<(LocalId, SlotKind)>,
}

fn diag(msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

struct Emitter<'a> {
    module: &'a Module,
    info: ModuleInfo,
    local_names: HashMap<LocalId, String>,
    allocas: HashMap<LocalId, String>,
    slot_kind: HashMap<LocalId, SlotKind>,
    out: String,
    body: String,
    helpers: String,
    str_globals: Vec<(String, String)>,
    tmp: usize,
    next_fn: usize,
    reaction_params: HashMap<LocalId, String>,
    reaction_captures: Vec<LocalId>,
}
