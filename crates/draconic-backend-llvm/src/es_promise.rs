//! N06.03–N06.11: lower Promise + async/await (incl. async arrows) to Runtime ABI.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Param, Pattern,
    Stmt,
};

/// True when this module is the supported Promise/async subset (E12.01–E12.09 / N06.03–N06.11).
pub(crate) fn is_es_promise_module(module: &Module) -> bool {
    match try_classify(module) {
        Ok(info) => info.uses_promise,
        Err(_) => false,
    }
}

pub(crate) fn emit_es_promise(module: &Module) -> Result<String, Diagnostic> {
    let info = try_classify(module).map_err(diag)?;
    if !info.uses_promise {
        return Err(diag("internal: not a Promise module"));
    }
    let mut em = Emitter::new(module, info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotKind {
    Number,
    String,
    Object,
}

struct ModuleInfo {
    uses_promise: bool,
    promise_id: Option<LocalId>,
    /// Top-level user locals to allocate / print (source order).
    user_locals: Vec<(LocalId, SlotKind)>,
}

fn try_classify(module: &Module) -> Result<ModuleInfo, String> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let promise_id = module
        .locals
        .iter()
        .find(|l| l.name == "Promise")
        .map(|l| l.id);

    let mut user_ids = HashSet::new();
    collect_top_level_decl_ids(&module.body, &mut user_ids);

    let mut user_locals = Vec::new();
    let mut seen = HashSet::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            if !seen.insert(*local) {
                continue;
            }
            if !user_ids.contains(local) {
                continue;
            }
            let Some(loc) = by_id.get(local) else {
                continue;
            };
            // Skip nested function param / arguments bindings that appear as declares.
            if loc.name == "arguments" {
                continue;
            }
            let kind = match loc.ty {
                Type::Number => SlotKind::Number,
                Type::String => SlotKind::String,
                Type::Object | Type::Any | Type::Function => SlotKind::Object,
                _ => return Err(format!("unsupported local type for `{}`", loc.name)),
            };
            user_locals.push((*local, kind));
        }
    }

    // Async function declarations bind a function local without a `Declare`.
    for stmt in &module.body {
        if let Stmt::Function { local, is_async, .. } = stmt {
            if !*is_async {
                return Err("only async function declarations supported in Promise path".into());
            }
            if seen.insert(*local) {
                user_locals.push((*local, SlotKind::Object));
            }
        }
    }

    let mut uses_promise = false;
    for stmt in &module.body {
        check_stmt(stmt, promise_id, &mut uses_promise)?;
    }

    Ok(ModuleInfo {
        uses_promise,
        promise_id,
        user_locals,
    })
}

fn check_simple_params(params: &[Param]) -> Result<(), String> {
    for p in params {
        if p.rest || p.default.is_some() {
            return Err("rest/default params not supported".into());
        }
        if !matches!(p.pattern, Pattern::Local(_)) {
            return Err("only simple params supported".into());
        }
    }
    Ok(())
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

fn check_stmt(stmt: &Stmt, promise_id: Option<LocalId>, uses: &mut bool) -> Result<(), String> {
    match stmt {
        Stmt::Declare { init, .. } => {
            if let Some(e) = init {
                check_expr(e, promise_id, uses)?;
            }
            Ok(())
        }
        Stmt::Expr { expr } => check_expr(expr, promise_id, uses),
        Stmt::Return { value } => {
            if let Some(e) = value {
                check_expr(e, promise_id, uses)?;
            }
            Ok(())
        }
        Stmt::Throw { value } => check_expr(value, promise_id, uses),
        Stmt::Block { body } => {
            for s in body {
                check_stmt(s, promise_id, uses)?;
            }
            Ok(())
        }
        Stmt::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if !*is_async || *is_generator {
                return Err("only async function declarations supported in Promise path".into());
            }
            check_simple_params(params)?;
            *uses = true;
            for s in body {
                check_stmt(s, promise_id, uses)?;
            }
            Ok(())
        }
        other => Err(format!("unsupported statement in Promise path: {other:?}")),
    }
}

fn check_expr(expr: &Expr, promise_id: Option<LocalId>, uses: &mut bool) -> Result<(), String> {
    match expr {
        Expr::Local { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. } => Ok(()),
        Expr::Unary { op, arg, .. } => {
            match op {
                UnaryOp::TypeOf | UnaryOp::Minus | UnaryOp::Plus | UnaryOp::Not => {}
                UnaryOp::Await => {
                    *uses = true;
                }
                _ => return Err(format!("unsupported unary {op:?}")),
            }
            check_expr(arg, promise_id, uses)
        }
        Expr::Binary { left, op, right, .. } => {
            match op {
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Rem => {}
                _ => return Err(format!("unsupported binary {op:?}")),
            }
            check_expr(left, promise_id, uses)?;
            check_expr(right, promise_id, uses)
        }
        Expr::Assign { target, value, .. } => {
            match target {
                AssignTarget::Local(_) => {}
                _ => return Err("only local assignment supported in Promise path".into()),
            }
            check_expr(value, promise_id, uses)
        }
        Expr::Call { callee, args, optional, .. } => {
            if *optional {
                return Err("optional call not supported".into());
            }
            check_expr(callee, promise_id, uses)?;
            for a in args {
                match a {
                    Arg::Expr(e) => check_expr(e, promise_id, uses)?,
                    Arg::Spread(_) => return Err("spread args not supported".into()),
                }
            }
            Ok(())
        }
        Expr::New { callee, args, .. } => {
            if let Expr::Local { id, .. } = callee.as_ref() {
                if Some(*id) == promise_id {
                    *uses = true;
                } else {
                    return Err("only `new Promise` supported".into());
                }
            } else {
                return Err("only `new Promise` supported".into());
            }
            for a in args {
                match a {
                    Arg::Expr(e) => check_expr(e, promise_id, uses)?,
                    Arg::Spread(_) => return Err("spread args not supported".into()),
                }
            }
            Ok(())
        }
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } => {
            if *optional {
                return Err("optional member not supported in Promise path".into());
            }
            check_expr(object, promise_id, uses)?;
            if *computed {
                check_expr(property, promise_id, uses)?;
                return Ok(());
            }
            let Expr::String { value, .. } = property.as_ref() else {
                return Err("only string property keys supported".into());
            };
            let prop = value.to_string_lossy();
            match prop.as_ref() {
                "then" | "catch" | "finally" => {
                    *uses = true;
                    Ok(())
                }
                "length" | "status" | "value" | "reason" | "name" | "errors" => Ok(()),
                "resolve" | "reject" | "all" | "race" | "allSettled" | "any" => {
                    if let Expr::Local { id, .. } = object.as_ref() {
                        if Some(*id) == promise_id {
                            *uses = true;
                            return Ok(());
                        }
                    }
                    Err(
                        "only Promise.resolve / Promise.reject / Promise.all / Promise.race / Promise.allSettled / Promise.any supported"
                            .into(),
                    )
                }
                _ => Err(format!("unsupported property `{}` in Promise path", prop)),
            }
        }
        Expr::Array { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => check_expr(e, promise_id, uses)?,
                    ArrayElement::Spread(_) => {
                        return Err("spread in array not supported in Promise path".into());
                    }
                }
            }
            Ok(())
        }
        Expr::Function {
            name,
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if *is_generator {
                return Err("generator functions not supported in Promise path".into());
            }
            if *is_async {
                *uses = true;
            }
            if name.is_some() {
                return Err("named function expressions not supported".into());
            }
            check_simple_params(params)?;
            for s in body {
                check_stmt(s, promise_id, uses)?;
            }
            Ok(())
        }
        other => Err(format!("unsupported expr in Promise path: {other:?}")),
    }
}

struct Emitter<'a> {
    module: &'a Module,
    info: ModuleInfo,
    out: String,
    body: String,
    tmp: u32,
    next_fn: u32,
    /// local id → alloca ptr name (`%lN`)
    allocas: HashMap<LocalId, String>,
    /// string constants: content → global name
    str_globals: HashMap<String, String>,
    /// emitted helper function IR (appended before main)
    helpers: String,
    /// While lowering an executor: param local → (settle_fn_ssa, cap_ssa)
    executor_params: HashMap<LocalId, (String, String)>,
    /// While lowering a reaction: param local → value ssa (ptr)
    reaction_params: HashMap<LocalId, String>,
    /// Capture slots passed as reaction `data` (env of alloca ptrs, or single).
    reaction_captures: Vec<LocalId>,
    /// Known async function locals → LLVM function name (0-arg, returns Promise ptr).
    async_fns: HashMap<LocalId, String>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: ModuleInfo) -> Self {
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            tmp: 0,
            next_fn: 0,
            allocas: HashMap::new(),
            str_globals: HashMap::new(),
            helpers: String::new(),
            executor_params: HashMap::new(),
            reaction_params: HashMap::new(),
            reaction_captures: Vec::new(),
            async_fns: HashMap::new(),
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

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N06.03–N06.11 Promise/async via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "declare void @draconic_rt_gc_init()").ok();
        writeln!(self.out, "declare void @draconic_rt_print_i64(i64)").ok();
        writeln!(self.out, "declare void @draconic_rt_print_str(ptr)").ok();
        writeln!(self.out, "declare void @draconic_rt_job_drain()").ok();
        writeln!(self.out, "declare ptr @draconic_rt_promise_new()").ok();
        writeln!(
            self.out,
            "declare void @draconic_rt_promise_resolve(ptr, ptr)"
        )
        .ok();
        writeln!(
            self.out,
            "declare void @draconic_rt_promise_reject(ptr, ptr)"
        )
        .ok();
        writeln!(
            self.out,
            "declare ptr @draconic_rt_promise_construct(ptr, ptr)"
        )
        .ok();
        writeln!(
            self.out,
            "declare ptr @draconic_rt_promise_then(ptr, ptr, ptr, ptr, ptr)"
        )
        .ok();
        writeln!(
            self.out,
            "declare ptr @draconic_rt_promise_finally(ptr, ptr, ptr)"
        )
        .ok();
        writeln!(self.out, "declare ptr @draconic_rt_array_new(i64)").ok();
        writeln!(
            self.out,
            "declare void @draconic_rt_array_set(ptr, i64, ptr)"
        )
        .ok();
        writeln!(self.out, "declare ptr @draconic_rt_array_get(ptr, i64)").ok();
        writeln!(self.out, "declare i64 @draconic_rt_array_len(ptr)").ok();
        writeln!(self.out, "declare ptr @draconic_rt_promise_all(ptr)").ok();
        writeln!(self.out, "declare ptr @draconic_rt_promise_race(ptr)").ok();
        writeln!(
            self.out,
            "declare ptr @draconic_rt_promise_all_settled(ptr)"
        )
        .ok();
        writeln!(self.out, "declare ptr @draconic_rt_promise_any(ptr)").ok();
        writeln!(self.out, "declare ptr @draconic_rt_promise_await(ptr)").ok();
        writeln!(self.out, "declare ptr @draconic_rt_object_get(ptr, ptr)").ok();
        writeln!(self.out).ok();

        // Pre-scan string constants from typeof etc. by emitting body into buffer first.
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
                SlotKind::String | SlotKind::Object => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        // Drain microtasks before observing locals.
        writeln!(self.body, "  call void @draconic_rt_job_drain()").ok();

        for (id, kind) in self.info.user_locals.clone() {
            let ptr = self.allocas.get(&id).cloned().unwrap();
            match kind {
                SlotKind::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
                    writeln!(self.body, "  call void @draconic_rt_print_i64(i64 {v})").ok();
                }
                SlotKind::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  call void @draconic_rt_print_str(ptr {v})").ok();
                }
                SlotKind::Object => {
                    // Promise objects are not printed (observation is via number/string side effects).
                }
            }
        }

        // String globals
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

        // Helpers (executors + reactions)
        self.out.push_str(&self.helpers);
        if !self.helpers.is_empty() {
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
            Stmt::Return { value } => {
                // Only valid inside reaction helpers — handled separately.
                let _ = value;
                Err(diag("return at top level not supported"))
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
            } => {
                if !*is_async || *is_generator {
                    return Err(diag("only async function declarations supported"));
                }
                let fn_name = self.emit_async_fn(params, body)?;
                self.async_fns.insert(*local, fn_name.clone());
                let t = self.fresh();
                writeln!(self.body, "  {t} = bitcast ptr @{fn_name} to ptr").ok();
                self.store_local(*local, &t)?;
                Ok(())
            }
            _ => Err(diag("unsupported statement")),
        }
    }

    fn store_local(&mut self, id: LocalId, value_ptr_or_num: &str) -> Result<(), Diagnostic> {
        let Some(kind) = self.slot_kind(id) else {
            // Nested param / unknown — ignore stores to non-top-level when not in reaction.
            if let Some(ptr) = self.allocas.get(&id).cloned() {
                writeln!(self.body, "  store ptr {value_ptr_or_num}, ptr {ptr}").ok();
                return Ok(());
            }
            return Ok(());
        };
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("missing alloca"))?;
        match kind {
            SlotKind::Number => {
                let n = self.fresh();
                writeln!(
                    self.body,
                    "  {n} = ptrtoint ptr {value_ptr_or_num} to i64"
                )
                .ok();
                writeln!(self.body, "  store i64 {n}, ptr {ptr}").ok();
            }
            SlotKind::String | SlotKind::Object => {
                writeln!(
                    self.body,
                    "  store ptr {value_ptr_or_num}, ptr {ptr}"
                )
                .ok();
            }
        }
        Ok(())
    }

    fn store_local_in(
        &mut self,
        buf: &mut String,
        id: LocalId,
        value_ptr: &str,
    ) -> Result<(), Diagnostic> {
        let Some(kind) = self.slot_kind(id) else {
            return Ok(());
        };
        // Capture allocas live in main; reaction env (`%data`) holds pointers to them.
        let slot = self.reaction_capture_slot(id, buf)?;
        match kind {
            SlotKind::Number => {
                let n = format!("%n{}", self.tmp);
                self.tmp += 1;
                writeln!(buf, "  {n} = ptrtoint ptr {value_ptr} to i64").ok();
                writeln!(buf, "  store i64 {n}, ptr {slot}").ok();
            }
            SlotKind::String | SlotKind::Object => {
                writeln!(buf, "  store ptr {value_ptr}, ptr {slot}").ok();
            }
        }
        Ok(())
    }

    /// Resolve the alloca ptr for a captured local inside a reaction (`%data` env).
    fn reaction_capture_slot(
        &mut self,
        id: LocalId,
        buf: &mut String,
    ) -> Result<String, Diagnostic> {
        let Some(pos) = self.reaction_captures.iter().position(|c| *c == id) else {
            return Err(diag("capture not in reaction env"));
        };
        if self.reaction_captures.len() == 1 {
            return Ok("%data".into());
        }
        let gep = format!("%envp{}", self.tmp);
        self.tmp += 1;
        let slot = format!("%slot{}", self.tmp);
        self.tmp += 1;
        let n = self.reaction_captures.len();
        writeln!(
            buf,
            "  {gep} = getelementptr inbounds [{n} x ptr], ptr %data, i64 0, i64 {pos}"
        )
        .ok();
        writeln!(buf, "  {slot} = load ptr, ptr {gep}").ok();
        Ok(slot)
    }

    fn slot_kind(&self, id: LocalId) -> Option<SlotKind> {
        self.info
            .user_locals
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, k)| *k)
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n: i64 = parse_number(raw)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                Ok(t)
            }
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => self.load_local(*id),
            Expr::Unary { op, arg, .. } => match op {
                UnaryOp::TypeOf => self.emit_typeof(arg),
                UnaryOp::Minus => {
                    let a = self.emit_expr(arg)?;
                    let n = self.fresh();
                    let m = self.fresh();
                    let r = self.fresh();
                    writeln!(self.body, "  {n} = ptrtoint ptr {a} to i64").ok();
                    writeln!(self.body, "  {m} = sub i64 0, {n}").ok();
                    writeln!(self.body, "  {r} = inttoptr i64 {m} to ptr").ok();
                    Ok(r)
                }
                UnaryOp::Plus => self.emit_expr(arg),
                UnaryOp::Await => Err(diag("await only valid inside async function body")),
                _ => Err(diag("unsupported unary")),
            },
            Expr::Binary { left, op, right, .. } => {
                let l = self.emit_expr(left)?;
                let r = self.emit_expr(right)?;
                let ln = self.fresh();
                let rn = self.fresh();
                let out = self.fresh();
                let res = self.fresh();
                writeln!(self.body, "  {ln} = ptrtoint ptr {l} to i64").ok();
                writeln!(self.body, "  {rn} = ptrtoint ptr {r} to i64").ok();
                let inst = match op {
                    BinaryOp::Add => "add",
                    BinaryOp::Sub => "sub",
                    BinaryOp::Mul => "mul",
                    BinaryOp::Div => "sdiv",
                    BinaryOp::Rem => "srem",
                    _ => return Err(diag("unsupported binary")),
                };
                writeln!(self.body, "  {out} = {inst} i64 {ln}, {rn}").ok();
                writeln!(self.body, "  {res} = inttoptr i64 {out} to ptr").ok();
                Ok(res)
            }
            Expr::Assign { target, value, .. } => {
                let v = self.emit_expr(value)?;
                match target {
                    AssignTarget::Local(id) => {
                        self.store_local(*id, &v)?;
                        Ok(v)
                    }
                    _ => Err(diag("unsupported assign target")),
                }
            }
            Expr::New { callee, args, .. } => self.emit_new_promise(callee, args),
            Expr::Call { callee, args, .. } => self.emit_call(callee, args),
            Expr::Array { elements, .. } => self.emit_array(elements),
            Expr::Member {
                object,
                property,
                computed,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("optional member not supported"));
                }
                self.emit_member(object, property, *computed)
            }
            Expr::Function {
                name,
                params,
                body,
                is_async,
                is_generator,
                ..
            } => {
                if name.is_some() {
                    return Err(diag("named function expressions not supported"));
                }
                if *is_generator {
                    return Err(diag("generator functions not supported"));
                }
                if *is_async {
                    let fn_name = self.emit_async_fn(params, body)?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = bitcast ptr @{fn_name} to ptr").ok();
                    return Ok(t);
                }
                Err(diag("bare function expr not supported at value position"))
            }
            _ => Err(diag("unsupported expression")),
        }
    }

    fn emit_array(&mut self, elements: &[ArrayElement]) -> Result<String, Diagnostic> {
        let n = elements.len();
        let arr = self.fresh();
        writeln!(
            self.body,
            "  {arr} = call ptr @draconic_rt_array_new(i64 {n})"
        )
        .ok();
        for (i, el) in elements.iter().enumerate() {
            let ArrayElement::Expr(e) = el else {
                return Err(diag("spread array elements not supported"));
            };
            let v = self.emit_expr(e)?;
            writeln!(
                self.body,
                "  call void @draconic_rt_array_set(ptr {arr}, i64 {i}, ptr {v})"
            )
            .ok();
        }
        Ok(arr)
    }

    fn emit_member(
        &mut self,
        object: &Expr,
        property: &Expr,
        computed: bool,
    ) -> Result<String, Diagnostic> {
        if computed {
            let obj = self.emit_expr(object)?;
            let idx_ptr = self.emit_expr(property)?;
            let idx = self.fresh();
            let t = self.fresh();
            writeln!(self.body, "  {idx} = ptrtoint ptr {idx_ptr} to i64").ok();
            writeln!(
                self.body,
                "  {t} = call ptr @draconic_rt_array_get(ptr {obj}, i64 {idx})"
            )
            .ok();
            return Ok(t);
        }
        let Expr::String { value, .. } = property else {
            return Err(diag("only string property keys supported"));
        };
        let prop = value.to_string_lossy();
        if prop == "length" {
            let obj = self.emit_expr(object)?;
            let n = self.fresh();
            let t = self.fresh();
            writeln!(self.body, "  {n} = call i64 @draconic_rt_array_len(ptr {obj})").ok();
            writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
            return Ok(t);
        }
        if prop == "status"
            || prop == "value"
            || prop == "reason"
            || prop == "name"
            || prop == "errors"
        {
            let obj = self.emit_expr(object)?;
            let key = self.string_const(&prop)?;
            let t = self.fresh();
            writeln!(
                self.body,
                "  {t} = call ptr @draconic_rt_object_get(ptr {obj}, ptr {key})"
            )
            .ok();
            return Ok(t);
        }
        Err(diag(format!("unsupported member `{}`", prop)))
    }

    fn load_local(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        if let Some((settle, cap)) = self.executor_params.get(&id).cloned() {
            // Calling resolve/reject is handled in emit_call; loading as value is unsupported.
            let _ = (settle, cap);
            return Err(diag("cannot use resolve/reject as values"));
        }
        if let Some(v) = self.reaction_params.get(&id).cloned() {
            return Ok(v);
        }
        let Some(kind) = self.slot_kind(id) else {
            // Unknown local (e.g. unused) → null
            let t = self.fresh();
            writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
            return Ok(t);
        };
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("missing alloca"))?;
        match kind {
            SlotKind::Number => {
                let n = self.fresh();
                let t = self.fresh();
                writeln!(self.body, "  {n} = load i64, ptr {ptr}").ok();
                writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                Ok(t)
            }
            SlotKind::String | SlotKind::Object => {
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
        }
    }

    fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        // typeof Promise → "function"
        if let Expr::Local { id, .. } = arg {
            if Some(*id) == self.info.promise_id {
                return self.string_const("function");
            }
        }
        // typeof Promise.resolve / Promise.reject / p.then|catch|finally → "function"
        if let Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } = arg
        {
            if !*optional && !*computed {
                if let Expr::String { value, .. } = property.as_ref() {
                    let prop = value.to_string_lossy();
                    match prop.as_ref() {
                        "resolve" | "reject" | "all" | "race" | "allSettled" | "any" => {
                            if let Expr::Local { id, .. } = object.as_ref() {
                                if Some(*id) == self.info.promise_id {
                                    return self.string_const("function");
                                }
                            }
                        }
                        "then" | "catch" | "finally" => {
                            let _ = self.emit_expr(object)?;
                            return self.string_const("function");
                        }
                        _ => {}
                    }
                }
            }
        }
        // typeof promise object → "object"
        let _ = self.emit_expr(arg)?;
        self.string_const("object")
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

    fn emit_new_promise(
        &mut self,
        callee: &Expr,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("new callee must be Promise"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only new Promise supported"));
        }
        if args.len() != 1 {
            return Err(diag("Promise constructor expects 1 argument"));
        }
        let Arg::Expr(exec_expr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let Expr::Function { params, body, .. } = exec_expr else {
            return Err(diag("Promise executor must be a function expression"));
        };
        let fn_name = self.emit_executor_fn(params, body)?;
        let t = self.fresh();
        writeln!(
            self.body,
            "  {t} = call ptr @draconic_rt_promise_construct(ptr @{fn_name}, ptr null)"
        )
        .ok();
        Ok(t)
    }

    fn emit_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        // resolve(value) / reject(reason) inside executor
        if let Expr::Local { id, .. } = callee {
            if let Some((settle, cap)) = self.executor_params.get(id).cloned() {
                if args.len() != 1 {
                    return Err(diag("resolve/reject expect 1 arg"));
                }
                let Arg::Expr(vexpr) = &args[0] else {
                    return Err(diag("spread not supported"));
                };
                let v = self.emit_expr(vexpr)?;
                writeln!(
                    self.body,
                    "  call void {settle}(ptr {cap}, ptr {v})"
                )
                .ok();
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                return Ok(t);
            }
            // User async function call (0+ simple args; N06.10–N06.11).
            let mut arg_ssas = Vec::with_capacity(args.len());
            for a in args {
                let Arg::Expr(e) = a else {
                    return Err(diag("spread args not supported"));
                };
                arg_ssas.push(self.emit_expr(e)?);
            }
            let arg_list = format_ptr_args(&arg_ssas);
            if let Some(name) = self.async_fns.get(id).cloned() {
                let t = self.fresh();
                writeln!(self.body, "  {t} = call ptr @{name}({arg_list})").ok();
                return Ok(t);
            }
            if matches!(self.slot_kind(*id), Some(SlotKind::Object)) {
                let fp = self.load_local(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = call ptr {fp}({arg_list})").ok();
                return Ok(t);
            }
        }

        // Promise.resolve(v) / Promise.reject(v) / promise.then / promise.catch
        if let Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } = callee
        {
            if *optional || *computed {
                return Err(diag("unsupported member call"));
            }
            let Expr::String { value, .. } = property.as_ref() else {
                return Err(diag("only string property calls supported"));
            };
            let prop = value.to_string_lossy();
            match prop.as_ref() {
                "resolve" | "reject" => {
                    return self.emit_promise_static(object, prop.as_ref(), args);
                }
                "all" => {
                    return self.emit_promise_all(object, args);
                }
                "race" => {
                    return self.emit_promise_race(object, args);
                }
                "allSettled" => {
                    return self.emit_promise_all_settled(object, args);
                }
                "any" => {
                    return self.emit_promise_any(object, args);
                }
                "then" => {
                    let p = self.emit_expr(object)?;
                    let mut on_ful = "null".to_string();
                    let mut ful_data = "null".to_string();
                    let mut on_rej = "null".to_string();
                    let mut rej_data = "null".to_string();
                    if let Some(Arg::Expr(f0)) = args.first() {
                        if let Expr::Function { params, body, .. } = f0 {
                            let (name, data) = self.emit_reaction_fn(params, body)?;
                            on_ful = format!("@{name}");
                            ful_data = data;
                        } else {
                            return Err(diag("then callback must be function expression"));
                        }
                    }
                    if let Some(Arg::Expr(f1)) = args.get(1) {
                        if let Expr::Function { params, body, .. } = f1 {
                            let (name, data) = self.emit_reaction_fn(params, body)?;
                            on_rej = format!("@{name}");
                            rej_data = data;
                        } else {
                            return Err(diag("then callback must be function expression"));
                        }
                    }
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {t} = call ptr @draconic_rt_promise_then(ptr {p}, ptr {on_ful}, ptr {ful_data}, ptr {on_rej}, ptr {rej_data})"
                    )
                    .ok();
                    return Ok(t);
                }
                "catch" => {
                    // p.catch(onRejected) ≡ p.then(undefined, onRejected)
                    if args.len() != 1 {
                        return Err(diag("catch expects 1 argument"));
                    }
                    let p = self.emit_expr(object)?;
                    let Arg::Expr(f0) = &args[0] else {
                        return Err(diag("spread not supported"));
                    };
                    let Expr::Function { params, body, .. } = f0 else {
                        return Err(diag("catch callback must be function expression"));
                    };
                    let (name, data) = self.emit_reaction_fn(params, body)?;
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {t} = call ptr @draconic_rt_promise_then(ptr {p}, ptr null, ptr null, ptr @{name}, ptr {data})"
                    )
                    .ok();
                    return Ok(t);
                }
                "finally" => {
                    if args.len() != 1 {
                        return Err(diag("finally expects 1 argument"));
                    }
                    let p = self.emit_expr(object)?;
                    let Arg::Expr(f0) = &args[0] else {
                        return Err(diag("spread not supported"));
                    };
                    let Expr::Function { params, body, .. } = f0 else {
                        return Err(diag("finally callback must be function expression"));
                    };
                    let (name, data) = self.emit_reaction_fn(params, body)?;
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {t} = call ptr @draconic_rt_promise_finally(ptr {p}, ptr @{name}, ptr {data})"
                    )
                    .ok();
                    return Ok(t);
                }
                _ => return Err(diag(format!("unsupported method `{prop}`"))),
            }
        }

        Err(diag("unsupported call"))
    }

    fn emit_promise_static(
        &mut self,
        object: &Expr,
        which: &str,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = object else {
            return Err(diag("static Promise methods require Promise receiver"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only Promise.resolve / Promise.reject supported"));
        }
        if args.len() != 1 {
            return Err(diag(format!("Promise.{which} expects 1 argument")));
        }
        let Arg::Expr(vexpr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let v = self.emit_expr(vexpr)?;
        let p = self.fresh();
        writeln!(self.body, "  {p} = call ptr @draconic_rt_promise_new()").ok();
        match which {
            "resolve" => {
                writeln!(
                    self.body,
                    "  call void @draconic_rt_promise_resolve(ptr {p}, ptr {v})"
                )
                .ok();
            }
            "reject" => {
                writeln!(
                    self.body,
                    "  call void @draconic_rt_promise_reject(ptr {p}, ptr {v})"
                )
                .ok();
            }
            _ => return Err(diag("internal: bad Promise static")),
        }
        Ok(p)
    }

    fn emit_promise_all(
        &mut self,
        object: &Expr,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = object else {
            return Err(diag("Promise.all requires Promise receiver"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only Promise.all supported"));
        }
        if args.len() != 1 {
            return Err(diag("Promise.all expects 1 argument"));
        }
        let Arg::Expr(vexpr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let arr = self.emit_expr(vexpr)?;
        let t = self.fresh();
        writeln!(
            self.body,
            "  {t} = call ptr @draconic_rt_promise_all(ptr {arr})"
        )
        .ok();
        Ok(t)
    }

    fn emit_promise_race(
        &mut self,
        object: &Expr,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = object else {
            return Err(diag("Promise.race requires Promise receiver"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only Promise.race supported"));
        }
        if args.len() != 1 {
            return Err(diag("Promise.race expects 1 argument"));
        }
        let Arg::Expr(vexpr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let arr = self.emit_expr(vexpr)?;
        let t = self.fresh();
        writeln!(
            self.body,
            "  {t} = call ptr @draconic_rt_promise_race(ptr {arr})"
        )
        .ok();
        Ok(t)
    }

    fn emit_promise_all_settled(
        &mut self,
        object: &Expr,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = object else {
            return Err(diag("Promise.allSettled requires Promise receiver"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only Promise.allSettled supported"));
        }
        if args.len() != 1 {
            return Err(diag("Promise.allSettled expects 1 argument"));
        }
        let Arg::Expr(vexpr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let arr = self.emit_expr(vexpr)?;
        let t = self.fresh();
        writeln!(
            self.body,
            "  {t} = call ptr @draconic_rt_promise_all_settled(ptr {arr})"
        )
        .ok();
        Ok(t)
    }

    fn emit_promise_any(
        &mut self,
        object: &Expr,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = object else {
            return Err(diag("Promise.any requires Promise receiver"));
        };
        if Some(*id) != self.info.promise_id {
            return Err(diag("only Promise.any supported"));
        }
        if args.len() != 1 {
            return Err(diag("Promise.any expects 1 argument"));
        }
        let Arg::Expr(vexpr) = &args[0] else {
            return Err(diag("spread not supported"));
        };
        let arr = self.emit_expr(vexpr)?;
        let t = self.fresh();
        writeln!(
            self.body,
            "  {t} = call ptr @draconic_rt_promise_any(ptr {arr})"
        )
        .ok();
        Ok(t)
    }

    /// Emit an async function/arrow: returns a Promise (N06.10–N06.11).
    /// Supports simple ident params (no rest/default); body may use `await` / `return` / `throw`.
    fn emit_async_fn(&mut self, params: &[Param], body: &[Stmt]) -> Result<String, Diagnostic> {
        let mut param_ids = Vec::with_capacity(params.len());
        for p in params {
            if p.rest || p.default.is_some() {
                return Err(diag("rest/default params not supported on async"));
            }
            let Pattern::Local(id) = &p.pattern else {
                return Err(diag("only simple async params supported"));
            };
            param_ids.push(*id);
        }
        let fn_name = self.fresh_fn("async");

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_exec = std::mem::take(&mut self.executor_params);
        let saved_react = std::mem::take(&mut self.reaction_params);
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.executor_params.clear();
        self.reaction_params.clear();
        self.reaction_captures.clear();
        for (i, id) in param_ids.iter().enumerate() {
            self.reaction_params.insert(*id, format!("%arg{i}"));
        }

        let ret_promise = self.emit_async_body(body)?;

        let mut sig_params = String::new();
        for i in 0..param_ids.len() {
            if i > 0 {
                sig_params.push_str(", ");
            }
            write!(sig_params, "ptr %arg{i}").ok();
        }
        let mut fn_ir = String::new();
        writeln!(fn_ir, "define ptr @{fn_name}({sig_params}) {{").ok();
        writeln!(fn_ir, "entry:").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret ptr {ret_promise}").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.executor_params = saved_exec;
        self.reaction_params = saved_react;
        self.reaction_captures = saved_caps;
        Ok(fn_name)
    }

    /// Lower async body into `self.body`; returns SSA of the result Promise.
    fn emit_async_body(&mut self, body: &[Stmt]) -> Result<String, Diagnostic> {
        // Linear await: optional prefix without await, then `let x = await e`, then rest.
        for (i, stmt) in body.iter().enumerate() {
            if let Some((bind, await_arg)) = match_await_declare(stmt) {
                for s in &body[..i] {
                    if stmt_contains_await(s) {
                        return Err(diag("await only supported as `let x = await expr`"));
                    }
                    self.emit_async_sync_stmt(s)?;
                }
                let v = self.emit_expr(await_arg)?;
                let p = self.fresh();
                writeln!(
                    self.body,
                    "  {p} = call ptr @draconic_rt_promise_await(ptr {v})"
                )
                .ok();
                let rest = &body[i + 1..];
                let (cont, data) = self.emit_async_continuation(bind, rest)?;
                let out = self.fresh();
                writeln!(
                    self.body,
                    "  {out} = call ptr @draconic_rt_promise_then(ptr {p}, ptr @{cont}, ptr {data}, ptr null, ptr null)"
                )
                .ok();
                return Ok(out);
            }
            if stmt_contains_await(stmt) {
                return Err(diag("await only supported as `let x = await expr`"));
            }
        }
        self.emit_async_sync_body(body)
    }

    fn emit_async_sync_body(&mut self, body: &[Stmt]) -> Result<String, Diagnostic> {
        let p = self.fresh();
        writeln!(self.body, "  {p} = call ptr @draconic_rt_promise_new()").ok();
        for stmt in body {
            match stmt {
                Stmt::Return { value } => {
                    let v = if let Some(e) = value {
                        self.emit_expr(e)?
                    } else {
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                        t
                    };
                    writeln!(
                        self.body,
                        "  call void @draconic_rt_promise_resolve(ptr {p}, ptr {v})"
                    )
                    .ok();
                    return Ok(p);
                }
                Stmt::Throw { value } => {
                    let v = self.emit_expr(value)?;
                    writeln!(
                        self.body,
                        "  call void @draconic_rt_promise_reject(ptr {p}, ptr {v})"
                    )
                    .ok();
                    return Ok(p);
                }
                other => self.emit_async_sync_stmt(other)?,
            }
        }
        let u = self.fresh();
        writeln!(self.body, "  {u} = inttoptr i64 0 to ptr").ok();
        writeln!(
            self.body,
            "  call void @draconic_rt_promise_resolve(ptr {p}, ptr {u})"
        )
        .ok();
        Ok(p)
    }

    fn emit_async_sync_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr)?;
                Ok(())
            }
            Stmt::Declare { local, init, .. } => {
                // Nested locals inside async without await: store via reaction_params map as SSA.
                if let Some(e) = init {
                    let v = self.emit_expr(e)?;
                    self.reaction_params.insert(*local, v);
                }
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_async_sync_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Return { .. } | Stmt::Throw { .. } => {
                Err(diag("return/throw must be handled by async body driver"))
            }
            _ => Err(diag("unsupported statement in async body")),
        }
    }

    /// Continuation after `let bind = await …`: reaction binds `%value` to `bind`, runs `rest`.
    fn emit_async_continuation(
        &mut self,
        bind: LocalId,
        rest: &[Stmt],
    ) -> Result<(String, String), Diagnostic> {
        // Captures: top-level number/string locals assigned in rest.
        let mut assigned = HashSet::new();
        collect_assigned_locals(rest, &mut assigned);
        let mut captures: Vec<LocalId> = assigned
            .into_iter()
            .filter(|id| {
                matches!(
                    self.slot_kind(*id),
                    Some(SlotKind::Number) | Some(SlotKind::String)
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

        let fn_name = self.fresh_fn("async_cont");

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_exec = std::mem::take(&mut self.executor_params);
        let saved_react = std::mem::take(&mut self.reaction_params);
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.executor_params.clear();
        self.reaction_params.clear();
        self.reaction_params.insert(bind, "%value".into());
        self.reaction_captures = captures;

        // If rest has another await, nest; else evaluate returns as reaction return value.
        let mut ret_val = "%value".to_string();
        let mut i = 0;
        while i < rest.len() {
            let stmt = &rest[i];
            if let Some((next_bind, await_arg)) = match_await_declare(stmt) {
                let v = self.emit_expr_in_reaction(await_arg)?;
                let p = self.fresh();
                writeln!(
                    self.body,
                    "  {p} = call ptr @draconic_rt_promise_await(ptr {v})"
                )
                .ok();
                // Nested await: build another continuation for remaining rest and return that promise.
                // Note: reaction resolve does not assimilate thenables; nested await deferred.
                let nested_rest = &rest[i + 1..];
                // Re-enter via emit_async_continuation needs parent body for data — not supported.
                let _ = (next_bind, nested_rest);
                return Err(diag("multiple awaits in one async function not supported yet"));
            }
            match stmt {
                Stmt::Return { value } => {
                    if let Some(e) = value {
                        ret_val = self.emit_expr_in_reaction(e)?;
                    } else {
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                        ret_val = t;
                    }
                    i += 1;
                }
                Stmt::Throw { value } => {
                    // Reject by throwing is not available in reaction; unsupported.
                    let _ = value;
                    return Err(diag("throw after await not supported yet"));
                }
                Stmt::Expr { expr } => {
                    if let Expr::Assign {
                        target: AssignTarget::Local(id),
                        value,
                        ..
                    } = expr
                    {
                        let v = self.emit_expr_in_reaction(value)?;
                        let mut buf = std::mem::take(&mut self.body);
                        self.store_local_in(&mut buf, *id, &v)?;
                        self.body = buf;
                        ret_val = v;
                    } else {
                        ret_val = self.emit_expr_in_reaction(expr)?;
                    }
                    i += 1;
                }
                Stmt::Declare { local, init, .. } => {
                    if let Some(e) = init {
                        let v = self.emit_expr_in_reaction(e)?;
                        self.reaction_params.insert(*local, v.clone());
                        ret_val = v;
                    }
                    i += 1;
                }
                Stmt::Block { body: inner } => {
                    for s in inner {
                        match s {
                            Stmt::Return { value } => {
                                if let Some(e) = value {
                                    ret_val = self.emit_expr_in_reaction(e)?;
                                }
                            }
                            Stmt::Expr { expr } => {
                                if let Expr::Assign {
                                    target: AssignTarget::Local(id),
                                    value,
                                    ..
                                } = expr
                                {
                                    let v = self.emit_expr_in_reaction(value)?;
                                    let mut buf = std::mem::take(&mut self.body);
                                    self.store_local_in(&mut buf, *id, &v)?;
                                    self.body = buf;
                                    ret_val = v;
                                } else {
                                    ret_val = self.emit_expr_in_reaction(expr)?;
                                }
                            }
                            _ => return Err(diag("unsupported stmt in async continuation block")),
                        }
                    }
                    i += 1;
                }
                _ => return Err(diag("unsupported stmt in async continuation")),
            }
        }

        let mut fn_ir = String::new();
        writeln!(
            fn_ir,
            "define ptr @{fn_name}(ptr %data, ptr %value) {{"
        )
        .ok();
        writeln!(fn_ir, "entry:").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret ptr {ret_val}").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.executor_params = saved_exec;
        self.reaction_params = saved_react;
        self.reaction_captures = saved_caps;

        Ok((fn_name, data_operand))
    }

    fn emit_executor_fn(
        &mut self,
        params: &[Param],
        body: &[Stmt],
    ) -> Result<String, Diagnostic> {
        let fn_name = self.fresh_fn("exec");
        let mut resolve_param = None;
        let mut reject_param = None;
        for (i, p) in params.iter().enumerate() {
            let Pattern::Local(id) = &p.pattern else {
                return Err(diag("bad param"));
            };
            if i == 0 {
                resolve_param = Some(*id);
            } else if i == 1 {
                reject_param = Some(*id);
            }
        }

        // Save main emission state
        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_exec = std::mem::take(&mut self.executor_params);
        let saved_react = std::mem::take(&mut self.reaction_params);
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.executor_params.clear();
        if let Some(id) = resolve_param {
            self.executor_params
                .insert(id, ("%resolve".into(), "%resolve_cap".into()));
        }
        if let Some(id) = reject_param {
            self.executor_params
                .insert(id, ("%reject".into(), "%reject_cap".into()));
        }

        for stmt in body {
            match stmt {
                Stmt::Expr { expr } => {
                    let _ = self.emit_expr(expr)?;
                }
                Stmt::Return { value } => {
                    if let Some(e) = value {
                        let _ = self.emit_expr(e)?;
                    }
                }
                Stmt::Block { body } => {
                    for s in body {
                        if let Stmt::Expr { expr } = s {
                            let _ = self.emit_expr(expr)?;
                        } else if let Stmt::Return { value } = s {
                            if let Some(e) = value {
                                let _ = self.emit_expr(e)?;
                            }
                        } else {
                            return Err(diag("unsupported stmt in executor"));
                        }
                    }
                }
                _ => return Err(diag("unsupported stmt in executor")),
            }
        }

        let mut fn_ir = String::new();
        writeln!(
            fn_ir,
            "define void @{fn_name}(ptr %data, ptr %resolve, ptr %resolve_cap, ptr %reject, ptr %reject_cap) {{"
        )
        .ok();
        writeln!(fn_ir, "entry:").ok();
        writeln!(fn_ir, "  ; data unused").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret void").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.executor_params = saved_exec;
        self.reaction_params = saved_react;
        self.reaction_captures = saved_caps;
        Ok(fn_name)
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

        // Find assigned top-level number/string locals (captures), stable order by id.
        let mut assigned = HashSet::new();
        collect_assigned_locals(body, &mut assigned);
        let mut captures: Vec<LocalId> = assigned
            .into_iter()
            .filter(|id| {
                matches!(
                    self.slot_kind(*id),
                    Some(SlotKind::Number) | Some(SlotKind::String)
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
            // Env: [N x ptr] of capture allocas, allocated in main.
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
        let saved_exec = std::mem::take(&mut self.executor_params);
        let saved_react = std::mem::take(&mut self.reaction_params);
        let saved_caps = std::mem::take(&mut self.reaction_captures);

        self.tmp = 0;
        self.body.clear();
        self.executor_params.clear();
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
                        let v = self.emit_expr_in_reaction(value)?;
                        // store into capture
                        let mut buf = std::mem::take(&mut self.body);
                        self.store_local_in(&mut buf, *id, &v)?;
                        self.body = buf;
                        ret_val = v;
                    } else {
                        let v = self.emit_expr_in_reaction(expr)?;
                        ret_val = v;
                    }
                }
                Stmt::Return { value } => {
                    if let Some(e) = value {
                        ret_val = self.emit_expr_in_reaction(e)?;
                    } else {
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                        ret_val = t;
                    }
                }
                Stmt::Block { body: inner } => {
                    for s in inner {
                        match s {
                            Stmt::Expr { expr } => {
                                if let Expr::Assign {
                                    target: AssignTarget::Local(id),
                                    value,
                                    ..
                                } = expr
                                {
                                    let v = self.emit_expr_in_reaction(value)?;
                                    let mut buf = std::mem::take(&mut self.body);
                                    self.store_local_in(&mut buf, *id, &v)?;
                                    self.body = buf;
                                    ret_val = v;
                                } else {
                                    ret_val = self.emit_expr_in_reaction(expr)?;
                                }
                            }
                            Stmt::Return { value } => {
                                if let Some(e) = value {
                                    ret_val = self.emit_expr_in_reaction(e)?;
                                }
                            }
                            _ => return Err(diag("unsupported stmt in reaction")),
                        }
                    }
                }
                _ => return Err(diag("unsupported stmt in reaction")),
            }
        }

        let mut fn_ir = String::new();
        writeln!(
            fn_ir,
            "define ptr @{fn_name}(ptr %data, ptr %value) {{"
        )
        .ok();
        writeln!(fn_ir, "entry:").ok();
        fn_ir.push_str(&self.body);
        writeln!(fn_ir, "  ret ptr {ret_val}").ok();
        writeln!(fn_ir, "}}").ok();
        self.helpers.push_str(&fn_ir);
        self.helpers.push('\n');

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.executor_params = saved_exec;
        self.reaction_params = saved_react;
        self.reaction_captures = saved_caps;

        Ok((fn_name, data_operand))
    }

    /// Emit expression while building a reaction (writes into self.body).
    fn emit_expr_in_reaction(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n: i64 = parse_number(raw)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                Ok(t)
            }
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                if let Some(v) = self.reaction_params.get(id).cloned() {
                    return Ok(v);
                }
                if self.reaction_captures.contains(id) {
                    let mut buf = String::new();
                    let slot = self.reaction_capture_slot(*id, &mut buf)?;
                    self.body.push_str(&buf);
                    match self.slot_kind(*id) {
                        Some(SlotKind::String) | Some(SlotKind::Object) => {
                            let t = self.fresh();
                            writeln!(self.body, "  {t} = load ptr, ptr {slot}").ok();
                            return Ok(t);
                        }
                        _ => {
                            let n = self.fresh();
                            let t = self.fresh();
                            writeln!(self.body, "  {n} = load i64, ptr {slot}").ok();
                            writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                            return Ok(t);
                        }
                    }
                }
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                Ok(t)
            }
            Expr::Unary {
                op: UnaryOp::Minus,
                arg,
                ..
            } => {
                let a = self.emit_expr_in_reaction(arg)?;
                let n = self.fresh();
                let m = self.fresh();
                let r = self.fresh();
                writeln!(self.body, "  {n} = ptrtoint ptr {a} to i64").ok();
                writeln!(self.body, "  {m} = sub i64 0, {n}").ok();
                writeln!(self.body, "  {r} = inttoptr i64 {m} to ptr").ok();
                Ok(r)
            }
            Expr::Binary { left, op, right, .. } => {
                let l = self.emit_expr_in_reaction(left)?;
                let r = self.emit_expr_in_reaction(right)?;
                let ln = self.fresh();
                let rn = self.fresh();
                let out = self.fresh();
                let res = self.fresh();
                writeln!(self.body, "  {ln} = ptrtoint ptr {l} to i64").ok();
                writeln!(self.body, "  {rn} = ptrtoint ptr {r} to i64").ok();
                let inst = match op {
                    BinaryOp::Add => "add",
                    BinaryOp::Sub => "sub",
                    BinaryOp::Mul => "mul",
                    BinaryOp::Div => "sdiv",
                    BinaryOp::Rem => "srem",
                    _ => return Err(diag("unsupported binary in reaction")),
                };
                writeln!(self.body, "  {out} = {inst} i64 {ln}, {rn}").ok();
                writeln!(self.body, "  {res} = inttoptr i64 {out} to ptr").ok();
                Ok(res)
            }
            Expr::Member {
                object,
                property,
                computed,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("optional member not supported in reaction"));
                }
                if *computed {
                    let obj = self.emit_expr_in_reaction(object)?;
                    let idx_ptr = self.emit_expr_in_reaction(property)?;
                    let idx = self.fresh();
                    let t = self.fresh();
                    writeln!(self.body, "  {idx} = ptrtoint ptr {idx_ptr} to i64").ok();
                    writeln!(
                        self.body,
                        "  {t} = call ptr @draconic_rt_array_get(ptr {obj}, i64 {idx})"
                    )
                    .ok();
                    return Ok(t);
                }
                let Expr::String { value, .. } = property.as_ref() else {
                    return Err(diag("only string property keys supported in reaction"));
                };
                let prop = value.to_string_lossy();
                if prop == "length" {
                    let obj = self.emit_expr_in_reaction(object)?;
                    let n = self.fresh();
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {n} = call i64 @draconic_rt_array_len(ptr {obj})"
                    )
                    .ok();
                    writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                    return Ok(t);
                }
                if prop == "status"
                    || prop == "value"
                    || prop == "reason"
                    || prop == "name"
                    || prop == "errors"
                {
                    let obj = self.emit_expr_in_reaction(object)?;
                    let key = self.string_const(&prop)?;
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {t} = call ptr @draconic_rt_object_get(ptr {obj}, ptr {key})"
                    )
                    .ok();
                    return Ok(t);
                }
                Err(diag(format!("unsupported member `{}` in reaction", prop)))
            }
            _ => Err(diag("unsupported expr in reaction")),
        }
    }
}

fn collect_assigned_locals(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Expr { expr } => {
                if let Expr::Assign {
                    target: AssignTarget::Local(id),
                    ..
                } = expr
                {
                    out.insert(*id);
                }
            }
            Stmt::Block { body } => collect_assigned_locals(body, out),
            Stmt::Return { .. } => {}
            _ => {}
        }
    }
}

fn match_await_declare(stmt: &Stmt) -> Option<(LocalId, &Expr)> {
    match stmt {
        Stmt::Declare {
            local,
            init: Some(Expr::Unary {
                op: UnaryOp::Await,
                arg,
                ..
            }),
            ..
        } => Some((*local, arg.as_ref())),
        _ => None,
    }
}

fn stmt_contains_await(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init, .. } => init.as_ref().is_some_and(expr_contains_await),
        Stmt::Expr { expr } => expr_contains_await(expr),
        Stmt::Return { value } => value.as_ref().is_some_and(expr_contains_await),
        Stmt::Throw { value } => expr_contains_await(value),
        Stmt::Block { body } => body.iter().any(stmt_contains_await),
        _ => false,
    }
}

fn expr_contains_await(expr: &Expr) -> bool {
    match expr {
        Expr::Unary {
            op: UnaryOp::Await, ..
        } => true,
        Expr::Unary { arg, .. } => expr_contains_await(arg),
        Expr::Binary { left, right, .. } => {
            expr_contains_await(left) || expr_contains_await(right)
        }
        Expr::Assign { value, .. } => expr_contains_await(value),
        Expr::Call { callee, args, .. } => {
            expr_contains_await(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_await(e),
                })
        }
        Expr::New { callee, args, .. } => {
            expr_contains_await(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_await(e),
                })
        }
        Expr::Member {
            object, property, ..
        } => expr_contains_await(object) || expr_contains_await(property),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_await(e),
        }),
        Expr::Function { body, .. } => body.iter().any(stmt_contains_await),
        _ => false,
    }
}

fn format_ptr_args(args: &[String]) -> String {
    let mut out = String::new();
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write!(out, "ptr {a}").ok();
    }
    out
}

fn parse_number(raw: &str) -> Result<i64, Diagnostic> {
    let s = raw.trim();
    if let Ok(n) = s.parse::<i64>() {
        return Ok(n);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Ok(f as i64);
    }
    Err(diag(&format!("bad number literal `{raw}`")))
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                write!(out, "\\{b:02X}").ok();
            }
        }
    }
    out
}

fn diag(msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(msg.into(), Span::dummy())
}
