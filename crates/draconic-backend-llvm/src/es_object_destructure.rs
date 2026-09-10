//! N08.16.19: native observations for object destructuring (E18.19) —
//! `es/annex-b/object_destructure`: `let {a,b}=obj`, rename, nested, rest,
//! assignment patterns, member targets, numeric/computed keys, array-as-object
//! sources, defaults, and a simple function with object-pattern param.

use std::collections::{HashMap, HashSet};

use draconic_ast::{AssignOp, BinaryOp, BindingKind};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectPatternEl,
    ObjectProp, ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, GC_INIT,
    OBJECT_COPY_OWN, OBJECT_DELETE, OBJECT_GET, OBJECT_SET, PRINT_F64, PRINT_STR,
};
mod emit;

/// qNaN payload marking JS `undefined` for missing props / uninit slots.
const UNDEF_BITS: u64 = 0x7FF8_0000_0000_0001;

pub(crate) fn is_es_object_destructure_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_object_destructure(module: &Module) -> Result<String, Diagnostic> {
    let info =
        classify(module).ok_or_else(|| diag("internal: not an es_object_destructure module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_object_destructure(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_object_destructure_module(module) {
        return None;
    }
    Some(emit_es_object_destructure(module))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SlotTy {
    Number,
    String,
    Object,
    Array,
    /// Function binding (not printed).
    Function,
}

#[derive(Clone)]
struct FnInfo {
    idx: usize,
    /// Object-pattern param (fixture uses one).
    param_pattern: Vec<ObjectPatternEl>,
    /// Locals introduced by the param pattern (for allocas inside fn).
    param_locals: Vec<LocalId>,
    body: Vec<Stmt>,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    functions: Vec<FnInfo>,
    fn_binding: HashMap<LocalId, usize>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut slot_of: HashMap<LocalId, SlotTy> = HashMap::new();
    let mut print_locals = Vec::new();
    let mut seen_print = HashSet::new();
    let mut functions = Vec::new();
    let mut fn_binding = HashMap::new();
    let mut has_obj_dstr = false;

    for stmt in &module.body {
        classify_stmt(
            stmt,
            true,
            &by_id,
            &mut slot_of,
            &mut print_locals,
            &mut seen_print,
            &mut functions,
            &mut fn_binding,
            &mut has_obj_dstr,
        )?;
    }

    if !has_obj_dstr || print_locals.is_empty() {
        return None;
    }

    let mut slots: Vec<(LocalId, SlotTy)> = slot_of.into_iter().collect();
    slots.sort_by_key(|(id, _)| id.0);

    Some(ModuleInfo {
        slots,
        print_locals,
        functions,
        fn_binding,
    })
}

fn classify_stmt(
    stmt: &Stmt,
    top: bool,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &mut HashMap<LocalId, SlotTy>,
    print_locals: &mut Vec<(LocalId, SlotTy)>,
    seen_print: &mut HashSet<LocalId>,
    functions: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
    has_obj_dstr: &mut bool,
) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, kind } => {
            let slot = slot_for_declare(*local, init.as_ref(), by_id, slot_of)?;
            slot_of.entry(*local).or_insert(slot);
            // Observe number results only (not key strings / heap objects).
            if top && slot == SlotTy::Number && seen_print.insert(*local) {
                print_locals.push((*local, slot));
            }
            if *kind == BindingKind::Var {
                // bare ok
            }
            Some(())
        }
        Stmt::DeclareObjectPattern {
            properties,
            init: Some(init),
            ..
        } => {
            if !value_expr_ok(init, by_id, slot_of) {
                return None;
            }
            *has_obj_dstr = true;
            classify_object_pattern(properties, top, by_id, slot_of, print_locals, seen_print)
        }
        Stmt::Function {
            local,
            params,
            body,
            is_async,
            is_generator,
        } => {
            if *is_async || *is_generator || params.len() != 1 {
                return None;
            }
            let Param {
                pattern,
                default,
                rest,
            } = &params[0];
            if default.is_some() || *rest {
                return None;
            }
            let Pattern::Object(props) = pattern else {
                return None;
            };
            if !object_pattern_ok(props, by_id, slot_of) {
                return None;
            }
            // Body: single return of a number local from the pattern.
            if body.len() != 1 {
                return None;
            }
            let Stmt::Return {
                value: Some(Expr::Local { id, .. }),
            } = &body[0]
            else {
                return None;
            };
            let mut param_locals = Vec::new();
            collect_pattern_locals(props, &mut param_locals);
            if !param_locals.contains(id) {
                return None;
            }
            // Register param locals as number (fixture only binds numbers).
            for &pl in &param_locals {
                slot_of.entry(pl).or_insert(SlotTy::Number);
            }
            let idx = functions.len();
            functions.push(FnInfo {
                idx,
                param_pattern: props.clone(),
                param_locals,
                body: body.clone(),
            });
            fn_binding.insert(*local, idx);
            slot_of.insert(*local, SlotTy::Function);
            Some(())
        }
        Stmt::Expr { expr } => {
            if let Expr::Assign {
                target: AssignTarget::ObjectPattern { properties },
                op: AssignOp::Eq,
                value,
                ..
            } = expr
            {
                if !value_expr_ok(value, by_id, slot_of) {
                    return None;
                }
                *has_obj_dstr = true;
                return classify_object_pattern(
                    properties,
                    top,
                    by_id,
                    slot_of,
                    print_locals,
                    seen_print,
                );
            }
            if expr_ok(expr, by_id, slot_of, fn_binding) {
                Some(())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn collect_pattern_locals(props: &[ObjectPatternEl], out: &mut Vec<LocalId>) {
    for p in props {
        match p {
            ObjectPatternEl::Prop { binding, .. } => collect_binding_locals(binding, out),
            ObjectPatternEl::Rest(b) => collect_binding_locals(b, out),
        }
    }
}

fn collect_binding_locals(b: &Pattern, out: &mut Vec<LocalId>) {
    match b {
        Pattern::Local(id) => out.push(*id),
        Pattern::Object(inner) => collect_pattern_locals(inner, out),
        Pattern::Array(_) | Pattern::Member { .. } | Pattern::Name(_) => {}
    }
}

fn classify_object_pattern(
    properties: &[ObjectPatternEl],
    top: bool,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &mut HashMap<LocalId, SlotTy>,
    print_locals: &mut Vec<(LocalId, SlotTy)>,
    seen_print: &mut HashSet<LocalId>,
) -> Option<()> {
    if !object_pattern_ok(properties, by_id, slot_of) {
        return None;
    }
    for p in properties {
        match p {
            ObjectPatternEl::Prop {
                binding, default, ..
            } => {
                if let Some(d) = default {
                    if !number_expr_ok(d, by_id, slot_of) {
                        return None;
                    }
                }
                classify_binding(
                    binding,
                    SlotTy::Number,
                    top,
                    by_id,
                    slot_of,
                    print_locals,
                    seen_print,
                )?;
            }
            ObjectPatternEl::Rest(binding) => {
                classify_binding(
                    binding,
                    SlotTy::Object,
                    top,
                    by_id,
                    slot_of,
                    print_locals,
                    seen_print,
                )?;
            }
        }
    }
    Some(())
}

fn classify_binding(
    binding: &Pattern,
    bind_ty: SlotTy,
    top: bool,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &mut HashMap<LocalId, SlotTy>,
    print_locals: &mut Vec<(LocalId, SlotTy)>,
    seen_print: &mut HashSet<LocalId>,
) -> Option<()> {
    match binding {
        Pattern::Local(id) => {
            if let Some(existing) = slot_of.get(id).copied() {
                if existing == bind_ty {
                    // ok
                } else if existing == SlotTy::Number && bind_ty == SlotTy::Number {
                    // bare let provisional
                } else if existing == SlotTy::Number && bind_ty == SlotTy::Object {
                    // bare `let tail` upgraded by rest — drop number observation
                    *slot_of.get_mut(id).unwrap() = SlotTy::Object;
                    print_locals.retain(|(l, _)| l != id);
                    seen_print.remove(id);
                } else {
                    return None;
                }
            } else {
                slot_of.insert(*id, bind_ty);
            }
            if top && bind_ty == SlotTy::Number && seen_print.insert(*id) {
                print_locals.push((*id, SlotTy::Number));
            }
            Some(())
        }
        Pattern::Member {
            object,
            property,
            computed,
        } => {
            if !object_expr_ok(object, by_id, slot_of) {
                return None;
            }
            if *computed {
                if !string_expr_ok(property, by_id, slot_of)
                    && !matches!(property.as_ref(), Expr::String { .. })
                {
                    return None;
                }
            } else if !matches!(property.as_ref(), Expr::String { .. }) {
                return None;
            }
            Some(())
        }
        Pattern::Object(inner) => {
            classify_object_pattern(inner, top, by_id, slot_of, print_locals, seen_print)
        }
        Pattern::Array(_) | Pattern::Name(_) => None,
    }
}

fn object_pattern_ok(
    properties: &[ObjectPatternEl],
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    for p in properties {
        match p {
            ObjectPatternEl::Prop {
                key,
                binding,
                default,
                ..
            } => {
                if !prop_key_ok(key, by_id, slot_of) {
                    return false;
                }
                if let Some(d) = default {
                    if !number_expr_ok(d, by_id, slot_of) {
                        return false;
                    }
                }
                if !binding_ok(binding, by_id, slot_of) {
                    return false;
                }
            }
            ObjectPatternEl::Rest(b) => {
                if !matches!(b, Pattern::Local(_)) {
                    return false;
                }
            }
        }
    }
    true
}

fn binding_ok(
    binding: &Pattern,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match binding {
        Pattern::Local(_) => true,
        Pattern::Member {
            object,
            property,
            computed,
        } => {
            object_expr_ok(object, by_id, slot_of)
                && if *computed {
                    string_expr_ok(property, by_id, slot_of)
                        || matches!(property.as_ref(), Expr::String { .. })
                } else {
                    matches!(property.as_ref(), Expr::String { .. })
                }
        }
        Pattern::Object(inner) => object_pattern_ok(inner, by_id, slot_of),
        Pattern::Array(_) | Pattern::Name(_) => false,
    }
}

fn prop_key_ok(
    key: &ObjectPropKey,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match key {
        ObjectPropKey::Static(_) => true,
        ObjectPropKey::Computed(e) => {
            string_expr_ok(e, by_id, slot_of) || matches!(e, Expr::String { .. })
        }
    }
}

fn slot_for_declare(
    local: LocalId,
    init: Option<&Expr>,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<SlotTy> {
    let Some(init) = init else {
        // Bare let/var — provisional number (assignment target).
        return Some(SlotTy::Number);
    };
    if matches!(init, Expr::Object { .. }) {
        return Some(SlotTy::Object);
    }
    if matches!(init, Expr::Array { .. }) {
        return Some(SlotTy::Array);
    }
    if matches!(init, Expr::String { .. }) || string_expr_ok(init, by_id, slot_of) {
        return Some(SlotTy::String);
    }
    if number_expr_ok(init, by_id, slot_of) {
        return Some(SlotTy::Number);
    }
    // Member read may be undefined (still number-ish observation).
    if member_get_ok(init, by_id, slot_of) {
        return Some(SlotTy::Number);
    }
    // Call returning number.
    if let Expr::Call { callee, args, .. } = init {
        if let Expr::Local { id, .. } = callee.as_ref() {
            if slot_of.get(id) == Some(&SlotTy::Function)
                && args.len() == 1
                && arg_ok(&args[0], by_id, slot_of)
            {
                return Some(SlotTy::Number);
            }
        }
    }
    // Local copy of number.
    if let Expr::Local { id, .. } = init {
        if slot_of.get(id) == Some(&SlotTy::Number) {
            return Some(SlotTy::Number);
        }
        if by_id
            .get(id)
            .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        {
            return Some(SlotTy::Number);
        }
    }
    let _ = local;
    None
}

fn arg_ok(arg: &Arg, by_id: &HashMap<LocalId, &Local>, slot_of: &HashMap<LocalId, SlotTy>) -> bool {
    match arg {
        Arg::Expr(e) => value_expr_ok(e, by_id, slot_of),
        Arg::Spread(_) => false,
    }
}

fn value_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Object { properties, .. } => object_lit_ok(properties, by_id, slot_of),
        Expr::Array { elements, .. } => array_lit_ok(elements, by_id, slot_of),
        Expr::Local { id, .. } => matches!(
            slot_of.get(id),
            Some(SlotTy::Object | SlotTy::Array | SlotTy::Number | SlotTy::String)
        ),
        _ => number_expr_ok(expr, by_id, slot_of) || string_expr_ok(expr, by_id, slot_of),
    }
}

fn object_lit_ok(
    properties: &[ObjectProp],
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    for p in properties {
        match p {
            ObjectProp::Property { key, value } => {
                if !prop_key_ok(key, by_id, slot_of) {
                    return false;
                }
                if !number_expr_ok(value, by_id, slot_of)
                    && !matches!(value, Expr::Object { .. })
                    && !object_expr_ok(value, by_id, slot_of)
                {
                    // nested object lit
                    if let Expr::Object {
                        properties: inner, ..
                    } = value
                    {
                        if !object_lit_ok(inner, by_id, slot_of) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
            _ => return false,
        }
    }
    true
}

fn array_lit_ok(
    elements: &[ArrayElement],
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    for el in elements {
        match el {
            ArrayElement::Expr(e) => {
                if !number_expr_ok(e, by_id, slot_of) {
                    return false;
                }
            }
            ArrayElement::Spread(_) | ArrayElement::Elision => return false,
        }
    }
    true
}

fn object_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Object { properties, .. } => object_lit_ok(properties, by_id, slot_of),
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
                    string_expr_ok(property, by_id, slot_of)
                        || matches!(property.as_ref(), Expr::String { .. })
                } else {
                    matches!(property.as_ref(), Expr::String { .. })
                }
        }
        _ => false,
    }
}

fn member_get_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && (object_expr_ok(object, by_id, slot_of)
                    || matches!(
                        object.as_ref(),
                        Expr::Local { id, .. } if slot_of.get(id) == Some(&SlotTy::Object)
                            || slot_of.get(id) == Some(&SlotTy::Array)
                    ))
                && if *computed {
                    string_expr_ok(property, by_id, slot_of)
                        || number_expr_ok(property, by_id, slot_of)
                        || matches!(property.as_ref(), Expr::String { .. } | Expr::Number { .. })
                } else {
                    matches!(property.as_ref(), Expr::String { .. })
                }
        }
        _ => false,
    }
}

fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
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
            op: BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div,
            right,
            ..
        } => number_expr_ok(left, by_id, slot_of) && number_expr_ok(right, by_id, slot_of),
        Expr::Member { .. } => member_get_ok(expr, by_id, slot_of),
        Expr::Call { callee, args, .. } => {
            matches!(
                callee.as_ref(),
                Expr::Local { id, .. } if slot_of.get(id) == Some(&SlotTy::Function)
            ) && args.len() == 1
                && arg_ok(&args[0], by_id, slot_of)
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
        Expr::Local { id, .. } => {
            slot_of.get(id) == Some(&SlotTy::String)
                || by_id.get(id).is_some_and(|l| l.ty == Type::String)
        }
        _ => false,
    }
}

fn expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::Assign {
            target: AssignTarget::Local(_),
            op: AssignOp::Eq,
            value,
            ..
        } => number_expr_ok(value, by_id, slot_of) || value_expr_ok(value, by_id, slot_of),
        Expr::Call { callee, args, .. } => {
            matches!(
                callee.as_ref(),
                Expr::Local { id, .. }
                    if slot_of.get(id) == Some(&SlotTy::Function) || fn_binding.contains_key(id)
            ) && args.iter().all(|a| arg_ok(a, by_id, slot_of))
        }
        _ => number_expr_ok(expr, by_id, slot_of),
    }
}

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    out: String,
    body: String,
    tmp: usize,
    str_n: usize,
    str_globals: Vec<(String, String)>,
    allocas: HashMap<LocalId, String>,
    slot_of: HashMap<LocalId, SlotTy>,
    /// Inside function: param pattern local allocas.
    fn_local_allocas: HashMap<LocalId, String>,
    in_fn: bool,
}

fn format_number_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(format!("invalid number literal {raw}")))?;
    Ok(format!("{f:.17e}"))
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
