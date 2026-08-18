//! H15.01: `processRun(argv, cwd?, env?)` — spawn, wait, exit code.
//! H15.02: `processSpawn` / `processStdinWrite` / `processWait` /
//! `processStdout` / `processStderr` / `processKill` / `processClose`.
//!
//! Supported subset for conformance:
//! - top-level number/bool/string locals
//! - `processRun(["prog", ...args])` (+ optional cwd/env)
//! - `processSpawn` same argv shape; handle as number
//! - `processStdinWrite(h, "text")`, `processWait(h)`, `processKill(h)`, `processClose(h)`
//! - `processStdout(h)` / `processStderr(h)` → string after wait
//! - `===` / `!==` / comparisons on numbers; `typeof` of APIs → `"function"`
//! - string `===` / `!==` for stdout/stderr checks

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, Expr, Local, LocalId, Module, ObjectProp, ObjectPropKey, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_PROCESS_CLOSE, HOST_PROCESS_KILL, HOST_PROCESS_RUN,
    HOST_PROCESS_SPAWN, HOST_PROCESS_STDERR, HOST_PROCESS_STDIN_WRITE, HOST_PROCESS_STDOUT,
    HOST_PROCESS_WAIT, PRINT_BOOL, PRINT_F64, PRINT_STR,
};

pub(crate) fn is_host_subprocess_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_subprocess(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_subprocess module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
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
    uses_run: bool,
    uses_spawn: bool,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    uses_run: bool,
    uses_spawn: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        uses_run: false,
        uses_spawn: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !(ctx.uses_run || ctx.uses_spawn) || ctx.print_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
        uses_run: ctx.uses_run,
        uses_spawn: ctx.uses_spawn,
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

fn classify_process_argv_call(args: &[Arg], ctx: &mut ClassifyCtx) -> Option<()> {
    if args.is_empty() || args.len() > 3 {
        return None;
    }
    string_array_lit(arg_expr(&args[0])?)?;
    if args.len() >= 2 {
        match arg_expr(&args[1])? {
            Expr::Null { .. } | Expr::String { .. } => {}
            _ => return None,
        }
    }
    if args.len() == 3 {
        env_object_lit(arg_expr(&args[2])?)?;
    }
    let _ = ctx;
    Some(())
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. } if is_named_callee(callee, "processRun") => {
            ctx.uses_run = true;
            classify_process_argv_call(args, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "processSpawn") => {
            ctx.uses_spawn = true;
            classify_process_argv_call(args, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "processStdinWrite") => {
            ctx.uses_spawn = true;
            if args.len() != 2 {
                return None;
            }
            let _ = classify_expr(arg_expr(&args[0])?, ctx)?;
            match arg_expr(&args[1])? {
                Expr::String { .. } => {}
                e => {
                    let _ = classify_expr(e, ctx)?;
                }
            }
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if is_named_callee(callee, "processWait")
                || is_named_callee(callee, "processKill")
                || is_named_callee(callee, "processClose") =>
        {
            ctx.uses_spawn = true;
            if args.len() != 1 {
                return None;
            }
            let _ = classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if is_named_callee(callee, "processStdout") || is_named_callee(callee, "processStderr")
        =>
        {
            ctx.uses_spawn = true;
            if args.len() != 1 {
                return None;
            }
            let _ = classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::String)
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
            if (lt == SlotTy::Number && rt == SlotTy::Number)
                || (lt == SlotTy::String && rt == SlotTy::String)
            {
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
            if is_named_ident(arg, "processRun") {
                ctx.uses_run = true;
                Some(SlotTy::String)
            } else if is_named_ident(arg, "processSpawn")
                || is_named_ident(arg, "processStdinWrite")
                || is_named_ident(arg, "processWait")
                || is_named_ident(arg, "processStdout")
                || is_named_ident(arg, "processStderr")
                || is_named_ident(arg, "processKill")
                || is_named_ident(arg, "processClose")
            {
                ctx.uses_spawn = true;
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

fn string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn string_array_lit(expr: &Expr) -> Option<Vec<String>> {
    match expr {
        Expr::Array { elements, .. } => {
            let mut out = Vec::new();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => out.push(string_lit(e)?),
                    _ => return None,
                }
            }
            if out.is_empty() {
                return None;
            }
            Some(out)
        }
        _ => None,
    }
}

fn env_object_lit(expr: &Expr) -> Option<Vec<(String, String)>> {
    match expr {
        Expr::Object { properties, .. } => {
            let mut out = Vec::new();
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key: ObjectPropKey::Static(k),
                        value,
                    } => {
                        let key = k.to_string_lossy();
                        let val = string_lit(value)?;
                        out.push((key, val));
                    }
                    _ => return None,
                }
            }
            Some(out)
        }
        Expr::Null { .. } => Some(Vec::new()),
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
    next_label: u32,
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
            next_label: 0,
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

    fn fresh_label(&mut self, tag: &str) -> String {
        let n = self.next_label;
        self.next_label += 1;
        format!("{tag}{n}")
    }

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .by_id
            .get(&id)
            .map(|l| l.name.as_str())
            .ok_or_else(|| diag("host_subprocess: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some(g) = self.str_globals.get(s) {
            return g.clone();
        }
        let g = format!(".hs.str.{}", self.str_globals.len());
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
            "; Draconic LLVM host_subprocess (H15.01 processRun / H15.02 spawn)"
        )
        .ok();
        let mut decls = vec![GC_INIT, PRINT_F64, PRINT_STR, PRINT_BOOL];
        if info.uses_run {
            decls.push(HOST_PROCESS_RUN);
        }
        if info.uses_spawn {
            decls.extend_from_slice(&[
                HOST_PROCESS_SPAWN,
                HOST_PROCESS_STDIN_WRITE,
                HOST_PROCESS_WAIT,
                HOST_PROCESS_STDOUT,
                HOST_PROCESS_STDERR,
                HOST_PROCESS_KILL,
                HOST_PROCESS_CLOSE,
            ]);
        }
        self.out.push_str(&llvm_declares(&decls));
        if info.uses_spawn {
            writeln!(self.out, "declare i32 @strcmp(ptr, ptr)").ok();
        }
        writeln!(self.out).ok();

        for (id, ty) in &info.slots {
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

        for (id, kind) in &info.print_locals {
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
                    .ok_or_else(|| diag("host_subprocess: declare unknown slot"))?;
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
            _ => Err(diag("host_subprocess: unsupported statement")),
        }
    }

    fn emit_argv_cwd_env(
        &mut self,
        args: &[Arg],
    ) -> Result<(i32, String, String, i32, String, String), Diagnostic> {
        let argv_expr = arg_expr(&args[0]).ok_or_else(|| diag("process argv"))?;
        let argv =
            string_array_lit(argv_expr).ok_or_else(|| diag("process argv must be string[] lit"))?;
        let argc = argv.len() as i32;

        let argv_arr = self.fresh();
        writeln!(
            self.body,
            "  {argv_arr} = alloca [{argc} x ptr], align 8"
        )
        .ok();
        for (i, s) in argv.iter().enumerate() {
            let p = self.emit_cstr_ptr(s);
            let ep = self.fresh();
            writeln!(
                self.body,
                "  {ep} = getelementptr inbounds [{argc} x ptr], ptr {argv_arr}, i64 0, i64 {i}"
            )
            .ok();
            writeln!(self.body, "  store ptr {p}, ptr {ep}").ok();
        }
        let argv_ptr = self.fresh();
        writeln!(
            self.body,
            "  {argv_ptr} = getelementptr inbounds [{argc} x ptr], ptr {argv_arr}, i64 0, i64 0"
        )
        .ok();

        let cwd_ptr = if args.len() >= 2 {
            match arg_expr(&args[1]).ok_or_else(|| diag("process cwd"))? {
                Expr::Null { .. } => "null".to_string(),
                Expr::String { value, .. } => {
                    let s = value.to_string_lossy();
                    self.emit_cstr_ptr(&s)
                }
                _ => return Err(diag("process cwd must be string or null")),
            }
        } else {
            "null".to_string()
        };

        let (env_n, keys_ptr, vals_ptr) = if args.len() == 3 {
            let env_expr = arg_expr(&args[2]).ok_or_else(|| diag("process env"))?;
            let pairs = env_object_lit(env_expr).ok_or_else(|| diag("process env object"))?;
            if pairs.is_empty() {
                (0i32, "null".to_string(), "null".to_string())
            } else {
                let n = pairs.len() as i32;
                let keys_arr = self.fresh();
                let vals_arr = self.fresh();
                writeln!(self.body, "  {keys_arr} = alloca [{n} x ptr], align 8").ok();
                writeln!(self.body, "  {vals_arr} = alloca [{n} x ptr], align 8").ok();
                for (i, (k, v)) in pairs.iter().enumerate() {
                    let kp = self.emit_cstr_ptr(k);
                    let vp = self.emit_cstr_ptr(v);
                    let ek = self.fresh();
                    let ev = self.fresh();
                    writeln!(
                        self.body,
                        "  {ek} = getelementptr inbounds [{n} x ptr], ptr {keys_arr}, i64 0, i64 {i}"
                    )
                    .ok();
                    writeln!(self.body, "  store ptr {kp}, ptr {ek}").ok();
                    writeln!(
                        self.body,
                        "  {ev} = getelementptr inbounds [{n} x ptr], ptr {vals_arr}, i64 0, i64 {i}"
                    )
                    .ok();
                    writeln!(self.body, "  store ptr {vp}, ptr {ev}").ok();
                }
                let kptr = self.fresh();
                let vptr = self.fresh();
                writeln!(
                    self.body,
                    "  {kptr} = getelementptr inbounds [{n} x ptr], ptr {keys_arr}, i64 0, i64 0"
                )
                .ok();
                writeln!(
                    self.body,
                    "  {vptr} = getelementptr inbounds [{n} x ptr], ptr {vals_arr}, i64 0, i64 0"
                )
                .ok();
                (n, kptr, vptr)
            }
        } else {
            (0i32, "null".to_string(), "null".to_string())
        };

        Ok((argc, argv_ptr, cwd_ptr, env_n, keys_ptr, vals_ptr))
    }

    fn emit_process_run(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let (argc, argv_ptr, cwd_ptr, env_n, keys_ptr, vals_ptr) = self.emit_argv_cwd_env(args)?;
        let code_i32 = self.fresh();
        let code_f = self.fresh();
        writeln!(
            self.body,
            "  {code_i32} = call i32 @{}(i32 {argc}, ptr {argv_ptr}, ptr {cwd_ptr}, i32 {env_n}, ptr {keys_ptr}, ptr {vals_ptr})",
            HOST_PROCESS_RUN.symbol
        )
        .ok();
        writeln!(self.body, "  {code_f} = sitofp i32 {code_i32} to double").ok();
        Ok(code_f)
    }

    fn emit_process_spawn(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let (argc, argv_ptr, cwd_ptr, env_n, keys_ptr, vals_ptr) = self.emit_argv_cwd_env(args)?;
        let h_i32 = self.fresh();
        let h_f = self.fresh();
        writeln!(
            self.body,
            "  {h_i32} = call i32 @{}(i32 {argc}, ptr {argv_ptr}, ptr {cwd_ptr}, i32 {env_n}, ptr {keys_ptr}, ptr {vals_ptr})",
            HOST_PROCESS_SPAWN.symbol
        )
        .ok();
        writeln!(self.body, "  {h_f} = sitofp i32 {h_i32} to double").ok();
        Ok(h_f)
    }

    fn emit_handle_i32(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let f = self.emit_number_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {f} to i32").ok();
        Ok(i)
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
            Expr::Call { callee, args, .. } if is_named_callee(callee, "processRun") => {
                self.emit_process_run(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "processSpawn") => {
                self.emit_process_spawn(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "processStdinWrite") => {
                if args.len() != 2 {
                    return Err(diag("processStdinWrite expects handle, text"));
                }
                let h = self.emit_handle_i32(
                    arg_expr(&args[0]).ok_or_else(|| diag("processStdinWrite handle"))?,
                )?;
                let text = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("processStdinWrite text"))?,
                )?;
                let rc = self.fresh();
                let f = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i32 {h}, ptr {text}, i64 -1)",
                    HOST_PROCESS_STDIN_WRITE.symbol
                )
                .ok();
                writeln!(self.body, "  {f} = sitofp i32 {rc} to double").ok();
                Ok(f)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "processWait") => {
                if args.len() != 1 {
                    return Err(diag("processWait expects handle"));
                }
                let h = self.emit_handle_i32(
                    arg_expr(&args[0]).ok_or_else(|| diag("processWait handle"))?,
                )?;
                let rc = self.fresh();
                let f = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i32 {h})",
                    HOST_PROCESS_WAIT.symbol
                )
                .ok();
                writeln!(self.body, "  {f} = sitofp i32 {rc} to double").ok();
                Ok(f)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "processKill") => {
                if args.len() != 1 {
                    return Err(diag("processKill expects handle"));
                }
                let h = self.emit_handle_i32(
                    arg_expr(&args[0]).ok_or_else(|| diag("processKill handle"))?,
                )?;
                let rc = self.fresh();
                let f = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i32 {h})",
                    HOST_PROCESS_KILL.symbol
                )
                .ok();
                writeln!(self.body, "  {f} = sitofp i32 {rc} to double").ok();
                Ok(f)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "processClose") => {
                if args.len() != 1 {
                    return Err(diag("processClose expects handle"));
                }
                let h = self.emit_handle_i32(
                    arg_expr(&args[0]).ok_or_else(|| diag("processClose handle"))?,
                )?;
                let rc = self.fresh();
                let f = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i32 {h})",
                    HOST_PROCESS_CLOSE.symbol
                )
                .ok();
                writeln!(self.body, "  {f} = sitofp i32 {rc} to double").ok();
                Ok(f)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_subprocess: expected number expr")),
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
                // Prefer string compare when either side is string-typed.
                if self.expr_is_string(left) || self.expr_is_string(right) {
                    let l = self.emit_string_expr(left)?;
                    let r = self.emit_string_expr(right)?;
                    let cmp = self.fresh();
                    writeln!(
                        self.body,
                        "  {cmp} = call i32 @strcmp(ptr {l}, ptr {r})"
                    )
                    .ok();
                    // Ensure strcmp is declared once via body — add declare in emit_module if needed.
                    let is_eq = matches!(op, BinaryOp::EqEqEq | BinaryOp::EqEq);
                    let z = self.fresh();
                    let pred = if is_eq { "eq" } else { "ne" };
                    writeln!(self.body, "  {z} = icmp {pred} i32 {cmp}, 0").ok();
                    let b = self.fresh();
                    writeln!(self.body, "  {b} = zext i1 {z} to i8").ok();
                    Ok(b)
                } else {
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
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_subprocess: expected bool expr")),
        }
    }

    fn expr_is_string(&self, expr: &Expr) -> bool {
        match expr {
            Expr::String { .. } => true,
            Expr::Call { callee, .. }
                if is_named_callee(callee, "processStdout")
                    || is_named_callee(callee, "processStderr") =>
            {
                true
            }
            Expr::Local { id, .. } => matches!(self.slot_of.get(id), Some(SlotTy::String)),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                ..
            } => true,
            _ => false,
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
            } if is_named_ident(arg, "processRun")
                || is_named_ident(arg, "processSpawn")
                || is_named_ident(arg, "processStdinWrite")
                || is_named_ident(arg, "processWait")
                || is_named_ident(arg, "processStdout")
                || is_named_ident(arg, "processStderr")
                || is_named_ident(arg, "processKill")
                || is_named_ident(arg, "processClose") =>
            {
                Ok(self.emit_cstr_ptr("function"))
            }
            Expr::Call { callee, args, .. }
                if is_named_callee(callee, "processStdout")
                    || is_named_callee(callee, "processStderr") =>
            {
                if args.len() != 1 {
                    return Err(diag("processStdout/Stderr expects handle"));
                }
                let h = self.emit_handle_i32(
                    arg_expr(&args[0]).ok_or_else(|| diag("processStdout/Stderr handle"))?,
                )?;
                let out = self.fresh();
                let rc = self.fresh();
                let sym = if is_named_callee(callee, "processStdout") {
                    HOST_PROCESS_STDOUT.symbol
                } else {
                    HOST_PROCESS_STDERR.symbol
                };
                writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{sym}(i32 {h}, ptr {out})"
                )
                .ok();
                let empty = self.emit_cstr_ptr("");
                let ok_l = self.fresh_label("ps_ok");
                let bad_l = self.fresh_label("ps_bad");
                let join_l = self.fresh_label("ps_join");
                let is_ok = self.fresh();
                writeln!(self.body, "  {is_ok} = icmp eq i32 {rc}, 0").ok();
                writeln!(self.body, "  br i1 {is_ok}, label %{ok_l}, label %{bad_l}").ok();
                writeln!(self.body, "{ok_l}:").ok();
                let v_ok = self.fresh();
                writeln!(self.body, "  {v_ok} = load ptr, ptr {out}").ok();
                writeln!(self.body, "  br label %{join_l}").ok();
                writeln!(self.body, "{bad_l}:").ok();
                writeln!(self.body, "  br label %{join_l}").ok();
                writeln!(self.body, "{join_l}:").ok();
                let phi = self.fresh();
                writeln!(
                    self.body,
                    "  {phi} = phi ptr [ {v_ok}, %{ok_l} ], [ {empty}, %{bad_l} ]"
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
            _ => Err(diag("host_subprocess: expected string expr")),
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
    fn classifies_process_run_exit_cwd_env() {
        let m = lower_src(
            r#"
            let code = processRun(["/bin/sh", "-c", "exit 42"]);
            let ok = code === 42;
            let cwd_ok = processRun(["/bin/sh", "-c", "[ \"$(pwd)\" = \"/tmp\" ]"], "/tmp") === 0;
            let env_ok = processRun(["/bin/sh", "-c", "test \"$DRACONIC_H1501\" = hi"], null, { DRACONIC_H1501: "hi" }) === 0;
            "#,
        );
        assert!(is_host_subprocess_module(&m));
        let ir = emit_host_subprocess(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_process_run"), "{ir}");
        let dir = std::env::temp_dir().join(format!(
            "draconic-hs-ir-{}",
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
    fn classifies_process_spawn_capture_kill() {
        let m = lower_src(
            r#"
            let h = processSpawn(["/bin/sh", "-c", "cat"]);
            let w = processStdinWrite(h, "hi");
            let code = processWait(h);
            let out = processStdout(h);
            let err = processStderr(h);
            let ok = out === "hi";
            let h2 = processSpawn(["/bin/sh", "-c", "sleep 30"]);
            let k = processKill(h2);
            let c2 = processWait(h2);
            let closed = processClose(h);
            "#,
        );
        assert!(is_host_subprocess_module(&m));
        let ir = emit_host_subprocess(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_process_spawn"), "{ir}");
        assert!(ir.contains("draconic_rt_host_process_stdin_write"), "{ir}");
        assert!(ir.contains("draconic_rt_host_process_wait"), "{ir}");
        assert!(ir.contains("draconic_rt_host_process_stdout"), "{ir}");
        assert!(ir.contains("draconic_rt_host_process_kill"), "{ir}");
        assert!(ir.contains("declare i32 @strcmp"), "{ir}");
        let dir = std::env::temp_dir().join(format!(
            "draconic-hs-spawn-{}",
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
