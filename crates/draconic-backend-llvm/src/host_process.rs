//! H01.01: native observations for `processArgs()` — user program args as string[].
//!
//! Fixture shape:
//! ```text
//! let args = processArgs();
//! let n = args.length;
//! let a0 = args[0];
//! ```
//! Prints number locals via `print_f64` and string locals via `print_str`.
//! `main` takes OS argc/argv and records them on the host Runtime before body.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, GC_INIT, HOST_PROCESS_SET_ARGV,
    HOST_PROCESS_USER_ARG, HOST_PROCESS_USER_ARGC, PRINT_F64, PRINT_STR,
};

pub(crate) fn is_host_process_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_process(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_process module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Array,
    Number,
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
    has_process_args: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        has_process_args: false,
    };

    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }

    if !ctx.has_process_args || ctx.print_locals.is_empty() {
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
            if matches!(ty, SlotTy::Number | SlotTy::String) {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        Stmt::Expr { .. } => Some(()),
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. } if args.is_empty() && is_process_args_callee(callee) => {
            ctx.has_process_args = true;
            Some(SlotTy::Array)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let obj_ty = classify_expr(object, ctx)?;
            let prop = string_lit(property)?;
            if obj_ty == SlotTy::Array && prop.as_str() == "length" {
                Some(SlotTy::Number)
            } else {
                None
            }
        }
        Expr::Member {
            object,
            property,
            computed: true,
            ..
        } => {
            let obj_ty = classify_expr(object, ctx)?;
            let _idx = classify_expr(property, ctx)?;
            if obj_ty == SlotTy::Array {
                // Index into processArgs array → string element.
                Some(SlotTy::String)
            } else {
                None
            }
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(SlotTy::Number),
        _ => None,
    }
}

fn is_process_args_callee(expr: &Expr) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == "processArgs")
}

fn string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy()),
        _ => None,
    }
}

struct Emitter<'a> {
    module: &'a Module,
    by_id: HashMap<LocalId, &'a Local>,
    slot_of: HashMap<LocalId, SlotTy>,
    body: String,
    out: String,
    next_tmp: u32,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &ModuleInfo) -> Self {
        let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
        let slot_of: HashMap<LocalId, SlotTy> = info.slots.iter().copied().collect();
        Self {
            module,
            by_id,
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
        let name = self
            .by_id
            .get(&id)
            .map(|l| l.name.as_str())
            .ok_or_else(|| diag("host_process: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        writeln!(self.out, "; Draconic LLVM host_process (H01.01 processArgs)").ok();
        let decls = llvm_declares(&[
            GC_INIT,
            PRINT_F64,
            PRINT_STR,
            ARRAY_NEW,
            ARRAY_SET,
            ARRAY_GET,
            ARRAY_LEN,
            HOST_PROCESS_SET_ARGV,
            HOST_PROCESS_USER_ARGC,
            HOST_PROCESS_USER_ARG,
        ]);
        self.out.push_str(&decls);
        writeln!(self.out).ok();

        for (id, ty) in &info.slots {
            let ptr = self.slot_ptr(*id)?;
            let llvm_ty = match ty {
                SlotTy::Number => "double",
                SlotTy::Array | SlotTy::String => "ptr",
            };
            writeln!(self.body, "  {ptr} = alloca {llvm_ty}, align 8").ok();
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, kind) in &info.print_locals {
            let ptr = self.slot_ptr(*id)?;
            match kind {
                SlotTy::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotTy::Array => {}
            }
        }

        writeln!(self.out, "define i32 @main(i32 %argc, ptr %argv) {{").ok();
        writeln!(self.out, "entry:").ok();
        writeln!(self.out, "  {}", GC_INIT.call("")).ok();
        writeln!(
            self.out,
            "  {}",
            HOST_PROCESS_SET_ARGV.call("i32 %argc, ptr %argv")
        )
        .ok();
        self.out.push_str(&self.body);
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
                    .ok_or_else(|| diag("host_process: declare unknown slot"))?;
                let ptr = self.slot_ptr(*local)?;
                match kind {
                    SlotTy::Array => {
                        let v = self.emit_array_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { .. } => Ok(()),
            _ => Err(diag("host_process: unsupported statement")),
        }
    }

    fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_process_args_callee(callee) =>
            {
                self.emit_process_args_array()
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_process: expected processArgs() array")),
        }
    }

    fn emit_process_args_array(&mut self) -> Result<String, Diagnostic> {
        let n32 = self.fresh();
        let n64 = self.fresh();
        let arr = self.fresh();
        let i_slot = self.fresh();
        let loop_cond = format!("args_loop_cond_{}", self.next_tmp);
        let loop_body = format!("args_loop_body_{}", self.next_tmp);
        let loop_end = format!("args_loop_end_{}", self.next_tmp);
        self.next_tmp += 1;

        writeln!(
            self.body,
            "  {}",
            HOST_PROCESS_USER_ARGC.call_to(&n32, "")
        )
        .ok();
        writeln!(self.body, "  {n64} = sext i32 {n32} to i64").ok();
        writeln!(
            self.body,
            "  {}",
            ARRAY_NEW.call_to(&arr, &format!("i64 {n64}"))
        )
        .ok();
        writeln!(self.body, "  {i_slot} = alloca i32, align 4").ok();
        writeln!(self.body, "  store i32 0, ptr {i_slot}").ok();
        writeln!(self.body, "  br label %{loop_cond}").ok();

        writeln!(self.body, "{loop_cond}:").ok();
        let i_load = self.fresh();
        let cmp = self.fresh();
        writeln!(self.body, "  {i_load} = load i32, ptr {i_slot}").ok();
        writeln!(self.body, "  {cmp} = icmp slt i32 {i_load}, {n32}").ok();
        writeln!(
            self.body,
            "  br i1 {cmp}, label %{loop_body}, label %{loop_end}"
        )
        .ok();

        writeln!(self.body, "{loop_body}:").ok();
        let arg = self.fresh();
        let i64v = self.fresh();
        let i_next = self.fresh();
        writeln!(
            self.body,
            "  {arg} = call ptr @{}(i32 {i_load})",
            HOST_PROCESS_USER_ARG.symbol
        )
        .ok();
        writeln!(self.body, "  {i64v} = sext i32 {i_load} to i64").ok();
        writeln!(
            self.body,
            "  call void @{}(ptr {arr}, i64 {i64v}, ptr {arg})",
            ARRAY_SET.symbol
        )
        .ok();
        writeln!(self.body, "  {i_next} = add i32 {i_load}, 1").ok();
        writeln!(self.body, "  store i32 {i_next}, ptr {i_slot}").ok();
        writeln!(self.body, "  br label %{loop_cond}").ok();

        writeln!(self.body, "{loop_end}:").ok();
        Ok(arr)
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let v = self.fresh();
                let n: f64 = raw.parse().unwrap_or(0.0);
                // Always emit a float literal so LLVM accepts the operand.
                let lit = if n.fract() == 0.0 {
                    format!("{n:.1}")
                } else {
                    format!("{n}")
                };
                writeln!(self.body, "  {v} = fadd double {lit}, 0.0").ok();
                Ok(v)
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_process: length prop"))?;
                if prop.as_str() != "length" {
                    return Err(diag("host_process: only .length on args array"));
                }
                let arr = self.emit_array_expr(object)?;
                let len = self.fresh();
                let f = self.fresh();
                writeln!(
                    self.body,
                    "  {len} = call i64 @{}(ptr {arr})",
                    ARRAY_LEN.symbol
                )
                .ok();
                writeln!(self.body, "  {f} = sitofp i64 {len} to double").ok();
                Ok(f)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_process: expected number expr")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Member {
                object,
                property,
                computed: true,
                ..
            } => {
                let arr = self.emit_array_expr(object)?;
                let idx_f = self.emit_number_expr(property)?;
                let idx = self.fresh();
                let el = self.fresh();
                let empty = self.fresh();
                let is_null = self.fresh();
                let join = format!("str_join_{}", self.next_tmp);
                let use_el = format!("str_el_{}", self.next_tmp);
                let end = format!("str_end_{}", self.next_tmp);
                self.next_tmp += 1;
                writeln!(self.body, "  {idx} = fptosi double {idx_f} to i64").ok();
                writeln!(
                    self.body,
                    "  {el} = call ptr @{}(ptr {arr}, i64 {idx})",
                    ARRAY_GET.symbol
                )
                .ok();
                // null → empty string for OOB
                writeln!(
                    self.body,
                    "  {empty} = alloca [1 x i8], align 1"
                )
                .ok();
                let empty_ptr = self.fresh();
                writeln!(
                    self.body,
                    "  {empty_ptr} = getelementptr inbounds [1 x i8], ptr {empty}, i64 0, i64 0"
                )
                .ok();
                writeln!(self.body, "  store i8 0, ptr {empty_ptr}").ok();
                writeln!(self.body, "  {is_null} = icmp eq ptr {el}, null").ok();
                writeln!(self.body, "  br i1 {is_null}, label %{join}, label %{use_el}").ok();
                writeln!(self.body, "{use_el}:").ok();
                writeln!(self.body, "  br label %{end}").ok();
                writeln!(self.body, "{join}:").ok();
                writeln!(self.body, "  br label %{end}").ok();
                writeln!(self.body, "{end}:").ok();
                let phi = self.fresh();
                writeln!(
                    self.body,
                    "  {phi} = phi ptr [ {el}, %{use_el} ], [ {empty_ptr}, %{join} ]"
                )
                .ok();
                Ok(phi)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_process: expected string index expr")),
        }
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn lower_src(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn classifies_process_args_length_and_index() {
        let m = lower_src(
            r#"
            let args = processArgs();
            let n = args.length;
            let a0 = args[0];
            "#,
        );
        assert!(is_host_process_module(&m));
        let ir = emit_host_process(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_process_set_argv"), "{ir}");
        assert!(ir.contains("draconic_rt_host_process_user_argc"), "{ir}");
        assert!(ir.contains("define i32 @main(i32 %argc, ptr %argv)"), "{ir}");
        // Validate clang accepts IR.
        let dir = std::env::temp_dir().join(format!(
            "draconic-hp-ir-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let ll = dir.join("t.ll");
        std::fs::write(&ll, &ir).unwrap();
        let clang = std::env::var("CLANG").unwrap_or_else(|_| "clang".into());
        let out = std::process::Command::new(&clang)
            .args(["-c", "-o"])
            .arg(dir.join("t.o"))
            .arg(&ll)
            .output()
            .expect("clang");
        assert!(
            out.status.success(),
            "clang reject IR:\n{}\n--- IR ---\n{ir}",
            String::from_utf8_lossy(&out.stderr)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
