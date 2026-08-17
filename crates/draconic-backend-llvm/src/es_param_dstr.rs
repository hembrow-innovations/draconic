//! N08.16.25: native observations for parameter destructuring (E18.25 /
//! `es/annex-b/param_destructure`).
//!
//! Functions/arrows with object or array binding patterns as formals; nested
//! patterns, rename, rest, element/property defaults, and whole-param defaults.
//! Values: numbers as double (heap as inttoptr); objects/arrays via Runtime GC.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::BinaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, ArrayPatternEl, Expr, Local, LocalId, Module, ObjectPatternEl,
    ObjectProp, ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, GC_INIT, OBJECT_GET,
    OBJECT_REST, OBJECT_SET, PRINT_F64,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    Object,
    Array,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ArgKind {
    Number,
    Ptr,
}

#[derive(Clone)]
struct FnInfo {
    idx: usize,
    params: Vec<Param>,
    arg_kinds: Vec<ArgKind>,
    body: Vec<Stmt>,
}

struct ModuleInfo {
    functions: Vec<FnInfo>,
    fn_binding: HashMap<LocalId, usize>,
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<LocalId>,
}

pub(crate) fn is_es_param_dstr_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_param_dstr(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not es_param_dstr"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut fn_binding = HashMap::new();
    let mut has_pattern_param = false;

    collect_fns(
        &module.body,
        &by_id,
        &mut functions,
        &mut fn_binding,
        &mut has_pattern_param,
    )?;
    if !has_pattern_param {
        return None;
    }

    let mut slots = Vec::new();
    let mut print_locals = Vec::new();
    let mut slot_of = HashMap::new();

    for f in &functions {
        for p in &f.params {
            register_pattern_slots(&p.pattern, &mut slots, &mut slot_of)?;
        }
        body_slots_ok(&f.body, &by_id, &fn_binding, &slot_of)?;
    }

    for stmt in &module.body {
        match stmt {
            Stmt::Function { .. } => {}
            Stmt::Declare { local, init, .. } => {
                if fn_binding.contains_key(local) {
                    continue;
                }
                let Some(init) = init else {
                    return None;
                };
                if matches!(init, Expr::Function { .. }) {
                    continue;
                }
                let ty = top_init_ty(init, &by_id, &fn_binding, &slot_of)?;
                if !slot_of.contains_key(local) {
                    slots.push((*local, ty));
                    slot_of.insert(*local, ty);
                }
                if ty == SlotTy::Number && !print_locals.contains(local) {
                    print_locals.push(*local);
                }
            }
            _ => return None,
        }
    }

    if print_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        functions,
        fn_binding,
        slots,
        print_locals,
    })
}

fn collect_fns(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    functions: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
    has_pattern: &mut bool,
) -> Option<()> {
    for stmt in body {
        match stmt {
            Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
                ..
            } => {
                if *is_async || *is_generator {
                    return None;
                }
                push_fn(
                    Some(*local),
                    params,
                    body,
                    by_id,
                    functions,
                    fn_binding,
                    has_pattern,
                )?;
            }
            Stmt::Declare {
                local,
                init: Some(Expr::Function {
                    params,
                    body,
                    is_async,
                    is_generator,
                    ..
                }),
                ..
            } => {
                if *is_async || *is_generator {
                    return None;
                }
                push_fn(
                    Some(*local),
                    params,
                    body,
                    by_id,
                    functions,
                    fn_binding,
                    has_pattern,
                )?;
            }
            _ => {}
        }
    }
    Some(())
}

fn push_fn(
    bind: Option<LocalId>,
    params: &[Param],
    body: &[Stmt],
    _by_id: &HashMap<LocalId, &Local>,
    functions: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
    has_pattern: &mut bool,
) -> Option<()> {
    let mut arg_kinds = Vec::with_capacity(params.len());
    for p in params {
        if p.rest {
            return None;
        }
        match &p.pattern {
            Pattern::Local(_) => arg_kinds.push(ArgKind::Number),
            Pattern::Object(_) | Pattern::Array(_) => {
                *has_pattern = true;
                arg_kinds.push(ArgKind::Ptr);
            }
            _ => return None,
        }
        if let Some(d) = &p.default {
            match d {
                Expr::Object { .. } | Expr::Array { .. } => {}
                _ => return None,
            }
        }
    }
    let idx = functions.len();
    if let Some(id) = bind {
        fn_binding.insert(id, idx);
    }
    functions.push(FnInfo {
        idx,
        params: params.to_vec(),
        arg_kinds,
        body: body.to_vec(),
    });
    Some(())
}

fn register_pattern_slots(
    pat: &Pattern,
    slots: &mut Vec<(LocalId, SlotTy)>,
    slot_of: &mut HashMap<LocalId, SlotTy>,
) -> Option<()> {
    match pat {
        Pattern::Local(id) => {
            if !slot_of.contains_key(id) {
                slots.push((*id, SlotTy::Number));
                slot_of.insert(*id, SlotTy::Number);
            }
            Some(())
        }
        Pattern::Object(props) => {
            for el in props {
                match el {
                    ObjectPatternEl::Prop { binding, default, .. } => {
                        if let Some(d) = default {
                            if !matches!(d, Expr::Number { .. }) {
                                return None;
                            }
                        }
                        register_pattern_slots(binding, slots, slot_of)?;
                    }
                    ObjectPatternEl::Rest(binding) => match binding {
                        Pattern::Local(id) => {
                            if !slot_of.contains_key(id) {
                                slots.push((*id, SlotTy::Object));
                                slot_of.insert(*id, SlotTy::Object);
                            }
                        }
                        other => register_pattern_slots(other, slots, slot_of)?,
                    },
                }
            }
            Some(())
        }
        Pattern::Array(els) => {
            for el in els {
                match el {
                    ArrayPatternEl::Elision => {}
                    ArrayPatternEl::Pattern { binding, default } => {
                        if let Some(d) = default {
                            if !matches!(d, Expr::Number { .. }) {
                                return None;
                            }
                        }
                        register_pattern_slots(binding, slots, slot_of)?;
                    }
                    ArrayPatternEl::Rest(binding) => match binding {
                        Pattern::Local(id) => {
                            if !slot_of.contains_key(id) {
                                slots.push((*id, SlotTy::Array));
                                slot_of.insert(*id, SlotTy::Array);
                            }
                        }
                        Pattern::Object(_) | Pattern::Array(_) => {
                            register_pattern_slots(binding, slots, slot_of)?;
                        }
                        _ => return None,
                    },
                }
            }
            Some(())
        }
        _ => None,
    }
}

fn body_slots_ok(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<()> {
    for s in body {
        match s {
            Stmt::Return { value: Some(e) } => {
                if !number_expr_ok(e, by_id, fn_binding, slot_of) {
                    return None;
                }
            }
            Stmt::Return { value: None } => {}
            _ => return None,
        }
    }
    Some(())
}

fn top_init_ty(
    init: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<SlotTy> {
    match init {
        Expr::Object { .. } => Some(SlotTy::Object),
        Expr::Array { .. } => Some(SlotTy::Array),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return None;
            }
            let Expr::Local { id, .. } = callee.as_ref() else {
                return None;
            };
            if !fn_binding.contains_key(id) {
                return None;
            }
            for a in args {
                match a {
                    Arg::Expr(e) => {
                        if !value_expr_ok(e, by_id, fn_binding, slot_of) {
                            return None;
                        }
                    }
                    Arg::Spread(_) => return None,
                }
            }
            Some(SlotTy::Number)
        }
        Expr::Local { id, .. } => slot_of.get(id).copied(),
        _ => None,
    }
}

fn value_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Object { properties, .. } => properties.iter().all(|p| match p {
            ObjectProp::Property { key, value } => {
                matches!(key, ObjectPropKey::Static(_)) && value_expr_ok(value, by_id, fn_binding, slot_of)
            }
            _ => false,
        }),
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => value_expr_ok(e, by_id, fn_binding, slot_of),
            ArrayElement::Elision => true,
            ArrayElement::Spread(_) => false,
        }),
        Expr::Number { .. } => true,
        Expr::Local { id, .. } => {
            slot_of.contains_key(id) || fn_binding.contains_key(id) || is_undef(*id, by_id)
        }
        _ => false,
    }
}

fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, .. } => {
            slot_of.get(id) == Some(&SlotTy::Number) || is_undef(*id, by_id)
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
            ) && number_expr_ok(left, by_id, fn_binding, slot_of)
                && number_expr_ok(right, by_id, fn_binding, slot_of)
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            if *optional {
                return false;
            }
            let Expr::Local { id, .. } = object.as_ref() else {
                return false;
            };
            matches!(
                slot_of.get(id),
                Some(SlotTy::Object) | Some(SlotTy::Array)
            ) && if *computed {
                matches!(property.as_ref(), Expr::Number { .. })
            } else {
                matches!(property.as_ref(), Expr::String { .. })
            }
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && matches!(callee.as_ref(), Expr::Local { id, .. } if fn_binding.contains_key(id))
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => value_expr_ok(e, by_id, fn_binding, slot_of),
                    Arg::Spread(_) => false,
                })
        }
        _ => false,
    }
}

fn is_undef(id: LocalId, by_id: &HashMap<LocalId, &Local>) -> bool {
    by_id.get(&id).is_some_and(|l| l.name == "undefined")
}

fn diag(msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    out: String,
    body: String,
    tmp: usize,
    label: usize,
    str_n: usize,
    str_globals: String,
    allocas: HashMap<LocalId, String>,
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
            tmp: 0,
            label: 0,
            str_n: 0,
            str_globals: String::new(),
            allocas: HashMap::new(),
            slot_of,
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("%t{t}")
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label;
        self.label += 1;
        format!("{prefix}{n}")
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.25 param destructuring)"
        )
        .ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[
                GC_INIT,
                ALLOC_OBJECT,
                OBJECT_SET,
                OBJECT_GET,
                OBJECT_REST,
                ARRAY_NEW,
                ARRAY_LEN,
                ARRAY_GET,
                ARRAY_SET,
                PRINT_F64,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for f in &self.info.functions.clone() {
            self.emit_fn(f)?;
        }

        // Main body first (collects more string globals), then flush strings, then main.
        self.body.clear();
        for (id, ty) in &self.info.slots.clone() {
            if self.info.fn_binding.contains_key(id) {
                continue;
            }
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            match ty {
                SlotTy::Number => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                    writeln!(
                        self.body,
                        "  store double 0.00000000000000000e+00, ptr {ptr}"
                    )
                    .ok();
                }
                SlotTy::Object | SlotTy::Array => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                }
            }
        }
        for stmt in &self.module.body.clone() {
            self.emit_top_stmt(stmt)?;
        }
        for id in &self.info.print_locals.clone() {
            let ptr = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("es_param_dstr: print missing alloca"))?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
        }

        write!(self.out, "{}", self.str_globals).ok();
        if !self.str_globals.is_empty() {
            writeln!(self.out).ok();
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        writeln!(self.out, "  {}", GC_INIT.call("")).ok();
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_fn(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let name = format!("d_fn_{}", f.idx);
        let mut sig = String::new();
        for (i, k) in f.arg_kinds.iter().enumerate() {
            if i > 0 {
                sig.push_str(", ");
            }
            match k {
                ArgKind::Number => write!(sig, "double %a{i}").ok(),
                ArgKind::Ptr => write!(sig, "ptr %a{i}").ok(),
            };
        }
        writeln!(self.out, "define double @{name}({sig}) {{").ok();
        writeln!(self.out, "entry:").ok();

        let saved_body = std::mem::take(&mut self.body);
        let saved_allocas = std::mem::take(&mut self.allocas);
        let saved_tmp = self.tmp;
        let saved_label = self.label;
        self.tmp = 0;
        self.label = 0;

        // Allocas for all pattern-bound locals + simple params.
        let mut bound = Vec::new();
        for p in &f.params {
            collect_bound_locals(&p.pattern, &mut bound);
        }
        for id in &bound {
            let ty = *self
                .slot_of
                .get(id)
                .unwrap_or(&SlotTy::Number);
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            match ty {
                SlotTy::Number => {
                    writeln!(self.out, "  {ptr} = alloca double, align 8").ok();
                    writeln!(
                        self.out,
                        "  store double 0.00000000000000000e+00, ptr {ptr}"
                    )
                    .ok();
                }
                SlotTy::Object | SlotTy::Array => {
                    writeln!(self.out, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.out, "  store ptr null, ptr {ptr}").ok();
                }
            }
        }

        // Bind each formal (body buffer), then returns.
        for (i, p) in f.params.iter().enumerate() {
            match f.arg_kinds[i] {
                ArgKind::Number => {
                    let Pattern::Local(id) = &p.pattern else {
                        return Err(diag("es_param_dstr: number arg non-local"));
                    };
                    let ptr = self.allocas.get(id).cloned().unwrap();
                    writeln!(self.body, "  store double %a{i}, ptr {ptr}").ok();
                }
                ArgKind::Ptr => {
                    let mut src = format!("%a{i}");
                    if let Some(def) = &p.default {
                        src = self.emit_ptr_default(&src, def)?;
                    }
                    match &p.pattern {
                        Pattern::Object(props) => self.emit_object_pattern(props, &src)?,
                        Pattern::Array(els) => self.emit_array_pattern(els, &src)?,
                        _ => return Err(diag("es_param_dstr: ptr arg pattern")),
                    }
                }
            }
        }

        for stmt in &f.body {
            match stmt {
                Stmt::Return { value: Some(e) } => {
                    let v = self.emit_number_expr(e)?;
                    writeln!(self.body, "  ret double {v}").ok();
                }
                Stmt::Return { value: None } => {
                    writeln!(
                        self.body,
                        "  ret double 0.00000000000000000e+00"
                    )
                    .ok();
                }
                _ => return Err(diag("es_param_dstr: unsupported fn stmt")),
            }
        }
        if !self.body_ends_ret() {
            writeln!(
                self.body,
                "  ret double 0.00000000000000000e+00"
            )
            .ok();
        }
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.allocas = saved_allocas;
        self.tmp = saved_tmp;
        self.label = saved_label;
        Ok(())
    }

    fn body_ends_ret(&self) -> bool {
        self.body
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .is_some_and(|l| l.trim().starts_with("ret "))
    }

    fn emit_ptr_default(&mut self, arg: &str, def: &Expr) -> Result<String, Diagnostic> {
        // if arg == null → use default object/array
        let is_null = self.fresh();
        writeln!(self.body, "  {is_null} = icmp eq ptr {arg}, null").ok();
        let then_l = self.fresh_label("pdef");
        let end_l = self.fresh_label("pdefend");
        let slot = self.fresh();
        writeln!(self.body, "  {slot} = alloca ptr, align 8").ok();
        writeln!(self.body, "  store ptr {arg}, ptr {slot}").ok();
        writeln!(
            self.body,
            "  br i1 {is_null}, label %{then_l}, label %{end_l}"
        )
        .ok();
        writeln!(self.body, "{then_l}:").ok();
        let d = self.emit_value_ptr(def)?;
        writeln!(self.body, "  store ptr {d}, ptr {slot}").ok();
        writeln!(self.body, "  br label %{end_l}").ok();
        writeln!(self.body, "{end_l}:").ok();
        let out = self.fresh();
        writeln!(self.body, "  {out} = load ptr, ptr {slot}").ok();
        Ok(out)
    }

    fn emit_top_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Function { .. } => Ok(()),
            Stmt::Declare { local, init, .. } => {
                if self.info.fn_binding.contains_key(local) {
                    return Ok(());
                }
                let init = init
                    .as_ref()
                    .ok_or_else(|| diag("es_param_dstr: declare needs init"))?;
                if matches!(init, Expr::Function { .. }) {
                    return Ok(());
                }
                let ty = *self
                    .slot_of
                    .get(local)
                    .ok_or_else(|| diag("es_param_dstr: unknown slot"))?;
                let ptr = self
                    .allocas
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("es_param_dstr: missing alloca"))?;
                match ty {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Object | SlotTy::Array => {
                        let v = self.emit_value_ptr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            _ => Err(diag("es_param_dstr: unsupported top stmt")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n: f64 = raw.parse().unwrap_or(0.0);
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {t} = fadd double 0.00000000000000000e+00, {n:.17e}"
                )
                .ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_param_dstr: number local missing"))?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let inst = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    BinaryOp::Rem => "frem",
                    _ => return Err(diag("es_param_dstr: bad binary")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                computed,
                ..
            } => {
                let obj = self.emit_value_ptr(object)?;
                let val = if *computed {
                    // array index
                    let Expr::Number { raw, .. } = property.as_ref() else {
                        return Err(diag("es_param_dstr: computed index must be number lit"));
                    };
                    let idx: u64 = raw.parse().unwrap_or(0);
                    let got = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_GET.call_to(&got, &format!("ptr {obj}, i64 {idx}"))
                    )
                    .ok();
                    got
                } else {
                    let Expr::String { value, .. } = property.as_ref() else {
                        return Err(diag("es_param_dstr: member key must be string"));
                    };
                    let key = self.string_const(&value.to_string_lossy())?;
                    let got = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_GET.call_to(&got, &format!("ptr {obj}, ptr {key}"))
                    )
                    .ok();
                    got
                };
                let i = self.fresh();
                writeln!(self.body, "  {i} = ptrtoint ptr {val} to i64").ok();
                let d = self.fresh();
                writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                Ok(d)
            }
            Expr::Call {
                callee, args, ..
            } => self.emit_call(callee, args),
            _ => Err(diag("es_param_dstr: unsupported number expr")),
        }
    }

    fn emit_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_param_dstr: call callee local"));
        };
        let idx = *self
            .info
            .fn_binding
            .get(id)
            .ok_or_else(|| diag("es_param_dstr: unknown fn"))?;
        let f = &self.info.functions[idx];
        let mut parts = Vec::new();
        for (i, k) in f.arg_kinds.iter().enumerate() {
            let arg_expr = args.get(i).and_then(|a| match a {
                Arg::Expr(e) => Some(e),
                Arg::Spread(_) => None,
            });
            match k {
                ArgKind::Number => {
                    let v = if let Some(e) = arg_expr {
                        self.emit_number_expr(e)?
                    } else {
                        "0.00000000000000000e+00".into()
                    };
                    parts.push(format!("double {v}"));
                }
                ArgKind::Ptr => {
                    let v = if let Some(e) = arg_expr {
                        self.emit_value_ptr(e)?
                    } else {
                        "null".into()
                    };
                    parts.push(format!("ptr {v}"));
                }
            }
        }
        let mut ty = String::new();
        for (i, k) in f.arg_kinds.iter().enumerate() {
            if i > 0 {
                ty.push_str(", ");
            }
            match k {
                ArgKind::Number => ty.push_str("double"),
                ArgKind::Ptr => ty.push_str("ptr"),
            }
        }
        let ret = self.fresh();
        let args_s = parts.join(", ");
        writeln!(
            self.body,
            "  {ret} = call double ({ty}) @d_fn_{idx}({args_s})"
        )
        .ok();
        Ok(ret)
    }

    fn emit_value_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => {
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_param_dstr: ptr local missing"))?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Object { properties, .. } => self.emit_object_lit(properties),
            Expr::Array { elements, .. } => self.emit_array_lit(elements),
            Expr::Number { raw, .. } => {
                let n: i64 = raw.parse::<f64>().unwrap_or(0.0) as i64;
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                Ok(t)
            }
            _ => Err(diag("es_param_dstr: unsupported value ptr")),
        }
    }

    fn emit_object_lit(&mut self, properties: &[ObjectProp]) -> Result<String, Diagnostic> {
        let obj = self.fresh();
        writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
        for p in properties {
            let ObjectProp::Property { key, value } = p else {
                return Err(diag("es_param_dstr: only plain props"));
            };
            let ObjectPropKey::Static(k) = key else {
                return Err(diag("es_param_dstr: static keys only"));
            };
            let key_p = self.string_const(&k.to_string_lossy())?;
            let val = self.emit_heap_number_or_ptr(value)?;
            writeln!(
                self.body,
                "  {}",
                OBJECT_SET.call(&format!("ptr {obj}, ptr {key_p}, ptr {val}"))
            )
            .ok();
        }
        Ok(obj)
    }

    fn emit_array_lit(&mut self, elements: &[ArrayElement]) -> Result<String, Diagnostic> {
        let n = elements.len();
        let arr = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_NEW.call_to(&arr, &format!("i64 {n}"))
        )
        .ok();
        for (i, el) in elements.iter().enumerate() {
            match el {
                ArrayElement::Expr(e) => {
                    let v = self.emit_heap_number_or_ptr(e)?;
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {arr}, i64 {i}, ptr {v}"))
                    )
                    .ok();
                }
                ArrayElement::Elision => {}
                ArrayElement::Spread(_) => {
                    return Err(diag("es_param_dstr: array spread lit"));
                }
            }
        }
        Ok(arr)
    }

    fn emit_heap_number_or_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { .. } => {
                let d = self.emit_number_expr(expr)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = fptosi double {d} to i64").ok();
                let p = self.fresh();
                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                Ok(p)
            }
            Expr::Object { .. } | Expr::Array { .. } | Expr::Local { .. } => {
                self.emit_value_ptr(expr)
            }
            _ => Err(diag("es_param_dstr: heap value")),
        }
    }

    fn emit_object_pattern(
        &mut self,
        props: &[ObjectPatternEl],
        src: &str,
    ) -> Result<(), Diagnostic> {
        let mut excluded: Vec<String> = Vec::new();
        for el in props {
            match el {
                ObjectPatternEl::Prop {
                    key,
                    binding,
                    default,
                    ..
                } => {
                    let key_s = match key {
                        ObjectPropKey::Static(s) => s.to_string_lossy().to_string(),
                        ObjectPropKey::Computed(_) => {
                            return Err(diag("es_param_dstr: computed pattern key"));
                        }
                    };
                    excluded.push(key_s.clone());
                    let key_p = self.string_const(&key_s)?;
                    let got = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_GET.call_to(&got, &format!("ptr {src}, ptr {key_p}"))
                    )
                    .ok();
                    let val = if let Some(def) = default {
                        self.emit_default_if_null(&got, def)?
                    } else {
                        got
                    };
                    self.emit_bind_pattern(binding, &val)?;
                }
                ObjectPatternEl::Rest(binding) => {
                    // Build exclude list global and call object_rest.
                    let rest = self.emit_object_rest(src, &excluded)?;
                    self.emit_bind_pattern(binding, &rest)?;
                }
            }
        }
        Ok(())
    }

    fn emit_object_rest(
        &mut self,
        src: &str,
        excluded: &[String],
    ) -> Result<String, Diagnostic> {
        // @exN = global [k+1 x ptr] [ptr @str…, …, ptr null]
        let n = excluded.len() + 1;
        let gname = format!("ex{}", self.str_n);
        self.str_n += 1;
        let mut inits = Vec::new();
        for k in excluded {
            let p = self.string_const(k)?;
            // string_const returns @.sN — use as ptr
            inits.push(format!("ptr {p}"));
        }
        inits.push("ptr null".into());
        writeln!(
            self.str_globals,
            "@{gname} = private unnamed_addr constant [{n} x ptr] [{}], align 8",
            inits.join(", ")
        )
        .ok();
        let list = self.fresh();
        writeln!(
            self.body,
            "  {list} = getelementptr inbounds [{n} x ptr], ptr @{gname}, i64 0, i64 0"
        )
        .ok();
        let out = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_REST.call_to(&out, &format!("ptr {src}, ptr {list}"))
        )
        .ok();
        Ok(out)
    }

    fn emit_array_pattern(
        &mut self,
        elements: &[ArrayPatternEl],
        src: &str,
    ) -> Result<(), Diagnostic> {
        let idx_ptr = self.fresh();
        writeln!(self.body, "  {idx_ptr} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {idx_ptr}").ok();
        for el in elements {
            match el {
                ArrayPatternEl::Elision => {
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = load i64, ptr {idx_ptr}").ok();
                    let n = self.fresh();
                    writeln!(self.body, "  {n} = add i64 {i}, 1").ok();
                    writeln!(self.body, "  store i64 {n}, ptr {idx_ptr}").ok();
                }
                ArrayPatternEl::Pattern { binding, default } => {
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = load i64, ptr {idx_ptr}").ok();
                    let got = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_GET.call_to(&got, &format!("ptr {src}, i64 {i}"))
                    )
                    .ok();
                    let val = if let Some(def) = default {
                        self.emit_default_if_null(&got, def)?
                    } else {
                        got
                    };
                    self.emit_bind_pattern(binding, &val)?;
                    let i2 = self.fresh();
                    writeln!(self.body, "  {i2} = load i64, ptr {idx_ptr}").ok();
                    let n = self.fresh();
                    writeln!(self.body, "  {n} = add i64 {i2}, 1").ok();
                    writeln!(self.body, "  store i64 {n}, ptr {idx_ptr}").ok();
                }
                ArrayPatternEl::Rest(binding) => {
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = load i64, ptr {idx_ptr}").ok();
                    let len = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&len, &format!("ptr {src}"))
                    )
                    .ok();
                    let ge = self.fresh();
                    writeln!(self.body, "  {ge} = icmp uge i64 {len}, {i}").ok();
                    let diff = self.fresh();
                    writeln!(self.body, "  {diff} = sub i64 {len}, {i}").ok();
                    let rest_len = self.fresh();
                    writeln!(
                        self.body,
                        "  {rest_len} = select i1 {ge}, i64 {diff}, i64 0"
                    )
                    .ok();
                    let rest = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_NEW.call_to(&rest, &format!("i64 {rest_len}"))
                    )
                    .ok();
                    let j_ptr = self.fresh();
                    writeln!(self.body, "  {j_ptr} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store i64 0, ptr {j_ptr}").ok();
                    let head = self.fresh_label("arest_h");
                    let bod = self.fresh_label("arest_b");
                    let end = self.fresh_label("arest_e");
                    writeln!(self.body, "  br label %{head}").ok();
                    writeln!(self.body, "{head}:").ok();
                    let j = self.fresh();
                    writeln!(self.body, "  {j} = load i64, ptr {j_ptr}").ok();
                    let cmp = self.fresh();
                    writeln!(self.body, "  {cmp} = icmp ult i64 {j}, {rest_len}").ok();
                    writeln!(self.body, "  br i1 {cmp}, label %{bod}, label %{end}").ok();
                    writeln!(self.body, "{bod}:").ok();
                    let src_i = self.fresh();
                    writeln!(self.body, "  {src_i} = add i64 {i}, {j}").ok();
                    let elv = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_GET.call_to(&elv, &format!("ptr {src}, i64 {src_i}"))
                    )
                    .ok();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {rest}, i64 {j}, ptr {elv}"))
                    )
                    .ok();
                    let jn = self.fresh();
                    writeln!(self.body, "  {jn} = add i64 {j}, 1").ok();
                    writeln!(self.body, "  store i64 {jn}, ptr {j_ptr}").ok();
                    writeln!(self.body, "  br label %{head}").ok();
                    writeln!(self.body, "{end}:").ok();
                    self.emit_bind_pattern(binding, &rest)?;
                    writeln!(self.body, "  store i64 {len}, ptr {idx_ptr}").ok();
                }
            }
        }
        Ok(())
    }

    fn emit_default_if_null(
        &mut self,
        got: &str,
        def: &Expr,
    ) -> Result<String, Diagnostic> {
        let is_null = self.fresh();
        writeln!(self.body, "  {is_null} = icmp eq ptr {got}, null").ok();
        let then_l = self.fresh_label("dfl");
        let end_l = self.fresh_label("dflend");
        let slot = self.fresh();
        writeln!(self.body, "  {slot} = alloca ptr, align 8").ok();
        writeln!(self.body, "  store ptr {got}, ptr {slot}").ok();
        writeln!(
            self.body,
            "  br i1 {is_null}, label %{then_l}, label %{end_l}"
        )
        .ok();
        writeln!(self.body, "{then_l}:").ok();
        let d = self.emit_heap_number_or_ptr(def)?;
        writeln!(self.body, "  store ptr {d}, ptr {slot}").ok();
        writeln!(self.body, "  br label %{end_l}").ok();
        writeln!(self.body, "{end_l}:").ok();
        let out = self.fresh();
        writeln!(self.body, "  {out} = load ptr, ptr {slot}").ok();
        Ok(out)
    }

    fn emit_bind_pattern(&mut self, binding: &Pattern, val: &str) -> Result<(), Diagnostic> {
        match binding {
            Pattern::Local(id) => {
                let ty = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_param_dstr: bind unknown"))?;
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_param_dstr: bind alloca"))?;
                match ty {
                    SlotTy::Number => {
                        let i = self.fresh();
                        writeln!(self.body, "  {i} = ptrtoint ptr {val} to i64").ok();
                        let d = self.fresh();
                        writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                        writeln!(self.body, "  store double {d}, ptr {ptr}").ok();
                    }
                    SlotTy::Object | SlotTy::Array => {
                        writeln!(self.body, "  store ptr {val}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Pattern::Object(props) => self.emit_object_pattern(props, val),
            Pattern::Array(els) => self.emit_array_pattern(els, val),
            _ => Err(diag("es_param_dstr: bind pattern")),
        }
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let n = self.str_n;
        self.str_n += 1;
        let name = format!(".s{n}");
        let mut bytes = s.as_bytes().to_vec();
        bytes.push(0);
        let len = bytes.len();
        let mut esc = String::new();
        for b in bytes {
            use std::fmt::Write;
            write!(esc, "\\{b:02X}").ok();
        }
        writeln!(
            self.str_globals,
            "@{name} = private unnamed_addr constant [{len} x i8] c\"{esc}\", align 1"
        )
        .ok();
        Ok(format!("@{name}"))
    }
}

fn collect_bound_locals(pat: &Pattern, out: &mut Vec<LocalId>) {
    match pat {
        Pattern::Local(id) => {
            if !out.contains(id) {
                out.push(*id);
            }
        }
        Pattern::Object(props) => {
            for el in props {
                match el {
                    ObjectPatternEl::Prop { binding, .. } | ObjectPatternEl::Rest(binding) => {
                        collect_bound_locals(binding, out);
                    }
                }
            }
        }
        Pattern::Array(els) => {
            for el in els {
                match el {
                    ArrayPatternEl::Elision => {}
                    ArrayPatternEl::Pattern { binding, .. } | ArrayPatternEl::Rest(binding) => {
                        collect_bound_locals(binding, out);
                    }
                }
            }
        }
        _ => {}
    }
}
