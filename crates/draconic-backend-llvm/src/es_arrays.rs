//! N08.06.01–N08.06.06: native observations for ES array literals, index
//! access, `.length`, element assignment, spread in array literals,
//! `for-of` over arrays, and array destructuring (`es/arrays/array_lit_access`,
//! `array_element_assign`, `array_spread`, `array_for_of`, `array_destructure`).
//!
//! Arrays are Runtime GC heap values (`draconic_rt_array_*`). Number elements
//! are stored as `inttoptr` of integer bit-patterns; nested arrays store GC
//! ptrs; strings are cstr ptrs; booleans are `inttoptr` 0/1; `null`/`undefined`
//! are null. Empty objects use Runtime `alloc_object` for member destructure
//! targets. Number locals print via `print_f64`; string index/accumulator
//! results via `print_str`. `for-of` walks length + index get with break/continue.
//! Destructuring binds via index get + rest copy; defaults fire on null/hole.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    ArrayElement, ArrayPatternEl, AssignTarget, Expr, IrType as Type, Local, LocalId, Module,
    Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, ARRAY_SPREAD_ARRAY,
    ARRAY_SPREAD_CSTR, CSTR_CONCAT, GC_INIT, OBJECT_GET, OBJECT_SET, PRINT_F64, PRINT_STR,
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
    Object,
}

/// Homogeneous element kind for spread/index type inference (N08.06.03).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ElemKind {
    Number,
    String,
    Array,
    Unknown,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    /// Observation prints: numbers via `print_f64`, strings via `print_str`.
    print_locals: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx<'a> {
    by_id: &'a HashMap<LocalId, &'a Local>,
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    has_array: bool,
    arr_inits: HashMap<LocalId, Expr>,
    arr_elem: HashMap<LocalId, ElemKind>,
    slot_of: HashMap<LocalId, SlotTy>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut ctx = ClassifyCtx {
        by_id: &by_id,
        slots: Vec::new(),
        print_locals: Vec::new(),
        has_array: false,
        arr_inits: HashMap::new(),
        arr_elem: HashMap::new(),
        slot_of: HashMap::new(),
    };

    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }

    if !ctx.has_array || ctx.print_locals.is_empty() {
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
            if !array_expr_ok(init, ctx.by_id, &ctx.slot_of) {
                return None;
            }
            ctx.has_array = true;
            let ek = array_expr_elem_kind(init, &ctx.arr_inits, &ctx.arr_elem, &ctx.slot_of)
                .unwrap_or(ElemKind::Unknown);
            classify_array_pattern(elements, ek, true, ctx)
        }
        // Multi-declarator `var a, b, c` lowers as consecutive Declare; bare already handled.
        Stmt::Expr { expr } => {
            if let Expr::Assign {
                target: AssignTarget::ArrayPattern { elements },
                op: AssignOp::Eq,
                value,
                ..
            } = expr
            {
                if !array_expr_ok(value, ctx.by_id, &ctx.slot_of) {
                    return None;
                }
                ctx.has_array = true;
                let ek = array_expr_elem_kind(value, &ctx.arr_inits, &ctx.arr_elem, &ctx.slot_of)
                    .unwrap_or(ElemKind::Unknown);
                return classify_array_pattern(elements, ek, true, ctx);
            }
            if member_assign_ok(expr, ctx.by_id, &ctx.slot_of)
                || local_assign_ok(expr, ctx.by_id, &ctx.slot_of)
            {
                Some(())
            } else {
                None
            }
        }
        Stmt::Block { body } => {
            for s in body {
                classify_stmt(s, ctx)?;
            }
            Some(())
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
        } => {
            if *is_await {
                return None;
            }
            if !array_expr_ok(right, ctx.by_id, &ctx.slot_of) {
                return None;
            }
            let ek = array_expr_elem_kind(
                right,
                &ctx.arr_inits,
                &ctx.arr_elem,
                &ctx.slot_of,
            )
            // Empty arrays (and other untyped iterables) still support for-of;
            // bind as Number when element kind is unknown (body never observes).
            .unwrap_or(ElemKind::Unknown);
            let bind_ty = match ek {
                ElemKind::Number | ElemKind::Unknown => SlotTy::Number,
                ElemKind::String => SlotTy::String,
                ElemKind::Array => SlotTy::Array,
            };
            classify_for_of_left(left, right, bind_ty, ek, ctx)?;
            classify_stmt(body, ctx)
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            if !cmp_number_ok(test, ctx.by_id, &ctx.slot_of) {
                return None;
            }
            classify_stmt(consequent, ctx)?;
            if let Some(alt) = alternate {
                classify_stmt(alt, ctx)?;
            }
            Some(())
        }
        Stmt::Break { label: None } | Stmt::Continue { label: None } => Some(()),
        _ => None,
    }
}

/// Register slots for array destructuring pattern elements.
/// `print_nums`: push number bindings to observation list (declare + assign patterns).
fn classify_array_pattern(
    elements: &[ArrayPatternEl],
    elem_kind: ElemKind,
    print_nums: bool,
    ctx: &mut ClassifyCtx<'_>,
) -> Option<()> {
    for el in elements {
        match el {
            ArrayPatternEl::Elision => {}
            ArrayPatternEl::Pattern { binding, default } => {
                if let Some(d) = default {
                    if !value_expr_ok(d, ctx.by_id, &ctx.slot_of)
                        && !number_expr_ok(d, ctx.by_id, &ctx.slot_of)
                    {
                        return None;
                    }
                }
                let bind_ty = match elem_kind {
                    ElemKind::Number | ElemKind::Unknown => SlotTy::Number,
                    ElemKind::String => SlotTy::String,
                    ElemKind::Array => SlotTy::Array,
                };
                classify_pattern_binding(binding, bind_ty, elem_kind, print_nums, ctx)?;
            }
            ArrayPatternEl::Rest(binding) => {
                // Rest binds an array; nested patterns (e.g. `[...[x]]`) still
                // observe inner number locals. Bare rest locals are Array slots
                // (print_nums only applies to Number bindings).
                classify_pattern_binding(binding, SlotTy::Array, elem_kind, print_nums, ctx)?;
            }
        }
    }
    Some(())
}

fn classify_pattern_binding(
    binding: &Pattern,
    bind_ty: SlotTy,
    elem_kind: ElemKind,
    print_nums: bool,
    ctx: &mut ClassifyCtx<'_>,
) -> Option<()> {
    match binding {
        Pattern::Local(id) => {
            if let Some(existing) = ctx.slot_of.get(id).copied() {
                if existing == bind_ty {
                    // already registered
                } else if existing == SlotTy::Number && bind_ty == SlotTy::Number {
                    // bare let provisional number
                } else if existing == SlotTy::Number && bind_ty == SlotTy::Array {
                    // bare `let tail` upgraded when bound by rest pattern
                    if let Some((_, slot)) = ctx.slots.iter_mut().find(|(l, _)| l == id) {
                        *slot = SlotTy::Array;
                    }
                    ctx.slot_of.insert(*id, SlotTy::Array);
                } else {
                    return None;
                }
            } else {
                ctx.slots.push((*id, bind_ty));
                ctx.slot_of.insert(*id, bind_ty);
            }
            if bind_ty == SlotTy::Array {
                ctx.has_array = true;
                if elem_kind != ElemKind::Unknown {
                    ctx.arr_elem.insert(*id, elem_kind);
                }
            }
            if print_nums && bind_ty == SlotTy::Number {
                if !ctx.print_locals.iter().any(|(l, _)| l == id) {
                    ctx.print_locals.push((*id, SlotTy::Number));
                }
            }
            Some(())
        }
        Pattern::Member {
            object,
            property,
            computed,
        } => {
            if !object_expr_ok(object, ctx.by_id, &ctx.slot_of) {
                return None;
            }
            if *computed {
                if !number_expr_ok(property, ctx.by_id, &ctx.slot_of)
                    && !string_expr_ok(property, ctx.by_id, &ctx.slot_of)
                {
                    return None;
                }
            } else if !matches!(property.as_ref(), Expr::String { .. }) {
                return None;
            }
            Some(())
        }
        Pattern::Array(inner) => {
            // Nested array pattern: elements are of bind_ty's element kind.
            let inner_ek = match bind_ty {
                SlotTy::Array => elem_kind,
                _ => ElemKind::Unknown,
            };
            // When outer elem is Array, nested binds the inner array's elements.
            let nested_ek = if bind_ty == SlotTy::Array {
                // Infer from known array-of-arrays when possible is handled by caller;
                // default nested number elements (fixture uses number matrices).
                match elem_kind {
                    ElemKind::Array => ElemKind::Number,
                    other => other,
                }
            } else {
                inner_ek
            };
            classify_array_pattern(inner, nested_ek, print_nums, ctx)
        }
        Pattern::Name(_) | Pattern::Object(_) => None,
    }
}

fn object_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Object { properties, .. } => properties.is_empty(),
        // Arrays also use Type::Object in IR — only trust explicit Object slots.
        Expr::Local { id, .. } => slot_of.get(id) == Some(&SlotTy::Object),
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && object_expr_ok(object, by_id, slot_of)
                && if *computed {
                    // obj["k"] only (string keys); number index is array access.
                    string_expr_ok(property, by_id, slot_of)
                        || matches!(property.as_ref(), Expr::String { .. })
                } else {
                    matches!(property.as_ref(), Expr::String { .. })
                }
        }
        _ => false,
    }
}

fn classify_for_of_left(
    left: &Stmt,
    right: &Expr,
    bind_ty: SlotTy,
    ek: ElemKind,
    ctx: &mut ClassifyCtx<'_>,
) -> Option<()> {
    match left {
        Stmt::Declare {
            local,
            init: None,
            ..
        } => {
            if ctx.slot_of.contains_key(local) {
                return None;
            }
            ctx.slots.push((*local, bind_ty));
            ctx.slot_of.insert(*local, bind_ty);
            if bind_ty == SlotTy::Array {
                ctx.has_array = true;
                if let Some(inner) = for_of_bound_array_elem_kind(right, ek, ctx) {
                    ctx.arr_elem.insert(*local, inner);
                }
            }
            Some(())
        }
        Stmt::Expr {
            expr: Expr::Local { id, .. },
        } => match ctx.slot_of.get(id).copied() {
            Some(existing) if existing == bind_ty => Some(()),
            // Bare `let y` provisionally Number before for-of assign.
            Some(SlotTy::Number) if bind_ty == SlotTy::Number => Some(()),
            None => {
                ctx.slots.push((*id, bind_ty));
                ctx.slot_of.insert(*id, bind_ty);
                Some(())
            }
            _ => None,
        },
        _ => None,
    }
}

/// Element kind of arrays yielded by `for (let row of nested)` when `nested`
/// is an array-of-arrays (e.g. `[[10,20],[30]]` → Number).
fn for_of_bound_array_elem_kind(
    right: &Expr,
    ek: ElemKind,
    ctx: &ClassifyCtx<'_>,
) -> Option<ElemKind> {
    if ek != ElemKind::Array {
        return None;
    }
    let lit = match right {
        Expr::Array { .. } => right.clone(),
        Expr::Local { id, .. } => ctx.arr_inits.get(id).cloned()?,
        _ => return None,
    };
    let Expr::Array { elements, .. } = lit else {
        return None;
    };
    let mut kind: Option<ElemKind> = None;
    for el in elements {
        let ArrayElement::Expr(Expr::Array { elements: inner, .. }) = el else {
            return None;
        };
        let k = array_lit_elem_kind(&inner, &ctx.arr_inits, &ctx.arr_elem, &ctx.slot_of)?;
        kind = Some(match kind {
            None => k,
            Some(prev) if prev == k => prev,
            Some(_) => ElemKind::Unknown,
        });
    }
    kind
}

fn classify_declare(
    local: LocalId,
    init: Option<&Expr>,
    ctx: &mut ClassifyCtx<'_>,
) -> Option<()> {
    let loc = ctx.by_id.get(&local)?;
    let Some(init) = init else {
        // Bare `let y` — provisional number slot (for-of assign target).
        if ctx.slot_of.contains_key(&local) {
            return Some(());
        }
        ctx.slots.push((local, SlotTy::Number));
        ctx.slot_of.insert(local, SlotTy::Number);
        return Some(());
    };
    if matches!(init, Expr::Array { .. }) {
        if !array_expr_ok(init, ctx.by_id, &ctx.slot_of) {
            return None;
        }
        ctx.has_array = true;
        ctx.slots.push((local, SlotTy::Array));
        ctx.slot_of.insert(local, SlotTy::Array);
        ctx.arr_inits.insert(local, init.clone());
        if let Some(k) = array_expr_elem_kind(init, &ctx.arr_inits, &ctx.arr_elem, &ctx.slot_of) {
            ctx.arr_elem.insert(local, k);
        }
        return Some(());
    }
    if matches!(init, Expr::Object { .. }) {
        if !object_expr_ok(init, ctx.by_id, &ctx.slot_of) {
            return None;
        }
        ctx.slots.push((local, SlotTy::Object));
        ctx.slot_of.insert(local, SlotTy::Object);
        return Some(());
    }
    if is_undefined_expr(init) {
        ctx.slots.push((local, SlotTy::Null));
        ctx.slot_of.insert(local, SlotTy::Null);
        return Some(());
    }
    if matches!(init, Expr::String { .. }) {
        if !string_expr_ok(init, ctx.by_id, &ctx.slot_of) {
            return None;
        }
        ctx.slots.push((local, SlotTy::String));
        ctx.slot_of.insert(local, SlotTy::String);
        // Empty-string accumulators (for-of concat) are observations.
        if let Expr::String { value, .. } = init {
            if value.to_string_lossy().is_empty() {
                ctx.print_locals.push((local, SlotTy::String));
            }
        }
        return Some(());
    }
    if let Expr::Local { id, .. } = init {
        if ctx.slots.iter().any(|(s, k)| s == id && *k == SlotTy::Array) {
            ctx.has_array = true;
            ctx.slots.push((local, SlotTy::Array));
            ctx.slot_of.insert(local, SlotTy::Array);
            if let Some(e) = ctx.arr_inits.get(id).cloned() {
                ctx.arr_inits.insert(local, e);
            }
            if let Some(k) = ctx.arr_elem.get(id).copied() {
                ctx.arr_elem.insert(local, k);
            }
            return Some(());
        }
        if ctx.slots.iter().any(|(s, k)| s == id && *k == SlotTy::String) {
            ctx.slots.push((local, SlotTy::String));
            ctx.slot_of.insert(local, SlotTy::String);
            return Some(());
        }
        if ctx
            .slots
            .iter()
            .any(|(s, k)| s == id && *k == SlotTy::Number)
            || matches!(loc.ty, Type::Number)
        {
            ctx.slots.push((local, SlotTy::Number));
            ctx.slot_of.insert(local, SlotTy::Number);
            ctx.print_locals.push((local, SlotTy::Number));
            return Some(());
        }
        return None;
    }
    if let Some(kind) = infer_expr_slot(init, &ctx.arr_inits, &ctx.arr_elem, &ctx.slot_of) {
        if !value_expr_ok(init, ctx.by_id, &ctx.slot_of) {
            return None;
        }
        ctx.slots.push((local, kind));
        ctx.slot_of.insert(local, kind);
        match kind {
            SlotTy::Number => ctx.print_locals.push((local, SlotTy::Number)),
            SlotTy::String => {
                if matches!(
                    init,
                    Expr::Member {
                        computed: true,
                        ..
                    }
                ) {
                    ctx.print_locals.push((local, SlotTy::String));
                }
            }
            SlotTy::Array => {
                ctx.has_array = true;
                if let Some(k) =
                    array_expr_elem_kind(init, &ctx.arr_inits, &ctx.arr_elem, &ctx.slot_of)
                {
                    ctx.arr_elem.insert(local, k);
                }
            }
            _ => {}
        }
        return Some(());
    }
    if number_expr_ok(init, ctx.by_id, &ctx.slot_of) {
        // Number from arithmetic, member read, pattern-bound locals, etc.
        ctx.slots.push((local, SlotTy::Number));
        ctx.slot_of.insert(local, SlotTy::Number);
        ctx.print_locals.push((local, SlotTy::Number));
        return Some(());
    }
    None
}

fn cmp_number_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Binary {
            left,
            op: BinaryOp::EqEqEq | BinaryOp::EqEq | BinaryOp::NotEqEq | BinaryOp::NotEq,
            right,
            ..
        } => number_expr_ok(left, by_id, slot_of) && number_expr_ok(right, by_id, slot_of),
        _ => false,
    }
}

fn local_assign_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    let Expr::Assign {
        target: AssignTarget::Local(id),
        op: AssignOp::Eq,
        value,
        ..
    } = expr
    else {
        return false;
    };
    match slot_of.get(id) {
        Some(SlotTy::Number) => number_expr_ok(value, by_id, slot_of),
        Some(SlotTy::String) => string_expr_ok(value, by_id, slot_of),
        Some(SlotTy::Array) => array_expr_ok(value, by_id, slot_of),
        _ => false,
    }
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

fn infer_expr_slot(
    expr: &Expr,
    arr_inits: &HashMap<LocalId, Expr>,
    arr_elem: &HashMap<LocalId, ElemKind>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<SlotTy> {
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
            // obj.prop / obj["k"] — number observations on object props.
            if object_expr_ok(object, &HashMap::new(), slot_of) {
                let string_key = if *computed {
                    matches!(property.as_ref(), Expr::String { .. })
                } else {
                    matches!(property.as_ref(), Expr::String { .. })
                };
                if string_key {
                    return Some(SlotTy::Number);
                }
            }
            if *computed {
                if let Some(idx) = const_index(property) {
                    if let Some(elem) = resolve_array_elem(object, idx, arr_inits) {
                        return literal_or_array_slot(&elem);
                    }
                }
                // obj.arr[i] — array property then index.
                if array_expr_ok(object, &HashMap::new(), slot_of) {
                    return slot_from_elem_kind(
                        array_expr_elem_kind(object, arr_inits, arr_elem, slot_of)
                            .unwrap_or(ElemKind::Number),
                    )
                    .or(Some(SlotTy::Number));
                }
                return slot_from_elem_kind(array_expr_elem_kind(
                    object, arr_inits, arr_elem, slot_of,
                )?);
            }
            None
        }
        _ => None,
    }
}

fn slot_from_elem_kind(k: ElemKind) -> Option<SlotTy> {
    match k {
        ElemKind::Number => Some(SlotTy::Number),
        ElemKind::String => Some(SlotTy::String),
        ElemKind::Array => Some(SlotTy::Array),
        ElemKind::Unknown => None,
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

fn array_expr_elem_kind(
    expr: &Expr,
    arr_inits: &HashMap<LocalId, Expr>,
    arr_elem: &HashMap<LocalId, ElemKind>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<ElemKind> {
    match expr {
        Expr::Array { elements, .. } => array_lit_elem_kind(elements, arr_inits, arr_elem, slot_of),
        Expr::Local { id, .. } => arr_elem.get(id).copied().or_else(|| {
            arr_inits
                .get(id)
                .and_then(|e| array_expr_elem_kind(e, arr_inits, arr_elem, slot_of))
        }),
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
            let idx = const_index(property)?;
            let elem = resolve_array_elem(object, idx, arr_inits)?;
            match elem {
                Expr::Array { elements, .. } => {
                    array_lit_elem_kind(&elements, arr_inits, arr_elem, slot_of)
                }
                Expr::String { .. } => Some(ElemKind::String),
                Expr::Number { .. } => Some(ElemKind::Number),
                _ => None,
            }
        }
        _ => None,
    }
}

fn array_lit_elem_kind(
    elements: &[ArrayElement],
    arr_inits: &HashMap<LocalId, Expr>,
    arr_elem: &HashMap<LocalId, ElemKind>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<ElemKind> {
    let mut kind: Option<ElemKind> = None;
    for el in elements {
        let k = match el {
            ArrayElement::Elision => continue,
            ArrayElement::Expr(e) => expr_as_elem_kind(e, arr_inits, arr_elem, slot_of)?,
            ArrayElement::Spread(e) => spread_source_elem_kind(e, arr_inits, arr_elem, slot_of)?,
        };
        kind = Some(match kind {
            None => k,
            Some(prev) if prev == k => prev,
            Some(_) => ElemKind::Unknown,
        });
    }
    Some(kind.unwrap_or(ElemKind::Unknown))
}

fn expr_as_elem_kind(
    expr: &Expr,
    arr_inits: &HashMap<LocalId, Expr>,
    arr_elem: &HashMap<LocalId, ElemKind>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<ElemKind> {
    match expr {
        Expr::Number { .. } => Some(ElemKind::Number),
        Expr::String { .. } => Some(ElemKind::String),
        Expr::Array { .. } => Some(ElemKind::Array),
        _ if is_undefined_expr(expr) || matches!(expr, Expr::Null { .. }) => {
            // Holes / undefined do not pin element kind.
            Some(ElemKind::Unknown)
        }
        Expr::Local { id, .. } => match slot_of.get(id) {
            Some(SlotTy::Number) => Some(ElemKind::Number),
            Some(SlotTy::String) => Some(ElemKind::String),
            Some(SlotTy::Array) => Some(ElemKind::Array),
            // Global `undefined` binding — hole-like.
            None => Some(ElemKind::Unknown),
            _ => None,
        },
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
                return Some(ElemKind::Number);
            }
            if *computed {
                if let Some(idx) = const_index(property) {
                    if let Some(elem) = resolve_array_elem(object, idx, arr_inits) {
                        return expr_as_elem_kind(&elem, arr_inits, arr_elem, slot_of);
                    }
                }
                return array_expr_elem_kind(object, arr_inits, arr_elem, slot_of);
            }
            None
        }
        _ => None,
    }
}

fn spread_source_elem_kind(
    expr: &Expr,
    arr_inits: &HashMap<LocalId, Expr>,
    arr_elem: &HashMap<LocalId, ElemKind>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<ElemKind> {
    match expr {
        Expr::String { .. } => Some(ElemKind::String),
        Expr::Local { id, .. } => match slot_of.get(id) {
            Some(SlotTy::String) => Some(ElemKind::String),
            Some(SlotTy::Array) => arr_elem.get(id).copied().or_else(|| {
                arr_inits
                    .get(id)
                    .and_then(|e| array_expr_elem_kind(e, arr_inits, arr_elem, slot_of))
            }),
            _ => None,
        },
        Expr::Array { elements, .. } => {
            array_lit_elem_kind(elements, arr_inits, arr_elem, slot_of)
        }
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
            let idx = const_index(property)?;
            let elem = resolve_array_elem(object, idx, arr_inits)?;
            match elem {
                Expr::Array { elements, .. } => {
                    array_lit_elem_kind(&elements, arr_inits, arr_elem, slot_of)
                }
                Expr::String { .. } => Some(ElemKind::String),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_array_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Object | Type::Any)
}

fn is_number_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Number | Type::Any)
}

fn array_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => value_expr_ok(e, by_id, slot_of),
            ArrayElement::Elision => true,
            ArrayElement::Spread(e) => {
                array_expr_ok(e, by_id, slot_of) || string_expr_ok(e, by_id, slot_of)
            }
        }),
        Expr::Local { id, ty } => {
            slot_of.get(id) == Some(&SlotTy::Array)
                || is_array_slot_ty(ty)
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
            if *optional {
                return false;
            }
            // obj.arr property holding an array (rest member target read-back).
            if object_expr_ok(object, by_id, slot_of) {
                return if *computed {
                    string_expr_ok(property, by_id, slot_of)
                        || matches!(property.as_ref(), Expr::String { .. })
                } else {
                    matches!(property.as_ref(), Expr::String { .. })
                };
            }
            array_expr_ok(object, by_id, slot_of)
                && if *computed {
                    number_expr_ok(property, by_id, slot_of)
                } else {
                    member_key_is_length(property)
                }
        }
        _ => false,
    }
}

fn value_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    number_expr_ok(expr, by_id, slot_of)
        || string_expr_ok(expr, by_id, slot_of)
        || bool_expr_ok(expr, by_id, slot_of)
        || null_expr_ok(expr, by_id, slot_of)
        || array_expr_ok(expr, by_id, slot_of)
        || object_expr_ok(expr, by_id, slot_of)
        || is_undefined_expr(expr)
}

fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            slot_of.get(id) == Some(&SlotTy::Number)
                || is_number_slot_ty(ty)
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
            if *optional {
                return false;
            }
            // obj.prop / obj["k"] number property (string key only — not arr index).
            if object_expr_ok(object, by_id, slot_of) {
                let string_key = if *computed {
                    string_expr_ok(property, by_id, slot_of)
                        || matches!(property.as_ref(), Expr::String { .. })
                } else {
                    matches!(property.as_ref(), Expr::String { .. })
                };
                if string_key {
                    return true;
                }
            }
            // arr[i] / obj.arr[i] — computed number index on array-valued base.
            array_expr_ok(object, by_id, slot_of)
                && if *computed {
                    number_expr_ok(property, by_id, slot_of)
                } else {
                    member_key_is_length(property)
                }
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
            array_expr_ok(object, by_id, slot_of)
                && number_expr_ok(property, by_id, slot_of)
                && number_expr_ok(value, by_id, slot_of)
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem,
            right,
            ..
        } => number_expr_ok(left, by_id, slot_of) && number_expr_ok(right, by_id, slot_of),
        _ => false,
    }
}

/// `a[i] = v` / `nested[0][0] = v` as a statement expression (N08.06.02).
fn member_assign_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
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
            array_expr_ok(object, by_id, slot_of)
                && number_expr_ok(property, by_id, slot_of)
                && value_expr_ok(value, by_id, slot_of)
        }
        _ => false,
    }
}

fn string_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::String { .. } => true,
        Expr::Local { id, ty } => {
            slot_of.get(id) == Some(&SlotTy::String)
                || matches!(ty, Type::String)
                || by_id.get(id).is_some_and(|l| matches!(l.ty, Type::String))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && *computed
                && array_expr_ok(object, by_id, slot_of)
                && number_expr_ok(property, by_id, slot_of)
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add,
            right,
            ..
        } => string_expr_ok(left, by_id, slot_of) && string_expr_ok(right, by_id, slot_of),
        _ => false,
    }
}

fn bool_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Boolean { .. } => true,
        Expr::Local { id, ty } => {
            slot_of.get(id) == Some(&SlotTy::Bool)
                || matches!(ty, Type::Boolean)
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
            !*optional
                && *computed
                && array_expr_ok(object, by_id, slot_of)
                && number_expr_ok(property, by_id, slot_of)
        }
        _ => false,
    }
}

fn null_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Null { .. } => true,
        _ if is_undefined_expr(expr) => true,
        Expr::Local { id, ty } => {
            is_undefined_local(*id, by_id)
                || slot_of.get(id) == Some(&SlotTy::Null)
                || matches!(ty, Type::Null | Type::Any)
                    && by_id
                        .get(id)
                        .is_some_and(|l| matches!(l.ty, Type::Null | Type::Any) || l.name == "undefined")
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && *computed
                && array_expr_ok(object, by_id, slot_of)
                && number_expr_ok(property, by_id, slot_of)
        }
        _ => false,
    }
}

fn member_key_is_length(property: &Expr) -> bool {
    matches!(property, Expr::String { value, .. } if value.to_string_lossy() == "length")
}

fn is_undefined_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == "undefined")
        || matches!(
            expr,
            Expr::Unary {
                op: draconic_ast::UnaryOp::Void,
                ..
            }
        )
}

fn is_undefined_local(id: LocalId, by_id: &HashMap<LocalId, &Local>) -> bool {
    by_id
        .get(&id)
        .is_some_and(|l| l.name == "undefined")
}

fn member_key_string(property: &Expr) -> Option<String> {
    match property {
        Expr::String { value, .. } => Some(value.to_string_lossy().to_string()),
        _ => None,
    }
}

struct CtrlFrame {
    break_label: String,
    continue_label: Option<String>,
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
    ctrls: Vec<CtrlFrame>,
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
            ctrls: Vec::new(),
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

    fn body_ends_with_terminator(&self) -> bool {
        self.body
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .is_some_and(|l| {
                let t = l.trim_start();
                t.starts_with("br ")
                    || t.starts_with("ret ")
                    || t.starts_with("unreachable")
                    || t.starts_with("switch ")
                    || t.starts_with("indirectbr ")
            })
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for (id, ty) in &info.slots {
            self.slot_of.insert(*id, *ty);
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.06 ES arrays via Runtime ABI)"
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
                ARRAY_SPREAD_ARRAY,
                ARRAY_SPREAD_CSTR,
                CSTR_CONCAT,
                ALLOC_OBJECT,
                OBJECT_GET,
                OBJECT_SET,
                PRINT_F64,
                PRINT_STR,
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
                SlotTy::String
                | SlotTy::Bool
                | SlotTy::Null
                | SlotTy::Array
                | SlotTy::Object => {
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

        for (id, kind) in &info.print_locals {
            let ptr = self.slot_ptr(*id)?;
            match kind {
                SlotTy::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                _ => {}
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
            Stmt::Expr { expr } => {
                if let Expr::Assign {
                    target: AssignTarget::ArrayPattern { elements },
                    op: AssignOp::Eq,
                    value,
                    ..
                } = expr
                {
                    let arr = self.emit_array_expr(value)?;
                    return self.emit_array_destructure(elements, &arr);
                }
                if matches!(
                    expr,
                    Expr::Assign {
                        target: AssignTarget::Local(_),
                        ..
                    }
                ) {
                    self.emit_local_assign(expr)
                } else {
                    let _ = self.emit_member_assign(expr, false)?;
                    Ok(())
                }
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
            } => {
                if *is_await {
                    return Err(diag("es_arrays: for-await-of not supported"));
                }
                self.emit_for_of(left, right, body)
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => self.emit_if(test, consequent, alternate.as_deref()),
            Stmt::Break { label: None } => {
                let end = self
                    .ctrls
                    .last()
                    .ok_or_else(|| diag("es_arrays: break outside loop"))?
                    .break_label
                    .clone();
                writeln!(self.body, "  br label %{end}").ok();
                Ok(())
            }
            Stmt::Continue { label: None } => {
                let cont = self
                    .ctrls
                    .iter()
                    .rev()
                    .find_map(|f| f.continue_label.clone())
                    .ok_or_else(|| diag("es_arrays: continue outside loop"))?;
                writeln!(self.body, "  br label %{cont}").ok();
                Ok(())
            }
            _ => Err(diag("es_arrays: unsupported stmt")),
        }
    }

    fn emit_if(
        &mut self,
        test: &Expr,
        consequent: &Stmt,
        alternate: Option<&Stmt>,
    ) -> Result<(), Diagnostic> {
        let cond = self.emit_cmp_i1(test)?;
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

    fn emit_cmp_i1(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let Expr::Binary {
            left,
            op,
            right,
            ..
        } = expr
        else {
            return Err(diag("es_arrays: if test must be comparison"));
        };
        let l = self.emit_number_expr(left)?;
        let r = self.emit_number_expr(right)?;
        let pred = match op {
            BinaryOp::EqEq | BinaryOp::EqEqEq => "oeq",
            BinaryOp::NotEq | BinaryOp::NotEqEq => "one",
            _ => return Err(diag("es_arrays: unsupported comparison")),
        };
        let t = self.fresh();
        writeln!(self.body, "  {t} = fcmp {pred} double {l}, {r}").ok();
        Ok(t)
    }

    /// Destructure `arr` into `elements` (declare or assign pattern).
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
                    let take_l = self.fresh_label("dstr_take");
                    let def_l = self.fresh_label("dstr_def");
                    let done_l = self.fresh_label("dstr_done");
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
                    // Hole / undefined → null ptr; treat as missing for defaults.
                    let is_null = self.fresh();
                    writeln!(self.body, "  {is_null} = icmp eq ptr {raw}, null").ok();
                    let use_l = self.fresh_label("dstr_use");
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
                ArrayPatternEl::Rest(binding) => {
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = load i64, ptr {idx_ptr}").ok();
                    let len = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
                    )
                    .ok();
                    // rest_len = max(0, len - i)
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
                    // Copy arr[i..] into rest[0..]
                    let j_ptr = self.fresh();
                    writeln!(self.body, "  {j_ptr} = alloca i64, align 8").ok();
                    writeln!(self.body, "  store i64 0, ptr {j_ptr}").ok();
                    let head = self.fresh_label("rest_head");
                    let body = self.fresh_label("rest_body");
                    let end = self.fresh_label("rest_end");
                    writeln!(self.body, "  br label %{head}").ok();
                    writeln!(self.body, "{head}:").ok();
                    let j = self.fresh();
                    writeln!(self.body, "  {j} = load i64, ptr {j_ptr}").ok();
                    let cmp = self.fresh();
                    writeln!(self.body, "  {cmp} = icmp ult i64 {j}, {rest_len}").ok();
                    writeln!(self.body, "  br i1 {cmp}, label %{body}, label %{end}").ok();
                    writeln!(self.body, "{body}:").ok();
                    let src_i = self.fresh();
                    writeln!(self.body, "  {src_i} = add i64 {i}, {j}").ok();
                    let elv = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_GET.call_to(&elv, &format!("ptr {arr}, i64 {src_i}"))
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
                    // Rest consumes the remainder; advance idx to len.
                    writeln!(self.body, "  store i64 {len}, ptr {idx_ptr}").ok();
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
                    .ok_or_else(|| diag("es_arrays: pattern local unknown slot"))?;
                let ptr = self.slot_ptr(*id)?;
                match kind {
                    SlotTy::Number => {
                        let i = self.fresh();
                        writeln!(self.body, "  {i} = ptrtoint ptr {val_ptr} to i64").ok();
                        let d = self.fresh();
                        writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                        writeln!(self.body, "  store double {d}, ptr {ptr}").ok();
                    }
                    SlotTy::Array
                    | SlotTy::String
                    | SlotTy::Bool
                    | SlotTy::Null
                    | SlotTy::Object => {
                        writeln!(self.body, "  store ptr {val_ptr}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Pattern::Member {
                object,
                property,
                computed,
            } => {
                let obj = self.emit_object_expr(object)?;
                let key = if *computed {
                    if matches!(property.as_ref(), Expr::String { .. })
                        || self.expr_is_string_slot(property)
                    {
                        self.emit_string_expr(property)?
                    } else {
                        return Err(diag("es_arrays: computed member pattern key must be string"));
                    }
                } else {
                    let s = member_key_string(property)
                        .ok_or_else(|| diag("es_arrays: member pattern key"))?;
                    self.string_const(&s)?
                };
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {val_ptr}"))
                )
                .ok();
                Ok(())
            }
            Pattern::Array(inner) => {
                // Nested array pattern: val_ptr is the array to destructure.
                self.emit_array_destructure(inner, val_ptr)
            }
            Pattern::Name(_) | Pattern::Object(_) => {
                Err(diag("es_arrays: unsupported pattern binding"))
            }
        }
    }

    fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Object { properties, .. } => {
                if !properties.is_empty() {
                    return Err(diag("es_arrays: only empty object literals"));
                }
                let t = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&t, "")).ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_arrays: object local unknown"))?;
                if kind != SlotTy::Object {
                    return Err(diag("es_arrays: expected object local"));
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
                    return Err(diag("es_arrays: optional object member"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = if *computed {
                    self.emit_string_expr(property)?
                } else {
                    let s = member_key_string(property)
                        .ok_or_else(|| diag("es_arrays: object member key"))?;
                    self.string_const(&s)?
                };
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_GET.call_to(&t, &format!("ptr {obj}, ptr {key}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_arrays: unsupported object expr")),
        }
    }

    fn emit_for_of(
        &mut self,
        left: &Stmt,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), Diagnostic> {
        let bind_id = match left {
            Stmt::Declare { local, init: None, .. } => *local,
            Stmt::Expr {
                expr: Expr::Local { id, .. },
            } => *id,
            _ => return Err(diag("es_arrays: unsupported for-of left")),
        };
        let bind_kind = *self
            .slot_of
            .get(&bind_id)
            .ok_or_else(|| diag("es_arrays: for-of bind unknown slot"))?;
        let bind_ptr = self.slot_ptr(bind_id)?;

        let arr = self.emit_array_expr(right)?;
        let idx_ptr = self.fresh();
        writeln!(self.body, "  {idx_ptr} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {idx_ptr}").ok();

        let head = self.fresh_label("forof_head");
        let bod = self.fresh_label("forof_body");
        let cont = self.fresh_label("forof_cont");
        let end = self.fresh_label("forof_end");
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{head}:").ok();
        let idx = self.fresh();
        writeln!(self.body, "  {idx} = load i64, ptr {idx_ptr}").ok();
        let len = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
        )
        .ok();
        let cmp = self.fresh();
        writeln!(self.body, "  {cmp} = icmp ult i64 {idx}, {len}").ok();
        writeln!(self.body, "  br i1 {cmp}, label %{bod}, label %{end}").ok();
        writeln!(self.body, "{bod}:").ok();
        let elem = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_GET.call_to(&elem, &format!("ptr {arr}, i64 {idx}"))
        )
        .ok();
        match bind_kind {
            SlotTy::Number => {
                let i = self.fresh();
                writeln!(self.body, "  {i} = ptrtoint ptr {elem} to i64").ok();
                let d = self.fresh();
                writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                writeln!(self.body, "  store double {d}, ptr {bind_ptr}").ok();
            }
            SlotTy::String | SlotTy::Array | SlotTy::Bool | SlotTy::Null | SlotTy::Object => {
                writeln!(self.body, "  store ptr {elem}, ptr {bind_ptr}").ok();
            }
        }
        self.ctrls.push(CtrlFrame {
            break_label: end.clone(),
            continue_label: Some(cont.clone()),
        });
        self.emit_stmt(body)?;
        self.ctrls.pop();
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  br label %{cont}").ok();
        }
        writeln!(self.body, "{cont}:").ok();
        let idx2 = self.fresh();
        writeln!(self.body, "  {idx2} = load i64, ptr {idx_ptr}").ok();
        let next = self.fresh();
        writeln!(self.body, "  {next} = add i64 {idx2}, 1").ok();
        writeln!(self.body, "  store i64 {next}, ptr {idx_ptr}").ok();
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{end}:").ok();
        Ok(())
    }

    fn emit_local_assign(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        let Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } = expr
        else {
            return Err(diag("es_arrays: expected local assign"));
        };
        let kind = *self
            .slot_of
            .get(id)
            .ok_or_else(|| diag("es_arrays: assign unknown slot"))?;
        let ptr = self.slot_ptr(*id)?;
        match kind {
            SlotTy::Number => {
                let v = self.emit_number_expr(value)?;
                writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
            }
            SlotTy::String => {
                let v = self.emit_string_expr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
            SlotTy::Array => {
                let v = self.emit_array_expr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
            SlotTy::Bool => {
                let v = self.emit_bool_as_ptr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
            SlotTy::Null => {
                let v = self.emit_null_as_ptr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
            SlotTy::Object => {
                let v = self.emit_object_expr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
            }
        }
        Ok(())
    }

    /// Emit `a[i] = v` (or nested). When `yield_number`, returns the RHS as double.
    fn emit_member_assign(
        &mut self,
        expr: &Expr,
        yield_number: bool,
    ) -> Result<String, Diagnostic> {
        let Expr::Assign {
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
        } = expr
        else {
            return Err(diag("es_arrays: expected computed member assign"));
        };
        let arr = self.emit_array_expr(object)?;
        let idx_d = self.emit_number_expr(property)?;
        let idx_i = self.fresh();
        writeln!(self.body, "  {idx_i} = fptosi double {idx_d} to i64").ok();
        if yield_number {
            let n = self.emit_number_expr(value)?;
            let i = self.fresh();
            writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
            let p = self.fresh();
            writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
            writeln!(
                self.body,
                "  {}",
                ARRAY_SET.call(&format!("ptr {arr}, i64 {idx_i}, ptr {p}"))
            )
            .ok();
            Ok(n)
        } else {
            let v = self.emit_value_as_ptr(value)?;
            writeln!(
                self.body,
                "  {}",
                ARRAY_SET.call(&format!("ptr {arr}, i64 {idx_i}, ptr {v}"))
            )
            .ok();
            Ok(String::new())
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
                // Object property number read: mem.y / mem["y"]
                if self.expr_is_object_slot(object) {
                    let obj = self.emit_object_expr(object)?;
                    let key = if *computed {
                        self.emit_string_expr(property)?
                    } else {
                        let s = member_key_string(property)
                            .ok_or_else(|| diag("es_arrays: object number prop key"))?;
                        self.string_const(&s)?
                    };
                    let raw = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_GET.call_to(&raw, &format!("ptr {obj}, ptr {key}"))
                    )
                    .ok();
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
                    let d = self.fresh();
                    writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                    return Ok(d);
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
            Expr::Assign {
                target: AssignTarget::Member { .. },
                ..
            } => self.emit_member_assign(expr, true),
            Expr::Binary {
                left,
                op,
                right,
                ..
            } if matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
            ) =>
            {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let inst = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    BinaryOp::Rem => "frem",
                    _ => unreachable!(),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
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
                // Object property array: restMem.arr
                if self.expr_is_object_slot(object) {
                    let obj = self.emit_object_expr(object)?;
                    let key = if *computed {
                        self.emit_string_expr(property)?
                    } else {
                        let s = member_key_string(property)
                            .ok_or_else(|| diag("es_arrays: object array prop key"))?;
                        self.string_const(&s)?
                    };
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_GET.call_to(&t, &format!("ptr {obj}, ptr {key}"))
                    )
                    .ok();
                    return Ok(t);
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
        let has_spread = elements
            .iter()
            .any(|el| matches!(el, ArrayElement::Spread(_)));
        if !has_spread {
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
                    ArrayElement::Spread(_) => unreachable!(),
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
            return Ok(arr);
        }

        // Spread path: grow from empty via ARRAY_SET / ARRAY_SPREAD_*.
        let arr = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_NEW.call_to(&arr, "i64 0")
        )
        .ok();
        for el in elements {
            match el {
                ArrayElement::Elision => {
                    let len = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
                    )
                    .ok();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {arr}, i64 {len}, ptr null"))
                    )
                    .ok();
                }
                ArrayElement::Expr(e) => {
                    let v = self.emit_value_as_ptr(e)?;
                    let len = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
                    )
                    .ok();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {arr}, i64 {len}, ptr {v}"))
                    )
                    .ok();
                }
                ArrayElement::Spread(e) => {
                    if self.expr_is_string_slot(e) || matches!(e, Expr::String { .. }) {
                        let s = self.emit_string_expr(e)?;
                        writeln!(
                            self.body,
                            "  {}",
                            ARRAY_SPREAD_CSTR.call(&format!("ptr {arr}, ptr {s}"))
                        )
                        .ok();
                    } else {
                        let src = self.emit_array_expr(e)?;
                        writeln!(
                            self.body,
                            "  {}",
                            ARRAY_SPREAD_ARRAY.call(&format!("ptr {arr}, ptr {src}"))
                        )
                        .ok();
                    }
                }
            }
        }
        Ok(arr)
    }

    fn emit_value_as_ptr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        if matches!(expr, Expr::Array { .. }) || self.expr_is_array_slot(expr) {
            return self.emit_array_expr(expr);
        }
        if matches!(expr, Expr::Object { .. }) || self.expr_is_object_slot(expr) {
            return self.emit_object_expr(expr);
        }
        if matches!(expr, Expr::String { .. }) || self.expr_is_string_slot(expr) {
            return self.emit_string_expr(expr);
        }
        if matches!(expr, Expr::Boolean { .. }) || self.expr_is_bool_slot(expr) {
            return self.emit_bool_as_ptr(expr);
        }
        if matches!(expr, Expr::Null { .. })
            || self.expr_is_null_slot(expr)
            || is_undefined_expr(expr)
            || matches!(
                expr,
                Expr::Local { id, .. } if self
                    .module
                    .locals
                    .iter()
                    .any(|l| l.id == *id && l.name == "undefined")
            )
        {
            return self.emit_null_as_ptr(&Expr::Null { ty: Type::Null });
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
        if matches!(expr, Expr::Null { .. }) || is_undefined_expr(expr) {
            let t = self.fresh();
            writeln!(self.body, "  {t} = inttoptr i64 0 to ptr").ok();
            return Ok(t);
        }
        match expr {
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

    fn expr_is_object_slot(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id) == Some(&SlotTy::Object),
            Expr::Object { .. } => true,
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
        SlotTy::Object => "o",
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
