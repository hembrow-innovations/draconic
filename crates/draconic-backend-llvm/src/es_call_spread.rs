//! N08.06.04: native observations for spread in call/`new` args (E06.04 /
//! `es/arrays/call_spread`).
//!
//! Combines array heap values (Runtime `array_*`), plain function decls
//! (number or string return), and simple constructors (`this.prop =` + `new`).
//! Spread args expand statically from known array inits (fixture arrays are not
//! mutated).

use std::collections::HashMap;

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
mod emit;


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

pub(crate) fn walk_es_call_spread(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_call_spread_module(module) {
        return None;
    }
    Some(emit_es_call_spread(module))
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
    refine_fn_kinds_from_calls(
        &module.body,
        &mut functions,
        &fn_binding,
        &arr_inits,
        &slot_of,
    )?;

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
            callee,
            args,
            optional,
            ..
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
            let any_string = expanded
                .iter()
                .any(|e| expr_is_stringish(e, slot_of, arr_inits));
            if any_string {
                functions[idx].kind = FnKind::String;
            }
            Some(())
        }
        Expr::New { callee, args, .. } => {
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
        } => {
            expr_is_stringish(left, slot_of, arr_inits)
                || expr_is_stringish(right, slot_of, arr_inits)
        }
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
        Expr::Member {
            object, property, ..
        } => call_or_new_has_spread(object) || call_or_new_has_spread(property),
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
                matches!(
                    e,
                    Expr::Number { .. } | Expr::String { .. } | Expr::Array { .. }
                ) || matches!(e, Expr::Local { id, .. } if slot_of.contains_key(id))
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
        Expr::New { callee, args, .. } => {
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
