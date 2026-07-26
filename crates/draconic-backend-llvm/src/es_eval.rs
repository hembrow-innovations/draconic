//! N07.02: lower direct `eval` basics via Embed (constant-string fold at emit).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::BinaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_embed::{eval_source, EmbedValue};
use draconic_ir::{
    Arg, Expr, IrType as Type, Local, LocalId, Module, Stmt,
};

/// True when this module is the supported direct-eval subset (E16.01 / N07.02).
pub(crate) fn is_es_eval_module(module: &Module) -> bool {
    match try_classify(module) {
        Ok(info) => info.uses_eval,
        Err(_) => false,
    }
}

pub(crate) fn emit_es_eval(module: &Module) -> Result<String, Diagnostic> {
    let info = try_classify(module).map_err(diag)?;
    if !info.uses_eval {
        return Err(diag("internal: not an eval module"));
    }
    let mut em = Emitter::new(module, info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotKind {
    Number,
    String,
    Bool,
}

struct ModuleInfo {
    uses_eval: bool,
    eval_id: Option<LocalId>,
    global_this_id: Option<LocalId>,
    /// Top-level user locals to allocate / print (source order).
    user_locals: Vec<(LocalId, SlotKind)>,
}

fn try_classify(module: &Module) -> Result<ModuleInfo, String> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let eval_id = module.locals.iter().find(|l| l.name == "eval").map(|l| l.id);
    let global_this_id = module
        .locals
        .iter()
        .find(|l| l.name == "globalThis")
        .map(|l| l.id);

    let mut user_ids = HashSet::new();
    collect_top_level_decl_ids(&module.body, &mut user_ids);

    let mut user_locals = Vec::new();
    let mut seen = HashSet::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, init, .. } = stmt {
            if !seen.insert(*local) {
                continue;
            }
            if !user_ids.contains(local) {
                continue;
            }
            let Some(loc) = by_id.get(local) else {
                continue;
            };
            let kind = match loc.ty {
                Type::Number => SlotKind::Number,
                Type::String => SlotKind::String,
                Type::Boolean => SlotKind::Bool,
                // `eval(...)` is typed `any`; infer observation kind from init shape.
                Type::Any => match init {
                    Some(e) => infer_slot_kind(e, eval_id)?,
                    None => return Err(format!("untyped local `{}` without init", loc.name)),
                },
                _ => return Err(format!("unsupported local type for `{}`", loc.name)),
            };
            user_locals.push((*local, kind));
        }
    }

    let mut uses_eval = false;
    for stmt in &module.body {
        check_stmt(stmt, eval_id, global_this_id, &mut uses_eval)?;
    }

    Ok(ModuleInfo {
        uses_eval,
        eval_id,
        global_this_id,
        user_locals,
    })
}

fn infer_slot_kind(expr: &Expr, eval_id: Option<LocalId>) -> Result<SlotKind, String> {
    match expr {
        Expr::Number { .. } => Ok(SlotKind::Number),
        Expr::String { .. } => Ok(SlotKind::String),
        Expr::Boolean { .. } => Ok(SlotKind::Bool),
        Expr::Unary {
            op: draconic_ast::UnaryOp::TypeOf,
            ..
        } => Ok(SlotKind::String),
        Expr::Binary { op, .. }
            if matches!(
                op,
                BinaryOp::EqEqEq | BinaryOp::NotEqEq | BinaryOp::EqEq | BinaryOp::NotEq
            ) =>
        {
            Ok(SlotKind::Bool)
        }
        Expr::Call { callee, args, .. } => {
            if let Expr::Local { id, .. } = callee.as_ref() {
                if Some(*id) == eval_id {
                    if let Some(Arg::Expr(Expr::String { value, .. })) = args.first() {
                        let src = value.to_string_lossy();
                        let v = eval_source(&src)
                            .map_err(|e| format!("embed eval failed for {src:?}: {e}"))?;
                        return match v {
                            EmbedValue::Number(_) => Ok(SlotKind::Number),
                            EmbedValue::String(_) | EmbedValue::Undefined => Ok(SlotKind::String),
                            EmbedValue::Boolean(_) => Ok(SlotKind::Bool),
                            EmbedValue::Null => Err("null eval result unsupported".into()),
                        };
                    }
                }
            }
            Err("cannot infer slot kind from call".into())
        }
        _ => Err(format!("cannot infer slot kind from {expr:?}")),
    }
}

fn collect_top_level_decl_ids(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Declare { local, .. } => {
                out.insert(*local);
            }
            Stmt::Expr { .. } => {}
            Stmt::Block { body } => collect_top_level_decl_ids(body, out),
            _ => {}
        }
    }
}

fn check_stmt(
    stmt: &Stmt,
    eval_id: Option<LocalId>,
    global_this_id: Option<LocalId>,
    uses: &mut bool,
) -> Result<(), String> {
    match stmt {
        Stmt::Declare { init, .. } => {
            if let Some(e) = init {
                check_expr(e, eval_id, global_this_id, uses)?;
            }
            Ok(())
        }
        Stmt::Expr { expr } => check_expr(expr, eval_id, global_this_id, uses),
        Stmt::Block { body } => {
            for s in body {
                check_stmt(s, eval_id, global_this_id, uses)?;
            }
            Ok(())
        }
        other => Err(format!("unsupported statement in eval path: {other:?}")),
    }
}

fn check_expr(
    expr: &Expr,
    eval_id: Option<LocalId>,
    global_this_id: Option<LocalId>,
    uses: &mut bool,
) -> Result<(), String> {
    match expr {
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } | Expr::Null { .. } => {
            Ok(())
        }
        Expr::Local { id, .. } => {
            if Some(*id) == eval_id {
                *uses = true;
            }
            Ok(())
        }
        Expr::Unary { arg, .. } => check_expr(arg, eval_id, global_this_id, uses),
        Expr::Binary {
            left, op, right, ..
        } => {
            if !matches!(
                op,
                BinaryOp::EqEqEq | BinaryOp::NotEqEq | BinaryOp::EqEq | BinaryOp::NotEq
            ) {
                return Err(format!("unsupported binary op in eval path: {op:?}"));
            }
            check_expr(left, eval_id, global_this_id, uses)?;
            check_expr(right, eval_id, global_this_id, uses)
        }
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } => {
            if *optional || *computed {
                return Err("optional/computed member not supported in eval path".into());
            }
            check_expr(object, eval_id, global_this_id, uses)?;
            check_expr(property, eval_id, global_this_id, uses)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return Err("optional call not supported in eval path".into());
            }
            if let Expr::Local { id, .. } = callee.as_ref() {
                if Some(*id) == eval_id {
                    *uses = true;
                    if args.len() != 1 {
                        return Err("eval expects 1 argument".into());
                    }
                    let Arg::Expr(arg) = &args[0] else {
                        return Err("spread not supported in eval".into());
                    };
                    let Expr::String { .. } = arg else {
                        return Err("only constant-string eval supported".into());
                    };
                    return Ok(());
                }
            }
            Err("only eval(...) calls supported in eval path".into())
        }
        other => Err(format!("unsupported expr in eval path: {other:?}")),
    }
}

struct Emitter<'a> {
    module: &'a Module,
    info: ModuleInfo,
    out: String,
    body: String,
    tmp: u32,
    allocas: HashMap<LocalId, String>,
    str_globals: HashMap<String, String>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: ModuleInfo) -> Self {
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            tmp: 0,
            allocas: HashMap::new(),
            str_globals: HashMap::new(),
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("%t{n}")
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N07.02 direct eval via Embed)"
        )
        .ok();
        writeln!(self.out, "declare void @draconic_rt_gc_init()").ok();
        writeln!(self.out, "declare void @draconic_rt_print_i64(i64)").ok();
        writeln!(self.out, "declare void @draconic_rt_print_bool(i8)").ok();
        writeln!(self.out, "declare void @draconic_rt_print_str(ptr)").ok();
        writeln!(self.out).ok();

        self.body.clear();
        self.tmp = 0;

        for (id, kind) in self.info.user_locals.clone() {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(id, ptr.clone());
            match kind {
                SlotKind::Number => {
                    writeln!(self.body, "  {ptr} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store i64 0, ptr {ptr}").ok();
                }
                SlotKind::Bool => {
                    writeln!(self.body, "  {ptr} = alloca i8, align 1").ok();
                    writeln!(self.body, "  store i8 0, ptr {ptr}").ok();
                }
                SlotKind::String => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, kind) in self.info.user_locals.clone() {
            let ptr = self.allocas.get(&id).cloned().unwrap();
            match kind {
                SlotKind::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
                    writeln!(self.body, "  call void @draconic_rt_print_i64(i64 {v})").ok();
                }
                SlotKind::Bool => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                    writeln!(self.body, "  call void @draconic_rt_print_bool(i8 {v})").ok();
                }
                SlotKind::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  call void @draconic_rt_print_str(ptr {v})").ok();
                }
            }
        }

        for (content, gname) in self.str_globals.clone() {
            let n = content.len() + 1;
            let esc = escape_llvm_string(&content);
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
        writeln!(self.out, "  call void @draconic_rt_gc_init()").ok();
        self.out.push_str(&self.body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                if let Some(e) = init {
                    let v = self.emit_expr(e)?;
                    self.store_local(*local, &v)?;
                }
                Ok(())
            }
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr)?;
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            other => Err(diag(format!(
                "unsupported statement in eval emit: {other:?}"
            ))),
        }
    }

    fn store_local(&mut self, id: LocalId, v: &str) -> Result<(), Diagnostic> {
        let Some(kind) = self
            .info
            .user_locals
            .iter()
            .find(|(l, _)| *l == id)
            .map(|(_, k)| *k)
        else {
            return Ok(());
        };
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag(format!("no alloca for local {}", id.0)))?;
        match kind {
            SlotKind::Number => {
                writeln!(self.body, "  store i64 {v}, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  store i8 {v}, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n = parse_i64_number(raw)?;
                Ok(format!("{n}"))
            }
            Expr::Boolean { value, .. } => Ok(if *value { "1".into() } else { "0".into() }),
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                if Some(*id) == self.info.eval_id {
                    // Opaque function token; only used for typeof / identity.
                    return Ok("null".into());
                }
                if Some(*id) == self.info.global_this_id {
                    return Ok("null".into());
                }
                if let Some(ptr) = self.allocas.get(id).cloned() {
                    let kind = self
                        .info
                        .user_locals
                        .iter()
                        .find(|(l, _)| *l == *id)
                        .map(|(_, k)| *k)
                        .ok_or_else(|| diag("load of non-user local"))?;
                    let v = self.fresh();
                    match kind {
                        SlotKind::Number => {
                            writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
                        }
                        SlotKind::Bool => {
                            writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                        }
                        SlotKind::String => {
                            writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                        }
                    }
                    return Ok(v);
                }
                Err(diag(format!("unbound local %{}", id.0)))
            }
            Expr::Unary {
                op: draconic_ast::UnaryOp::TypeOf,
                arg,
                ..
            } => self.emit_typeof(arg),
            Expr::Binary {
                left, op, right, ..
            } => self.emit_equality(left, *op, right),
            Expr::Member {
                object,
                property,
                computed,
                optional,
                ..
            } => {
                if *optional || *computed {
                    return Err(diag("optional/computed member not supported"));
                }
                // globalThis.eval → eval builtin token
                if let Expr::Local { id, .. } = object.as_ref() {
                    if Some(*id) == self.info.global_this_id {
                        if let Expr::String { value, .. } = property.as_ref() {
                            if value.to_string_lossy() == "eval" {
                                return Ok("null".into());
                            }
                        }
                    }
                }
                Err(diag("unsupported member in eval path"))
            }
            Expr::Call {
                callee, args, ..
            } => self.emit_eval_call(callee, args),
            other => Err(diag(format!(
                "unsupported expr in eval emit: {other:?}"
            ))),
        }
    }

    fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        if let Expr::Local { id, .. } = arg {
            if Some(*id) == self.info.eval_id {
                return self.string_const("function");
            }
        }
        // typeof of other supported values: not needed for direct_eval fixture.
        Err(diag("typeof only supported on eval in eval path"))
    }

    fn emit_equality(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
    ) -> Result<String, Diagnostic> {
        // globalThis.eval === eval (or ==) → true; !== / != → false
        let left_is_eval = is_eval_ref(left, self.info.eval_id, self.info.global_this_id);
        let right_is_eval = is_eval_ref(right, self.info.eval_id, self.info.global_this_id);
        if left_is_eval && right_is_eval {
            let eq = matches!(op, BinaryOp::EqEqEq | BinaryOp::EqEq);
            return Ok(if eq { "1".into() } else { "0".into() });
        }
        // Fall back: evaluate both (for completeness) then compare numbers/bools only.
        let _l = self.emit_expr(left)?;
        let _r = self.emit_expr(right)?;
        Err(diag(format!(
            "unsupported equality operands for op {op:?}"
        )))
    }

    fn emit_eval_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("eval callee must be local"));
        };
        if Some(*id) != self.info.eval_id {
            return Err(diag("only eval(...) supported"));
        }
        if args.len() != 1 {
            return Err(diag("eval expects 1 argument"));
        }
        let Arg::Expr(arg) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let Expr::String { value, .. } = arg else {
            return Err(diag("only constant-string eval supported"));
        };
        let src = value.to_string_lossy();
        let result = eval_source(&src).map_err(|e| {
            diag(format!(
                "embed eval failed for {src:?}: {e}"
            ))
        })?;
        match result {
            EmbedValue::Number(n) => {
                if n.fract() == 0.0 && n.is_finite() && n.abs() < (i64::MAX as f64) {
                    Ok(format!("{}", n as i64))
                } else {
                    Err(diag(format!(
                        "eval result number not representable as i64: {n}"
                    )))
                }
            }
            EmbedValue::String(s) => self.string_const(&s),
            EmbedValue::Boolean(b) => Ok(if b { "1".into() } else { "0".into() }),
            EmbedValue::Undefined => self.string_const("undefined"),
            EmbedValue::Null => Err(diag("null eval result not supported in observations")),
        }
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some(g) = self.str_globals.get(s) {
            g.clone()
        } else {
            let g = format!(".str.{}", self.str_globals.len());
            self.str_globals.insert(s.to_string(), g.clone());
            g
        };
        let t = self.fresh();
        let n = s.len() + 1;
        writeln!(
            self.body,
            "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
        )
        .ok();
        Ok(t)
    }
}

fn is_eval_ref(expr: &Expr, eval_id: Option<LocalId>, global_this_id: Option<LocalId>) -> bool {
    match expr {
        Expr::Local { id, .. } => Some(*id) == eval_id,
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } if !*computed && !*optional => {
            if let Expr::Local { id, .. } = object.as_ref() {
                if Some(*id) == global_this_id {
                    if let Expr::String { value, .. } = property.as_ref() {
                        return value.to_string_lossy() == "eval";
                    }
                }
            }
            false
        }
        _ => false,
    }
}

fn parse_i64_number(raw: &str) -> Result<i64, Diagnostic> {
    let s = raw.replace('_', "");
    s.parse::<f64>()
        .ok()
        .filter(|n| n.fract() == 0.0 && n.is_finite())
        .map(|n| n as i64)
        .ok_or_else(|| diag(format!("bad number literal: {raw}")))
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

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
