//! H14.01 / H14.02: SIGINT/SIGTERM watch/ignore/restore via Runtime signal ABI.
//!
//! Supported subset for conformance:
//! - top-level number/bool/string locals
//! - `onSignal("SIGINT"|"SIGTERM", function () { … })`
//! - `raiseSignal("SIGINT"|"SIGTERM")`
//! - `ignoreSignal("SIGINT"|"SIGTERM")` / `restoreSignal("SIGINT"|"SIGTERM")`
//! - `typeof` on `onSignal` / `raiseSignal` / `ignoreSignal` / `restoreSignal`
//! - number assigns in signal callbacks
//!
//! End of main: `job_drain` (promotes pending signals, runs handlers), then
//! print observation locals in declaration order.
//!
//! Default without `onSignal`/`ignoreSignal`: OS terminate (SIG_DFL) — documented
//! in Runtime host header; covered by Runtime unit tests (subprocess).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_SIGNAL_DECLARES, HOST_SIGNAL_IGNORE, HOST_SIGNAL_RAISE,
    HOST_SIGNAL_RESTORE, HOST_SIGNAL_WATCH, JOB_DRAIN, PRINT_BOOL, PRINT_I64, PRINT_STR,
};

/// Portable codes matching `DRACONIC_HOST_SIG_*` in draconic_rt_host.h.
const SIG_INT: i32 = 2;
const SIG_TERM: i32 = 15;

pub(crate) fn is_host_signal_module(module: &Module) -> bool {
    match try_classify(module) {
        Ok(info) => info.uses_signal,
        Err(_) => false,
    }
}

pub(crate) fn emit_host_signals(module: &Module) -> Result<String, Diagnostic> {
    let info = try_classify(module).map_err(diag)?;
    if !info.uses_signal {
        return Err(diag("internal: not a host_signal module"));
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
}

struct ModuleInfo {
    uses_signal: bool,
    user_locals: Vec<(LocalId, SlotKind)>,
}

fn try_classify(module: &Module) -> Result<ModuleInfo, String> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_ids = HashSet::new();
    collect_top_level_decl_ids(&module.body, &mut user_ids);

    let mut user_locals = Vec::new();
    let mut seen = HashSet::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, init, .. } = stmt {
            if !seen.insert(*local) {
                continue;
            }
            if !user_ids.contains(local) {
                continue;
            }
            let kind = if let Some(e) = init {
                if let Some(k) = kind_from_init(e) {
                    k
                } else {
                    match by_id.get(local).map(|l| &l.ty) {
                        Some(Type::Boolean) => SlotKind::Bool,
                        Some(Type::String) => SlotKind::String,
                        _ => SlotKind::Number,
                    }
                }
            } else {
                match by_id.get(local).map(|l| &l.ty) {
                    Some(Type::Boolean) => SlotKind::Bool,
                    Some(Type::String) => SlotKind::String,
                    _ => SlotKind::Number,
                }
            };
            user_locals.push((*local, kind));
        }
    }

    let mut uses_signal = false;
    for stmt in &module.body {
        check_stmt(stmt, &mut uses_signal)?;
    }

    Ok(ModuleInfo {
        uses_signal,
        user_locals,
    })
}

fn collect_top_level_decl_ids(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        if let Stmt::Declare { local, .. } = stmt {
            out.insert(*local);
        }
    }
}

fn kind_from_init(e: &Expr) -> Option<SlotKind> {
    match e {
        Expr::Number { .. } => Some(SlotKind::Number),
        Expr::Boolean { .. } => Some(SlotKind::Bool),
        Expr::String { .. } => Some(SlotKind::String),
        Expr::Unary {
            op: UnaryOp::TypeOf,
            ..
        } => Some(SlotKind::String),
        Expr::Binary {
            op:
                BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEqEq
                | BinaryOp::EqEq,
            ..
        } => Some(SlotKind::Bool),
        _ => None,
    }
}

fn check_stmt(stmt: &Stmt, uses: &mut bool) -> Result<(), String> {
    match stmt {
        Stmt::Declare { init, .. } => {
            if let Some(e) = init {
                check_expr(e, uses)?;
            }
            Ok(())
        }
        Stmt::Expr { expr, .. } => check_expr(expr, uses),
        Stmt::Block { body } => {
            for s in body {
                check_stmt(s, uses)?;
            }
            Ok(())
        }
        _ => Err("unsupported stmt in host_signal module".into()),
    }
}

fn is_signal_api_name(name: &str) -> bool {
    name == "onSignal" || name == "raiseSignal" || name == "ignoreSignal" || name == "restoreSignal"
}

fn is_named_callee(callee: &Expr, name: &str) -> bool {
    matches!(callee, Expr::IdentName { name: n, .. } if n == name)
}

fn check_expr(expr: &Expr, uses: &mut bool) -> Result<(), String> {
    match expr {
        Expr::Call { callee, args, .. } if is_named_callee(callee, "onSignal") => {
            *uses = true;
            if args.len() != 2 {
                return Err("onSignal needs (name, handler)".into());
            }
            let name = string_lit_arg(&args[0]).ok_or("onSignal name must be string lit")?;
            if name.as_str() != "SIGINT" && name.as_str() != "SIGTERM" {
                return Err("onSignal name must be SIGINT or SIGTERM".into());
            }
            match &args[1] {
                Arg::Expr(Expr::Function {
                    params,
                    body,
                    is_async,
                    is_generator,
                    ..
                }) => {
                    if *is_async || *is_generator {
                        return Err("async/generator signal handler unsupported".into());
                    }
                    if !params.is_empty() {
                        return Err("signal handler params unsupported".into());
                    }
                    for s in body {
                        check_handler_stmt(s)?;
                    }
                }
                _ => return Err("onSignal handler must be function".into()),
            }
            Ok(())
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "raiseSignal") => {
            *uses = true;
            if args.len() != 1 {
                return Err("raiseSignal needs (name)".into());
            }
            let name = string_lit_arg(&args[0]).ok_or("raiseSignal name must be string lit")?;
            if name.as_str() != "SIGINT" && name.as_str() != "SIGTERM" {
                return Err("raiseSignal name must be SIGINT or SIGTERM".into());
            }
            Ok(())
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "ignoreSignal") => {
            *uses = true;
            if args.len() != 1 {
                return Err("ignoreSignal needs (name)".into());
            }
            let name = string_lit_arg(&args[0]).ok_or("ignoreSignal name must be string lit")?;
            if name.as_str() != "SIGINT" && name.as_str() != "SIGTERM" {
                return Err("ignoreSignal name must be SIGINT or SIGTERM".into());
            }
            Ok(())
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "restoreSignal") => {
            *uses = true;
            if args.len() != 1 {
                return Err("restoreSignal needs (name)".into());
            }
            let name = string_lit_arg(&args[0]).ok_or("restoreSignal name must be string lit")?;
            if name.as_str() != "SIGINT" && name.as_str() != "SIGTERM" {
                return Err("restoreSignal name must be SIGINT or SIGTERM".into());
            }
            Ok(())
        }
        Expr::Call { .. } => Err("unsupported call in host_signal".into()),
        Expr::Assign {
            target: AssignTarget::Local(_),
            value,
            ..
        } => check_expr(value, uses),
        Expr::Assign { .. } => Err("only local assign targets in host_signal".into()),
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            if matches!(&**arg, Expr::IdentName { name, .. } if is_signal_api_name(name)) {
                *uses = true;
                Ok(())
            } else {
                check_expr(arg, uses)
            }
        }
        Expr::Binary { left, right, .. } => {
            check_expr(left, uses)?;
            check_expr(right, uses)
        }
        Expr::Number { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Local { .. }
        | Expr::IdentName { .. } => Ok(()),
        _ => Err("unsupported expr in host_signal".into()),
    }
}

fn check_handler_stmt(stmt: &Stmt) -> Result<(), String> {
    match stmt {
        Stmt::Expr { expr } => check_handler_expr(expr),
        Stmt::Block { body } => {
            for s in body {
                check_handler_stmt(s)?;
            }
            Ok(())
        }
        Stmt::Return { value } => {
            if let Some(e) = value {
                check_handler_expr(e)?;
            }
            Ok(())
        }
        _ => Err("unsupported stmt in signal handler".into()),
    }
}

fn check_handler_expr(expr: &Expr) -> Result<(), String> {
    match expr {
        Expr::Assign {
            target: AssignTarget::Local(_),
            value,
            ..
        } => check_handler_expr(value),
        Expr::Number { .. } | Expr::Boolean { .. } | Expr::Local { .. } => Ok(()),
        Expr::Binary { left, right, .. } => {
            check_handler_expr(left)?;
            check_handler_expr(right)
        }
        _ => Err("unsupported expr in signal handler".into()),
    }
}

fn string_lit_arg(arg: &Arg) -> Option<String> {
    match arg {
        Arg::Expr(Expr::String { value, .. }) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn diag(msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(msg.into(), Span::dummy())
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

struct Emitter<'a> {
    module: &'a Module,
    info: ModuleInfo,
    out: String,
    body: String,
    helpers: String,
    tmp: usize,
    fn_tmp: usize,
    allocas: HashMap<LocalId, String>,
    str_globals: Vec<(String, String)>,
    reaction_captures: Vec<LocalId>,
    in_callback: bool,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: ModuleInfo) -> Self {
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            helpers: String::new(),
            tmp: 0,
            fn_tmp: 0,
            allocas: HashMap::new(),
            str_globals: Vec::new(),
            reaction_captures: Vec::new(),
            in_callback: false,
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("%t{n}")
    }

    fn fresh_fn(&mut self, prefix: &str) -> String {
        let n = self.fn_tmp;
        self.fn_tmp += 1;
        format!("drac_{prefix}_{n}")
    }

    fn slot_kind(&self, id: LocalId) -> Option<SlotKind> {
        self.info
            .user_locals
            .iter()
            .find(|(l, _)| *l == id)
            .map(|(_, k)| *k)
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.{}", self.str_globals.len());
        self.str_globals.push((s.to_string(), g.clone()));
        g
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM host_signals (H14 SIGINT/SIGTERM watch/ignore/restore)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(HOST_SIGNAL_DECLARES)).ok();
        writeln!(self.out).ok();

        self.body.clear();
        self.tmp = 0;

        for (id, kind) in self.info.user_locals.clone() {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(id, ptr.clone());
            match kind {
                SlotKind::Number => {
                    writeln!(self.body, "  {ptr} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store i64 0, ptr {ptr}").ok();
                }
                SlotKind::Bool => {
                    writeln!(self.body, "  {ptr} = alloca i8, align 1").ok();
                    writeln!(self.body, "  store i8 0, ptr {ptr}").ok();
                }
                SlotKind::String => {
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
            let ptr = self.allocas.get(&id).cloned().unwrap();
            match kind {
                SlotKind::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {v}"))).ok();
                }
                SlotKind::Bool => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
                }
                SlotKind::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
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
            _ => Err(diag("host_signal: unsupported stmt")),
        }
    }

    fn store_local(&mut self, id: LocalId, val: &str) -> Result<(), Diagnostic> {
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("host_signal: unknown local"))?;
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
            SlotKind::Number => {
                writeln!(self.body, "  store i64 {val}, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  store i8 {val}, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  store ptr {val}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    fn load_local(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("host_signal: unknown local"))?;
        let v = self.fresh();
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
            SlotKind::Number => {
                writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
            }
        }
        Ok(v)
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n: i64 = raw.parse::<f64>().unwrap_or(0.0) as i64;
                Ok(format!("{n}"))
            }
            Expr::Boolean { value, .. } => Ok(if *value { "1".into() } else { "0".into() }),
            Expr::Local { id, .. } => {
                if self.in_callback {
                    if let Some(pos) = self.reaction_captures.iter().position(|c| c == id) {
                        return self.load_capture(pos);
                    }
                }
                self.load_local(*id)
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => self.emit_typeof(arg),
            Expr::Binary {
                op, left, right, ..
            } => self.emit_binary(*op, left, right),
            Expr::Call { callee, args, .. } if is_named_callee(callee, "onSignal") => {
                self.emit_on_signal(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "raiseSignal") => {
                self.emit_raise_signal(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "ignoreSignal") => {
                self.emit_ignore_signal(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "restoreSignal") => {
                self.emit_restore_signal(args)
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                value,
                ..
            } => {
                let v = self.emit_expr(value)?;
                if self.in_callback {
                    self.store_capture(*id, &v)?;
                } else {
                    self.store_local(*id, &v)?;
                }
                Ok(v)
            }
            Expr::IdentName { name, .. } if is_signal_api_name(name) => Ok("null".into()),
            _ => Err(diag("host_signal: unsupported expr")),
        }
    }

    fn load_capture(&mut self, pos: usize) -> Result<String, Diagnostic> {
        let n = self.reaction_captures.len();
        let v = self.fresh();
        if n == 1 {
            let id = self.reaction_captures[0];
            match self.slot_kind(id).unwrap_or(SlotKind::Number) {
                SlotKind::Number => {
                    writeln!(self.body, "  {v} = load i64, ptr %data").ok();
                }
                SlotKind::Bool => {
                    writeln!(self.body, "  {v} = load i8, ptr %data").ok();
                }
                SlotKind::String => {
                    writeln!(self.body, "  {v} = load ptr, ptr %data").ok();
                }
            }
            return Ok(v);
        }
        let slot = self.fresh();
        let ptr = self.fresh();
        writeln!(
            self.body,
            "  {slot} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
        )
        .ok();
        writeln!(self.body, "  {ptr} = load ptr, ptr {slot}").ok();
        let id = self.reaction_captures[pos];
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
            SlotKind::Number => {
                writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
            }
        }
        Ok(v)
    }

    fn store_capture(&mut self, id: LocalId, val: &str) -> Result<(), Diagnostic> {
        let pos = self
            .reaction_captures
            .iter()
            .position(|c| *c == id)
            .ok_or_else(|| diag("host_signal: capture missing"))?;
        let n = self.reaction_captures.len();
        if n == 1 {
            match self.slot_kind(id).unwrap_or(SlotKind::Number) {
                SlotKind::Number => {
                    writeln!(self.body, "  store i64 {val}, ptr %data").ok();
                }
                SlotKind::Bool => {
                    writeln!(self.body, "  store i8 {val}, ptr %data").ok();
                }
                SlotKind::String => {
                    writeln!(self.body, "  store ptr {val}, ptr %data").ok();
                }
            }
            return Ok(());
        }
        let slot = self.fresh();
        let ptr = self.fresh();
        writeln!(
            self.body,
            "  {slot} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
        )
        .ok();
        writeln!(self.body, "  {ptr} = load ptr, ptr {slot}").ok();
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
            SlotKind::Number => {
                writeln!(self.body, "  store i64 {val}, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  store i8 {val}, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  store ptr {val}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        let s = match arg {
            Expr::IdentName { name, .. } if is_signal_api_name(name) => "function",
            _ => return Err(diag("host_signal: typeof only on signal host APIs")),
        };
        let g = self.intern_cstr(s);
        let n = s.len() + 1;
        let p = self.fresh();
        writeln!(
            self.body,
            "  {p} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0"
        )
        .ok();
        Ok(p)
    }

    fn emit_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<String, Diagnostic> {
        let l = self.emit_expr(left)?;
        let r = self.emit_expr(right)?;
        let v = self.fresh();
        match op {
            BinaryOp::Add => {
                writeln!(self.body, "  {v} = add i64 {l}, {r}").ok();
                Ok(v)
            }
            BinaryOp::Sub => {
                writeln!(self.body, "  {v} = sub i64 {l}, {r}").ok();
                Ok(v)
            }
            BinaryOp::Gt => {
                writeln!(self.body, "  {v} = icmp sgt i64 {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {v} to i8").ok();
                Ok(b)
            }
            BinaryOp::EqEq | BinaryOp::EqEqEq => {
                writeln!(self.body, "  {v} = icmp eq i64 {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {v} to i8").ok();
                Ok(b)
            }
            _ => Err(diag("host_signal: unsupported binary")),
        }
    }

    fn sig_code(name: &str) -> Result<i32, Diagnostic> {
        match name {
            "SIGINT" => Ok(SIG_INT),
            "SIGTERM" => Ok(SIG_TERM),
            _ => Err(diag("host_signal: bad signal name")),
        }
    }

    fn emit_on_signal(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let name = string_lit_arg(&args[0]).ok_or_else(|| diag("onSignal name"))?;
        let code = Self::sig_code(name.as_str())?;
        let fn_expr = match &args[1] {
            Arg::Expr(e) => e,
            _ => return Err(diag("onSignal handler")),
        };
        let Expr::Function { body, .. } = fn_expr else {
            return Err(diag("onSignal needs function"));
        };
        let (fn_name, data_op) = self.emit_handler(body)?;
        let err = self.fresh();
        writeln!(
            self.body,
            "  {}",
            HOST_SIGNAL_WATCH.call_to(&err, &format!("i32 {code}, ptr @{fn_name}, ptr {data_op}"))
        )
        .ok();
        Ok("0".into())
    }

    fn emit_raise_signal(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let name = string_lit_arg(&args[0]).ok_or_else(|| diag("raiseSignal name"))?;
        let code = Self::sig_code(name.as_str())?;
        let err = self.fresh();
        writeln!(
            self.body,
            "  {}",
            HOST_SIGNAL_RAISE.call_to(&err, &format!("i32 {code}"))
        )
        .ok();
        Ok("0".into())
    }

    fn emit_ignore_signal(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let name = string_lit_arg(&args[0]).ok_or_else(|| diag("ignoreSignal name"))?;
        let code = Self::sig_code(name.as_str())?;
        let err = self.fresh();
        writeln!(
            self.body,
            "  {}",
            HOST_SIGNAL_IGNORE.call_to(&err, &format!("i32 {code}"))
        )
        .ok();
        Ok("0".into())
    }

    fn emit_restore_signal(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let name = string_lit_arg(&args[0]).ok_or_else(|| diag("restoreSignal name"))?;
        let code = Self::sig_code(name.as_str())?;
        let err = self.fresh();
        writeln!(
            self.body,
            "  {}",
            HOST_SIGNAL_RESTORE.call_to(&err, &format!("i32 {code}"))
        )
        .ok();
        Ok("0".into())
    }

    fn emit_handler(&mut self, body: &[Stmt]) -> Result<(String, String), Diagnostic> {
        let fn_name = self.fresh_fn("sig");
        let mut used = HashSet::new();
        collect_used_locals(body, &mut used);
        let mut captures: Vec<LocalId> = used
            .into_iter()
            .filter(|id| {
                matches!(
                    self.slot_kind(*id),
                    Some(SlotKind::Number) | Some(SlotKind::Bool) | Some(SlotKind::String)
                )
            })
            .collect();
        captures.sort_by_key(|id| id.0);

        let data_operand = if captures.is_empty() {
            "null".to_string()
        } else if captures.len() == 1 {
            self.allocas
                .get(&captures[0])
                .cloned()
                .ok_or_else(|| diag("capture missing alloca"))?
        } else {
            let n = captures.len();
            let env = self.fresh();
            writeln!(self.body, "  {env} = alloca [{n} x ptr], align 8").ok();
            for (i, id) in captures.iter().enumerate() {
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("capture missing alloca"))?;
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
        let saved_caps = std::mem::take(&mut self.reaction_captures);
        let saved_in = self.in_callback;

        self.tmp = 0;
        self.body.clear();
        self.reaction_captures = captures;
        self.in_callback = true;

        for stmt in body {
            self.emit_handler_stmt(stmt)?;
        }

        let mut fn_ir = String::new();
        writeln!(fn_ir, "define void @{fn_name}(ptr %data) {{").ok();
        writeln!(fn_ir, "entry:").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret void").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.reaction_captures = saved_caps;
        self.in_callback = saved_in;
        Ok((fn_name, data_operand))
    }

    fn emit_handler_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr)?;
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_handler_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Return { value } => {
                if let Some(e) = value {
                    let _ = self.emit_expr(e)?;
                }
                Ok(())
            }
            _ => Err(diag("host_signal cb: unsupported stmt")),
        }
    }
}

fn collect_used_locals(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Expr { expr } => collect_used_in_expr(expr, out),
            Stmt::Block { body } => collect_used_locals(body, out),
            Stmt::Return { value } => {
                if let Some(e) = value {
                    collect_used_in_expr(e, out);
                }
            }
            _ => {}
        }
    }
}

fn collect_used_in_expr(expr: &Expr, out: &mut HashSet<LocalId>) {
    match expr {
        Expr::Local { id, .. } => {
            out.insert(*id);
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            value,
            ..
        } => {
            out.insert(*id);
            collect_used_in_expr(value, out);
        }
        Expr::Binary { left, right, .. } => {
            collect_used_in_expr(left, out);
            collect_used_in_expr(right, out);
        }
        Expr::Unary { arg, .. } => collect_used_in_expr(arg, out),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn ir_of(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn classifies_on_signal_sigterm() {
        let m = ir_of(
            r#"
            let fired = 0;
            onSignal("SIGTERM", function () { fired = 1; });
            raiseSignal("SIGTERM");
            let t = typeof onSignal;
            "#,
        );
        assert!(is_host_signal_module(&m));
        let ir = emit_host_signals(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_signal_watch"), "{ir}");
        assert!(ir.contains("draconic_rt_host_signal_raise"), "{ir}");
        assert!(ir.contains("draconic_rt_job_drain"), "{ir}");
    }

    #[test]
    fn classifies_on_signal_sigint() {
        let m = ir_of(
            r#"
            let n = 0;
            onSignal("SIGINT", function () { n = n + 1; });
            raiseSignal("SIGINT");
            "#,
        );
        assert!(is_host_signal_module(&m));
        let ir = emit_host_signals(&m).expect("emit");
        assert!(ir.contains("i32 2,"), "{ir}");
    }

    #[test]
    fn classifies_ignore_and_restore() {
        let m = ir_of(
            r#"
            let fired = 0;
            onSignal("SIGTERM", function () { fired = 1; });
            ignoreSignal("SIGTERM");
            raiseSignal("SIGTERM");
            restoreSignal("SIGTERM");
            onSignal("SIGTERM", function () { fired = 2; });
            raiseSignal("SIGTERM");
            let t_ign = typeof ignoreSignal;
            let t_rest = typeof restoreSignal;
            "#,
        );
        assert!(is_host_signal_module(&m));
        let ir = emit_host_signals(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_signal_ignore"), "{ir}");
        assert!(ir.contains("draconic_rt_host_signal_restore"), "{ir}");
    }
}
