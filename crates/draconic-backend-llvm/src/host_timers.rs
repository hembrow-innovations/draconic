//! H05.03: `setTimeout` / `clearTimeout` via Runtime timer + job queue ABI.
//!
//! Supported subset for conformance:
//! - top-level number/bool/string locals
//! - `setTimeout(function () { … }, delay)` with number assigns in body
//! - nested `setTimeout` inside timer callbacks
//! - `clearTimeout(id)`
//! - `typeof setTimeout` / `typeof clearTimeout`
//! - comparison `id > 0`
//!
//! End of main: `job_drain` (promotes due timers, runs callbacks), then print
//! observation locals (numbers, strings, bools) in declaration order.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_TIMER_DECLARES, JOB_DRAIN, PRINT_BOOL, PRINT_I64, PRINT_STR,
    TIMER_CLEAR, TIMER_SET,
};

pub(crate) fn is_host_timer_module(module: &Module) -> bool {
    match try_classify(module) {
        Ok(info) => info.uses_timer,
        Err(_) => false,
    }
}

pub(crate) fn emit_host_timers(module: &Module) -> Result<String, Diagnostic> {
    let info = try_classify(module).map_err(diag)?;
    if !info.uses_timer {
        return Err(diag("internal: not a host_timer module"));
    }
    let mut em = Emitter::new(module, info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotKind {
    Number,
    Bool,
    String,
}

struct ModuleInfo {
    uses_timer: bool,
    user_locals: Vec<(LocalId, SlotKind)>,
}

fn try_classify(module: &Module) -> Result<ModuleInfo, String> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
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
            let kind = if let Some(e) = init {
                if let Some(k) = kind_from_init(e) {
                    k
                } else {
                    match by_id.get(local).map(|l| &l.ty) {
                        Some(Type::Boolean) => SlotKind::Bool,
                        Some(Type::String) => SlotKind::String,
                        _ => SlotKind::Number,
                    }
                }
            } else {
                match by_id.get(local).map(|l| &l.ty) {
                    Some(Type::Boolean) => SlotKind::Bool,
                    Some(Type::String) => SlotKind::String,
                    _ => SlotKind::Number,
                }
            };
            user_locals.push((*local, kind));
        }
    }

    let mut uses_timer = false;
    for stmt in &module.body {
        check_stmt(stmt, &mut uses_timer)?;
    }

    Ok(ModuleInfo {
        uses_timer,
        user_locals,
    })
}

fn kind_from_init(expr: &Expr) -> Option<SlotKind> {
    match expr {
        Expr::Unary {
            op: UnaryOp::TypeOf,
            ..
        } => Some(SlotKind::String),
        Expr::Binary {
            op: BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq,
            ..
        } => Some(SlotKind::Bool),
        Expr::Call { callee, .. } if is_named_callee(callee, "setTimeout") => Some(SlotKind::Number),
        Expr::Number { .. } => Some(SlotKind::Number),
        Expr::Boolean { .. } => Some(SlotKind::Bool),
        Expr::String { .. } => Some(SlotKind::String),
        _ => None,
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

fn check_stmt(stmt: &Stmt, uses: &mut bool) -> Result<(), String> {
    match stmt {
        Stmt::Declare { init, .. } => {
            if let Some(e) = init {
                check_expr(e, uses)?;
            }
            Ok(())
        }
        Stmt::Expr { expr } => check_expr(expr, uses),
        Stmt::Block { body } => {
            for s in body {
                check_stmt(s, uses)?;
            }
            Ok(())
        }
        _ => Err("unsupported stmt in host_timer module".into()),
    }
}

fn check_expr(expr: &Expr, uses: &mut bool) -> Result<(), String> {
    match expr {
        Expr::Call { callee, args, .. } if is_named_callee(callee, "setTimeout") => {
            *uses = true;
            if args.len() != 2 {
                return Err("setTimeout expects (fn, delay)".into());
            }
            let fn_expr = arg_expr(&args[0])?;
            match fn_expr {
                Expr::Function {
                    params,
                    body,
                    is_async,
                    is_generator,
                    ..
                } => {
                    if *is_async || *is_generator {
                        return Err("async/generator timer callback unsupported".into());
                    }
                    if !params.is_empty() {
                        return Err("timer callback params unsupported".into());
                    }
                    for s in body {
                        check_timer_body_stmt(s, uses)?;
                    }
                }
                _ => return Err("setTimeout callback must be function expression".into()),
            }
            check_expr(arg_expr(&args[1])?, uses)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "clearTimeout") => {
            *uses = true;
            if args.len() != 1 {
                return Err("clearTimeout expects (id)".into());
            }
            check_expr(arg_expr(&args[0])?, uses)
        }
        Expr::Call { .. } => Err("unsupported call in host_timer".into()),
        Expr::Assign { target, value, .. } => {
            match target {
                AssignTarget::Local(_) => {}
                _ => return Err("only local assign targets in host_timer".into()),
            }
            check_expr(value, uses)
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            if is_named_callee(arg, "setTimeout") || is_named_callee(arg, "clearTimeout") {
                *uses = true;
                Ok(())
            } else {
                check_expr(arg, uses)
            }
        }
        Expr::Binary { left, right, .. } => {
            check_expr(left, uses)?;
            check_expr(right, uses)
        }
        Expr::Local { .. }
        | Expr::Number { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. } => Ok(()),
        Expr::IdentName { name, .. } if name == "setTimeout" || name == "clearTimeout" => {
            *uses = true;
            Ok(())
        }
        Expr::Function { .. } => Err("bare function expr unsupported".into()),
        _ => Err("unsupported expr in host_timer".into()),
    }
}

fn check_timer_body_stmt(stmt: &Stmt, uses: &mut bool) -> Result<(), String> {
    match stmt {
        Stmt::Expr { expr } => check_expr(expr, uses),
        Stmt::Block { body } => {
            for s in body {
                check_timer_body_stmt(s, uses)?;
            }
            Ok(())
        }
        Stmt::Return { value } => {
            if let Some(e) = value {
                check_expr(e, uses)?;
            }
            Ok(())
        }
        _ => Err("unsupported stmt in timer callback".into()),
    }
}

fn arg_expr(arg: &Arg) -> Result<&Expr, String> {
    match arg {
        Arg::Expr(e) => Ok(e),
        Arg::Spread(_) => Err("spread args unsupported".into()),
    }
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn diag(msg: impl Into<String>) -> Diagnostic {
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
    info: ModuleInfo,
    out: String,
    body: String,
    helpers: String,
    tmp: u32,
    next_fn: u32,
    allocas: HashMap<LocalId, String>,
    str_globals: Vec<(String, String)>,
    /// Locals assigned inside the callback currently being emitted.
    reaction_captures: Vec<LocalId>,
    in_callback: bool,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: ModuleInfo) -> Self {
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            helpers: String::new(),
            tmp: 0,
            next_fn: 0,
            allocas: HashMap::new(),
            str_globals: Vec::new(),
            reaction_captures: Vec::new(),
            in_callback: false,
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

    fn fresh_fn(&mut self, prefix: &str) -> String {
        let n = self.next_fn;
        self.next_fn += 1;
        format!("d_{prefix}_{n}")
    }

    fn slot_kind(&self, id: LocalId) -> Option<SlotKind> {
        self.info
            .user_locals
            .iter()
            .find(|(l, _)| *l == id)
            .map(|(_, k)| *k)
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.{}", self.str_globals.len());
        self.str_globals.push((s.to_string(), g.clone()));
        g
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM host_timers (H05.03 setTimeout/clearTimeout)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(HOST_TIMER_DECLARES)).ok();
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

        writeln!(self.body, "  {}", JOB_DRAIN.call("")).ok();

        for (id, kind) in self.info.user_locals.clone() {
            let ptr = self.allocas.get(&id).cloned().unwrap();
            match kind {
                SlotKind::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {v}"))).ok();
                }
                SlotKind::Bool => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
                }
                SlotKind::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
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

        self.out.push_str(&self.helpers);
        if !self.helpers.is_empty() {
            writeln!(self.out).ok();
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        writeln!(self.out, "  {}", GC_INIT.call("")).ok();
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
            Stmt::Return { value } => {
                if let Some(e) = value {
                    let _ = self.emit_expr(e)?;
                }
                Ok(())
            }
            _ => Err(diag("host_timer: unsupported stmt")),
        }
    }

    fn store_local(&mut self, id: LocalId, val: &str) -> Result<(), Diagnostic> {
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("host_timer: unknown local"))?;
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
            SlotKind::Number => {
                writeln!(self.body, "  store i64 {val}, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  store i8 {val}, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  store ptr {val}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    fn load_local(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("host_timer: unknown local"))?;
        let v = self.fresh();
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
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
        Ok(v)
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n: i64 = raw.parse::<f64>().unwrap_or(0.0) as i64;
                Ok(format!("{n}"))
            }
            Expr::Boolean { value, .. } => Ok(if *value { "1".into() } else { "0".into() }),
            Expr::Local { id, .. } => {
                if self.in_callback {
                    if let Some(pos) = self.reaction_captures.iter().position(|c| c == id) {
                        return self.load_capture(pos);
                    }
                }
                self.load_local(*id)
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => self.emit_typeof(arg),
            Expr::Binary {
                op,
                left,
                right,
                ..
            } => self.emit_binary(*op, left, right),
            Expr::Call { callee, args, .. } if is_named_callee(callee, "setTimeout") => {
                if self.in_callback {
                    self.emit_set_timeout_in_callback(args)
                } else {
                    self.emit_set_timeout(args)
                }
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "clearTimeout") => {
                self.emit_clear_timeout(args)
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                value,
                ..
            } => {
                let v = self.emit_expr(value)?;
                if self.in_callback {
                    self.store_capture(*id, &v)?;
                } else {
                    self.store_local(*id, &v)?;
                }
                Ok(v)
            }
            Expr::IdentName { name, .. } if name == "setTimeout" || name == "clearTimeout" => {
                // Only valid under typeof — return dummy.
                Ok("null".into())
            }
            _ => Err(diag("host_timer: unsupported expr")),
        }
    }

    fn load_capture(&mut self, pos: usize) -> Result<String, Diagnostic> {
        let n = self.reaction_captures.len();
        let slot = self.fresh();
        let ptr = self.fresh();
        let v = self.fresh();
        if n == 1 {
            // data is the alloca ptr directly
            let id = self.reaction_captures[0];
            match self.slot_kind(id).unwrap_or(SlotKind::Number) {
                SlotKind::Number => {
                    writeln!(self.body, "  {v} = load i64, ptr %data").ok();
                }
                SlotKind::Bool => {
                    writeln!(self.body, "  {v} = load i8, ptr %data").ok();
                }
                SlotKind::String => {
                    writeln!(self.body, "  {v} = load ptr, ptr %data").ok();
                }
            }
            return Ok(v);
        }
        writeln!(
            self.body,
            "  {slot} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
        )
        .ok();
        writeln!(self.body, "  {ptr} = load ptr, ptr {slot}").ok();
        let id = self.reaction_captures[pos];
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
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
        Ok(v)
    }

    fn store_capture(&mut self, id: LocalId, val: &str) -> Result<(), Diagnostic> {
        let pos = self
            .reaction_captures
            .iter()
            .position(|c| *c == id)
            .ok_or_else(|| diag("host_timer: capture missing"))?;
        let n = self.reaction_captures.len();
        if n == 1 {
            match self.slot_kind(id).unwrap_or(SlotKind::Number) {
                SlotKind::Number => {
                    writeln!(self.body, "  store i64 {val}, ptr %data").ok();
                }
                SlotKind::Bool => {
                    writeln!(self.body, "  store i8 {val}, ptr %data").ok();
                }
                SlotKind::String => {
                    writeln!(self.body, "  store ptr {val}, ptr %data").ok();
                }
            }
            return Ok(());
        }
        let slot = self.fresh();
        let ptr = self.fresh();
        writeln!(
            self.body,
            "  {slot} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
        )
        .ok();
        writeln!(self.body, "  {ptr} = load ptr, ptr {slot}").ok();
        match self.slot_kind(id).unwrap_or(SlotKind::Number) {
            SlotKind::Number => {
                writeln!(self.body, "  store i64 {val}, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  store i8 {val}, ptr {ptr}").ok();
            }
            SlotKind::String => {
                writeln!(self.body, "  store ptr {val}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        let s = if is_named_callee(arg, "setTimeout") || is_named_callee(arg, "clearTimeout") {
            "function"
        } else {
            return Err(diag("host_timer: typeof only on setTimeout/clearTimeout"));
        };
        let g = self.intern_cstr(s);
        let n = s.len() + 1;
        let p = self.fresh();
        writeln!(
            self.body,
            "  {p} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0"
        )
        .ok();
        Ok(p)
    }

    fn emit_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<String, Diagnostic> {
        let l = self.emit_expr(left)?;
        let r = self.emit_expr(right)?;
        let v = self.fresh();
        match op {
            BinaryOp::Gt => {
                writeln!(self.body, "  {v} = icmp sgt i64 {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {v} to i8").ok();
                Ok(b)
            }
            BinaryOp::GtEq => {
                writeln!(self.body, "  {v} = icmp sge i64 {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {v} to i8").ok();
                Ok(b)
            }
            BinaryOp::Lt => {
                writeln!(self.body, "  {v} = icmp slt i64 {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {v} to i8").ok();
                Ok(b)
            }
            BinaryOp::LtEq => {
                writeln!(self.body, "  {v} = icmp sle i64 {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {v} to i8").ok();
                Ok(b)
            }
            BinaryOp::Add => {
                writeln!(self.body, "  {v} = add i64 {l}, {r}").ok();
                Ok(v)
            }
            BinaryOp::Sub => {
                writeln!(self.body, "  {v} = sub i64 {l}, {r}").ok();
                Ok(v)
            }
            _ => Err(diag("host_timer: unsupported binary")),
        }
    }

    fn emit_set_timeout(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let fn_expr = match &args[0] {
            Arg::Expr(e) => e,
            _ => return Err(diag("setTimeout bad arg")),
        };
        let delay_expr = match &args[1] {
            Arg::Expr(e) => e,
            _ => return Err(diag("setTimeout bad delay")),
        };
        let Expr::Function {
            params,
            body,
            ..
        } = fn_expr
        else {
            return Err(diag("setTimeout needs function"));
        };
        let _ = params;
        let (fn_name, data_op) = self.emit_timer_callback(body)?;
        let delay_i = self.emit_expr(delay_expr)?;
        let delay_d = self.fresh();
        writeln!(self.body, "  {delay_d} = sitofp i64 {delay_i} to double").ok();
        let id = self.fresh();
        writeln!(
            self.body,
            "  {}",
            TIMER_SET.call_to(&id, &format!("ptr @{fn_name}, ptr {data_op}, double {delay_d}"))
        )
        .ok();
        Ok(id)
    }

    fn emit_clear_timeout(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let id_expr = match &args[0] {
            Arg::Expr(e) => e,
            _ => return Err(diag("clearTimeout bad arg")),
        };
        let id = self.emit_expr(id_expr)?;
        writeln!(self.body, "  {}", TIMER_CLEAR.call(&format!("i64 {id}"))).ok();
        Ok("0".into())
    }

    fn emit_timer_callback(&mut self, body: &[Stmt]) -> Result<(String, String), Diagnostic> {
        let fn_name = self.fresh_fn("timer");
        let mut assigned = HashSet::new();
        collect_assigned_locals(body, &mut assigned);
        let mut captures: Vec<LocalId> = assigned
            .into_iter()
            .filter(|id| {
                matches!(
                    self.slot_kind(*id),
                    Some(SlotKind::Number) | Some(SlotKind::Bool) | Some(SlotKind::String)
                )
            })
            .collect();
        captures.sort_by_key(|id| id.0);

        let data_operand = if captures.is_empty() {
            "null".to_string()
        } else if captures.len() == 1 {
            self.allocas
                .get(&captures[0])
                .cloned()
                .ok_or_else(|| diag("capture missing alloca"))?
        } else {
            let n = captures.len();
            let env = self.fresh();
            writeln!(self.body, "  {env} = alloca [{n} x ptr], align 8").ok();
            for (i, id) in captures.iter().enumerate() {
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("capture missing alloca"))?;
                let slot = self.fresh();
                writeln!(
                    self.body,
                    "  {slot} = getelementptr inbounds [{n} x ptr], ptr {env}, i64 0, i64 {i}"
                )
                .ok();
                writeln!(self.body, "  store ptr {ptr}, ptr {slot}").ok();
            }
            env
        };

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_caps = std::mem::take(&mut self.reaction_captures);
        let saved_in = self.in_callback;

        self.tmp = 0;
        self.body.clear();
        self.reaction_captures = captures;
        self.in_callback = true;

        for stmt in body {
            self.emit_callback_stmt(stmt)?;
        }

        let mut fn_ir = String::new();
        writeln!(fn_ir, "define void @{fn_name}(ptr %data) {{").ok();
        writeln!(fn_ir, "entry:").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret void").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.reaction_captures = saved_caps;
        self.in_callback = saved_in;
        Ok((fn_name, data_operand))
    }

    fn emit_callback_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr)?;
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_callback_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Return { value } => {
                if let Some(e) = value {
                    let _ = self.emit_expr(e)?;
                }
                Ok(())
            }
            _ => Err(diag("host_timer cb: unsupported stmt")),
        }
    }

    fn emit_set_timeout_in_callback(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        // Nested timer: build another helper; captures are top-level allocas.
        // We need allocas available — they live in main. Pass main's capture env
        // by rebuilding from reaction_captures paths (pointers to main stack —
        // **invalid** after main returns, but job_drain runs before main returns).
        //
        // For nested setTimeout inside a timer job, main is still on stack
        // (drain called from main). Nested callback captures use the same
        // alloca pointers stored in the outer env.
        //
        // Simpler approach for fixture: nested only assigns `nested = 1` with
        // no outer local reads except the assign target. emit_timer_callback
        // already handles that when called from main body. From callback we
        // must call emit_timer_callback but it uses self.body which is the
        // helper body — and allocas map still points to main %lN names which
        // are NOT in scope in the helper!
        //
        // Fix: for nested timers, pass the same %data pointer if captures are
        // a subset of current captures, or build env from current %data.
        //
        // Fixture nested only assigns `nested` — capture is one alloca.
        // When outer callback runs, %data is that alloca (or env). Nested
        // callback also assigns nested — same capture.
        //
        // Implementation: emit nested helper that uses same capture layout as
        // if scheduled from main, but data_operand = current %data when the
        // nested capture set equals current reaction_captures, else error.

        let fn_expr = match &args[0] {
            Arg::Expr(e) => e,
            _ => return Err(diag("nested setTimeout bad arg")),
        };
        let delay_expr = match &args[1] {
            Arg::Expr(e) => e,
            _ => return Err(diag("nested setTimeout bad delay")),
        };
        let Expr::Function { body, .. } = fn_expr else {
            return Err(diag("nested setTimeout needs function"));
        };

        let mut assigned = HashSet::new();
        collect_assigned_locals(body, &mut assigned);
        let mut captures: Vec<LocalId> = assigned
            .into_iter()
            .filter(|id| {
                matches!(
                    self.slot_kind(*id),
                    Some(SlotKind::Number) | Some(SlotKind::Bool) | Some(SlotKind::String)
                )
            })
            .collect();
        captures.sort_by_key(|id| id.0);

        // Require nested captures ⊆ outer captures; pass through %data when equal
        // single-capture or rebuild — for fixture, both assign only `nested`.
        if captures != self.reaction_captures && !captures.is_empty() {
            // If nested is subset of one-element outer, still ok when equal.
            for c in &captures {
                if !self.reaction_captures.contains(c) {
                    return Err(diag("nested timer capture not in outer env"));
                }
            }
        }

        let data_op = if captures.is_empty() {
            "null".to_string()
        } else if captures == self.reaction_captures {
            "%data".to_string()
        } else if self.reaction_captures.len() == 1 && captures.len() == 1 {
            "%data".to_string()
        } else {
            // Build env of pointers loaded from outer env.
            let n = captures.len();
            let env = self.fresh();
            writeln!(self.body, "  {env} = alloca [{n} x ptr], align 8").ok();
            for (i, id) in captures.iter().enumerate() {
                let outer_pos = self
                    .reaction_captures
                    .iter()
                    .position(|c| c == id)
                    .ok_or_else(|| diag("nested capture missing"))?;
                let ptr = if self.reaction_captures.len() == 1 {
                    "%data".to_string()
                } else {
                    let on = self.reaction_captures.len();
                    let slot = self.fresh();
                    let p = self.fresh();
                    writeln!(
                        self.body,
                        "  {slot} = getelementptr inbounds [{on} x ptr], ptr %data, i64 0, i64 {outer_pos}"
                    )
                    .ok();
                    writeln!(self.body, "  {p} = load ptr, ptr {slot}").ok();
                    p
                };
                let slot = self.fresh();
                writeln!(
                    self.body,
                    "  {slot} = getelementptr inbounds [{n} x ptr], ptr {env}, i64 0, i64 {i}"
                )
                .ok();
                writeln!(self.body, "  store ptr {ptr}, ptr {slot}").ok();
            }
            env
        };

        let fn_name = self.fresh_fn("timer");
        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.reaction_captures = captures;
        self.in_callback = true;

        for stmt in body {
            self.emit_callback_stmt(stmt)?;
        }

        let mut fn_ir = String::new();
        writeln!(fn_ir, "define void @{fn_name}(ptr %data) {{").ok();
        writeln!(fn_ir, "entry:").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret void").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.reaction_captures = saved_caps;

        let delay_i = self.emit_expr(delay_expr)?;
        let delay_d = self.fresh();
        writeln!(self.body, "  {delay_d} = sitofp i64 {delay_i} to double").ok();
        let id = self.fresh();
        writeln!(
            self.body,
            "  {}",
            TIMER_SET.call_to(&id, &format!("ptr @{fn_name}, ptr {data_op}, double {delay_d}"))
        )
        .ok();
        Ok(id)
    }
}

fn collect_assigned_locals(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Expr { expr } => collect_assigned_in_expr(expr, out),
            Stmt::Block { body } => collect_assigned_locals(body, out),
            _ => {}
        }
    }
}

fn collect_assigned_in_expr(expr: &Expr, out: &mut HashSet<LocalId>) {
    match expr {
        Expr::Assign {
            target: AssignTarget::Local(id),
            value,
            ..
        } => {
            out.insert(*id);
            collect_assigned_in_expr(value, out);
        }
        Expr::Call { callee, args, .. } => {
            if is_named_callee(callee, "setTimeout") {
                if let Some(Arg::Expr(Expr::Function { body, .. })) = args.first() {
                    collect_assigned_locals(body, out);
                }
            }
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_assigned_in_expr(e, out);
                }
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_assigned_in_expr(left, out);
            collect_assigned_in_expr(right, out);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn ir_of(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn classifies_set_timeout_fixture() {
        let m = ir_of(
            r#"
            let fired = 0;
            setTimeout(function () { fired = 1; }, 0);
            let t = typeof setTimeout;
            "#,
        );
        assert!(is_host_timer_module(&m));
        let ir = emit_host_timers(&m).expect("emit");
        assert!(ir.contains("draconic_rt_timer_set"), "{ir}");
        assert!(ir.contains("draconic_rt_job_drain"), "{ir}");
    }

    #[test]
    fn classifies_clear_timeout() {
        let m = ir_of(
            r#"
            let cancelled = 0;
            let id = setTimeout(function () { cancelled = 1; }, 0);
            clearTimeout(id);
            "#,
        );
        assert!(is_host_timer_module(&m));
        let ir = emit_host_timers(&m).expect("emit");
        assert!(ir.contains("draconic_rt_timer_clear"), "{ir}");
    }
}
