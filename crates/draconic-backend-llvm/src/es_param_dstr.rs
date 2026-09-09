//! N08.16.25: native observations for parameter destructuring (E18.25 /
//! `es/annex-b/param_destructure`).
//!
//! Functions/arrows with object or array binding patterns as formals; nested
//! patterns, rename, rest, element/property defaults, and whole-param defaults.
//! Values: numbers as double (heap as inttoptr); objects/arrays via Runtime GC.

use std::collections::HashMap;

use draconic_ast::BinaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, ArrayPatternEl, Expr, Local, LocalId, Module, ObjectPatternEl, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, GC_INIT, OBJECT_GET,
    OBJECT_REST, OBJECT_SET, PRINT_F64,
};
#[path = "es_param_dstr_emit.rs"]
mod emit;


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

pub(crate) fn walk_es_param_dstr(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_param_dstr_module(module) {
        return None;
    }
    Some(emit_es_param_dstr(module))
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
                init:
                    Some(Expr::Function {
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
                    ObjectPatternEl::Prop {
                        binding, default, ..
                    } => {
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
                matches!(key, ObjectPropKey::Static(_))
                    && value_expr_ok(value, by_id, fn_binding, slot_of)
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
        Expr::Local { id, .. } => slot_of.get(id) == Some(&SlotTy::Number) || is_undef(*id, by_id),
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
            matches!(slot_of.get(id), Some(SlotTy::Object) | Some(SlotTy::Array))
                && if *computed {
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
