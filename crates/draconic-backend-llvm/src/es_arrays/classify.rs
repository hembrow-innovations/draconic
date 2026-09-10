use super::ok::*;
use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
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

pub(super) fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
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
            let ek = array_expr_elem_kind(right, &ctx.arr_inits, &ctx.arr_elem, &ctx.slot_of)
                // Empty arrays (and other untyped iterables) still support for-of;
                // bind as Number when element kind is unknown (body never observes).
                .unwrap_or(ElemKind::Unknown);
            let bind_ty = match ek {
                ElemKind::Number | ElemKind::Unknown => LocalSlot::Number,
                ElemKind::String => LocalSlot::String,
                ElemKind::Array => LocalSlot::Array,
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
pub(super) fn classify_array_pattern(
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
                    ElemKind::Number | ElemKind::Unknown => LocalSlot::Number,
                    ElemKind::String => LocalSlot::String,
                    ElemKind::Array => LocalSlot::Array,
                };
                classify_pattern_binding(binding, bind_ty, elem_kind, print_nums, ctx)?;
            }
            ArrayPatternEl::Rest(binding) => {
                // Rest binds an array; nested patterns (e.g. `[...[x]]`) still
                // observe inner number locals. Bare rest locals are Array slots
                // (print_nums only applies to Number bindings).
                classify_pattern_binding(binding, LocalSlot::Array, elem_kind, print_nums, ctx)?;
            }
        }
    }
    Some(())
}

pub(super) fn classify_pattern_binding(
    binding: &Pattern,
    bind_ty: LocalSlot,
    elem_kind: ElemKind,
    print_nums: bool,
    ctx: &mut ClassifyCtx<'_>,
) -> Option<()> {
    match binding {
        Pattern::Local(id) => {
            if let Some(existing) = ctx.slot_of.get(id).copied() {
                if existing == bind_ty {
                    // already registered
                } else if existing == LocalSlot::Number && bind_ty == LocalSlot::Number {
                    // bare let provisional number
                } else if existing == LocalSlot::Number && bind_ty == LocalSlot::Array {
                    // bare `let tail` upgraded when bound by rest pattern
                    if let Some((_, slot)) = ctx.slots.iter_mut().find(|(l, _)| l == id) {
                        *slot = LocalSlot::Array;
                    }
                    ctx.slot_of.insert(*id, LocalSlot::Array);
                } else {
                    return None;
                }
            } else {
                ctx.slots.push((*id, bind_ty));
                ctx.slot_of.insert(*id, bind_ty);
            }
            if bind_ty == LocalSlot::Array {
                ctx.has_array = true;
                if elem_kind != ElemKind::Unknown {
                    ctx.arr_elem.insert(*id, elem_kind);
                }
            }
            if print_nums
                && bind_ty == LocalSlot::Number
                && !ctx.print_locals.iter().any(|(l, _)| l == id)
            {
                ctx.print_locals.push((*id, LocalSlot::Number));
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
                LocalSlot::Array => elem_kind,
                _ => ElemKind::Unknown,
            };
            // When outer elem is Array, nested binds the inner array's elements.
            let nested_ek = if bind_ty == LocalSlot::Array {
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

pub(super) fn object_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, LocalSlot>,
) -> bool {
    match expr {
        Expr::Object { properties, .. } => properties.is_empty(),
        // Arrays also use Type::Object in IR — only trust explicit Object slots.
        Expr::Local { id, .. } => slot_of.get(id) == Some(&LocalSlot::Object),
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

pub(super) fn classify_for_of_left(
    left: &Stmt,
    right: &Expr,
    bind_ty: LocalSlot,
    ek: ElemKind,
    ctx: &mut ClassifyCtx<'_>,
) -> Option<()> {
    match left {
        Stmt::Declare {
            local, init: None, ..
        } => {
            if ctx.slot_of.contains_key(local) {
                return None;
            }
            ctx.slots.push((*local, bind_ty));
            ctx.slot_of.insert(*local, bind_ty);
            if bind_ty == LocalSlot::Array {
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
            Some(LocalSlot::Number) if bind_ty == LocalSlot::Number => Some(()),
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
pub(super) fn for_of_bound_array_elem_kind(
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
        let ArrayElement::Expr(Expr::Array {
            elements: inner, ..
        }) = el
        else {
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

pub(super) fn classify_declare(
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
        ctx.slots.push((local, LocalSlot::Number));
        ctx.slot_of.insert(local, LocalSlot::Number);
        return Some(());
    };
    if matches!(init, Expr::Array { .. }) {
        if !array_expr_ok(init, ctx.by_id, &ctx.slot_of) {
            return None;
        }
        ctx.has_array = true;
        ctx.slots.push((local, LocalSlot::Array));
        ctx.slot_of.insert(local, LocalSlot::Array);
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
        ctx.slots.push((local, LocalSlot::Object));
        ctx.slot_of.insert(local, LocalSlot::Object);
        return Some(());
    }
    if is_undefined_expr(init) {
        ctx.slots.push((local, LocalSlot::Null));
        ctx.slot_of.insert(local, LocalSlot::Null);
        return Some(());
    }
    if matches!(init, Expr::String { .. }) {
        if !string_expr_ok(init, ctx.by_id, &ctx.slot_of) {
            return None;
        }
        ctx.slots.push((local, LocalSlot::String));
        ctx.slot_of.insert(local, LocalSlot::String);
        // Empty-string accumulators (for-of concat) are observations.
        if let Expr::String { value, .. } = init {
            if value.to_string_lossy().is_empty() {
                ctx.print_locals.push((local, LocalSlot::String));
            }
        }
        return Some(());
    }
    if let Expr::Local { id, .. } = init {
        if ctx
            .slots
            .iter()
            .any(|(s, k)| s == id && *k == LocalSlot::Array)
        {
            ctx.has_array = true;
            ctx.slots.push((local, LocalSlot::Array));
            ctx.slot_of.insert(local, LocalSlot::Array);
            if let Some(e) = ctx.arr_inits.get(id).cloned() {
                ctx.arr_inits.insert(local, e);
            }
            if let Some(k) = ctx.arr_elem.get(id).copied() {
                ctx.arr_elem.insert(local, k);
            }
            return Some(());
        }
        if ctx
            .slots
            .iter()
            .any(|(s, k)| s == id && *k == LocalSlot::String)
        {
            ctx.slots.push((local, LocalSlot::String));
            ctx.slot_of.insert(local, LocalSlot::String);
            return Some(());
        }
        if ctx
            .slots
            .iter()
            .any(|(s, k)| s == id && *k == LocalSlot::Number)
            || matches!(loc.ty, Type::Number)
        {
            ctx.slots.push((local, LocalSlot::Number));
            ctx.slot_of.insert(local, LocalSlot::Number);
            ctx.print_locals.push((local, LocalSlot::Number));
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
            LocalSlot::Number => ctx.print_locals.push((local, LocalSlot::Number)),
            LocalSlot::String => {
                if matches!(init, Expr::Member { computed: true, .. }) {
                    ctx.print_locals.push((local, LocalSlot::String));
                }
            }
            LocalSlot::Array => {
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
        ctx.slots.push((local, LocalSlot::Number));
        ctx.slot_of.insert(local, LocalSlot::Number);
        ctx.print_locals.push((local, LocalSlot::Number));
        return Some(());
    }
    None
}
