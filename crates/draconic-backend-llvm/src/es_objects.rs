//! N08.04.01–N08.04.06 + N08.16.28: native observations for ES object literals,
//! property access, simple property assignment, method call + `this`, `new`
//! constructors, prototypes, object-literal sugar (E04.01–E04.06 /
//! `es/objects/*` incl. `object_lit_sugar`), and object spread in literals
//! (E18.28 / `es/annex-b/object_spread`).
//!
//! Object values are Runtime GC heap ptrs; number props are stored as
//! `inttoptr` of integer bit-patterns (fixture uses small integers). Nested
//! objects store GC ptrs. Function-valued props and constructors store LLVM
//! method fn ptrs. Method/ctor calls use a uniform signature
//! `double (ptr this, double a0..a3)`. Function decls are ctor objects with a
//! `.prototype` own prop; `new C(args)` allocates an instance, sets
//! `[[Prototype]]` from `C.prototype`, calls the ctor, and yields the instance.
//! Runtime `object_get` walks the prototype chain so inherited methods resolve.
//! Property shorthand / method shorthand lower as static keys; computed keys
//! (`[expr]`) accept string locals or string literals. Object spread
//! (`{...src}`) copies own enumerable string keys via Runtime
//! `object_spread` (null/undefined sources are no-ops). Top-level number/string
//! slots are module globals so methods can read free number locals.
//! Number locals from member reads / method returns are printed via `print_f64`.

use std::collections::HashMap;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey,
    Param, Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, GC_INIT, OBJECT_GET, OBJECT_SET, OBJECT_SET_PROTO, OBJECT_SPREAD,
    PRINT_F64,
};
mod emit;

/// Max fixed args for method/ctor calling convention (fixtures use ≤2).
const MAX_METHOD_ARGS: usize = 4;

pub(crate) fn is_es_objects_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_objects(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_objects module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_objects(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_objects_module(module) {
        return None;
    }
    Some(emit_es_objects(module))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    Object,
    String,
}

#[derive(Clone)]
struct FnInfo {
    idx: usize,
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    number_locals: Vec<LocalId>,
    functions: Vec<FnInfo>,
    /// Function-declaration bindings → LLVM method index (`new` callees).
    fn_binding: HashMap<LocalId, usize>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut fn_binding = HashMap::new();
    collect_all_fns(&module.body, &by_id, &mut functions, &mut fn_binding)?;

    for f in &functions {
        if !method_body_ok(&f.body, &by_id, &functions, &fn_binding, &f.params) {
            return None;
        }
    }

    let mut slots = Vec::new();
    let mut number_locals = Vec::new();
    let mut has_object = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Function { local, .. } => {
                // N08.04.05: ctor binding is a heap object with `.prototype`.
                has_object = true;
                slots.push((*local, SlotTy::Object));
            }
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                let init = init.as_ref()?;
                if is_object_slot_ty(&loc.ty) || expr_is_object_init(init) {
                    if !object_expr_ok(init, &by_id, &functions, &fn_binding) {
                        return None;
                    }
                    has_object = true;
                    slots.push((*local, SlotTy::Object));
                } else if is_string_slot_ty(&loc.ty) || expr_is_string_init(init) {
                    if !string_expr_ok(init, &by_id) {
                        return None;
                    }
                    slots.push((*local, SlotTy::String));
                } else if is_number_slot_ty(&loc.ty) || expr_is_number_init(init) {
                    if !number_expr_ok(init, &by_id, &functions, &fn_binding) {
                        return None;
                    }
                    slots.push((*local, SlotTy::Number));
                    number_locals.push(*local);
                } else {
                    return None;
                }
            }
            Stmt::Expr { expr } => {
                if !member_assign_ok(expr, &by_id, &functions, &fn_binding)
                    && !number_expr_ok(expr, &by_id, &functions, &fn_binding)
                {
                    return None;
                }
            }
            _ => return None,
        }
    }

    if !has_object || number_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        slots,
        number_locals,
        functions,
        fn_binding,
    })
}

fn collect_all_fns(
    stmts: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    out: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
) -> Option<()> {
    for stmt in stmts {
        match stmt {
            Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
            } => {
                if *is_async || *is_generator {
                    return None;
                }
                let param_ids = simple_param_ids(params, by_id)?;
                collect_all_fns(body, by_id, out, fn_binding)?;
                let idx = out.len();
                out.push(FnInfo {
                    idx,
                    params: param_ids,
                    body: body.clone(),
                });
                fn_binding.insert(*local, idx);
            }
            Stmt::Declare { init: Some(e), .. } => collect_expr_fns(e, by_id, out, fn_binding)?,
            Stmt::Expr { expr } => collect_expr_fns(expr, by_id, out, fn_binding)?,
            Stmt::Block { body } => collect_all_fns(body, by_id, out, fn_binding)?,
            Stmt::Return { value: Some(e) } => collect_expr_fns(e, by_id, out, fn_binding)?,
            Stmt::Return { value: None } => {}
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                collect_expr_fns(test, by_id, out, fn_binding)?;
                collect_all_fns(std::slice::from_ref(consequent), by_id, out, fn_binding)?;
                if let Some(a) = alternate {
                    collect_all_fns(std::slice::from_ref(a), by_id, out, fn_binding)?;
                }
            }
            _ => {}
        }
    }
    Some(())
}

fn collect_expr_fns(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    out: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
) -> Option<()> {
    match expr {
        Expr::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if *is_async || *is_generator {
                return None;
            }
            let param_ids = simple_param_ids(params, by_id)?;
            collect_all_fns(body, by_id, out, fn_binding)?;
            let idx = out.len();
            out.push(FnInfo {
                idx,
                params: param_ids,
                body: body.clone(),
            });
            Some(())
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { value, .. } => {
                        collect_expr_fns(value, by_id, out, fn_binding)?
                    }
                    ObjectProp::Accessor { value, .. } => {
                        collect_expr_fns(value, by_id, out, fn_binding)?
                    }
                    ObjectProp::Spread(e) => collect_expr_fns(e, by_id, out, fn_binding)?,
                }
            }
            Some(())
        }
        Expr::Member {
            object, property, ..
        } => {
            collect_expr_fns(object, by_id, out, fn_binding)?;
            collect_expr_fns(property, by_id, out, fn_binding)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            collect_expr_fns(callee, by_id, out, fn_binding)?;
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_expr_fns(e, by_id, out, fn_binding)?;
                }
            }
            Some(())
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_fns(left, by_id, out, fn_binding)?;
            collect_expr_fns(right, by_id, out, fn_binding)
        }
        Expr::Assign { value, target, .. } => {
            collect_expr_fns(value, by_id, out, fn_binding)?;
            if let AssignTarget::Member {
                object, property, ..
            } = target
            {
                collect_expr_fns(object, by_id, out, fn_binding)?;
                collect_expr_fns(property, by_id, out, fn_binding)?;
            }
            Some(())
        }
        _ => Some(()),
    }
}

fn simple_param_ids(params: &[Param], by_id: &HashMap<LocalId, &Local>) -> Option<Vec<LocalId>> {
    let mut ids = Vec::new();
    for p in params {
        if p.rest || p.default.is_some() {
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
    if ids.len() > MAX_METHOD_ARGS {
        return None;
    }
    Some(ids)
}

fn find_fn_idx(params: &[Param], body: &[Stmt], functions: &[FnInfo]) -> Option<usize> {
    let ids: Vec<LocalId> = params
        .iter()
        .filter_map(|p| match &p.pattern {
            Pattern::Local(id) => Some(*id),
            _ => None,
        })
        .collect();
    if ids.len() != params.len() {
        return None;
    }
    // Match params + body so zero-arity methods (getX / m / g) stay distinct.
    functions
        .iter()
        .find(|f| f.params == ids && f.body == body)
        .map(|f| f.idx)
}

fn is_object_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Object | Type::Shape(_) | Type::Function)
}

fn is_number_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Number | Type::Any)
}

fn is_string_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::String)
}

fn expr_is_string_init(expr: &Expr) -> bool {
    matches!(expr, Expr::String { .. })
}

fn string_expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::String { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::String)
                || by_id.get(id).is_some_and(|l| matches!(l.ty, Type::String))
        }
        _ => false,
    }
}

fn prop_key_ok(key: &ObjectPropKey, by_id: &HashMap<LocalId, &Local>) -> bool {
    match key {
        ObjectPropKey::Static(_) => true,
        ObjectPropKey::Computed(e) => string_expr_ok(e, by_id),
    }
}

fn expr_is_object_init(expr: &Expr) -> bool {
    match expr {
        Expr::Object { .. } | Expr::New { .. } => true,
        Expr::Local { ty, .. } => is_object_slot_ty(ty),
        Expr::Member { ty, .. } => is_object_slot_ty(ty),
        _ => false,
    }
}

fn expr_is_number_init(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { ty, .. } => matches!(ty, Type::Number),
        Expr::Member {
            ty: Type::Number | Type::Any,
            ..
        } => true,
        Expr::Assign {
            op: AssignOp::Eq,
            ty: Type::Number | Type::Any,
            ..
        } => true,
        Expr::Call {
            ty: Type::Number | Type::Any,
            ..
        } => true,
        Expr::Binary {
            ty: Type::Number | Type::Any,
            ..
        } => true,
        _ => false,
    }
}

fn member_assign_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::Assign {
            target: AssignTarget::Member {
                object, property, ..
            },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            object_expr_ok(object, by_id, functions, fn_binding)
                && member_key_ok(property)
                && (number_expr_ok(value, by_id, functions, fn_binding)
                    || function_expr_ok(value, by_id, functions, fn_binding)
                    || object_expr_ok(value, by_id, functions, fn_binding))
        }
        _ => false,
    }
}

/// N08.16.28: spread source is object, null, or undefined (void 0 / unbound Any).
fn spread_source_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::Null { .. } => true,
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => true,
        _ => object_expr_ok(expr, by_id, functions, fn_binding),
    }
}

fn object_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::This { .. } => true,
        Expr::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { key, value } => {
                        if !prop_key_ok(key, by_id) {
                            return false;
                        }
                        if object_expr_ok(value, by_id, functions, fn_binding) {
                            continue;
                        }
                        if number_expr_ok(value, by_id, functions, fn_binding) {
                            continue;
                        }
                        if function_expr_ok(value, by_id, functions, fn_binding) {
                            continue;
                        }
                        return false;
                    }
                    ObjectProp::Spread(e) => {
                        if !spread_source_ok(e, by_id, functions, fn_binding) {
                            return false;
                        }
                    }
                    ObjectProp::Accessor { .. } => return false,
                }
            }
            true
        }
        Expr::Local { id, ty } => {
            fn_binding.contains_key(id)
                || is_object_slot_ty(ty)
                || by_id.get(id).is_some_and(|l| {
                    is_object_slot_ty(&l.ty)
                        || matches!(l.ty, Type::Any | Type::Function)
                        || fn_binding.contains_key(id)
                })
        }
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && object_expr_ok(object, by_id, functions, fn_binding)
                && member_key_ok(property)
        }
        Expr::New { callee, args, .. } => {
            let Expr::Local { id, .. } = callee.as_ref() else {
                return false;
            };
            if !fn_binding.contains_key(id) {
                return false;
            }
            if args.len() > MAX_METHOD_ARGS {
                return false;
            }
            args.iter().all(|a| match a {
                Arg::Expr(e) => number_expr_ok(e, by_id, functions, fn_binding),
                Arg::Spread(_) => false,
            })
        }
        _ => false,
    }
}

fn function_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if *is_async || *is_generator {
                return false;
            }
            let Some(param_ids) = simple_param_ids(params, by_id) else {
                return false;
            };
            if find_fn_idx(params, body, functions).is_none() {
                return false;
            }
            method_body_ok(body, by_id, functions, fn_binding, &param_ids)
        }
        _ => false,
    }
}

fn method_body_ok(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    for stmt in body {
        match stmt {
            Stmt::Return { value: Some(e) } => {
                if !number_expr_ok_in_method(e, by_id, functions, fn_binding, params) {
                    return false;
                }
            }
            Stmt::Return { value: None } => {}
            Stmt::Block { body } => {
                if !method_body_ok(body, by_id, functions, fn_binding, params) {
                    return false;
                }
            }
            Stmt::Expr { expr } => {
                if !ctor_or_method_expr_ok(expr, by_id, functions, fn_binding, params) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

/// Method/ctor statement expressions: number exprs, or `this.k =` number/function.
fn ctor_or_method_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    match expr {
        Expr::Assign {
            target: AssignTarget::Member {
                object, property, ..
            },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let obj_ok = object_expr_ok(object, by_id, functions, fn_binding)
                || matches!(object.as_ref(), Expr::This { .. });
            if !obj_ok || !member_key_ok(property) {
                return false;
            }
            number_expr_ok_in_method(value, by_id, functions, fn_binding, params)
                || function_expr_ok(value, by_id, functions, fn_binding)
        }
        _ => number_expr_ok_in_method(expr, by_id, functions, fn_binding, params),
    }
}

fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    number_expr_ok_in_method(expr, by_id, functions, fn_binding, &[])
}

fn number_expr_ok_in_method(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            if params.contains(id) {
                return true;
            }
            matches!(ty, Type::Number | Type::Any)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::This { .. } => false, // this alone is object, not number
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && member_key_ok(property)
                && (object_expr_ok(object, by_id, functions, fn_binding)
                    || matches!(object.as_ref(), Expr::This { .. }))
        }
        Expr::Assign {
            target: AssignTarget::Member {
                object, property, ..
            },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            (object_expr_ok(object, by_id, functions, fn_binding)
                || matches!(object.as_ref(), Expr::This { .. }))
                && member_key_ok(property)
                && number_expr_ok_in_method(value, by_id, functions, fn_binding, params)
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem,
            right,
            ..
        } => {
            number_expr_ok_in_method(left, by_id, functions, fn_binding, params)
                && number_expr_ok_in_method(right, by_id, functions, fn_binding, params)
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
            if !args.iter().all(|a| match a {
                Arg::Expr(e) => number_expr_ok_in_method(e, by_id, functions, fn_binding, params),
                Arg::Spread(_) => false,
            }) {
                return false;
            }
            // Method call: obj.m(...) / obj["m"](...)
            match callee.as_ref() {
                Expr::Member {
                    object,
                    property,
                    optional: mop,
                    ..
                } => {
                    !*mop
                        && member_key_ok(property)
                        && object_expr_ok(object, by_id, functions, fn_binding)
                        && args.len() <= MAX_METHOD_ARGS
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn member_key_ok(property: &Expr) -> bool {
    matches!(property, Expr::String { .. })
}

fn number_global_name(id: LocalId) -> String {
    format!("es_obj_n_{}", id.0)
}

fn string_global_name(id: LocalId) -> String {
    format!("es_obj_s_{}", id.0)
}

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    slot_of: HashMap<LocalId, SlotTy>,
    allocas: HashMap<LocalId, String>,
    /// Method param local → alloca name (only while emitting a method).
    param_allocas: HashMap<LocalId, String>,
    /// Current method `this` SSA value (ptr), if any.
    this_ssa: Option<String>,
    str_globals: Vec<(String, String)>,
    out: String,
    body: String,
    tmp: usize,
    str_n: usize,
}

fn object_value_is_object(expr: &Expr) -> bool {
    match expr {
        Expr::Object { .. } | Expr::New { .. } => true,
        Expr::Member { ty, .. } => is_object_slot_ty(ty),
        Expr::Local { ty, .. } => is_object_slot_ty(ty) || matches!(ty, Type::Function),
        _ => false,
    }
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
