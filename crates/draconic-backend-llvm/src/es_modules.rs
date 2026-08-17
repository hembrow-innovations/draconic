//! N08.11 / N08.16.32: native observations for linked ESM fixtures (E11) and
//! `export class` (E18.32).
//!
//! After the linker flattens static imports, module programs are ordinary IR with
//! mangled `__mN_*` locals. Compile-time evaluation covers named/default/namespace/
//! cyclic fixtures (number/string values, simple param calls, live `let` assign,
//! `import * as ns` via `__draconic_make_ns` pairs) plus exported class IIFEs
//! (`new`, instance props, methods, namespace class access). Emits Runtime prints
//! of entry top-level number/string locals (not mangled deps).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BYTES, PRINT_F64};

pub(crate) fn is_es_modules_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_modules(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_modules module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Str(String),
    Undef,
    Fn(LocalId),
    /// Function expression value (namespace getters, etc.).
    FnExpr(FnRec),
    /// Class constructor + prototype methods (from export class IIFE).
    Class(ClassRec),
    /// Instance object id into `EvalCtx::heap`.
    Obj(u32),
    /// Module namespace object: export name → getter (Fn / FnExpr).
    Ns(HashMap<String, JsVal>),
    /// Temporary array for `__draconic_make_ns` pair construction only.
    Arr(Vec<JsVal>),
}

#[derive(Clone, Debug)]
struct FnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
struct ClassRec {
    ctor: FnRec,
    methods: HashMap<String, FnRec>,
}

#[derive(Clone, Debug)]
struct InstRec {
    props: HashMap<String, JsVal>,
    methods: HashMap<String, FnRec>,
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Return(JsVal),
}

struct EvalCtx<'a> {
    env: HashMap<LocalId, JsVal>,
    functions: &'a HashMap<LocalId, FnRec>,
    make_ns: Option<LocalId>,
    heap: HashMap<u32, InstRec>,
    next_obj: u32,
    this_obj: Option<u32>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !is_linked_module_ir(module) {
        return None;
    }
    if !body_ok(&module.body, &by_id) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    let mut functions: HashMap<LocalId, FnRec> = HashMap::new();
    let mut make_ns: Option<LocalId> = None;

    // Hoist function decls (JS / linked module bodies).
    for stmt in &module.body {
        if let Stmt::Function {
            local,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } = stmt
        {
            let param_ids = simple_param_ids(params)?;
            functions.insert(
                *local,
                FnRec {
                    params: param_ids,
                    body: body.clone(),
                },
            );
            env.insert(*local, JsVal::Fn(*local));
            if by_id.get(local).is_some_and(|l| l.name == "__draconic_make_ns") {
                make_ns = Some(*local);
            }
        }
    }

    let mut ctx = EvalCtx {
        env,
        functions: &functions,
        make_ns,
        heap: HashMap::new(),
        next_obj: 1,
        this_obj: None,
    };

    match eval_body(&module.body, &mut ctx) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }

    let mut user_locals = Vec::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            if is_mangled_or_internal(&loc.name) {
                continue;
            }
            if matches!(
                ctx.env.get(local),
                Some(JsVal::Fn(_) | JsVal::Class(_) | JsVal::Obj(_) | JsVal::Ns(_))
            ) {
                continue;
            }
            if matches!(loc.ty, Type::Number | Type::Any | Type::String) {
                user_locals.push(*local);
            }
        }
    }

    if user_locals.is_empty() {
        return None;
    }

    let mut values = HashMap::new();
    for id in &user_locals {
        let v = ctx.env.get(id)?.clone();
        match &v {
            JsVal::Num(_) | JsVal::Str(_) => {
                values.insert(*id, v);
            }
            _ => return None,
        }
    }

    Some(ModuleInfo {
        user_locals,
        values,
    })
}

fn is_linked_module_ir(module: &Module) -> bool {
    module.locals.iter().any(|l| {
        l.name.starts_with("__m")
            || l.name.starts_with("__ns")
            || l.name.starts_with("__draconic_make_ns")
    })
}

fn is_mangled_or_internal(name: &str) -> bool {
    name.starts_with("__m")
        || name.starts_with("__ns")
        || name.starts_with("__draconic")
        || name == "arguments"
}

fn simple_param_ids(params: &[draconic_ir::Param]) -> Option<Vec<LocalId>> {
    let mut ids = Vec::with_capacity(params.len());
    for p in params {
        if p.default.is_some() || p.rest {
            return None;
        }
        match &p.pattern {
            Pattern::Local(id) => ids.push(*id),
            _ => return None,
        }
    }
    Some(ids)
}

fn body_ok(body: &[Stmt], by_id: &HashMap<LocalId, &Local>) -> bool {
    body.iter().all(|s| stmt_ok(s, by_id))
}

fn stmt_ok(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let Some(loc) = by_id.get(local) else {
                return false;
            };
            if !matches!(
                loc.ty,
                Type::Number | Type::Any | Type::String | Type::Function | Type::Object
            ) {
                return false;
            }
            match init {
                None => true,
                Some(e) => expr_ok(e, by_id),
            }
        }
        Stmt::Function {
            local,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => {
            // Namespace polyfill body is not CT-eval'd; pairs are interpreted at call.
            if by_id.get(local).is_some_and(|l| l.name == "__draconic_make_ns") {
                return simple_param_ids(params).is_some();
            }
            simple_param_ids(params).is_some() && body_ok(body, by_id)
        }
        Stmt::Return { value } => match value {
            None => true,
            Some(e) => expr_ok(e, by_id),
        },
        Stmt::Block { body } => body_ok(body, by_id),
        Stmt::Expr { expr } => expr_ok(expr, by_id),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_ok(test, by_id)
                && stmt_ok(consequent, by_id)
                && alternate.as_ref().is_none_or(|a| stmt_ok(a, by_id))
        }
        _ => false,
    }
}

fn expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::NewTarget { .. }
        | Expr::IdentName { .. } => true,
        Expr::Local { id, .. } => by_id.contains_key(id),
        Expr::Unary { arg, .. } => expr_ok(arg, by_id),
        Expr::Binary {
            left, right, op, ..
        } => {
            matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
                    | BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
                    | BinaryOp::And
                    | BinaryOp::Or
                    | BinaryOp::Comma
                    | BinaryOp::In
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq
            ) && expr_ok(left, by_id)
                && expr_ok(right, by_id)
        }
        Expr::Assign {
            target: AssignTarget::Local(_),
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(value, by_id),
        Expr::Assign {
            target: AssignTarget::Member {
                object, property, ..
            },
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(object, by_id) && expr_ok(property, by_id) && expr_ok(value, by_id),
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            expr_ok(callee, by_id)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e, by_id),
                    _ => false,
                })
        }
        Expr::New {
            callee,
            args,
            ..
        } => {
            expr_ok(callee, by_id)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e, by_id),
                    _ => false,
                })
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => expr_ok(object, by_id) && expr_ok(property, by_id),
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => expr_ok(e, by_id),
            _ => false,
        }),
        Expr::Object { properties, .. } => properties.iter().all(|p| match p {
            ObjectProp::Property { value, .. } => expr_ok(value, by_id),
            _ => false,
        }),
        Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => {
            if simple_param_ids(params).is_none() {
                return false;
            }
            // Class builder IIFEs are extracted at eval time; skip deep body walk.
            if looks_like_class_iife(body) {
                return true;
            }
            body_ok(body, by_id)
        }
        _ => false,
    }
}

fn looks_like_class_iife(body: &[Stmt]) -> bool {
    let mut saw_strict = false;
    let mut saw_ctor_fn = false;
    let mut saw_return = false;
    for s in body {
        match s {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => saw_strict = true,
            Stmt::Declare {
                init: Some(Expr::Function { .. }),
                ..
            } => saw_ctor_fn = true,
            Stmt::Return {
                value: Some(Expr::Local { .. }),
            } => saw_return = true,
            _ => {}
        }
    }
    saw_strict && saw_ctor_fn && saw_return
}

fn eval_body(body: &[Stmt], ctx: &mut EvalCtx<'_>) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, ctx)? {
            Flow::Normal => {}
            other => return Ok(other),
        }
    }
    Ok(Flow::Normal)
}

fn eval_stmt(stmt: &Stmt, ctx: &mut EvalCtx<'_>) -> Result<Flow, ()> {
    match stmt {
        Stmt::Function { .. } => Ok(Flow::Normal),
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => eval_expr(e, ctx)?,
                None => JsVal::Undef,
            };
            ctx.env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Return { value } => match value {
            None => Ok(Flow::Return(JsVal::Undef)),
            Some(e) => Ok(Flow::Return(eval_expr(e, ctx)?)),
        },
        Stmt::Block { body } => eval_body(body, ctx),
        Stmt::Expr { expr } => {
            eval_expr(expr, ctx)?;
            Ok(Flow::Normal)
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = to_boolean(&eval_expr(test, ctx)?);
            if t {
                eval_stmt(consequent, ctx)
            } else if let Some(a) = alternate {
                eval_stmt(a, ctx)
            } else {
                Ok(Flow::Normal)
            }
        }
        _ => Err(()),
    }
}

fn eval_expr(expr: &Expr, ctx: &mut EvalCtx<'_>) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
            let n: f64 = cleaned.parse().map_err(|_| ())?;
            Ok(JsVal::Num(n))
        }
        Expr::String { value, .. } => Ok(JsVal::Str(value.to_string_lossy())),
        Expr::Boolean { value, .. } => Ok(JsVal::Num(if *value { 1.0 } else { 0.0 })),
        Expr::Null { .. } => Ok(JsVal::Num(0.0)),
        Expr::Local { id, .. } => ctx.env.get(id).cloned().ok_or(()),
        Expr::This { .. } => {
            let id = ctx.this_obj.ok_or(())?;
            Ok(JsVal::Obj(id))
        }
        Expr::NewTarget { .. } | Expr::IdentName { .. } => Ok(JsVal::Undef),
        Expr::Unary { op, arg, .. } => {
            let v = eval_expr(arg, ctx)?;
            match op {
                UnaryOp::Plus => Ok(JsVal::Num(to_number(&v))),
                UnaryOp::Minus => Ok(JsVal::Num(-to_number(&v))),
                UnaryOp::Not => Ok(JsVal::Num(if to_boolean(&v) { 0.0 } else { 1.0 })),

                UnaryOp::Void => Ok(JsVal::Undef),
                UnaryOp::TypeOf => Ok(JsVal::Str(typeof_str(&v).into())),
                UnaryOp::Delete => Ok(JsVal::Num(1.0)),
                _ => Err(()),
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            if matches!(op, BinaryOp::And) {
                let l = eval_expr(left, ctx)?;
                if !to_boolean(&l) {
                    return Ok(l);
                }
                return eval_expr(right, ctx);
            }
            if matches!(op, BinaryOp::Or) {
                let l = eval_expr(left, ctx)?;
                if to_boolean(&l) {
                    return Ok(l);
                }
                return eval_expr(right, ctx);
            }
            if matches!(op, BinaryOp::Comma) {
                eval_expr(left, ctx)?;
                return eval_expr(right, ctx);
            }
            let l = eval_expr(left, ctx)?;
            let r = eval_expr(right, ctx)?;
            match op {
                BinaryOp::Add => {
                    if matches!((&l, &r), (JsVal::Str(_), _) | (_, JsVal::Str(_))) {
                        Ok(JsVal::Str(format!("{}{}", to_string_val(&l), to_string_val(&r))))
                    } else {
                        Ok(JsVal::Num(to_number(&l) + to_number(&r)))
                    }
                }
                BinaryOp::Sub => Ok(JsVal::Num(to_number(&l) - to_number(&r))),
                BinaryOp::Mul => Ok(JsVal::Num(to_number(&l) * to_number(&r))),
                BinaryOp::Div => Ok(JsVal::Num(to_number(&l) / to_number(&r))),
                BinaryOp::Rem => Ok(JsVal::Num(to_number(&l) % to_number(&r))),
                BinaryOp::EqEqEq => Ok(JsVal::Num(if strict_eq(&l, &r) { 1.0 } else { 0.0 })),
                BinaryOp::NotEqEq => Ok(JsVal::Num(if strict_eq(&l, &r) { 0.0 } else { 1.0 })),
                BinaryOp::Lt => Ok(JsVal::Num(if to_number(&l) < to_number(&r) {
                    1.0
                } else {
                    0.0
                })),
                BinaryOp::LtEq => Ok(JsVal::Num(if to_number(&l) <= to_number(&r) {
                    1.0
                } else {
                    0.0
                })),
                BinaryOp::Gt => Ok(JsVal::Num(if to_number(&l) > to_number(&r) {
                    1.0
                } else {
                    0.0
                })),
                BinaryOp::GtEq => Ok(JsVal::Num(if to_number(&l) >= to_number(&r) {
                    1.0
                } else {
                    0.0
                })),
                BinaryOp::In => Ok(JsVal::Num(0.0)),
                _ => Err(()),
            }
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = eval_expr(value, ctx)?;
            ctx.env.insert(*id, v.clone());
            Ok(v)
        }
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    computed: false,
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let obj = eval_expr(object, ctx)?;
            let key = prop_key(property, ctx)?;
            let v = eval_expr(value, ctx)?;
            let JsVal::Obj(oid) = obj else {
                return Err(());
            };
            let inst = ctx.heap.get_mut(&oid).ok_or(())?;
            inst.props.insert(key, v.clone());
            Ok(v)
        }
        Expr::Array { elements, .. } => {
            let mut items = Vec::with_capacity(elements.len());
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => items.push(eval_expr(e, ctx)?),
                    _ => return Err(()),
                }
            }
            Ok(JsVal::Arr(items))
        }
        Expr::Object { .. } => Ok(JsVal::Undef),
        Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => {
            let param_ids = simple_param_ids(params).ok_or(())?;
            Ok(JsVal::FnExpr(FnRec {
                params: param_ids,
                body: body.clone(),
            }))
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = eval_expr(object, ctx)?;
            let key = prop_key(property, ctx)?;
            match obj {

                JsVal::Ns(map) => {
                    let getter = map.get(&key).cloned().ok_or(())?;
                    call_value(getter, &[], None, ctx)
                }
                JsVal::Obj(oid) => {
                    let inst = ctx.heap.get(&oid).ok_or(())?.clone();
                    if let Some(v) = inst.props.get(&key) {
                        return Ok(v.clone());
                    }
                    if let Some(m) = inst.methods.get(&key) {
                        return Ok(JsVal::FnExpr(m.clone()));
                    }
                    Err(())
                }
                JsVal::Class(c) => {
                    if let Some(m) = c.methods.get(&key) {
                        return Ok(JsVal::FnExpr(m.clone()));
                    }
                    Err(())
                }
                _ => Err(()),
            }
        }
        Expr::New { callee, args, .. } => {
            let c = eval_expr(callee, ctx)?;
            let mut arg_vals = Vec::with_capacity(args.len());
            for a in args {
                match a {
                    Arg::Expr(e) => arg_vals.push(eval_expr(e, ctx)?),
                    _ => return Err(()),
                }
            }
            let JsVal::Class(class) = c else {
                return Err(());
            };
            let oid = ctx.next_obj;
            ctx.next_obj += 1;
            ctx.heap.insert(
                oid,
                InstRec {
                    props: HashMap::new(),
                    methods: class.methods.clone(),
                },
            );
            let prev_this = ctx.this_obj.replace(oid);
            let _ = call_fnrec(&class.ctor, &arg_vals, Some(oid), ctx)?;
            ctx.this_obj = prev_this;
            Ok(JsVal::Obj(oid))
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            // Method call: recv.m(args) — bind `this` when callee is Member on instance.
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = callee.as_ref()
            {
                let recv = eval_expr(object, ctx)?;
                let key = prop_key(property, ctx)?;
                let mut arg_vals = Vec::with_capacity(args.len());
                for a in args {
                    match a {
                        Arg::Expr(e) => arg_vals.push(eval_expr(e, ctx)?),
                        _ => return Err(()),
                    }
                }
                if let JsVal::Obj(oid) = recv {
                    let inst = ctx.heap.get(&oid).ok_or(())?.clone();
                    let method = inst.methods.get(&key).cloned().ok_or(())?;
                    return call_fnrec(&method, &arg_vals, Some(oid), ctx);
                }
                if let JsVal::Ns(map) = recv {
                    let getter = map.get(&key).cloned().ok_or(())?;
                    // Namespace export accessed as call target is unusual; fall through.
                    let c = call_value(getter, &[], None, ctx)?;
                    return call_value(c, &arg_vals, None, ctx);
                }
                return Err(());
            }

            let c = eval_expr(callee, ctx)?;
            let mut arg_vals = Vec::with_capacity(args.len());
            for a in args {
                match a {
                    Arg::Expr(e) => arg_vals.push(eval_expr(e, ctx)?),
                    _ => return Err(()),
                }
            }
            // Class builder IIFE: ` (function(){ … return ctor })() `
            if let JsVal::FnExpr(f) = &c {
                if f.params.is_empty() && looks_like_class_iife(&f.body) {
                    return extract_class_iife(&f.body);
                }
            }
            if let JsVal::Fn(fid) = &c {
                if ctx.make_ns == Some(*fid) {
                    return eval_make_ns(&arg_vals);
                }
            }
            call_value(c, &arg_vals, None, ctx)
        }
        _ => Err(()),
    }
}

fn prop_key(property: &Expr, ctx: &mut EvalCtx<'_>) -> Result<String, ()> {
    match property {
        Expr::String { value, .. } => Ok(value.to_string_lossy()),
        other => match eval_expr(other, ctx)? {
            JsVal::Str(s) => Ok(s),
            JsVal::Num(n) if n.is_finite() && n.fract() == 0.0 => Ok(format!("{}", n as i64)),
            _ => Err(()),
        },
    }
}

fn extract_class_iife(body: &[Stmt]) -> Result<JsVal, ()> {
    let mut ctor: Option<FnRec> = None;
    let mut methods: HashMap<String, FnRec> = HashMap::new();
    let mut pending_key: Option<String> = None;
    let mut ctor_local: Option<LocalId> = None;

    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            Stmt::Declare {
                local,
                init:
                    Some(Expr::Function {
                        params,
                        body: cbody,
                        is_async: false,
                        is_generator: false,
                        is_arrow: false,
                        ..
                    }),
                ..
            } if ctor.is_none() => {
                let param_ids = simple_param_ids(params).ok_or(())?;
                ctor = Some(FnRec {
                    params: param_ids,
                    body: filter_ctor_body(cbody),
                });
                ctor_local = Some(*local);
            }
            Stmt::Declare {
                init: Some(Expr::String { value, .. }),
                ..
            } => {
                pending_key = Some(value.to_string_lossy());
            }
            Stmt::Expr {
                expr:
                    Expr::Call {
                        callee,
                        args,
                        ..
                    },
            } if is_object_define_property(callee) && args.len() == 3 => {
                let Some(cl) = ctor_local else {
                    continue;
                };
                // Skip defineProperty on the constructor itself (name/prototype).
                if matches!(&args[0], Arg::Expr(Expr::Local { id, .. }) if *id == cl) {
                    pending_key = None;
                    continue;
                }
                if !is_define_on_proto(args, cl) {
                    return Err(());
                }
                let key = pending_key
                    .take()
                    .or_else(|| string_arg(&args[1]))
                    .ok_or(())?;
                let Arg::Expr(desc) = &args[2] else {
                    return Err(());
                };
                let method_fn = find_method_function(desc).ok_or(())?;
                let Expr::Function {
                    params,
                    body: mbody,
                    is_async: false,
                    is_generator: false,
                    ..
                } = method_fn
                else {
                    return Err(());
                };
                let param_ids = simple_param_ids(params).ok_or(())?;
                methods.insert(
                    key,
                    FnRec {
                        params: param_ids,
                        body: filter_method_body(mbody),
                    },
                );
            }
            Stmt::Return {
                value: Some(Expr::Local { id, .. }),
            } if Some(*id) == ctor_local => {}
            _ => return Err(()),
        }
    }

    Ok(JsVal::Class(ClassRec {
        ctor: ctor.ok_or(())?,
        methods,
    }))
}

fn is_object_define_property(callee: &Expr) -> bool {
    matches!(
        callee,
        Expr::Member {
            object,
            property,
            ..
        } if matches!(
            (object.as_ref(), property.as_ref()),
            (
                Expr::IdentName { name, .. },
                Expr::String { value, .. }
            ) if name == "Object" && value.to_string_lossy() == "defineProperty"
        )
    )
}

fn is_define_on_proto(args: &[Arg], ctor: LocalId) -> bool {
    matches!(
        &args[0],
        Arg::Expr(Expr::Member {
            object,
            property,
            ..
        }) if matches!(
            (object.as_ref(), property.as_ref()),
            (
                Expr::Local { id, .. },
                Expr::String { value, .. }
            ) if *id == ctor && value.to_string_lossy() == "prototype"
        )
    )
}

fn string_arg(arg: &Arg) -> Option<String> {
    match arg {
        Arg::Expr(Expr::String { value, .. }) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn find_method_function(expr: &Expr) -> Option<&Expr> {
    match expr {
        Expr::Function {
            is_method: true, ..
        } => Some(expr),
        Expr::Function { body, .. } => {
            for s in body {
                if let Some(f) = find_method_function_in_stmt(s) {
                    return Some(f);
                }
            }
            None
        }
        Expr::Call { callee, args, .. } => {
            if let Some(f) = find_method_function(callee) {
                return Some(f);
            }
            for a in args {
                if let Arg::Expr(e) = a {
                    if let Some(f) = find_method_function(e) {
                        return Some(f);
                    }
                }
            }
            None
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                if let ObjectProp::Property { value, .. } = p {
                    if let Some(f) = find_method_function(value) {
                        return Some(f);
                    }
                }
            }
            None
        }
        Expr::Member {
            object, property, ..
        } => find_method_function(object).or_else(|| find_method_function(property)),
        Expr::Binary { left, right, .. } => {
            find_method_function(left).or_else(|| find_method_function(right))
        }
        Expr::Assign { value, .. } => find_method_function(value),
        Expr::Unary { arg, .. } => find_method_function(arg),
        _ => None,
    }
}

fn find_method_function_in_stmt(stmt: &Stmt) -> Option<&Expr> {
    match stmt {
        Stmt::Expr { expr } | Stmt::Return { value: Some(expr) } => find_method_function(expr),
        Stmt::Declare {
            init: Some(expr), ..
        } => find_method_function(expr),
        Stmt::Block { body } => body.iter().find_map(find_method_function_in_stmt),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => find_method_function(test)
            .or_else(|| find_method_function_in_stmt(consequent))
            .or_else(|| alternate.as_ref().and_then(|a| find_method_function_in_stmt(a))),
        _ => None,
    }
}

fn filter_ctor_body(body: &[Stmt]) -> Vec<Stmt> {
    body.iter()
        .filter(|s| {
            matches!(
                s,
                Stmt::Expr {
                    expr: Expr::Assign {
                        target: AssignTarget::Member { .. },
                        op: AssignOp::Eq,
                        ..
                    },
                } | Stmt::Return { .. }
                    | Stmt::Block { .. }
            )
        })
        .cloned()
        .collect()
}

fn filter_method_body(body: &[Stmt]) -> Vec<Stmt> {
    body.iter()
        .filter(|s| {
            !matches!(
                s,
                Stmt::Expr {
                    expr: Expr::String { value, .. },
                } if value.to_string_lossy() == "use strict"
            )
        })
        .cloned()
        .collect()
}

/// Build a namespace object from `__draconic_make_ns(pairs, names, tag)` args.
/// Each pair is `[exportName, getterFn]`; getters run on property access (live bindings).
fn eval_make_ns(args: &[JsVal]) -> Result<JsVal, ()> {
    if args.is_empty() {
        return Err(());
    }
    let JsVal::Arr(pairs) = &args[0] else {
        return Err(());
    };
    let mut map = HashMap::new();
    for p in pairs {
        let JsVal::Arr(kv) = p else {
            return Err(());
        };
        if kv.len() != 2 {
            return Err(());
        }
        let name = match &kv[0] {
            JsVal::Str(s) => s.clone(),
            _ => return Err(()),
        };
        match &kv[1] {
            JsVal::Fn(_) | JsVal::FnExpr(_) => {}
            _ => return Err(()),
        }
        map.insert(name, kv[1].clone());
    }
    Ok(JsVal::Ns(map))
}

fn call_value(
    callee: JsVal,
    arg_vals: &[JsVal],
    this_obj: Option<u32>,
    ctx: &mut EvalCtx<'_>,
) -> Result<JsVal, ()> {
    match callee {
        JsVal::Fn(fid) => {
            let frec = ctx.functions.get(&fid).ok_or(())?.clone();
            call_fnrec(&frec, arg_vals, this_obj, ctx)
        }
        JsVal::FnExpr(f) => call_fnrec(&f, arg_vals, this_obj, ctx),
        JsVal::Class(_) => Err(()),
        _ => Err(()),
    }
}

fn call_fnrec(
    frec: &FnRec,
    arg_vals: &[JsVal],
    this_obj: Option<u32>,
    ctx: &mut EvalCtx<'_>,
) -> Result<JsVal, ()> {
    if arg_vals.len() > frec.params.len() {
        return Err(());
    }
    for (i, pid) in frec.params.iter().enumerate() {
        let v = arg_vals.get(i).cloned().unwrap_or(JsVal::Undef);
        ctx.env.insert(*pid, v);
    }
    let prev_this = ctx.this_obj;
    if this_obj.is_some() {
        ctx.this_obj = this_obj;
    }
    let flow = eval_body(&frec.body, ctx);
    ctx.this_obj = prev_this;
    match flow? {
        Flow::Normal => Ok(JsVal::Undef),
        Flow::Return(v) => Ok(v),
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Undef => "undefined".to_string(),
        JsVal::Num(_) => "number".to_string(),
        JsVal::Str(_) => "string".to_string(),
        JsVal::Fn(_) | JsVal::FnExpr(_) => "function".to_string(),
        JsVal::Ns(_) | JsVal::Arr(_) => "object".to_string(),
    }
}

fn to_number(v: &JsVal) -> f64 {
    match v {
        JsVal::Num(n) => *n,
        JsVal::Str(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse().unwrap_or(f64::NAN)
            }
        }
        JsVal::Undef => f64::NAN,
        JsVal::Fn(_)
        | JsVal::FnExpr(_)
        | JsVal::Class(_)
        | JsVal::Obj(_)
        | JsVal::Ns(_)
        | JsVal::Arr(_) => f64::NAN,
    }
}

fn to_string_val(v: &JsVal) -> String {
    match v {
        JsVal::Num(n) => {
            if n.is_nan() {
                "NaN".into()
            } else if n.is_infinite() {
                if n.is_sign_negative() {
                    "-Infinity".into()
                } else {
                    "Infinity".into()
                }
            } else if *n == 0.0 {
                "0".into()
            } else {
                format!("{n}")
            }
        }
        JsVal::Str(s) => s.clone(),
        JsVal::Undef => "undefined".into(),
        _ => "[object Object]".into(),
    }
}

fn typeof_str(v: &JsVal) -> &'static str {
    match v {
        JsVal::Num(_) => "number",
        JsVal::Str(_) => "string",
        JsVal::Undef => "undefined",
        JsVal::Fn(_) | JsVal::FnExpr(_) | JsVal::Class(_) => "function",
        JsVal::Obj(_) | JsVal::Ns(_) | JsVal::Arr(_) => "object",
    }
}

fn strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => x == y,
        (JsVal::Str(x), JsVal::Str(y)) => x == y,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Obj(x), JsVal::Obj(y)) => x == y,
        _ => false,
    }
}

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef => false,
        JsVal::Fn(_)
        | JsVal::FnExpr(_)
        | JsVal::Class(_)
        | JsVal::Obj(_)
        | JsVal::Ns(_) => true,
        JsVal::Arr(a) => !a.is_empty(),
    }
}

struct Emitter {
    out: String,
    body: String,
    str_globals: Vec<(Vec<u8>, String)>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
            str_globals: Vec::new(),
        }
    }

    fn string_const(&mut self, s: &str) -> (String, usize) {
        let bytes = s.as_bytes().to_vec();
        let len = bytes.len();
        let name = format!("@.s{}", self.str_globals.len());
        self.str_globals.push((bytes, name.clone()));
        let data = format!(
            "getelementptr inbounds ([{n} x i8], ptr {name}, i64 0, i64 0)",
            n = len + 1
        );
        (data, len)
    }

    fn emit_num(&mut self, n: f64) {
        let lit = if n.is_nan() {
            "0x7FF8000000000000".to_string()
        } else if n.is_infinite() {
            if n.is_sign_negative() {
                "0xFFF0000000000000".into()
            } else {
                "0x7FF0000000000000".into()
            }
        } else {
            format!("{n:?}")
        };
        writeln!(
            self.body,
            "  {}",
            PRINT_F64.call(&format!("double {lit}"))
        )
        .ok();
    }

    fn emit_str(&mut self, s: &str) {
        let (data, len) = self.string_const(s);
        writeln!(
            self.body,
            "  {}",
            PRINT_BYTES.call(&format!("ptr {data}, i64 {len}"))
        )
        .ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_modules: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => self.emit_str(s),
                _ => return Err(diag("es_modules: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.11 linked ESM modules, incl. namespace)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        let mut globals: Vec<(Vec<u8>, String)> = self.str_globals.clone();
        globals.sort_by(|a, b| a.1.cmp(&b.1));
        for (bytes, name) in globals {
            let n = bytes.len() + 1;
            let mut esc = String::new();
            for &b in &bytes {
                match b {
                    b'\\' => esc.push_str("\\5C"),
                    b'"' => esc.push_str("\\22"),
                    c if (0x20..0x7f).contains(&c) => esc.push(c as char),
                    c => esc.push_str(&format!("\\{c:02X}")),
                }
            }
            writeln!(
                self.out,
                "{name} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        writeln!(self.out, "\ndefine i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        self.out.push_str(&self.body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn finish(self) -> String {
        self.out
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}


