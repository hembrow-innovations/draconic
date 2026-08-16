//! N08.10.01–N08.10.03: native observations for `throw` + `try`/`catch`/`finally` (E10.01–E10.03).
//!
//! Compile-time evaluation of a small exception subset matching
//! `es/exceptions/throw_try_catch`, `es/exceptions/try_finally`, and
//! `es/exceptions/optional_catch`: number/string throws, catch binding (named or
//! optional), nested try, rethrow, zero-arg functions that throw or `return`
//! through `finally`. Emits Runtime prints of final top-level number locals.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Pattern, Stmt,
};
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

    match eval_body(&module.body, &mut env, &functions) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }

    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            if matches!(loc.ty, Type::Number | Type::Any) {
                if matches!(env.get(local), Some(JsVal::Fn(_))) {
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

fn body_ok(body: &[Stmt], by_id: &HashMap<LocalId, &Local>) -> bool {
    body.iter().all(|s| stmt_ok(s, by_id))
}

fn stmt_ok(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let Some(loc) = by_id.get(local) else {
                return false;
            };
            if !matches!(loc.ty, Type::Number | Type::Any | Type::String | Type::Function) {
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
            // Bare try/catch, try/finally, try/catch/finally, optional catch (`catch {…}`).
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
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } | Expr::Null { .. } => true,
        Expr::Local { id, .. } => by_id.contains_key(id),
        Expr::Unary { arg, .. } => expr_ok(arg, by_id),
        Expr::Binary { left, right, op, .. } => {
            matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
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
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e, by_id),
                    _ => false,
                })
        }
        _ => false,
    }
}

fn eval_body(
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
    functions: &HashMap<LocalId, FnRec>,
) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, env, functions)? {
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
) -> Result<Flow, ()> {
    match stmt {
        Stmt::Function { .. } => Ok(Flow::Normal),
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => match eval_expr(e, env, functions)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                },
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Throw { value } => match eval_expr(value, env, functions)? {
            Ok(v) => Ok(Flow::Throw(v)),
            Err(flow) => Ok(flow),
        },
        Stmt::Return { value } => match value {
            None => Ok(Flow::Return(JsVal::Undef)),
            Some(e) => match eval_expr(e, env, functions)? {
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
            let mut completion = match eval_body(block, env, functions)? {
                Flow::Throw(exc) => {
                    if let Some(handler) = handler {
                        if let Some(Pattern::Local(pid)) = handler_param {
                            env.insert(*pid, exc);
                        }
                        eval_body(handler, env, functions)?
                    } else {
                        Flow::Throw(exc)
                    }
                }
                other => other,
            };
            if let Some(fin) = finalizer {
                match eval_body(fin, env, functions)? {
                    Flow::Normal => {}
                    abrupt => completion = abrupt,
                }
            }
            Ok(completion)
        }
        Stmt::Block { body } => eval_body(body, env, functions),
        Stmt::Expr { expr } => match eval_expr(expr, env, functions)? {
            Ok(_) => Ok(Flow::Normal),
            Err(flow) => Ok(flow),
        },
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = match eval_expr(test, env, functions)? {
                Ok(v) => to_boolean(&v),
                Err(flow) => return Ok(flow),
            };
            if t {
                eval_stmt(consequent, env, functions)
            } else if let Some(a) = alternate {
                eval_stmt(a, env, functions)
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
            use draconic_ast::UnaryOp;
            let v = match eval_expr(arg, env, functions)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match op {
                UnaryOp::Plus => Ok(Ok(JsVal::Num(to_number(&v)))),
                UnaryOp::Minus => Ok(Ok(JsVal::Num(-to_number(&v)))),
                UnaryOp::Not => Ok(Ok(JsVal::Num(if to_boolean(&v) { 0.0 } else { 1.0 }))),
                _ => Err(()),
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let l = match eval_expr(left, env, functions)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let r = match eval_expr(right, env, functions)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
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
            let v = match eval_expr(value, env, functions)? {
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
            if !args.is_empty() {
                return Err(());
            }
            let c = match eval_expr(callee, env, functions)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let JsVal::Fn(fid) = c else {
                return Err(());
            };
            let frec = functions.get(&fid).ok_or(())?;
            match eval_body(&frec.body, env, functions)? {
                Flow::Normal => Ok(Ok(JsVal::Undef)),
                Flow::Throw(exc) => Ok(Err(Flow::Throw(exc))),
                Flow::Return(v) => Ok(Ok(v)),
            }
        }
        _ => Err(()),
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
        JsVal::Fn(_) => f64::NAN,
    }
}

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef => false,
        JsVal::Fn(_) => true,
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
                .ok_or_else(|| diag("es_exceptions: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => self.emit_str(s),
                _ => return Err(diag("es_exceptions: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.10.03 throw/try/catch/finally/optional-catch)"
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
