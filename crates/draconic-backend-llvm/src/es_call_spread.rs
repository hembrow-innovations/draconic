//! N08.06.04: native observations for spread in call/`new` args (E06.04 /
//! `es/arrays/call_spread`).
//!
//! Combines array heap values (Runtime `array_*`), plain function decls
//! (number or string return), and simple constructors (`this.prop =` + `new`).
//! Spread args expand statically from known array inits (fixture arrays are not
//! mutated).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Param, Pattern,
    Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, CSTR_CONCAT, GC_INIT,
    OBJECT_GET, OBJECT_SET, OBJECT_SET_PROTO, PRINT_F64, PRINT_STR,
};

const MAX_ARGS: usize = 8;

pub(crate) fn is_es_call_spread_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_call_spread(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_call_spread module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    String,
    Array,
    Object,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FnKind {
    /// `double (double a0..)` number params, number return.
    Number,
    /// `ptr (ptr a0..)` string params, string return.
    String,
    /// `double (ptr this, double a0..)` ctor body sets `this` props.
    Ctor,
}

#[derive(Clone)]
struct FnInfo {
    idx: usize,
    params: Vec<LocalId>,
    body: Vec<Stmt>,
    kind: FnKind,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    /// Observation prints in declare order (numbers + strings).
    print_locals: Vec<(LocalId, SlotTy)>,
    functions: Vec<FnInfo>,
    fn_binding: HashMap<LocalId, usize>,
    /// Array local → literal init expr (for static spread expansion).
    arr_inits: HashMap<LocalId, Expr>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut fn_binding = HashMap::new();
    collect_fns(&module.body, &by_id, &mut functions, &mut fn_binding)?;

    // First pass: array inits only (needed to expand spreads when refining kinds).
    let mut arr_inits: HashMap<LocalId, Expr> = HashMap::new();
    let mut slot_of: HashMap<LocalId, SlotTy> = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare {
            local,
            init: Some(init),
            ..
        } = stmt
        {
            if matches!(init, Expr::Array { .. }) {
                if !array_expr_ok(init, &slot_of) {
                    return None;
                }
                slot_of.insert(*local, SlotTy::Array);
                arr_inits.insert(*local, init.clone());
            }
        }
    }

    // Refine Number vs String from call-site argument kinds (untyped params are `any`).
    refine_fn_kinds_from_calls(&module.body, &mut functions, &fn_binding, &arr_inits, &slot_of)?;

    let mut has_spread_call = false;
    let mut slots = Vec::new();
    let mut print_locals = Vec::new();
    // Reset slot_of; rebuild in program order.
    slot_of.clear();

    for stmt in &module.body {
        match stmt {
            Stmt::Function { local, .. } => {
                let idx = *fn_binding.get(local)?;
                if functions[idx].kind == FnKind::Ctor {
                    slots.push((*local, SlotTy::Object));
                    slot_of.insert(*local, SlotTy::Object);
                }
            }
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                let init = init.as_ref()?;
                if matches!(init, Expr::Array { .. }) {
                    if !array_expr_ok(init, &slot_of) {
                        return None;
                    }
                    slots.push((*local, SlotTy::Array));
                    slot_of.insert(*local, SlotTy::Array);
                    arr_inits.insert(*local, init.clone());
                } else if let Some(kind) = infer_init_slot(init, &slot_of, &fn_binding, &functions)
                {
                    if !value_expr_ok(init, &slot_of, &fn_binding, &functions, &arr_inits) {
                        return None;
                    }
                    if call_or_new_has_spread(init) {
                        has_spread_call = true;
                    }
                    slots.push((*local, kind));
                    slot_of.insert(*local, kind);
                    match kind {
                        SlotTy::Number | SlotTy::String => print_locals.push((*local, kind)),
                        SlotTy::Array => {
                            if let Expr::Array { .. } = init {
                                arr_inits.insert(*local, init.clone());
                            }
                        }
                        SlotTy::Object => {}
                    }
                } else if matches!(loc.ty, Type::Number | Type::Any)
                    && number_expr_ok(init, &slot_of, &fn_binding, &functions, &arr_inits)
                {
                    if call_or_new_has_spread(init) {
                        has_spread_call = true;
                    }
                    slots.push((*local, SlotTy::Number));
                    slot_of.insert(*local, SlotTy::Number);
                    print_locals.push((*local, SlotTy::Number));
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    if !has_spread_call || print_locals.is_empty() || functions.is_empty() {
        return None;
    }
    for f in &functions {
        if !fn_body_ok(f, &by_id) {
            return None;
        }
    }
    Some(ModuleInfo {
        slots,
        print_locals,
        functions,
        fn_binding,
        arr_inits,
    })
}

fn collect_fns(
    stmts: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    out: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
) -> Option<()> {
    for stmt in stmts {
        if let Stmt::Function {
            local,
            params,
            body,
            is_async,
            is_generator,
        } = stmt
        {
            if *is_async || *is_generator {
                return None;
            }
            let param_ids = simple_param_ids(params, by_id)?;
            let kind = classify_fn_kind(body)?;
            let idx = out.len();
            out.push(FnInfo {
                idx,
                params: param_ids,
                body: body.clone(),
                kind,
            });
            fn_binding.insert(*local, idx);
        }
    }
    Some(())
}

/// Ctor stays Ctor; plain functions start as Number and become String if any
/// call site passes a string (after static spread expansion).
fn refine_fn_kinds_from_calls(
    body: &[Stmt],
    functions: &mut [FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    arr_inits: &HashMap<LocalId, Expr>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<()> {
    for stmt in body {
        let init = match stmt {
            Stmt::Declare { init: Some(e), .. } => e,
            _ => continue,
        };
        walk_calls_for_kind(init, functions, fn_binding, arr_inits, slot_of)?;
    }
    Some(())
}

fn walk_calls_for_kind(
    expr: &Expr,
    functions: &mut [FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    arr_inits: &HashMap<LocalId, Expr>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<()> {
    match expr {
        Expr::Call {
            callee, args, optional, ..
        } => {
            if *optional {
                return None;
            }
            walk_calls_for_kind(callee, functions, fn_binding, arr_inits, slot_of)?;
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => {
                        walk_calls_for_kind(e, functions, fn_binding, arr_inits, slot_of)?;
                    }
                }
            }
            let Expr::Local { id, .. } = callee.as_ref() else {
                return None;
            };
            let &idx = fn_binding.get(id)?;
            if functions[idx].kind == FnKind::Ctor {
                return Some(());
            }
            let expanded = expand_args_static(args, arr_inits, slot_of)?;
            let any_string = expanded.iter().any(|e| expr_is_stringish(e, slot_of, arr_inits));
            if any_string {
                functions[idx].kind = FnKind::String;
            }
            Some(())
        }
        Expr::New {
            callee, args, ..
        } => {
            walk_calls_for_kind(callee, functions, fn_binding, arr_inits, slot_of)?;
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => {
                        walk_calls_for_kind(e, functions, fn_binding, arr_inits, slot_of)?;
                    }
                }
            }
            Some(())
        }
        Expr::Binary { left, right, .. } => {
            walk_calls_for_kind(left, functions, fn_binding, arr_inits, slot_of)?;
            walk_calls_for_kind(right, functions, fn_binding, arr_inits, slot_of)
        }
        Expr::Member {
            object, property, ..
        } => {
            walk_calls_for_kind(object, functions, fn_binding, arr_inits, slot_of)?;
            walk_calls_for_kind(property, functions, fn_binding, arr_inits, slot_of)
        }
        Expr::Array { elements, .. } => {
            for el in elements {
                if let ArrayElement::Expr(e) | ArrayElement::Spread(e) = el {
                    walk_calls_for_kind(e, functions, fn_binding, arr_inits, slot_of)?;
                }
            }
            Some(())
        }
        _ => Some(()),
    }
}

fn expr_is_stringish(
    expr: &Expr,
    slot_of: &HashMap<LocalId, SlotTy>,
    arr_inits: &HashMap<LocalId, Expr>,
) -> bool {
    match expr {
        Expr::String { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::String) || slot_of.get(id) == Some(&SlotTy::String)
        }
        Expr::Binary {
            op: BinaryOp::Add,
            left,
            right,
            ..
        } => expr_is_stringish(left, slot_of, arr_inits) || expr_is_stringish(right, slot_of, arr_inits),
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } if !*optional && *computed => {
            // chars[i] from string array — treat element of string array as string.
            if let Some(elems) = resolve_array_elems(object, arr_inits, slot_of) {
                if let Some(idx) = const_index(property) {
                    return elems
                        .get(idx)
                        .is_some_and(|e| expr_is_stringish(e, slot_of, arr_inits));
                }
            }
            false
        }
        _ => false,
    }
}

fn simple_param_ids(params: &[Param], by_id: &HashMap<LocalId, &Local>) -> Option<Vec<LocalId>> {
    let mut ids = Vec::new();
    for p in params {
        if p.default.is_some() || p.rest {
            return None;
        }
        match &p.pattern {
            Pattern::Local(id) => {
                let _ = by_id.get(id)?;
                ids.push(*id);
            }
            _ => return None,
        }
    }
    if ids.len() > MAX_ARGS {
        return None;
    }
    Some(ids)
}

fn classify_fn_kind(body: &[Stmt]) -> Option<FnKind> {
    let mut has_this_assign = false;
    let mut has_return = false;
    let mut ret_string = false;
    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr:
                    Expr::Assign {
                        target:
                            AssignTarget::Member {
                                object,
                                property,
                                computed: false,
                                ..
                            },
                        op: AssignOp::Eq,
                        ..
                    },
            } => {
                if !matches!(object.as_ref(), Expr::This { .. }) {
                    return None;
                }
                if !matches!(property.as_ref(), Expr::String { .. }) {
                    return None;
                }
                has_this_assign = true;
            }
            Stmt::Return { value: Some(e) } => {
                has_return = true;
                ret_string = expr_looks_string(e);
            }
            Stmt::Return { value: None } => has_return = true,
            Stmt::Block { body: inner } => {
                let k = classify_fn_kind(inner)?;
                match k {
                    FnKind::Ctor => has_this_assign = true,
                    FnKind::String => {
                        has_return = true;
                        ret_string = true;
                    }
                    FnKind::Number => has_return = true,
                }
            }
            _ => return None,
        }
    }
    if has_this_assign && !has_return {
        Some(FnKind::Ctor)
    } else if has_return && ret_string && !has_this_assign {
        Some(FnKind::String)
    } else if has_return && !has_this_assign {
        Some(FnKind::Number)
    } else {
        None
    }
}

fn expr_looks_string(expr: &Expr) -> bool {
    match expr {
        Expr::String { .. } => true,
        Expr::Local { ty, .. } => matches!(ty, Type::String),
        Expr::Binary {
            op: BinaryOp::Add,
            left,
            right,
            ..
        } => expr_looks_string(left) || expr_looks_string(right),
        _ => false,
    }
}

fn fn_body_ok(f: &FnInfo, by_id: &HashMap<LocalId, &Local>) -> bool {
    let params: std::collections::HashSet<_> = f.params.iter().copied().collect();
    body_ok(&f.body, f.kind, &params, by_id)
}

fn body_ok(
    body: &[Stmt],
    kind: FnKind,
    params: &std::collections::HashSet<LocalId>,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    for stmt in body {
        match stmt {
            Stmt::Return { value: Some(e) } => {
                if kind == FnKind::Ctor {
                    return false;
                }
                if !body_expr_ok(e, kind, params, by_id) {
                    return false;
                }
            }
            Stmt::Return { value: None } => {
                if kind == FnKind::Ctor {
                    return false;
                }
            }
            Stmt::Expr {
                expr:
                    Expr::Assign {
                        target:
                            AssignTarget::Member {
                                object,
                                property,
                                computed: false,
                                ..
                            },
                        op: AssignOp::Eq,
                        value,
                        ..
                    },
            } => {
                if kind != FnKind::Ctor {
                    return false;
                }
                if !matches!(object.as_ref(), Expr::This { .. }) {
                    return false;
                }
                if !matches!(property.as_ref(), Expr::String { .. }) {
                    return false;
                }
                if !body_expr_ok(value, FnKind::Number, params, by_id) {
                    return false;
                }
            }
            Stmt::Block { body: inner } => {
                if !body_ok(inner, kind, params, by_id) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

fn body_expr_ok(
    expr: &Expr,
    kind: FnKind,
    params: &std::collections::HashSet<LocalId>,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    match expr {
        Expr::Number { .. } => kind == FnKind::Number || kind == FnKind::Ctor,
        Expr::String { .. } => kind == FnKind::String,
        Expr::Local { id, ty } => {
            if !params.contains(id) {
                return false;
            }
            // Untyped params are `any`; allow both number and string fn bodies.
            match kind {
                FnKind::String => {
                    matches!(ty, Type::String | Type::Any)
                        || by_id
                            .get(id)
                            .is_some_and(|l| matches!(l.ty, Type::String | Type::Any))
                }
                FnKind::Number | FnKind::Ctor => {
                    matches!(ty, Type::Number | Type::Any)
                        || by_id
                            .get(id)
                            .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
                }
            }
        }
        Expr::Binary {
            op: BinaryOp::Add,
            left,
            right,
            ..
        } => body_expr_ok(left, kind, params, by_id) && body_expr_ok(right, kind, params, by_id),
        _ => false,
    }
}

fn call_or_new_has_spread(expr: &Expr) -> bool {
    match expr {
        Expr::Call { args, .. } | Expr::New { args, .. } => {
            args.iter().any(|a| matches!(a, Arg::Spread(_)))
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => call_or_new_has_spread(e),
                })
        }
        Expr::Binary { left, right, .. } => {
            call_or_new_has_spread(left) || call_or_new_has_spread(right)
        }
        Expr::Member { object, property, .. } => {
            call_or_new_has_spread(object) || call_or_new_has_spread(property)
        }
        _ => false,
    }
}

fn infer_init_slot(
    init: &Expr,
    slot_of: &HashMap<LocalId, SlotTy>,
    fn_binding: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
) -> Option<SlotTy> {
    match init {
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Array { .. } => Some(SlotTy::Array),
        Expr::New { .. } => Some(SlotTy::Object),
        Expr::Call { callee, .. } => {
            let Expr::Local { id, .. } = callee.as_ref() else {
                return None;
            };
            let idx = *fn_binding.get(id)?;
            match functions[idx].kind {
                FnKind::Number => Some(SlotTy::Number),
                FnKind::String => Some(SlotTy::String),
                FnKind::Ctor => None,
            }
        }
        Expr::Member {
            object,
            computed: false,
            ..
        } => {
            if matches!(
                object.as_ref(),
                Expr::Local { id, .. } if slot_of.get(id) == Some(&SlotTy::Object)
            ) || matches!(object.as_ref(), Expr::New { .. })
            {
                Some(SlotTy::Number)
            } else {
                None
            }
        }
        Expr::Local { id, .. } => slot_of.get(id).copied(),
        _ => None,
    }
}

fn value_expr_ok(
    expr: &Expr,
    slot_of: &HashMap<LocalId, SlotTy>,
    fn_binding: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    arr_inits: &HashMap<LocalId, Expr>,
) -> bool {
    number_expr_ok(expr, slot_of, fn_binding, functions, arr_inits)
        || string_expr_ok(expr, slot_of, fn_binding, functions, arr_inits)
        || array_expr_ok(expr, slot_of)
        || object_expr_ok(expr, slot_of, fn_binding, functions, arr_inits)
}

fn number_expr_ok(
    expr: &Expr,
    slot_of: &HashMap<LocalId, SlotTy>,
    fn_binding: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    arr_inits: &HashMap<LocalId, Expr>,
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, .. } => slot_of.get(id) == Some(&SlotTy::Number),
        Expr::Binary {
            op: BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem,
            left,
            right,
            ..
        } => {
            number_expr_ok(left, slot_of, fn_binding, functions, arr_inits)
                && number_expr_ok(right, slot_of, fn_binding, functions, arr_inits)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return false;
            }
            let Expr::Local { id, .. } = callee.as_ref() else {
                return false;
            };
            let Some(&idx) = fn_binding.get(id) else {
                return false;
            };
            if functions[idx].kind != FnKind::Number {
                return false;
            }
            let Some(expanded) = expand_args_static(args, arr_inits, slot_of) else {
                return false;
            };
            if expanded.len() > functions[idx].params.len() {
                return false;
            }
            expanded
                .iter()
                .all(|e| number_expr_ok(e, slot_of, fn_binding, functions, arr_inits))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && !*computed
                && matches!(property.as_ref(), Expr::String { .. })
                && object_expr_ok(object, slot_of, fn_binding, functions, arr_inits)
        }
        _ => false,
    }
}

fn string_expr_ok(
    expr: &Expr,
    slot_of: &HashMap<LocalId, SlotTy>,
    fn_binding: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    arr_inits: &HashMap<LocalId, Expr>,
) -> bool {
    match expr {
        Expr::String { .. } => true,
        Expr::Local { id, .. } => slot_of.get(id) == Some(&SlotTy::String),
        Expr::Binary {
            op: BinaryOp::Add,
            left,
            right,
            ..
        } => {
            string_expr_ok(left, slot_of, fn_binding, functions, arr_inits)
                && string_expr_ok(right, slot_of, fn_binding, functions, arr_inits)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return false;
            }
            let Expr::Local { id, .. } = callee.as_ref() else {
                return false;
            };
            let Some(&idx) = fn_binding.get(id) else {
                return false;
            };
            if functions[idx].kind != FnKind::String {
                return false;
            }
            let Some(expanded) = expand_args_static(args, arr_inits, slot_of) else {
                return false;
            };
            if expanded.len() > functions[idx].params.len() {
                return false;
            }
            expanded
                .iter()
                .all(|e| string_expr_ok(e, slot_of, fn_binding, functions, arr_inits))
        }
        _ => false,
    }
}

fn array_expr_ok(expr: &Expr, slot_of: &HashMap<LocalId, SlotTy>) -> bool {
    match expr {
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => {
                matches!(e, Expr::Number { .. } | Expr::String { .. } | Expr::Array { .. })
                    || matches!(e, Expr::Local { id, .. } if slot_of.contains_key(id))
                    || array_expr_ok(e, slot_of)
            }
            ArrayElement::Elision => true,
            ArrayElement::Spread(_) => false,
        }),
        Expr::Local { id, .. } => slot_of.get(id) == Some(&SlotTy::Array),
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && *computed
                && array_expr_ok(object, slot_of)
                && matches!(property.as_ref(), Expr::Number { .. })
        }
        _ => false,
    }
}

fn object_expr_ok(
    expr: &Expr,
    slot_of: &HashMap<LocalId, SlotTy>,
    fn_binding: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    arr_inits: &HashMap<LocalId, Expr>,
) -> bool {
    match expr {
        Expr::Local { id, .. } => slot_of.get(id) == Some(&SlotTy::Object),
        Expr::New {
            callee,
            args,
            ..
        } => {
            let Expr::Local { id, .. } = callee.as_ref() else {
                return false;
            };
            let Some(&idx) = fn_binding.get(id) else {
                return false;
            };
            if functions[idx].kind != FnKind::Ctor {
                return false;
            }
            let Some(expanded) = expand_args_static(args, arr_inits, slot_of) else {
                return false;
            };
            if expanded.len() > functions[idx].params.len() {
                return false;
            }
            expanded
                .iter()
                .all(|e| number_expr_ok(e, slot_of, fn_binding, functions, arr_inits))
        }
        _ => false,
    }
}

/// Expand call/new args with static knowledge of array literal inits.
fn expand_args_static(
    args: &[Arg],
    arr_inits: &HashMap<LocalId, Expr>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<Vec<Expr>> {
    let mut out = Vec::new();
    for a in args {
        match a {
            Arg::Expr(e) => out.push(e.clone()),
            Arg::Spread(e) => {
                let elems = resolve_array_elems(e, arr_inits, slot_of)?;
                out.extend(elems);
            }
        }
    }
    if out.len() > MAX_ARGS {
        return None;
    }
    Some(out)
}

fn resolve_array_elems(
    expr: &Expr,
    arr_inits: &HashMap<LocalId, Expr>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<Vec<Expr>> {
    let lit = match expr {
        Expr::Array { .. } => expr.clone(),
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
            let idx = const_index(property)?;
            let outer = match object.as_ref() {
                Expr::Local { id, .. } => arr_inits.get(id).cloned()?,
                Expr::Array { .. } => object.as_ref().clone(),
                _ => return None,
            };
            let Expr::Array { elements, .. } = outer else {
                return None;
            };
            match elements.get(idx)? {
                ArrayElement::Expr(e) => e.clone(),
                _ => return None,
            }
        }
        _ => return None,
    };
    let Expr::Array { elements, .. } = lit else {
        return None;
    };
    let mut out = Vec::new();
    for el in elements {
        match el {
            ArrayElement::Expr(e) => out.push(e),
            ArrayElement::Elision => out.push(Expr::Number {
                raw: "0".into(),
                ty: Type::Number,
            }),
            ArrayElement::Spread(_) => return None,
        }
    }
    let _ = slot_of;
    Some(out)
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

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    out: String,
    body: String,
    allocas: HashMap<LocalId, String>,
    slot_of: HashMap<LocalId, SlotTy>,
    param_allocas: HashMap<LocalId, String>,
    this_ssa: Option<String>,
    str_globals: Vec<(String, String)>,
    tmp: usize,
    str_n: usize,
    /// When emitting a string fn, return ptr not double.
    in_string_fn: bool,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            allocas: HashMap::new(),
            slot_of: HashMap::new(),
            param_allocas: HashMap::new(),
            this_ssa: None,
            str_globals: Vec::new(),
            tmp: 0,
            str_n: 0,
            in_string_fn: false,
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
            "; Draconic LLVM backend (N08.06.04 call/new spread via Runtime ABI)"
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
                OBJECT_SET,
                OBJECT_GET,
                OBJECT_SET_PROTO,
                CSTR_CONCAT,
                PRINT_F64,
                PRINT_STR,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for (id, kind) in &info.slots {
            match kind {
                SlotTy::Number => {
                    let g = format!("es_cs_n{}", id.0);
                    writeln!(
                        self.out,
                        "@{g} = internal global double 0.00000000000000000e+00, align 8"
                    )
                    .ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::String | SlotTy::Array | SlotTy::Object => {
                    let tag = match kind {
                        SlotTy::String => "s",
                        SlotTy::Array => "a",
                        SlotTy::Object => "o",
                        SlotTy::Number => "n",
                    };
                    let g = format!("es_cs_{tag}{}", id.0);
                    writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
            }
        }
        if !info.slots.is_empty() {
            writeln!(self.out).ok();
        }

        for f in &info.functions.clone() {
            self.emit_fn(f)?;
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

    fn emit_fn(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let name = format!("cs_fn_{}", f.idx);
        let saved_body = std::mem::take(&mut self.body);
        let saved_this = self.this_ssa.take();
        let saved_params = std::mem::take(&mut self.param_allocas);
        let saved_in_str = self.in_string_fn;

        match f.kind {
            FnKind::Number => {
                self.in_string_fn = false;
                let mut params_s = String::new();
                for i in 0..f.params.len() {
                    if i > 0 {
                        params_s.push_str(", ");
                    }
                    write!(params_s, "double %a{i}").ok();
                }
                writeln!(self.out, "define double @{name}({params_s}) {{").ok();
                writeln!(self.out, "entry:").ok();
                for (i, pid) in f.params.iter().enumerate() {
                    let ptr = format!("%p{}", pid.0);
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                    writeln!(self.body, "  store double %a{i}, ptr {ptr}").ok();
                    self.param_allocas.insert(*pid, ptr);
                }
                let mut saw_ret = false;
                for stmt in &f.body {
                    if matches!(stmt, Stmt::Return { .. }) {
                        saw_ret = true;
                    }
                    self.emit_fn_stmt(stmt, f.kind)?;
                }
                if !saw_ret {
                    writeln!(
                        self.body,
                        "  ret double 0.00000000000000000e+00"
                    )
                    .ok();
                }
            }
            FnKind::String => {
                self.in_string_fn = true;
                let mut params_s = String::new();
                for i in 0..f.params.len() {
                    if i > 0 {
                        params_s.push_str(", ");
                    }
                    write!(params_s, "ptr %a{i}").ok();
                }
                writeln!(self.out, "define ptr @{name}({params_s}) {{").ok();
                writeln!(self.out, "entry:").ok();
                for (i, pid) in f.params.iter().enumerate() {
                    let ptr = format!("%p{}", pid.0);
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr %a{i}, ptr {ptr}").ok();
                    self.param_allocas.insert(*pid, ptr);
                }
                let mut saw_ret = false;
                for stmt in &f.body {
                    if matches!(stmt, Stmt::Return { .. }) {
                        saw_ret = true;
                    }
                    self.emit_fn_stmt(stmt, f.kind)?;
                }
                if !saw_ret {
                    writeln!(self.body, "  ret ptr null").ok();
                }
            }
            FnKind::Ctor => {
                self.in_string_fn = false;
                let mut params_s = String::from("ptr %this");
                for i in 0..f.params.len() {
                    write!(params_s, ", double %a{i}").ok();
                }
                writeln!(self.out, "define double @{name}({params_s}) {{").ok();
                writeln!(self.out, "entry:").ok();
                self.this_ssa = Some("%this".to_string());
                for (i, pid) in f.params.iter().enumerate() {
                    let ptr = format!("%p{}", pid.0);
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                    writeln!(self.body, "  store double %a{i}, ptr {ptr}").ok();
                    self.param_allocas.insert(*pid, ptr);
                }
                for stmt in &f.body {
                    self.emit_fn_stmt(stmt, f.kind)?;
                }
                writeln!(
                    self.body,
                    "  ret double 0.00000000000000000e+00"
                )
                .ok();
            }
        }

        self.out.push_str(&self.body);
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.this_ssa = saved_this;
        self.param_allocas = saved_params;
        self.in_string_fn = saved_in_str;
        Ok(())
    }

    fn emit_fn_stmt(&mut self, stmt: &Stmt, kind: FnKind) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(e) } => match kind {
                FnKind::Number => {
                    let v = self.emit_number_expr(e)?;
                    writeln!(self.body, "  ret double {v}").ok();
                    Ok(())
                }
                FnKind::String => {
                    let v = self.emit_string_expr(e)?;
                    writeln!(self.body, "  ret ptr {v}").ok();
                    Ok(())
                }
                FnKind::Ctor => Err(diag("es_call_spread: ctor must not return value")),
            },
            Stmt::Return { value: None } => {
                match kind {
                    FnKind::Number => writeln!(
                        self.body,
                        "  ret double 0.00000000000000000e+00"
                    )
                    .ok(),
                    FnKind::String => writeln!(self.body, "  ret ptr null").ok(),
                    FnKind::Ctor => {
                        return Err(diag("es_call_spread: bare return in ctor"));
                    }
                };
                Ok(())
            }
            Stmt::Expr {
                expr:
                    Expr::Assign {
                        target:
                            AssignTarget::Member {
                                object,
                                property,
                                computed: false,
                                ..
                            },
                        op: AssignOp::Eq,
                        value,
                        ..
                    },
            } => {
                if !matches!(object.as_ref(), Expr::This { .. }) {
                    return Err(diag("es_call_spread: only this.prop assign in ctor"));
                }
                let this = self
                    .this_ssa
                    .clone()
                    .ok_or_else(|| diag("es_call_spread: this outside ctor"))?;
                let key = match property.as_ref() {
                    Expr::String { value, .. } => self.string_const(&value.to_string_lossy())?,
                    _ => return Err(diag("es_call_spread: prop key must be string")),
                };
                let n = self.emit_number_expr(value)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                let p = self.fresh();
                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {this}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_fn_stmt(s, kind)?;
                }
                Ok(())
            }
            _ => Err(diag("es_call_spread: unsupported fn stmt")),
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Function { local, .. } => {
                let idx = *self
                    .info
                    .fn_binding
                    .get(local)
                    .ok_or_else(|| diag("es_call_spread: unknown fn"))?;
                if self.info.functions[idx].kind != FnKind::Ctor {
                    return Ok(());
                }
                // Ctor object with empty .prototype (same as es_objects).
                let ctor = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&ctor, "")).ok();
                let proto = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&proto, "")).ok();
                let key = self.string_const("prototype")?;
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {ctor}, ptr {key}, ptr {proto}"))
                )
                .ok();
                let ptr = self.slot_ptr(*local)?;
                writeln!(self.body, "  store ptr {ctor}, ptr {ptr}").ok();
                Ok(())
            }
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                let kind = *self
                    .slot_of
                    .get(local)
                    .ok_or_else(|| diag("es_call_spread: declare unknown slot"))?;
                let ptr = self.slot_ptr(*local)?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
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
            _ => Err(diag("es_call_spread: unsupported top-level stmt")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => format_number_const(raw),
            Expr::Local { id, .. } => {
                if let Some(ptr) = self.param_allocas.get(id).cloned() {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                    return Ok(t);
                }
                if self.slot_of.get(id) != Some(&SlotTy::Number) {
                    return Err(diag("es_call_spread: expected number local"));
                }
                let ptr = self.slot_ptr(*id)?;
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
                    _ => return Err(diag("es_call_spread: unsupported binary")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_call_spread: optional call"));
                }
                self.emit_number_call(callee, args)
            }
            Expr::Member {
                object,
                property,
                optional,
                computed,
                ..
            } => {
                if *optional || *computed {
                    return Err(diag("es_call_spread: only static prop get"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = match property.as_ref() {
                    Expr::String { value, .. } => self.string_const(&value.to_string_lossy())?,
                    _ => return Err(diag("es_call_spread: prop key string")),
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
                Ok(d)
            }
            _ => Err(diag("es_call_spread: unsupported number expr")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                if let Some(ptr) = self.param_allocas.get(id).cloned() {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                    return Ok(t);
                }
                if self.slot_of.get(id) != Some(&SlotTy::String) {
                    return Err(diag("es_call_spread: expected string local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Binary {
                op: BinaryOp::Add,
                left,
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
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_call_spread: optional call"));
                }
                self.emit_string_call(callee, args)
            }
            _ => Err(diag("es_call_spread: unsupported string expr")),
        }
    }

    fn emit_number_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_call_spread: call callee must be local"));
        };
        let idx = *self
            .info
            .fn_binding
            .get(id)
            .ok_or_else(|| diag("es_call_spread: unbound callee"))?;
        let f = &self.info.functions[idx];
        if f.kind != FnKind::Number {
            return Err(diag("es_call_spread: expected number fn"));
        }
        let expanded = expand_args_static(args, &self.info.arr_inits, &self.slot_of)
            .ok_or_else(|| diag("es_call_spread: cannot expand spread args"))?;
        if expanded.len() > f.params.len() {
            return Err(diag("es_call_spread: too many args"));
        }
        let mut arg_vals = Vec::new();
        for e in &expanded {
            arg_vals.push(self.emit_number_expr(e)?);
        }
        while arg_vals.len() < f.params.len() {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }
        let parts: Vec<_> = arg_vals.iter().map(|v| format!("double {v}")).collect();
        let t = self.fresh();
        if parts.is_empty() {
            writeln!(self.body, "  {t} = call double @cs_fn_{idx}()").ok();
        } else {
            writeln!(
                self.body,
                "  {t} = call double @cs_fn_{idx}({})",
                parts.join(", ")
            )
            .ok();
        }
        Ok(t)
    }

    fn emit_string_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_call_spread: call callee must be local"));
        };
        let idx = *self
            .info
            .fn_binding
            .get(id)
            .ok_or_else(|| diag("es_call_spread: unbound callee"))?;
        let f = &self.info.functions[idx];
        if f.kind != FnKind::String {
            return Err(diag("es_call_spread: expected string fn"));
        }
        let expanded = expand_args_static(args, &self.info.arr_inits, &self.slot_of)
            .ok_or_else(|| diag("es_call_spread: cannot expand spread args"))?;
        if expanded.len() > f.params.len() {
            return Err(diag("es_call_spread: too many args"));
        }
        let mut arg_vals = Vec::new();
        for e in &expanded {
            arg_vals.push(self.emit_string_expr(e)?);
        }
        while arg_vals.len() < f.params.len() {
            arg_vals.push("null".to_string());
        }
        let parts: Vec<_> = arg_vals.iter().map(|v| format!("ptr {v}")).collect();
        let t = self.fresh();
        if parts.is_empty() {
            writeln!(self.body, "  {t} = call ptr @cs_fn_{idx}()").ok();
        } else {
            writeln!(
                self.body,
                "  {t} = call ptr @cs_fn_{idx}({})",
                parts.join(", ")
            )
            .ok();
        }
        Ok(t)
    }

    fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => {
                if self.slot_of.get(id) != Some(&SlotTy::Object) {
                    return Err(diag("es_call_spread: expected object local"));
                }
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::New { callee, args, .. } => self.emit_new(callee, args),
            _ => Err(diag("es_call_spread: unsupported object expr")),
        }
    }

    fn emit_new(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_call_spread: new callee must be local"));
        };
        let idx = *self
            .info
            .fn_binding
            .get(id)
            .ok_or_else(|| diag("es_call_spread: unknown ctor"))?;
        if self.info.functions[idx].kind != FnKind::Ctor {
            return Err(diag("es_call_spread: not a ctor"));
        }
        let ctor_ptr = self.slot_ptr(*id)?;
        let ctor = self.fresh();
        writeln!(self.body, "  {ctor} = load ptr, ptr {ctor_ptr}").ok();
        let proto_key = self.string_const("prototype")?;
        let proto = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&proto, &format!("ptr {ctor}, ptr {proto_key}"))
        )
        .ok();
        let obj = self.fresh();
        writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
        writeln!(
            self.body,
            "  {}",
            OBJECT_SET_PROTO.call(&format!("ptr {obj}, ptr {proto}"))
        )
        .ok();

        let expanded = expand_args_static(args, &self.info.arr_inits, &self.slot_of)
            .ok_or_else(|| diag("es_call_spread: cannot expand new spread"))?;
        let n_params = self.info.functions[idx].params.len();
        if expanded.len() > n_params {
            return Err(diag("es_call_spread: too many new args"));
        }
        let mut arg_vals = Vec::new();
        for e in &expanded {
            arg_vals.push(self.emit_number_expr(e)?);
        }
        while arg_vals.len() < n_params {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }
        let mut call_args = format!("ptr {obj}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        let ret = self.fresh();
        writeln!(
            self.body,
            "  {ret} = call double @cs_fn_{idx}({call_args})"
        )
        .ok();
        let _ = ret;
        Ok(obj)
    }

    fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Array { elements, .. } => self.emit_array_lit(elements),
            Expr::Local { id, .. } => {
                if self.slot_of.get(id) != Some(&SlotTy::Array) {
                    return Err(diag("es_call_spread: expected array local"));
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
                    return Err(diag("es_call_spread: nested array via index only"));
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
            _ => Err(diag("es_call_spread: unsupported array expr")),
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
                    return Err(diag("es_call_spread: array lit spread not in this path"));
                }
                ArrayElement::Expr(e) => {
                    let v = if matches!(e, Expr::Array { .. })
                        || matches!(e, Expr::Local { id, .. } if self.slot_of.get(id) == Some(&SlotTy::Array))
                    {
                        self.emit_array_expr(e)?
                    } else if matches!(e, Expr::String { .. })
                        || matches!(e, Expr::Local { id, .. } if self.slot_of.get(id) == Some(&SlotTy::String))
                    {
                        self.emit_string_expr(e)?
                    } else {
                        let n = self.emit_number_expr(e)?;
                        let i64v = self.fresh();
                        writeln!(self.body, "  {i64v} = fptosi double {n} to i64").ok();
                        let p = self.fresh();
                        writeln!(self.body, "  {p} = inttoptr i64 {i64v} to ptr").ok();
                        p
                    };
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

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        self.allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("es_call_spread: slot missing"))
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_cs_str.{}", self.str_n);
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
