//! N08.12.01–N08.12.08 + N08.16.44: native observations for generator function
//! declaration + expression + methods (object/class/static) + `yield` / `yield*` /
//! `return` + `.next()` / `.next(arg)` / `.return(arg)` / `.throw(arg)` →
//! `{value, done}` + `for-of` over generators (E13.01–E13.08), and async
//! generators (E18.43): `async function*` / `{ async *m() }` / class
//! `async *m()` / `static async *m()`, `.next()` thenables, `await
//! Promise.resolve`, `for await` over async gens inside `async function`.
//!
//! Compile-time evaluation of a small generator subset: generator decls and
//! `function*` / `async function*` expressions (incl. named + IIFE) with simple
//! ident params, object/class generator methods (`*m()` / `async *m()` /
//! `static *m()`), `this` prop reads in methods, `yield` of
//! number/string/binary/local/GenFn/`void 0` (bare yield), `let x = yield …`
//! resume binding, `yield*` of generators/arrays (incl. completion value),
//! `return` of same, iterator `.next()` / `.return()` / `.throw()`, try/catch/
//! finally in generator bodies, property reads `.value` / `.done`, top-level
//! `for-of` / `try` over generators, and async-gen thenables / await / for-await.
//! Emits Runtime prints of final top-level number/boolean/string/undefined locals.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BOOL, PRINT_F64, PRINT_STR};

pub(crate) fn is_es_generators_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_generators(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_generators module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    /// Generator function index into `gen_fns`.
    GenFn(usize),
    /// Live generator instance index into `gens`.
    GenInst(usize),
    /// Array iterable (for `yield* […]` / nested `for-of`).
    Array(Vec<JsVal>),
    /// Iterator result `{ value, done }`.
    Result { value: Box<JsVal>, done: bool },
    /// Plain object or class instance (own props + optional prototype methods).
    Object {
        props: HashMap<String, JsVal>,
        methods: HashMap<String, usize>,
    },
    /// Class constructor value (static methods + prototype methods + simple ctor).
    Class {
        methods: HashMap<String, usize>,
        statics: HashMap<String, usize>,
        ctor_params: Vec<LocalId>,
        /// `this.prop = param` assignments from the constructor body.
        ctor_assigns: Vec<(String, LocalId)>,
    },
    /// `async function` (non-generator) — body run to completion on call.
    AsyncFn {
        params: Vec<LocalId>,
        body: Vec<Stmt>,
    },
    /// Global `Promise` for `Promise.resolve(x)`.
    BuiltinPromise,
}

/// Loop control from break/continue inside for-of bodies; throw for `.throw` / try.
#[derive(Clone, Debug)]
enum Flow {
    Next,
    Break,
    Continue,
    Throw(JsVal),
}

/// Eval failure: unsupported shape, or a JS exception to propagate.
#[derive(Clone, Debug)]
enum Ev {
    U,
    Throw(JsVal),
}

#[derive(Clone, Debug)]
struct GenFnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
    /// Named function expression binding (local to body), if any.
    name: Option<LocalId>,
}

/// Active `yield*` delegate while suspended.
enum YieldStarState {
    /// Nested generator instance.
    Gen { idx: usize, bind: Option<LocalId> },
    /// Array iterator: next element index.
    Array {
        elems: Vec<JsVal>,
        next_i: usize,
        bind: Option<LocalId>,
    },
}

/// Where execution sits inside a `try` when suspended or unwinding.
#[derive(Clone, Debug)]
struct TryCtx {
    /// Index of the `Stmt::Try` in the generator body.
    try_pc: usize,
    /// 0 = try block, 1 = catch handler, 2 = finally.
    region: u8,
    /// PC within the active region body.
    pc: usize,
    handler_param: Option<LocalId>,
    has_handler: bool,
    has_finally: bool,
    /// Completion to apply after finally (`return` value from `.return`).
    pending_return: Option<JsVal>,
    /// Exception to rethrow after finally.
    pending_throw: Option<JsVal>,
}

/// Suspended generator: body + program counter + param env + done flag.
struct GenInst {
    /// Index into `gen_fns`.
    fn_id: usize,
    /// Next statement index to execute (or complete if suspended).
    pc: usize,
    /// True after the first `.next` has started execution.
    started: bool,
    /// True when paused on the statement at `pc` awaiting resume value.
    suspended: bool,
    done: bool,
    env: HashMap<LocalId, JsVal>,
    /// Method call receiver (`this`), when spawned via method call.
    this_val: Option<JsVal>,
    /// Active `yield*` when suspended mid-delegate.
    yield_star: Option<YieldStarState>,
    /// Nested try state when suspended inside try/catch/finally.
    try_ctx: Option<TryCtx>,
}

struct ModuleInfo {
    /// Top-level observation locals in declare order.
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_generator(&module.body) {
        return None;
    }
    // Top-level shape only; detailed acceptance is eval success (methods/class/for-of/try).
    if !module.body.iter().all(|s| {
        matches!(
            s,
            Stmt::Function { .. }
                | Stmt::Declare { .. }
                | Stmt::Expr { .. }
                | Stmt::ForOf { .. }
                | Stmt::Block { .. }
                | Stmt::If { .. }
                | Stmt::Try { .. }
        )
    }) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    let mut gen_fns: Vec<GenFnRec> = Vec::new();
    let mut fn_bind: HashMap<LocalId, usize> = HashMap::new();
    let mut gens: Vec<GenInst> = Vec::new();
    let mut user_locals = Vec::new();

    // Seed Promise builtin for `Promise.resolve` inside async gens.
    for loc in &module.locals {
        if loc.name == "Promise" {
            env.insert(loc.id, JsVal::BuiltinPromise);
        }
    }

    // Hoist generator / async generator function decls.
    for stmt in &module.body {
        if let Stmt::Function {
            local,
            params,
            body,
            is_generator: true,
            ..
        } = stmt
        {
            let param_ids = simple_param_locals(params)?;
            let idx = gen_fns.len();
            gen_fns.push(GenFnRec {
                params: param_ids,
                body: filter_gen_body(body),
                name: None,
            });
            fn_bind.insert(*local, idx);
            env.insert(*local, JsVal::GenFn(idx));
        }
    }

    match eval_body(
        &module.body,
        &mut env,
        &mut gen_fns,
        &mut fn_bind,
        &mut gens,
    ) {
        Ok(()) => {}
        Err(()) => return None,
    }

    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            if matches!(
                loc.ty,
                Type::Number | Type::Any | Type::Boolean | Type::String
            ) {
                match env.get(local) {
                    Some(JsVal::Num(_) | JsVal::Bool(_) | JsVal::Str(_) | JsVal::Undef) => {
                        user_locals.push(*local)
                    }
                    Some(
                        JsVal::GenFn(_)
                        | JsVal::GenInst(_)
                        | JsVal::Result { .. }
                        | JsVal::Array(_)
                        | JsVal::Object { .. }
                        | JsVal::Class { .. }
                        | JsVal::AsyncFn { .. }
                        | JsVal::BuiltinPromise,
                    ) => {}
                    None => return None,
                }
            }
        }
    }

    if user_locals.is_empty() {
        return None;
    }

    let mut values = HashMap::new();
    for id in &user_locals {
        let v = env.get(id)?.clone();
        match &v {
            JsVal::Num(_) | JsVal::Bool(_) | JsVal::Str(_) | JsVal::Undef => {
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

fn simple_param_locals(params: &[Param]) -> Option<Vec<LocalId>> {
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

fn module_has_generator(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::Function {
            is_generator: true, ..
        } => true,
        Stmt::Block { body } => module_has_generator(body),
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } => expr_has_generator(e),
        _ => false,
    })
}

fn expr_has_generator(expr: &Expr) -> bool {
    match expr {
        Expr::Function {
            is_generator: true, ..
        } => true,
        Expr::Call { callee, args, .. } => {
            expr_has_generator(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_generator(e),
                    _ => false,
                })
        }
        Expr::Member { object, property, .. } => {
            expr_has_generator(object) || expr_has_generator(property)
        }
        Expr::Unary { arg, .. } => expr_has_generator(arg),
        Expr::Binary { left, right, .. } => {
            expr_has_generator(left) || expr_has_generator(right)
        }
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) => expr_has_generator(e),
            _ => false,
        }),
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                expr_has_generator(value)
            }
            ObjectProp::Spread(e) => expr_has_generator(e),
        }),
        Expr::New { callee, args, .. } => {
            expr_has_generator(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_generator(e),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_has_generator(value),
        _ => false,
    }
}

fn filter_gen_body(body: &[Stmt]) -> Vec<Stmt> {
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

fn eval_body(
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<(), ()> {
    for stmt in body {
        match eval_stmt(stmt, env, gen_fns, fn_bind, gens)? {
            Flow::Next => {}
            Flow::Break | Flow::Continue | Flow::Throw(_) => return Err(()),
        }
    }
    Ok(())
}

fn eval_stmt(
    stmt: &Stmt,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<Flow, ()> {
    match stmt {
        Stmt::Function {
            local,
            params,
            body,
            is_async: true,
            is_generator: false,
            ..
        } => {
            let param_ids = simple_param_locals(params).ok_or(())?;
            env.insert(
                *local,
                JsVal::AsyncFn {
                    params: param_ids,
                    body: filter_gen_body(body),
                },
            );
            Ok(Flow::Next)
        }
        Stmt::Function { .. } => Ok(Flow::Next),
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => match eval_expr(e, env, gen_fns, fn_bind, gens) {
                    Ok(v) => v,
                    Err(Ev::Throw(exc)) => return Ok(Flow::Throw(exc)),
                    Err(Ev::U) => return Err(()),
                },
                None => JsVal::Undef,
            };
            if let JsVal::GenFn(idx) = &v {
                fn_bind.insert(*local, *idx);
            }
            env.insert(*local, v);
            Ok(Flow::Next)
        }
        Stmt::Expr { expr } => match eval_expr(expr, env, gen_fns, fn_bind, gens) {
            Ok(_) => Ok(Flow::Next),
            Err(Ev::Throw(exc)) => Ok(Flow::Throw(exc)),
            Err(Ev::U) => Err(()),
        },
        Stmt::Block { body } => eval_block(body, env, gen_fns, fn_bind, gens),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = match eval_expr(test, env, gen_fns, fn_bind, gens) {
                Ok(v) => v,
                Err(Ev::Throw(exc)) => return Ok(Flow::Throw(exc)),
                Err(Ev::U) => return Err(()),
            };
            if is_truthy(&t) {
                eval_stmt(consequent, env, gen_fns, fn_bind, gens)
            } else if let Some(alt) = alternate {
                eval_stmt(alt, env, gen_fns, fn_bind, gens)
            } else {
                Ok(Flow::Next)
            }
        }
        Stmt::Break { label: None } => Ok(Flow::Break),
        Stmt::Continue { label: None } => Ok(Flow::Continue),
        Stmt::ForOf {
            left,
            right,
            body,
            is_await: _,
        } => {
            eval_for_of(left, right, body, env, gen_fns, fn_bind, gens)?;
            Ok(Flow::Next)
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            let mut completion = match eval_block(block, env, gen_fns, fn_bind, gens)? {
                Flow::Throw(exc) => {
                    if let Some(handler) = handler {
                        if let Some(Pattern::Local(pid)) = handler_param {
                            env.insert(*pid, exc);
                        } else if handler_param.is_some() {
                            return Err(());
                        }
                        eval_block(handler, env, gen_fns, fn_bind, gens)?
                    } else {
                        Flow::Throw(exc)
                    }
                }
                other => other,
            };
            if let Some(fin) = finalizer {
                match eval_block(fin, env, gen_fns, fn_bind, gens)? {
                    Flow::Next => {}
                    abrupt => completion = abrupt,
                }
            }
            Ok(completion)
        }
        _ => Err(()),
    }
}

fn eval_block(
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, env, gen_fns, fn_bind, gens)? {
            Flow::Next => {}
            flow => return Ok(flow),
        }
    }
    Ok(Flow::Next)
}

fn is_truthy(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef => false,
        _ => true,
    }
}

fn bind_for_of_left(
    left: &Stmt,
    value: JsVal,
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<(), ()> {
    match left {
        Stmt::Declare { local, init: None, .. } => {
            env.insert(*local, value);
            Ok(())
        }
        Stmt::Expr {
            expr: Expr::Local { id, .. },
        } => {
            env.insert(*id, value);
            Ok(())
        }
        _ => Err(()),
    }
}

fn eval_for_of(
    left: &Stmt,
    right: &Expr,
    body: &Stmt,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<(), ()> {
    let iterable = match eval_expr(right, env, gen_fns, fn_bind, gens) {
        Ok(v) => v,
        Err(_) => return Err(()),
    };
    match iterable {
        JsVal::GenInst(idx) => loop {
            let r = match gen_next(
                &JsVal::GenInst(idx),
                JsVal::Undef,
                gen_fns,
                fn_bind,
                gens,
            ) {
                Ok(v) => v,
                Err(_) => return Err(()),
            };
            let JsVal::Result { value, done } = r else {
                return Err(());
            };
            if done {
                break;
            }
            bind_for_of_left(left, *value, env)?;
            match eval_stmt(body, env, gen_fns, fn_bind, gens)? {
                Flow::Next | Flow::Continue => {}
                Flow::Break => break,
                Flow::Throw(_) => return Err(()),
            }
        },
        JsVal::Array(elems) => {
            for el in elems {
                bind_for_of_left(left, el, env)?;
                match eval_stmt(body, env, gen_fns, fn_bind, gens)? {
                    Flow::Next | Flow::Continue => {}
                    Flow::Break => break,
                    Flow::Throw(_) => return Err(()),
                }
            }
        }
        _ => return Err(()),
    }
    Ok(())
}

fn register_gen_fn_expr(
    name: Option<LocalId>,
    params: &[Param],
    body: &[Stmt],
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
) -> Result<JsVal, Ev> {
    let param_ids = simple_param_locals(params).ok_or(Ev::U)?;
    let idx = gen_fns.len();
    gen_fns.push(GenFnRec {
        params: param_ids,
        body: filter_gen_body(body),
        name,
    });
    if let Some(n) = name {
        fn_bind.insert(n, idx);
    }
    Ok(JsVal::GenFn(idx))
}

fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| Ev::U)?;
            Ok(JsVal::Num(n))
        }
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::String { value, .. } => Ok(JsVal::Str(value.to_string_lossy())),
        Expr::Local { id, .. } => env.get(id).cloned().ok_or(Ev::U),
        Expr::This { .. } => Err(Ev::U),
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => Ok(JsVal::Undef),
        Expr::Array { elements, .. } => {
            let mut vals = Vec::new();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => {
                        vals.push(eval_expr(e, env, gen_fns, fn_bind, gens)?)
                    }
                    _ => return Err(Ev::U),
                }
            }
            Ok(JsVal::Array(vals))
        }
        Expr::Object { properties, .. } => eval_object_lit(properties, env, gen_fns, fn_bind, gens),
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            let l = eval_expr(left, env, gen_fns, fn_bind, gens)?;
            let r = eval_expr(right, env, gen_fns, fn_bind, gens)?;
            bin_val(op, &l, &r)
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = eval_expr(value, env, gen_fns, fn_bind, gens)?;
            env.insert(*id, v.clone());
            Ok(v)
        }
        Expr::Function {
            name,
            params,
            body,
            is_generator: true,
            is_arrow: false,
            ..
        } => register_gen_fn_expr(*name, params, body, gen_fns, fn_bind),
        Expr::Function {
            params,
            body,
            is_async: true,
            is_generator: false,
            is_arrow: false,
            ..
        } => {
            let param_ids = simple_param_locals(params).ok_or(Ev::U)?;
            Ok(JsVal::AsyncFn {
                params: param_ids,
                body: filter_gen_body(body),
            })
        }
        Expr::Unary {
            op: UnaryOp::Await,
            arg,
            ..
        } => eval_expr(arg, env, gen_fns, fn_bind, gens),
        // Class builder IIFE: `(function(){ … return ctor })()`
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } if args.is_empty() => {
            if let Expr::Function {
                params,
                body,
                is_async: false,
                is_generator: false,
                is_arrow: false,
                ..
            } = callee.as_ref()
            {
                if params.is_empty() {
                    if let Ok(cls) = try_eval_class_iife(body, gen_fns, fn_bind) {
                        return Ok(cls);
                    }
                }
            }
            eval_call(callee, args, env, gen_fns, fn_bind, gens)
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => eval_call(callee, args, env, gen_fns, fn_bind, gens),
        Expr::New {
            callee,
            args,
            ..
        } => {
            let c = eval_expr(callee, env, gen_fns, fn_bind, gens)?;
            eval_new(c, args, env, gen_fns, fn_bind, gens)
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = eval_expr(object, env, gen_fns, fn_bind, gens)?;
            let prop = prop_name(property)?;
            lookup_prop(&obj, &prop)
        }
        _ => Err(Ev::U),
    }
}

fn eval_call(
    callee: &Expr,
    args: &[Arg],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    // Method call: `it.next()` / `obj.m(args)` / `C.sgen(args)` / `.then` / `Promise.resolve`.
    if let Expr::Member {
        object,
        property,
        optional: false,
        ..
    } = callee
    {
        let obj = eval_expr(object, env, gen_fns, fn_bind, gens)?;
        let prop = prop_name(property)?;
        if prop == "next" {
            let resume = if args.is_empty() {
                JsVal::Undef
            } else if args.len() == 1 {
                match &args[0] {
                    Arg::Expr(e) => eval_expr(e, env, gen_fns, fn_bind, gens)?,
                    _ => return Err(Ev::U),
                }
            } else {
                return Err(Ev::U);
            };
            return gen_next(&obj, resume, gen_fns, fn_bind, gens);
        }
        if prop == "return" {
            let val = if args.is_empty() {
                JsVal::Undef
            } else if args.len() == 1 {
                match &args[0] {
                    Arg::Expr(e) => eval_expr(e, env, gen_fns, fn_bind, gens)?,
                    _ => return Err(Ev::U),
                }
            } else {
                return Err(Ev::U);
            };
            return gen_return(&obj, val, gen_fns, fn_bind, gens);
        }
        if prop == "throw" {
            let val = if args.is_empty() {
                JsVal::Undef
            } else if args.len() == 1 {
                match &args[0] {
                    Arg::Expr(e) => eval_expr(e, env, gen_fns, fn_bind, gens)?,
                    _ => return Err(Ev::U),
                }
            } else {
                return Err(Ev::U);
            };
            return gen_throw(&obj, val, gen_fns, fn_bind, gens);
        }
        if prop == "then" {
            // Immediately-settled thenable: invoke onfulfill with `obj` as value.
            if args.is_empty() || args.len() > 2 {
                return Err(Ev::U);
            }
            let Arg::Expr(cb) = &args[0] else {
                return Err(Ev::U);
            };
            return call_thenable_cb(cb, obj, env, gen_fns, fn_bind, gens);
        }
        if prop == "resolve" {
            if !matches!(obj, JsVal::BuiltinPromise) || args.len() != 1 {
                return Err(Ev::U);
            }
            let Arg::Expr(e) = &args[0] else {
                return Err(Ev::U);
            };
            return eval_expr(e, env, gen_fns, fn_bind, gens);
        }
        let method = lookup_prop(&obj, &prop)?;
        let this_val = match &obj {
            JsVal::Object { .. } | JsVal::Class { .. } => Some(obj),
            _ => None,
        };
        return spawn_gen_call(method, args, this_val, env, gen_fns, fn_bind, gens);
    }

    // Call generator / async function: `g(args)` / IIFE → iterator or promise value.
    let c = eval_expr(callee, env, gen_fns, fn_bind, gens)?;
    match c {
        JsVal::AsyncFn { params, body } => {
            call_async_fn(&params, &body, args, env, gen_fns, fn_bind, gens)
        }
        other => spawn_gen_call(other, args, None, env, gen_fns, fn_bind, gens),
    }
}

fn call_thenable_cb(
    cb: &Expr,
    value: JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let Expr::Function {
        params,
        body,
        is_async: false,
        is_generator: false,
        ..
    } = cb
    else {
        return Err(Ev::U);
    };
    let param_ids = simple_param_locals(params).ok_or(Ev::U)?;
    let mut saved: HashMap<LocalId, Option<JsVal>> = HashMap::new();
    for (i, pid) in param_ids.iter().enumerate() {
        saved.insert(*pid, env.get(pid).cloned());
        let v = if i == 0 {
            value.clone()
        } else {
            JsVal::Undef
        };
        env.insert(*pid, v);
    }
    let mut ret = JsVal::Undef;
    for stmt in body {
        match stmt {
            Stmt::Return { value: Some(e) } => {
                ret = eval_expr(e, env, gen_fns, fn_bind, gens)?;
                break;
            }
            Stmt::Return { value: None } => {
                ret = JsVal::Undef;
                break;
            }
            other => match eval_stmt(other, env, gen_fns, fn_bind, gens) {
                Ok(Flow::Next) => {}
                Ok(Flow::Throw(exc)) => {
                    restore_params(env, &saved);
                    return Err(Ev::Throw(exc));
                }
                Ok(_) | Err(()) => {
                    restore_params(env, &saved);
                    return Err(Ev::U);
                }
            },
        }
    }
    restore_params(env, &saved);
    Ok(ret)
}

fn restore_params(env: &mut HashMap<LocalId, JsVal>, saved: &HashMap<LocalId, Option<JsVal>>) {
    for (pid, prev) in saved {
        match prev {
            Some(v) => {
                env.insert(*pid, v.clone());
            }
            None => {
                env.remove(pid);
            }
        }
    }
}

fn call_async_fn(
    params: &[LocalId],
    body: &[Stmt],
    args: &[Arg],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    if args.len() > params.len() {
        return Err(Ev::U);
    }
    let mut arg_vals = Vec::new();
    for a in args {
        match a {
            Arg::Expr(e) => arg_vals.push(eval_expr(e, env, gen_fns, fn_bind, gens)?),
            _ => return Err(Ev::U),
        }
    }
    // Nested bindings shadow outer; restore after.
    let mut saved: HashMap<LocalId, Option<JsVal>> = HashMap::new();
    for (i, pid) in params.iter().enumerate() {
        saved.entry(*pid).or_insert_with(|| env.get(pid).cloned());
        let v = if i < arg_vals.len() {
            arg_vals[i].clone()
        } else {
            JsVal::Undef
        };
        env.insert(*pid, v);
    }
    // Hoist nested generator / async-gen decls inside async function body.
    for stmt in body {
        if let Stmt::Function {
            local,
            params: gp,
            body: gbody,
            is_generator: true,
            ..
        } = stmt
        {
            let param_ids = simple_param_locals(gp).ok_or(Ev::U)?;
            let idx = gen_fns.len();
            gen_fns.push(GenFnRec {
                params: param_ids,
                body: filter_gen_body(gbody),
                name: None,
            });
            fn_bind.insert(*local, idx);
            saved.entry(*local).or_insert_with(|| env.get(local).cloned());
            env.insert(*local, JsVal::GenFn(idx));
        }
    }
    let mut ret = JsVal::Undef;
    for stmt in body {
        match stmt {
            Stmt::Function { .. } => {}
            Stmt::Return { value: Some(e) } => {
                ret = eval_expr(e, env, gen_fns, fn_bind, gens)?;
                break;
            }
            Stmt::Return { value: None } => {
                ret = JsVal::Undef;
                break;
            }
            other => match eval_stmt(other, env, gen_fns, fn_bind, gens) {
                Ok(Flow::Next) => {}
                Ok(Flow::Throw(exc)) => {
                    restore_params(env, &saved);
                    return Err(Ev::Throw(exc));
                }
                Ok(_) | Err(()) => {
                    restore_params(env, &saved);
                    return Err(Ev::U);
                }
            },
        }
    }
    restore_params(env, &saved);
    Ok(ret)
}

fn eval_object_lit(
    properties: &[ObjectProp],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let mut props = HashMap::new();
    let methods = HashMap::new();
    for p in properties {
        let ObjectProp::Property { key, value } = p else {
            return Err(Ev::U);
        };
        let ObjectPropKey::Static(k) = key else {
            return Err(Ev::U);
        };
        let name = k.to_string_lossy();
        let v = eval_expr(value, env, gen_fns, fn_bind, gens)?;
        props.insert(name, v);
    }
    Ok(JsVal::Object { props, methods })
}

fn lookup_prop(obj: &JsVal, prop: &str) -> Result<JsVal, Ev> {
    match obj {
        JsVal::Result { value, done } => match prop {
            "value" => Ok(*value.clone()),
            "done" => Ok(JsVal::Bool(*done)),
            _ => Err(Ev::U),
        },
        JsVal::Object { props, methods } => {
            if let Some(v) = props.get(prop) {
                return Ok(v.clone());
            }
            if let Some(&fid) = methods.get(prop) {
                return Ok(JsVal::GenFn(fid));
            }
            Err(Ev::U)
        }
        JsVal::Class { statics, .. } => {
            if let Some(&fid) = statics.get(prop) {
                return Ok(JsVal::GenFn(fid));
            }
            Err(Ev::U)
        }
        _ => Err(Ev::U),
    }
}

fn eval_new(
    callee: JsVal,
    args: &[Arg],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::Class {
        methods,
        ctor_params,
        ctor_assigns,
        ..
    } = callee
    else {
        return Err(Ev::U);
    };
    if args.len() > ctor_params.len() {
        return Err(Ev::U);
    }
    let mut arg_vals = Vec::new();
    for a in args {
        match a {
            Arg::Expr(e) => arg_vals.push(eval_expr(e, env, gen_fns, fn_bind, gens)?),
            _ => return Err(Ev::U),
        }
    }
    let mut param_env = HashMap::new();
    for (i, pid) in ctor_params.iter().enumerate() {
        let v = if i < arg_vals.len() {
            arg_vals[i].clone()
        } else {
            JsVal::Undef
        };
        param_env.insert(*pid, v);
    }
    let mut props = HashMap::new();
    for (prop, pid) in &ctor_assigns {
        let v = param_env.get(pid).cloned().unwrap_or(JsVal::Undef);
        props.insert(prop.clone(), v);
    }
    Ok(JsVal::Object { props, methods })
}

/// Extract class from builder IIFE body (ctor + proto/static generator methods).
fn try_eval_class_iife(
    body: &[Stmt],
    gen_fns: &mut Vec<GenFnRec>,
    _fn_bind: &mut HashMap<LocalId, usize>,
) -> Result<JsVal, ()> {
    let mut ctor_local: Option<LocalId> = None;
    let mut ctor_params: Vec<LocalId> = Vec::new();
    let mut ctor_assigns: Vec<(String, LocalId)> = Vec::new();
    let mut pending_methods: Vec<(bool, String, Vec<LocalId>, Vec<Stmt>)> = Vec::new();
    let mut pending_key: Option<String> = None;
    let mut saw_return_ctor = false;
    let mut saw_any_gen_method = false;

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
            } if ctor_local.is_none() => {
                ctor_params = simple_param_locals(params).ok_or(())?;
                ctor_assigns = extract_ctor_assigns(cbody)?;
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
                let ctor = ctor_local.ok_or(())?;
                let key = pending_key
                    .take()
                    .or_else(|| string_arg_key(&args[1]))
                    .ok_or(())?;
                // Skip non-method defines (name, prototype descriptor without method).
                let Some(method_fn) = find_method_function(&args[2]) else {
                    continue;
                };
                let Expr::Function {
                    params,
                    body: mbody,
                    is_generator: true,
                    ..
                } = method_fn
                else {
                    // Non-generator method — not in this adapter's scope.
                    return Err(());
                };
                let param_ids = simple_param_locals(params).ok_or(())?;
                let is_static = if is_define_on_ctor(args, ctor) {
                    true
                } else if is_define_on_proto(args, ctor) {
                    false
                } else {
                    return Err(());
                };
                saw_any_gen_method = true;
                pending_methods.push((is_static, key, param_ids, filter_gen_body(mbody)));
            }
            Stmt::Return {
                value: Some(Expr::Local { id, .. }),
            } if Some(*id) == ctor_local => {
                saw_return_ctor = true;
            }
            // Ignore other class-builder scaffolding (defineProperty name/proto, etc.).
            Stmt::Declare { .. } | Stmt::Expr { .. } | Stmt::If { .. } => {}
            _ => return Err(()),
        }
    }

    if !saw_return_ctor || ctor_local.is_none() || !saw_any_gen_method {
        return Err(());
    }

    let mut methods: HashMap<String, usize> = HashMap::new();
    let mut statics: HashMap<String, usize> = HashMap::new();
    for (is_static, key, params, mbody) in pending_methods {
        let idx = gen_fns.len();
        gen_fns.push(GenFnRec {
            params,
            body: mbody,
            name: None,
        });
        if is_static {
            statics.insert(key, idx);
        } else {
            methods.insert(key, idx);
        }
    }

    Ok(JsVal::Class {
        methods,
        statics,
        ctor_params,
        ctor_assigns,
    })
}

fn extract_ctor_assigns(body: &[Stmt]) -> Result<Vec<(String, LocalId)>, ()> {
    let mut out = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            Stmt::If { .. } => {} // new.target check
            Stmt::Expr {
                expr:
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
                    },
            } => {
                if !matches!(object.as_ref(), Expr::This { .. }) {
                    return Err(());
                }
                let prop = prop_name(property).map_err(|_| ())?;
                let Expr::Local { id, .. } = value.as_ref() else {
                    return Err(());
                };
                out.push((prop, *id));
            }
            _ => {}
        }
    }
    Ok(out)
}

fn is_object_define_property(callee: &Expr) -> bool {
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
        ) if name == "Object" && value.to_string_lossy() == "defineProperty"
    )
}

fn is_define_on_ctor(args: &[Arg], ctor: LocalId) -> bool {
    matches!(
        &args[0],
        Arg::Expr(Expr::Local { id, .. }) if *id == ctor
    )
}

fn is_define_on_proto(args: &[Arg], ctor: LocalId) -> bool {
    let Arg::Expr(Expr::Member {
        object, property, ..
    }) = &args[0]
    else {
        return false;
    };
    matches!(
        (object.as_ref(), property.as_ref()),
        (
            Expr::Local { id, .. },
            Expr::String { value, .. }
        ) if *id == ctor && value.to_string_lossy() == "prototype"
    )
}

fn string_arg_key(arg: &Arg) -> Option<String> {
    match arg {
        Arg::Expr(Expr::String { value, .. }) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn find_method_function<'a>(arg: &'a Arg) -> Option<&'a Expr> {
    let Arg::Expr(expr) = arg else {
        return None;
    };
    find_method_function_expr(expr)
}

fn find_method_function_expr(expr: &Expr) -> Option<&Expr> {
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
            if let Some(f) = find_method_function_expr(callee) {
                return Some(f);
            }
            for a in args {
                if let Some(f) = find_method_function(a) {
                    return Some(f);
                }
            }
            None
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                if let ObjectProp::Property { value, .. } = p {
                    if let Some(f) = find_method_function_expr(value) {
                        return Some(f);
                    }
                }
            }
            None
        }
        Expr::Member {
            object, property, ..
        } => find_method_function_expr(object).or_else(|| find_method_function_expr(property)),
        Expr::Binary { left, right, .. } => {
            find_method_function_expr(left).or_else(|| find_method_function_expr(right))
        }
        Expr::Assign { value, .. } => find_method_function_expr(value),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => find_method_function_expr(test)
            .or_else(|| find_method_function_expr(consequent))
            .or_else(|| find_method_function_expr(alternate)),
        Expr::Unary { arg, .. } => find_method_function_expr(arg),
        _ => None,
    }
}

fn find_method_function_in_stmt(stmt: &Stmt) -> Option<&Expr> {
    match stmt {
        Stmt::Expr { expr } | Stmt::Return { value: Some(expr) } => {
            find_method_function_expr(expr)
        }
        Stmt::Declare {
            init: Some(expr), ..
        } => find_method_function_expr(expr),
        Stmt::Block { body } => {
            for s in body {
                if let Some(f) = find_method_function_in_stmt(s) {
                    return Some(f);
                }
            }
            None
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => find_method_function_expr(test)
            .or_else(|| find_method_function_in_stmt(consequent))
            .or_else(|| alternate.as_ref().and_then(|a| find_method_function_in_stmt(a))),
        _ => None,
    }
}

fn spawn_gen_call(
    callee: JsVal,
    args: &[Arg],
    this_val: Option<JsVal>,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::GenFn(fid) = callee else {
        return Err(Ev::U);
    };
    if fid >= gen_fns.len() {
        return Err(Ev::U);
    }
    let n_params = gen_fns[fid].params.len();
    if args.len() > n_params {
        return Err(Ev::U);
    }
    let mut arg_vals = Vec::new();
    for a in args {
        match a {
            Arg::Expr(e) => arg_vals.push(eval_expr(e, env, gen_fns, fn_bind, gens)?),
            _ => return Err(Ev::U),
        }
    }
    spawn_gen_vals(
        JsVal::GenFn(fid),
        &arg_vals,
        this_val,
        gen_fns,
        fn_bind,
        gens,
    )
}

fn bin_val(op: &BinaryOp, left: &JsVal, right: &JsVal) -> Result<JsVal, Ev> {
    match op {
        BinaryOp::Add => match (left, right) {
            (JsVal::Num(a), JsVal::Num(b)) => Ok(JsVal::Num(a + b)),
            (JsVal::Str(a), JsVal::Str(b)) => Ok(JsVal::Str(format!("{a}{b}"))),
            (JsVal::Str(a), JsVal::Num(b)) => Ok(JsVal::Str(format!("{a}{b}"))),
            (JsVal::Num(a), JsVal::Str(b)) => Ok(JsVal::Str(format!("{a}{b}"))),
            _ => Err(Ev::U),
        },
        BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
            let JsVal::Num(a) = left else {
                return Err(Ev::U);
            };
            let JsVal::Num(b) = right else {
                return Err(Ev::U);
            };
            let n = match op {
                BinaryOp::Sub => a - b,
                BinaryOp::Mul => a * b,
                BinaryOp::Div => a / b,
                BinaryOp::Rem => a % b,
                _ => unreachable!(),
            };
            Ok(JsVal::Num(n))
        }
        BinaryOp::EqEqEq => Ok(JsVal::Bool(strict_eq(left, right))),
        BinaryOp::NotEqEq => Ok(JsVal::Bool(!strict_eq(left, right))),
        _ => Err(Ev::U),
    }
}

fn strict_eq(left: &JsVal, right: &JsVal) -> bool {
    match (left, right) {
        (JsVal::Num(a), JsVal::Num(b)) => a == b,
        (JsVal::Bool(a), JsVal::Bool(b)) => a == b,
        (JsVal::Str(a), JsVal::Str(b)) => a == b,
        (JsVal::Undef, JsVal::Undef) => true,
        _ => false,
    }
}

fn prop_name(expr: &Expr) -> Result<String, Ev> {
    match expr {
        Expr::String { value, .. } => Ok(value.to_string_lossy()),
        _ => Err(Ev::U),
    }
}

fn map_in_gen<T>(r: Result<T, ()>) -> Result<T, Ev> {
    r.map_err(|_| Ev::U)
}

fn iter_result(value: JsVal, done: bool) -> JsVal {
    JsVal::Result {
        value: Box::new(value),
        done,
    }
}

/// Resume generator until next `yield` / `yield*` step / `return` / end.
/// First `.next` ignores `resume`; later calls inject it as the yield value.
fn gen_next(
    obj: &JsVal,
    resume: JsVal,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::GenInst(idx) = obj else {
        return Err(Ev::U);
    };
    if *idx >= gens.len() {
        return Err(Ev::U);
    }
    if gens[*idx].done {
        return Ok(iter_result(JsVal::Undef, true));
    }

    let mut inject: Option<JsVal> = if !gens[*idx].started {
        gens[*idx].started = true;
        None
    } else if gens[*idx].suspended || gens[*idx].yield_star.is_some() {
        gens[*idx].suspended = false;
        Some(resume)
    } else {
        Some(JsVal::Undef)
    };

    gen_continue(*idx, &mut inject, gen_fns, fn_bind, gens)
}

/// Generator.prototype.return(value)
fn gen_return(
    obj: &JsVal,
    value: JsVal,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::GenInst(idx) = obj else {
        return Err(Ev::U);
    };
    if *idx >= gens.len() {
        return Err(Ev::U);
    }
    if gens[*idx].done {
        return Ok(iter_result(value, true));
    }
    if !gens[*idx].started {
        gens[*idx].started = true;
        gens[*idx].done = true;
        return Ok(iter_result(value, true));
    }
    gens[*idx].suspended = false;
    gens[*idx].yield_star = None;

    // Unwind through try: run finally if present, else close with value.
    if let Some(ctx) = gens[*idx].try_ctx.as_mut() {
        if ctx.has_finally && ctx.region != 2 {
            ctx.pending_return = Some(value);
            ctx.region = 2;
            ctx.pc = 0;
            let mut inject = None;
            return gen_continue(*idx, &mut inject, gen_fns, fn_bind, gens);
        }
    }
    gens[*idx].try_ctx = None;
    gens[*idx].done = true;
    Ok(iter_result(value, true))
}

/// Generator.prototype.throw(exception)
fn gen_throw(
    obj: &JsVal,
    value: JsVal,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::GenInst(idx) = obj else {
        return Err(Ev::U);
    };
    if *idx >= gens.len() {
        return Err(Ev::U);
    }
    if gens[*idx].done {
        return Err(Ev::Throw(value));
    }
    if !gens[*idx].started {
        gens[*idx].started = true;
        gens[*idx].done = true;
        return Err(Ev::Throw(value));
    }
    gens[*idx].suspended = false;
    gens[*idx].yield_star = None;

    if let Some(ctx) = gens[*idx].try_ctx.clone() {
        if ctx.region == 0 && ctx.has_handler {
            if let Some(pid) = ctx.handler_param {
                gens[*idx].env.insert(pid, value);
            }
            if let Some(c) = gens[*idx].try_ctx.as_mut() {
                c.region = 1;
                c.pc = 0;
            }
            let mut inject = None;
            return gen_continue(*idx, &mut inject, gen_fns, fn_bind, gens);
        }
        if ctx.has_finally && ctx.region != 2 {
            if let Some(c) = gens[*idx].try_ctx.as_mut() {
                c.pending_throw = Some(value);
                c.region = 2;
                c.pc = 0;
            }
            let mut inject = None;
            return gen_continue(*idx, &mut inject, gen_fns, fn_bind, gens);
        }
    }
    gens[*idx].try_ctx = None;
    gens[*idx].done = true;
    Err(Ev::Throw(value))
}

fn gen_continue(
    idx: usize,
    inject: &mut Option<JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    // Active yield* first.
    if gens[idx].yield_star.is_some() {
        match map_in_gen(step_yield_star(
            idx,
            inject.take().unwrap_or(JsVal::Undef),
            gen_fns,
            fn_bind,
            gens,
        ))? {
            Step::Yield(v) => return Ok(v),
            Step::Continue => {}
        }
    }

    let fn_id = gens[idx].fn_id;
    if fn_id >= gen_fns.len() {
        return Err(Ev::U);
    }
    let body = gen_fns[fn_id].body.clone();

    loop {
        // Nested try/catch/finally execution.
        if gens[idx].try_ctx.is_some() {
            match step_try_region(idx, inject, &body, gen_fns, fn_bind, gens)? {
                Some(v) => return Ok(v),
                None => continue,
            }
        }

        let pc = gens[idx].pc;
        if pc >= body.len() {
            gens[idx].done = true;
            return Ok(iter_result(JsVal::Undef, true));
        }

        match &body[pc] {
            Stmt::Try {
                block: _,
                handler_param,
                handler,
                finalizer,
            } => {
                let hp = match handler_param {
                    None => None,
                    Some(Pattern::Local(id)) => Some(*id),
                    Some(_) => return Err(Ev::U),
                };
                gens[idx].try_ctx = Some(TryCtx {
                    try_pc: pc,
                    region: 0,
                    pc: 0,
                    handler_param: hp,
                    has_handler: handler.is_some(),
                    has_finally: finalizer.is_some(),
                    pending_return: None,
                    pending_throw: None,
                });
                // do not advance outer pc yet
            }
            other => match exec_gen_stmt(idx, other, inject, true, gen_fns, fn_bind, gens)? {
                ExecOut::Yield(v) => return Ok(v),
                ExecOut::Done(v) => return Ok(v),
                ExecOut::Advanced => {
                    gens[idx].pc = pc + 1;
                }
                ExecOut::Stay => {}
            },
        }
    }
}

enum ExecOut {
    Yield(JsVal),
    Done(JsVal),
    Advanced,
    Stay,
}

/// Execute one generator statement. `advance_outer` unused for nest (caller advances nest pc).
fn exec_gen_stmt(
    idx: usize,
    stmt: &Stmt,
    inject: &mut Option<JsVal>,
    _advance_outer: bool,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<ExecOut, Ev> {
    match stmt {
        Stmt::Expr {
            expr:
                Expr::Unary {
                    op: UnaryOp::Yield,
                    arg,
                    ..
                },
        } => {
            if let Some(_v) = inject.take() {
                return Ok(ExecOut::Advanced);
            }
            let yv = map_in_gen(eval_in_gen(arg, idx, gen_fns, fn_bind, gens))?;
            gens[idx].suspended = true;
            Ok(ExecOut::Yield(iter_result(yv, false)))
        }
        Stmt::Expr {
            expr:
                Expr::Unary {
                    op: UnaryOp::YieldStar,
                    arg,
                    ..
                },
        } => {
            if inject.is_some() {
                inject.take();
            }
            map_in_gen(start_yield_star(idx, arg, None, gen_fns, fn_bind, gens))?;
            match map_in_gen(step_yield_star(idx, JsVal::Undef, gen_fns, fn_bind, gens))? {
                Step::Yield(v) => Ok(ExecOut::Yield(v)),
                Step::Continue => Ok(ExecOut::Stay),
            }
        }
        Stmt::Declare {
            local,
            init:
                Some(Expr::Unary {
                    op: UnaryOp::Yield,
                    arg,
                    ..
                }),
            ..
        } => {
            if let Some(v) = inject.take() {
                gens[idx].env.insert(*local, v);
                return Ok(ExecOut::Advanced);
            }
            let yv = map_in_gen(eval_in_gen(arg, idx, gen_fns, fn_bind, gens))?;
            gens[idx].suspended = true;
            Ok(ExecOut::Yield(iter_result(yv, false)))
        }
        Stmt::Declare {
            local,
            init:
                Some(Expr::Unary {
                    op: UnaryOp::YieldStar,
                    arg,
                    ..
                }),
            ..
        } => {
            if inject.is_some() {
                inject.take();
            }
            let bind = Some(*local);
            map_in_gen(start_yield_star(idx, arg, bind, gen_fns, fn_bind, gens))?;
            match map_in_gen(step_yield_star(idx, JsVal::Undef, gen_fns, fn_bind, gens))? {
                Step::Yield(v) => Ok(ExecOut::Yield(v)),
                Step::Continue => Ok(ExecOut::Stay),
            }
        }
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                None => JsVal::Undef,
                Some(e) => map_in_gen(eval_in_gen(e, idx, gen_fns, fn_bind, gens))?,
            };
            gens[idx].env.insert(*local, v);
            Ok(ExecOut::Advanced)
        }
        Stmt::Return { value } => {
            let v = match value {
                None => JsVal::Undef,
                Some(e) => map_in_gen(eval_in_gen(e, idx, gen_fns, fn_bind, gens))?,
            };
            // If inside try with finally, run finally before completing.
            if let Some(ctx) = gens[idx].try_ctx.as_mut() {
                if ctx.has_finally && ctx.region != 2 {
                    ctx.pending_return = Some(v);
                    ctx.region = 2;
                    ctx.pc = 0;
                    return Ok(ExecOut::Stay);
                }
            }
            gens[idx].try_ctx = None;
            gens[idx].done = true;
            Ok(ExecOut::Done(iter_result(v, true)))
        }
        Stmt::Expr { expr } => {
            map_in_gen(eval_in_gen(expr, idx, gen_fns, fn_bind, gens))?;
            Ok(ExecOut::Advanced)
        }
        Stmt::Block { body } => {
            // Flatten one level: not expected as nested block PC; unsupported multi-stmt block
            // without its own PC — only empty or single-pass via sequential not supported.
            let _ = body;
            Err(Ev::U)
        }
        _ => Err(Ev::U),
    }
}

/// Step inside active try_ctx. `Some(v)` = yield/done result; `None` = keep looping.
fn step_try_region(
    idx: usize,
    inject: &mut Option<JsVal>,
    body: &[Stmt],
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<Option<JsVal>, Ev> {
    let ctx = gens[idx].try_ctx.clone().ok_or(Ev::U)?;
    let try_pc = ctx.try_pc;
    if try_pc >= body.len() {
        return Err(Ev::U);
    }
    let Stmt::Try {
        block,
        handler,
        finalizer,
        ..
    } = &body[try_pc]
    else {
        return Err(Ev::U);
    };

    let region_body: &[Stmt] = match ctx.region {
        0 => block.as_slice(),
        1 => handler.as_ref().map(|h| h.as_slice()).unwrap_or(&[]),
        2 => finalizer.as_ref().map(|f| f.as_slice()).unwrap_or(&[]),
        _ => return Err(Ev::U),
    };

    if ctx.pc >= region_body.len() {
        // Region finished.
        match ctx.region {
            0 => {
                if ctx.has_finally {
                    if let Some(c) = gens[idx].try_ctx.as_mut() {
                        c.region = 2;
                        c.pc = 0;
                    }
                } else {
                    gens[idx].try_ctx = None;
                    gens[idx].pc = try_pc + 1;
                }
                return Ok(None);
            }
            1 => {
                if ctx.has_finally {
                    if let Some(c) = gens[idx].try_ctx.as_mut() {
                        c.region = 2;
                        c.pc = 0;
                    }
                } else {
                    gens[idx].try_ctx = None;
                    gens[idx].pc = try_pc + 1;
                }
                return Ok(None);
            }
            2 => {
                let pending_return = ctx.pending_return.clone();
                let pending_throw = ctx.pending_throw.clone();
                gens[idx].try_ctx = None;
                if let Some(v) = pending_return {
                    gens[idx].done = true;
                    return Ok(Some(iter_result(v, true)));
                }
                if let Some(t) = pending_throw {
                    gens[idx].done = true;
                    return Err(Ev::Throw(t));
                }
                gens[idx].pc = try_pc + 1;
                return Ok(None);
            }
            _ => return Err(Ev::U),
        }
    }

    let stmt = &region_body[ctx.pc];
    match exec_gen_stmt(idx, stmt, inject, false, gen_fns, fn_bind, gens)? {
        ExecOut::Yield(v) => Ok(Some(v)),
        ExecOut::Done(v) => Ok(Some(v)),
        ExecOut::Advanced => {
            if let Some(c) = gens[idx].try_ctx.as_mut() {
                c.pc += 1;
            }
            Ok(None)
        }
        ExecOut::Stay => Ok(None),
    }
}

enum Step {
    Yield(JsVal),
    Continue,
}

fn start_yield_star(
    outer_idx: usize,
    arg: &Expr,
    bind: Option<LocalId>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<(), ()> {
    let iterable = eval_in_gen(arg, outer_idx, gen_fns, fn_bind, gens)?;
    let state = match iterable {
        JsVal::GenInst(i) => YieldStarState::Gen { idx: i, bind },
        JsVal::Array(elems) => YieldStarState::Array {
            elems,
            next_i: 0,
            bind,
        },
        JsVal::GenFn(fid) => {
            let inst = match spawn_gen_vals(JsVal::GenFn(fid), &[], None, gen_fns, fn_bind, gens)
            {
                Ok(v) => v,
                Err(_) => return Err(()),
            };
            let JsVal::GenInst(i) = inst else {
                return Err(());
            };
            YieldStarState::Gen { idx: i, bind }
        }
        _ => return Err(()),
    };
    gens[outer_idx].yield_star = Some(state);
    Ok(())
}

fn step_yield_star(
    outer_idx: usize,
    resume: JsVal,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<Step, ()> {
    let state = gens[outer_idx].yield_star.take().ok_or(())?;
    match state {
        YieldStarState::Gen { idx: inner, bind } => {
            let r = match gen_next(&JsVal::GenInst(inner), resume, gen_fns, fn_bind, gens) {
                Ok(v) => v,
                Err(_) => return Err(()),
            };
            match r {
                JsVal::Result { value, done: false } => {
                    gens[outer_idx].yield_star = Some(YieldStarState::Gen { idx: inner, bind });
                    gens[outer_idx].suspended = true;
                    Ok(Step::Yield(JsVal::Result {
                        value,
                        done: false,
                    }))
                }
                JsVal::Result { value, done: true } => {
                    if let Some(local) = bind {
                        gens[outer_idx].env.insert(local, *value);
                    }
                    gens[outer_idx].pc += 1;
                    gens[outer_idx].suspended = false;
                    Ok(Step::Continue)
                }
                _ => Err(()),
            }
        }
        YieldStarState::Array {
            elems,
            next_i,
            bind,
        } => {
            let _ = resume; // array iterators ignore resume for this subset
            if next_i < elems.len() {
                let v = elems[next_i].clone();
                gens[outer_idx].yield_star = Some(YieldStarState::Array {
                    elems,
                    next_i: next_i + 1,
                    bind,
                });
                gens[outer_idx].suspended = true;
                Ok(Step::Yield(JsVal::Result {
                    value: Box::new(v),
                    done: false,
                }))
            } else {
                // Array iterator completion value is undefined.
                if let Some(local) = bind {
                    gens[outer_idx].env.insert(local, JsVal::Undef);
                }
                gens[outer_idx].pc += 1;
                gens[outer_idx].suspended = false;
                Ok(Step::Continue)
            }
        }
    }
}

/// Evaluate expression in generator body (params, arithmetic, arrays, gen calls, this).
fn eval_in_gen(
    expr: &Expr,
    gen_idx: usize,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| ())?;
            Ok(JsVal::Num(n))
        }
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::String { value, .. } => Ok(JsVal::Str(value.to_string_lossy())),
        Expr::Local { id, .. } => gens[gen_idx].env.get(id).cloned().ok_or(()),
        Expr::This { .. } => gens[gen_idx].this_val.clone().ok_or(()),
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => Ok(JsVal::Undef),
        Expr::Unary {
            op: UnaryOp::Await,
            arg,
            ..
        } => eval_in_gen(arg, gen_idx, gen_fns, fn_bind, gens),
        Expr::Array { elements, .. } => {
            let mut vals = Vec::new();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => {
                        vals.push(eval_in_gen(e, gen_idx, gen_fns, fn_bind, gens)?)
                    }
                    _ => return Err(()),
                }
            }
            Ok(JsVal::Array(vals))
        }
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            let l = eval_in_gen(left, gen_idx, gen_fns, fn_bind, gens)?;
            let r = eval_in_gen(right, gen_idx, gen_fns, fn_bind, gens)?;
            bin_val(op, &l, &r).map_err(|_| ())
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = eval_in_gen(object, gen_idx, gen_fns, fn_bind, gens)?;
            let prop = prop_name(property).map_err(|_| ())?;
            lookup_prop(&obj, &prop).map_err(|_| ())
        }
        Expr::Function {
            name,
            params,
            body,
            is_generator: true,
            is_arrow: false,
            ..
        } => register_gen_fn_expr(*name, params, body, gen_fns, fn_bind).map_err(|_| ()),
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            // `Promise.resolve(x)` inside async gen (Promise may be missing from gen env).
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = callee.as_ref()
            {
                let prop = prop_name(property).map_err(|_| ())?;
                if prop == "resolve" {
                    let obj = match object.as_ref() {
                        Expr::Local { id, .. } => gens[gen_idx]
                            .env
                            .get(id)
                            .cloned()
                            .unwrap_or(JsVal::BuiltinPromise),
                        Expr::IdentName { name, .. } if name == "Promise" => JsVal::BuiltinPromise,
                        _ => eval_in_gen(object, gen_idx, gen_fns, fn_bind, gens)?,
                    };
                    if matches!(obj, JsVal::BuiltinPromise) && args.len() == 1 {
                        let Arg::Expr(e) = &args[0] else {
                            return Err(());
                        };
                        return eval_in_gen(e, gen_idx, gen_fns, fn_bind, gens);
                    }
                }
            }
            // Resolve callee in gen env; spawn if GenFn.
            let c = match callee.as_ref() {
                Expr::Local { id, .. } => gens[gen_idx].env.get(id).cloned().ok_or(())?,
                Expr::Function {
                    name,
                    params,
                    body,
                    is_generator: true,
                    is_arrow: false,
                    ..
                } => register_gen_fn_expr(*name, params, body, gen_fns, fn_bind).map_err(|_| ())?,
                _ => return Err(()),
            };
            // Build args using gen env.
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => {
                        arg_vals.push(eval_in_gen(e, gen_idx, gen_fns, fn_bind, gens)?)
                    }
                    _ => return Err(()),
                }
            }
            spawn_gen_vals(c, &arg_vals, None, gen_fns, fn_bind, gens).map_err(|_| ())
        }
        _ => Err(()),
    }
}

fn spawn_gen_vals(
    callee: JsVal,
    args: &[JsVal],
    this_val: Option<JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::GenFn(fid) = callee else {
        return Err(Ev::U);
    };
    if fid >= gen_fns.len() {
        return Err(Ev::U);
    }
    let n_params = gen_fns[fid].params.len();
    if args.len() > n_params {
        return Err(Ev::U);
    }
    let params = gen_fns[fid].params.clone();
    let name = gen_fns[fid].name;
    let mut gen_env = HashMap::new();
    for (i, pid) in params.iter().enumerate() {
        let v = if i < args.len() {
            args[i].clone()
        } else {
            JsVal::Undef
        };
        gen_env.insert(*pid, v);
    }
    // Free GenFn bindings: inject all known gen fn bindings so nested yield* can call peers.
    for (lid, idx) in fn_bind {
        gen_env.entry(*lid).or_insert(JsVal::GenFn(*idx));
    }
    // Named function expression self-binding.
    if let Some(n) = name {
        gen_env.insert(n, JsVal::GenFn(fid));
    }
    let idx = gens.len();
    gens.push(GenInst {
        fn_id: fid,
        pc: 0,
        started: false,
        suspended: false,
        done: false,
        env: gen_env,
        this_val,
        yield_star: None,
        try_ctx: None,
    });
    Ok(JsVal::GenInst(idx))
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
        let name = format!("@.gstr.{}", self.str_consts.len());
        self.str_consts.push((s.to_string(), name.clone()));
        name
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

    fn emit_bool(&mut self, b: bool) {
        let v: u8 = if b { 1 } else { 0 };
        writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
    }

    fn emit_undef(&mut self) {
        let s = self.string_const("undefined");
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {s}"))).ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_generators: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Bool(b) => self.emit_bool(*b),
                JsVal::Str(s) => {
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                JsVal::Undef => self.emit_undef(),
                _ => return Err(diag("es_generators: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.12 + N08.16.44 async generators)"
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


