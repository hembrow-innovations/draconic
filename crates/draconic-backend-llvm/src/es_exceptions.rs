//! N08.10.01–N08.10.03 + N08.16.17: native observations for `throw` +
//! `try`/`catch`/`finally` (E10.01–E10.03) and Annex B.3.4 VariableStatements in
//! Catch (E18.17 / `es/annex-b/var_catch`).
//!
//! Compile-time evaluation of a small exception subset: number/string throws,
//! catch binding (named or optional), nested try, rethrow, zero-arg functions
//! that throw or `return` through `finally`, string concat / `typeof` /
//! `String(…)`, and Annex B `catch (e) { var e = … }` (initializer assigns the
//! catch binding; outer `var` stays hoisted-undefined). Emits Runtime prints of
//! final top-level number/string/undefined locals.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, BindingKind, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Pattern, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BYTES, PRINT_F64};

pub(crate) fn is_es_exceptions_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_exceptions(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_exceptions module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Str(String),
    Undef,
    /// Function local bound at declaration.
    Fn(LocalId),
    /// Global `String` (ToString).
    BuiltinString,
}

#[derive(Clone, Debug)]
struct FnRec {
    body: Vec<Stmt>,
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Throw(JsVal),
    Return(JsVal),
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_throw_or_try(&module.body) {
        return None;
    }
    if !body_ok(&module.body, &by_id) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    let mut functions: HashMap<LocalId, FnRec> = HashMap::new();
    let mut user_locals = Vec::new();

    for loc in &module.locals {
        if loc.name == "String" {
            env.insert(loc.id, JsVal::BuiltinString);
        }
    }

    // Hoist function decls first (JS).
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
            if !params.is_empty() {
                return None;
            }
            functions.insert(*local, FnRec { body: body.clone() });
            env.insert(*local, JsVal::Fn(*local));
        }
    }

    hoist_vars_in_stmts(&module.body, &mut env);

    match eval_body(&module.body, &mut env, &functions, &by_id, None) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }

    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            if matches!(loc.ty, Type::Number | Type::Any | Type::String) {
                if matches!(
                    env.get(local),
                    Some(JsVal::Fn(_)) | Some(JsVal::BuiltinString)
                ) {
                    continue;
                }
                user_locals.push(*local);
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
            JsVal::Num(_) | JsVal::Str(_) | JsVal::Undef => {
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

fn module_has_throw_or_try(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::Throw { .. } | Stmt::Try { .. } => true,
        Stmt::Block { body } => module_has_throw_or_try(body),
        Stmt::Function { body, .. } => module_has_throw_or_try(body),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            module_has_throw_or_try(std::slice::from_ref(consequent.as_ref()))
                || alternate
                    .as_ref()
                    .is_some_and(|a| module_has_throw_or_try(std::slice::from_ref(a.as_ref())))
        }
        _ => false,
    })
}

fn hoist_vars_in_stmts(body: &[Stmt], env: &mut HashMap<LocalId, JsVal>) {
    for stmt in body {
        hoist_vars_in_stmt(stmt, env);
    }
}

fn hoist_vars_in_stmt(stmt: &Stmt, env: &mut HashMap<LocalId, JsVal>) {
    match stmt {
        Stmt::Declare {
            local,
            kind: BindingKind::Var,
            ..
        } => {
            env.entry(*local).or_insert(JsVal::Undef);
        }
        Stmt::Block { body } => hoist_vars_in_stmts(body, env),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            hoist_vars_in_stmts(block, env);
            if let Some(h) = handler {
                hoist_vars_in_stmts(h, env);
            }
            if let Some(f) = finalizer {
                hoist_vars_in_stmts(f, env);
            }
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            hoist_vars_in_stmt(consequent, env);
            if let Some(a) = alternate {
                hoist_vars_in_stmt(a, env);
            }
        }
        // Nested functions hoist their own vars on entry.
        Stmt::Function { .. } => {}
        _ => {}
    }
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
                Type::Number | Type::Any | Type::String | Type::Function
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
            is_generator: false,
            ..
        } => params.is_empty() && body_ok(body, by_id),
        Stmt::Throw { value } => expr_ok(value, by_id),
        Stmt::Return { value } => match value {
            None => true,
            Some(e) => expr_ok(e, by_id),
        },
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            match (handler.is_some(), handler_param) {
                (true, None) => {}
                (true, Some(Pattern::Local(_))) => {}
                (false, None) => {}
                _ => return false,
            }
            body_ok(block, by_id)
                && handler.as_ref().is_none_or(|h| body_ok(h, by_id))
                && finalizer.as_ref().is_none_or(|f| body_ok(f, by_id))
        }
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
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } | Expr::Null { .. } => {
            true
        }
        Expr::Local { id, .. } => by_id.contains_key(id),
        Expr::Unary { op, arg, .. } => {
            matches!(
                op,
                UnaryOp::Plus | UnaryOp::Minus | UnaryOp::Not | UnaryOp::TypeOf
            ) && expr_ok(arg, by_id)
        }
        Expr::Binary {
            left, right, op, ..
        } => {
            matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
            ) && expr_ok(left, by_id)
                && expr_ok(right, by_id)
        }
        Expr::Assign {
            target: AssignTarget::Local(_),
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(value, by_id),
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            expr_ok(callee, by_id)
                && args.len() <= 1
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e, by_id),
                    _ => false,
                })
        }
        _ => false,
    }
}

fn same_name(a: LocalId, b: LocalId, by_id: &HashMap<LocalId, &Local>) -> bool {
    match (by_id.get(&a), by_id.get(&b)) {
        (Some(la), Some(lb)) => la.name == lb.name,
        _ => false,
    }
}

fn eval_body(
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
    functions: &HashMap<LocalId, FnRec>,
    by_id: &HashMap<LocalId, &Local>,
    catch_param: Option<LocalId>,
) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, env, functions, by_id, catch_param)? {
            Flow::Normal => {}
            other => return Ok(other),
        }
    }
    Ok(Flow::Normal)
}

fn eval_stmt(
    stmt: &Stmt,
    env: &mut HashMap<LocalId, JsVal>,
    functions: &HashMap<LocalId, FnRec>,
    by_id: &HashMap<LocalId, &Local>,
    catch_param: Option<LocalId>,
) -> Result<Flow, ()> {
    match stmt {
        Stmt::Function { .. } => Ok(Flow::Normal),
        Stmt::Declare {
            local, kind, init, ..
        } => {
            // Annex B.3.4: `catch (e) { var e = init }` — initializer assigns the
            // catch binding; bare `var e` is a no-op on the catch binding; outer
            // var slot stays hoisted-undefined.
            let annex_b_catch = *kind == BindingKind::Var
                && catch_param.is_some_and(|cp| same_name(*local, cp, by_id));
            if annex_b_catch {
                if let Some(e) = init {
                    let v = match eval_expr(e, env, functions, by_id)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(flow),
                    };
                    env.insert(catch_param.unwrap(), v);
                }
                return Ok(Flow::Normal);
            }
            if *kind == BindingKind::Var && init.is_none() {
                // Already hoisted to undefined.
                return Ok(Flow::Normal);
            }
            let v = match init {
                Some(e) => match eval_expr(e, env, functions, by_id)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                },
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Throw { value } => match eval_expr(value, env, functions, by_id)? {
            Ok(v) => Ok(Flow::Throw(v)),
            Err(flow) => Ok(flow),
        },
        Stmt::Return { value } => match value {
            None => Ok(Flow::Return(JsVal::Undef)),
            Some(e) => match eval_expr(e, env, functions, by_id)? {
                Ok(v) => Ok(Flow::Return(v)),
                Err(flow) => Ok(flow),
            },
        },
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            let mut completion = match eval_body(block, env, functions, by_id, catch_param)? {
                Flow::Throw(exc) => {
                    if let Some(handler) = handler {
                        let hp = if let Some(Pattern::Local(pid)) = handler_param {
                            env.insert(*pid, exc);
                            Some(*pid)
                        } else {
                            None
                        };
                        eval_body(handler, env, functions, by_id, hp)?
                    } else {
                        Flow::Throw(exc)
                    }
                }
                other => other,
            };
            if let Some(fin) = finalizer {
                match eval_body(fin, env, functions, by_id, catch_param)? {
                    Flow::Normal => {}
                    abrupt => completion = abrupt,
                }
            }
            Ok(completion)
        }
        Stmt::Block { body } => eval_body(body, env, functions, by_id, catch_param),
        Stmt::Expr { expr } => match eval_expr(expr, env, functions, by_id)? {
            Ok(_) => Ok(Flow::Normal),
            Err(flow) => Ok(flow),
        },
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = match eval_expr(test, env, functions, by_id)? {
                Ok(v) => to_boolean(&v),
                Err(flow) => return Ok(flow),
            };
            if t {
                eval_stmt(consequent, env, functions, by_id, catch_param)
            } else if let Some(a) = alternate {
                eval_stmt(a, env, functions, by_id, catch_param)
            } else {
                Ok(Flow::Normal)
            }
        }
        _ => Err(()),
    }
}

/// `Ok(Ok(v))` = value; `Ok(Err(flow))` = abrupt from nested throw/return; `Err(())` = unsupported.
fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<LocalId, JsVal>,
    functions: &HashMap<LocalId, FnRec>,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<Result<JsVal, Flow>, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| ())?;
            Ok(Ok(JsVal::Num(n)))
        }
        Expr::String { value, .. } => Ok(Ok(JsVal::Str(value.to_string_lossy()))),
        Expr::Boolean { value, .. } => Ok(Ok(JsVal::Num(if *value { 1.0 } else { 0.0 }))),
        Expr::Null { .. } => Ok(Ok(JsVal::Num(0.0))),
        Expr::Local { id, .. } => {
            let v = env.get(id).cloned().ok_or(())?;
            Ok(Ok(v))
        }
        Expr::Unary { op, arg, .. } => {
            let v = match eval_expr(arg, env, functions, by_id)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match op {
                UnaryOp::Plus => Ok(Ok(JsVal::Num(to_number(&v)))),
                UnaryOp::Minus => Ok(Ok(JsVal::Num(-to_number(&v)))),
                UnaryOp::Not => Ok(Ok(JsVal::Num(if to_boolean(&v) { 0.0 } else { 1.0 }))),
                UnaryOp::TypeOf => Ok(Ok(JsVal::Str(typeof_str(&v).to_string()))),
                _ => Err(()),
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let l = match eval_expr(left, env, functions, by_id)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let r = match eval_expr(right, env, functions, by_id)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            if *op == BinaryOp::Add && (matches!(l, JsVal::Str(_)) || matches!(r, JsVal::Str(_))) {
                return Ok(Ok(JsVal::Str(format!(
                    "{}{}",
                    to_string(&l),
                    to_string(&r)
                ))));
            }
            let ln = to_number(&l);
            let rn = to_number(&r);
            let n = match op {
                BinaryOp::Add => ln + rn,
                BinaryOp::Sub => ln - rn,
                BinaryOp::Mul => ln * rn,
                BinaryOp::Div => ln / rn,
                BinaryOp::Rem => ln % rn,
                _ => return Err(()),
            };
            Ok(Ok(JsVal::Num(n)))
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = match eval_expr(value, env, functions, by_id)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            env.insert(*id, v.clone());
            Ok(Ok(v))
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let c = match eval_expr(callee, env, functions, by_id)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match c {
                JsVal::BuiltinString => {
                    if args.len() != 1 {
                        return Err(());
                    }
                    let Arg::Expr(e) = &args[0] else {
                        return Err(());
                    };
                    let v = match eval_expr(e, env, functions, by_id)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(Err(flow)),
                    };
                    Ok(Ok(JsVal::Str(to_string(&v))))
                }
                JsVal::Fn(fid) => {
                    if !args.is_empty() {
                        return Err(());
                    }
                    let frec = functions.get(&fid).ok_or(())?;
                    // Fresh-ish: hoist function-scoped vars, keep outer env (unique LocalIds).
                    hoist_vars_in_stmts(&frec.body, env);
                    match eval_body(&frec.body, env, functions, by_id, None)? {
                        Flow::Normal => Ok(Ok(JsVal::Undef)),
                        Flow::Throw(exc) => Ok(Err(Flow::Throw(exc))),
                        Flow::Return(v) => Ok(Ok(v)),
                    }
                }
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

fn typeof_str(v: &JsVal) -> &'static str {
    match v {
        JsVal::Num(_) => "number",
        JsVal::Str(_) => "string",
        JsVal::Undef => "undefined",
        JsVal::Fn(_) | JsVal::BuiltinString => "function",
    }
}

fn to_string(v: &JsVal) -> String {
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
                // Prefer integer-looking print for whole numbers.
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
        }
        JsVal::Str(s) => s.clone(),
        JsVal::Undef => "undefined".into(),
        JsVal::Fn(_) | JsVal::BuiltinString => "function".into(),
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
        JsVal::Fn(_) | JsVal::BuiltinString => f64::NAN,
    }
}

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef => false,
        JsVal::Fn(_) | JsVal::BuiltinString => true,
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
        writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
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
                .ok_or_else(|| diag("es_exceptions: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => self.emit_str(s),
                JsVal::Undef => self.emit_str("undefined"),
                _ => return Err(diag("es_exceptions: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.10.03 + N08.16.17 throw/try/catch/finally + Annex B var-in-catch)"
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
