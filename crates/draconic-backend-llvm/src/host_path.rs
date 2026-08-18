//! H03.01–H03.03: native observations for path string helpers.
//!
//! Path ops via Runtime ABI (`pathResolve` uses cwd). String locals auto-printed
//! via `print_str`; bool locals (`pathIsAbsolute`) via `print_bool`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_PATH_BASENAME, HOST_PATH_DIRNAME, HOST_PATH_EXTNAME,
    HOST_PATH_IS_ABSOLUTE, HOST_PATH_JOIN, HOST_PATH_NORMALIZE, HOST_PATH_RESOLVE, PRINT_BOOL,
    PRINT_STR,
};

pub(crate) fn is_host_path_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_path(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_path module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    String,
    Bool,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    has_path: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_path: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_path || ctx.print_locals.is_empty() {
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
            ctx.print_locals.push((*local, ty));
            Some(())
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. } if is_named_callee(callee, "pathNormalize") => {
            if args.len() != 1 {
                return None;
            }
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_path = true;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "pathJoin") => {
            for a in args {
                classify_string_arg(arg_expr(a)?, ctx)?;
            }
            ctx.has_path = true;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "pathResolve") => {
            for a in args {
                classify_string_arg(arg_expr(a)?, ctx)?;
            }
            ctx.has_path = true;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "pathDirname") => {
            if args.len() != 1 {
                return None;
            }
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_path = true;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "pathBasename") => {
            if args.len() != 1 {
                return None;
            }
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_path = true;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "pathExtname") => {
            if args.len() != 1 {
                return None;
            }
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_path = true;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "pathIsAbsolute") => {
            if args.len() != 1 {
                return None;
            }
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_path = true;
            Some(SlotTy::Bool)
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
            SlotTy::Bool => None,
        },
        // Nested path helpers that yield strings (e.g. pathIsAbsolute(pathResolve(...))).
        Expr::Call { callee, args, .. }
            if is_named_callee(callee, "pathResolve")
                || is_named_callee(callee, "pathJoin")
                || is_named_callee(callee, "pathNormalize")
                || is_named_callee(callee, "pathDirname")
                || is_named_callee(callee, "pathBasename")
                || is_named_callee(callee, "pathExtname") =>
        {
            for a in args {
                classify_string_arg(arg_expr(a)?, ctx)?;
            }
            ctx.has_path = true;
            Some(())
        }
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
            .ok_or_else(|| diag("host_path: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.path.{}", self.str_globals.len());
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
            "; Draconic LLVM host_path (H03 path helpers)"
        )
        .ok();
        let decls = vec![
            GC_INIT,
            PRINT_STR,
            PRINT_BOOL,
            HOST_PATH_NORMALIZE,
            HOST_PATH_JOIN,
            HOST_PATH_DIRNAME,
            HOST_PATH_BASENAME,
            HOST_PATH_EXTNAME,
            HOST_PATH_IS_ABSOLUTE,
            HOST_PATH_RESOLVE,
        ];
        self.out.push_str(&llvm_declares(&decls));
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            let ptr = self.slot_ptr(*id)?;
            let llvm_ty = match ty {
                SlotTy::String => "ptr",
                SlotTy::Bool => "i8",
            };
            writeln!(self.body, "  {ptr} = alloca {llvm_ty}, align 8").ok();
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
                SlotTy::Bool => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
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
                let ptr = self.slot_ptr(*local)?;
                let ty = self
                    .info
                    .slots
                    .iter()
                    .find(|(id, _)| id == local)
                    .map(|(_, t)| *t)
                    .ok_or_else(|| diag("host_path: declare unknown slot"))?;
                match ty {
                    SlotTy::String => {
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
            _ => Err(diag("host_path: unsupported statement")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. } if is_named_callee(callee, "pathIsAbsolute") => {
                if args.len() != 1 {
                    return Err(diag("host_path: pathIsAbsolute expects 1 arg"));
                }
                let a = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_path: pathIsAbsolute arg"))?,
                )?;
                let r32 = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    HOST_PATH_IS_ABSOLUTE.call_to(&r32, &format!("ptr {a}"))
                )
                .ok();
                let r = self.fresh();
                writeln!(self.body, "  {r} = trunc i32 {r32} to i8").ok();
                Ok(r)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_path: unsupported bool expr")),
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
            Expr::Call { callee, args, .. } if is_named_callee(callee, "pathNormalize") => {
                if args.len() != 1 {
                    return Err(diag("host_path: pathNormalize expects 1 arg"));
                }
                let a = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_path: pathNormalize arg"))?,
                )?;
                let r = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    HOST_PATH_NORMALIZE.call_to(&r, &format!("ptr {a}"))
                )
                .ok();
                Ok(r)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "pathJoin") => {
                self.emit_variadic_path_call(args, "pathJoin", HOST_PATH_JOIN)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "pathResolve") => {
                self.emit_variadic_path_call(args, "pathResolve", HOST_PATH_RESOLVE)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "pathDirname") => {
                self.emit_unary_path_call(args, "pathDirname", HOST_PATH_DIRNAME)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "pathBasename") => {
                self.emit_unary_path_call(args, "pathBasename", HOST_PATH_BASENAME)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "pathExtname") => {
                self.emit_unary_path_call(args, "pathExtname", HOST_PATH_EXTNAME)
            }
            _ => Err(diag("host_path: unsupported string expr")),
        }
    }

    fn emit_unary_path_call(
        &mut self,
        args: &[Arg],
        name: &str,
        abi: draconic_runtime::abi::AbiFn,
    ) -> Result<String, Diagnostic> {
        if args.len() != 1 {
            return Err(diag(&format!("host_path: {name} expects 1 arg")));
        }
        let a = self.emit_string_expr(
            arg_expr(&args[0]).ok_or_else(|| diag(&format!("host_path: {name} arg")))?,
        )?;
        let r = self.fresh();
        writeln!(
            self.body,
            "  {}",
            abi.call_to(&r, &format!("ptr {a}"))
        )
        .ok();
        Ok(r)
    }

    fn emit_variadic_path_call(
        &mut self,
        args: &[Arg],
        name: &str,
        abi: draconic_runtime::abi::AbiFn,
    ) -> Result<String, Diagnostic> {
        let n = args.len();
        let r = self.fresh();
        if n == 0 {
            writeln!(
                self.body,
                "  {}",
                abi.call_to(&r, "i64 0, ptr null")
            )
            .ok();
            return Ok(r);
        }
        let arr = self.fresh();
        writeln!(self.body, "  {arr} = alloca [{n} x ptr], align 8").ok();
        for (i, a) in args.iter().enumerate() {
            let s = self.emit_string_expr(
                arg_expr(a).ok_or_else(|| diag(&format!("host_path: {name} arg")))?,
            )?;
            let ep = self.fresh();
            writeln!(
                self.body,
                "  {ep} = getelementptr inbounds [{n} x ptr], ptr {arr}, i64 0, i64 {i}"
            )
            .ok();
            writeln!(self.body, "  store ptr {s}, ptr {ep}").ok();
        }
        let base = self.fresh();
        writeln!(
            self.body,
            "  {base} = getelementptr inbounds [{n} x ptr], ptr {arr}, i64 0, i64 0"
        )
        .ok();
        writeln!(
            self.body,
            "  {}",
            abi.call_to(&r, &format!("i64 {n}, ptr {base}"))
        )
        .ok();
        Ok(r)
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
    fn path_normalize_emits() {
        let m = lower_src(
            r#"
            let a = pathNormalize("foo//bar");
            let b = pathNormalize("foo/./bar/../baz");
            "#,
        );
        assert!(is_host_path_module(&m));
        let ir = emit_host_path(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_path_normalize"));
        assert!(ir.contains("draconic_rt_print_str"));
    }

    #[test]
    fn path_join_emits() {
        let m = lower_src(
            r#"
            let a = pathJoin("foo", "bar");
            let b = pathJoin("/foo", "bar", "baz");
            let c = pathJoin();
            "#,
        );
        assert!(is_host_path_module(&m));
        let ir = emit_host_path(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_path_join"));
    }

    #[test]
    fn path_dirname_basename_extname_emits() {
        let m = lower_src(
            r#"
            let a = pathDirname("/foo/bar");
            let b = pathBasename("/foo/bar.txt");
            let c = pathExtname("a.md");
            "#,
        );
        assert!(is_host_path_module(&m));
        let ir = emit_host_path(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_path_dirname"));
        assert!(ir.contains("draconic_rt_host_path_basename"));
        assert!(ir.contains("draconic_rt_host_path_extname"));
    }

    #[test]
    fn path_is_absolute_emits_bool() {
        let m = lower_src(
            r#"
            let a = pathIsAbsolute("/foo");
            let b = pathIsAbsolute("foo");
            "#,
        );
        assert!(is_host_path_module(&m));
        let ir = emit_host_path(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_path_is_absolute"));
        assert!(ir.contains("draconic_rt_print_bool"));
    }

    #[test]
    fn path_resolve_emits() {
        let m = lower_src(
            r#"
            let a = pathResolve("/foo", "bar");
            let b = pathResolve("/foo", "/bar");
            let c = pathResolve();
            "#,
        );
        assert!(is_host_path_module(&m));
        let ir = emit_host_path(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_path_resolve"), "{ir}");
    }
}
