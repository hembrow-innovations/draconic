//! N08.12.01–N08.12.08 + N08.16.44 + N08.16.43.01–N08.16.43.02: native observations
//! for generator function declaration + expression + methods (object/class/static) +
//! `yield` / `yield*` / `return` + `.next()` / `.next(arg)` / `.return(arg)` /
//! `.throw(arg)` → `{value, done}` + `for-of` over generators (E13.01–E13.08),
//! async generators (E18.43): `async function*` / `{ async *m() }` / class
//! `async *m()` / `static async *m()`, `.next()` thenables, `await
//! Promise.resolve`, `for await` over async gens inside `async function`,
//! `for await` over arrays (CreateAsyncFromSyncIterator values) with
//! `let`/`const`/assign binding + `break`/`continue` (N08.16.43.01), and
//! `for await` over custom `Symbol.asyncIterator` async iterables
//! (N08.16.43.02).
//!
//! Compile-time evaluation of a small generator / for-await subset: generator
//! decls and `function*` / `async function*` expressions (incl. named + IIFE)
//! with simple ident params, object/class generator methods (`*m()` /
//! `async *m()` / `static *m()`), plain function expressions as object methods,
//! `this` prop reads in methods, `yield` of number/string/binary/local/GenFn/
//! `void 0` (bare yield), `let x = yield …` resume binding, `yield*` of
//! generators/arrays (incl. completion value), `return` of same, iterator
//! `.next()` / `.return()` / `.throw()`, try/catch/finally in generator bodies,
//! property reads `.value` / `.done` / array `.length` / index, top-level
//! `for-of` / `try` over generators, async-gen thenables / await / for-await,
//! async functions with `for await` over arrays or `Symbol.asyncIterator`
//! custom async iterables (`Promise.resolve` settle identity).
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

#[path = "es_generators_eval.rs"]
mod eval;
#[path = "es_generators_class.rs"]
mod class;
#[path = "es_generators_exec.rs"]
mod exec;

use eval::eval_body;

pub(crate) fn is_es_generators_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_generators(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_generators module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_generators(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_generators_module(module) {
        return None;
    }
    Some(emit_es_generators(module))
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
    Result {
        value: Box<JsVal>,
        done: bool,
    },
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
    /// Well-known symbol key (e.g. `asyncIterator` from `Symbol.asyncIterator`).
    SymKey(&'static str),
}

/// Loop control from break/continue inside for-of bodies; throw for `.throw` / try;
/// `Return` for nested `return` inside plain/async function bodies.
#[derive(Clone, Debug)]
enum Flow {
    Next,
    Break,
    Continue,
    Return(JsVal),
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
    if !module_has_generator(&module.body) && !module_has_for_await(&module.body) {
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

    // Seed builtins: Promise, Symbol.asyncIterator, undefined.
    for loc in &module.locals {
        match loc.name.as_str() {
            "Promise" => {
                env.insert(loc.id, JsVal::BuiltinPromise);
            }
            "undefined" => {
                env.insert(loc.id, JsVal::Undef);
            }
            "Symbol" => {
                let mut props = HashMap::new();
                props.insert("asyncIterator".into(), JsVal::SymKey("asyncIterator"));
                env.insert(
                    loc.id,
                    JsVal::Object {
                        props,
                        methods: HashMap::new(),
                    },
                );
            }
            _ => {}
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
                        | JsVal::BuiltinPromise
                        | JsVal::SymKey(_),
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

/// True when body (incl. nested function bodies) contains `for await…of`.
fn module_has_for_await(body: &[Stmt]) -> bool {
    body.iter().any(stmt_has_for_await)
}

fn stmt_has_for_await(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::ForOf { is_await: true, .. } => true,
        Stmt::Function { body, .. } | Stmt::Block { body } => module_has_for_await(body),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            stmt_has_for_await(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_has_for_await(a))
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            module_has_for_await(block)
                || handler.as_ref().is_some_and(|h| module_has_for_await(h))
                || finalizer.as_ref().is_some_and(|f| module_has_for_await(f))
        }
        Stmt::Labeled { body, .. } | Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
            stmt_has_for_await(body)
        }
        Stmt::For { body, init, .. } => {
            stmt_has_for_await(body) || init.as_ref().is_some_and(|i| stmt_has_for_await(i))
        }
        Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
            stmt_has_for_await(left) || stmt_has_for_await(body)
        }
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } => expr_has_for_await(e),
        _ => false,
    }
}

fn expr_has_for_await(expr: &Expr) -> bool {
    match expr {
        Expr::Function { body, .. } => module_has_for_await(body),
        Expr::Call { callee, args, .. } => {
            expr_has_for_await(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_for_await(e),
                    _ => false,
                })
        }
        Expr::Member {
            object, property, ..
        } => expr_has_for_await(object) || expr_has_for_await(property),
        Expr::Unary { arg, .. } => expr_has_for_await(arg),
        Expr::Binary { left, right, .. } => expr_has_for_await(left) || expr_has_for_await(right),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) => expr_has_for_await(e),
            _ => false,
        }),
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                expr_has_for_await(value)
            }
            ObjectProp::Spread(e) => expr_has_for_await(e),
        }),
        Expr::New { callee, args, .. } => {
            expr_has_for_await(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_for_await(e),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_has_for_await(value),
        _ => false,
    }
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
        Expr::Member {
            object, property, ..
        } => expr_has_generator(object) || expr_has_generator(property),
        Expr::Unary { arg, .. } => expr_has_generator(arg),
        Expr::Binary { left, right, .. } => expr_has_generator(left) || expr_has_generator(right),
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
        writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
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
