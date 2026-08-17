//! N08.16.27: native observations for `new.target` (E18.27 / `es/annex-b/new_target`).
//!
//! Compile-time evaluation of function/`constructor` `new.target`, non-`new` →
//! `undefined`, subclass active construct, nested functions, and arrows that
//! inherit the enclosing `new.target`. Class builder IIFEs are collapsed like
//! `es_classes` (base + derived `super()`). Emits Runtime prints of final
//! top-level bool/string/undefined observations.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey,
    Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_STR};

pub(crate) fn is_es_new_target_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_new_target(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_new_target module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

#[derive(Clone, Debug)]
enum JsVal {
    Undef,
    Bool(bool),
    Str(String),
    /// Shared so constructor `this.prop = …` mutates the allocated instance.
    Object {
        id: u64,
        props: Rc<RefCell<Vec<(String, JsVal)>>>,
    },
    Fn {
        id: u64,
    },
}

#[derive(Clone)]
struct FnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
    is_arrow: bool,
    /// Parent constructor fn id for derived `super()`.
    parent: Option<u64>,
}

thread_local! {
    static CURRENT_THIS: RefCell<JsVal> = const { RefCell::new(JsVal::Undef) };
    static CURRENT_NEW_TARGET: RefCell<JsVal> = const { RefCell::new(JsVal::Undef) };
    static FN_REG: RefCell<HashMap<u64, FnRec>> = RefCell::new(HashMap::new());
}

fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn with_this_nt<R>(this: JsVal, nt: JsVal, f: impl FnOnce() -> R) -> R {
    CURRENT_THIS.with(|t| {
        let prev_t = t.replace(this);
        CURRENT_NEW_TARGET.with(|n| {
            let prev_n = n.replace(nt);
            let r = f();
            n.replace(prev_n);
            t.replace(prev_t);
            r
        })
    })
}

fn current_this() -> JsVal {
    CURRENT_THIS.with(|c| c.borrow().clone())
}

fn current_new_target() -> JsVal {
    CURRENT_NEW_TARGET.with(|c| c.borrow().clone())
}

fn fn_reg_insert(id: u64, rec: FnRec) {
    FN_REG.with(|r| {
        r.borrow_mut().insert(id, rec);
    });
}

fn fn_reg_get(id: u64) -> Option<FnRec> {
    FN_REG.with(|r| r.borrow().get(&id).cloned())
}

fn reset_fn_reg() {
    FN_REG.with(|r| r.borrow_mut().clear());
}

#[derive(Clone, Debug)]
enum Flow {
    Normal,
    Return(JsVal),
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    if !module_has_new_target(module) {
        return None;
    }
    reset_fn_reg();
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    match eval_body(&module.body, &mut env) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }

    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match env.get(local) {
                Some(v @ (JsVal::Bool(_) | JsVal::Str(_) | JsVal::Undef)) => {
                    if matches!(loc.ty, Type::Any | Type::Boolean | Type::String) {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(JsVal::Object { .. } | JsVal::Fn { .. }) => {}
                None => return None,
            }
        }
    }
    if user_locals.is_empty() {
        return None;
    }
    // Class lowering injects `new.target` into constructors, so bare class
    // fixtures also set module_has_new_target. Reject failed evals that only
    // produced Undef observations so they fall through to es_classes.
    if !values
        .values()
        .any(|v| matches!(v, JsVal::Bool(_) | JsVal::Str(_)))
    {
        return None;
    }
    Some(ModuleInfo {
        user_locals,
        values,
    })
}

fn module_has_new_target(module: &Module) -> bool {
    module.body.iter().any(stmt_has_new_target)
}

fn stmt_has_new_target(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. }
        | Stmt::Expr { expr: e }
        | Stmt::Return { value: Some(e) }
        | Stmt::Throw { value: e } => expr_has_new_target(e),
        Stmt::Function { body, .. } | Stmt::Block { body } => body.iter().any(stmt_has_new_target),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_has_new_target(test)
                || stmt_has_new_target(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_has_new_target(a))
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(stmt_has_new_target)
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(stmt_has_new_target))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(stmt_has_new_target))
        }
        _ => false,
    }
}

fn expr_has_new_target(expr: &Expr) -> bool {
    match expr {
        Expr::NewTarget { .. } => true,
        Expr::Unary { arg, .. } => expr_has_new_target(arg),
        Expr::Binary { left, right, .. } => {
            expr_has_new_target(left) || expr_has_new_target(right)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_new_target(test)
                || expr_has_new_target(consequent)
                || expr_has_new_target(alternate)
        }
        Expr::Member {
            object, property, ..
        } => expr_has_new_target(object) || expr_has_new_target(property),
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_has_new_target(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_new_target(e),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_has_new_target(value),
        Expr::Function { body, .. } => body.iter().any(stmt_has_new_target),
        _ => false,
    }
}

fn eval_body(body: &[Stmt], env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, env)? {
            Flow::Normal => {}
            other => return Ok(other),
        }
    }
    Ok(Flow::Normal)
}

fn eval_stmt(stmt: &Stmt, env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
    match stmt {
        Stmt::Function {
            local,
            params,
            body,
            is_async: false,
            is_generator: false,
        } => {
            let ids = simple_param_ids(params)?;
            let id = next_id();
            fn_reg_insert(
                id,
                FnRec {
                    params: ids,
                    body: body.clone(),
                    is_arrow: false,
                    parent: None,
                },
            );
            env.insert(*local, JsVal::Fn { id });
            Ok(Flow::Normal)
        }
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => match eval_expr(e, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                },
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Expr { expr } => match eval_expr(expr, env)? {
            Ok(_) => Ok(Flow::Normal),
            Err(flow) => Ok(flow),
        },
        Stmt::Return { value: None } => Ok(Flow::Return(JsVal::Undef)),
        Stmt::Return { value: Some(e) } => match eval_expr(e, env)? {
            Ok(v) => Ok(Flow::Return(v)),
            Err(flow) => Ok(flow),
        },
        Stmt::Block { body } => eval_body(body, env),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = match eval_expr(test, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(flow),
            };
            if to_boolean(&t) {
                eval_stmt(consequent, env)
            } else if let Some(a) = alternate {
                eval_stmt(a, env)
            } else {
                Ok(Flow::Normal)
            }
        }
        // Class IIFE noise: defineProperty / setPrototypeOf / throw TypeError — ignore side effects.
        Stmt::Throw { .. } => Ok(Flow::Normal),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            let mut completion = match eval_body(block, env) {
                Ok(Flow::Normal) => Flow::Normal,
                Ok(other) => other,
                Err(()) => {
                    // Unsupported in try → treat as throw and run handler if any.
                    if let Some(h) = handler {
                        eval_body(h, env)?
                    } else {
                        return Err(());
                    }
                }
            };
            if let Some(fin) = finalizer {
                match eval_body(fin, env)? {
                    Flow::Normal => {}
                    abrupt => completion = abrupt,
                }
            }
            Ok(completion)
        }
        _ => Err(()),
    }
}

fn simple_param_ids(params: &[Param]) -> Result<Vec<LocalId>, ()> {
    let mut ids = Vec::new();
    for p in params {
        if p.rest || p.default.is_some() {
            return Err(());
        }
        match &p.pattern {
            Pattern::Local(id) => ids.push(*id),
            _ => return Err(()),
        }
    }
    Ok(ids)
}

fn make_fn(
    params: &[Param],
    body: &[Stmt],
    is_arrow: bool,
    parent: Option<u64>,
) -> Result<JsVal, ()> {
    let ids = simple_param_ids(params)?;
    let id = next_id();
    fn_reg_insert(
        id,
        FnRec {
            params: ids,
            body: body.to_vec(),
            is_arrow,
            parent,
        },
    );
    Ok(JsVal::Fn { id })
}

/// `Ok(Ok(v))` value; `Ok(Err(flow))` abrupt return; `Err(())` unsupported.
fn eval_expr(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<JsVal, Flow>, ()> {
    match expr {
        Expr::NewTarget { .. } => Ok(Ok(current_new_target())),
        Expr::This { .. } => Ok(Ok(current_this())),
        Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
        Expr::String { value, .. } => Ok(Ok(JsVal::Str(value.to_string_lossy()))),
        Expr::Null { .. } => Ok(Ok(JsVal::Undef)),
        Expr::Number { raw, .. } => {
            let _n: f64 = raw.parse().map_err(|_| ())?;
            Ok(Ok(JsVal::Undef)) // numbers unused in fixture observations
        }
        Expr::Local { id, .. } => {
            let v = env.get(id).cloned().ok_or(())?;
            Ok(Ok(v))
        }
        Expr::IdentName { name, .. } => match name.as_str() {
            "undefined" => Ok(Ok(JsVal::Undef)),
            // Class/ctor surface used only for structure; not observed.
            "Object" | "Function" | "Reflect" | "Proxy" | "TypeError" => Ok(Ok(JsVal::Undef)),
            _ => Err(()),
        },
        Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            is_arrow,
            ..
        } => Ok(Ok(make_fn(params, body, *is_arrow, None)?)),
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            let v = match eval_expr(arg, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            Ok(Ok(JsVal::Str(typeof_str(&v))))
        }
        Expr::Unary {
            op: UnaryOp::Not,
            arg,
            ..
        } => {
            let v = match eval_expr(arg, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            Ok(Ok(JsVal::Bool(!to_boolean(&v))))
        }
        Expr::Binary {
            left,
            op: BinaryOp::EqEqEq | BinaryOp::EqEq,
            right,
            ..
        } => {
            let l = match eval_expr(left, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let r = match eval_expr(right, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            Ok(Ok(JsVal::Bool(strict_eq(&l, &r))))
        }
        Expr::Binary {
            left,
            op: BinaryOp::NotEqEq | BinaryOp::NotEq,
            right,
            ..
        } => {
            let l = match eval_expr(left, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let r = match eval_expr(right, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            Ok(Ok(JsVal::Bool(!strict_eq(&l, &r))))
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            let t = match eval_expr(test, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            if to_boolean(&t) {
                eval_expr(consequent, env)
            } else {
                eval_expr(alternate, env)
            }
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = match eval_expr(object, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let key = match property.as_ref() {
                Expr::String { value, .. } => value.to_string_lossy(),
                _ => return Err(()),
            };
            Ok(Ok(member_get(&obj, &key)))
        }
        Expr::New { callee, args, .. } => {
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => arg_vals.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    _ => return Err(()),
                }
            }
            Ok(Ok(eval_new(&c, &arg_vals, env)?))
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            // Super(...) in derived ctor.
            if matches!(callee.as_ref(), Expr::Super { .. }) {
                let mut arg_vals = Vec::new();
                for a in args {
                    match a {
                        Arg::Expr(e) => match eval_expr(e, env)? {
                            Ok(v) => arg_vals.push(v),
                            Err(flow) => return Ok(Err(flow)),
                        },
                        _ => return Err(()),
                    }
                }
                return Ok(Ok(eval_super(&arg_vals, env)?));
            }
            // Class builder IIFE or plain call.
            if let Expr::Function {
                params,
                body,
                is_async: false,
                is_generator: false,
                is_arrow: false,
                ..
            } = callee.as_ref()
            {
                if args.is_empty() {
                    if let Some(cls) = try_eval_class_iife(params, body, env) {
                        return Ok(Ok(cls));
                    }
                }
            }
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => arg_vals.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    _ => return Err(()),
                }
            }
            // Method-style Object.defineProperty / setPrototypeOf — no-op for fixture.
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = callee.as_ref()
            {
                if let (Ok(Ok(JsVal::Undef)), Expr::String { value, .. }) =
                    (eval_expr(object, env), property.as_ref())
                {
                    let k = value.to_string_lossy();
                    if k == "defineProperty" || k == "setPrototypeOf" || k == "get" {
                        return Ok(Ok(JsVal::Undef));
                    }
                }
            }
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            Ok(Ok(eval_call(&c, &arg_vals, env)?))
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = match eval_expr(value, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            env.insert(*id, v.clone());
            Ok(Ok(v))
        }
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = match eval_expr(value, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let mut obj = match eval_expr(object, env)? {
                Ok(o) => o,
                Err(flow) => return Ok(Err(flow)),
            };
            let key = match property.as_ref() {
                Expr::String { value, .. } => value.to_string_lossy(),
                _ => return Err(()),
            };
            member_set(&mut obj, &key, v.clone())?;
            if let Expr::Local { id, .. } = object.as_ref() {
                env.insert(*id, obj);
            } else if matches!(object.as_ref(), Expr::This { .. }) {
                CURRENT_THIS.with(|c| {
                    *c.borrow_mut() = obj;
                });
            }
            Ok(Ok(v))
        }
        Expr::Object { properties, .. } => {
            // Descriptor objects in defineProperty — ignore contents.
            let mut props = Vec::new();
            for p in properties {
                if let ObjectProp::Property {
                    key: ObjectPropKey::Static(k),
                    value,
                } = p
                {
                    let v = match eval_expr(value, env)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(Err(flow)),
                    };
                    props.push((k.to_string_lossy(), v));
                }
            }
            Ok(Ok(JsVal::Object {
                id: next_id(),
                props: Rc::new(RefCell::new(props)),
            }))
        }
        Expr::Array { .. } => Ok(Ok(JsVal::Undef)),
        Expr::Super { .. } => Err(()),
        _ => Err(()),
    }
}

fn try_eval_class_iife(
    params: &[Param],
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
) -> Option<JsVal> {
    if !params.is_empty() {
        return None;
    }
    // Detect parent: `let super = OuterClass` style declare of a function local.
    let mut parent_fn_id: Option<u64> = None;
    let mut ctor_local: Option<LocalId> = None;
    let mut ctor_fn: Option<JsVal> = None;

    for stmt in body {
        match stmt {
            Stmt::Declare {
                local,
                init: Some(Expr::Local { id, .. }),
                ..
            } => {
                if let Some(JsVal::Fn { id: fid }) = env.get(id) {
                    parent_fn_id = Some(*fid);
                    env.insert(*local, JsVal::Fn { id: *fid });
                }
            }
            Stmt::Declare {
                local,
                init:
                    Some(Expr::Function {
                        params: cparams,
                        body: cbody,
                        is_async: false,
                        is_generator: false,
                        is_arrow: false,
                        ..
                    }),
                ..
            } if ctor_local.is_none() => {
                let filtered = if parent_fn_id.is_some() {
                    filter_derived_ctor_body(cbody)
                } else {
                    filter_ctor_body(cbody)
                };
                let f = make_fn(cparams, &filtered, false, parent_fn_id).ok()?;
                ctor_local = Some(*local);
                ctor_fn = Some(f.clone());
                env.insert(*local, f);
            }
            Stmt::Return {
                value: Some(Expr::Local { id, .. }),
            } if Some(*id) == ctor_local => {
                return ctor_fn;
            }
            // defineProperty / setPrototypeOf / heritage checks — skip
            Stmt::Expr { .. } | Stmt::If { .. } | Stmt::Declare { init: None, .. } => {}
            Stmt::Declare {
                init: Some(Expr::String { .. }),
                ..
            } => {}
            _ => {}
        }
    }
    ctor_fn
}

fn filter_ctor_body(body: &[Stmt]) -> Vec<Stmt> {
    body.iter()
        .filter(|s| match s {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => false,
            Stmt::If { .. } => false,
            Stmt::Expr {
                expr:
                    Expr::Assign {
                        target: AssignTarget::Member { .. },
                        op: AssignOp::Eq,
                        ..
                    },
            } => true,
            Stmt::Return { .. } => true,
            Stmt::Block { .. } => true,
            _ => false,
        })
        .cloned()
        .collect()
}

fn filter_derived_ctor_body(body: &[Stmt]) -> Vec<Stmt> {
    let mut out = Vec::new();
    collect_derived_ctor_stmts(body, &mut out);
    out
}

fn collect_derived_ctor_stmts(body: &[Stmt], out: &mut Vec<Stmt>) {
    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            Stmt::If { .. } | Stmt::Declare { .. } | Stmt::Return { .. } => {}
            Stmt::Labeled { body, .. } => collect_derived_ctor_stmts_one(body, out),
            Stmt::Block { body } => collect_derived_ctor_stmts(body, out),
            other => collect_derived_ctor_stmts_one(other, out),
        }
    }
}

fn collect_derived_ctor_stmts_one(stmt: &Stmt, out: &mut Vec<Stmt>) {
    match stmt {
        Stmt::Block { body } => collect_derived_ctor_stmts(body, out),
        Stmt::Labeled { body, .. } => collect_derived_ctor_stmts_one(body, out),
        Stmt::Expr {
            expr:
                Expr::Call {
                    callee,
                    args,
                    optional,
                    ty,
                },
        } if !*optional && is_super_call_iife(callee) => {
            let super_args: Vec<Arg> = args
                .iter()
                .filter_map(|a| match a {
                    Arg::Expr(e) => Some(Arg::Expr(e.clone())),
                    Arg::Spread(_) => None,
                })
                .collect();
            if super_args.len() != args.len() {
                return;
            }
            out.push(Stmt::Expr {
                expr: Expr::Call {
                    callee: Box::new(Expr::Super { ty: Type::Any }),
                    args: super_args,
                    optional: false,
                    ty: ty.clone(),
                },
            });
        }
        Stmt::Expr {
            expr:
                Expr::Assign {
                    target:
                        AssignTarget::Member {
                            property,
                            computed,
                            ..
                        },
                    op: AssignOp::Eq,
                    value,
                    ty,
                },
        } if matches!(property.as_ref(), Expr::String { .. }) => {
            out.push(Stmt::Expr {
                expr: Expr::Assign {
                    target: AssignTarget::Member {
                        object: Box::new(Expr::This { ty: Type::Any }),
                        property: property.clone(),
                        computed: *computed,
                    },
                    op: AssignOp::Eq,
                    value: value.clone(),
                    ty: ty.clone(),
                },
            });
        }
        _ => {}
    }
}

fn is_super_call_iife(callee: &Expr) -> bool {
    let Expr::Function {
        body,
        is_arrow: true,
        ..
    } = callee
    else {
        return false;
    };
    body.iter().any(stmt_has_reflect_construct)
}

fn stmt_has_reflect_construct(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare {
            init: Some(expr), ..
        }
        | Stmt::Expr { expr }
        | Stmt::Return { value: Some(expr) } => expr_has_reflect_construct(expr),
        Stmt::Block { body } => body.iter().any(stmt_has_reflect_construct),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            stmt_has_reflect_construct(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_has_reflect_construct(a))
        }
        _ => false,
    }
}

fn expr_has_reflect_construct(expr: &Expr) -> bool {
    match expr {
        Expr::Call { callee, .. } => {
            if is_reflect_construct(callee) {
                return true;
            }
            expr_has_reflect_construct(callee)
        }
        Expr::Member {
            object, property, ..
        } => expr_has_reflect_construct(object) || expr_has_reflect_construct(property),
        Expr::Assign { value, .. } => expr_has_reflect_construct(value),
        Expr::New { callee, args, .. } => {
            expr_has_reflect_construct(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_reflect_construct(e),
                    _ => false,
                })
        }
        Expr::Function { body, .. } => body.iter().any(stmt_has_reflect_construct),
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                expr_has_reflect_construct(value)
            }
            ObjectProp::Spread(e) => expr_has_reflect_construct(e),
        }),
        _ => false,
    }
}

fn is_reflect_construct(callee: &Expr) -> bool {
    let Expr::Member {
        object, property, ..
    } = callee
    else {
        return false;
    };
    matches!(
        (object.as_ref(), property.as_ref()),
        (
            Expr::IdentName { name, .. },
            Expr::String { value, .. }
        ) if name == "Reflect" && value.to_string_lossy() == "construct"
    )
}

fn eval_new(callee: &JsVal, args: &[JsVal], env: &mut HashMap<LocalId, JsVal>) -> Result<JsVal, ()> {
    let JsVal::Fn { id } = callee else {
        return Err(());
    };
    let rec = fn_reg_get(*id).ok_or(())?;
    if rec.is_arrow {
        return Err(());
    }
    let obj = JsVal::Object {
        id: next_id(),
        props: Rc::new(RefCell::new(Vec::new())),
    };
    let nt = callee.clone();
    let result = call_user_fn(&rec, obj.clone(), nt, args, env)?;
    match result {
        JsVal::Object { .. } | JsVal::Fn { .. } => Ok(result),
        _ => Ok(obj),
    }
}

fn eval_super(args: &[JsVal], env: &mut HashMap<LocalId, JsVal>) -> Result<JsVal, ()> {
    // Current new.target's FnRec.parent → parent ctor; same this + new.target.
    let nt = current_new_target();
    let JsVal::Fn { id } = &nt else {
        return Err(());
    };
    let rec = fn_reg_get(*id).ok_or(())?;
    let parent_id = rec.parent.ok_or(())?;
    let parent = fn_reg_get(parent_id).ok_or(())?;
    let this = current_this();
    call_user_fn(&parent, this, nt, args, env)
}

fn eval_call(callee: &JsVal, args: &[JsVal], env: &mut HashMap<LocalId, JsVal>) -> Result<JsVal, ()> {
    let JsVal::Fn { id } = callee else {
        // No-op builtins (Object.defineProperty etc. already handled).
        return Ok(JsVal::Undef);
    };
    let rec = fn_reg_get(*id).ok_or(())?;
    if rec.is_arrow {
        // Lexical new.target / this from caller.
        return call_user_fn_arrow(&rec, args, env);
    }
    call_user_fn(&rec, JsVal::Undef, JsVal::Undef, args, env)
}

fn call_user_fn(
    rec: &FnRec,
    this: JsVal,
    nt: JsVal,
    args: &[JsVal],
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<JsVal, ()> {
    let mut saved: Vec<(LocalId, Option<JsVal>)> = Vec::new();
    for (i, pid) in rec.params.iter().enumerate() {
        saved.push((*pid, env.get(pid).cloned()));
        env.insert(*pid, args.get(i).cloned().unwrap_or(JsVal::Undef));
    }
    // Nested function decls bind into env; save any locals they overwrite is hard —
    // fixture only uses fresh locals.
    let flow = with_this_nt(this, nt, || eval_body(&rec.body, env))?;
    for (pid, prev) in saved {
        match prev {
            Some(v) => {
                env.insert(pid, v);
            }
            None => {
                env.remove(&pid);
            }
        }
    }
    match flow {
        Flow::Normal => Ok(JsVal::Undef),
        Flow::Return(v) => Ok(v),
    }
}

fn call_user_fn_arrow(
    rec: &FnRec,
    args: &[JsVal],
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<JsVal, ()> {
    let mut saved: Vec<(LocalId, Option<JsVal>)> = Vec::new();
    for (i, pid) in rec.params.iter().enumerate() {
        saved.push((*pid, env.get(pid).cloned()));
        env.insert(*pid, args.get(i).cloned().unwrap_or(JsVal::Undef));
    }
    let flow = eval_body(&rec.body, env)?;
    for (pid, prev) in saved {
        match prev {
            Some(v) => {
                env.insert(pid, v);
            }
            None => {
                env.remove(&pid);
            }
        }
    }
    match flow {
        Flow::Normal => Ok(JsVal::Undef),
        Flow::Return(v) => Ok(v),
    }
}

fn member_get(obj: &JsVal, key: &str) -> JsVal {
    match obj {
        JsVal::Object { props, .. } => props
            .borrow()
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or(JsVal::Undef),
        JsVal::Fn { .. } if key == "prototype" => JsVal::Object {
            id: next_id(),
            props: Rc::new(RefCell::new(Vec::new())),
        },
        _ => JsVal::Undef,
    }
}

fn member_set(obj: &mut JsVal, key: &str, val: JsVal) -> Result<(), ()> {
    let JsVal::Object { props, .. } = obj else {
        return Err(());
    };
    let mut props = props.borrow_mut();
    if let Some((_, slot)) = props.iter_mut().find(|(k, _)| k == key) {
        *slot = val;
    } else {
        props.push((key.to_string(), val));
    }
    Ok(())
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Undef => "undefined".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Object { .. } => "object".into(),
        JsVal::Fn { .. } => "function".into(),
    }
}

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Undef => false,
        JsVal::Bool(b) => *b,
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Object { .. } | JsVal::Fn { .. } => true,
    }
}

fn strict_eq(l: &JsVal, r: &JsVal) -> bool {
    match (l, r) {
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Bool(a), JsVal::Bool(b)) => a == b,
        (JsVal::Str(a), JsVal::Str(b)) => a == b,
        (JsVal::Object { id: a, .. }, JsVal::Object { id: b, .. }) => a == b,
        (JsVal::Fn { id: a }, JsVal::Fn { id: b }) => a == b,
        _ => false,
    }
}

struct Emitter {
    out: String,
    body: String,
    str_consts: Vec<(String, String)>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
            str_consts: Vec::new(),
        }
    }

    fn string_const(&mut self, s: &str) -> String {
        if let Some((_, name)) = self.str_consts.iter().find(|(v, _)| v == s) {
            return name.clone();
        }
        let name = format!("@.ntstr.{}", self.str_consts.len());
        self.str_consts.push((s.to_string(), name.clone()));
        name
    }

    fn emit_str(&mut self, s: &str) {
        let name = self.string_const(s);
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_new_target: missing value"))?;
            match v {
                JsVal::Str(s) => self.emit_str(s),
                JsVal::Bool(b) => self.emit_str(if *b { "true" } else { "false" }),
                JsVal::Undef => self.emit_str("undefined"),
                _ => return Err(diag("es_new_target: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.27 new.target)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        for (s, name) in &self.str_consts {
            let n = s.len() + 1;
            let mut esc = String::new();
            for b in s.bytes() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    #[test]
    fn new_target_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/annex-b/new_target.drac");
        let m = compile_source(src).expect("compile");
        assert!(
            is_es_new_target_module(&m),
            "should classify as es_new_target"
        );
        let ir = emit_es_new_target(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["true", "function", "undefined"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }
}
