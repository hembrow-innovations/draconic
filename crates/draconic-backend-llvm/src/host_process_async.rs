//! H15.03: `processWaitAsync(h)` → Promise of exit code via job queue.
//!
//! Supported subset:
//! - `processSpawn([...])` → handle (number)
//! - `processWaitAsync(h)` → Promise
//! - `processClose(h)`
//! - `p.then(function (code) { … })` (assign numbers, close)
//! - `typeof processWaitAsync` / `typeof processSpawn`
//! - end of main: `job_drain`, print number/string/bool observation locals

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, Local, LocalId, Module, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_PROCESS_ASYNC_DECLARES, HOST_PROCESS_CLOSE, HOST_PROCESS_SPAWN,
    HOST_PROCESS_WAIT_ASYNC, JOB_DRAIN, PRINT_BOOL, PRINT_I64, PRINT_STR, PROMISE_THEN,
};

pub(crate) fn is_host_process_async_module(module: &Module) -> bool {
    match try_classify(module) {
        Ok(info) => info.uses_async,
        Err(_) => false,
    }
}

pub(crate) fn emit_host_process_async(module: &Module) -> Result<String, Diagnostic> {
    let info = try_classify(module).map_err(diag)?;
    if !info.uses_async {
        return Err(diag("internal: not a host_process_async module"));
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
    Promise,
}

struct ModuleInfo {
    uses_async: bool,
    user_locals: Vec<(LocalId, SlotKind)>,
}

fn try_classify(module: &Module) -> Result<ModuleInfo, String> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_ids = HashSet::new();
    collect_top_level_decl_ids(&module.body, &mut user_ids);

    let mut slot_of: HashMap<LocalId, SlotKind> = HashMap::new();
    let mut user_locals = Vec::new();
    let mut seen = HashSet::new();
    let mut uses_async = false;

    for stmt in &module.body {
        check_stmt(stmt, &mut uses_async, &mut slot_of)?;
        if let Stmt::Declare { local, init, .. } = stmt {
            if !seen.insert(*local) {
                continue;
            }
            if !user_ids.contains(local) {
                continue;
            }
            let kind = if let Some(e) = init {
                kind_from_expr(e, &slot_of).unwrap_or(SlotKind::Number)
            } else {
                SlotKind::Number
            };
            let _ = by_id.get(local);
            slot_of.insert(*local, kind);
            user_locals.push((*local, kind));
        }
    }

    Ok(ModuleInfo {
        uses_async,
        user_locals,
    })
}

fn collect_top_level_decl_ids(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Declare { local, .. } => {
                out.insert(*local);
            }
            Stmt::Block { body } => collect_top_level_decl_ids(body, out),
            _ => {}
        }
    }
}

fn kind_from_expr(expr: &Expr, slot_of: &HashMap<LocalId, SlotKind>) -> Option<SlotKind> {
    match expr {
        Expr::Unary {
            op: UnaryOp::TypeOf,
            ..
        } => Some(SlotKind::String),
        Expr::Binary {
            op:
                BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq,
            ..
        } => Some(SlotKind::Bool),
        Expr::Call { callee, .. } if is_named_callee(callee, "processWaitAsync") => {
            Some(SlotKind::Promise)
        }
        Expr::Call { callee, .. }
            if is_named_callee(callee, "processSpawn")
                || is_named_callee(callee, "processClose") =>
        {
            Some(SlotKind::Number)
        }
        Expr::Number { .. } => Some(SlotKind::Number),
        Expr::Boolean { .. } => Some(SlotKind::Bool),
        Expr::String { .. } => Some(SlotKind::String),
        Expr::Local { id, .. } => slot_of.get(id).copied(),
        Expr::Member {
            property,
            computed: false,
            ..
        } => {
            let Expr::String { value, .. } = property.as_ref() else {
                return None;
            };
            let prop = value.to_string_lossy();
            if prop == "then" {
                return Some(SlotKind::Promise);
            }
            None
        }
        _ => None,
    }
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn check_stmt(
    stmt: &Stmt,
    uses: &mut bool,
    slot_of: &mut HashMap<LocalId, SlotKind>,
) -> Result<(), String> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            if let Some(e) = init {
                check_expr(e, uses, slot_of)?;
                if let Some(k) = kind_from_expr(e, slot_of) {
                    slot_of.insert(*local, k);
                }
            }
            Ok(())
        }
        Stmt::Expr { expr } => check_expr(expr, uses, slot_of),
        Stmt::Block { body } => {
            for s in body {
                check_stmt(s, uses, slot_of)?;
            }
            Ok(())
        }
        _ => Err("unsupported stmt in host_process_async".into()),
    }
}

fn check_expr(
    expr: &Expr,
    uses: &mut bool,
    slot_of: &mut HashMap<LocalId, SlotKind>,
) -> Result<(), String> {
    match expr {
        Expr::Call { callee, args, .. } => {
            if is_named_callee(callee, "processWaitAsync") {
                *uses = true;
            }
            if is_named_callee(callee, "processSpawn")
                || is_named_callee(callee, "processClose")
                || is_named_callee(callee, "processWaitAsync")
            {
                for a in args {
                    if let Arg::Expr(e) = a {
                        check_expr(e, uses, slot_of)?;
                    }
                }
                return Ok(());
            }
            if let Expr::Member {
                object,
                property,
                computed: false,
                ..
            } = callee.as_ref()
            {
                if let Expr::String { value, .. } = property.as_ref() {
                    if value.to_string_lossy() == "then" {
                        *uses = true;
                        check_expr(object, uses, slot_of)?;
                        for a in args {
                            if let Arg::Expr(e) = a {
                                check_expr(e, uses, slot_of)?;
                            }
                        }
                        return Ok(());
                    }
                }
            }
            Err("unsupported call in host_process_async".into())
        }
        Expr::Function { body, .. } => {
            for s in body {
                check_stmt(s, uses, slot_of)?;
            }
            Ok(())
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            value,
            ..
        } => {
            check_expr(value, uses, slot_of)?;
            if let Some(k) = kind_from_expr(value, slot_of) {
                slot_of.insert(*id, k);
            }
            Ok(())
        }
        Expr::Unary { arg, .. } => check_expr(arg, uses, slot_of),
        Expr::Binary { left, right, .. } => {
            check_expr(left, uses, slot_of)?;
            check_expr(right, uses, slot_of)
        }
        Expr::Local { .. }
        | Expr::Number { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::IdentName { .. }
        | Expr::Array { .. } => Ok(()),
        Expr::Member { object, .. } => check_expr(object, uses, slot_of),
        _ => Err("unsupported expr in host_process_async".into()),
    }
}

fn collect_assigned_locals(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Expr { expr } => collect_assigned_in_expr(expr, out),
            Stmt::Declare { local, init, .. } => {
                out.insert(*local);
                if let Some(e) = init {
                    collect_assigned_in_expr(e, out);
                }
            }
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
        Expr::Call { args, .. } => {
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
        Expr::Unary { arg, .. } => collect_assigned_in_expr(arg, out),
        _ => {}
    }
}

fn collect_used_locals(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Expr { expr } => collect_used_in_expr(expr, out),
            Stmt::Declare { init, .. } => {
                if let Some(e) = init {
                    collect_used_in_expr(e, out);
                }
            }
            Stmt::Block { body } => collect_used_locals(body, out),
            _ => {}
        }
    }
}

fn collect_used_in_expr(expr: &Expr, out: &mut HashSet<LocalId>) {
    match expr {
        Expr::Local { id, .. } => {
            out.insert(*id);
        }
        Expr::Assign { value, .. } => collect_used_in_expr(value, out),
        Expr::Call { callee, args, .. } => {
            collect_used_in_expr(callee, out);
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_used_in_expr(e, out);
                }
            }
        }
        Expr::Member { object, .. } => collect_used_in_expr(object, out),
        Expr::Binary { left, right, .. } => {
            collect_used_in_expr(left, out);
            collect_used_in_expr(right, out);
        }
        Expr::Unary { arg, .. } => collect_used_in_expr(arg, out),
        _ => {}
    }
}

struct Emitter<'a> {
    module: &'a Module,
    info: ModuleInfo,
    out: String,
    body: String,
    helpers: String,
    tmp: usize,
    fn_n: usize,
    str_globals: Vec<(String, String)>,
    allocas: HashMap<LocalId, String>,
    slot_kind: HashMap<LocalId, SlotKind>,
    reaction_params: HashMap<LocalId, String>,
    reaction_captures: Vec<LocalId>,
    local_names: HashMap<LocalId, String>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: ModuleInfo) -> Self {
        let mut slot_kind = HashMap::new();
        for (id, k) in &info.user_locals {
            slot_kind.insert(*id, *k);
        }
        let local_names: HashMap<LocalId, String> = module
            .locals
            .iter()
            .map(|l| (l.id, l.name.clone()))
            .collect();
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            helpers: String::new(),
            tmp: 0,
            fn_n: 0,
            str_globals: Vec::new(),
            allocas: HashMap::new(),
            slot_kind,
            reaction_params: HashMap::new(),
            reaction_captures: Vec::new(),
            local_names,
        }
    }

    fn fresh(&mut self) -> String {
        let t = format!("%t{}", self.tmp);
        self.tmp += 1;
        t
    }

    fn fresh_fn(&mut self, prefix: &str) -> String {
        let n = self.fn_n;
        self.fn_n += 1;
        format!("{prefix}{n}")
    }

    fn finish(self) -> String {
        self.out
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        for (id, kind) in &self.info.user_locals {
            let name = format!("%loc{}", id.0);
            self.allocas.insert(*id, name.clone());
            match kind {
                SlotKind::Number | SlotKind::Promise => {}
                _ => {}
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        writeln!(self.body, "  {}", JOB_DRAIN.call("")).ok();

        for (id, kind) in &self.info.user_locals.clone() {
            match kind {
                SlotKind::Number => {
                    let v = self.load_local(*id)?;
                    writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {v}"))).ok();
                }
                SlotKind::Bool => {
                    let v = self.load_local(*id)?;
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i1 {v}"))).ok();
                }
                SlotKind::String => {
                    let v = self.load_local(*id)?;
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotKind::Promise => {}
            }
        }

        self.out
            .push_str(&llvm_declares(HOST_PROCESS_ASYNC_DECLARES));
        writeln!(self.out).ok();

        for (s, gname) in &self.str_globals {
            let esc = escape_llvm_string(s);
            let n = s.len() + 1;
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
        for (id, kind) in &self.info.user_locals {
            let name = self.allocas.get(id).cloned().unwrap();
            match kind {
                SlotKind::Number => {
                    writeln!(self.out, "  {name} = alloca i64, align 8").ok();
                }
                SlotKind::Bool => {
                    writeln!(self.out, "  {name} = alloca i1, align 1").ok();
                }
                SlotKind::String | SlotKind::Promise => {
                    writeln!(self.out, "  {name} = alloca ptr, align 8").ok();
                }
            }
        }
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
            _ => Err(diag("unsupported stmt")),
        }
    }

    fn store_local(&mut self, id: LocalId, value: &str) -> Result<(), Diagnostic> {
        let kind = self.slot_kind.get(&id).copied().unwrap_or(SlotKind::Number);
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("missing alloca"))?;
        match kind {
            SlotKind::Number => {
                writeln!(self.body, "  store i64 {value}, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  store i1 {value}, ptr {ptr}").ok();
            }
            SlotKind::String | SlotKind::Promise => {
                writeln!(self.body, "  store ptr {value}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        if let Expr::Local { id, .. } = expr {
            if let Some(v) = self.reaction_params.get(id).cloned() {
                return Ok(v);
            }
            if let Some(pos) = self.reaction_captures.iter().position(|c| c == id) {
                return self.load_capture(pos);
            }
            return self.load_local(*id);
        }
        match expr {
            Expr::Number { raw, .. } => {
                let n: i64 = raw.parse().unwrap_or(0);
                Ok(format!("{n}"))
            }
            Expr::Boolean { value, .. } => Ok(if *value { "true" } else { "false" }.into()),
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let g = self.intern_str(&s);
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {t} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0",
                    n = s.len() + 1
                )
                .ok();
                Ok(t)
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => {
                let name = match arg.as_ref() {
                    Expr::IdentName { name, .. } => Some(name.as_str()),
                    Expr::Local { id, .. } => self.local_names.get(id).map(|s| s.as_str()),
                    _ => None,
                };
                if let Some(name) = name {
                    if matches!(name, "processWaitAsync" | "processSpawn" | "processClose") {
                        let g = self.intern_str("function");
                        let t = self.fresh();
                        writeln!(
                            self.body,
                            "  {t} = getelementptr inbounds [9 x i8], ptr @{g}, i64 0, i64 0"
                        )
                        .ok();
                        return Ok(t);
                    }
                }
                Err(diag("typeof only on host APIs in host_process_async"))
            }
            Expr::Unary {
                op: UnaryOp::Minus,
                arg,
                ..
            } => {
                let v = self.emit_expr(arg)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = sub i64 0, {v}").ok();
                Ok(t)
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                let l = self.emit_expr(left)?;
                let r = self.emit_expr(right)?;
                let t = self.fresh();
                let pred = match op {
                    BinaryOp::Gt => "sgt",
                    BinaryOp::GtEq => "sge",
                    BinaryOp::Lt => "slt",
                    BinaryOp::LtEq => "sle",
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "eq",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "ne",
                    _ => return Err(diag("unsupported binary")),
                };
                writeln!(self.body, "  {t} = icmp {pred} i64 {l}, {r}").ok();
                Ok(t)
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                value,
                ..
            } => {
                let v = self.emit_expr(value)?;
                if !self.reaction_captures.is_empty() {
                    let mut buf = std::mem::take(&mut self.body);
                    self.store_capture(&mut buf, *id, &v)?;
                    self.body = buf;
                } else {
                    self.store_local(*id, &v)?;
                }
                Ok(v)
            }
            Expr::Call { callee, args, .. } => self.emit_call(callee, args),
            _ => Err(diag(format!("unsupported expr: {expr:?}"))),
        }
    }

    fn load_local(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        let kind = self.slot_kind.get(&id).copied().unwrap_or(SlotKind::Number);
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("missing local"))?;
        let t = self.fresh();
        match kind {
            SlotKind::Number => {
                writeln!(self.body, "  {t} = load i64, ptr {ptr}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  {t} = load i1, ptr {ptr}").ok();
            }
            SlotKind::String | SlotKind::Promise => {
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
            }
        }
        Ok(t)
    }

    fn load_capture(&mut self, pos: usize) -> Result<String, Diagnostic> {
        let id = self.reaction_captures[pos];
        let kind = self.slot_kind.get(&id).copied().unwrap_or(SlotKind::Number);
        let slot = self.capture_slot_ssa(pos)?;
        let t = self.fresh();
        match kind {
            SlotKind::Number => {
                writeln!(self.body, "  {t} = load i64, ptr {slot}").ok();
            }
            SlotKind::Bool => {
                writeln!(self.body, "  {t} = load i1, ptr {slot}").ok();
            }
            SlotKind::String | SlotKind::Promise => {
                writeln!(self.body, "  {t} = load ptr, ptr {slot}").ok();
            }
        }
        Ok(t)
    }

    fn capture_slot_ssa(&mut self, pos: usize) -> Result<String, Diagnostic> {
        if self.reaction_captures.len() == 1 {
            Ok("%data".into())
        } else {
            let n = self.reaction_captures.len();
            let t = self.fresh();
            writeln!(
                self.body,
                "  {t} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
            )
            .ok();
            let p = self.fresh();
            writeln!(self.body, "  {p} = load ptr, ptr {t}").ok();
            Ok(p)
        }
    }

    fn store_capture(
        &mut self,
        buf: &mut String,
        id: LocalId,
        value: &str,
    ) -> Result<(), Diagnostic> {
        let Some(pos) = self.reaction_captures.iter().position(|c| *c == id) else {
            return Ok(());
        };
        let kind = self.slot_kind.get(&id).copied().unwrap_or(SlotKind::Number);
        let slot = if self.reaction_captures.len() == 1 {
            "%data".to_string()
        } else {
            let n = self.reaction_captures.len();
            let t = format!("%cs{}", self.tmp);
            self.tmp += 1;
            writeln!(
                buf,
                "  {t} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
            )
            .ok();
            let p = format!("%csp{}", self.tmp);
            self.tmp += 1;
            writeln!(buf, "  {p} = load ptr, ptr {t}").ok();
            p
        };
        match kind {
            SlotKind::Number => {
                if value == "%value" {
                    let n = format!("%n{}", self.tmp);
                    self.tmp += 1;
                    writeln!(buf, "  {n} = ptrtoint ptr {value} to i64").ok();
                    writeln!(buf, "  store i64 {n}, ptr {slot}").ok();
                } else {
                    writeln!(buf, "  store i64 {value}, ptr {slot}").ok();
                }
            }
            SlotKind::Bool => {
                writeln!(buf, "  store i1 {value}, ptr {slot}").ok();
            }
            SlotKind::String | SlotKind::Promise => {
                writeln!(buf, "  store ptr {value}, ptr {slot}").ok();
            }
        }
        Ok(())
    }

    fn intern_str(&mut self, s: &str) -> String {
        if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            return g.clone();
        }
        let g = format!(".str.{}", self.str_globals.len());
        self.str_globals.push((s.to_string(), g.clone()));
        g
    }

    fn emit_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        if let Expr::Member {
            object,
            property,
            computed: false,
            ..
        } = callee
        {
            if let Expr::String { value, .. } = property.as_ref() {
                let prop = value.to_string_lossy();
                if prop == "then" {
                    return self.emit_then(object, args);
                }
            }
        }

        if is_named_callee(callee, "processSpawn") {
            return self.emit_spawn(args);
        }
        if is_named_callee(callee, "processWaitAsync") {
            let h = match args.first() {
                Some(Arg::Expr(e)) => self.emit_expr(e)?,
                _ => return Err(diag("processWaitAsync handle")),
            };
            let h32 = self.fresh();
            writeln!(self.body, "  {h32} = trunc i64 {h} to i32").ok();
            let t = self.fresh();
            writeln!(
                self.body,
                "  {t} = {}",
                HOST_PROCESS_WAIT_ASYNC.call(&format!("i32 {h32}"))
            )
            .ok();
            return Ok(t);
        }
        if is_named_callee(callee, "processClose") {
            let h = match args.first() {
                Some(Arg::Expr(e)) => self.emit_expr(e)?,
                _ => return Err(diag("processClose handle")),
            };
            let h32 = self.fresh();
            writeln!(self.body, "  {h32} = trunc i64 {h} to i32").ok();
            let t = self.fresh();
            writeln!(
                self.body,
                "  {t} = {}",
                HOST_PROCESS_CLOSE.call(&format!("i32 {h32}"))
            )
            .ok();
            let z = self.fresh();
            writeln!(self.body, "  {z} = sext i32 {t} to i64").ok();
            return Ok(z);
        }
        Err(diag("unsupported call in host_process_async"))
    }

    fn emit_spawn(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        if args.is_empty() || args.len() > 1 {
            return Err(diag("processSpawn(argv) only in async subset"));
        }
        let argv_expr = match &args[0] {
            Arg::Expr(e) => e,
            _ => return Err(diag("processSpawn argv")),
        };
        let Expr::Array { elements, .. } = argv_expr else {
            return Err(diag("processSpawn argv must be array lit"));
        };
        let mut strs = Vec::new();
        for el in elements {
            let ArrayElement::Expr(e) = el else {
                return Err(diag("spread in processSpawn argv"));
            };
            let Expr::String { value, .. } = e else {
                return Err(diag("processSpawn argv string lit"));
            };
            strs.push(value.to_string_lossy());
        }
        let n = strs.len();
        let arr = self.fresh();
        writeln!(self.body, "  {arr} = alloca [{n} x ptr], align 8").ok();
        for (i, s) in strs.iter().enumerate() {
            let g = self.intern_str(s);
            let p = self.fresh();
            writeln!(
                self.body,
                "  {p} = getelementptr inbounds [{m} x i8], ptr @{g}, i64 0, i64 0",
                m = s.len() + 1
            )
            .ok();
            let slot = self.fresh();
            writeln!(
                self.body,
                "  {slot} = getelementptr inbounds [{n} x ptr], ptr {arr}, i64 0, i64 {i}"
            )
            .ok();
            writeln!(self.body, "  store ptr {p}, ptr {slot}").ok();
        }
        let base = self.fresh();
        writeln!(
            self.body,
            "  {base} = getelementptr inbounds [{n} x ptr], ptr {arr}, i64 0, i64 0"
        )
        .ok();
        let t = self.fresh();
        writeln!(
            self.body,
            "  {t} = {}",
            HOST_PROCESS_SPAWN.call(&format!(
                "i32 {n}, ptr {base}, ptr null, i32 0, ptr null, ptr null"
            ))
        )
        .ok();
        let z = self.fresh();
        writeln!(self.body, "  {z} = sext i32 {t} to i64").ok();
        Ok(z)
    }

    fn emit_then(&mut self, object: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let p = self.emit_expr(object)?;
        let mut on_ful = "null".to_string();
        let mut ful_data = "null".to_string();
        if let Some(Arg::Expr(f0)) = args.first() {
            let Expr::Function { params, body, .. } = f0 else {
                return Err(diag("then callback must be function expression"));
            };
            let (name, data) = self.emit_reaction_fn(params, body)?;
            on_ful = format!("@{name}");
            ful_data = data;
        }
        let t = self.fresh();
        writeln!(
            self.body,
            "  {}",
            PROMISE_THEN.call_to(
                &t,
                &format!("ptr {p}, ptr {on_ful}, ptr {ful_data}, ptr null, ptr null")
            )
        )
        .ok();
        Ok(t)
    }

    fn emit_reaction_fn(
        &mut self,
        params: &[Param],
        body: &[Stmt],
    ) -> Result<(String, String), Diagnostic> {
        let fn_name = self.fresh_fn("react");
        let mut param_id = None;
        if let Some(p) = params.first() {
            let Pattern::Local(id) = &p.pattern else {
                return Err(diag("bad param"));
            };
            param_id = Some(*id);
        }

        let mut assigned = HashSet::new();
        collect_assigned_locals(body, &mut assigned);
        collect_used_locals(body, &mut assigned);
        let mut captures: Vec<LocalId> = assigned
            .into_iter()
            .filter(|id| {
                matches!(
                    self.slot_kind.get(id),
                    Some(SlotKind::Number) | Some(SlotKind::Bool) | Some(SlotKind::String)
                )
            })
            .filter(|id| self.allocas.contains_key(id))
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
        let saved_react = std::mem::take(&mut self.reaction_params);
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.reaction_params.clear();
        if let Some(id) = param_id {
            self.reaction_params.insert(id, "%value".into());
        }
        self.reaction_captures = captures;

        let mut ret_val = "%value".to_string();
        for stmt in body {
            match stmt {
                Stmt::Expr { expr } => {
                    if let Expr::Assign {
                        target: AssignTarget::Local(id),
                        value,
                        ..
                    } = expr
                    {
                        let v = self.emit_expr(value)?;
                        let mut buf = std::mem::take(&mut self.body);
                        self.store_capture(&mut buf, *id, &v)?;
                        self.body = buf;
                        ret_val = v;
                    } else {
                        ret_val = self.emit_expr(expr)?;
                    }
                }
                Stmt::Block { body: inner } => {
                    for s in inner {
                        if let Stmt::Expr { expr } = s {
                            if let Expr::Assign {
                                target: AssignTarget::Local(id),
                                value,
                                ..
                            } = expr
                            {
                                let v = self.emit_expr(value)?;
                                let mut buf = std::mem::take(&mut self.body);
                                self.store_capture(&mut buf, *id, &v)?;
                                self.body = buf;
                                ret_val = v;
                            } else {
                                ret_val = self.emit_expr(expr)?;
                            }
                        }
                    }
                }
                _ => return Err(diag("unsupported stmt in reaction")),
            }
        }

        let ret_ptr = if ret_val == "%value" {
            ret_val.clone()
        } else {
            let t = format!("%ret{}", self.tmp);
            self.tmp += 1;
            writeln!(self.body, "  {t} = inttoptr i64 {ret_val} to ptr").ok();
            t
        };

        let mut helper = String::new();
        writeln!(helper, "define ptr @{fn_name}(ptr %data, ptr %value) {{").ok();
        writeln!(helper, "entry:").ok();
        helper.push_str(&self.body);
        writeln!(helper, "  ret ptr {ret_ptr}").ok();
        writeln!(helper, "}}").ok();
        self.helpers.push_str(&helper);

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.reaction_params = saved_react;
        self.reaction_captures = saved_caps;

        Ok((fn_name, data_operand))
    }
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

fn diag(msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    #[test]
    fn emit_process_wait_async_has_job_drain_and_abi() {
        let src = r#"
            let settled = 0;
            let code = -1;
            let h = processSpawn(["/bin/sh", "-c", "exit 7"]);
            processWaitAsync(h).then(function (c) {
              code = c;
              settled = 1;
              processClose(h);
            });
            let t = typeof processWaitAsync;
        "#;
        let m = compile_source(src).expect("compile");
        assert!(is_host_process_async_module(&m));
        let ir = emit_host_process_async(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_process_wait_async"), "{ir}");
        assert!(ir.contains("draconic_rt_job_drain"), "{ir}");
        assert!(ir.contains("draconic_rt_promise_then"), "{ir}");
        assert!(ir.contains("draconic_rt_host_process_spawn"), "{ir}");
    }
}
