//! N08.06.01: native observations for ES array literals + index access +
//! `.length` (E06.01 / `es/arrays/array_lit_access`).
//!
//! Arrays are Runtime GC heap values (`draconic_rt_array_*`). Number elements
//! are stored as `inttoptr` of integer bit-patterns; nested arrays store GC
//! ptrs; strings are cstr ptrs; booleans are `inttoptr` 0/1; `null` is null.
//! Number locals (including `.length` and index results) print via `print_f64`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    ArrayElement, Expr, IrType as Type, Local, LocalId, Module, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, GC_INIT, PRINT_F64,
};

pub(crate) fn is_es_arrays_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_arrays(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_arrays module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    Array,
    String,
    Bool,
    Null,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    number_locals: Vec<LocalId>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut slots = Vec::new();
    let mut number_locals = Vec::new();
    let mut has_array = false;
    // Array local → its array-literal init (for constant-index type inference).
    let mut arr_inits: HashMap<LocalId, Expr> = HashMap::new();

    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                let init = init.as_ref()?;
                if matches!(init, Expr::Array { .. }) {
                    if !array_expr_ok(init, &by_id) {
                        return None;
                    }
                    has_array = true;
                    slots.push((*local, SlotTy::Array));
                    arr_inits.insert(*local, init.clone());
                } else if let Expr::Local { id, .. } = init {
                    if slots.iter().any(|(s, k)| s == id && *k == SlotTy::Array) {
                        has_array = true;
                        slots.push((*local, SlotTy::Array));
                        if let Some(e) = arr_inits.get(id).cloned() {
                            arr_inits.insert(*local, e);
                        }
                    } else if slots.iter().any(|(s, k)| s == id && *k == SlotTy::Number)
                        || matches!(loc.ty, Type::Number)
                    {
                        slots.push((*local, SlotTy::Number));
                        number_locals.push(*local);
                    } else {
                        return None;
                    }
                } else if let Some(kind) = infer_expr_slot(init, &arr_inits) {
                    if !value_expr_ok(init, &by_id) {
                        return None;
                    }
                    slots.push((*local, kind));
                    if kind == SlotTy::Number {
                        number_locals.push(*local);
                    }
                } else if matches!(loc.ty, Type::Number) && number_expr_ok(init, &by_id) {
                    // Dynamic index into number array (e.g. `a[i]`).
                    slots.push((*local, SlotTy::Number));
                    number_locals.push(*local);
                } else if number_expr_ok(init, &by_id)
                    && matches!(
                        init,
                        Expr::Member {
                            computed: true,
                            ..
                        }
                    )
                {
                    // Dynamic computed index with Any type — treat as number (dyn).
                    slots.push((*local, SlotTy::Number));
                    number_locals.push(*local);
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    if !has_array || number_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        slots,
        number_locals,
    })
}

fn const_index(expr: &Expr) -> Option<usize> {
    match expr {
        Expr::Number { raw, .. } => {
            let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
            let f: f64 = cleaned.parse().ok()?;
            if f.is_finite() && f >= 0.0 && f.fract() == 0.0 {
                Some(f as usize)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn infer_expr_slot(expr: &Expr, arr_inits: &HashMap<LocalId, Expr>) -> Option<SlotTy> {
    match expr {
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Boolean { .. } => Some(SlotTy::Bool),
        Expr::Null { .. } => Some(SlotTy::Null),
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            if *optional {
                return None;
            }
            if !*computed && member_key_is_length(property) {
                return Some(SlotTy::Number);
            }
            if *computed {
                let idx = const_index(property)?;
                let elem = resolve_array_elem(object, idx, arr_inits)?;
                return literal_or_array_slot(&elem);
            }
            None
        }
        _ => None,
    }
}

fn resolve_array_elem(
    array_expr: &Expr,
    idx: usize,
    arr_inits: &HashMap<LocalId, Expr>,
) -> Option<Expr> {
    let lit = match array_expr {
        Expr::Array { .. } => array_expr.clone(),
        Expr::Local { id, .. } => arr_inits.get(id).cloned()?,
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            if *optional || !*computed {
                return None;
            }
            let outer_idx = const_index(property)?;
            let outer = resolve_array_elem(object, outer_idx, arr_inits)?;
            outer
        }
        _ => return None,
    };
    let Expr::Array { elements, .. } = lit else {
        return None;
    };
    match elements.get(idx)? {
        ArrayElement::Expr(e) => Some(e.clone()),
        ArrayElement::Elision => Some(Expr::Null { ty: Type::Null }),
        ArrayElement::Spread(_) => None,
    }
}

fn literal_or_array_slot(expr: &Expr) -> Option<SlotTy> {
    match expr {
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Boolean { .. } => Some(SlotTy::Bool),
        Expr::Null { .. } => Some(SlotTy::Null),
        Expr::Array { .. } => Some(SlotTy::Array),
        _ => None,
    }
}

fn is_array_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Object | Type::Any)
}

fn is_number_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Number | Type::Any)
}

fn array_expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => value_expr_ok(e, by_id),
            ArrayElement::Elision => true,
            ArrayElement::Spread(_) => false,
        }),
        Expr::Local { id, ty } => {
            is_array_slot_ty(ty)
                || by_id
                    .get(id)
                    .is_some_and(|l| is_array_slot_ty(&l.ty) || matches!(l.ty, Type::Any))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && array_expr_ok(object, by_id)
                && if *computed {
                    number_expr_ok(property, by_id)
                } else {
                    member_key_is_length(property)
                }
        }
        _ => false,
    }
}

fn value_expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    number_expr_ok(expr, by_id)
        || string_expr_ok(expr, by_id)
        || bool_expr_ok(expr, by_id)
        || null_expr_ok(expr, by_id)
        || array_expr_ok(expr, by_id)
}

fn number_expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            is_number_slot_ty(ty)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && array_expr_ok(object, by_id)
                && if *computed {
                    number_expr_ok(property, by_id)
                } else {
                    member_key_is_length(property)
                }
        }
        _ => false,
    }
}

fn string_expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::String { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::String)
                || by_id.get(id).is_some_and(|l| matches!(l.ty, Type::String))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional && *computed && array_expr_ok(object, by_id) && number_expr_ok(property, by_id)
        }
        _ => false,
    }
}

fn bool_expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Boolean { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::Boolean)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Boolean))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional && *computed && array_expr_ok(object, by_id) && number_expr_ok(property, by_id)
        }
        _ => false,
    }
}

fn null_expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Null { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::Null | Type::Any)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Null | Type::Any))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional && *computed && array_expr_ok(object, by_id) && number_expr_ok(property, by_id)
        }
        _ => false,
    }
}

fn member_key_is_length(property: &Expr) -> bool {
    matches!(property, Expr::String { value, .. } if value.to_string_lossy() == "length")
}

struct Emitter<'a> {
    module: &'a Module,
    out: String,
    body: String,
    allocas: HashMap<LocalId, String>,
    slot_of: HashMap<LocalId, SlotTy>,
    str_globals: Vec<(String, String)>,
    tmp: usize,
    str_n: usize,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, _info: &'a ModuleInfo) -> Self {
        Self {
            module,
            out: String::new(),
            body: String::new(),
            allocas: HashMap::new(),
            slot_of: HashMap::new(),
            str_globals: Vec::new(),
            tmp: 0,
            str_n: 0,
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

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for (id, ty) in &info.slots {
            self.slot_of.insert(*id, *ty);
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.06.01 ES arrays via Runtime ABI)"
        )
        .ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[
                GC_INIT,
                ARRAY_NEW,
                ARRAY_GET,
                ARRAY_SET,
                ARRAY_LEN,
                PRINT_F64,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for (id, kind) in &info.slots {
            match kind {
                SlotTy::Number => {
                    let g = number_global_name(*id);
                    writeln!(
                        self.out,
                        "@{g} = internal global double 0.00000000000000000e+00, align 8"
                    )
                    .ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::String | SlotTy::Bool | SlotTy::Null | SlotTy::Array => {
                    let g = ptr_global_name(*id, *kind);
                    writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
            }
        }
        if !info.slots.is_empty() {
            writeln!(self.out).ok();
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for id in &info.number_locals {
            let ptr = self.slot_ptr(*id)?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
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
        writeln!(self.out, "  {}", GC_INIT.call("")).ok();
        self.out.push_str(&self.body);
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
                let kind = *self
                    .slot_of
                    .get(local)
                    .ok_or_else(|| diag("es_arrays: declare unknown slot"))?;
                let ptr = self.slot_ptr(*local)?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Array => {
                        let v = self.emit_array_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Bool => {
                        let v = self.emit_bool_as_ptr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Null => {
                        let v = self.emit_null_as_ptr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            _ => Err(diag("es_arrays: unsupported stmt")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => format_number_const(raw),
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: number local unknown"))?;
                if kind != SlotTy::Number {
                    return Err(diag("es_arrays: expected number local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                computed,
                ..
            } => {
                if *optional {
                    return Err(diag("es_arrays: optional member not supported"));
                }
                if *computed {
                    let arr = self.emit_array_expr(object)?;
                    let idx_d = self.emit_number_expr(property)?;
                    let idx_i = self.fresh();
                    writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
                    let raw = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_GET.call_to(&raw, &format!("ptr {arr}, i64 {idx_i}"))
                    )
                    .ok();
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
                    let d = self.fresh();
                    writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                    Ok(d)
                } else if member_key_is_length(property) {
                    let arr = self.emit_array_expr(object)?;
                    let n = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&n, &format!("ptr {arr}"))
                    )
                    .ok();
                    let d = self.fresh();
                    writeln!(self.body, "  {d} = sitofp i64 {n} to double").ok();
                    Ok(d)
                } else {
                    Err(diag("es_arrays: only .length or computed index on arrays"))
                }
            }
            _ => Err(diag("es_arrays: unsupported number expr")),
        }
    }

    fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Array { elements, .. } => self.emit_array_lit(elements),
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: array local unknown"))?;
                if kind != SlotTy::Array {
                    return Err(diag("es_arrays: expected array local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                computed,
                ..
            } => {
                if *optional {
                    return Err(diag("es_arrays: optional member not supported"));
                }
                if !*computed {
                    return Err(diag("es_arrays: nested array only via computed index"));
                }
                let arr = self.emit_array_expr(object)?;
                let idx_d = self.emit_number_expr(property)?;
                let idx_i = self.fresh();
                writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_GET.call_to(&t, &format!("ptr {arr}, i64 {idx_i}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported array expr")),
        }
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
                ArrayElement::Elision => {}
                ArrayElement::Spread(_) => {
                    return Err(diag("es_arrays: spread not supported in this item"));
                }
                ArrayElement::Expr(e) => {
                    let v = self.emit_value_as_ptr(e)?;
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {arr}, i64 {i}, ptr {v}"))
                    )
                    .ok();
                }
            }
        }
        Ok(arr)
    }

    fn emit_value_as_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        if matches!(expr, Expr::Array { .. }) || self.expr_is_array_slot(expr) {
            return self.emit_array_expr(expr);
        }
        if matches!(expr, Expr::String { .. }) || self.expr_is_string_slot(expr) {
            return self.emit_string_expr(expr);
        }
        if matches!(expr, Expr::Boolean { .. }) || self.expr_is_bool_slot(expr) {
            return self.emit_bool_as_ptr(expr);
        }
        if matches!(expr, Expr::Null { .. }) || self.expr_is_null_slot(expr) {
            return self.emit_null_as_ptr(expr);
        }
        let n = self.emit_number_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
        let p = self.fresh();
        writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
        Ok(p)
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: string local unknown"))?;
                if kind != SlotTy::String {
                    return Err(diag("es_arrays: expected string local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                computed,
                ..
            } => {
                if *optional || !*computed {
                    return Err(diag("es_arrays: string element only via computed index"));
                }
                let arr = self.emit_array_expr(object)?;
                let idx_d = self.emit_number_expr(property)?;
                let idx_i = self.fresh();
                writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_GET.call_to(&t, &format!("ptr {arr}, i64 {idx_i}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported string expr")),
        }
    }

    fn emit_bool_as_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Boolean { value, .. } => {
                let n = if *value { 1i64 } else { 0i64 };
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: bool local unknown"))?;
                if kind != SlotTy::Bool {
                    return Err(diag("es_arrays: expected bool local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                computed,
                ..
            } => {
                if *optional || !*computed {
                    return Err(diag("es_arrays: bool element only via computed index"));
                }
                let arr = self.emit_array_expr(object)?;
                let idx_d = self.emit_number_expr(property)?;
                let idx_i = self.fresh();
                writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_GET.call_to(&t, &format!("ptr {arr}, i64 {idx_i}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported bool expr")),
        }
    }

    fn emit_null_as_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Null { .. } => {
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: null local unknown"))?;
                if kind != SlotTy::Null {
                    return Err(diag("es_arrays: expected null local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                computed,
                ..
            } => {
                if *optional || !*computed {
                    return Err(diag("es_arrays: null element only via computed index"));
                }
                let arr = self.emit_array_expr(object)?;
                let idx_d = self.emit_number_expr(property)?;
                let idx_i = self.fresh();
                writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_GET.call_to(&t, &format!("ptr {arr}, i64 {idx_i}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported null expr")),
        }
    }

    fn expr_is_array_slot(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id) == Some(&SlotTy::Array),
            Expr::Member {
                ty, computed: true, ..
            } => matches!(ty, Type::Object | Type::Any),
            _ => false,
        }
    }

    fn expr_is_string_slot(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id) == Some(&SlotTy::String),
            Expr::Member { ty, .. } => matches!(ty, Type::String),
            _ => false,
        }
    }

    fn expr_is_bool_slot(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id) == Some(&SlotTy::Bool),
            Expr::Member { ty, .. } => matches!(ty, Type::Boolean),
            _ => false,
        }
    }

    fn expr_is_null_slot(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id) == Some(&SlotTy::Null),
            Expr::Member { ty, .. } => matches!(ty, Type::Null | Type::Any),
            _ => false,
        }
    }

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        self.allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("es_arrays: slot missing"))
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_arr_str.{}", self.str_n);
            self.str_n += 1;
            self.str_globals.push((s.to_string(), g.clone()));
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

fn number_global_name(id: LocalId) -> String {
    format!("es_arr_n{}", id.0)
}

fn ptr_global_name(id: LocalId, kind: SlotTy) -> String {
    let tag = match kind {
        SlotTy::Array => "a",
        SlotTy::String => "s",
        SlotTy::Bool => "b",
        SlotTy::Null => "z",
        SlotTy::Number => "n",
    };
    format!("es_arr_{tag}{}", id.0)
}

fn format_number_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(format!("invalid number literal {raw}")))?;
    Ok(format!("{f:.17e}"))
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

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
