//! H04.01–H04.06: native observations for file + directory host APIs.
//!
//! - `readFileText(path)` → string (auto-printed)
//! - `readFileBytes(path)` → dynamic bytes; `.length` + `stdoutWrite`
//! - `writeFileText(path, text)` / `appendFileText(path, text)`
//! - `writeFileBytes(path, data)` / `appendFileBytes(path, data)` (string or DynBytes)
//! - `exists(path)` → bool (auto-printed)
//! - `stat(path)` → Stat; `.size` / `.isFile` / `.isDir` / `.mtime` (+ `>` for mtime check)
//! - `mkdir(path)` / `mkdirAll(path)` / `rmdir(path)` / `removeFile(path)`
//! - `readdir(path)` → string[]; `.length` + index `[i]`
//! - `renameFile(from, to)` / `copyFile(from, to)`
//! - `openFile(path, mode)` → handle; `fileWrite` / `fileRead` / `fileSeek` / `closeFile`
//!
//! Missing path (read/write/stat/dir): stderr `ENOENT` + exit 1 (typed HostError on js).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::BinaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, GC_INIT, HOST_FS_APPEND_FILE,
    HOST_FS_APPEND_TEXT, HOST_FS_COPY_FILE, HOST_FS_EXISTS, HOST_FS_HANDLE_READ,
    HOST_FS_HANDLE_SEEK, HOST_FS_HANDLE_WRITE, HOST_FS_MKDIR, HOST_FS_MKDIR_ALL, HOST_FS_OPEN,
    HOST_FS_READDIR, HOST_FS_READ_FILE, HOST_FS_READ_TEXT, HOST_FS_REMOVE_FILE,
    HOST_FS_RENAME_FILE, HOST_FS_RMDIR, HOST_FS_STAT, HOST_FS_WRITE_FILE, HOST_FS_WRITE_TEXT,
    HOST_HANDLE_CLOSE, HOST_PROCESS_EXIT, HOST_STDERR_WRITE, HOST_STDOUT_WRITE, PRINT_BOOL,
    PRINT_F64, PRINT_STR,
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
    Bool,
    /// Opaque stat result; fields via `.size` / `.isFile` / `.isDir` / `.mtime`.
    Stat,
    /// GC string array from `readdir` (`.length` + index).
    Array,
    /// Open file handle (`openFile`); not auto-printed.
    Handle,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    needs_text: bool,
    needs_bytes: bool,
    needs_write: bool,
    needs_write_text: bool,
    needs_append_text: bool,
    needs_write_bytes: bool,
    needs_append_bytes: bool,
    needs_exists: bool,
    needs_stat: bool,
    needs_mkdir: bool,
    needs_mkdir_all: bool,
    needs_readdir: bool,
    needs_rmdir: bool,
    needs_remove_file: bool,
    needs_rename_file: bool,
    needs_copy_file: bool,
    needs_open: bool,
    needs_handle_read: bool,
    needs_handle_write: bool,
    needs_handle_seek: bool,
    needs_close_file: bool,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    needs_text: bool,
    needs_bytes: bool,
    needs_write: bool,
    needs_write_text: bool,
    needs_append_text: bool,
    needs_write_bytes: bool,
    needs_append_bytes: bool,
    needs_exists: bool,
    needs_stat: bool,
    needs_mkdir: bool,
    needs_mkdir_all: bool,
    needs_readdir: bool,
    needs_rmdir: bool,
    needs_remove_file: bool,
    needs_rename_file: bool,
    needs_copy_file: bool,
    needs_open: bool,
    needs_handle_read: bool,
    needs_handle_write: bool,
    needs_handle_seek: bool,
    needs_close_file: bool,
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
        needs_write_text: false,
        needs_append_text: false,
        needs_write_bytes: false,
        needs_append_bytes: false,
        needs_exists: false,
        needs_stat: false,
        needs_mkdir: false,
        needs_mkdir_all: false,
        needs_readdir: false,
        needs_rmdir: false,
        needs_remove_file: false,
        needs_rename_file: false,
        needs_copy_file: false,
        needs_open: false,
        needs_handle_read: false,
        needs_handle_write: false,
        needs_handle_seek: false,
        needs_close_file: false,
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
        needs_write_text: ctx.needs_write_text,
        needs_append_text: ctx.needs_append_text,
        needs_write_bytes: ctx.needs_write_bytes,
        needs_append_bytes: ctx.needs_append_bytes,
        needs_exists: ctx.needs_exists,
        needs_stat: ctx.needs_stat,
        needs_mkdir: ctx.needs_mkdir,
        needs_mkdir_all: ctx.needs_mkdir_all,
        needs_readdir: ctx.needs_readdir,
        needs_rmdir: ctx.needs_rmdir,
        needs_remove_file: ctx.needs_remove_file,
        needs_rename_file: ctx.needs_rename_file,
        needs_copy_file: ctx.needs_copy_file,
        needs_open: ctx.needs_open,
        needs_handle_read: ctx.needs_handle_read,
        needs_handle_write: ctx.needs_handle_write,
        needs_handle_seek: ctx.needs_handle_seek,
        needs_close_file: ctx.needs_close_file,
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
                SlotTy::String | SlotTy::Number | SlotTy::Bool => {
                    ctx.print_locals.push((*local, ty));
                }
                SlotTy::DynBytes | SlotTy::Stat | SlotTy::Array | SlotTy::Handle => {}
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
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "writeFileText") =>
        {
            ctx.has_fs = true;
            ctx.needs_write_text = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "appendFileText") =>
        {
            ctx.has_fs = true;
            ctx.needs_append_text = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "writeFileBytes") =>
        {
            ctx.has_fs = true;
            ctx.needs_write_bytes = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_or_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "appendFileBytes") =>
        {
            ctx.has_fs = true;
            ctx.needs_append_bytes = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_or_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && (is_named_callee(callee, "exists") || is_named_callee(callee, "stat")) =>
        {
            ctx.has_fs = true;
            if is_named_callee(callee, "exists") {
                ctx.needs_exists = true;
            } else {
                ctx.needs_stat = true;
            }
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "mkdir") => {
            ctx.has_fs = true;
            ctx.needs_mkdir = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "mkdirAll") =>
        {
            ctx.has_fs = true;
            ctx.needs_mkdir_all = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "rmdir") => {
            ctx.has_fs = true;
            ctx.needs_rmdir = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "removeFile") =>
        {
            ctx.has_fs = true;
            ctx.needs_remove_file = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "renameFile") =>
        {
            ctx.has_fs = true;
            ctx.needs_rename_file = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "copyFile") =>
        {
            ctx.has_fs = true;
            ctx.needs_copy_file = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readdir") =>
        {
            ctx.has_fs = true;
            ctx.needs_readdir = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "fileWrite") =>
        {
            ctx.has_fs = true;
            ctx.needs_handle_write = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_or_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if (args.len() == 2 || args.len() == 3) && is_named_callee(callee, "fileSeek") =>
        {
            ctx.has_fs = true;
            ctx.needs_handle_seek = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            if args.len() == 3 {
                classify_number_arg(arg_expr(&args[2])?, ctx)?;
            }
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "closeFile") =>
        {
            ctx.has_fs = true;
            ctx.needs_close_file = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_handle_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Handle | SlotTy::Number => Some(()),
            _ => None,
        },
        _ => None,
    }
}

fn classify_number_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Number { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Number | SlotTy::Handle => Some(()),
            _ => None,
        },
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
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "openFile") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_open = true;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "fileRead") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_handle_read = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "exists") => {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_exists = true;
            Some(SlotTy::Bool)
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "stat") => {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_stat = true;
            Some(SlotTy::Stat)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readdir") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_readdir = true;
            Some(SlotTy::Array)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let obj = match object.as_ref() {
                Expr::Local { id, .. } => ctx.slot_of.get(id).copied()?,
                _ => classify_expr(object, ctx)?,
            };
            let prop = string_lit(property)?;
            match (obj, prop.as_str()) {
                (SlotTy::DynBytes, "length") => Some(SlotTy::Number),
                (SlotTy::Array, "length") => Some(SlotTy::Number),
                (SlotTy::Stat, "size" | "mtime") => Some(SlotTy::Number),
                (SlotTy::Stat, "isFile" | "isDir") => Some(SlotTy::Bool),
                _ => None,
            }
        }
        Expr::Member {
            object,
            property,
            computed: true,
            ..
        } => {
            let obj = match object.as_ref() {
                Expr::Local { id, .. } => ctx.slot_of.get(id).copied()?,
                _ => classify_expr(object, ctx)?,
            };
            if obj != SlotTy::Array {
                return None;
            }
            let idx_ty = classify_expr(property, ctx)?;
            if idx_ty != SlotTy::Number {
                return None;
            }
            Some(SlotTy::String)
        }
        Expr::Binary {
            op: BinaryOp::Gt,
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
        Expr::Number { .. } => Some(SlotTy::Number),
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

fn classify_bytes_or_string_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::String | SlotTy::DynBytes => Some(()),
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

    fn slot_stat_field(&self, id: LocalId, field: &str) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_fs: unknown local"))?;
        Ok(format!("%slot_{name}_{field}"))
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
        writeln!(
            self.out,
            "; Draconic LLVM host_fs (H04.01–H04.06 file + directory + open handle)"
        )
        .ok();
        let mut decls = vec![
            GC_INIT,
            PRINT_STR,
            PRINT_F64,
            PRINT_BOOL,
            HOST_PROCESS_EXIT,
            HOST_STDERR_WRITE,
        ];
        if self.info.needs_text {
            decls.push(HOST_FS_READ_TEXT);
        }
        if self.info.needs_bytes {
            decls.push(HOST_FS_READ_FILE);
        }
        if self.info.needs_write {
            decls.push(HOST_STDOUT_WRITE);
        }
        if self.info.needs_write_text {
            decls.push(HOST_FS_WRITE_TEXT);
        }
        if self.info.needs_append_text {
            decls.push(HOST_FS_APPEND_TEXT);
        }
        if self.info.needs_write_bytes {
            decls.push(HOST_FS_WRITE_FILE);
        }
        if self.info.needs_append_bytes {
            decls.push(HOST_FS_APPEND_FILE);
        }
        if self.info.needs_exists {
            decls.push(HOST_FS_EXISTS);
        }
        if self.info.needs_stat {
            decls.push(HOST_FS_STAT);
        }
        if self.info.needs_mkdir {
            decls.push(HOST_FS_MKDIR);
        }
        if self.info.needs_mkdir_all {
            decls.push(HOST_FS_MKDIR_ALL);
        }
        if self.info.needs_readdir {
            decls.push(HOST_FS_READDIR);
            decls.push(ARRAY_NEW);
            decls.push(ARRAY_SET);
            decls.push(ARRAY_GET);
            decls.push(ARRAY_LEN);
        }
        if self.info.needs_rmdir {
            decls.push(HOST_FS_RMDIR);
        }
        if self.info.needs_remove_file {
            decls.push(HOST_FS_REMOVE_FILE);
        }
        if self.info.needs_rename_file {
            decls.push(HOST_FS_RENAME_FILE);
        }
        if self.info.needs_copy_file {
            decls.push(HOST_FS_COPY_FILE);
        }
        if self.info.needs_open {
            decls.push(HOST_FS_OPEN);
        }
        if self.info.needs_handle_read {
            decls.push(HOST_FS_HANDLE_READ);
        }
        if self.info.needs_handle_write {
            decls.push(HOST_FS_HANDLE_WRITE);
        }
        if self.info.needs_handle_seek {
            decls.push(HOST_FS_HANDLE_SEEK);
        }
        if self.info.needs_close_file {
            decls.push(HOST_HANDLE_CLOSE);
        }
        self.out.push_str(&llvm_declares(&decls));
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            let ptr = self.slot_ptr(*id)?;
            match ty {
                SlotTy::String | SlotTy::Array => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                }
                SlotTy::Number | SlotTy::Handle => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                }
                SlotTy::Bool => {
                    writeln!(self.body, "  {ptr} = alloca i8, align 1").ok();
                }
                SlotTy::DynBytes => {
                    let lp = self.slot_len_ptr(*id)?;
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {lp} = alloca i64, align 8").ok();
                }
                SlotTy::Stat => {
                    let size = self.slot_stat_field(*id, "size")?;
                    let is_file = self.slot_stat_field(*id, "is_file")?;
                    let is_dir = self.slot_stat_field(*id, "is_dir")?;
                    let mtime = self.slot_stat_field(*id, "mtime")?;
                    writeln!(self.body, "  {size} = alloca i64, align 8").ok();
                    writeln!(self.body, "  {is_file} = alloca i32, align 4").ok();
                    writeln!(self.body, "  {is_dir} = alloca i32, align 4").ok();
                    writeln!(self.body, "  {mtime} = alloca double, align 8").ok();
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
                SlotTy::Bool => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
                }
                SlotTy::DynBytes | SlotTy::Stat | SlotTy::Array | SlotTy::Handle => {}
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
                    SlotTy::Handle => {
                        let v = self.emit_handle_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Bool => {
                        let v = self.emit_bool_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store i8 {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Stat => {
                        self.emit_stat_into(*local, init)?;
                    }
                    SlotTy::Array => {
                        let v = self.emit_array_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
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
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "writeFileText") =>
            {
                self.emit_write_text_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: writeFileText path"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: writeFileText text"))?,
                    HOST_FS_WRITE_TEXT.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "appendFileText") =>
            {
                self.emit_write_text_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: appendFileText path"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: appendFileText text"))?,
                    HOST_FS_APPEND_TEXT.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "writeFileBytes") =>
            {
                self.emit_write_bytes_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: writeFileBytes path"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: writeFileBytes data"))?,
                    HOST_FS_WRITE_FILE.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "appendFileBytes") =>
            {
                self.emit_write_bytes_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: appendFileBytes path"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: appendFileBytes data"))?,
                    HOST_FS_APPEND_FILE.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "exists") =>
            {
                let _ = self.emit_bool_expr(expr)?;
                Ok(())
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "stat") =>
            {
                // discard; still checks error
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: stat path"))?,
                )?;
                let out_size = self.fresh();
                let out_file = self.fresh();
                let out_dir = self.fresh();
                let out_mt = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_size} = alloca i64, align 8").ok();
                writeln!(self.body, "  {out_file} = alloca i32, align 4").ok();
                writeln!(self.body, "  {out_dir} = alloca i32, align 4").ok();
                writeln!(self.body, "  {out_mt} = alloca double, align 8").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {out_size}, ptr {out_file}, ptr {out_dir}, ptr {out_mt})",
                    HOST_FS_STAT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "mkdir") =>
            {
                self.emit_path_void_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: mkdir path"))?,
                    HOST_FS_MKDIR.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "mkdirAll") =>
            {
                self.emit_path_void_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: mkdirAll path"))?,
                    HOST_FS_MKDIR_ALL.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "rmdir") =>
            {
                self.emit_path_void_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: rmdir path"))?,
                    HOST_FS_RMDIR.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "removeFile") =>
            {
                self.emit_path_void_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: removeFile path"))?,
                    HOST_FS_REMOVE_FILE.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "renameFile") =>
            {
                self.emit_two_path_void_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: renameFile from"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: renameFile to"))?,
                    HOST_FS_RENAME_FILE.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "copyFile") =>
            {
                self.emit_two_path_void_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: copyFile from"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: copyFile to"))?,
                    HOST_FS_COPY_FILE.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "fileWrite") =>
            {
                self.emit_file_write(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: fileWrite handle"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: fileWrite data"))?,
                )
            }
            Expr::Call { callee, args, .. }
                if (args.len() == 2 || args.len() == 3) && is_named_callee(callee, "fileSeek") =>
            {
                let whence = if args.len() == 3 {
                    Some(arg_expr(&args[2]).ok_or_else(|| diag("host_fs: fileSeek whence"))?)
                } else {
                    None
                };
                self.emit_file_seek(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: fileSeek handle"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: fileSeek offset"))?,
                    whence,
                )?;
                Ok(())
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "closeFile") =>
            {
                self.emit_close_file(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: closeFile handle"))?,
                )
            }
            _ => Err(diag("host_fs: unsupported expr stmt")),
        }
    }

    fn emit_path_void_call(&mut self, path: &Expr, symbol: &str) -> Result<(), Diagnostic> {
        let p = self.emit_string_expr(path)?;
        let rc = self.fresh();
        writeln!(self.body, "  {rc} = call i32 @{symbol}(ptr {p})").ok();
        self.emit_check_rc(&rc)
    }

    fn emit_two_path_void_call(
        &mut self,
        from: &Expr,
        to: &Expr,
        symbol: &str,
    ) -> Result<(), Diagnostic> {
        let a = self.emit_string_expr(from)?;
        let b = self.emit_string_expr(to)?;
        let rc = self.fresh();
        writeln!(self.body, "  {rc} = call i32 @{symbol}(ptr {a}, ptr {b})").ok();
        self.emit_check_rc(&rc)
    }

    fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "readdir") =>
            {
                self.emit_readdir(arg_expr(&args[0]).ok_or_else(|| diag("host_fs: readdir path"))?)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_fs: expected readdir array")),
        }
    }

    fn emit_readdir(&mut self, path: &Expr) -> Result<String, Diagnostic> {
        let p = self.emit_string_expr(path)?;
        let out_names = self.fresh();
        let out_count = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out_names} = alloca ptr, align 8").ok();
        writeln!(self.body, "  {out_count} = alloca i64, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out_names}").ok();
        writeln!(self.body, "  store i64 0, ptr {out_count}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(ptr {p}, ptr {out_names}, ptr {out_count})",
            HOST_FS_READDIR.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let names = self.fresh();
        let n = self.fresh();
        writeln!(self.body, "  {names} = load ptr, ptr {out_names}").ok();
        writeln!(self.body, "  {n} = load i64, ptr {out_count}").ok();
        let arr = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_NEW.call_to(&arr, &format!("i64 {n}"))
        )
        .ok();
        let i_slot = self.fresh();
        let loop_cond = format!("rd_loop_cond_{}", self.next_tmp);
        let loop_body = format!("rd_loop_body_{}", self.next_tmp);
        let loop_end = format!("rd_loop_end_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {i_slot} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {i_slot}").ok();
        writeln!(self.body, "  br label %{loop_cond}").ok();
        writeln!(self.body, "{loop_cond}:").ok();
        let i_load = self.fresh();
        let cmp = self.fresh();
        writeln!(self.body, "  {i_load} = load i64, ptr {i_slot}").ok();
        writeln!(self.body, "  {cmp} = icmp slt i64 {i_load}, {n}").ok();
        writeln!(
            self.body,
            "  br i1 {cmp}, label %{loop_body}, label %{loop_end}"
        )
        .ok();
        writeln!(self.body, "{loop_body}:").ok();
        let name_pp = self.fresh();
        let name_p = self.fresh();
        let i_next = self.fresh();
        writeln!(
            self.body,
            "  {name_pp} = getelementptr inbounds ptr, ptr {names}, i64 {i_load}"
        )
        .ok();
        writeln!(self.body, "  {name_p} = load ptr, ptr {name_pp}").ok();
        writeln!(
            self.body,
            "  call void @{}(ptr {arr}, i64 {i_load}, ptr {name_p})",
            ARRAY_SET.symbol
        )
        .ok();
        writeln!(self.body, "  {i_next} = add i64 {i_load}, 1").ok();
        writeln!(self.body, "  store i64 {i_next}, ptr {i_slot}").ok();
        writeln!(self.body, "  br label %{loop_cond}").ok();
        writeln!(self.body, "{loop_end}:").ok();
        Ok(arr)
    }

    fn emit_write_text_call(
        &mut self,
        path: &Expr,
        text: &Expr,
        symbol: &str,
    ) -> Result<(), Diagnostic> {
        let p = self.emit_string_expr(path)?;
        let t = self.emit_string_expr(text)?;
        let rc = self.fresh();
        writeln!(self.body, "  {rc} = call i32 @{symbol}(ptr {p}, ptr {t})").ok();
        self.emit_check_rc(&rc)
    }

    fn emit_write_bytes_call(
        &mut self,
        path: &Expr,
        data: &Expr,
        symbol: &str,
    ) -> Result<(), Diagnostic> {
        let p = self.emit_string_expr(path)?;
        let (d, n) = self.emit_bytes_ptr_len(data)?;
        let rc = self.fresh();
        writeln!(
            self.body,
            "  {rc} = call i32 @{symbol}(ptr {p}, ptr {d}, i64 {n})"
        )
        .ok();
        self.emit_check_rc(&rc)
    }

    fn emit_bytes_ptr_len(&mut self, expr: &Expr) -> Result<(String, String), Diagnostic> {
        match expr {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let p = self.emit_cstr_ptr(&s);
                Ok((p, s.len().to_string()))
            }
            Expr::Local { id, .. } => match self.slot_of.get(id) {
                Some(SlotTy::DynBytes) => {
                    let dp = self.slot_ptr(*id)?;
                    let lp = self.slot_len_ptr(*id)?;
                    let d = self.fresh();
                    let n = self.fresh();
                    writeln!(self.body, "  {d} = load ptr, ptr {dp}").ok();
                    writeln!(self.body, "  {n} = load i64, ptr {lp}").ok();
                    Ok((d, n))
                }
                Some(SlotTy::String) => {
                    let sp = self.slot_ptr(*id)?;
                    let s = self.fresh();
                    writeln!(self.body, "  {s} = load ptr, ptr {sp}").ok();
                    let n = self.emit_cstr_len(&s)?;
                    Ok((s, n))
                }
                _ => Err(diag("host_fs: bytes arg unsupported")),
            },
            _ => Err(diag("host_fs: bytes arg unsupported")),
        }
    }

    fn emit_cstr_len(&mut self, s: &str) -> Result<String, Diagnostic> {
        let i = self.fresh();
        let ch = self.fresh();
        let is0 = self.fresh();
        let loop_l = format!("wlen_loop_{}", self.next_tmp);
        let done_l = format!("wlen_done_{}", self.next_tmp);
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
        let inc_l = format!("wlen_inc_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  br i1 {is0}, label %{done_l}, label %{inc_l}").ok();
        writeln!(self.body, "{inc_l}:").ok();
        let iv2 = self.fresh();
        let iv3 = self.fresh();
        writeln!(self.body, "  {iv2} = load i64, ptr {i}").ok();
        writeln!(self.body, "  {iv3} = add i64 {iv2}, 1").ok();
        writeln!(self.body, "  store i64 {iv3}, ptr {i}").ok();
        writeln!(self.body, "  br label %{loop_l}").ok();
        writeln!(self.body, "{done_l}:").ok();
        let n = self.fresh();
        writeln!(self.body, "  {n} = load i64, ptr {i}").ok();
        Ok(n)
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
                writeln!(self.body, "  {idx} = fptosi double {idx_f} to i64").ok();
                writeln!(
                    self.body,
                    "  {el} = call ptr @{}(ptr {arr}, i64 {idx})",
                    ARRAY_GET.symbol
                )
                .ok();
                Ok(el)
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
            Expr::Call { callee, args, .. } if is_named_callee(callee, "fileRead") => {
                if args.len() != 2 {
                    return Err(diag("host_fs: fileRead expects 2 args"));
                }
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: fileRead handle"))?,
                )?;
                let max_f = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: fileRead maxLen"))?,
                )?;
                let max_i = self.fresh();
                writeln!(self.body, "  {max_i} = fptosi double {max_f} to i64").ok();
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
                    "  {rc} = call i32 @{}(i64 {h}, i64 {max_i}, ptr {out_data}, ptr {out_len})",
                    HOST_FS_HANDLE_READ.symbol
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
            _ => Err(diag("host_fs: expected readFileBytes or fileRead")),
        }
    }

    fn emit_handle_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if is_named_callee(callee, "openFile") => {
                if args.len() != 2 {
                    return Err(diag("host_fs: openFile expects 2 args"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: openFile path"))?,
                )?;
                let mode = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_fs: openFile mode"))?,
                )?;
                let out_h = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_h} = alloca i64, align 8").ok();
                writeln!(self.body, "  store i64 -1, ptr {out_h}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {mode}, ptr {out_h})",
                    HOST_FS_OPEN.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let iv = self.fresh();
                let fv = self.fresh();
                writeln!(self.body, "  {iv} = load i64, ptr {out_h}").ok();
                writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                Ok(fv)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_fs: expected openFile handle")),
        }
    }

    fn emit_handle_i64(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let f = self.emit_handle_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {f} to i64").ok();
        Ok(i)
    }

    fn emit_file_write(&mut self, handle: &Expr, data: &Expr) -> Result<(), Diagnostic> {
        let h = self.emit_handle_i64(handle)?;
        let (d, n) = self.emit_bytes_ptr_len(data)?;
        let rc = self.fresh();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(i64 {h}, ptr {d}, i64 {n})",
            HOST_FS_HANDLE_WRITE.symbol
        )
        .ok();
        self.emit_check_rc(&rc)
    }

    fn emit_file_seek(
        &mut self,
        handle: &Expr,
        offset: &Expr,
        whence: Option<&Expr>,
    ) -> Result<String, Diagnostic> {
        let h = self.emit_handle_i64(handle)?;
        let off_f = self.emit_number_expr(offset)?;
        let off_i = self.fresh();
        writeln!(self.body, "  {off_i} = fptosi double {off_f} to i64").ok();
        let wh_i = if let Some(w) = whence {
            let wf = self.emit_number_expr(w)?;
            let wi = self.fresh();
            writeln!(self.body, "  {wi} = fptosi double {wf} to i32").ok();
            wi
        } else {
            "0".to_string()
        };
        let out_pos = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out_pos} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {out_pos}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(i64 {h}, i64 {off_i}, i32 {wh_i}, ptr {out_pos})",
            HOST_FS_HANDLE_SEEK.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let iv = self.fresh();
        let fv = self.fresh();
        writeln!(self.body, "  {iv} = load i64, ptr {out_pos}").ok();
        writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
        Ok(fv)
    }

    fn emit_close_file(&mut self, handle: &Expr) -> Result<(), Diagnostic> {
        let h = self.emit_handle_i64(handle)?;
        let rc = self.fresh();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(i64 {h})",
            HOST_HANDLE_CLOSE.symbol
        )
        .ok();
        self.emit_check_rc(&rc)
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                // LLVM double constants need a decimal form (`0` alone is rejected).
                if raw.contains('.') || raw.contains('e') || raw.contains('E') {
                    Ok(raw.clone())
                } else {
                    Ok(format!("{raw}.0"))
                }
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_fs: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_fs: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::DynBytes), "length") => {
                        let lp = self.slot_len_ptr(id)?;
                        let iv = self.fresh();
                        let fv = self.fresh();
                        writeln!(self.body, "  {iv} = load i64, ptr {lp}").ok();
                        writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                        Ok(fv)
                    }
                    (Some(SlotTy::Array), "length") => {
                        let ap = self.slot_ptr(id)?;
                        let arr = self.fresh();
                        let iv = self.fresh();
                        let fv = self.fresh();
                        writeln!(self.body, "  {arr} = load ptr, ptr {ap}").ok();
                        writeln!(
                            self.body,
                            "  {iv} = call i64 @{}(ptr {arr})",
                            ARRAY_LEN.symbol
                        )
                        .ok();
                        writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                        Ok(fv)
                    }
                    (Some(SlotTy::Stat), "size") => {
                        let sp = self.slot_stat_field(id, "size")?;
                        let iv = self.fresh();
                        let fv = self.fresh();
                        writeln!(self.body, "  {iv} = load i64, ptr {sp}").ok();
                        writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                        Ok(fv)
                    }
                    (Some(SlotTy::Stat), "mtime") => {
                        let mp = self.slot_stat_field(id, "mtime")?;
                        let v = self.fresh();
                        writeln!(self.body, "  {v} = load double, ptr {mp}").ok();
                        Ok(v)
                    }
                    _ => Err(diag("host_fs: unsupported number member")),
                }
            }
            _ => Err(diag("host_fs: unsupported number expr")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if is_named_callee(callee, "exists") => {
                if args.len() != 1 {
                    return Err(diag("host_fs: exists expects 1 arg"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: exists path"))?,
                )?;
                let rc = self.fresh();
                let b = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path})",
                    HOST_FS_EXISTS.symbol
                )
                .ok();
                // exists returns 0/1 i32 → i8
                writeln!(self.body, "  {b} = trunc i32 {rc} to i8").ok();
                Ok(b)
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_fs: bool member"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_fs: bool member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::Stat), "isFile") => {
                        let p = self.slot_stat_field(id, "is_file")?;
                        let iv = self.fresh();
                        let b = self.fresh();
                        writeln!(self.body, "  {iv} = load i32, ptr {p}").ok();
                        writeln!(self.body, "  {b} = trunc i32 {iv} to i8").ok();
                        Ok(b)
                    }
                    (Some(SlotTy::Stat), "isDir") => {
                        let p = self.slot_stat_field(id, "is_dir")?;
                        let iv = self.fresh();
                        let b = self.fresh();
                        writeln!(self.body, "  {iv} = load i32, ptr {p}").ok();
                        writeln!(self.body, "  {b} = trunc i32 {iv} to i8").ok();
                        Ok(b)
                    }
                    _ => Err(diag("host_fs: unsupported bool member")),
                }
            }
            Expr::Binary {
                op: BinaryOp::Gt,
                left,
                right,
                ..
            } => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let cmp = self.fresh();
                let b = self.fresh();
                writeln!(self.body, "  {cmp} = fcmp ogt double {l}, {r}").ok();
                writeln!(self.body, "  {b} = zext i1 {cmp} to i8").ok();
                Ok(b)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_fs: unsupported bool expr")),
        }
    }

    fn emit_stat_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if is_named_callee(callee, "stat") => {
                if args.len() != 1 {
                    return Err(diag("host_fs: stat expects 1 arg"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_fs: stat path"))?,
                )?;
                let size = self.slot_stat_field(local, "size")?;
                let is_file = self.slot_stat_field(local, "is_file")?;
                let is_dir = self.slot_stat_field(local, "is_dir")?;
                let mtime = self.slot_stat_field(local, "mtime")?;
                let rc = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {path}, ptr {size}, ptr {is_file}, ptr {is_dir}, ptr {mtime})",
                    HOST_FS_STAT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            _ => Err(diag("host_fs: expected stat")),
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

    #[test]
    fn write_file_text_emits() {
        let m = lower_src(
            r#"
            writeFileText("/tmp/h0402.txt", "wt");
            let t = readFileText("/tmp/h0402.txt");
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_write_text"), "{ir}");
        assert!(ir.contains("draconic_rt_host_fs_read_text"), "{ir}");
    }

    #[test]
    fn append_file_text_emits() {
        let m = lower_src(
            r#"
            writeFileText("/tmp/h0402a.txt", "a");
            appendFileText("/tmp/h0402a.txt", "b");
            let t = readFileText("/tmp/h0402a.txt");
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_append_text"), "{ir}");
    }

    #[test]
    fn write_file_bytes_emits() {
        let m = lower_src(
            r#"
            writeFileBytes("/tmp/h0402b.bin", "xy");
            let u = readFileBytes("/tmp/h0402b.bin");
            let n = u.length;
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_write_file"), "{ir}");
    }

    #[test]
    fn exists_emits() {
        let m = lower_src(
            r#"
            let a = exists("hello.txt");
            let b = exists("__missing__");
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_exists"), "{ir}");
        assert!(ir.contains("draconic_rt_print_bool"), "{ir}");
    }

    #[test]
    fn stat_emits() {
        let m = lower_src(
            r#"
            let s = stat("hello.txt");
            let size = s.size;
            let isF = s.isFile;
            let isD = s.isDir;
            let mtOk = s.mtime > 0;
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_stat"), "{ir}");
        assert!(ir.contains("draconic_rt_print_f64"), "{ir}");
        assert!(ir.contains("draconic_rt_print_bool"), "{ir}");
    }

    #[test]
    fn mkdir_emits() {
        let m = lower_src(
            r#"
            mkdir("/tmp/h0404");
            let a = exists("/tmp/h0404");
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_mkdir"), "{ir}");
    }

    #[test]
    fn mkdir_all_emits() {
        let m = lower_src(
            r#"
            mkdirAll("/tmp/h0404/a/b");
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_mkdir_all"), "{ir}");
    }

    #[test]
    fn readdir_emits() {
        let m = lower_src(
            r#"
            let names = readdir("/tmp/h0404");
            let n = names.length;
            let a0 = names[0];
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_readdir"), "{ir}");
        assert!(ir.contains("draconic_rt_array_new"), "{ir}");
    }

    #[test]
    fn rmdir_and_remove_file_emits() {
        let m = lower_src(
            r#"
            rmdir("/tmp/h0404d");
            removeFile("/tmp/h0404f");
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_rmdir"), "{ir}");
        assert!(ir.contains("draconic_rt_host_fs_remove_file"), "{ir}");
    }

    #[test]
    fn open_handle_emits() {
        let m = lower_src(
            r#"
            let h = openFile("/tmp/h0406.txt", "w+");
            fileWrite(h, "hello-h0406");
            fileSeek(h, 0);
            let u = fileRead(h, 64);
            let n = u.length;
            stdoutWrite(u);
            closeFile(h);
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = emit_host_fs(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_open"), "{ir}");
        assert!(ir.contains("draconic_rt_host_fs_handle_write"), "{ir}");
        assert!(ir.contains("draconic_rt_host_fs_handle_seek"), "{ir}");
        assert!(ir.contains("draconic_rt_host_fs_handle_read"), "{ir}");
        assert!(ir.contains("draconic_rt_host_handle_close"), "{ir}");
    }
}
