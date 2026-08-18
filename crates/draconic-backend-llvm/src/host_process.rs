//! H01.01 / H01.02 / H01.03: native observations for process host APIs.
//!
//! - `processArgs()` — user program args as string[]
//! - `envGet` / `envSet` / `envDelete` — string env; missing get → undefined
//! - `exit` / `exitCode` / `setExitCode` — terminate / deferred status (default 0)
//!
//! Prints number locals via `print_f64` and string / maybe-string locals via
//! `print_str` (null maybe-string prints as `undefined`). `main` takes OS
//! argc/argv when processArgs is used. Exit-only modules return deferred code.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::UnaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, GC_INIT, HOST_ENV_DELETE,
    HOST_ENV_GET, HOST_ENV_SET, HOST_PROCESS_EXIT, HOST_PROCESS_GET_EXIT_CODE,
    HOST_PROCESS_SET_ARGV, HOST_PROCESS_SET_EXIT_CODE, HOST_PROCESS_USER_ARG,
    HOST_PROCESS_USER_ARGC, PRINT_F64, PRINT_STR,
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
    /// Always-present C string (processArgs element, typeof result, string lit).
    String,
    /// `envGet` result: non-null C string or null (= undefined).
    MaybeString,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    needs_argv: bool,
    needs_env: bool,
    needs_exit: bool,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    has_process_args: bool,
    has_env: bool,
    has_exit: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        has_process_args: false,
        has_env: false,
        has_exit: false,
    };

    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }

    if !(ctx.has_process_args || ctx.has_env || ctx.has_exit) {
        return None;
    }
    // Exit-only modules (e.g. `exit(7);`) have no print locals.
    if ctx.print_locals.is_empty() && !ctx.has_exit {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
        needs_argv: ctx.has_process_args,
        needs_env: ctx.has_env,
        needs_exit: ctx.has_exit,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            if matches!(
                ty,
                SlotTy::Number | SlotTy::String | SlotTy::MaybeString
            ) {
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
        Expr::Call { callee, args, .. } => {
            let name = ident_name(callee)?;
            match name {
                "envSet" if args.len() == 2 => {
                    ctx.has_env = true;
                    classify_expr(arg_expr(&args[0])?, ctx)?;
                    classify_expr(arg_expr(&args[1])?, ctx)?;
                    Some(())
                }
                "envDelete" if args.len() == 1 => {
                    ctx.has_env = true;
                    classify_expr(arg_expr(&args[0])?, ctx)?;
                    Some(())
                }
                "exit" if args.is_empty() || args.len() == 1 => {
                    ctx.has_exit = true;
                    if args.len() == 1 {
                        classify_expr(arg_expr(&args[0])?, ctx)?;
                    }
                    Some(())
                }
                "setExitCode" if args.len() == 1 => {
                    ctx.has_exit = true;
                    classify_expr(arg_expr(&args[0])?, ctx)?;
                    Some(())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. } if args.is_empty() && is_named_callee(callee, "processArgs") => {
            ctx.has_process_args = true;
            Some(SlotTy::Array)
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "envGet") => {
            ctx.has_env = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::MaybeString)
        }
        Expr::Call { callee, args, .. } if args.len() == 2 && is_named_callee(callee, "envSet") => {
            ctx.has_env = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            classify_expr(arg_expr(&args[1])?, ctx)?;
            // Not assigned as value in fixtures; treat as void if ever used as expr.
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "envDelete") => {
            ctx.has_env = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "exitCode") =>
        {
            ctx.has_exit = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if (args.is_empty() || args.len() == 1) && is_named_callee(callee, "exit") =>
        {
            ctx.has_exit = true;
            if args.len() == 1 {
                classify_expr(arg_expr(&args[0])?, ctx)?;
            }
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "setExitCode") =>
        {
            ctx.has_exit = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            let _ = classify_expr(arg, ctx)?;
            Some(SlotTy::String)
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
                Some(SlotTy::String)
            } else {
                None
            }
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        _ => None,
    }
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn ident_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::IdentName { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

fn arg_expr(arg: &Arg) -> Option<&Expr> {
    match arg {
        Arg::Expr(e) => Some(e),
        Arg::Spread(_) => None,
    }
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
    str_globals: HashMap<String, String>,
    /// After `exit()` emits `unreachable`, skip further body/print/ret.
    terminated: bool,
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
            str_globals: HashMap::new(),
            terminated: false,
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

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some(g) = self.str_globals.get(s) {
            return g.clone();
        }
        let g = format!(".hp.str.{}", self.str_globals.len());
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

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM host_process (H01 processArgs / env / exit)"
        )
        .ok();
        let mut decls = vec![GC_INIT, PRINT_F64, PRINT_STR];
        if info.needs_argv {
            decls.extend([
                ARRAY_NEW,
                ARRAY_SET,
                ARRAY_GET,
                ARRAY_LEN,
                HOST_PROCESS_SET_ARGV,
                HOST_PROCESS_USER_ARGC,
                HOST_PROCESS_USER_ARG,
            ]);
        }
        if info.needs_env {
            decls.extend([HOST_ENV_GET, HOST_ENV_SET, HOST_ENV_DELETE]);
        }
        if info.needs_exit {
            decls.extend([
                HOST_PROCESS_EXIT,
                HOST_PROCESS_SET_EXIT_CODE,
                HOST_PROCESS_GET_EXIT_CODE,
            ]);
        }
        self.out.push_str(&llvm_declares(&decls));
        writeln!(self.out).ok();

        for (id, ty) in &info.slots {
            let ptr = self.slot_ptr(*id)?;
            let llvm_ty = match ty {
                SlotTy::Number => "double",
                SlotTy::Array | SlotTy::String | SlotTy::MaybeString => "ptr",
            };
            writeln!(self.body, "  {ptr} = alloca {llvm_ty}, align 8").ok();
        }

        for stmt in &self.module.body {
            if self.terminated {
                break;
            }
            self.emit_stmt(stmt)?;
        }

        if !self.terminated {
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
                    SlotTy::MaybeString => {
                        self.emit_print_maybe_string(ptr)?;
                    }
                    SlotTy::Array => {}
                }
            }
        }

        // Emit string globals before main.
        let body = std::mem::take(&mut self.body);
        let terminated = self.terminated;
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

        if info.needs_argv {
            writeln!(self.out, "define i32 @main(i32 %argc, ptr %argv) {{").ok();
        } else {
            writeln!(self.out, "define i32 @main() {{").ok();
        }
        writeln!(self.out, "entry:").ok();
        writeln!(self.out, "  {}", GC_INIT.call("")).ok();
        if info.needs_argv {
            writeln!(
                self.out,
                "  {}",
                HOST_PROCESS_SET_ARGV.call("i32 %argc, ptr %argv")
            )
            .ok();
        }
        self.out.push_str(&body);
        if !terminated {
            if info.needs_exit {
                let code = self.fresh();
                writeln!(
                    self.out,
                    "  {}",
                    HOST_PROCESS_GET_EXIT_CODE.call_to(&code, "")
                )
                .ok();
                writeln!(self.out, "  ret i32 {code}").ok();
            } else {
                writeln!(self.out, "  ret i32 0").ok();
            }
        }
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_print_maybe_string(&mut self, slot_ptr: String) -> Result<(), Diagnostic> {
        let v = self.fresh();
        let is_null = self.fresh();
        let lab_str = format!("ms_str_{}", self.next_tmp);
        let lab_und = format!("ms_und_{}", self.next_tmp);
        let lab_end = format!("ms_end_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {v} = load ptr, ptr {slot_ptr}").ok();
        writeln!(self.body, "  {is_null} = icmp eq ptr {v}, null").ok();
        writeln!(
            self.body,
            "  br i1 {is_null}, label %{lab_und}, label %{lab_str}"
        )
        .ok();
        writeln!(self.body, "{lab_str}:").ok();
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
        writeln!(self.body, "  br label %{lab_end}").ok();
        writeln!(self.body, "{lab_und}:").ok();
        let und = self.emit_cstr_ptr("undefined");
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {und}"))).ok();
        writeln!(self.body, "  br label %{lab_end}").ok();
        writeln!(self.body, "{lab_end}:").ok();
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
                    SlotTy::MaybeString => {
                        let v = self.emit_maybe_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => self.emit_side_effect(expr),
            _ => Err(diag("host_process: unsupported statement")),
        }
    }

    fn emit_side_effect(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } => {
                let name = ident_name(callee).ok_or_else(|| diag("host_process: bad call"))?;
                match name {
                    "envSet" if args.len() == 2 => {
                        let k = self.emit_string_expr(arg_expr(&args[0]).ok_or_else(|| {
                            diag("host_process: envSet key")
                        })?)?;
                        let v = self.emit_string_expr(arg_expr(&args[1]).ok_or_else(|| {
                            diag("host_process: envSet value")
                        })?)?;
                        let _rc = self.fresh();
                        writeln!(
                            self.body,
                            "  {_rc} = call i32 @{}(ptr {k}, ptr {v})",
                            HOST_ENV_SET.symbol
                        )
                        .ok();
                        Ok(())
                    }
                    "envDelete" if args.len() == 1 => {
                        let k = self.emit_string_expr(arg_expr(&args[0]).ok_or_else(|| {
                            diag("host_process: envDelete key")
                        })?)?;
                        let _rc = self.fresh();
                        writeln!(
                            self.body,
                            "  {_rc} = call i32 @{}(ptr {k})",
                            HOST_ENV_DELETE.symbol
                        )
                        .ok();
                        Ok(())
                    }
                    "exit" if args.is_empty() || args.len() == 1 => {
                        let code_i32 = if args.is_empty() {
                            let c = self.fresh();
                            writeln!(
                                self.body,
                                "  {}",
                                HOST_PROCESS_GET_EXIT_CODE.call_to(&c, "")
                            )
                            .ok();
                            c
                        } else {
                            let f = self.emit_number_expr(arg_expr(&args[0]).ok_or_else(|| {
                                diag("host_process: exit code")
                            })?)?;
                            let c = self.fresh();
                            writeln!(self.body, "  {c} = fptosi double {f} to i32").ok();
                            c
                        };
                        writeln!(
                            self.body,
                            "  call void @{}(i32 {code_i32})",
                            HOST_PROCESS_EXIT.symbol
                        )
                        .ok();
                        // exit never returns; terminate this block (no further insts).
                        writeln!(self.body, "  unreachable").ok();
                        self.terminated = true;
                        Ok(())
                    }
                    "setExitCode" if args.len() == 1 => {
                        let f = self.emit_number_expr(arg_expr(&args[0]).ok_or_else(|| {
                            diag("host_process: setExitCode code")
                        })?)?;
                        let c = self.fresh();
                        writeln!(self.body, "  {c} = fptosi double {f} to i32").ok();
                        writeln!(
                            self.body,
                            "  call void @{}(i32 {c})",
                            HOST_PROCESS_SET_EXIT_CODE.symbol
                        )
                        .ok();
                        Ok(())
                    }
                    _ => Err(diag("host_process: unsupported side-effect call")),
                }
            }
            _ => Err(diag("host_process: unsupported expr stmt")),
        }
    }

    fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "processArgs") =>
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
            Expr::Call { callee, args, .. }
                if args.is_empty() && is_named_callee(callee, "exitCode") =>
            {
                let c = self.fresh();
                let f = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    HOST_PROCESS_GET_EXIT_CODE.call_to(&c, "")
                )
                .ok();
                writeln!(self.body, "  {f} = sitofp i32 {c} to double").ok();
                Ok(f)
            }
            _ => Err(diag("host_process: expected number expr")),
        }
    }

    fn emit_maybe_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "envGet") =>
            {
                let k = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_process: envGet key"))?,
                )?;
                let v = self.fresh();
                writeln!(
                    self.body,
                    "  {v} = call ptr @{}(ptr {k})",
                    HOST_ENV_GET.symbol
                )
                .ok();
                Ok(v)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_process: expected envGet maybe-string")),
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
            } => self.emit_typeof(arg),
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
                writeln!(self.body, "  {empty} = alloca [1 x i8], align 1").ok();
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
                let kind = self
                    .slot_of
                    .get(id)
                    .copied()
                    .ok_or_else(|| diag("host_process: unknown string local"))?;
                if kind == SlotTy::MaybeString {
                    return Err(diag("host_process: maybe-string not a bare string"));
                }
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_process: expected string expr")),
        }
    }

    fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        match arg {
            Expr::Local { id, .. } => {
                let kind = self
                    .slot_of
                    .get(id)
                    .copied()
                    .ok_or_else(|| diag("host_process: typeof unknown local"))?;
                match kind {
                    SlotTy::MaybeString => {
                        let ptr = self.slot_ptr(*id)?;
                        let v = self.fresh();
                        let is_null = self.fresh();
                        let lab_s = format!("tof_s_{}", self.next_tmp);
                        let lab_u = format!("tof_u_{}", self.next_tmp);
                        let lab_e = format!("tof_e_{}", self.next_tmp);
                        self.next_tmp += 1;
                        let out_slot = self.fresh();
                        writeln!(self.body, "  {out_slot} = alloca ptr, align 8").ok();
                        writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                        writeln!(self.body, "  {is_null} = icmp eq ptr {v}, null").ok();
                        writeln!(
                            self.body,
                            "  br i1 {is_null}, label %{lab_u}, label %{lab_s}"
                        )
                        .ok();
                        writeln!(self.body, "{lab_s}:").ok();
                        let ps = self.emit_cstr_ptr("string");
                        writeln!(self.body, "  store ptr {ps}, ptr {out_slot}").ok();
                        writeln!(self.body, "  br label %{lab_e}").ok();
                        writeln!(self.body, "{lab_u}:").ok();
                        let pu = self.emit_cstr_ptr("undefined");
                        writeln!(self.body, "  store ptr {pu}, ptr {out_slot}").ok();
                        writeln!(self.body, "  br label %{lab_e}").ok();
                        writeln!(self.body, "{lab_e}:").ok();
                        let r = self.fresh();
                        writeln!(self.body, "  {r} = load ptr, ptr {out_slot}").ok();
                        Ok(r)
                    }
                    SlotTy::String => Ok(self.emit_cstr_ptr("string")),
                    SlotTy::Array => Ok(self.emit_cstr_ptr("object")),
                    SlotTy::Number => Ok(self.emit_cstr_ptr("number")),
                }
            }
            _ => Err(diag("host_process: typeof unsupported arg")),
        }
    }
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) && c != b'\\' => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
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

    #[test]
    fn classifies_env_get_set_delete() {
        let m = lower_src(
            r#"
            envSet("DRACONIC_H0102_ENV_KEY", "alpha");
            let got = envGet("DRACONIC_H0102_ENV_KEY");
            let missing = envGet("DRACONIC_H0102_ENV_MISSING_XYZ");
            envDelete("DRACONIC_H0102_ENV_KEY");
            let after = envGet("DRACONIC_H0102_ENV_KEY");
            let t_got = typeof got;
            let t_missing = typeof missing;
            let t_after = typeof after;
            "#,
        );
        assert!(is_host_process_module(&m));
        let ir = emit_host_process(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_env_get"), "{ir}");
        assert!(ir.contains("draconic_rt_host_env_set"), "{ir}");
        assert!(ir.contains("draconic_rt_host_env_delete"), "{ir}");
        assert!(ir.contains("define i32 @main()"), "{ir}");
        let dir = std::env::temp_dir().join(format!(
            "draconic-hp-env-{}",
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

    #[test]
    fn classifies_exit_and_exit_code() {
        let m = lower_src(
            r#"
            setExitCode(5);
            let code = exitCode();
            "#,
        );
        assert!(is_host_process_module(&m));
        let ir = emit_host_process(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_process_set_exit_code"), "{ir}");
        assert!(ir.contains("draconic_rt_host_process_get_exit_code"), "{ir}");
        assert!(ir.contains("ret i32"), "{ir}");
        let m2 = lower_src("exit(7);");
        assert!(is_host_process_module(&m2));
        let ir2 = emit_host_process(&m2).expect("emit");
        assert!(ir2.contains("draconic_rt_host_process_exit"), "{ir2}");
        assert!(ir2.contains("unreachable"), "{ir2}");
        let m3 = lower_src("exit();");
        assert!(is_host_process_module(&m3));
        let ir3 = emit_host_process(&m3).expect("emit");
        assert!(ir3.contains("draconic_rt_host_process_exit"), "{ir3}");
        assert!(ir3.contains("draconic_rt_host_process_get_exit_code"), "{ir3}");
    }
}
