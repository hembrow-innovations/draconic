//! H16 / H16.01 / H16.02 / H16.03: native observations for OS host APIs.
//!
//! - `cwd()` → absolute path string (not auto-printed; use in `===` / `typeof`)
//! - `chdir(path)` → side-effect; missing path → HostError (ENOENT) on full path later
//! - `hostname()` / `osType()` / `osArch()` → non-empty strings (H16.02)
//! - `tempDir()` / `homeDir()` → non-empty path strings (H16.03)
//!
//! Auto-prints string locals from `typeof` and bool locals from comparisons.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_CHDIR, HOST_CWD, HOST_HOME_DIR, HOST_HOSTNAME, HOST_OS_ARCH,
    HOST_OS_TYPE, HOST_TEMP_DIR, PRINT_BOOL, PRINT_STR,
};

pub(crate) fn is_host_os_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_os(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_os module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    /// Path from `cwd()` — stored, not auto-printed.
    Path,
    /// `typeof` / string literals — auto-printed.
    String,
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
    has_os: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        has_os: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_os || ctx.print_locals.is_empty() {
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
            if matches!(ty, SlotTy::String | SlotTy::Bool) {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => {
            classify_side_effect(expr, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. } if is_named_callee(callee, "chdir") && args.len() == 1 => {
            ctx.has_os = true;
            let _ = classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.is_empty()
                && (is_named_callee(callee, "cwd")
                    || is_named_callee(callee, "hostname")
                    || is_named_callee(callee, "osType")
                    || is_named_callee(callee, "osArch")
                    || is_named_callee(callee, "tempDir")
                    || is_named_callee(callee, "homeDir")) =>
        {
            ctx.has_os = true;
            Some(SlotTy::Path)
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "chdir") => {
            ctx.has_os = true;
            let _ = classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Bool)
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            let _ = classify_expr(arg, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Binary {
            op: BinaryOp::EqEqEq | BinaryOp::EqEq | BinaryOp::NotEqEq | BinaryOp::NotEq,
            left,
            right,
            ..
        } => {
            let lt = classify_expr(left, ctx)?;
            let rt = classify_expr(right, ctx)?;
            if matches!(lt, SlotTy::Path | SlotTy::String)
                && matches!(rt, SlotTy::Path | SlotTy::String)
            {
                Some(SlotTy::Bool)
            } else {
                None
            }
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::String { .. } => Some(SlotTy::String),
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
            .ok_or_else(|| diag("host_os: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some(g) = self.str_globals.get(s) {
            return g.clone();
        }
        let g = format!(".hos.str.{}", self.str_globals.len());
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
        writeln!(self.out, "; Draconic LLVM host_os (H16)").ok();
        let decls = vec![
            GC_INIT,
            PRINT_STR,
            PRINT_BOOL,
            HOST_CWD,
            HOST_CHDIR,
            HOST_HOSTNAME,
            HOST_OS_TYPE,
            HOST_OS_ARCH,
            HOST_TEMP_DIR,
            HOST_HOME_DIR,
        ];
        self.out.push_str(&llvm_declares(&decls));
        writeln!(self.out, "declare i32 @strcmp(ptr, ptr)").ok();
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            let ptr = self.slot_ptr(*id)?;
            let llvm_ty = match ty {
                SlotTy::Path | SlotTy::String => "ptr",
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
                SlotTy::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotTy::Bool => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
                }
                SlotTy::Path => {}
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
                    .ok_or_else(|| diag("host_os: declare unknown slot"))?;
                let ptr = self.slot_ptr(*local)?;
                match kind {
                    SlotTy::Path | SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Bool => {
                        let v = self.emit_bool_expr(init)?;
                        writeln!(self.body, "  store i8 {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => self.emit_side_effect(expr),
            _ => Err(diag("host_os: unsupported statement")),
        }
    }

    fn emit_side_effect(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if is_named_callee(callee, "chdir") && args.len() == 1 => {
                let p = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_os: chdir path"))?,
                )?;
                let rc = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {p})",
                    HOST_CHDIR.symbol
                )
                .ok();
                Ok(())
            }
            _ => Err(diag("host_os: unsupported side effect")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if args.is_empty() && is_named_callee(callee, "cwd") => {
                let r = self.fresh();
                writeln!(self.body, "  {}", HOST_CWD.call_to(&r, "")).ok();
                Ok(r)
            }
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "hostname") =>
            {
                let r = self.fresh();
                writeln!(self.body, "  {}", HOST_HOSTNAME.call_to(&r, "")).ok();
                Ok(r)
            }
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "osType") =>
            {
                let r = self.fresh();
                writeln!(self.body, "  {}", HOST_OS_TYPE.call_to(&r, "")).ok();
                Ok(r)
            }
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "osArch") =>
            {
                let r = self.fresh();
                writeln!(self.body, "  {}", HOST_OS_ARCH.call_to(&r, "")).ok();
                Ok(r)
            }
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "tempDir") =>
            {
                let r = self.fresh();
                writeln!(self.body, "  {}", HOST_TEMP_DIR.call_to(&r, "")).ok();
                Ok(r)
            }
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "homeDir") =>
            {
                let r = self.fresh();
                writeln!(self.body, "  {}", HOST_HOME_DIR.call_to(&r, "")).ok();
                Ok(r)
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => {
                // typeof of string-returning host APIs / path locals / strings.
                let _ = arg;
                Ok(self.emit_cstr_ptr("string"))
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                Ok(self.emit_cstr_ptr(&s))
            }
            _ => Err(diag("host_os: unsupported string expr")),
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
                BinaryOp::EqEqEq | BinaryOp::EqEq | BinaryOp::NotEqEq | BinaryOp::NotEq
            ) =>
            {
                let l = self.emit_string_expr(left)?;
                let r = self.emit_string_expr(right)?;
                let cmp = self.fresh();
                writeln!(self.body, "  {cmp} = call i32 @strcmp(ptr {l}, ptr {r})").ok();
                let is_eq = matches!(op, BinaryOp::EqEqEq | BinaryOp::EqEq);
                let z = self.fresh();
                let pred = if is_eq { "eq" } else { "ne" };
                writeln!(self.body, "  {z} = icmp {pred} i32 {cmp}, 0").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {z} to i8").ok();
                Ok(b)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_os: unsupported bool expr")),
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
    fn classifies_cwd_chdir() {
        let m = lower_src(
            r#"
            let t = typeof cwd();
            let saved = cwd();
            chdir("/");
            let at_root = cwd() === "/";
            chdir(saved);
            let restored = cwd() === saved;
            "#,
        );
        assert!(is_host_os_module(&m));
        let ir = emit_host_os(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_cwd"), "{ir}");
        assert!(ir.contains("draconic_rt_host_chdir"), "{ir}");
        assert!(ir.contains("strcmp"), "{ir}");
    }

    #[test]
    fn classifies_hostname_os_type_arch() {
        let m = lower_src(
            r#"
            let t_h = typeof hostname();
            let t_o = typeof osType();
            let t_a = typeof osArch();
            let h_ok = hostname() !== "";
            let o_ok = osType() !== "";
            let a_ok = osArch() !== "";
            "#,
        );
        assert!(is_host_os_module(&m));
        let ir = emit_host_os(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_hostname"), "{ir}");
        assert!(ir.contains("draconic_rt_host_os_type"), "{ir}");
        assert!(ir.contains("draconic_rt_host_os_arch"), "{ir}");
    }

    #[test]
    fn classifies_combined_os_misc_surface() {
        let m = lower_src(
            r#"
            let t_cwd = typeof cwd();
            let saved = cwd();
            chdir("/");
            let at_root = cwd() === "/";
            chdir(saved);
            let restored = cwd() === saved;
            let t_h = typeof hostname();
            let t_o = typeof osType();
            let t_a = typeof osArch();
            let h_ok = hostname() !== "";
            let o_ok = osType() !== "";
            let a_ok = osArch() !== "";
            let t_t = typeof tempDir();
            let t_hd = typeof homeDir();
            let td = tempDir();
            let hd = homeDir();
            let t_ok = td !== "";
            let hd_ok = hd !== "";
            "#,
        );
        assert!(is_host_os_module(&m));
        let ir = emit_host_os(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_cwd"), "{ir}");
        assert!(ir.contains("draconic_rt_host_chdir"), "{ir}");
        assert!(ir.contains("draconic_rt_host_hostname"), "{ir}");
        assert!(ir.contains("draconic_rt_host_os_type"), "{ir}");
        assert!(ir.contains("draconic_rt_host_os_arch"), "{ir}");
        assert!(ir.contains("draconic_rt_host_temp_dir"), "{ir}");
        assert!(ir.contains("draconic_rt_host_home_dir"), "{ir}");
    }

    #[test]
    fn classifies_temp_home_dir() {
        let m = lower_src(
            r#"
            let t_t = typeof tempDir();
            let t_h = typeof homeDir();
            let td = tempDir();
            let hd = homeDir();
            let t_ok = td !== "";
            let h_ok = hd !== "";
            "#,
        );
        assert!(is_host_os_module(&m));
        let ir = emit_host_os(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_temp_dir"), "{ir}");
        assert!(ir.contains("draconic_rt_host_home_dir"), "{ir}");
    }
}
