//! N08.07.04: native observations for tagged templates (`es/strings/tagged_template`).
//!
//! `` tag`a${x}b` `` → call `tag(stringsArray, …interps)` where `stringsArray` is a
//! Runtime array of cooked quasi cstrings. Tag may be a function decl, a call that
//! returns a function, or `obj.method`. Tag bodies use array index/`.length`, string
//! concat (incl. number ToString), and `===` / `&&` for the empty-template bool case.

use std::collections::HashMap;

use draconic_ast::BinaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, CSTR_CONCAT,
    CSTR_EQ_N, CSTR_FROM_U64, CSTR_LEN, GC_INIT, OBJECT_GET, OBJECT_SET, PRINT_BOOL, PRINT_STR,
};
mod emit;

/// Function indices encoded as `inttoptr i64 (idx + FN_TAG)`.
const FN_TAG: i64 = 1000;

pub(crate) fn is_es_tagged_template_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_tagged_template(module: &Module) -> Result<String, Diagnostic> {
    let info =
        classify(module).ok_or_else(|| diag("internal: not an es_tagged_template module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_tagged_template(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_tagged_template_module(module) {
        return None;
    }
    Some(emit_es_tagged_template(module))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LocalSlot {
    String,
    Bool,
    Object,
    Function,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RetKind {
    String,
    Bool,
    Function,
}

#[derive(Clone)]
struct FnInfo {
    idx: usize,
    params: Vec<LocalId>,
    body: Vec<Stmt>,
    ret: RetKind,
}

struct ModuleInfo {
    functions: Vec<FnInfo>,
    /// Local → function index (decls + method expr bindings).
    fn_binding: HashMap<LocalId, usize>,
    /// Top-level slots (declare order for non-fn).
    slots: Vec<(LocalId, LocalSlot)>,
    /// Observation prints in declare order.
    print_locals: Vec<(LocalId, LocalSlot)>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut fn_binding = HashMap::new();

    collect_functions(&module.body, &by_id, &mut functions, &mut fn_binding)?;

    let mut has_tt = false;
    for f in &functions {
        if !fn_body_ok(&f.body, &by_id, &fn_binding, &f.params) {
            return None;
        }
        if body_has_tagged(&f.body) {
            has_tt = true;
        }
    }

    let ret_of: HashMap<usize, RetKind> = functions.iter().map(|f| (f.idx, f.ret)).collect();

    let mut slots = Vec::new();
    let mut print_locals = Vec::new();
    let mut slot_of: HashMap<LocalId, LocalSlot> = HashMap::new();

    for stmt in &module.body {
        match stmt {
            Stmt::Function { .. } => {}
            Stmt::Declare { local, init, .. } => {
                let init = init.as_ref()?;
                if fn_binding.contains_key(local) {
                    continue;
                }
                if let Expr::Object { properties, .. } = init {
                    if !object_ok(properties, &by_id, &fn_binding) {
                        return None;
                    }
                    slots.push((*local, LocalSlot::Object));
                    slot_of.insert(*local, LocalSlot::Object);
                    continue;
                }
                if matches!(init, Expr::Function { .. }) {
                    continue;
                }
                let kind = slot_kind_of(init, &by_id, &fn_binding, &slot_of, &ret_of)?;
                if !expr_ok(init, &by_id, &fn_binding, &slot_of) {
                    return None;
                }
                if matches!(init, Expr::TaggedTemplate { .. }) {
                    has_tt = true;
                }
                slots.push((*local, kind));
                slot_of.insert(*local, kind);
                if matches!(kind, LocalSlot::String | LocalSlot::Bool) {
                    print_locals.push((*local, kind));
                }
            }
            _ => return None,
        }
    }

    if !has_tt || print_locals.is_empty() || functions.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        functions,
        fn_binding,
        slots,
        print_locals,
    })
}

fn collect_functions(
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
                let ids = simple_params(params)?;
                let ret = infer_ret(body, by_id, fn_binding)?;
                let idx = out.len();
                fn_binding.insert(*local, idx);
                out.push(FnInfo {
                    idx,
                    params: ids,
                    body: body.clone(),
                    ret,
                });
                collect_functions(body, by_id, out, fn_binding)?;
            }
            Stmt::Declare {
                local,
                init:
                    Some(Expr::Function {
                        params,
                        body,
                        is_async,
                        is_generator,
                        is_arrow,
                        ..
                    }),
                ..
            } => {
                if *is_async || *is_generator || *is_arrow {
                    return None;
                }
                let ids = simple_params(params)?;
                let ret = infer_ret(body, by_id, fn_binding)?;
                let idx = out.len();
                fn_binding.insert(*local, idx);
                out.push(FnInfo {
                    idx,
                    params: ids,
                    body: body.clone(),
                    ret,
                });
                collect_functions(body, by_id, out, fn_binding)?;
            }
            Stmt::Declare {
                init: Some(Expr::Object { properties, .. }),
                ..
            } => {
                for p in properties {
                    if let ObjectProp::Property {
                        value:
                            Expr::Function {
                                params,
                                body,
                                is_async,
                                is_generator,
                                ..
                            },
                        ..
                    } = p
                    {
                        if *is_async || *is_generator {
                            return None;
                        }
                        let ids = simple_params(params)?;
                        let ret = infer_ret(body, by_id, fn_binding)?;
                        let idx = out.len();
                        out.push(FnInfo {
                            idx,
                            params: ids,
                            body: body.clone(),
                            ret,
                        });
                        collect_functions(body, by_id, out, fn_binding)?;
                    }
                }
            }
            Stmt::Declare { init: Some(e), .. } => collect_expr_fns(e, by_id, out, fn_binding)?,
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
            is_arrow,
            ..
        } => {
            if *is_async || *is_generator || *is_arrow {
                return None;
            }
            let ids = simple_params(params)?;
            let ret = infer_ret(body, by_id, fn_binding)?;
            let idx = out.len();
            out.push(FnInfo {
                idx,
                params: ids,
                body: body.clone(),
                ret,
            });
            collect_functions(body, by_id, out, fn_binding)?;
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            collect_expr_fns(tag, by_id, out, fn_binding)?;
            for e in expressions {
                collect_expr_fns(e, by_id, out, fn_binding)?;
            }
        }
        Expr::Call { callee, args, .. } => {
            collect_expr_fns(callee, by_id, out, fn_binding)?;
            for a in args {
                if let draconic_ir::Arg::Expr(e) = a {
                    collect_expr_fns(e, by_id, out, fn_binding)?;
                }
            }
        }
        Expr::Member { object, .. } => collect_expr_fns(object, by_id, out, fn_binding)?,
        Expr::Object { properties, .. } => {
            for p in properties {
                if let ObjectProp::Property { value, .. } = p {
                    collect_expr_fns(value, by_id, out, fn_binding)?;
                }
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_fns(left, by_id, out, fn_binding)?;
            collect_expr_fns(right, by_id, out, fn_binding)?;
        }
        _ => {}
    }
    Some(())
}

fn simple_params(params: &[Param]) -> Option<Vec<LocalId>> {
    let mut ids = Vec::new();
    for p in params {
        if p.rest || p.default.is_some() {
            return None;
        }
        match &p.pattern {
            Pattern::Local(id) => ids.push(*id),
            _ => return None,
        }
    }
    Some(ids)
}

fn infer_ret(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
) -> Option<RetKind> {
    let ret = find_return(body)?;
    match ret {
        Expr::Local { id, .. } if fn_binding.contains_key(id) => Some(RetKind::Function),
        Expr::Local { id, ty } => match ty {
            Type::Function => Some(RetKind::Function),
            Type::Boolean => Some(RetKind::Bool),
            Type::String => Some(RetKind::String),
            Type::Any => {
                if by_id.get(id).is_some_and(|l| l.ty == Type::Function) {
                    Some(RetKind::Function)
                } else {
                    Some(RetKind::String)
                }
            }
            _ => Some(RetKind::String),
        },
        Expr::Binary {
            op: BinaryOp::EqEqEq | BinaryOp::EqEq | BinaryOp::And | BinaryOp::Or,
            ..
        }
        | Expr::Boolean { .. } => Some(RetKind::Bool),
        Expr::Binary {
            op: BinaryOp::Add, ..
        }
        | Expr::String { .. }
        | Expr::Member { .. } => Some(RetKind::String),
        _ => Some(RetKind::String),
    }
}

fn find_return(body: &[Stmt]) -> Option<&Expr> {
    for s in body {
        match s {
            Stmt::Return { value: Some(v) } => return Some(v),
            Stmt::Block { body } => {
                if let Some(v) = find_return(body) {
                    return Some(v);
                }
            }
            _ => {}
        }
    }
    None
}

fn body_has_tagged(body: &[Stmt]) -> bool {
    body.iter().any(stmt_has_tagged)
}

fn stmt_has_tagged(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { value: Some(e) } | Stmt::Expr { expr: e } => expr_has_tagged(e),
        Stmt::Declare { init: Some(e), .. } => expr_has_tagged(e),
        Stmt::Block { body } => body.iter().any(stmt_has_tagged),
        _ => false,
    }
}

fn expr_has_tagged(expr: &Expr) -> bool {
    match expr {
        Expr::TaggedTemplate { .. } => true,
        Expr::Binary { left, right, .. } => expr_has_tagged(left) || expr_has_tagged(right),
        Expr::Call { callee, args, .. } => {
            expr_has_tagged(callee)
                || args
                    .iter()
                    .any(|a| matches!(a, draconic_ir::Arg::Expr(e) if expr_has_tagged(e)))
        }
        Expr::Member { object, .. } => expr_has_tagged(object),
        _ => false,
    }
}

fn object_ok(
    properties: &[ObjectProp],
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    properties.iter().all(|p| match p {
        ObjectProp::Property {
            key: ObjectPropKey::Static(_),
            value:
                Expr::Function {
                    params,
                    body,
                    is_async: false,
                    is_generator: false,
                    ..
                },
        } => {
            simple_params(params).is_some()
                && fn_body_ok(body, by_id, fn_binding, &simple_params(params).unwrap())
        }
        _ => false,
    })
}

fn slot_kind_of(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    slot_of: &HashMap<LocalId, LocalSlot>,
    ret_of: &HashMap<usize, RetKind>,
) -> Option<LocalSlot> {
    match expr {
        Expr::String { .. } => Some(LocalSlot::String),
        Expr::Boolean { .. } => Some(LocalSlot::Bool),
        Expr::TaggedTemplate { tag, .. } => match tag.as_ref() {
            Expr::Local { id, .. } => {
                let idx = *fn_binding.get(id)?;
                match ret_of.get(&idx).copied().unwrap_or(RetKind::String) {
                    RetKind::Bool => Some(LocalSlot::Bool),
                    RetKind::Function => Some(LocalSlot::Function),
                    RetKind::String => Some(LocalSlot::String),
                }
            }
            Expr::Call { .. } | Expr::Member { .. } => Some(LocalSlot::String),
            _ => None,
        },
        Expr::Local { id, ty } => {
            if let Some(k) = slot_of.get(id) {
                return Some(*k);
            }
            match ty {
                Type::String => Some(LocalSlot::String),
                Type::Boolean => Some(LocalSlot::Bool),
                Type::Object => Some(LocalSlot::Object),
                Type::Function => Some(LocalSlot::Function),
                Type::Any => by_id.get(id).map(|l| match l.ty {
                    Type::String => LocalSlot::String,
                    Type::Boolean => LocalSlot::Bool,
                    Type::Object => LocalSlot::Object,
                    Type::Function => LocalSlot::Function,
                    _ => LocalSlot::String,
                }),
                _ => None,
            }
        }
        Expr::Binary {
            op: BinaryOp::EqEqEq | BinaryOp::EqEq | BinaryOp::And | BinaryOp::Or,
            ..
        } => Some(LocalSlot::Bool),
        Expr::Binary {
            op: BinaryOp::Add, ..
        } => Some(LocalSlot::String),
        Expr::Object { .. } => Some(LocalSlot::Object),
        _ => None,
    }
}

fn fn_body_ok(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    body.iter().all(|s| stmt_ok(s, by_id, fn_binding, params))
}

fn stmt_ok(
    stmt: &Stmt,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    match stmt {
        Stmt::Return { value: Some(e) } => {
            expr_ok(e, by_id, fn_binding, &HashMap::new()) || {
                // allow param locals
                expr_ok_with_params(e, by_id, fn_binding, params)
            }
        }
        Stmt::Return { value: None } => true,
        Stmt::Block { body } => body.iter().all(|s| stmt_ok(s, by_id, fn_binding, params)),
        _ => false,
    }
}

fn expr_ok_with_params(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    match expr {
        Expr::Local { id, .. } => {
            params.contains(id) || fn_binding.contains_key(id) || by_id.contains_key(id)
        }
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } => true,
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            expr_ok_with_params(object, by_id, fn_binding, params)
                && (matches!(property.as_ref(), Expr::String { value, .. } if value.to_string_lossy() == "length")
                    || matches!(property.as_ref(), Expr::Number { .. })
                    || expr_ok_with_params(property, by_id, fn_binding, params))
        }
        Expr::Binary {
            left, right, op, ..
        } => {
            matches!(
                op,
                BinaryOp::Add | BinaryOp::EqEqEq | BinaryOp::EqEq | BinaryOp::And | BinaryOp::Or
            ) && expr_ok_with_params(left, by_id, fn_binding, params)
                && expr_ok_with_params(right, by_id, fn_binding, params)
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            expr_ok_with_params(callee, by_id, fn_binding, params)
                && args.iter().all(|a| match a {
                    draconic_ir::Arg::Expr(e) => expr_ok_with_params(e, by_id, fn_binding, params),
                    _ => false,
                })
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            expr_ok_with_params(tag, by_id, fn_binding, params)
                && expressions
                    .iter()
                    .all(|e| expr_ok_with_params(e, by_id, fn_binding, params))
        }
        _ => false,
    }
}

fn expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    slot_of: &HashMap<LocalId, LocalSlot>,
) -> bool {
    match expr {
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } => true,
        Expr::Local { id, .. } => {
            fn_binding.contains_key(id) || slot_of.contains_key(id) || by_id.contains_key(id)
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            tag_ok(tag, by_id, fn_binding, slot_of)
                && expressions
                    .iter()
                    .all(|e| expr_ok(e, by_id, fn_binding, slot_of))
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            expr_ok(callee, by_id, fn_binding, slot_of)
                && args.iter().all(|a| match a {
                    draconic_ir::Arg::Expr(e) => expr_ok(e, by_id, fn_binding, slot_of),
                    _ => false,
                })
        }
        Expr::Member {
            object,
            property,
            optional: false,
            computed: false,
            ..
        } => {
            expr_ok(object, by_id, fn_binding, slot_of)
                && matches!(property.as_ref(), Expr::String { .. })
        }
        Expr::Object { properties, .. } => object_ok(properties, by_id, fn_binding),
        Expr::Binary {
            left, right, op, ..
        } => {
            matches!(
                op,
                BinaryOp::Add | BinaryOp::EqEqEq | BinaryOp::EqEq | BinaryOp::And | BinaryOp::Or
            ) && expr_ok(left, by_id, fn_binding, slot_of)
                && expr_ok(right, by_id, fn_binding, slot_of)
        }
        _ => false,
    }
}

fn tag_ok(
    tag: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    slot_of: &HashMap<LocalId, LocalSlot>,
) -> bool {
    match tag {
        Expr::Local { id, .. } => fn_binding.contains_key(id),
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            matches!(callee.as_ref(), Expr::Local { id, .. } if fn_binding.contains_key(id))
                && args.is_empty()
        }
        Expr::Member {
            object,
            property,
            computed: false,
            optional: false,
            ..
        } => {
            matches!(object.as_ref(), Expr::Local { id, .. } if slot_of.get(id) == Some(&LocalSlot::Object) || matches!(by_id.get(id).map(|l| l.ty), Some(Type::Object | Type::Any)))
                && matches!(property.as_ref(), Expr::String { .. })
        }
        _ => false,
    }
}

// --- Emitter ---

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    out: String,
    body: String,
    tmp: usize,
    str_n: usize,
    str_globals: Vec<(String, String)>,
    /// local → global ptr name
    slots: HashMap<LocalId, String>,
    /// param local → alloca while in function
    param_alloca: HashMap<LocalId, String>,
    /// method key "m" on object local → fn idx (filled during object emit)
    method_keys: HashMap<(LocalId, String), usize>,
    /// object local → list of (key, fn_idx) for property install
    object_methods: HashMap<LocalId, Vec<(String, usize)>>,
}

fn is_length_member(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Member {
            property,
            computed: false,
            optional: false,
            ..
        } if matches!(property.as_ref(), Expr::String { value, .. } if value.to_string_lossy() == "length")
    )
}

fn slot_tag(k: LocalSlot) -> &'static str {
    match k {
        LocalSlot::String => "s",
        LocalSlot::Bool => "b",
        LocalSlot::Object => "o",
        LocalSlot::Function => "f",
    }
}

fn parse_nonneg_int(raw: &str) -> Result<u64, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    if let Ok(n) = cleaned.parse::<u64>() {
        return Ok(n);
    }
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(format!("es_tt: bad number {raw}")))?;
    if f >= 0.0 && f.fract() == 0.0 && f < (u64::MAX as f64) {
        Ok(f as u64)
    } else {
        Err(diag(format!("es_tt: non-int number {raw}")))
    }
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
