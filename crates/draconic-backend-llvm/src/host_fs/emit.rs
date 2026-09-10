use std::fmt::Write as _;

use super::*;
use crate::emitter::escape_llvm_string;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
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

    pub(super) fn finish(self) -> String {
        self.out
    }

    pub(super) fn fresh(&mut self) -> String {
        let n = self.next_tmp;
        self.next_tmp += 1;
        format!("%t{n}")
    }

    pub(super) fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_fs: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    pub(super) fn slot_len_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_fs: unknown local"))?;
        Ok(format!("%slot_{name}_len"))
    }

    pub(super) fn slot_stat_field(&self, id: LocalId, field: &str) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_fs: unknown local"))?;
        Ok(format!("%slot_{name}_{field}"))
    }

    pub(super) fn intern_cstr(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.fs.{}", self.str_globals.len());
        self.str_globals.push((s.to_string(), g.clone()));
        g
    }

    pub(super) fn emit_cstr_ptr(&mut self, s: &str) -> String {
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

    pub(super) fn emit_module(&mut self) -> Result<(), Diagnostic> {
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

    pub(super) fn emit_host_err_exit(&mut self, code: &str) -> Result<(), Diagnostic> {
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

    pub(super) fn emit_check_rc(&mut self, rc: &str) -> Result<(), Diagnostic> {
        let ok = self.fresh();
        let fail = format!("fs_err_{}", self.next_tmp);
        let cont = format!("fs_ok_{}", self.next_tmp);
        self.next_tmp += 1;
        // HOST_OK = 0, HOST_E_NOENT = 2, HOST_E_PERM = 6
        writeln!(self.body, "  {ok} = icmp eq i32 {rc}, 0").ok();
        writeln!(self.body, "  br i1 {ok}, label %{cont}, label %{fail}").ok();
        writeln!(self.body, "{fail}:").ok();
        let is_noent = self.fresh();
        let noent_l = format!("fs_noent_{}", self.next_tmp);
        let not_noent_l = format!("fs_not_noent_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {is_noent} = icmp eq i32 {rc}, 2").ok();
        writeln!(
            self.body,
            "  br i1 {is_noent}, label %{noent_l}, label %{not_noent_l}"
        )
        .ok();
        writeln!(self.body, "{noent_l}:").ok();
        self.emit_host_err_exit("ENOENT")?;
        writeln!(self.body, "{not_noent_l}:").ok();
        let is_perm = self.fresh();
        let perm_l = format!("fs_perm_{}", self.next_tmp);
        let other_l = format!("fs_other_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {is_perm} = icmp eq i32 {rc}, 6").ok();
        writeln!(
            self.body,
            "  br i1 {is_perm}, label %{perm_l}, label %{other_l}"
        )
        .ok();
        writeln!(self.body, "{perm_l}:").ok();
        self.emit_host_err_exit("EPERM")?;
        writeln!(self.body, "{other_l}:").ok();
        self.emit_host_err_exit("EIO")?;
        writeln!(self.body, "{cont}:").ok();
        Ok(())
    }

    pub(super) fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
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

    pub(super) fn emit_side_effect(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
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
}
