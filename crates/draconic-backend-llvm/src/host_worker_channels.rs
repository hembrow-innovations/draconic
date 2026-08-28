//! C02.04: `spawnWorker(fn, ch)` + parent/worker `makeChannel` / `channelSend` /
//! `channelRecv` number FIFO (worker body inlined; OS threads back spawn/join).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::BinaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, LocalId, Module, Pattern, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_CHANNEL_MAKE, HOST_CHANNEL_RECV_F64, HOST_CHANNEL_SEND_F64,
    HOST_WORKER_JOIN, HOST_WORKER_SPAWN, PRINT_BOOL, PRINT_F64,
};

pub(crate) fn is_host_worker_channels_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_worker_channels(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_worker_channels module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    Bool,
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
    uses_spawn: bool,
    uses_join: bool,
    uses_channel: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        print_top: true,
        uses_spawn: false,
        uses_join: false,
        uses_channel: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !(ctx.uses_spawn && ctx.uses_channel && ctx.uses_join) || ctx.print_locals.is_empty() {
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
            if ctx.print_top {
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

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. } if is_named_callee(callee, "makeChannel") => {
            if !args.is_empty() {
                return None;
            }
            ctx.uses_channel = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "channelSend") => {
            if args.len() != 2 {
                return None;
            }
            if classify_expr(arg_expr(&args[0])?, ctx)? != SlotTy::Number {
                return None;
            }
            if classify_expr(arg_expr(&args[1])?, ctx)? != SlotTy::Number {
                return None;
            }
            ctx.uses_channel = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "channelRecv") => {
            if args.len() != 1 {
                return None;
            }
            if classify_expr(arg_expr(&args[0])?, ctx)? != SlotTy::Number {
                return None;
            }
            ctx.uses_channel = true;
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
            ctx.uses_spawn = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "joinWorker") => {
            if args.len() != 1 {
                return None;
            }
            if classify_expr(arg_expr(&args[0])?, ctx)? != SlotTy::Number {
                return None;
            }
            ctx.uses_join = true;
            Some(SlotTy::Number)
        }
        Expr::Binary {
            op: BinaryOp::Add,
            left,
            right,
            ..
        } => {
            let lt = classify_expr(left, ctx)?;
            let rt = classify_expr(right, ctx)?;
            if lt == SlotTy::Number && rt == SlotTy::Number {
                Some(SlotTy::Number)
            } else {
                None
            }
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
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::Boolean { .. } => Some(SlotTy::Bool),
        _ => None,
    }
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
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

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    slot_of: HashMap<LocalId, SlotTy>,
    body: String,
    out: String,
    next_tmp: u32,
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

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM host_worker_channels (C02.04)"
        )
        .ok();
        let decls = vec![
            GC_INIT,
            PRINT_F64,
            PRINT_BOOL,
            HOST_WORKER_SPAWN,
            HOST_WORKER_JOIN,
            HOST_CHANNEL_MAKE,
            HOST_CHANNEL_SEND_F64,
            HOST_CHANNEL_RECV_F64,
        ];
        self.out.push_str(&llvm_declares(&decls));
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            let ptr = self.slot_ptr(*id)?;
            let llvm_ty = match ty {
                SlotTy::Number => "double",
                SlotTy::Bool => "i8",
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
            }
        }

        let body = std::mem::take(&mut self.body);
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
                    .ok_or_else(|| diag("host_worker_channels: declare unknown slot"))?;
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
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => {
                let _ = self.emit_number_expr(expr)?;
                Ok(())
            }
            _ => Err(diag("host_worker_channels: unsupported statement")),
        }
    }

    fn emit_handle_i32(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let h_f = self.emit_number_expr(expr)?;
        let h_i32 = self.fresh();
        writeln!(self.body, "  {h_i32} = fptosi double {h_f} to i32").ok();
        Ok(h_i32)
    }

    fn emit_spawn(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let entry = arg_expr(&args[0]).ok_or_else(|| diag("spawnWorker entry"))?;
        let ch = arg_expr(&args[1]).ok_or_else(|| diag("spawnWorker channel"))?;
        let ch_f = self.emit_number_expr(ch)?;
        let Expr::Function { params, body, .. } = entry else {
            return Err(diag("spawnWorker entry must be function"));
        };
        let Pattern::Local(pid) = &params[0].pattern else {
            return Err(diag("spawnWorker fn needs ident param"));
        };
        let pptr = self.slot_ptr(*pid)?;
        writeln!(self.body, "  store double {ch_f}, ptr {pptr}").ok();
        for stmt in body {
            self.emit_stmt(stmt)?;
        }
        let h_i32 = self.fresh();
        let h_f = self.fresh();
        writeln!(
            self.body,
            "  {h_i32} = call i32 @{}(i32 0, ptr null)",
            HOST_WORKER_SPAWN.symbol
        )
        .ok();
        writeln!(self.body, "  {h_f} = sitofp i32 {h_i32} to double").ok();
        Ok(h_f)
    }

    fn emit_join(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("joinWorker handle"))?;
        let h_f = self.emit_number_expr(handle)?;
        let h_i32 = self.fresh();
        let r_i32 = self.fresh();
        let r_f = self.fresh();
        writeln!(self.body, "  {h_i32} = fptosi double {h_f} to i32").ok();
        writeln!(
            self.body,
            "  {r_i32} = call i32 @{}(i32 {h_i32})",
            HOST_WORKER_JOIN.symbol
        )
        .ok();
        writeln!(self.body, "  {r_f} = sitofp i32 {r_i32} to double").ok();
        Ok(r_f)
    }

    fn emit_make(&mut self) -> Result<String, Diagnostic> {
        let z = self.fresh();
        let h_i32 = self.fresh();
        let h_f = self.fresh();
        writeln!(self.body, "  {z} = add i32 0, 0").ok();
        writeln!(
            self.body,
            "  {h_i32} = call i32 @{}(i32 {z})",
            HOST_CHANNEL_MAKE.symbol
        )
        .ok();
        writeln!(self.body, "  {h_f} = sitofp i32 {h_i32} to double").ok();
        Ok(h_f)
    }

    fn emit_send(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("channelSend handle"))?;
        let value = arg_expr(&args[1]).ok_or_else(|| diag("channelSend value"))?;
        let h_i32 = self.emit_handle_i32(handle)?;
        let v = self.emit_number_expr(value)?;
        let r_i32 = self.fresh();
        let r_f = self.fresh();
        writeln!(
            self.body,
            "  {r_i32} = call i32 @{}(i32 {h_i32}, double {v})",
            HOST_CHANNEL_SEND_F64.symbol
        )
        .ok();
        writeln!(self.body, "  {r_f} = sitofp i32 {r_i32} to double").ok();
        Ok(r_f)
    }

    fn emit_recv(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("channelRecv handle"))?;
        let h_i32 = self.emit_handle_i32(handle)?;
        let tmp = self.fresh();
        let st = self.fresh();
        let v = self.fresh();
        writeln!(self.body, "  {tmp} = alloca double, align 8").ok();
        writeln!(
            self.body,
            "  {st} = call i32 @{}(i32 {h_i32}, ptr {tmp})",
            HOST_CHANNEL_RECV_F64.symbol
        )
        .ok();
        let _ = st;
        writeln!(self.body, "  {v} = load double, ptr {tmp}").ok();
        Ok(v)
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
            Expr::Call { callee, args, .. } if is_named_callee(callee, "makeChannel") => {
                self.emit_make()
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "channelSend") => {
                self.emit_send(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "channelRecv") => {
                self.emit_recv(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "spawnWorker") => {
                self.emit_spawn(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "joinWorker") => {
                self.emit_join(args)
            }
            Expr::Binary {
                op: BinaryOp::Add,
                left,
                right,
                ..
            } => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = fadd double {l}, {r}").ok();
                Ok(v)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_worker_channels: expected number expr")),
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
                op,
                left,
                right,
                ..
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
            _ => Err(diag("host_worker_channels: expected bool expr")),
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
    fn classifies_worker_channel_e2e() {
        let m = lower_src(
            r#"
            let ch = makeChannel();
            let sentIn = channelSend(ch, 7);
            let h = spawnWorker(function (ch) {
              let x = channelRecv(ch);
              channelSend(ch, x + 1);
            }, ch);
            let joined = joinWorker(h);
            let v = channelRecv(ch);
            let ok = v === 8;
            "#,
        );
        assert!(is_host_worker_channels_module(&m));
        let ir = emit_host_worker_channels(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_channel_make"), "{ir}");
        assert!(ir.contains("draconic_rt_host_channel_send_f64"), "{ir}");
        assert!(ir.contains("draconic_rt_host_channel_recv_f64"), "{ir}");
        assert!(ir.contains("draconic_rt_host_worker_spawn"), "{ir}");
        assert!(ir.contains("draconic_rt_host_worker_join"), "{ir}");
        assert!(ir.contains("fadd double"), "{ir}");
    }
}
