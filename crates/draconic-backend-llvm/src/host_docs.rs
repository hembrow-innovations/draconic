//! Native lowering for the docs SSG Program: host file I/O plus string scan
//! (concat, `.length`, index, `===`) and `if` / `while` so `website/generate.drac`
//! can render the locked markdown subset.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, CSTR_CONCAT, CSTR_EQ_N, CSTR_FROM_CODE_UNIT, CSTR_LEN, GC_INIT,
    HOST_FS_APPEND_TEXT, HOST_FS_READ_TEXT, HOST_FS_WRITE_TEXT, HOST_PROCESS_EXIT,
    HOST_STDERR_WRITE,
};

pub(crate) fn is_host_docs_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_docs(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_docs module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    String,
    Number,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    has_fs: bool,
    has_script: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        has_fs: false,
        has_script: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_fs || !ctx.has_script {
        return None;
    }
    Some(ModuleInfo { slots: ctx.slots })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        Stmt::Block { body, .. } => {
            ctx.has_script = true;
            for s in body {
                classify_stmt(s, ctx)?;
            }
            Some(())
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            ctx.has_script = true;
            classify_test(test, ctx)?;
            classify_stmt(consequent, ctx)?;
            if let Some(alt) = alternate {
                classify_stmt(alt, ctx)?;
            }
            Some(())
        }
        Stmt::While { test, body, .. } => {
            ctx.has_script = true;
            classify_test(test, ctx)?;
            classify_stmt(body, ctx)
        }
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 2
                && (is_named_callee(callee, "writeFileText")
                    || is_named_callee(callee, "appendFileText")) =>
        {
            ctx.has_fs = true;
            classify_string_expr(arg_expr(&args[0])?, ctx)?;
            classify_string_expr(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Assign { target, op, value, .. } => {
            if !matches!(op, AssignOp::Eq) {
                return None;
            }
            let AssignTarget::Local(id) = target else {
                return None;
            };
            let ty = classify_expr(value, ctx)?;
            let want = ctx.slot_of.get(id).copied()?;
            if ty != want {
                return None;
            }
            Some(())
        }
        _ => None,
    }
}

fn classify_test(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Binary { left, op, right, .. } => match op {
            BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => {
                let lt = classify_expr(left, ctx)?;
                let rt = classify_expr(right, ctx)?;
                if lt == SlotTy::Number && rt == SlotTy::Number {
                    Some(())
                } else {
                    None
                }
            }
            BinaryOp::EqEqEq | BinaryOp::NotEqEq | BinaryOp::EqEq | BinaryOp::NotEq => {
                let lt = classify_expr(left, ctx)?;
                let rt = classify_expr(right, ctx)?;
                if lt == rt {
                    Some(())
                } else {
                    None
                }
            }
            _ => None,
        },
        _ => None,
    }
}

fn classify_string_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    matches!(classify_expr(expr, ctx)?, SlotTy::String).then_some(())
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readFileText") =>
        {
            classify_string_expr(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            Some(SlotTy::String)
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add,
            right,
            ..
        } => {
            ctx.has_script = true;
            let lt = classify_expr(left, ctx)?;
            let rt = classify_expr(right, ctx)?;
            match (lt, rt) {
                (SlotTy::String, _) | (_, SlotTy::String) => Some(SlotTy::String),
                (SlotTy::Number, SlotTy::Number) => Some(SlotTy::Number),
            }
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            ctx.has_script = true;
            let obj = classify_expr(object, ctx)?;
            let prop = string_lit(property)?;
            if obj == SlotTy::String && prop == "length" {
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
            ctx.has_script = true;
            let obj = classify_expr(object, ctx)?;
            let idx = classify_expr(property, ctx)?;
            if obj == SlotTy::String && idx == SlotTy::Number {
                Some(SlotTy::String)
            } else {
                None
            }
        }
        Expr::Assign { target, op, value, .. } => {
            if !matches!(op, AssignOp::Eq) {
                return None;
            }
            let AssignTarget::Local(id) = target else {
                return None;
            };
            let ty = classify_expr(value, ctx)?;
            let want = ctx.slot_of.get(id).copied()?;
            if ty == want {
                Some(ty)
            } else {
                None
            }
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
    slot_of: HashMap<LocalId, SlotTy>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
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

    fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.next_tmp;
        self.next_tmp += 1;
        format!("{prefix}_{n}")
    }

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        Ok(format!("%slot_{}", id.0))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.docs.{}", self.str_globals.len());
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
        writeln!(self.out, "; Draconic LLVM host_docs (docs SSG markdown subset)").ok();
        let decls = [
            GC_INIT,
            HOST_PROCESS_EXIT,
            HOST_STDERR_WRITE,
            HOST_FS_READ_TEXT,
            HOST_FS_WRITE_TEXT,
            HOST_FS_APPEND_TEXT,
            CSTR_LEN,
            CSTR_CONCAT,
            CSTR_FROM_CODE_UNIT,
            CSTR_EQ_N,
        ];
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
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
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
        writeln!(self.body, "  unreachable").ok();
        Ok(())
    }

    fn emit_check_rc(&mut self, rc: &str) -> Result<(), Diagnostic> {
        let ok = self.fresh();
        let fail = self.fresh_label("fs_err");
        let cont = self.fresh_label("fs_ok");
        writeln!(self.body, "  {ok} = icmp eq i32 {rc}, 0").ok();
        writeln!(self.body, "  br i1 {ok}, label %{cont}, label %{fail}").ok();
        writeln!(self.body, "{fail}:").ok();
        let is_noent = self.fresh();
        let noent_l = self.fresh_label("fs_noent");
        let other_l = self.fresh_label("fs_other");
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

    fn body_ends_with_terminator(&self) -> bool {
        self.body
            .lines()
            .rev()
            .find(|l| !l.is_empty())
            .is_some_and(|l| {
                let t = l.trim();
                t.starts_with("br ") || t.starts_with("ret ") || t == "unreachable"
            })
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
                    .ok_or_else(|| diag("host_docs: declare unknown slot"))?;
                match ty {
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
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
            Stmt::Block { body, .. } => {
                for s in body {
                    if self.body_ends_with_terminator() {
                        break;
                    }
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => {
                let cond = self.emit_test(test)?;
                let then_l = self.fresh_label("then");
                let else_l = self.fresh_label("else");
                let end_l = self.fresh_label("endif");
                if alternate.is_some() {
                    writeln!(
                        self.body,
                        "  br i1 {cond}, label %{then_l}, label %{else_l}"
                    )
                    .ok();
                } else {
                    writeln!(
                        self.body,
                        "  br i1 {cond}, label %{then_l}, label %{end_l}"
                    )
                    .ok();
                }
                writeln!(self.body, "{then_l}:").ok();
                self.emit_stmt(consequent)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{end_l}").ok();
                }
                if let Some(alt) = alternate {
                    writeln!(self.body, "{else_l}:").ok();
                    self.emit_stmt(alt)?;
                    if !self.body_ends_with_terminator() {
                        writeln!(self.body, "  br label %{end_l}").ok();
                    }
                }
                writeln!(self.body, "{end_l}:").ok();
                Ok(())
            }
            Stmt::While { test, body, .. } => {
                let head = self.fresh_label("while_head");
                let bod = self.fresh_label("while_body");
                let end = self.fresh_label("while_end");
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{head}:").ok();
                let cond = self.emit_test(test)?;
                writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                writeln!(self.body, "{bod}:").ok();
                self.emit_stmt(body)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{head}").ok();
                }
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            _ => Err(diag("host_docs: unsupported statement")),
        }
    }

    fn emit_side_effect(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "writeFileText") =>
            {
                self.emit_write_text_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_docs: writeFileText path"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_docs: writeFileText text"))?,
                    HOST_FS_WRITE_TEXT.symbol,
                )
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "appendFileText") =>
            {
                self.emit_write_text_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_docs: appendFileText path"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_docs: appendFileText text"))?,
                    HOST_FS_APPEND_TEXT.symbol,
                )
            }
            Expr::Assign { .. } => {
                let _ = self.emit_assign(expr)?;
                Ok(())
            }
            _ => Err(diag("host_docs: unsupported expr statement")),
        }
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
        writeln!(
            self.body,
            "  {rc} = call i32 @{symbol}(ptr {p}, ptr {t})"
        )
        .ok();
        self.emit_check_rc(&rc)
    }

    fn emit_assign(&mut self, expr: &Expr) -> Result<SlotTy, Diagnostic> {
        let Expr::Assign { target, op, value, .. } = expr else {
            return Err(diag("host_docs: expected assign"));
        };
        if !matches!(op, AssignOp::Eq) {
            return Err(diag("host_docs: only simple ="));
        }
        let AssignTarget::Local(id) = target else {
            return Err(diag("host_docs: only local assign"));
        };
        let ty = self
            .slot_of
            .get(id)
            .copied()
            .ok_or_else(|| diag("host_docs: assign unknown slot"))?;
        let ptr = self.slot_ptr(*id)?;
        match ty {
            SlotTy::String => {
                let v = self.emit_string_expr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
            SlotTy::Number => {
                let v = self.emit_number_expr(value)?;
                writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
            }
        }
        Ok(ty)
    }

    fn expr_slot(&self, expr: &Expr) -> Option<SlotTy> {
        match expr {
            Expr::String { .. } => Some(SlotTy::String),
            Expr::Number { .. } => Some(SlotTy::Number),
            Expr::Local { id, .. } => self.slot_of.get(id).copied(),
            Expr::Call { callee, .. } if is_named_callee(callee, "readFileText") => {
                Some(SlotTy::String)
            }
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } => match (self.expr_slot(left), self.expr_slot(right)) {
                (Some(SlotTy::String), _) | (_, Some(SlotTy::String)) => Some(SlotTy::String),
                (Some(SlotTy::Number), Some(SlotTy::Number)) => Some(SlotTy::Number),
                _ => None,
            },
            Expr::Member {
                computed: false, ..
            } => Some(SlotTy::Number),
            Expr::Member {
                computed: true, ..
            } => Some(SlotTy::String),
            Expr::Assign { value, .. } => self.expr_slot(value),
            _ => None,
        }
    }

    fn emit_test(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let Expr::Binary { left, op, right, .. } = expr else {
            return Err(diag("host_docs: unsupported test"));
        };
        match op {
            BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let pred = match op {
                    BinaryOp::Lt => "olt",
                    BinaryOp::Gt => "ogt",
                    BinaryOp::LtEq => "ole",
                    BinaryOp::GtEq => "oge",
                    _ => unreachable!(),
                };
                let cmp = self.fresh();
                writeln!(self.body, "  {cmp} = fcmp {pred} double {l}, {r}").ok();
                Ok(cmp)
            }
            BinaryOp::EqEqEq | BinaryOp::NotEqEq | BinaryOp::EqEq | BinaryOp::NotEq => {
                let neg = matches!(op, BinaryOp::NotEqEq | BinaryOp::NotEq);
                match (self.expr_slot(left), self.expr_slot(right)) {
                    (Some(SlotTy::Number), Some(SlotTy::Number)) => {
                        let l = self.emit_number_expr(left)?;
                        let r = self.emit_number_expr(right)?;
                        let pred = if neg { "one" } else { "oeq" };
                        let cmp = self.fresh();
                        writeln!(self.body, "  {cmp} = fcmp {pred} double {l}, {r}").ok();
                        Ok(cmp)
                    }
                    (Some(SlotTy::String), Some(SlotTy::String)) => {
                        let l = self.emit_string_expr(left)?;
                        let r = self.emit_string_expr(right)?;
                        let la = self.fresh();
                        let lb = self.fresh();
                        let eq = self.fresh();
                        let cmp = self.fresh();
                        writeln!(
                            self.body,
                            "  {}",
                            CSTR_LEN.call_to(&la, &format!("ptr {l}"))
                        )
                        .ok();
                        writeln!(
                            self.body,
                            "  {}",
                            CSTR_LEN.call_to(&lb, &format!("ptr {r}"))
                        )
                        .ok();
                        writeln!(
                            self.body,
                            "  {}",
                            CSTR_EQ_N.call_to(&eq, &format!("ptr {l}, i64 {la}, ptr {r}, i64 {lb}"))
                        )
                        .ok();
                        let pred = if neg { "ne" } else { "eq" };
                        writeln!(self.body, "  {cmp} = icmp {pred} i32 {eq}, 1").ok();
                        Ok(cmp)
                    }
                    _ => Err(diag("host_docs: equality type mismatch")),
                }
            }
            _ => Err(diag("host_docs: unsupported compare")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
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
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = fadd double {l}, {r}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_docs: length prop"))?;
                if prop != "length" {
                    return Err(diag("host_docs: unsupported number member"));
                }
                let s = self.emit_string_expr(object)?;
                let n = self.fresh();
                let f = self.fresh();
                writeln!(self.body, "  {}", CSTR_LEN.call_to(&n, &format!("ptr {s}"))).ok();
                writeln!(self.body, "  {f} = sitofp i64 {n} to double").ok();
                Ok(f)
            }
            Expr::Assign { .. } => {
                self.emit_assign(expr)?;
                if let Expr::Assign { target: AssignTarget::Local(id), .. } = expr {
                    let ptr = self.slot_ptr(*id)?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    Ok(v)
                } else {
                    Err(diag("host_docs: number assign"))
                }
            }
            _ => Err(diag("host_docs: unsupported number expr")),
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
                    return Err(diag("host_docs: readFileText expects 1 arg"));
                }
                let path = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_docs: readFileText path"))?,
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
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } => {
                let l = self.emit_string_expr(left)?;
                let r = self.emit_string_expr(right)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    CSTR_CONCAT.call_to(&t, &format!("ptr {l}, ptr {r}"))
                )
                .ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                computed: true,
                ..
            } => {
                let s = self.emit_string_expr(object)?;
                let idx_f = self.emit_number_expr(property)?;
                let idx = self.fresh();
                writeln!(self.body, "  {idx} = fptoui double {idx_f} to i64").ok();
                let ch = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    CSTR_FROM_CODE_UNIT.call_to(&ch, &format!("ptr {s}, i64 {idx}"))
                )
                .ok();
                Ok(ch)
            }
            Expr::Assign { .. } => {
                self.emit_assign(expr)?;
                if let Expr::Assign { target: AssignTarget::Local(id), .. } = expr {
                    let ptr = self.slot_ptr(*id)?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    Ok(v)
                } else {
                    Err(diag("host_docs: string assign"))
                }
            }
            _ => Err(diag("host_docs: unsupported string expr")),
        }
    }
}
