//! H04.01: native observations for whole-file read.
//!
//! - `readFileText(path)` → string (auto-printed)
//! - `readFileBytes(path)` → dynamic bytes; `.length` + `stdoutWrite`
//!
//! Missing path: stderr `ENOENT` + exit 1 (typed HostError on js).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_FS_READ_FILE, HOST_FS_READ_TEXT, HOST_PROCESS_EXIT,
    HOST_STDERR_WRITE, HOST_STDOUT_WRITE, PRINT_F64, PRINT_STR,
};

pub(crate) fn is_host_fs_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_fs(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_fs module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    String,
    DynBytes,
    Number,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    needs_text: bool,
    needs_bytes: bool,
    needs_write: bool,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    needs_text: bool,
    needs_bytes: bool,
    needs_write: bool,
    has_fs: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        needs_text: false,
        needs_bytes: false,
        needs_write: false,
        has_fs: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_fs {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
        needs_text: ctx.needs_text,
        needs_bytes: ctx.needs_bytes,
        needs_write: ctx.needs_write,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            match ty {
                SlotTy::String | SlotTy::Number => {
                    ctx.print_locals.push((*local, ty));
                }
                SlotTy::DynBytes => {}
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
        {
            ctx.needs_write = true;
            classify_write_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && (is_named_callee(callee, "readFileText")
                    || is_named_callee(callee, "readFileBytes")) =>
        {
            // bare call as statement (error path / discard)
            ctx.has_fs = true;
            if is_named_callee(callee, "readFileText") {
                ctx.needs_text = true;
            } else {
                ctx.needs_bytes = true;
            }
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_write_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::DynBytes | SlotTy::String => Some(()),
            _ => None,
        },
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readFileText") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_text = true;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readFileBytes") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_bytes = true;
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
                    SlotTy::DynBytes => Some(SlotTy::Number),
                    _ => None,
                }
            } else {
                None
            }
        }
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        _ => None,
    }
}

fn classify_string_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::String => Some(()),
            _ => None,
        },
        _ => None,
    }
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn arg_expr(arg: &Arg) -> Option<&Expr> {
    match arg {
        Arg::Expr(e) => Some(e),
        _ => None,
    }
}

fn string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy().to_string()),
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
    slot_of: HashMap<LocalId, SlotTy>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        let mut local_name = HashMap::new();
        for Local { id, name, .. } in &module.locals {
            local_name.insert(*id, name.clone());
        }
        let mut slot_of = HashMap::new();
        for (id, ty) in &info.slots {
            slot_of.insert(*id, *ty);
        }
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            next_tmp: 0,
            str_globals: Vec::new(),
            local_name,
            slot_of,
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
            .ok_or_else(|| diag("host_fs: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn slot_len_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_fs: unknown local"))?;
        Ok(format!("%slot_{name}_len"))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.fs.{}", self.str_globals.len());
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
        writeln!(self.out, "; Draconic LLVM host_fs (H04.01 file read)").ok();
        let mut decls = vec![GC_INIT, PRINT_STR, PRINT_F64, HOST_PROCESS_EXIT, HOST_STDERR_WRITE];
        if self.info.needs_text {
            decls.push(HOST_FS_READ_TEXT);
        }
        if self.info.needs_bytes {
            decls.push(HOST_FS_READ_FILE);
        }
        if self.info.needs_write {
            decls.push(HOST_STDOUT_WRITE);
        }
        self.out.push_str(&llvm_declares(&decls));
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            let ptr = self.slot_ptr(*id)?;
            match ty {
                SlotTy::String => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                }
                SlotTy::Number => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                }
                SlotTy::DynBytes => {
                    let lp = self.slot_len_ptr(*id)?;
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {lp} = alloca i64, align 8").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, ty) in &self.info.print_locals {
            let ptr = self.slot_ptr(*id)?;
            match ty {
                SlotTy::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotTy::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::DynBytes => {}
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

    fn emit_host_err_exit(&mut self, code: &str) -> Result<(), Diagnostic> {
        let msg = format!("{code}\n");
        let p = self.emit_cstr_ptr(&msg);
        let n = msg.len();
        writeln!(
            self.body,
            "  {}",
            HOST_STDERR_WRITE.call(&format!("ptr {p}, i64 {n}"))
        )
        .ok();
        writeln!(self.body, "  {}", HOST_PROCESS_EXIT.call("i32 1")).ok();
        // unreachable after exit; keep CFG valid
        writeln!(self.body, "  unreachable").ok();
        Ok(())
    }

    fn emit_check_rc(&mut self, rc: &str) -> Result<(), Diagnostic> {
        let ok = self.fresh();
        let fail = format!("fs_err_{}", self.next_tmp);
        let cont = format!("fs_ok_{}", self.next_tmp);
        self.next_tmp += 1;
        // HOST_OK = 0, HOST_E_NOENT = 2
        writeln!(self.body, "  {ok} = icmp eq i32 {rc}, 0").ok();
        writeln!(self.body, "  br i1 {ok}, label %{cont}, label %{fail}").ok();
        writeln!(self.body, "{fail}:").ok();
        let is_noent = self.fresh();
        let noent_l = format!("fs_noent_{}", self.next_tmp);
        let other_l = format!("fs_other_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {is_noent} = icmp eq i32 {rc}, 2").ok();
        writeln!(
            self.body,
            "  br i1 {is_noent}, label %{noent_l}, label %{other_l}"
        )
        .ok();
        writeln!(self.body, "{noent_l}:").ok();
        self.emit_host_err_exit("ENOENT")?;
        writeln!(self.body, "{other_l}:").ok();
        self.emit_host_err_exit("EIO")?;
        writeln!(self.body, "{cont}:").ok();
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                let ty = self
                    .slot_of
                    .get(local)
                    .copied()
                    .ok_or_else(|| diag("host_fs: declare unknown slot"))?;
                match ty {
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::DynBytes => {
                        self.emit_read_bytes_into(*local, init)?;
                    }
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => self.emit_side_effect(expr),
            _ => Err(diag("host_fs: unsupported statement")),
        }
    }

    fn emit_side_effect(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
            {
                self.emit_stdout_write(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: stdoutWrite arg"))?,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "readFileText") =>
            {
                let _ = self.emit_string_expr(expr)?;
                Ok(())
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "readFileBytes") =>
            {
                // discard result; still checks error
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: readFileBytes path"))?,
                )?;
                let out_data = self.fresh();
                let out_len = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {out_data}, ptr {out_len})",
                    HOST_FS_READ_FILE.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                Ok(())
            }
            _ => Err(diag("host_fs: unsupported expr stmt")),
        }
    }

    fn emit_stdout_write(&mut self, arg: &Expr) -> Result<(), Diagnostic> {
        match arg {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let p = self.emit_cstr_ptr(&s);
                let n = s.len();
                writeln!(
                    self.body,
                    "  {}",
                    HOST_STDOUT_WRITE.call(&format!("ptr {p}, i64 {n}"))
                )
                .ok();
                Ok(())
            }
            Expr::Local { id, .. } => match self.slot_of.get(id) {
                Some(SlotTy::DynBytes) => {
                    let dp = self.slot_ptr(*id)?;
                    let lp = self.slot_len_ptr(*id)?;
                    let d = self.fresh();
                    let n = self.fresh();
                    writeln!(self.body, "  {d} = load ptr, ptr {dp}").ok();
                    writeln!(self.body, "  {n} = load i64, ptr {lp}").ok();
                    writeln!(
                        self.body,
                        "  {}",
                        HOST_STDOUT_WRITE.call(&format!("ptr {d}, i64 {n}"))
                    )
                    .ok();
                    Ok(())
                }
                Some(SlotTy::String) => {
                    let sp = self.slot_ptr(*id)?;
                    let s = self.fresh();
                    let n = self.fresh();
                    writeln!(self.body, "  {s} = load ptr, ptr {sp}").ok();
                    // strlen via loop-free: print_str already adds newline; for write use host
                    // Approximate: walk C string length
                    let i = self.fresh();
                    let ch = self.fresh();
                    let is0 = self.fresh();
                    let loop_l = format!("slen_loop_{}", self.next_tmp);
                    let done_l = format!("slen_done_{}", self.next_tmp);
                    self.next_tmp += 1;
                    writeln!(self.body, "  {i} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store i64 0, ptr {i}").ok();
                    writeln!(self.body, "  br label %{loop_l}").ok();
                    writeln!(self.body, "{loop_l}:").ok();
                    let iv = self.fresh();
                    writeln!(self.body, "  {iv} = load i64, ptr {i}").ok();
                    let cp = self.fresh();
                    writeln!(
                        self.body,
                        "  {cp} = getelementptr inbounds i8, ptr {s}, i64 {iv}"
                    )
                    .ok();
                    writeln!(self.body, "  {ch} = load i8, ptr {cp}").ok();
                    writeln!(self.body, "  {is0} = icmp eq i8 {ch}, 0").ok();
                    writeln!(
                        self.body,
                        "  br i1 {is0}, label %{done_l}, label %slen_inc_{}",
                        self.next_tmp
                    )
                    .ok();
                    let inc_l = format!("slen_inc_{}", self.next_tmp);
                    self.next_tmp += 1;
                    writeln!(self.body, "{inc_l}:").ok();
                    let iv2 = self.fresh();
                    let iv3 = self.fresh();
                    writeln!(self.body, "  {iv2} = load i64, ptr {i}").ok();
                    writeln!(self.body, "  {iv3} = add i64 {iv2}, 1").ok();
                    writeln!(self.body, "  store i64 {iv3}, ptr {i}").ok();
                    writeln!(self.body, "  br label %{loop_l}").ok();
                    writeln!(self.body, "{done_l}:").ok();
                    writeln!(self.body, "  {n} = load i64, ptr {i}").ok();
                    writeln!(
                        self.body,
                        "  {}",
                        HOST_STDOUT_WRITE.call(&format!("ptr {s}, i64 {n}"))
                    )
                    .ok();
                    Ok(())
                }
                _ => Err(diag("host_fs: stdoutWrite unsupported arg")),
            },
            _ => Err(diag("host_fs: stdoutWrite unsupported arg")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => Ok(self.emit_cstr_ptr(&value.to_string_lossy())),
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "readFileText") => {
                if args.len() != 1 {
                    return Err(diag("host_fs: readFileText expects 1 arg"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: readFileText path"))?,
                )?;
                let out = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {out})",
                    HOST_FS_READ_TEXT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {out}").ok();
                Ok(v)
            }
            _ => Err(diag("host_fs: unsupported string expr")),
        }
    }

    fn emit_read_bytes_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if is_named_callee(callee, "readFileBytes") => {
                if args.len() != 1 {
                    return Err(diag("host_fs: readFileBytes expects 1 arg"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: readFileBytes path"))?,
                )?;
                let data_slot = self.slot_ptr(local)?;
                let len_slot = self.slot_len_ptr(local)?;
                let out_data = self.fresh();
                let out_len = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_len}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {out_data}, ptr {out_len})",
                    HOST_FS_READ_FILE.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let d = self.fresh();
                let n = self.fresh();
                writeln!(self.body, "  {d} = load ptr, ptr {out_data}").ok();
                writeln!(self.body, "  {n} = load i64, ptr {out_len}").ok();
                writeln!(self.body, "  store ptr {d}, ptr {data_slot}").ok();
                writeln!(self.body, "  store i64 {n}, ptr {len_slot}").ok();
                Ok(())
            }
            _ => Err(diag("host_fs: expected readFileBytes")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_fs: length prop"))?;
                if prop != "length" {
                    return Err(diag("host_fs: only .length"));
                }
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_fs: length object must be local")),
                };
                match self.slot_of.get(&id) {
                    Some(SlotTy::DynBytes) => {
                        let lp = self.slot_len_ptr(id)?;
                        let iv = self.fresh();
                        let fv = self.fresh();
                        writeln!(self.body, "  {iv} = load i64, ptr {lp}").ok();
                        writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                        Ok(fv)
                    }
                    _ => Err(diag("host_fs: .length on non-bytes")),
                }
            }
            _ => Err(diag("host_fs: unsupported number expr")),
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
    fn read_file_text_emits() {
        let m = lower_src(
            r#"
            let t = readFileText("hello.txt");
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_read_text"), "{ir}");
        assert!(ir.contains("draconic_rt_print_str"), "{ir}");
    }

    #[test]
    fn read_file_bytes_emits() {
        let m = lower_src(
            r#"
            let u = readFileBytes("hello.txt");
            let n = u.length;
            stdoutWrite(u);
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_read_file"), "{ir}");
        assert!(ir.contains("draconic_rt_host_stdout_write"), "{ir}");
    }
}
