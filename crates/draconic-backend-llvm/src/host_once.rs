//! C03 / C03.01: `makeOnce` + `onceRun` thread-safe init primitive.
//! C03 parent also locks that mutex is not a user Host API (`typeof makeMutex`
//! / `mutexLock` / `mutexUnlock` → `"undefined"`).
//!
//! Supported subset:
//! - `typeof makeOnce` / `typeof onceRun` → `"function"`
//! - `typeof` unresolved ident → `"undefined"` (no public mutex Host API)
//! - `makeOnce()` → handle number >= 1
//! - `onceRun(h)` → 1 first caller / 0 already done / negative invalid
//! - number comparisons and bool locals

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_ONCE_MAKE, HOST_ONCE_RUN, PRINT_BOOL, PRINT_F64, PRINT_STR,
};

pub(crate) fn is_host_once_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_once(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_once module"))?;
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
    uses_make: bool,
    uses_run: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        uses_make: false,
        uses_run: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !(ctx.uses_make || ctx.uses_run) || ctx.print_locals.is_empty() {
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
        Expr::Call { callee, args, .. } if is_named_callee(callee, "makeOnce") => {
            if !args.is_empty() {
                return None;
            }
            ctx.uses_make = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "onceRun") => {
            if args.len() != 1 {
                return None;
            }
            let ty = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ty != SlotTy::Number {
                return None;
            }
            ctx.uses_run = true;
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
            if is_named_ident(arg, "makeOnce") {
                ctx.uses_make = true;
                Some(SlotTy::String)
            } else if is_named_ident(arg, "onceRun") {
                ctx.uses_run = true;
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
            .ok_or_else(|| diag("host_once: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some(g) = self.str_globals.get(s) {
            return g.clone();
        }
        let g = format!(".ho.str.{}", self.str_globals.len());
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
        writeln!(self.out, "; Draconic LLVM host_once (C03.01)").ok();
        let decls = vec![
            GC_INIT,
            PRINT_F64,
            PRINT_STR,
            PRINT_BOOL,
            HOST_ONCE_MAKE,
            HOST_ONCE_RUN,
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
                    .ok_or_else(|| diag("host_once: declare unknown slot"))?;
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
            _ => Err(diag("host_once: unsupported statement")),
        }
    }

    fn emit_make(&mut self) -> Result<String, Diagnostic> {
        let h_i32 = self.fresh();
        let h_f = self.fresh();
        writeln!(
            self.body,
            "  {h_i32} = call i32 @{}()",
            HOST_ONCE_MAKE.symbol
        )
        .ok();
        writeln!(self.body, "  {h_f} = sitofp i32 {h_i32} to double").ok();
        Ok(h_f)
    }

    fn emit_run(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("onceRun handle"))?;
        let h_f = self.emit_number_expr(handle)?;
        let h_i32 = self.fresh();
        let r_i32 = self.fresh();
        let r_f = self.fresh();
        writeln!(self.body, "  {h_i32} = fptosi double {h_f} to i32").ok();
        writeln!(
            self.body,
            "  {r_i32} = call i32 @{}(i32 {h_i32}, ptr null)",
            HOST_ONCE_RUN.symbol
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
            Expr::Call { callee, args, .. } if is_named_callee(callee, "makeOnce") => {
                let _ = args;
                self.emit_make()
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "onceRun") => {
                self.emit_run(args)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_once: expected number expr")),
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
            _ => Err(diag("host_once: expected bool expr")),
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
            } if is_named_ident(arg, "makeOnce") || is_named_ident(arg, "onceRun") => {
                Ok(self.emit_cstr_ptr("function"))
            }
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
            _ => Err(diag("host_once: expected string expr")),
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
    fn classifies_make_and_run() {
        let m = lower_src(
            r#"
            let t = typeof makeOnce;
            let o = makeOnce();
            let a = onceRun(o);
            let b = onceRun(o);
            let ok = a === 1;
            "#,
        );
        assert!(is_host_once_module(&m));
        let ir = emit_host_once(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_once_make"), "{ir}");
        assert!(ir.contains("draconic_rt_host_once_run"), "{ir}");
        assert!(ir.contains("ptr null"), "{ir}");
    }

    #[test]
    fn classifies_typeof_once_run() {
        let m = lower_src(
            r#"
            let u = typeof onceRun;
            let o = makeOnce();
            let r = onceRun(o);
            "#,
        );
        assert!(is_host_once_module(&m));
        let ir = emit_host_once(&m).expect("emit");
        assert!(ir.contains("function"), "{ir}");
    }

    #[test]
    fn classifies_typeof_unresolved_mutex_as_undefined() {
        let m = lower_src(
            r#"
            let t = typeof makeOnce;
            let o = makeOnce();
            let a = onceRun(o);
            let mu = typeof makeMutex;
            let lock = typeof mutexLock;
            let unlock = typeof mutexUnlock;
            "#,
        );
        assert!(is_host_once_module(&m));
        let ir = emit_host_once(&m).expect("emit");
        assert!(ir.contains("undefined"), "{ir}");
        assert!(!ir.contains("draconic_rt_host_internal_mutex"), "{ir}");
    }
}
