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
//! Grant deny / ABI `HOST_E_PERM`: stderr `EPERM` + exit 1 (R02.02).

use std::collections::HashMap;

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

mod classify;
mod emit;
mod io;
mod values;

use classify::classify;

pub(crate) fn is_host_fs_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn walk_host_fs(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_host_fs_module(module) {
        return None;
    }
    Some(emit_host_fs(module))
}

pub(crate) fn emit_host_fs(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_fs module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LocalSlot {
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
    slots: Vec<(LocalId, LocalSlot)>,
    print_locals: Vec<(LocalId, LocalSlot)>,
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
    slots: Vec<(LocalId, LocalSlot)>,
    slot_of: HashMap<LocalId, LocalSlot>,
    print_locals: Vec<(LocalId, LocalSlot)>,
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

fn catalog_callee(expr: &Expr) -> Option<&'static draconic_check::HostApiEntry> {
    match expr {
        Expr::IdentName { name, .. } => draconic_check::lookup_host_api(name),
        _ => None,
    }
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    catalog_callee(expr).is_some_and(|entry| entry.name == want)
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

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    out: String,
    body: String,
    next_tmp: usize,
    str_globals: Vec<(String, String)>,
    local_name: HashMap<LocalId, String>,
    slot_of: HashMap<LocalId, LocalSlot>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn lower_src(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn classify_fs_idents_resolve_through_catalog() {
        let entry = draconic_check::lookup_host_api("readFileText")
            .expect("readFileText must be a HOST_APIS row");
        assert!(
            entry.note.starts_with("H04"),
            "host_fs claims H04 catalog rows, note={}",
            entry.note
        );
        let src = format!("let t = {}(\"hello.txt\");", entry.name);
        let m = lower_src(&src);
        assert!(is_host_fs_module(&m));
        assert!(draconic_check::lookup_host_api("notAHostApi").is_none());
        for api in draconic_check::host_apis() {
            if !api.note.starts_with("H04") {
                continue;
            }
            assert!(
                draconic_check::lookup_host_api(api.name).is_some(),
                "{} must resolve through the catalog",
                api.name
            );
        }
    }

    #[test]
    fn leftover_read_file_text_emits_via_walker() {
        let m = lower_src(
            r#"
            let t = readFileText("hello.txt");
            "#,
        );
        assert!(
            !crate::es_expr::is_es_expr_module(&m),
            "FS IR must miss is_es_expr_module so the walker lowers it"
        );
        let ir = crate::emit_llvm_ir(&m).expect("walker emit");
        assert!(ir.contains("draconic_rt_host_fs_read_text"), "{ir}");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "leftover host_fs must not use hello stub:\n{ir}"
        );
    }

    #[test]
    fn read_file_text_emits() {
        let m = lower_src(
            r#"
            let t = readFileText("hello.txt");
            "#,
        );
        assert!(is_host_fs_module(&m));
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
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
        let ir = crate::emit_llvm_ir(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_fs_open"), "{ir}");
        assert!(ir.contains("draconic_rt_host_fs_handle_write"), "{ir}");
        assert!(ir.contains("draconic_rt_host_fs_handle_seek"), "{ir}");
        assert!(ir.contains("draconic_rt_host_fs_handle_read"), "{ir}");
        assert!(ir.contains("draconic_rt_host_handle_close"), "{ir}");
    }
}
