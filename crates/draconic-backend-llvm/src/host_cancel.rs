//! C05.01: `makeCancelToken` + `cancelTokenAbort` + `cancelTokenAborted` +
//! `cancelTokenLink`.
//!
//! Supported subset:
//! - `typeof` on the four host APIs → `"function"`
//! - `makeCancelToken()` → handle number >= 1
//! - `cancelTokenAbort(h)` / `cancelTokenAborted(h)` → 0 / 1 / -1
//! - `cancelTokenLink(child, parent)` → 0 / -1
//! - number comparisons and bool locals

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_CANCEL_ABORT, HOST_CANCEL_ABORTED, HOST_CANCEL_LINK,
    HOST_CANCEL_MAKE, PRINT_BOOL, PRINT_F64, PRINT_STR,
};

pub(crate) fn is_host_cancel_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_cancel(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_cancel module"))?;
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
    uses_cancel: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        uses_cancel: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.uses_cancel || ctx.print_locals.is_empty() {
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
            if matches!(ty, SlotTy::Number | SlotTy::Bool | SlotTy::String) {
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

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. } if is_named_callee(callee, "makeCancelToken") => {
            if !args.is_empty() {
                return None;
            }
            ctx.uses_cancel = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "cancelTokenAbort") => {
            if args.len() != 1 {
                return None;
            }
            let ty = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ty != SlotTy::Number {
                return None;
            }
            ctx.uses_cancel = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "cancelTokenAborted") => {
            if args.len() != 1 {
                return None;
            }
            let ty = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ty != SlotTy::Number {
                return None;
            }
            ctx.uses_cancel = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "cancelTokenLink") => {
            if args.len() != 2 {
                return None;
            }
            let a = classify_expr(arg_expr(&args[0])?, ctx)?;
            let b = classify_expr(arg_expr(&args[1])?, ctx)?;
            if a != SlotTy::Number || b != SlotTy::Number {
                return None;
            }
            ctx.uses_cancel = true;
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
            if is_cancel_host_ident(arg) {
                ctx.uses_cancel = true;
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

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn is_named_ident(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn is_cancel_host_ident(expr: &Expr) -> bool {
    is_named_ident(expr, "makeCancelToken")
        || is_named_ident(expr, "cancelTokenAbort")
        || is_named_ident(expr, "cancelTokenAborted")
        || is_named_ident(expr, "cancelTokenLink")
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
    by_id: HashMap<LocalId, &'a Local>,
    slot_of: HashMap<LocalId, SlotTy>,
    body: String,
    out: String,
    next_tmp: u32,
    str_globals: HashMap<String, String>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
        let slot_of: HashMap<LocalId, SlotTy> = info.slots.iter().copied().collect();
        Self {
            module,
            info,
            by_id,
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
        let name = self
            .by_id
            .get(&id)
            .map(|l| l.name.as_str())
            .ok_or_else(|| diag("host_cancel: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some(g) = self.str_globals.get(s) {
            return g.clone();
        }
        let g = format!(".hc.str.{}", self.str_globals.len());
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
        writeln!(self.out, "; Draconic LLVM host_cancel (C05.01)").ok();
        let decls = vec![
            GC_INIT,
            PRINT_F64,
            PRINT_STR,
            PRINT_BOOL,
            HOST_CANCEL_MAKE,
            HOST_CANCEL_ABORT,
            HOST_CANCEL_ABORTED,
            HOST_CANCEL_LINK,
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
                    .ok_or_else(|| diag("host_cancel: declare unknown slot"))?;
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
            _ => Err(diag("host_cancel: unsupported statement")),
        }
    }

    fn emit_i32_call0(&mut self, symbol: &str) -> Result<String, Diagnostic> {
        let h_i32 = self.fresh();
        let h_f = self.fresh();
        writeln!(self.body, "  {h_i32} = call i32 @{symbol}()").ok();
        writeln!(self.body, "  {h_f} = sitofp i32 {h_i32} to double").ok();
        Ok(h_f)
    }

    fn emit_i32_call1(&mut self, symbol: &str, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("cancel token handle"))?;
        let h_f = self.emit_number_expr(handle)?;
        let h_i32 = self.fresh();
        let r_i32 = self.fresh();
        let r_f = self.fresh();
        writeln!(self.body, "  {h_i32} = fptosi double {h_f} to i32").ok();
        writeln!(self.body, "  {r_i32} = call i32 @{symbol}(i32 {h_i32})").ok();
        writeln!(self.body, "  {r_f} = sitofp i32 {r_i32} to double").ok();
        Ok(r_f)
    }

    fn emit_link(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let child = arg_expr(&args[0]).ok_or_else(|| diag("cancelTokenLink child"))?;
        let parent = arg_expr(&args[1]).ok_or_else(|| diag("cancelTokenLink parent"))?;
        let c_f = self.emit_number_expr(child)?;
        let p_f = self.emit_number_expr(parent)?;
        let c_i32 = self.fresh();
        let p_i32 = self.fresh();
        let r_i32 = self.fresh();
        let r_f = self.fresh();
        writeln!(self.body, "  {c_i32} = fptosi double {c_f} to i32").ok();
        writeln!(self.body, "  {p_i32} = fptosi double {p_f} to i32").ok();
        writeln!(
            self.body,
            "  {r_i32} = call i32 @{}(i32 {c_i32}, i32 {p_i32})",
            HOST_CANCEL_LINK.symbol
        )
        .ok();
        writeln!(self.body, "  {r_f} = sitofp i32 {r_i32} to double").ok();
        Ok(r_f)
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
            Expr::Call { callee, args, .. } if is_named_callee(callee, "makeCancelToken") => {
                let _ = args;
                self.emit_i32_call0(HOST_CANCEL_MAKE.symbol)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "cancelTokenAbort") => {
                self.emit_i32_call1(HOST_CANCEL_ABORT.symbol, args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "cancelTokenAborted") => {
                self.emit_i32_call1(HOST_CANCEL_ABORTED.symbol, args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "cancelTokenLink") => {
                self.emit_link(args)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_cancel: expected number expr")),
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
            _ => Err(diag("host_cancel: expected bool expr")),
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
            } if is_cancel_host_ident(arg) => Ok(self.emit_cstr_ptr("function")),
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_cancel: expected string expr")),
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
    fn classifies_make_abort_aborted() {
        let m = lower_src(
            r#"
            let t = typeof makeCancelToken;
            let tok = makeCancelToken();
            let before = cancelTokenAborted(tok);
            let abortOk = cancelTokenAbort(tok);
            let after = cancelTokenAborted(tok);
            "#,
        );
        assert!(is_host_cancel_module(&m));
        let ir = emit_host_cancel(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_cancel_make"), "{ir}");
        assert!(ir.contains("draconic_rt_host_cancel_abort"), "{ir}");
        assert!(ir.contains("draconic_rt_host_cancel_aborted"), "{ir}");
    }

    #[test]
    fn classifies_link() {
        let m = lower_src(
            r#"
            let parent = makeCancelToken();
            let child = makeCancelToken();
            let linked = cancelTokenLink(child, parent);
            cancelTokenAbort(parent);
            let childAborted = cancelTokenAborted(child);
            "#,
        );
        assert!(is_host_cancel_module(&m));
        let ir = emit_host_cancel(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_cancel_link"), "{ir}");
    }
}
