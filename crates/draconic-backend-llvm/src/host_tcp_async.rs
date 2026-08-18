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
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, Local, LocalId, Module, Param, Pattern, Stmt};
use draconic_runtime::abi::{
    llvm_declares, HOST_HANDLE_CLOSE, HOST_TCP_ACCEPT, HOST_TCP_ACCEPT_ASYNC,
    HOST_TCP_ASYNC_DECLARES, HOST_TCP_CONNECT, HOST_TCP_CONNECT_ASYNC, HOST_TCP_LISTEN,
    HOST_TCP_LOCAL_PORT, HOST_TCP_READ_ASYNC, HOST_TCP_WRITE_ASYNC, JOB_DRAIN, PRINT_BOOL,
    PRINT_I64, PRINT_STR, PROMISE_THEN, GC_INIT,
};

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

fn try_classify(module: &Module) -> Result<ModuleInfo, String> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_ids = HashSet::new();
    collect_top_level_decl_ids(&module.body, &mut user_ids);

    let mut slot_of: HashMap<LocalId, SlotKind> = HashMap::new();
    let mut user_locals = Vec::new();
    let mut seen = HashSet::new();
    let mut uses_async = false;

    for stmt in &module.body {
        check_stmt(stmt, &mut uses_async, &mut slot_of)?;
        if let Stmt::Declare { local, init, .. } = stmt {
            if !seen.insert(*local) {
                continue;
            }
            if !user_ids.contains(local) {
                continue;
            }
            let kind = if let Some(e) = init {
                kind_from_expr(e, &slot_of).unwrap_or(SlotKind::Number)
            } else {
                SlotKind::Number
            };
            let _ = by_id.get(local);
            slot_of.insert(*local, kind);
            user_locals.push((*local, kind));
        }
    }

    Ok(ModuleInfo {
        uses_async,
        user_locals,
    })
}

fn collect_top_level_decl_ids(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Declare { local, .. } => {
                out.insert(*local);
            }
            Stmt::Block { body } => collect_top_level_decl_ids(body, out),
            _ => {}
        }
    }
}

fn kind_from_expr(expr: &Expr, slot_of: &HashMap<LocalId, SlotKind>) -> Option<SlotKind> {
    match expr {
        Expr::Unary {
            op: UnaryOp::TypeOf,
            ..
        } => Some(SlotKind::String),
        Expr::Binary {
            op: BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq,
            ..
        } => Some(SlotKind::Bool),
        Expr::Call { callee, .. } if is_async_api_callee(callee) => Some(SlotKind::Promise),
        Expr::Call { callee, .. }
            if is_named_callee(callee, "tcpListen")
                || is_named_callee(callee, "tcpAccept")
                || is_named_callee(callee, "tcpConnect") =>
        {
            Some(SlotKind::Handle)
        }
        Expr::Call { callee, .. } if is_named_callee(callee, "tcpLocalPort") => {
            Some(SlotKind::Number)
        }
        Expr::Number { .. } => Some(SlotKind::Number),
        Expr::Boolean { .. } => Some(SlotKind::Bool),
        Expr::String { .. } => Some(SlotKind::String),
        Expr::Local { id, .. } => slot_of.get(id).copied(),
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let Expr::String { value, .. } = property.as_ref() else {
                return None;
            };
            let prop = value.to_string_lossy();
            if prop == "then" || prop == "catch" {
                return Some(SlotKind::Promise);
            }
            let _ = object;
            None
        }
        _ => None,
    }
}

fn is_async_api_name(name: &str) -> bool {
    matches!(
        name,
        "tcpAcceptAsync" | "tcpConnectAsync" | "tcpReadAsync" | "tcpWriteAsync"
    )
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn is_async_api_callee(callee: &Expr) -> bool {
    match callee {
        Expr::IdentName { name, .. } => is_async_api_name(name),
        _ => false,
    }
}

fn check_stmt(
    stmt: &Stmt,
    uses: &mut bool,
    slot_of: &mut HashMap<LocalId, SlotKind>,
) -> Result<(), String> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            if let Some(e) = init {
                check_expr(e, uses, slot_of)?;
                if let Some(k) = kind_from_expr(e, slot_of) {
                    slot_of.insert(*local, k);
                }
            }
            Ok(())
        }
        Stmt::Expr { expr } => check_expr(expr, uses, slot_of),
        Stmt::Block { body } => {
            for s in body {
                check_stmt(s, uses, slot_of)?;
            }
            Ok(())
        }
        _ => Err("unsupported stmt in host_tcp_async".into()),
    }
}

fn check_expr(
    expr: &Expr,
    uses: &mut bool,
    slot_of: &mut HashMap<LocalId, SlotKind>,
) -> Result<(), String> {
    match expr {
        Expr::Call { callee, args, .. } => {
            if is_async_api_callee(callee) {
                *uses = true;
            }
            check_callee_and_args(callee, args, uses, slot_of)
        }
        Expr::Member { object, .. } => check_expr(object, uses, slot_of),
        Expr::Assign { target: _, value, .. } => check_expr(value, uses, slot_of),
        Expr::Binary { left, right, .. } => {
            check_expr(left, uses, slot_of)?;
            check_expr(right, uses, slot_of)
        }
        Expr::Unary { arg, .. } => check_expr(arg, uses, slot_of),
        Expr::Function { body, .. } => {
            for s in body {
                check_stmt(s, uses, slot_of)?;
            }
            Ok(())
        }
        Expr::Local { .. }
        | Expr::IdentName { .. }
        | Expr::Number { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Null { .. }
        | Expr::This { .. } => Ok(()),
        _ => Err(format!("unsupported expr in host_tcp_async: {expr:?}")),
    }
}

fn check_callee_and_args(
    callee: &Expr,
    args: &[Arg],
    uses: &mut bool,
    slot_of: &mut HashMap<LocalId, SlotKind>,
) -> Result<(), String> {
    if let Expr::Member {
        object,
        property,
        computed: false,
        ..
    } = callee
    {
        check_expr(object, uses, slot_of)?;
        let _ = property;
    } else {
        check_expr(callee, uses, slot_of)?;
    }
    for a in args {
        if let Arg::Expr(e) = a {
            check_expr(e, uses, slot_of)?;
        }
    }
    Ok(())
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

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: ModuleInfo) -> Self {
        let local_names: HashMap<LocalId, String> = module
            .locals
            .iter()
            .map(|l| (l.id, l.name.clone()))
            .collect();
        let mut slot_kind = HashMap::new();
        for (id, k) in &info.user_locals {
            slot_kind.insert(*id, *k);
        }
        Self {
            module,
            info,
            local_names,
            allocas: HashMap::new(),
            slot_kind,
            out: String::new(),
            body: String::new(),
            helpers: String::new(),
            str_globals: Vec::new(),
            tmp: 0,
            next_fn: 0,
            reaction_params: HashMap::new(),
            reaction_captures: Vec::new(),
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("%t{t}")
    }

    fn fresh_fn(&mut self, prefix: &str) -> String {
        let n = self.next_fn;
        self.next_fn += 1;
        format!("d_{prefix}_{n}")
    }

    fn is_named(&self, callee: &Expr, name: &str) -> bool {
        is_named_callee(callee, name)
            || matches!(callee, Expr::Local { id, .. } if self.local_names.get(id).map(|s| s.as_str()) == Some(name))
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM host_tcp_async (H07.02 async TCP → Promise)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(HOST_TCP_ASYNC_DECLARES)).ok();
        writeln!(self.out).ok();

        self.body.clear();
        self.tmp = 0;

        for (id, kind) in self.info.user_locals.clone() {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(id, ptr.clone());
            match kind {
                SlotKind::Number | SlotKind::Handle => {
                    writeln!(self.body, "  {ptr} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store i64 0, ptr {ptr}").ok();
                }
                SlotKind::Bool => {
                    writeln!(self.body, "  {ptr} = alloca i1, align 1").ok();
                    writeln!(self.body, "  store i1 false, ptr {ptr}").ok();
                }
                SlotKind::String | SlotKind::Promise => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        writeln!(self.body, "  {}", JOB_DRAIN.call("")).ok();

        for (id, kind) in self.info.user_locals.clone() {
            match kind {
                SlotKind::Number | SlotKind::Handle => {
                    let ptr = self.allocas.get(&id).cloned().unwrap();
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
                    // Skip pure handles (ports printed only if assigned observation numbers).
                    // Print all numbers — fixtures use small observation locals.
                    if kind == SlotKind::Number {
                        writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {v}"))).ok();
                    }
                }
                SlotKind::Bool => {
                    let ptr = self.allocas.get(&id).cloned().unwrap();
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i1, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i1 {v}"))).ok();
                }
                SlotKind::String => {
                    let ptr = self.allocas.get(&id).cloned().unwrap();
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotKind::Promise => {}
            }
        }

        for (content, gname) in self.str_globals.clone() {
            let n = content.len() + 1;
            let esc = escape_llvm_string(&content);
            writeln!(
                self.out,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        if !self.str_globals.is_empty() {
            writeln!(self.out).ok();
        }

        self.out.push_str(&self.helpers);
        if !self.helpers.is_empty() {
            writeln!(self.out).ok();
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        writeln!(self.out, "  {}", GC_INIT.call("")).ok();
        self.out.push_str(&self.body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                if let Some(e) = init {
                    let v = self.emit_expr(e)?;
                    self.store_local(*local, &v)?;
                }
                Ok(())
            }
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr)?;
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            _ => Err(diag("unsupported stmt")),
        }
    }

    fn store_local(&mut self, id: LocalId, value: &str) -> Result<(), Diagnostic> {
        let kind = self.slot_kind.get(&id).copied().unwrap_or(SlotKind::Number);
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("missing alloca"))?;
        match kind {
            SlotKind::Number | SlotKind::Handle => {
                // value may be i64 ssa or ptr (promise payload)
                if value.starts_with('%') {
                    // try ptrtoint if it's a ptr-typed ssa from async resolve path stored as ptr
                    // Convention: emit_expr returns i64 SSA for numbers/handles, ptr for promises/strings
                    if self.slot_kind.get(&id) == Some(&SlotKind::Handle)
                        || self.slot_kind.get(&id) == Some(&SlotKind::Number)
                    {
                        // If already i64 from number lit path, store directly — detect by tracking is hard.
                        // Always ptrtoint: number path returns inttoptr? We'll use dual convention:
                        // numbers/handles as i64 SSA names starting with %t — store as i64.
                        writeln!(self.body, "  store i64 {value}, ptr {ptr}").ok();
                    }
                } else {
                    writeln!(self.body, "  store i64 {value}, ptr {ptr}").ok();
                }
            }
            SlotKind::Bool => {
                writeln!(self.body, "  store i1 {value}, ptr {ptr}").ok();
            }
            SlotKind::String | SlotKind::Promise => {
                writeln!(self.body, "  store ptr {value}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        // Inside reaction: param is ptr value
        if let Expr::Local { id, .. } = expr {
            if let Some(v) = self.reaction_params.get(id).cloned() {
                return Ok(v);
            }
            if let Some(pos) = self.reaction_captures.iter().position(|c| c == id) {
                return self.load_capture(pos);
            }
            return self.load_local(*id);
        }
        match expr {
            Expr::Number { raw, .. } => {
                let n: i64 = raw.parse().unwrap_or(0);
                Ok(format!("{n}"))
            }
            Expr::Boolean { value, .. } => Ok(if *value { "true" } else { "false" }.into()),
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let g = self.intern_str(&s);
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {t} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0",
                    n = s.len() + 1
                )
                .ok();
                Ok(t)
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => {
                let name = match arg.as_ref() {
                    Expr::IdentName { name, .. } => Some(name.as_str()),
                    Expr::Local { id, .. } => self.local_names.get(id).map(|s| s.as_str()),
                    _ => None,
                };
                if let Some(name) = name {
                    if is_async_api_name(name)
                        || matches!(name, "tcpListen" | "tcpLocalPort" | "closeTcp")
                    {
                        let g = self.intern_str("function");
                        let t = self.fresh();
                        writeln!(
                            self.body,
                            "  {t} = getelementptr inbounds [9 x i8], ptr @{g}, i64 0, i64 0"
                        )
                        .ok();
                        return Ok(t);
                    }
                }
                Err(diag("typeof only on host APIs in host_tcp_async"))
            }
            Expr::Binary { op, left, right, .. } => {
                let l = self.emit_expr(left)?;
                let r = self.emit_expr(right)?;
                let t = self.fresh();
                let pred = match op {
                    BinaryOp::Gt => "sgt",
                    BinaryOp::GtEq => "sge",
                    BinaryOp::Lt => "slt",
                    BinaryOp::LtEq => "sle",
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "eq",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "ne",
                    _ => return Err(diag("unsupported binary")),
                };
                writeln!(self.body, "  {t} = icmp {pred} i64 {l}, {r}").ok();
                Ok(t)
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                value,
                ..
            } => {
                let v = self.emit_expr(value)?;
                if !self.reaction_captures.is_empty() {
                    let mut buf = std::mem::take(&mut self.body);
                    self.store_capture(&mut buf, *id, &v)?;
                    self.body = buf;
                } else {
                    self.store_local(*id, &v)?;
                }
                Ok(v)
            }
            Expr::Call { callee, args, .. } => self.emit_call(callee, args),
            _ => Err(diag(format!("unsupported expr: {expr:?}"))),
        }
    }

    fn load_local(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        let kind = self.slot_kind.get(&id).copied().unwrap_or(SlotKind::Number);
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("missing local"))?;
        let t = self.fresh();
        match kind {
            SlotKind::Number | SlotKind::Handle => {
                writeln!(self.body, "  {t} = load i64, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  {t} = load i1, ptr {ptr}").ok();
            }
            SlotKind::String | SlotKind::Promise => {
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
            }
        }
        Ok(t)
    }

    fn load_capture(&mut self, pos: usize) -> Result<String, Diagnostic> {
        let id = self.reaction_captures[pos];
        let kind = self.slot_kind.get(&id).copied().unwrap_or(SlotKind::Number);
        let slot = self.capture_slot_ssa(pos)?;
        let t = self.fresh();
        match kind {
            SlotKind::Number | SlotKind::Handle => {
                writeln!(self.body, "  {t} = load i64, ptr {slot}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  {t} = load i1, ptr {slot}").ok();
            }
            SlotKind::String | SlotKind::Promise => {
                writeln!(self.body, "  {t} = load ptr, ptr {slot}").ok();
            }
        }
        Ok(t)
    }

    fn capture_slot_ssa(&mut self, pos: usize) -> Result<String, Diagnostic> {
        if self.reaction_captures.len() == 1 {
            Ok("%data".into())
        } else {
            let n = self.reaction_captures.len();
            let t = self.fresh();
            writeln!(
                self.body,
                "  {t} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
            )
            .ok();
            let p = self.fresh();
            writeln!(self.body, "  {p} = load ptr, ptr {t}").ok();
            Ok(p)
        }
    }

    fn store_capture(
        &mut self,
        buf: &mut String,
        id: LocalId,
        value: &str,
    ) -> Result<(), Diagnostic> {
        let Some(pos) = self.reaction_captures.iter().position(|c| *c == id) else {
            return Ok(());
        };
        let kind = self.slot_kind.get(&id).copied().unwrap_or(SlotKind::Number);
        let slot = if self.reaction_captures.len() == 1 {
            "%data".to_string()
        } else {
            let n = self.reaction_captures.len();
            let t = format!("%cs{}", self.tmp);
            self.tmp += 1;
            writeln!(
                buf,
                "  {t} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
            )
            .ok();
            let p = format!("%csp{}", self.tmp);
            self.tmp += 1;
            writeln!(buf, "  {p} = load ptr, ptr {t}").ok();
            p
        };
        match kind {
            SlotKind::Number | SlotKind::Handle => {
                // value may be ptr (from promise param) → ptrtoint
                if value == "%value" {
                    let n = format!("%n{}", self.tmp);
                    self.tmp += 1;
                    writeln!(buf, "  {n} = ptrtoint ptr {value} to i64").ok();
                    writeln!(buf, "  store i64 {n}, ptr {slot}").ok();
                } else {
                    writeln!(buf, "  store i64 {value}, ptr {slot}").ok();
                }
            }
            SlotKind::Bool => {
                writeln!(buf, "  store i1 {value}, ptr {slot}").ok();
            }
            SlotKind::String | SlotKind::Promise => {
                writeln!(buf, "  store ptr {value}, ptr {slot}").ok();
            }
        }
        Ok(())
    }

    fn intern_str(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.{}", self.str_globals.len());
        self.str_globals.push((s.to_string(), g.clone()));
        g
    }

    fn emit_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        // p.then(...)
        if let Expr::Member {
            object,
            property,
            computed: false,
            ..
        } = callee
        {
            if let Expr::String { value, .. } = property.as_ref() {
                let prop = value.to_string_lossy();
                if prop == "then" {
                    return self.emit_then(object, args);
                }
            }
        }

        if self.is_named(callee, "tcpListen") {
            let port = match args.first() {
                Some(Arg::Expr(e)) => self.emit_expr(e)?,
                _ => return Err(diag("tcpListen port")),
            };
            let backlog = if args.len() >= 2 {
                match &args[1] {
                    Arg::Expr(e) => self.emit_expr(e)?,
                    _ => return Err(diag("tcpListen backlog")),
                }
            } else {
                "128".into()
            };
            let hptr = self.fresh();
            writeln!(self.body, "  {hptr} = alloca i64, align 8").ok();
            let err = self.fresh();
            writeln!(
                self.body,
                "  {err} = {}",
                HOST_TCP_LISTEN.call(&format!("i32 {port}, i32 {backlog}, ptr {hptr}"))
            )
            .ok();
            let h = self.fresh();
            writeln!(self.body, "  {h} = load i64, ptr {hptr}").ok();
            return Ok(h);
        }

        if self.is_named(callee, "tcpLocalPort") {
            let h = match args.first() {
                Some(Arg::Expr(e)) => self.emit_expr(e)?,
                _ => return Err(diag("tcpLocalPort handle")),
            };
            let pptr = self.fresh();
            writeln!(self.body, "  {pptr} = alloca i32, align 4").ok();
            let err = self.fresh();
            writeln!(
                self.body,
                "  {err} = {}",
                HOST_TCP_LOCAL_PORT.call(&format!("i64 {h}, ptr {pptr}"))
            )
            .ok();
            let p32 = self.fresh();
            writeln!(self.body, "  {p32} = load i32, ptr {pptr}").ok();
            let p = self.fresh();
            writeln!(self.body, "  {p} = sext i32 {p32} to i64").ok();
            return Ok(p);
        }

        if self.is_named(callee, "closeTcp") {
            let h = match args.first() {
                Some(Arg::Expr(e)) => self.emit_expr(e)?,
                _ => return Err(diag("closeTcp handle")),
            };
            // h may be promise param ptr
            let hi = if h == "%value" {
                let t = self.fresh();
                writeln!(self.body, "  {t} = ptrtoint ptr {h} to i64").ok();
                t
            } else {
                h
            };
            let err = self.fresh();
            writeln!(
                self.body,
                "  {err} = {}",
                HOST_HANDLE_CLOSE.call(&format!("i64 {hi}"))
            )
            .ok();
            return Ok("0".into());
        }

        if self.is_named(callee, "tcpAccept") {
            let h = match args.first() {
                Some(Arg::Expr(e)) => self.emit_expr(e)?,
                _ => return Err(diag("tcpAccept handle")),
            };
            let hptr = self.fresh();
            writeln!(self.body, "  {hptr} = alloca i64, align 8").ok();
            let err = self.fresh();
            writeln!(
                self.body,
                "  {err} = {}",
                HOST_TCP_ACCEPT.call(&format!("i64 {h}, ptr {hptr}"))
            )
            .ok();
            let out = self.fresh();
            writeln!(self.body, "  {out} = load i64, ptr {hptr}").ok();
            return Ok(out);
        }

        if self.is_named(callee, "tcpConnect") {
            if args.len() != 2 {
                return Err(diag("tcpConnect(host, port)"));
            }
            let host = match &args[0] {
                Arg::Expr(e) => self.emit_expr(e)?,
                _ => return Err(diag("host")),
            };
            let port = match &args[1] {
                Arg::Expr(e) => self.emit_expr(e)?,
                _ => return Err(diag("port")),
            };
            let p32 = self.fresh();
            writeln!(self.body, "  {p32} = trunc i64 {port} to i32").ok();
            let hptr = self.fresh();
            writeln!(self.body, "  {hptr} = alloca i64, align 8").ok();
            let err = self.fresh();
            writeln!(
                self.body,
                "  {err} = {}",
                HOST_TCP_CONNECT.call(&format!("ptr {host}, i32 {p32}, ptr {hptr}"))
            )
            .ok();
            let out = self.fresh();
            writeln!(self.body, "  {out} = load i64, ptr {hptr}").ok();
            return Ok(out);
        }

        if self.is_named(callee, "tcpAcceptAsync") {
            let h = match args.first() {
                Some(Arg::Expr(e)) => self.emit_expr(e)?,
                _ => return Err(diag("tcpAcceptAsync handle")),
            };
            let t = self.fresh();
            writeln!(
                self.body,
                "  {t} = {}",
                HOST_TCP_ACCEPT_ASYNC.call(&format!("i64 {h}"))
            )
            .ok();
            return Ok(t);
        }

        if self.is_named(callee, "tcpConnectAsync") {
            if args.len() != 2 {
                return Err(diag("tcpConnectAsync(host, port)"));
            }
            let host = match &args[0] {
                Arg::Expr(e) => self.emit_expr(e)?,
                _ => return Err(diag("host")),
            };
            let port = match &args[1] {
                Arg::Expr(e) => self.emit_expr(e)?,
                _ => return Err(diag("port")),
            };
            let p32 = self.fresh();
            writeln!(self.body, "  {p32} = trunc i64 {port} to i32").ok();
            let t = self.fresh();
            writeln!(
                self.body,
                "  {t} = {}",
                HOST_TCP_CONNECT_ASYNC.call(&format!("ptr {host}, i32 {p32}"))
            )
            .ok();
            return Ok(t);
        }

        if self.is_named(callee, "tcpReadAsync") {
            if args.len() != 2 {
                return Err(diag("tcpReadAsync(conn, maxLen)"));
            }
            let mut h = match &args[0] {
                Arg::Expr(e) => self.emit_expr(e)?,
                _ => return Err(diag("conn")),
            };
            if h == "%value" {
                let t = self.fresh();
                writeln!(self.body, "  {t} = ptrtoint ptr {h} to i64").ok();
                h = t;
            }
            let max = match &args[1] {
                Arg::Expr(e) => self.emit_expr(e)?,
                _ => return Err(diag("maxLen")),
            };
            let t = self.fresh();
            writeln!(
                self.body,
                "  {t} = {}",
                HOST_TCP_READ_ASYNC.call(&format!("i64 {h}, i64 {max}"))
            )
            .ok();
            return Ok(t);
        }

        if self.is_named(callee, "tcpWriteAsync") {
            if args.len() != 2 {
                return Err(diag("tcpWriteAsync(conn, data)"));
            }
            let mut h = match &args[0] {
                Arg::Expr(e) => self.emit_expr(e)?,
                _ => return Err(diag("conn")),
            };
            if h == "%value" {
                let t = self.fresh();
                writeln!(self.body, "  {t} = ptrtoint ptr {h} to i64").ok();
                h = t;
            }
            let data_expr = match &args[1] {
                Arg::Expr(e) => e,
                _ => return Err(diag("data")),
            };
            let (ptr, len) = self.emit_bytes(data_expr)?;
            let t = self.fresh();
            writeln!(
                self.body,
                "  {t} = {}",
                HOST_TCP_WRITE_ASYNC.call(&format!("i64 {h}, ptr {ptr}, i64 {len}"))
            )
            .ok();
            return Ok(t);
        }

        Err(diag("unsupported call in host_tcp_async"))
    }

    fn emit_bytes(&mut self, expr: &Expr) -> Result<(String, String), Diagnostic> {
        match expr {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let g = self.intern_str(&s);
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {t} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0",
                    n = s.len() + 1
                )
                .ok();
                Ok((t, format!("{}", s.len())))
            }
            _ => Err(diag("tcpWriteAsync data must be string literal")),
        }
    }

    fn emit_then(&mut self, object: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let p = self.emit_expr(object)?;
        let mut on_ful = "null".to_string();
        let mut ful_data = "null".to_string();
        let mut on_rej = "null".to_string();
        let mut rej_data = "null".to_string();
        if let Some(Arg::Expr(f0)) = args.first() {
            let Expr::Function { params, body, .. } = f0 else {
                return Err(diag("then callback must be function expression"));
            };
            let (name, data) = self.emit_reaction_fn(params, body)?;
            on_ful = format!("@{name}");
            ful_data = data;
        }
        if let Some(Arg::Expr(f1)) = args.get(1) {
            let Expr::Function { params, body, .. } = f1 else {
                return Err(diag("then reject callback must be function expression"));
            };
            let (name, data) = self.emit_reaction_fn(params, body)?;
            on_rej = format!("@{name}");
            rej_data = data;
        }
        let t = self.fresh();
        writeln!(
            self.body,
            "  {}",
            PROMISE_THEN.call_to(
                &t,
                &format!(
                    "ptr {p}, ptr {on_ful}, ptr {ful_data}, ptr {on_rej}, ptr {rej_data}"
                )
            )
        )
        .ok();
        Ok(t)
    }

    fn emit_reaction_fn(
        &mut self,
        params: &[Param],
        body: &[Stmt],
    ) -> Result<(String, String), Diagnostic> {
        let fn_name = self.fresh_fn("react");
        let mut param_id = None;
        if let Some(p) = params.first() {
            let Pattern::Local(id) = &p.pattern else {
                return Err(diag("bad param"));
            };
            param_id = Some(*id);
        }

        let mut assigned = HashSet::new();
        collect_assigned_locals(body, &mut assigned);
        // also captures used in nested calls
        collect_used_locals(body, &mut assigned);
        let mut captures: Vec<LocalId> = assigned
            .into_iter()
            .filter(|id| {
                matches!(
                    self.slot_kind.get(id),
                    Some(SlotKind::Number)
                        | Some(SlotKind::Handle)
                        | Some(SlotKind::Bool)
                        | Some(SlotKind::String)
                )
            })
            .filter(|id| self.allocas.contains_key(id))
            .collect();
        captures.sort_by_key(|id| id.0);

        // Capture env points at main allocas. When nesting `then` inside a reaction,
        // resolve those pointers via the current reaction's `%data` (main names are
        // not in scope inside helper functions).
        let nested = !self.reaction_captures.is_empty();
        let data_operand = if captures.is_empty() {
            "null".to_string()
        } else if captures.len() == 1 {
            if nested {
                let id = captures[0];
                let Some(pos) = self.reaction_captures.iter().position(|c| c == &id) else {
                    return Err(diag("nested capture not in outer reaction"));
                };
                self.capture_slot_ssa(pos)?
            } else {
                self.allocas
                    .get(&captures[0])
                    .cloned()
                    .ok_or_else(|| diag("capture missing alloca"))?
            }
        } else {
            let n = captures.len();
            let env = self.fresh();
            writeln!(self.body, "  {env} = alloca [{n} x ptr], align 8").ok();
            for (i, id) in captures.iter().enumerate() {
                let ptr = if nested {
                    let Some(pos) = self.reaction_captures.iter().position(|c| c == id) else {
                        return Err(diag("nested capture not in outer reaction"));
                    };
                    self.capture_slot_ssa(pos)?
                } else {
                    self.allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("capture missing alloca"))?
                };
                let slot = self.fresh();
                writeln!(
                    self.body,
                    "  {slot} = getelementptr inbounds [{n} x ptr], ptr {env}, i64 0, i64 {i}"
                )
                .ok();
                writeln!(self.body, "  store ptr {ptr}, ptr {slot}").ok();
            }
            env
        };

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_react = std::mem::take(&mut self.reaction_params);
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.reaction_params.clear();
        if let Some(id) = param_id {
            self.reaction_params.insert(id, "%value".into());
        }
        self.reaction_captures = captures;

        let mut ret_val = "%value".to_string();
        for stmt in body {
            match stmt {
                Stmt::Expr { expr } => {
                    if let Expr::Assign {
                        target: AssignTarget::Local(id),
                        value,
                        ..
                    } = expr
                    {
                        let v = self.emit_expr(value)?;
                        let mut buf = std::mem::take(&mut self.body);
                        self.store_capture(&mut buf, *id, &v)?;
                        self.body = buf;
                        ret_val = v;
                    } else {
                        ret_val = self.emit_expr(expr)?;
                    }
                }
                Stmt::Declare { local, init, .. } => {
                    // reaction-local: treat as assign if we have capture
                    if let Some(e) = init {
                        let v = self.emit_expr(e)?;
                        let mut buf = std::mem::take(&mut self.body);
                        self.store_capture(&mut buf, *local, &v)?;
                        self.body = buf;
                        ret_val = v;
                    }
                }
                Stmt::Block { body: inner } => {
                    for s in inner {
                        if let Stmt::Expr { expr } = s {
                            if let Expr::Assign {
                                target: AssignTarget::Local(id),
                                value,
                                ..
                            } = expr
                            {
                                let v = self.emit_expr(value)?;
                                let mut buf = std::mem::take(&mut self.body);
                                self.store_capture(&mut buf, *id, &v)?;
                                self.body = buf;
                                ret_val = v;
                            } else {
                                ret_val = self.emit_expr(expr)?;
                            }
                        }
                    }
                }
                _ => return Err(diag("unsupported stmt in reaction")),
            }
        }

        // Ensure return is ptr
        let ret_ptr = if ret_val == "%value" || ret_val.starts_with('%') && self.looks_like_ptr(&ret_val) {
            ret_val.clone()
        } else {
            let t = format!("%ret{}", self.tmp);
            self.tmp += 1;
            writeln!(self.body, "  {t} = inttoptr i64 {ret_val} to ptr").ok();
            t
        };

        let mut helper = String::new();
        writeln!(
            helper,
            "define ptr @{fn_name}(ptr %data, ptr %value) {{"
        )
        .ok();
        writeln!(helper, "entry:").ok();
        helper.push_str(&self.body);
        writeln!(helper, "  ret ptr {ret_ptr}").ok();
        writeln!(helper, "}}").ok();
        self.helpers.push_str(&helper);

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.reaction_params = saved_react;
        self.reaction_captures = saved_caps;

        Ok((fn_name, data_operand))
    }

    fn looks_like_ptr(&self, _s: &str) -> bool {
        // Heuristic: after emit_call async APIs return ptr; numbers are i64.
        // Safer: always inttoptr for non-%value numeric literals handled above.
        true
    }
}

fn collect_assigned_locals(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Expr { expr } => collect_assigned_expr(expr, out),
            Stmt::Declare { local, init, .. } => {
                out.insert(*local);
                if let Some(e) = init {
                    collect_used_expr(e, out);
                }
            }
            Stmt::Block { body } => collect_assigned_locals(body, out),
            _ => {}
        }
    }
}

fn collect_assigned_expr(expr: &Expr, out: &mut HashSet<LocalId>) {
    match expr {
        Expr::Assign {
            target: AssignTarget::Local(id),
            value,
            ..
        } => {
            out.insert(*id);
            collect_used_expr(value, out);
            collect_assigned_expr(value, out);
        }
        Expr::Call { args, callee, .. } => {
            collect_used_expr(callee, out);
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_used_expr(e, out);
                    collect_assigned_expr(e, out);
                }
            }
        }
        Expr::Function { body, .. } => {
            collect_assigned_locals(body, out);
            collect_used_locals(body, out);
        }
        Expr::Member { object, .. } => collect_assigned_expr(object, out),
        _ => collect_used_expr(expr, out),
    }
}

fn collect_used_locals(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Expr { expr } => collect_used_expr(expr, out),
            Stmt::Declare { init, .. } => {
                if let Some(e) = init {
                    collect_used_expr(e, out);
                }
            }
            Stmt::Block { body } => collect_used_locals(body, out),
            _ => {}
        }
    }
}

fn collect_used_expr(expr: &Expr, out: &mut HashSet<LocalId>) {
    match expr {
        Expr::Local { id, .. } => {
            out.insert(*id);
        }
        Expr::Call { callee, args, .. } => {
            collect_used_expr(callee, out);
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_used_expr(e, out);
                }
            }
        }
        Expr::Member { object, .. } => collect_used_expr(object, out),
        Expr::Assign {
            target: AssignTarget::Local(id),
            value,
            ..
        } => {
            out.insert(*id);
            collect_used_expr(value, out);
        }
        Expr::Assign { value, .. } => collect_used_expr(value, out),
        Expr::Binary { left, right, .. } => {
            collect_used_expr(left, out);
            collect_used_expr(right, out);
        }
        Expr::Unary { arg, .. } => collect_used_expr(arg, out),
        Expr::Function { body, .. } => {
            collect_used_locals(body, out);
            collect_assigned_locals(body, out);
        }
        _ => {}
    }
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}
