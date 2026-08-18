//! H05.01–H05.02: native clocks — `nowMs()` / `Date.now()` / `monotonicMs()` via Runtime ABI.
//!
//! Calls `draconic_rt_host_now_ms` / `draconic_rt_host_monotonic_ms` at run time
//! (not compile-time fold). Prints string (`typeof`) and bool locals; number
//! locals used only in comparisons.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_MONOTONIC_MS, HOST_NOW_MS, PRINT_BOOL, PRINT_STR,
};

pub(crate) fn is_host_time_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_time(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_time module"))?;
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

struct ClassifyCtx<'a> {
    module: &'a Module,
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    has_now: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        module,
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_now: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_now || ctx.print_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            if matches!(ty, SlotTy::Bool | SlotTy::String) {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx<'_>) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. } if args.is_empty() && is_named_callee(callee, "nowMs") => {
            ctx.has_now = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "monotonicMs") =>
        {
            ctx.has_now = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_date_now_callee(callee, ctx.module) =>
        {
            ctx.has_now = true;
            Some(SlotTy::Number)
        }
        Expr::Binary {
            op: BinaryOp::Gt | BinaryOp::GtEq | BinaryOp::Lt | BinaryOp::LtEq,
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
        Expr::Binary {
            op: BinaryOp::Sub | BinaryOp::Add,
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
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            let _ = classify_expr(arg, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(SlotTy::Number),
        _ => None,
    }
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn local_name(module: &Module, id: LocalId) -> Option<&str> {
    module
        .locals
        .iter()
        .find(|l| l.id == id)
        .map(|l| l.name.as_str())
}

fn is_date_now_callee(expr: &Expr, module: &Module) -> bool {
    match expr {
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let is_date = match object.as_ref() {
                Expr::IdentName { name, .. } => name == "Date",
                Expr::Local { id, .. } => local_name(module, *id) == Some("Date"),
                _ => false,
            };
            is_date && string_lit(property).as_deref() == Some("now")
        }
        _ => false,
    }
}

fn string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy()),
        _ => None,
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
    out: String,
    body: String,
    next_tmp: usize,
    str_globals: Vec<(String, String)>,
    local_name: HashMap<LocalId, String>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        let mut local_name = HashMap::new();
        for Local { id, name, .. } in &module.locals {
            local_name.insert(*id, name.clone());
        }
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            next_tmp: 0,
            str_globals: Vec::new(),
            local_name,
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
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_time: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.time.{}", self.str_globals.len());
        self.str_globals.push((s.to_string(), g.clone()));
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
        writeln!(
            self.out,
            "; Draconic LLVM host_time (H05.01–H05.02 nowMs / Date.now / monotonicMs)"
        )
        .ok();
        self.out.push_str(&llvm_declares(&[
            GC_INIT,
            PRINT_STR,
            PRINT_BOOL,
            HOST_NOW_MS,
            HOST_MONOTONIC_MS,
        ]));
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
                SlotTy::Number => {}
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
                let init = init
                    .as_ref()
                    .ok_or_else(|| diag("host_time: declare needs init"))?;
                let ptr = self.slot_ptr(*local)?;
                let ty = self
                    .info
                    .slots
                    .iter()
                    .find(|(id, _)| id == local)
                    .map(|(_, t)| *t)
                    .ok_or_else(|| diag("host_time: unknown slot"))?;
                match ty {
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
            _ => Err(diag("host_time: unsupported stmt")),
        }
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
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "nowMs") =>
            {
                let v = self.fresh();
                writeln!(self.body, "  {}", HOST_NOW_MS.call_to(&v, "")).ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "monotonicMs") =>
            {
                let v = self.fresh();
                writeln!(self.body, "  {}", HOST_MONOTONIC_MS.call_to(&v, "")).ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_date_now_callee(callee, self.module) =>
            {
                let v = self.fresh();
                writeln!(self.body, "  {}", HOST_NOW_MS.call_to(&v, "")).ok();
                Ok(v)
            }
            Expr::Binary {
                op: BinaryOp::Sub,
                left,
                right,
                ..
            } => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = fsub double {l}, {r}").ok();
                Ok(v)
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
            _ => Err(diag("host_time: expected number expr")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Binary {
                op,
                left,
                right,
                ..
            } if matches!(
                op,
                BinaryOp::Gt | BinaryOp::GtEq | BinaryOp::Lt | BinaryOp::LtEq
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
            _ => Err(diag("host_time: expected bool expr")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => self.emit_typeof(arg),
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_time: expected string expr")),
        }
    }

    fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        match arg {
            Expr::Call { callee, args, .. }
                if args.is_empty()
                    && (is_named_callee(callee, "nowMs")
                        || is_named_callee(callee, "monotonicMs")
                        || is_date_now_callee(callee, self.module)) =>
            {
                Ok(self.emit_cstr_ptr("number"))
            }
            Expr::Local { id, .. } => {
                let ty = self
                    .info
                    .slots
                    .iter()
                    .find(|(i, _)| i == id)
                    .map(|(_, t)| *t)
                    .ok_or_else(|| diag("host_time: typeof unknown local"))?;
                let s = match ty {
                    SlotTy::Number => "number",
                    SlotTy::Bool => "boolean",
                    SlotTy::String => "string",
                };
                Ok(self.emit_cstr_ptr(s))
            }
            _ => Err(diag("host_time: typeof unsupported arg")),
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
    fn classifies_now_ms() {
        let m = lower_src(
            r#"
            let t = typeof nowMs();
            let a = nowMs();
            let b = nowMs();
            let ok_range = a > 1600000000000;
            let ok_order = b >= a;
            "#,
        );
        assert!(is_host_time_module(&m));
        let ir = emit_host_time(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_now_ms"), "{ir}");
    }

    #[test]
    fn classifies_date_now() {
        let m = lower_src(
            r#"
            let t = typeof Date.now();
            let a = Date.now();
            let ok = a > 1600000000000;
            "#,
        );
        assert!(is_host_time_module(&m));
        let ir = emit_host_time(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_now_ms"), "{ir}");
    }

    #[test]
    fn classifies_monotonic_ms() {
        let m = lower_src(
            r#"
            let t = typeof monotonicMs();
            let a = monotonicMs();
            let b = monotonicMs();
            let ok_nonneg = a >= 0;
            let ok_order = b >= a;
            let ok_delta = (b - a) < 60000;
            "#,
        );
        assert!(is_host_time_module(&m));
        let ir = emit_host_time(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_monotonic_ms"), "{ir}");
    }
}

// temp
