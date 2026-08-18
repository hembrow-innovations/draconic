//! H02.01: native observations for `stdoutWrite`.
//!
//! - `stdoutWrite(string)` — UTF-8 bytes to OS stdout (no auto newline)
//! - `stdoutWrite(Uint8Array)` — raw bytes (simple `new Uint8Array(n)` + index assigns)
//!
//! Side-effect-only modules: program writes are the observed stdout.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::AssignOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{llvm_declares, GC_INIT, HOST_STDOUT_WRITE};

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
    /// `new Uint8Array(n)` backing store (raw bytes).
    Bytes(usize),
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx<'a> {
    module: &'a Module,
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    has_stdout: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        module,
        slots: Vec::new(),
        slot_of: HashMap::new(),
        has_stdout: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_stdout {
        return None;
    }
    Some(ModuleInfo { slots: ctx.slots })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite", ctx.module) =>
        {
            ctx.has_stdout = true;
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
            SlotTy::Bytes(_) => Some(()),
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
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
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
    body: String,
    out: String,
    next_tmp: u32,
    str_globals: HashMap<String, (String, usize)>,
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

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM host_stdio (H02.01 stdoutWrite)"
        )
        .ok();
        self.out
            .push_str(&llvm_declares(&[GC_INIT, HOST_STDOUT_WRITE]));
        writeln!(self.out).ok();

        // Collect string/byte globals while emitting body.
        let mut byte_payloads: HashMap<String, Vec<u8>> = HashMap::new();

        for (id, ty) in &self.slot_of.clone() {
            let ptr = self.slot_ptr(*id)?;
            match ty {
                SlotTy::Bytes(n) => {
                    if *n == 0 {
                        // empty: keep a 1-byte alloca for a stable pointer
                        writeln!(self.body, "  {ptr} = alloca [1 x i8], align 1").ok();
                    } else {
                        writeln!(self.body, "  {ptr} = alloca [{n} x i8], align 1").ok();
                        // zero-init
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
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt, &mut byte_payloads)?;
        }

        // Emit globals for string payloads.
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

        // memset intrinsic declare when any Bytes slot.
        let needs_memset = self.slot_of.values().any(|t| matches!(t, SlotTy::Bytes(n) if *n > 0));
        if needs_memset {
            writeln!(
                self.out,
                "declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg)"
            )
            .ok();
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

    fn emit_stmt(
        &mut self,
        stmt: &Stmt,
        payloads: &mut HashMap<String, Vec<u8>>,
    ) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                // Uint8Array alloc already zeroed in prolog; nothing more for New.
                match init {
                    Expr::New { callee, args, .. }
                        if args.len() == 1
                            && is_named_callee(callee, "Uint8Array", self.module) =>
                    {
                        let _ = local;
                        Ok(())
                    }
                    _ => Err(diag("host_stdio: unsupported declare")),
                }
            }
            Stmt::Expr { expr, .. } => self.emit_side_effect(expr, payloads),
            _ => Err(diag("host_stdio: unsupported statement")),
        }
    }

    fn emit_side_effect(
        &mut self,
        expr: &Expr,
        payloads: &mut HashMap<String, Vec<u8>>,
    ) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "stdoutWrite", self.module) =>
            {
                self.emit_stdout_write(arg_expr(&args[0]).ok_or_else(|| {
                    diag("host_stdio: stdoutWrite arg")
                })?, payloads)
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
                    .ok_or_else(|| diag("host_stdio: assign unknown bytes local"))?;
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

    fn emit_stdout_write(
        &mut self,
        arg: &Expr,
        _payloads: &mut HashMap<String, Vec<u8>>,
    ) -> Result<(), Diagnostic> {
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
                    "  {rc} = call i32 @{}(ptr {p}, i64 {n})",
                    HOST_STDOUT_WRITE.symbol
                )
                .ok();
                Ok(())
            }
            Expr::Local { id, .. } => {
                let SlotTy::Bytes(n) = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("host_stdio: stdoutWrite local not bytes"))?;
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
                    "  {rc} = call i32 @{}(ptr {p}, i64 {n})",
                    HOST_STDOUT_WRITE.symbol
                )
                .ok();
                Ok(())
            }
            _ => Err(diag("host_stdio: stdoutWrite expects string or Uint8Array")),
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
        let dir = std::env::temp_dir().join(format!(
            "draconic-hs-str-{}",
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
        let dir = std::env::temp_dir().join(format!(
            "draconic-hs-bytes-{}",
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
