//! N08.12.01–N08.12.03: native observations for generator function declaration +
//! `yield` / `return` + `.next()` / `.next(arg)` → `{value, done}` (E13.01–E13.03).
//!
//! Compile-time evaluation of a small generator subset: generator decls with
//! simple ident params, `yield` of number/binary/local/`void 0` (bare yield),
//! `let x = yield …` resume binding, `return` of same, iterator `.next()` /
//! `.next(arg)` (first arg ignored; later args become the yield value), and
//! property reads `.value` / `.done`. Emits Runtime prints of final top-level
//! number/boolean/undefined locals.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, IrType as Type, Local, LocalId, Module, Param, Pattern, Stmt};
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
    Undef,
    /// Generator function binding.
    GenFn(LocalId),
    /// Live generator instance index into `gens`.
    GenInst(usize),
    /// Iterator result `{ value, done }`.
    Result { value: Box<JsVal>, done: bool },
}

#[derive(Clone, Debug)]
struct GenFnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

/// Suspended generator: body + program counter + param env + done flag.
struct GenInst {
    fn_id: LocalId,
    /// Next statement index to execute (or complete if suspended).
    pc: usize,
    /// True after the first `.next` has started execution.
    started: bool,
    /// True when paused on the statement at `pc` awaiting resume value.
    suspended: bool,
    done: bool,
    env: HashMap<LocalId, JsVal>,
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
    if !body_ok(&module.body, &by_id) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    let mut gen_fns: HashMap<LocalId, GenFnRec> = HashMap::new();
    let mut gens: Vec<GenInst> = Vec::new();
    let mut user_locals = Vec::new();

    // Hoist generator function decls.
    for stmt in &module.body {
        if let Stmt::Function {
            local,
            params,
            body,
            is_async: false,
            is_generator: true,
            ..
        } = stmt
        {
            let param_ids = simple_param_locals(params)?;
            gen_fns.insert(
                *local,
                GenFnRec {
                    params: param_ids,
                    body: body.clone(),
                },
            );
            env.insert(*local, JsVal::GenFn(*local));
        }
    }

    match eval_body(&module.body, &mut env, &gen_fns, &mut gens) {
        Ok(()) => {}
        Err(()) => return None,
    }

    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            if matches!(loc.ty, Type::Number | Type::Any | Type::Boolean) {
                match env.get(local) {
                    Some(JsVal::Num(_) | JsVal::Bool(_) | JsVal::Undef) => {
                        user_locals.push(*local)
                    }
                    Some(JsVal::GenFn(_) | JsVal::GenInst(_) | JsVal::Result { .. }) => {}
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
            JsVal::Num(_) | JsVal::Bool(_) | JsVal::Undef => {
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
        _ => false,
    })
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
                Type::Number | Type::Any | Type::Boolean | Type::Function
            ) {
                return false;
            }
            match init {
                None => true,
                Some(e) => expr_ok(e, by_id),
            }
        }
        Stmt::Function {
            params,
            body,
            is_async: false,
            is_generator: true,
            ..
        } => simple_param_locals(params).is_some() && gen_body_ok(body, by_id),
        Stmt::Function {
            is_generator: false,
            ..
        } => false,
        Stmt::Expr { expr } => expr_ok(expr, by_id),
        _ => false,
    }
}

fn gen_body_ok(body: &[Stmt], by_id: &HashMap<LocalId, &Local>) -> bool {
    body.iter().all(|s| match s {
        Stmt::Expr { expr } => match expr {
            Expr::Unary {
                op: UnaryOp::Yield,
                arg,
                ..
            } => expr_ok(arg, by_id),
            _ => false,
        },
        Stmt::Declare { local, init, .. } => {
            let Some(loc) = by_id.get(local) else {
                return false;
            };
            if !matches!(loc.ty, Type::Number | Type::Any | Type::Boolean) {
                return false;
            }
            match init {
                None => true,
                Some(Expr::Unary {
                    op: UnaryOp::Yield,
                    arg,
                    ..
                }) => expr_ok(arg, by_id),
                Some(e) => expr_ok(e, by_id),
            }
        }
        Stmt::Return { value } => match value {
            None => true,
            Some(e) => expr_ok(e, by_id),
        },
        _ => false,
    })
}

fn expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { .. } | Expr::Boolean { .. } | Expr::String { .. } => true,
        Expr::Local { id, .. } => by_id.contains_key(id),
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
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => expr_ok(object, by_id) && expr_ok(property, by_id),
        Expr::Unary {
            op: UnaryOp::Yield | UnaryOp::Void,
            arg,
            ..
        } => expr_ok(arg, by_id),
        Expr::Binary {
            left,
            op:
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Rem,
            right,
            ..
        } => expr_ok(left, by_id) && expr_ok(right, by_id),
        _ => false,
    }
}

fn eval_body(
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &HashMap<LocalId, GenFnRec>,
    gens: &mut Vec<GenInst>,
) -> Result<(), ()> {
    for stmt in body {
        eval_stmt(stmt, env, gen_fns, gens)?;
    }
    Ok(())
}

fn eval_stmt(
    stmt: &Stmt,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &HashMap<LocalId, GenFnRec>,
    gens: &mut Vec<GenInst>,
) -> Result<(), ()> {
    match stmt {
        Stmt::Function { .. } => Ok(()),
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => eval_expr(e, env, gen_fns, gens)?,
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(())
        }
        Stmt::Expr { expr } => {
            eval_expr(expr, env, gen_fns, gens)?;
            Ok(())
        }
        _ => Err(()),
    }
}

fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &HashMap<LocalId, GenFnRec>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| ())?;
            Ok(JsVal::Num(n))
        }
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::Local { id, .. } => env.get(id).cloned().ok_or(()),
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => Ok(JsVal::Undef),
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            let l = eval_expr(left, env, gen_fns, gens)?;
            let r = eval_expr(right, env, gen_fns, gens)?;
            bin_num(op, &l, &r)
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            // Method call: `it.next()` / `it.next(arg)`.
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = callee.as_ref()
            {
                let obj = eval_expr(object, env, gen_fns, gens)?;
                let prop = prop_name(property)?;
                if prop == "next" {
                    let resume = if args.is_empty() {
                        JsVal::Undef
                    } else if args.len() == 1 {
                        match &args[0] {
                            Arg::Expr(e) => eval_expr(e, env, gen_fns, gens)?,
                            _ => return Err(()),
                        }
                    } else {
                        return Err(());
                    };
                    return gen_next(&obj, resume, gen_fns, gens);
                }
                return Err(());
            }

            // Call generator function: `g(args)` → iterator.
            let c = eval_expr(callee, env, gen_fns, gens)?;
            let JsVal::GenFn(fid) = c else {
                return Err(());
            };
            let rec = gen_fns.get(&fid).ok_or(())?;
            if args.len() != rec.params.len() {
                // Missing args → undefined; extra → ignore (ECMA-262). Keep strict match for fixture.
                if args.len() > rec.params.len() {
                    return Err(());
                }
            }
            let mut gen_env = HashMap::new();
            for (i, pid) in rec.params.iter().enumerate() {
                let v = if i < args.len() {
                    match &args[i] {
                        Arg::Expr(e) => eval_expr(e, env, gen_fns, gens)?,
                        _ => return Err(()),
                    }
                } else {
                    JsVal::Undef
                };
                gen_env.insert(*pid, v);
            }
            let idx = gens.len();
            gens.push(GenInst {
                fn_id: fid,
                pc: 0,
                started: false,
                suspended: false,
                done: false,
                env: gen_env,
            });
            Ok(JsVal::GenInst(idx))
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = eval_expr(object, env, gen_fns, gens)?;
            let prop = prop_name(property)?;
            match obj {
                JsVal::Result { value, done } => match prop.as_str() {
                    "value" => Ok(*value),
                    "done" => Ok(JsVal::Bool(done)),
                    _ => Err(()),
                },
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

fn bin_num(op: &BinaryOp, left: &JsVal, right: &JsVal) -> Result<JsVal, ()> {
    let JsVal::Num(a) = left else {
        return Err(());
    };
    let JsVal::Num(b) = right else {
        return Err(());
    };
    let n = match op {
        BinaryOp::Add => a + b,
        BinaryOp::Sub => a - b,
        BinaryOp::Mul => a * b,
        BinaryOp::Div => a / b,
        BinaryOp::Rem => a % b,
        _ => return Err(()),
    };
    Ok(JsVal::Num(n))
}

fn prop_name(expr: &Expr) -> Result<String, ()> {
    match expr {
        Expr::String { value, .. } => Ok(value.to_string_lossy()),
        _ => Err(()),
    }
}

/// Resume generator until next `yield` / `return` / end.
/// First `.next` ignores `resume`; later calls inject it as the yield value.
fn gen_next(
    obj: &JsVal,
    resume: JsVal,
    gen_fns: &HashMap<LocalId, GenFnRec>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, ()> {
    let JsVal::GenInst(idx) = obj else {
        return Err(());
    };
    if *idx >= gens.len() {
        return Err(());
    }
    if gens[*idx].done {
        return Ok(JsVal::Result {
            value: Box::new(JsVal::Undef),
            done: true,
        });
    }

    let mut inject: Option<JsVal> = if !gens[*idx].started {
        gens[*idx].started = true;
        None
    } else if gens[*idx].suspended {
        gens[*idx].suspended = false;
        Some(resume)
    } else {
        // Resumed without a pending yield — treat like inject undefined.
        Some(JsVal::Undef)
    };

    let fn_id = gens[*idx].fn_id;
    let body = gen_fns.get(&fn_id).ok_or(())?.body.clone();

    loop {
        let pc = gens[*idx].pc;
        if pc >= body.len() {
            gens[*idx].done = true;
            return Ok(JsVal::Result {
                value: Box::new(JsVal::Undef),
                done: true,
            });
        }

        match &body[pc] {
            Stmt::Expr {
                expr:
                    Expr::Unary {
                        op: UnaryOp::Yield,
                        arg,
                        ..
                    },
            } => {
                if let Some(_v) = inject.take() {
                    // Complete prior yield (value discarded for bare yield stmt).
                    gens[*idx].pc = pc + 1;
                    continue;
                }
                let yv = eval_in_gen(arg, &gens[*idx].env)?;
                gens[*idx].suspended = true;
                return Ok(JsVal::Result {
                    value: Box::new(yv),
                    done: false,
                });
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
                    gens[*idx].env.insert(*local, v);
                    gens[*idx].pc = pc + 1;
                    continue;
                }
                let yv = eval_in_gen(arg, &gens[*idx].env)?;
                gens[*idx].suspended = true;
                return Ok(JsVal::Result {
                    value: Box::new(yv),
                    done: false,
                });
            }
            Stmt::Declare { local, init, .. } => {
                let v = match init {
                    None => JsVal::Undef,
                    Some(e) => eval_in_gen(e, &gens[*idx].env)?,
                };
                gens[*idx].env.insert(*local, v);
                gens[*idx].pc = pc + 1;
            }
            Stmt::Return { value } => {
                let v = match value {
                    None => JsVal::Undef,
                    Some(e) => eval_in_gen(e, &gens[*idx].env)?,
                };
                gens[*idx].pc = body.len();
                gens[*idx].done = true;
                return Ok(JsVal::Result {
                    value: Box::new(v),
                    done: true,
                });
            }
            _ => return Err(()),
        }
    }
}

/// Evaluate yield/return argument in generator body (params + arithmetic + bare yield).
fn eval_in_gen(expr: &Expr, gen_env: &HashMap<LocalId, JsVal>) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| ())?;
            Ok(JsVal::Num(n))
        }
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::Local { id, .. } => gen_env.get(id).cloned().ok_or(()),
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => Ok(JsVal::Undef),
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            let l = eval_in_gen(left, gen_env)?;
            let r = eval_in_gen(right, gen_env)?;
            bin_num(op, &l, &r)
        }
        _ => Err(()),
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
                JsVal::Undef => self.emit_undef(),
                _ => return Err(diag("es_generators: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.12.03 generators yield_resume)"
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
