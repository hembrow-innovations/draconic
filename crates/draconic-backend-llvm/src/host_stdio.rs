//! H02.01–H02.03: native observations for host stdio.
//!
//! - `stdoutWrite` / `stderrWrite` — string or Uint8Array
//! - `stdinReadLine()` — maybe-string (null at EOF); auto-printed via `print_str`
//! - `stdinReadBytes(n)` — dynamic-length bytes; `.length` + write

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_STDERR_WRITE, HOST_STDIN_READ_BYTES, HOST_STDIN_READ_LINE,
    HOST_STDOUT_WRITE, PRINT_F64, PRINT_STR,
};

pub(crate) fn is_host_stdio_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_stdio(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_stdio module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    /// `new Uint8Array(n)` fixed backing store.
    Bytes(usize),
    /// `stdinReadBytes(n)` result: data ptr + actual len.
    DynBytes,
    /// `stdinReadLine()` → C string or null.
    MaybeString,
    /// Number (e.g. `.length`).
    Number,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    needs_stdin_line: bool,
    needs_stdin_bytes: bool,
    needs_write: bool,
}

struct ClassifyCtx<'a> {
    module: &'a Module,
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    needs_stdin_line: bool,
    needs_stdin_bytes: bool,
    needs_write: bool,
    has_stdio: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        module,
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        needs_stdin_line: false,
        needs_stdin_bytes: false,
        needs_write: false,
        has_stdio: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_stdio {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
        needs_stdin_line: ctx.needs_stdin_line,
        needs_stdin_bytes: ctx.needs_stdin_bytes,
        needs_write: ctx.needs_write,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            match ty {
                SlotTy::MaybeString | SlotTy::Number => {
                    ctx.print_locals.push((*local, ty));
                }
                SlotTy::Bytes(_) | SlotTy::DynBytes => {}
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && (is_named_callee(callee, "stdoutWrite", ctx.module)
                    || is_named_callee(callee, "stderrWrite", ctx.module)) =>
        {
            ctx.has_stdio = true;
            ctx.needs_write = true;
            classify_write_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    computed: true,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let obj_ty = classify_expr(object, ctx)?;
            let idx = number_lit_usize(property)?;
            let _byte = number_lit_u8(value)?;
            match obj_ty {
                SlotTy::Bytes(n) if idx < n => Some(()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_write_arg(expr: &Expr, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Bytes(_) | SlotTy::DynBytes => Some(()),
            _ => None,
        },
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx<'_>) -> Option<SlotTy> {
    match expr {
        Expr::New { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "Uint8Array", ctx.module) =>
        {
            let n = number_lit_usize(arg_expr(&args[0])?)?;
            Some(SlotTy::Bytes(n))
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "stdinReadLine", ctx.module) =>
        {
            ctx.has_stdio = true;
            ctx.needs_stdin_line = true;
            Some(SlotTy::MaybeString)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdinReadBytes", ctx.module) =>
        {
            let n = number_lit_usize(arg_expr(&args[0])?)?;
            if n > (isize::MAX as usize) {
                return None;
            }
            ctx.has_stdio = true;
            ctx.needs_stdin_bytes = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let obj = classify_expr(object, ctx)?;
            let prop = string_lit(property)?;
            if prop == "length" {
                match obj {
                    SlotTy::Bytes(_) | SlotTy::DynBytes => Some(SlotTy::Number),
                    _ => None,
                }
            } else {
                None
            }
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            classify_expr(arg, ctx)?;
            // typeof result is a string; print as string via typeof emit path.
            // Store as MaybeString? Better: treat as String printed via PRINT_STR.
            // Use Number path won't work. Reuse MaybeString only for nullability.
            // Host_process uses String slot for typeof. Add String = always present.
            // Simpler: typeof of maybe-string → store as MaybeString no - typeof never null.
            // Use a dedicated approach: classify as Number is wrong.
            // I'll emit typeof into a string global and store as "string slot" via MaybeString
            // but always non-null. PRINT_STR works.
            let _ = arg;
            Some(SlotTy::MaybeString) // reused: non-null cstr from typeof
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::MaybeString),
        _ => None,
    }
}

fn number_lit_usize(expr: &Expr) -> Option<usize> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().ok()?;
            if n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n <= (usize::MAX as f64) {
                Some(n as usize)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn number_lit_u8(expr: &Expr) -> Option<u8> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().ok()?;
            if n.is_finite() && n >= 0.0 && n <= 255.0 && n.fract() == 0.0 {
                Some(n as u8)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy()),
        Expr::IdentName { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn is_named_callee(expr: &Expr, want: &str, module: &Module) -> bool {
    match expr {
        Expr::IdentName { name, .. } => name == want,
        Expr::Local { id, .. } => module
            .locals
            .iter()
            .find(|l| l.id == *id)
            .is_some_and(|l| l.name == want),
        _ => false,
    }
}

fn arg_expr(arg: &Arg) -> Option<&Expr> {
    match arg {
        Arg::Expr(e) => Some(e),
        Arg::Spread(_) => None,
    }
}

struct Emitter<'a> {
    module: &'a Module,
    by_id: HashMap<LocalId, &'a Local>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    needs_stdin_line: bool,
    needs_stdin_bytes: bool,
    needs_write: bool,
    body: String,
    out: String,
    next_tmp: u32,
    /// hex → (global name, len) for write string payloads
    str_globals: HashMap<String, (String, usize)>,
    /// cstr content → global name (NUL-terminated)
    cstr_globals: HashMap<String, String>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &ModuleInfo) -> Self {
        let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
        let slot_of: HashMap<LocalId, SlotTy> = info.slots.iter().copied().collect();
        Self {
            module,
            by_id,
            slot_of,
            print_locals: info.print_locals.clone(),
            needs_stdin_line: info.needs_stdin_line,
            needs_stdin_bytes: info.needs_stdin_bytes,
            needs_write: info.needs_write,
            body: String::new(),
            out: String::new(),
            next_tmp: 0,
            str_globals: HashMap::new(),
            cstr_globals: HashMap::new(),
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
            .ok_or_else(|| diag("host_stdio: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn slot_len_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .by_id
            .get(&id)
            .map(|l| l.name.as_str())
            .ok_or_else(|| diag("host_stdio: unknown local"))?;
        Ok(format!("%slot_{name}_len"))
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM host_stdio (H02.01–H02.03 stdout/stderr/stdin)"
        )
        .ok();
        let mut decls = vec![GC_INIT];
        let push_unique = |decls: &mut Vec<_>, f: draconic_runtime::abi::AbiFn| {
            if !decls.iter().any(|d: &draconic_runtime::abi::AbiFn| d.symbol == f.symbol) {
                decls.push(f);
            }
        };
        if self.needs_write || self.needs_stdin_bytes {
            push_unique(&mut decls, HOST_STDOUT_WRITE);
            push_unique(&mut decls, HOST_STDERR_WRITE);
        }
        if self.needs_stdin_line {
            push_unique(&mut decls, HOST_STDIN_READ_LINE);
        }
        if self.needs_stdin_bytes {
            push_unique(&mut decls, HOST_STDIN_READ_BYTES);
        }
        if !self.print_locals.is_empty()
            || self.needs_stdin_line
            || self
                .print_locals
                .iter()
                .any(|(_, t)| matches!(t, SlotTy::MaybeString))
        {
            push_unique(&mut decls, PRINT_STR);
        }
        if self
            .print_locals
            .iter()
            .any(|(_, t)| matches!(t, SlotTy::Number))
        {
            push_unique(&mut decls, PRINT_F64);
        }
        self.out.push_str(&llvm_declares(&decls));
        writeln!(self.out).ok();

        let needs_memset = self
            .slot_of
            .values()
            .any(|t| matches!(t, SlotTy::Bytes(n) if *n > 0));
        if needs_memset {
            writeln!(
                self.out,
                "declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg)"
            )
            .ok();
            writeln!(self.out).ok();
        }

        for (id, ty) in &self.slot_of.clone() {
            let ptr = self.slot_ptr(*id)?;
            match ty {
                SlotTy::Bytes(n) => {
                    if *n == 0 {
                        writeln!(self.body, "  {ptr} = alloca [1 x i8], align 1").ok();
                    } else {
                        writeln!(self.body, "  {ptr} = alloca [{n} x i8], align 1").ok();
                        let cast = self.fresh();
                        writeln!(
                            self.body,
                            "  {cast} = getelementptr inbounds [{n} x i8], ptr {ptr}, i64 0, i64 0"
                        )
                        .ok();
                        writeln!(
                            self.body,
                            "  call void @llvm.memset.p0.i64(ptr {cast}, i8 0, i64 {n}, i1 false)"
                        )
                        .ok();
                    }
                }
                SlotTy::DynBytes => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    let lp = self.slot_len_ptr(*id)?;
                    writeln!(self.body, "  {lp} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                    writeln!(self.body, "  store i64 0, ptr {lp}").ok();
                }
                SlotTy::MaybeString => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                }
                SlotTy::Number => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                    writeln!(self.body, "  store double 0.0, ptr {ptr}").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, kind) in &self.print_locals.clone() {
            let ptr = self.slot_ptr(*id)?;
            match kind {
                SlotTy::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::MaybeString => {
                    self.emit_print_maybe_string(ptr)?;
                }
                _ => {}
            }
        }

        let body = std::mem::take(&mut self.body);

        for (hex_key, (gname, n)) in &self.str_globals {
            let bytes = hex_decode(hex_key).unwrap_or_default();
            assert_eq!(bytes.len(), *n);
            let esc = escape_llvm_bytes(&bytes);
            if *n == 0 {
                writeln!(
                    self.out,
                    "@{gname} = private unnamed_addr constant [1 x i8] zeroinitializer, align 1"
                )
                .ok();
            } else {
                writeln!(
                    self.out,
                    "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\", align 1"
                )
                .ok();
            }
        }
        if !self.str_globals.is_empty() {
            writeln!(self.out).ok();
        }

        for (content, gname) in &self.cstr_globals {
            let n = content.len() + 1;
            let esc = escape_llvm_bytes(content.as_bytes());
            writeln!(
                self.out,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        if !self.cstr_globals.is_empty() {
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

    fn emit_print_maybe_string(&mut self, slot_ptr: String) -> Result<(), Diagnostic> {
        let v = self.fresh();
        let is_null = self.fresh();
        // EOF / null prints as JS `null`.
        let nul = self.emit_cstr_ptr("null");
        let join = format!("ms_join_{}", self.next_tmp);
        let use_v = format!("ms_use_{}", self.next_tmp);
        let end = format!("ms_end_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {v} = load ptr, ptr {slot_ptr}").ok();
        writeln!(self.body, "  {is_null} = icmp eq ptr {v}, null").ok();
        writeln!(self.body, "  br i1 {is_null}, label %{join}, label %{use_v}").ok();
        writeln!(self.body, "{use_v}:").ok();
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
        writeln!(self.body, "  br label %{end}").ok();
        writeln!(self.body, "{join}:").ok();
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {nul}"))).ok();
        writeln!(self.body, "  br label %{end}").ok();
        writeln!(self.body, "{end}:").ok();
        Ok(())
    }

    fn emit_cstr_ptr(&mut self, s: &str) -> String {
        let g = if let Some(g) = self.cstr_globals.get(s) {
            g.clone()
        } else {
            let g = format!(".hs.cstr.{}", self.cstr_globals.len());
            self.cstr_globals.insert(s.to_string(), g.clone());
            g
        };
        let p = self.fresh();
        let n = s.len() + 1;
        writeln!(
            self.body,
            "  {p} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0"
        )
        .ok();
        p
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                match init {
                    Expr::New { callee, args, .. }
                        if args.len() == 1
                            && is_named_callee(callee, "Uint8Array", self.module) =>
                    {
                        let _ = local;
                        Ok(())
                    }
                    Expr::Call { callee, args, .. }
                        if args.is_empty()
                            && is_named_callee(callee, "stdinReadLine", self.module) =>
                    {
                        let v = self.fresh();
                        writeln!(
                            self.body,
                            "  {}",
                            HOST_STDIN_READ_LINE.call_to(&v, "")
                        )
                        .ok();
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                        Ok(())
                    }
                    Expr::Call { callee, args, .. }
                        if args.len() == 1
                            && is_named_callee(callee, "stdinReadBytes", self.module) =>
                    {
                        let max = number_lit_usize(
                            arg_expr(&args[0])
                                .ok_or_else(|| diag("host_stdio: stdinReadBytes arg"))?,
                        )
                        .ok_or_else(|| diag("host_stdio: stdinReadBytes max must be lit"))?;
                        let data_slot = self.slot_ptr(*local)?;
                        let len_slot = self.slot_len_ptr(*local)?;
                        let out_data = self.fresh();
                        let out_len = self.fresh();
                        let rc = self.fresh();
                        writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                        writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                        writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                        writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                        writeln!(
                            self.body,
                            "  {rc} = call i32 @{}(i64 {max}, ptr {out_data}, ptr {out_len})",
                            HOST_STDIN_READ_BYTES.symbol
                        )
                        .ok();
                        let d = self.fresh();
                        let n = self.fresh();
                        writeln!(self.body, "  {d} = load ptr, ptr {out_data}").ok();
                        writeln!(self.body, "  {n} = load i64, ptr {out_len}").ok();
                        writeln!(self.body, "  store ptr {d}, ptr {data_slot}").ok();
                        writeln!(self.body, "  store i64 {n}, ptr {len_slot}").ok();
                        Ok(())
                    }
                    Expr::Member {
                        object,
                        property,
                        computed: false,
                        ..
                    } => {
                        let prop = string_lit(property)
                            .ok_or_else(|| diag("host_stdio: length prop"))?;
                        if prop != "length" {
                            return Err(diag("host_stdio: only .length"));
                        }
                        let id = match object.as_ref() {
                            Expr::Local { id, .. } => *id,
                            _ => return Err(diag("host_stdio: length object must be local")),
                        };
                        let n = match self.slot_of.get(&id) {
                            Some(SlotTy::Bytes(n)) => *n as f64,
                            Some(SlotTy::DynBytes) => {
                                let lp = self.slot_len_ptr(id)?;
                                let iv = self.fresh();
                                let fv = self.fresh();
                                writeln!(self.body, "  {iv} = load i64, ptr {lp}").ok();
                                writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                                let ptr = self.slot_ptr(*local)?;
                                writeln!(self.body, "  store double {fv}, ptr {ptr}").ok();
                                return Ok(());
                            }
                            _ => return Err(diag("host_stdio: .length on non-bytes")),
                        };
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store double {n}, ptr {ptr}").ok();
                        Ok(())
                    }
                    Expr::Unary {
                        op: UnaryOp::TypeOf,
                        arg,
                        ..
                    } => {
                        let s = self.emit_typeof_cstr(arg)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {s}, ptr {ptr}").ok();
                        Ok(())
                    }
                    _ => Err(diag("host_stdio: unsupported declare")),
                }
            }
            Stmt::Expr { expr, .. } => self.emit_side_effect(expr),
            _ => Err(diag("host_stdio: unsupported statement")),
        }
    }

    fn emit_typeof_cstr(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        match arg {
            Expr::Local { id, .. } => match self.slot_of.get(id) {
                Some(SlotTy::MaybeString) => {
                    let ptr = self.slot_ptr(*id)?;
                    let v = self.fresh();
                    let is_null = self.fresh();
                    let join = format!("tof_join_{}", self.next_tmp);
                    let use_s = format!("tof_s_{}", self.next_tmp);
                    let end = format!("tof_end_{}", self.next_tmp);
                    self.next_tmp += 1;
                    let s_str = self.emit_cstr_ptr("string");
                    // typeof null === "object" (JS).
                    let s_obj = self.emit_cstr_ptr("object");
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {is_null} = icmp eq ptr {v}, null").ok();
                    writeln!(self.body, "  br i1 {is_null}, label %{join}, label %{use_s}").ok();
                    writeln!(self.body, "{use_s}:").ok();
                    writeln!(self.body, "  br label %{end}").ok();
                    writeln!(self.body, "{join}:").ok();
                    writeln!(self.body, "  br label %{end}").ok();
                    writeln!(self.body, "{end}:").ok();
                    let phi = self.fresh();
                    writeln!(
                        self.body,
                        "  {phi} = phi ptr [ {s_str}, %{use_s} ], [ {s_obj}, %{join} ]"
                    )
                    .ok();
                    Ok(phi)
                }
                Some(SlotTy::Bytes(_) | SlotTy::DynBytes) => Ok(self.emit_cstr_ptr("object")),
                Some(SlotTy::Number) => Ok(self.emit_cstr_ptr("number")),
                None => Err(diag("host_stdio: typeof unknown local")),
            },
            _ => Err(diag("host_stdio: typeof unsupported")),
        }
    }

    fn emit_side_effect(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "stdoutWrite", self.module) =>
            {
                self.emit_stream_write(
                    HOST_STDOUT_WRITE.symbol,
                    arg_expr(&args[0]).ok_or_else(|| diag("host_stdio: stdoutWrite arg"))?,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "stderrWrite", self.module) =>
            {
                self.emit_stream_write(
                    HOST_STDERR_WRITE.symbol,
                    arg_expr(&args[0]).ok_or_else(|| diag("host_stdio: stderrWrite arg"))?,
                )
            }
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object,
                        property,
                        computed: true,
                        ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_stdio: assign object must be local")),
                };
                let SlotTy::Bytes(n) = *self
                    .slot_of
                    .get(&id)
                    .ok_or_else(|| diag("host_stdio: assign unknown bytes local"))?
                else {
                    return Err(diag("host_stdio: assign into non-fixed bytes"));
                };
                let idx = number_lit_usize(property)
                    .ok_or_else(|| diag("host_stdio: index must be number lit"))?;
                if idx >= n {
                    return Err(diag("host_stdio: index out of range"));
                }
                let byte = number_lit_u8(value)
                    .ok_or_else(|| diag("host_stdio: byte value must be 0..255 lit"))?;
                let base = self.slot_ptr(id)?;
                let ep = self.fresh();
                if n == 0 {
                    return Err(diag("host_stdio: write into empty Uint8Array"));
                }
                writeln!(
                    self.body,
                    "  {ep} = getelementptr inbounds [{n} x i8], ptr {base}, i64 0, i64 {idx}"
                )
                .ok();
                writeln!(self.body, "  store i8 {byte}, ptr {ep}").ok();
                Ok(())
            }
            _ => Err(diag("host_stdio: unsupported side-effect")),
        }
    }

    fn emit_stream_write(&mut self, abi_symbol: &str, arg: &Expr) -> Result<(), Diagnostic> {
        match arg {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let bytes = s.as_bytes();
                let hex_key: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                let g = if let Some((g, _)) = self.str_globals.get(&hex_key) {
                    g.clone()
                } else {
                    let g = format!(".hs.bytes.{}", self.str_globals.len());
                    self.str_globals
                        .insert(hex_key, (g.clone(), bytes.len()));
                    g
                };
                let n = bytes.len();
                let p = self.fresh();
                let rc = self.fresh();
                if n == 0 {
                    writeln!(
                        self.body,
                        "  {p} = getelementptr inbounds [1 x i8], ptr @{g}, i64 0, i64 0"
                    )
                    .ok();
                } else {
                    writeln!(
                        self.body,
                        "  {p} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0"
                    )
                    .ok();
                }
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{abi_symbol}(ptr {p}, i64 {n})"
                )
                .ok();
                Ok(())
            }
            Expr::Local { id, .. } => {
                let ty = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("host_stdio: stream write unknown local"))?;
                match ty {
                    SlotTy::Bytes(n) => {
                        let base = self.slot_ptr(*id)?;
                        let p = self.fresh();
                        let rc = self.fresh();
                        if n == 0 {
                            writeln!(
                                self.body,
                                "  {p} = getelementptr inbounds [1 x i8], ptr {base}, i64 0, i64 0"
                            )
                            .ok();
                        } else {
                            writeln!(
                                self.body,
                                "  {p} = getelementptr inbounds [{n} x i8], ptr {base}, i64 0, i64 0"
                            )
                            .ok();
                        }
                        writeln!(
                            self.body,
                            "  {rc} = call i32 @{abi_symbol}(ptr {p}, i64 {n})"
                        )
                        .ok();
                        Ok(())
                    }
                    SlotTy::DynBytes => {
                        let dp = self.slot_ptr(*id)?;
                        let lp = self.slot_len_ptr(*id)?;
                        let p = self.fresh();
                        let n = self.fresh();
                        let rc = self.fresh();
                        writeln!(self.body, "  {p} = load ptr, ptr {dp}").ok();
                        writeln!(self.body, "  {n} = load i64, ptr {lp}").ok();
                        let is_null = self.fresh();
                        let do_w = format!("w_do_{}", self.next_tmp);
                        let end = format!("w_end_{}", self.next_tmp);
                        self.next_tmp += 1;
                        writeln!(self.body, "  {is_null} = icmp eq ptr {p}, null").ok();
                        writeln!(self.body, "  br i1 {is_null}, label %{end}, label %{do_w}")
                            .ok();
                        writeln!(self.body, "{do_w}:").ok();
                        writeln!(
                            self.body,
                            "  {rc} = call i32 @{abi_symbol}(ptr {p}, i64 {n})"
                        )
                        .ok();
                        writeln!(self.body, "  br label %{end}").ok();
                        writeln!(self.body, "{end}:").ok();
                        Ok(())
                    }
                    _ => Err(diag(
                        "host_stdio: stdoutWrite/stderrWrite expects string or Uint8Array",
                    )),
                }
            }
            _ => Err(diag(
                "host_stdio: stdoutWrite/stderrWrite expects string or Uint8Array",
            )),
        }
    }
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return Some(Vec::new());
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let b = hex.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let hi = hex_nibble(b[i])?;
        let lo = hex_nibble(b[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Some(out)
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn escape_llvm_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
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

    fn clang_ok(ir: &str, tag: &str) {
        let dir = std::env::temp_dir().join(format!("draconic-hs-{tag}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let ll = dir.join("t.ll");
        std::fs::write(&ll, ir).unwrap();
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
    fn classifies_stdout_write_string() {
        let m = lower_src(
            r#"
            stdoutWrite("hello\n");
            stdoutWrite("world\n");
            "#,
        );
        assert!(is_host_stdio_module(&m));
        let ir = emit_host_stdio(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_stdout_write"), "{ir}");
        assert!(ir.contains("define i32 @main()"), "{ir}");
        clang_ok(&ir, "str");
    }

    #[test]
    fn classifies_stdout_write_bytes() {
        let m = lower_src(
            r#"
            let u = new Uint8Array(3);
            u[0] = 65;
            u[1] = 66;
            u[2] = 10;
            stdoutWrite(u);
            "#,
        );
        assert!(is_host_stdio_module(&m));
        let ir = emit_host_stdio(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_stdout_write"), "{ir}");
        clang_ok(&ir, "bytes");
    }

    #[test]
    fn classifies_stderr_write_string() {
        let m = lower_src(
            r#"
            stderrWrite("err\n");
            "#,
        );
        assert!(is_host_stdio_module(&m));
        let ir = emit_host_stdio(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_stderr_write"), "{ir}");
        clang_ok(&ir, "err-str");
    }

    #[test]
    fn classifies_stderr_write_bytes() {
        let m = lower_src(
            r#"
            let u = new Uint8Array(2);
            u[0] = 69;
            u[1] = 10;
            stderrWrite(u);
            "#,
        );
        assert!(is_host_stdio_module(&m));
        let ir = emit_host_stdio(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_stderr_write"), "{ir}");
        clang_ok(&ir, "err-bytes");
    }

    #[test]
    fn classifies_stdin_read_line() {
        let m = lower_src(
            r#"
            let line = stdinReadLine();
            let t = typeof line;
            "#,
        );
        assert!(is_host_stdio_module(&m));
        let ir = emit_host_stdio(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_stdin_read_line"), "{ir}");
        assert!(ir.contains("draconic_rt_print_str"), "{ir}");
        clang_ok(&ir, "stdin-line");
    }

    #[test]
    fn classifies_stdin_read_bytes() {
        let m = lower_src(
            r#"
            let u = stdinReadBytes(3);
            let n = u.length;
            stdoutWrite(u);
            "#,
        );
        assert!(is_host_stdio_module(&m));
        let ir = emit_host_stdio(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_stdin_read_bytes"), "{ir}");
        clang_ok(&ir, "stdin-bytes");
    }
}
