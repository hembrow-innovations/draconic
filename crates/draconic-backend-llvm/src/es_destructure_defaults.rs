//! N08.16.20: native observations for ES destructuring defaults
//! (`es/annex-b/destructure_defaults` / E18.20).
//!
//! Array + object patterns with defaults (declare and assignment), nested object
//! defaults, rename. Heap values via Runtime array/object ABI; number locals via
//! `print_f64`. Missing / hole / undefined → null ptr → default fires.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    ArrayElement, ArrayPatternEl, AssignTarget, Expr, IrType as Type, Local, LocalId, Module,
    ObjectPatternEl, ObjectProp, ObjectPropKey, Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, GC_INIT, OBJECT_GET,
    OBJECT_SET, PRINT_F64,
};

pub(crate) fn is_es_destructure_defaults_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_destructure_defaults(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not destructure_defaults"))?;
    let mut em = Emitter::new(module);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    Array,
    Object,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<LocalId>,
}

struct ClassifyCtx<'a> {
    by_id: &'a HashMap<LocalId, &'a Local>,
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<LocalId>,
    has_array_pat: bool,
    has_object_pat: bool,
    has_default: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut ctx = ClassifyCtx {
        by_id: &by_id,
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_array_pat: false,
        has_object_pat: false,
        has_default: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_array_pat || !ctx.has_object_pat || !ctx.has_default || ctx.print_locals.is_empty()
    {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => classify_declare(*local, init.as_ref(), ctx),
        Stmt::DeclareArrayPattern {
            elements,
            init: Some(init),
            ..
        } => {
            if !array_expr_ok(init, &ctx.slot_of, ctx.by_id) {
                return None;
            }
            ctx.has_array_pat = true;
            classify_array_pattern(elements, ctx)
        }
        Stmt::DeclareObjectPattern {
            properties,
            init: Some(init),
            ..
        } => {
            if !object_expr_ok(init, &ctx.slot_of, ctx.by_id) {
                return None;
            }
            ctx.has_object_pat = true;
            classify_object_pattern(properties, ctx)
        }
        Stmt::Expr { expr } => match expr {
            Expr::Assign {
                target: AssignTarget::ArrayPattern { elements },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                if !array_expr_ok(value, &ctx.slot_of, ctx.by_id) {
                    return None;
                }
                ctx.has_array_pat = true;
                classify_array_pattern(elements, ctx)
            }
            Expr::Assign {
                target: AssignTarget::ObjectPattern { properties },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                if !object_expr_ok(value, &ctx.slot_of, ctx.by_id) {
                    return None;
                }
                ctx.has_object_pat = true;
                classify_object_pattern(properties, ctx)
            }
            _ => None,
        },
        _ => None,
    }
}

fn classify_declare(
    local: LocalId,
    init: Option<&Expr>,
    ctx: &mut ClassifyCtx<'_>,
) -> Option<()> {
    let Some(init) = init else {
        register_number(local, ctx);
        return Some(());
    };
    if matches!(init, Expr::Array { .. }) {
        if !array_expr_ok(init, &ctx.slot_of, ctx.by_id) {
            return None;
        }
        register_slot(local, SlotTy::Array, ctx);
        return Some(());
    }
    if matches!(init, Expr::Object { .. }) {
        if !object_expr_ok(init, &ctx.slot_of, ctx.by_id) {
            return None;
        }
        register_slot(local, SlotTy::Object, ctx);
        return Some(());
    }
    if number_expr_ok(init, &ctx.slot_of, ctx.by_id) {
        register_number(local, ctx);
        return Some(());
    }
    None
}

fn classify_array_pattern(elements: &[ArrayPatternEl], ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    for el in elements {
        match el {
            ArrayPatternEl::Elision => {}
            ArrayPatternEl::Pattern { binding, default } => {
                if let Some(d) = default {
                    ctx.has_default = true;
                    if !value_expr_ok(d, &ctx.slot_of, ctx.by_id) {
                        return None;
                    }
                }
                classify_pattern(binding, SlotTy::Number, ctx)?;
            }
            ArrayPatternEl::Rest(_) => return None,
        }
    }
    Some(())
}

fn classify_object_pattern(
    properties: &[ObjectPatternEl],
    ctx: &mut ClassifyCtx<'_>,
) -> Option<()> {
    for p in properties {
        match p {
            ObjectPatternEl::Prop {
                key,
                binding,
                default,
                ..
            } => {
                if !prop_key_ok(key) {
                    return None;
                }
                if let Some(d) = default {
                    ctx.has_default = true;
                    if !value_expr_ok(d, &ctx.slot_of, ctx.by_id) {
                        return None;
                    }
                }
                let bind_ty = match binding {
                    Pattern::Object(_) => SlotTy::Object,
                    _ => SlotTy::Number,
                };
                classify_pattern(binding, bind_ty, ctx)?;
            }
            ObjectPatternEl::Rest(_) => return None,
        }
    }
    Some(())
}

fn classify_pattern(
    binding: &Pattern,
    bind_ty: SlotTy,
    ctx: &mut ClassifyCtx<'_>,
) -> Option<()> {
    match binding {
        Pattern::Local(id) => {
            match bind_ty {
                SlotTy::Number => register_number(*id, ctx),
                other => register_slot(*id, other, ctx),
            }
            Some(())
        }
        Pattern::Object(props) => {
            ctx.has_object_pat = true;
            classify_object_pattern(props, ctx)
        }
        Pattern::Array(els) => {
            ctx.has_array_pat = true;
            classify_array_pattern(els, ctx)
        }
        Pattern::Name(_) | Pattern::Member { .. } => None,
    }
}

fn register_number(id: LocalId, ctx: &mut ClassifyCtx<'_>) {
    if let Some(existing) = ctx.slot_of.get(&id).copied() {
        if existing == SlotTy::Number {
            if !ctx.print_locals.contains(&id) {
                ctx.print_locals.push(id);
            }
            return;
        }
        // Upgrade provisional — should not happen for numbers.
        return;
    }
    ctx.slots.push((id, SlotTy::Number));
    ctx.slot_of.insert(id, SlotTy::Number);
    if !ctx.print_locals.contains(&id) {
        ctx.print_locals.push(id);
    }
}

fn register_slot(id: LocalId, ty: SlotTy, ctx: &mut ClassifyCtx<'_>) {
    if ctx.slot_of.contains_key(&id) {
        return;
    }
    ctx.slots.push((id, ty));
    ctx.slot_of.insert(id, ty);
}

fn prop_key_ok(key: &ObjectPropKey) -> bool {
    matches!(key, ObjectPropKey::Static(_))
}

fn array_expr_ok(
    expr: &Expr,
    slot_of: &HashMap<LocalId, SlotTy>,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    match expr {
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Elision => true,
            ArrayElement::Expr(e) => value_expr_ok(e, slot_of, by_id),
            ArrayElement::Spread(_) => false,
        }),
        Expr::Local { id, .. } => slot_of.get(id) == Some(&SlotTy::Array),
        _ => false,
    }
}

fn object_expr_ok(
    expr: &Expr,
    slot_of: &HashMap<LocalId, SlotTy>,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    match expr {
        Expr::Object { properties, .. } => properties.iter().all(|p| match p {
            ObjectProp::Property { key, value } => {
                prop_key_ok(key) && value_expr_ok(value, slot_of, by_id)
            }
            _ => false,
        }),
        Expr::Local { id, .. } => slot_of.get(id) == Some(&SlotTy::Object),
        _ => false,
    }
}

fn value_expr_ok(
    expr: &Expr,
    slot_of: &HashMap<LocalId, SlotTy>,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Null { .. } => true,
        Expr::Array { .. } => array_expr_ok(expr, slot_of, by_id),
        Expr::Object { .. } => object_expr_ok(expr, slot_of, by_id),
        Expr::Local { id, .. } => {
            slot_of.contains_key(id)
                || by_id.get(id).is_some_and(|l| {
                    l.name == "undefined" || matches!(l.ty, Type::Number | Type::Any)
                })
        }
        _ => false,
    }
}

fn number_expr_ok(
    expr: &Expr,
    slot_of: &HashMap<LocalId, SlotTy>,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, .. } => {
            slot_of.get(id) == Some(&SlotTy::Number)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add,
            right,
            ..
        } => number_expr_ok(left, slot_of, by_id) && number_expr_ok(right, slot_of, by_id),
        _ => false,
    }
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
    undef_id: Option<LocalId>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        let undef_id = module.locals.iter().find(|l| l.name == "undefined").map(|l| l.id);
        Self {
            module,
            out: String::new(),
            body: String::new(),
            allocas: HashMap::new(),
            slot_of: HashMap::new(),
            str_globals: Vec::new(),
            tmp: 0,
            str_n: 0,
            undef_id,
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
        let t = self.tmp;
        self.tmp += 1;
        format!("{prefix}{t}")
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for (id, ty) in &info.slots {
            self.slot_of.insert(*id, *ty);
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.20 destructure defaults via Runtime ABI)"
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
                ALLOC_OBJECT,
                OBJECT_GET,
                OBJECT_SET,
                PRINT_F64,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for (id, kind) in &info.slots {
            match kind {
                SlotTy::Number => {
                    let g = format!("es_dd_n{}", id.0);
                    writeln!(
                        self.out,
                        "@{g} = internal global double 0.00000000000000000e+00, align 8"
                    )
                    .ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::Array => {
                    let g = format!("es_dd_a{}", id.0);
                    writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::Object => {
                    let g = format!("es_dd_o{}", id.0);
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

        for id in &info.print_locals {
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
                    .ok_or_else(|| diag("es_dd: declare unknown slot"))?;
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
                    SlotTy::Object => {
                        let v = self.emit_object_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::DeclareArrayPattern {
                elements,
                init: Some(init),
                ..
            } => {
                let arr = self.emit_array_expr(init)?;
                self.emit_array_destructure(elements, &arr)
            }
            Stmt::DeclareObjectPattern {
                properties,
                init: Some(init),
                ..
            } => {
                let obj = self.emit_object_expr(init)?;
                self.emit_object_destructure(properties, &obj)
            }
            Stmt::Expr { expr } => match expr {
                Expr::Assign {
                    target: AssignTarget::ArrayPattern { elements },
                    op: AssignOp::Eq,
                    value,
                    ..
                } => {
                    let arr = self.emit_array_expr(value)?;
                    self.emit_array_destructure(elements, &arr)
                }
                Expr::Assign {
                    target: AssignTarget::ObjectPattern { properties },
                    op: AssignOp::Eq,
                    value,
                    ..
                } => {
                    let obj = self.emit_object_expr(value)?;
                    self.emit_object_destructure(properties, &obj)
                }
                _ => Err(diag("es_dd: unsupported expr stmt")),
            },
            _ => Err(diag("es_dd: unsupported stmt")),
        }
    }

    fn emit_array_destructure(
        &mut self,
        elements: &[ArrayPatternEl],
        arr: &str,
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
                    let len = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
                    )
                    .ok();
                    let in_range = self.fresh();
                    writeln!(self.body, "  {in_range} = icmp ult i64 {i}, {len}").ok();
                    let got = self.fresh();
                    writeln!(self.body, "  {got} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {got}").ok();
                    let take_l = self.fresh_label("dd_take");
                    let def_l = self.fresh_label("dd_def");
                    let done_l = self.fresh_label("dd_done");
                    writeln!(
                        self.body,
                        "  br i1 {in_range}, label %{take_l}, label %{def_l}"
                    )
                    .ok();
                    writeln!(self.body, "{take_l}:").ok();
                    let raw = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_GET.call_to(&raw, &format!("ptr {arr}, i64 {i}"))
                    )
                    .ok();
                    let is_null = self.fresh();
                    writeln!(self.body, "  {is_null} = icmp eq ptr {raw}, null").ok();
                    let use_l = self.fresh_label("dd_use");
                    writeln!(
                        self.body,
                        "  br i1 {is_null}, label %{def_l}, label %{use_l}"
                    )
                    .ok();
                    writeln!(self.body, "{use_l}:").ok();
                    writeln!(self.body, "  store ptr {raw}, ptr {got}").ok();
                    writeln!(self.body, "  br label %{done_l}").ok();
                    writeln!(self.body, "{def_l}:").ok();
                    if let Some(d) = default {
                        let dv = self.emit_value_as_ptr(d)?;
                        writeln!(self.body, "  store ptr {dv}, ptr {got}").ok();
                    }
                    writeln!(self.body, "  br label %{done_l}").ok();
                    writeln!(self.body, "{done_l}:").ok();
                    let val = self.fresh();
                    writeln!(self.body, "  {val} = load ptr, ptr {got}").ok();
                    self.emit_bind_pattern(binding, &val)?;
                    let i2 = self.fresh();
                    writeln!(self.body, "  {i2} = load i64, ptr {idx_ptr}").ok();
                    let n = self.fresh();
                    writeln!(self.body, "  {n} = add i64 {i2}, 1").ok();
                    writeln!(self.body, "  store i64 {n}, ptr {idx_ptr}").ok();
                }
                ArrayPatternEl::Rest(_) => {
                    return Err(diag("es_dd: array rest not supported"));
                }
            }
        }
        Ok(())
    }

    fn emit_object_destructure(
        &mut self,
        properties: &[ObjectPatternEl],
        obj: &str,
    ) -> Result<(), Diagnostic> {
        for p in properties {
            match p {
                ObjectPatternEl::Prop {
                    key,
                    binding,
                    default,
                    ..
                } => {
                    let key_ptr = self.emit_prop_key(key)?;
                    let got = self.fresh();
                    writeln!(self.body, "  {got} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {got}").ok();
                    let raw = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_GET.call_to(&raw, &format!("ptr {obj}, ptr {key_ptr}"))
                    )
                    .ok();
                    let is_null = self.fresh();
                    writeln!(self.body, "  {is_null} = icmp eq ptr {raw}, null").ok();
                    let use_l = self.fresh_label("od_use");
                    let def_l = self.fresh_label("od_def");
                    let done_l = self.fresh_label("od_done");
                    writeln!(
                        self.body,
                        "  br i1 {is_null}, label %{def_l}, label %{use_l}"
                    )
                    .ok();
                    writeln!(self.body, "{use_l}:").ok();
                    writeln!(self.body, "  store ptr {raw}, ptr {got}").ok();
                    writeln!(self.body, "  br label %{done_l}").ok();
                    writeln!(self.body, "{def_l}:").ok();
                    if let Some(d) = default {
                        let dv = self.emit_value_as_ptr(d)?;
                        writeln!(self.body, "  store ptr {dv}, ptr {got}").ok();
                    }
                    writeln!(self.body, "  br label %{done_l}").ok();
                    writeln!(self.body, "{done_l}:").ok();
                    let val = self.fresh();
                    writeln!(self.body, "  {val} = load ptr, ptr {got}").ok();
                    self.emit_bind_pattern(binding, &val)?;
                }
                ObjectPatternEl::Rest(_) => {
                    return Err(diag("es_dd: object rest not supported"));
                }
            }
        }
        Ok(())
    }

    fn emit_bind_pattern(&mut self, binding: &Pattern, val_ptr: &str) -> Result<(), Diagnostic> {
        match binding {
            Pattern::Local(id) => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_dd: pattern local unknown"))?;
                let ptr = self.slot_ptr(*id)?;
                match kind {
                    SlotTy::Number => {
                        let i = self.fresh();
                        writeln!(self.body, "  {i} = ptrtoint ptr {val_ptr} to i64").ok();
                        let d = self.fresh();
                        writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                        writeln!(self.body, "  store double {d}, ptr {ptr}").ok();
                    }
                    SlotTy::Array | SlotTy::Object => {
                        writeln!(self.body, "  store ptr {val_ptr}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Pattern::Object(props) => self.emit_object_destructure(props, val_ptr),
            Pattern::Array(els) => self.emit_array_destructure(els, val_ptr),
            Pattern::Name(_) | Pattern::Member { .. } => {
                Err(diag("es_dd: unsupported pattern binding"))
            }
        }
    }

    fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Array { elements, .. } => {
                let n = elements.len() as i64;
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
                        ArrayElement::Expr(e) => {
                            let v = self.emit_value_as_ptr(e)?;
                            writeln!(
                                self.body,
                                "  {}",
                                ARRAY_SET.call(&format!("ptr {arr}, i64 {i}, ptr {v}"))
                            )
                            .ok();
                        }
                        ArrayElement::Spread(_) => {
                            return Err(diag("es_dd: array spread unsupported"));
                        }
                    }
                }
                Ok(arr)
            }
            Expr::Local { id, .. } => {
                if self.slot_of.get(id) != Some(&SlotTy::Array) {
                    return Err(diag("es_dd: expected array local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            _ => Err(diag("es_dd: unsupported array expr")),
        }
    }

    fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Object { properties, .. } => {
                let obj = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
                for p in properties {
                    match p {
                        ObjectProp::Property { key, value } => {
                            let key_ptr = self.emit_prop_key(key)?;
                            let val_ptr = self.emit_value_as_ptr(value)?;
                            writeln!(
                                self.body,
                                "  {}",
                                OBJECT_SET.call(&format!(
                                    "ptr {obj}, ptr {key_ptr}, ptr {val_ptr}"
                                ))
                            )
                            .ok();
                        }
                        _ => return Err(diag("es_dd: only plain object props")),
                    }
                }
                Ok(obj)
            }
            Expr::Local { id, .. } => {
                if self.slot_of.get(id) != Some(&SlotTy::Object) {
                    return Err(diag("es_dd: expected object local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            _ => Err(diag("es_dd: unsupported object expr")),
        }
    }

    fn emit_value_as_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        if self.is_undefined(expr) {
            let t = self.fresh();
            writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
            return Ok(t);
        }
        match expr {
            Expr::Number { .. } => {
                let n = self.emit_number_expr(expr)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                let p = self.fresh();
                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                Ok(p)
            }
            Expr::Null { .. } => {
                let t = self.fresh();
                writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                Ok(t)
            }
            Expr::Array { .. } => self.emit_array_expr(expr),
            Expr::Object { .. } => self.emit_object_expr(expr),
            Expr::Local { id, .. } => {
                if Some(*id) == self.undef_id {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
                    return Ok(t);
                }
                match self.slot_of.get(id).copied() {
                    Some(SlotTy::Number) => {
                        let ptr = self.slot_ptr(*id)?;
                        let d = self.fresh();
                        writeln!(self.body, "  {d} = load double, ptr {ptr}").ok();
                        let i = self.fresh();
                        writeln!(self.body, "  {i} = fptosi double {d} to i64").ok();
                        let p = self.fresh();
                        writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                        Ok(p)
                    }
                    Some(SlotTy::Array) | Some(SlotTy::Object) => {
                        let ptr = self.slot_ptr(*id)?;
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                        Ok(t)
                    }
                    None => Err(diag("es_dd: value local unknown")),
                }
            }
            _ => Err(diag("es_dd: unsupported value expr")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let lit = format_number_const(raw)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = fadd double {lit}, 0.000000e+00").ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                if self.slot_of.get(id) != Some(&SlotTy::Number) {
                    return Err(diag("es_dd: expected number local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
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
            _ => Err(diag("es_dd: unsupported number expr")),
        }
    }

    fn emit_prop_key(&mut self, key: &ObjectPropKey) -> Result<String, Diagnostic> {
        match key {
            ObjectPropKey::Static(s) => self.string_const(&s.to_string_lossy()),
            ObjectPropKey::Computed(_) => Err(diag("es_dd: computed keys unsupported")),
        }
    }

    fn is_undefined(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => Some(*id) == self.undef_id,
            _ => false,
        }
    }

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        self.allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("es_dd: slot missing"))
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_dd_str.{}", self.str_n);
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
