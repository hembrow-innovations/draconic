//! N08.11: native observations for linked ESM fixtures (E11).
//!
//! After the linker flattens static imports, module programs are ordinary IR with
//! mangled `__mN_*` locals. Compile-time evaluation covers named/default/cyclic
//! fixtures (number/string values, simple param calls, live `let` assign). Emits
//! Runtime prints of entry top-level number/string locals (not mangled deps).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Pattern, Stmt,
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
}

#[derive(Clone, Debug)]
struct FnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Return(JsVal),
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
        }
    }

    match eval_body(&module.body, &mut env, &functions) {
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
            if matches!(env.get(local), Some(JsVal::Fn(_))) {
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
        } => simple_param_ids(params).is_some() && body_ok(body, by_id),
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
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } | Expr::Null { .. } => true,
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
                Some(e) => eval_expr(e, env, functions)?,
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Return { value } => match value {
            None => Ok(Flow::Return(JsVal::Undef)),
            Some(e) => Ok(Flow::Return(eval_expr(e, env, functions)?)),
        },
        Stmt::Block { body } => eval_body(body, env, functions),
        Stmt::Expr { expr } => {
            eval_expr(expr, env, functions)?;
            Ok(Flow::Normal)
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = to_boolean(&eval_expr(test, env, functions)?);
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

fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<LocalId, JsVal>,
    functions: &HashMap<LocalId, FnRec>,
) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
            let n: f64 = cleaned.parse().map_err(|_| ())?;
            Ok(JsVal::Num(n))
        }
        Expr::String { value, .. } => Ok(JsVal::Str(value.to_string_lossy())),
        Expr::Boolean { value, .. } => Ok(JsVal::Num(if *value { 1.0 } else { 0.0 })),
        Expr::Null { .. } => Ok(JsVal::Num(0.0)),
        Expr::Local { id, .. } => env.get(id).cloned().ok_or(()),
        Expr::Unary { op, arg, .. } => {
            use draconic_ast::UnaryOp;
            let v = eval_expr(arg, env, functions)?;
            match op {
                UnaryOp::Plus => Ok(JsVal::Num(to_number(&v))),
                UnaryOp::Minus => Ok(JsVal::Num(-to_number(&v))),
                UnaryOp::Not => Ok(JsVal::Num(if to_boolean(&v) { 0.0 } else { 1.0 })),
                _ => Err(()),
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let l = eval_expr(left, env, functions)?;
            let r = eval_expr(right, env, functions)?;
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
            Ok(JsVal::Num(n))
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = eval_expr(value, env, functions)?;
            env.insert(*id, v.clone());
            Ok(v)
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let c = eval_expr(callee, env, functions)?;
            let JsVal::Fn(fid) = c else {
                return Err(());
            };
            let frec = functions.get(&fid).ok_or(())?.clone();
            if args.len() > frec.params.len() {
                return Err(());
            }
            let mut arg_vals = Vec::with_capacity(args.len());
            for a in args {
                match a {
                    Arg::Expr(e) => arg_vals.push(eval_expr(e, env, functions)?),
                    _ => return Err(()),
                }
            }
            for (i, pid) in frec.params.iter().enumerate() {
                let v = arg_vals.get(i).cloned().unwrap_or(JsVal::Undef);
                env.insert(*pid, v);
            }
            match eval_body(&frec.body, env, functions)? {
                Flow::Normal => Ok(JsVal::Undef),
                Flow::Return(v) => Ok(v),
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
                .ok_or_else(|| diag("es_modules: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => self.emit_str(s),
                _ => return Err(diag("es_modules: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.11 linked ESM modules)"
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
