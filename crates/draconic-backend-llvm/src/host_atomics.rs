//! C06: shared-memory atomics — `makeSharedMemory` + load/store/add/CAS/
//! wait/notify. Optional `spawnWorker(fn, mem)` + `joinWorker` so a worker
//! isolate sees the same integer buffer (no shared JS heap).
//!
//! Supported subset:
//! - `typeof` the seven Host APIs → `"function"`
//! - `makeSharedMemory(n)` → handle >= 1
//! - `sharedLoad` / `sharedStore` / `sharedAdd` / `sharedCompareExchange`
//! - `sharedWait(h, i, expected, timeoutMs)` / `sharedNotify(h, i)`
//! - `spawnWorker(function (mem) { … }, mem)` inlines worker body then dummy spawn
//! - number comparisons and bool locals

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, LocalId, Module, Pattern, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_SHARED_ADD, HOST_SHARED_CMPXCHG, HOST_SHARED_LOAD,
    HOST_SHARED_MAKE, HOST_SHARED_NOTIFY, HOST_SHARED_STORE, HOST_SHARED_WAIT, HOST_WORKER_JOIN,
    HOST_WORKER_SPAWN, PRINT_BOOL, PRINT_F64, PRINT_STR,
};

pub(crate) fn is_host_atomics_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_atomics(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_atomics module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    Bool,
    String,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_top: bool,
    uses_shared: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        print_top: true,
        uses_shared: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.uses_shared || ctx.print_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            if ctx.print_top && matches!(ty, SlotTy::Number | SlotTy::Bool | SlotTy::String) {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => {
            let _ = classify_expr(expr, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_worker_fn(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    let Expr::Function { params, body, .. } = expr else {
        return None;
    };
    if params.len() != 1 {
        return None;
    }
    let Pattern::Local(pid) = &params[0].pattern else {
        return None;
    };
    ctx.slots.push((*pid, SlotTy::Number));
    ctx.slot_of.insert(*pid, SlotTy::Number);
    let prev = ctx.print_top;
    ctx.print_top = false;
    for stmt in body {
        classify_stmt(stmt, ctx)?;
    }
    ctx.print_top = prev;
    Some(())
}

fn classify_number_args(args: &[Arg], n: usize, ctx: &mut ClassifyCtx) -> Option<()> {
    if args.len() != n {
        return None;
    }
    for arg in args {
        if classify_expr(arg_expr(arg)?, ctx)? != SlotTy::Number {
            return None;
        }
    }
    Some(())
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. } if is_named_callee(callee, "makeSharedMemory") => {
            classify_number_args(args, 1, ctx)?;
            ctx.uses_shared = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedLoad") => {
            classify_number_args(args, 2, ctx)?;
            ctx.uses_shared = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedStore") => {
            classify_number_args(args, 3, ctx)?;
            ctx.uses_shared = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedAdd") => {
            classify_number_args(args, 3, ctx)?;
            ctx.uses_shared = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedCompareExchange") => {
            classify_number_args(args, 4, ctx)?;
            ctx.uses_shared = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedWait") => {
            classify_number_args(args, 4, ctx)?;
            ctx.uses_shared = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedNotify") => {
            classify_number_args(args, 2, ctx)?;
            ctx.uses_shared = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "spawnWorker") => {
            if args.len() != 2 {
                return None;
            }
            classify_worker_fn(arg_expr(&args[0])?, ctx)?;
            if classify_expr(arg_expr(&args[1])?, ctx)? != SlotTy::Number {
                return None;
            }
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "joinWorker") => {
            classify_number_args(args, 1, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Binary {
            op:
                BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq,
            left,
            right,
            ..
        } => {
            let lt = classify_expr(left, ctx)?;
            let rt = classify_expr(right, ctx)?;
            if lt == SlotTy::Number && rt == SlotTy::Number {
                Some(SlotTy::Bool)
            } else {
                None
            }
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            if is_shared_ident(arg) {
                ctx.uses_shared = true;
                Some(SlotTy::String)
            } else if matches!(arg.as_ref(), Expr::IdentName { .. }) {
                Some(SlotTy::String)
            } else {
                let _ = classify_expr(arg, ctx)?;
                Some(SlotTy::String)
            }
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Boolean { .. } => Some(SlotTy::Bool),
        _ => None,
    }
}

fn is_shared_ident(expr: &Expr) -> bool {
    is_named_ident(expr, "makeSharedMemory")
        || is_named_ident(expr, "sharedLoad")
        || is_named_ident(expr, "sharedStore")
        || is_named_ident(expr, "sharedAdd")
        || is_named_ident(expr, "sharedCompareExchange")
        || is_named_ident(expr, "sharedWait")
        || is_named_ident(expr, "sharedNotify")
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn is_named_ident(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn arg_expr(arg: &Arg) -> Option<&Expr> {
    match arg {
        Arg::Expr(e) => Some(e),
        Arg::Spread(_) => None,
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) && c != b'"' => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    slot_of: HashMap<LocalId, SlotTy>,
    body: String,
    out: String,
    next_tmp: u32,
    str_globals: HashMap<String, String>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        let slot_of: HashMap<LocalId, SlotTy> = info.slots.iter().copied().collect();
        Self {
            module,
            info,
            slot_of,
            body: String::new(),
            out: String::new(),
            next_tmp: 0,
            str_globals: HashMap::new(),
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let n = self.next_tmp;
        self.next_tmp += 1;
        format!("%t{n}")
    }

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        Ok(format!("%s{}", id.0))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some(g) = self.str_globals.get(s) {
            return g.clone();
        }
        let g = format!(".ha.str.{}", self.str_globals.len());
        self.str_globals.insert(s.to_string(), g.clone());
        g
    }

    fn emit_cstr_ptr(&mut self, s: &str) -> String {
        let g = self.intern_cstr(s);
        let n = s.len() + 1;
        let p = self.fresh();
        writeln!(
            self.body,
            "  {p} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0"
        )
        .ok();
        p
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(self.out, "; Draconic LLVM host_atomics (C06)").ok();
        let decls = vec![
            GC_INIT,
            PRINT_F64,
            PRINT_STR,
            PRINT_BOOL,
            HOST_SHARED_MAKE,
            HOST_SHARED_LOAD,
            HOST_SHARED_STORE,
            HOST_SHARED_ADD,
            HOST_SHARED_CMPXCHG,
            HOST_SHARED_WAIT,
            HOST_SHARED_NOTIFY,
            HOST_WORKER_SPAWN,
            HOST_WORKER_JOIN,
        ];
        self.out.push_str(&llvm_declares(&decls));
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            let ptr = self.slot_ptr(*id)?;
            let llvm_ty = match ty {
                SlotTy::Number => "double",
                SlotTy::Bool => "i8",
                SlotTy::String => "ptr",
            };
            writeln!(self.body, "  {ptr} = alloca {llvm_ty}, align 8").ok();
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, kind) in &self.info.print_locals {
            let ptr = self.slot_ptr(*id)?;
            match kind {
                SlotTy::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::Bool => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
                }
                SlotTy::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
            }
        }

        let body = std::mem::take(&mut self.body);
        for (content, gname) in &self.str_globals {
            let n = content.len() + 1;
            let esc = escape_llvm_string(content);
            writeln!(
                self.out,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        if !self.str_globals.is_empty() {
            writeln!(self.out).ok();
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        writeln!(self.out, "  {}", GC_INIT.call("")).ok();
        self.out.push_str(&body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                let kind = *self
                    .slot_of
                    .get(local)
                    .ok_or_else(|| diag("host_atomics: declare unknown slot"))?;
                let ptr = self.slot_ptr(*local)?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Bool => {
                        let v = self.emit_bool_expr(init)?;
                        writeln!(self.body, "  store i8 {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => {
                let _ = self.emit_number_expr(expr)?;
                Ok(())
            }
            _ => Err(diag("host_atomics: unsupported statement")),
        }
    }

    fn emit_i32(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let f = self.emit_number_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {f} to i32").ok();
        Ok(i)
    }

    fn emit_i32_result(sym: &str, args: String, em: &mut Emitter) -> String {
        let r_i32 = em.fresh();
        let r_f = em.fresh();
        writeln!(em.body, "  {r_i32} = call i32 @{sym}({args})").ok();
        writeln!(em.body, "  {r_f} = sitofp i32 {r_i32} to double").ok();
        r_f
    }

    fn emit_make(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let n = arg_expr(&args[0]).ok_or_else(|| diag("makeSharedMemory len"))?;
        let n_i32 = self.emit_i32(n)?;
        Ok(Self::emit_i32_result(
            HOST_SHARED_MAKE.symbol,
            format!("i32 {n_i32}"),
            self,
        ))
    }

    fn emit_load(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let h = arg_expr(&args[0]).ok_or_else(|| diag("sharedLoad handle"))?;
        let i = arg_expr(&args[1]).ok_or_else(|| diag("sharedLoad index"))?;
        let h_i32 = self.emit_i32(h)?;
        let i_i32 = self.emit_i32(i)?;
        Ok(Self::emit_i32_result(
            HOST_SHARED_LOAD.symbol,
            format!("i32 {h_i32}, i32 {i_i32}"),
            self,
        ))
    }

    fn emit_store(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let h = arg_expr(&args[0]).ok_or_else(|| diag("sharedStore handle"))?;
        let i = arg_expr(&args[1]).ok_or_else(|| diag("sharedStore index"))?;
        let v = arg_expr(&args[2]).ok_or_else(|| diag("sharedStore value"))?;
        let h_i32 = self.emit_i32(h)?;
        let i_i32 = self.emit_i32(i)?;
        let v_i32 = self.emit_i32(v)?;
        Ok(Self::emit_i32_result(
            HOST_SHARED_STORE.symbol,
            format!("i32 {h_i32}, i32 {i_i32}, i32 {v_i32}"),
            self,
        ))
    }

    fn emit_add(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let h = arg_expr(&args[0]).ok_or_else(|| diag("sharedAdd handle"))?;
        let i = arg_expr(&args[1]).ok_or_else(|| diag("sharedAdd index"))?;
        let d = arg_expr(&args[2]).ok_or_else(|| diag("sharedAdd delta"))?;
        let h_i32 = self.emit_i32(h)?;
        let i_i32 = self.emit_i32(i)?;
        let d_i32 = self.emit_i32(d)?;
        Ok(Self::emit_i32_result(
            HOST_SHARED_ADD.symbol,
            format!("i32 {h_i32}, i32 {i_i32}, i32 {d_i32}"),
            self,
        ))
    }

    fn emit_cas(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let h = arg_expr(&args[0]).ok_or_else(|| diag("sharedCompareExchange handle"))?;
        let i = arg_expr(&args[1]).ok_or_else(|| diag("sharedCompareExchange index"))?;
        let e = arg_expr(&args[2]).ok_or_else(|| diag("sharedCompareExchange expected"))?;
        let r = arg_expr(&args[3]).ok_or_else(|| diag("sharedCompareExchange replacement"))?;
        let h_i32 = self.emit_i32(h)?;
        let i_i32 = self.emit_i32(i)?;
        let e_i32 = self.emit_i32(e)?;
        let r_i32 = self.emit_i32(r)?;
        Ok(Self::emit_i32_result(
            HOST_SHARED_CMPXCHG.symbol,
            format!("i32 {h_i32}, i32 {i_i32}, i32 {e_i32}, i32 {r_i32}"),
            self,
        ))
    }

    fn emit_wait(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let h = arg_expr(&args[0]).ok_or_else(|| diag("sharedWait handle"))?;
        let i = arg_expr(&args[1]).ok_or_else(|| diag("sharedWait index"))?;
        let e = arg_expr(&args[2]).ok_or_else(|| diag("sharedWait expected"))?;
        let t = arg_expr(&args[3]).ok_or_else(|| diag("sharedWait timeout"))?;
        let h_i32 = self.emit_i32(h)?;
        let i_i32 = self.emit_i32(i)?;
        let e_i32 = self.emit_i32(e)?;
        let t_f = self.emit_number_expr(t)?;
        Ok(Self::emit_i32_result(
            HOST_SHARED_WAIT.symbol,
            format!("i32 {h_i32}, i32 {i_i32}, i32 {e_i32}, double {t_f}"),
            self,
        ))
    }

    fn emit_notify(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let h = arg_expr(&args[0]).ok_or_else(|| diag("sharedNotify handle"))?;
        let i = arg_expr(&args[1]).ok_or_else(|| diag("sharedNotify index"))?;
        let h_i32 = self.emit_i32(h)?;
        let i_i32 = self.emit_i32(i)?;
        Ok(Self::emit_i32_result(
            HOST_SHARED_NOTIFY.symbol,
            format!("i32 {h_i32}, i32 {i_i32}"),
            self,
        ))
    }

    fn emit_spawn(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let entry = arg_expr(&args[0]).ok_or_else(|| diag("spawnWorker entry"))?;
        let mem = arg_expr(&args[1]).ok_or_else(|| diag("spawnWorker shared mem"))?;
        let mem_f = self.emit_number_expr(mem)?;
        let Expr::Function { params, body, .. } = entry else {
            return Err(diag("spawnWorker entry must be function"));
        };
        let Pattern::Local(pid) = &params[0].pattern else {
            return Err(diag("spawnWorker fn needs ident param"));
        };
        let pptr = self.slot_ptr(*pid)?;
        writeln!(self.body, "  store double {mem_f}, ptr {pptr}").ok();
        for stmt in body {
            self.emit_stmt(stmt)?;
        }
        Ok(Self::emit_i32_result(
            HOST_WORKER_SPAWN.symbol,
            "i32 0, ptr null".to_string(),
            self,
        ))
    }

    fn emit_join(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("joinWorker handle"))?;
        let h_i32 = self.emit_i32(handle)?;
        Ok(Self::emit_i32_result(
            HOST_WORKER_JOIN.symbol,
            format!("i32 {h_i32}"),
            self,
        ))
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let v = self.fresh();
                let n: f64 = raw.parse().unwrap_or(0.0);
                let lit = if n.fract() == 0.0 {
                    format!("{n:.1}")
                } else {
                    format!("{n}")
                };
                writeln!(self.body, "  {v} = fadd double {lit}, 0.0").ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "makeSharedMemory") => {
                self.emit_make(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedLoad") => {
                self.emit_load(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedStore") => {
                self.emit_store(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedAdd") => {
                self.emit_add(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedCompareExchange") => {
                self.emit_cas(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedWait") => {
                self.emit_wait(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "sharedNotify") => {
                self.emit_notify(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "spawnWorker") => {
                self.emit_spawn(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "joinWorker") => {
                self.emit_join(args)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_atomics: expected number expr")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Boolean { value, .. } => {
                let v = self.fresh();
                let b = if *value { 1 } else { 0 };
                writeln!(self.body, "  {v} = add i8 {b}, 0").ok();
                Ok(v)
            }
            Expr::Binary {
                op, left, right, ..
            } if matches!(
                op,
                BinaryOp::Gt
                    | BinaryOp::GtEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
                    | BinaryOp::EqEq
                    | BinaryOp::NotEq
            ) =>
            {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let cmp = self.fresh();
                let pred = match op {
                    BinaryOp::Gt => "ogt",
                    BinaryOp::GtEq => "oge",
                    BinaryOp::Lt => "olt",
                    BinaryOp::LtEq => "ole",
                    BinaryOp::EqEqEq | BinaryOp::EqEq => "oeq",
                    BinaryOp::NotEqEq | BinaryOp::NotEq => "one",
                    _ => unreachable!(),
                };
                writeln!(self.body, "  {cmp} = fcmp {pred} double {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {cmp} to i8").ok();
                Ok(b)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_atomics: expected bool expr")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                Ok(self.emit_cstr_ptr(&s))
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } if is_shared_ident(arg) => Ok(self.emit_cstr_ptr("function")),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } if matches!(arg.as_ref(), Expr::IdentName { .. }) => {
                Ok(self.emit_cstr_ptr("undefined"))
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_atomics: expected string expr")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn lower_src(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn classifies_load_store_add_cas() {
        let m = lower_src(
            r#"
            let mem = makeSharedMemory(2);
            let st = sharedStore(mem, 0, 7);
            let v = sharedLoad(mem, 0);
            let old = sharedAdd(mem, 0, 3);
            let cas = sharedCompareExchange(mem, 0, 10, 42);
            "#,
        );
        assert!(is_host_atomics_module(&m));
        let ir = emit_host_atomics(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_shared_make"), "{ir}");
        assert!(ir.contains("draconic_rt_host_shared_store"), "{ir}");
        assert!(ir.contains("draconic_rt_host_shared_load"), "{ir}");
        assert!(ir.contains("draconic_rt_host_shared_add"), "{ir}");
        assert!(ir.contains("draconic_rt_host_shared_cmpxchg"), "{ir}");
    }

    #[test]
    fn classifies_wait_notify_and_worker() {
        let m = lower_src(
            r#"
            let mem = makeSharedMemory(1);
            let h = spawnWorker(function (mem) {
              sharedStore(mem, 0, 1);
              sharedNotify(mem, 0);
            }, mem);
            let w = sharedWait(mem, 0, 0, 1000);
            let joined = joinWorker(h);
            "#,
        );
        assert!(is_host_atomics_module(&m));
        let ir = emit_host_atomics(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_shared_wait"), "{ir}");
        assert!(ir.contains("draconic_rt_host_shared_notify"), "{ir}");
        assert!(ir.contains("draconic_rt_host_worker_spawn"), "{ir}");
        assert!(ir.contains("draconic_rt_host_worker_join"), "{ir}");
    }
}
