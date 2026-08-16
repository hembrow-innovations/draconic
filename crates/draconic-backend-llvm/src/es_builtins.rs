//! N08.14.01: native observations for global object basics (E15.01).
//!
//! Compile-time evaluation of `undefined`, `globalThis`, fundamental constructors
//! `Object`/`Function`/`Array`/`String`/`Boolean` (`typeof`, identity via `===`/`!==`,
//! `globalThis.X === X`, `Object["prototype"]` / `Array["isArray"]` presence). Emits
//! Runtime prints of final top-level number/string/bool locals.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Expr, IrType as Type, Local, LocalId, Module, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

pub(crate) fn is_es_builtins_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_builtins(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_builtins module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum BuiltinId {
    Undefined,
    GlobalThis,
    Object,
    Function,
    Array,
    String,
    Boolean,
    ObjectPrototype,
    ArrayIsArray,
}

#[derive(Clone, Debug, PartialEq)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Builtin(BuiltinId),
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

struct Emitter {
    out: String,
    body: String,
    str_consts: Vec<(String, String)>,
}

fn js_string_to_utf8(s: &JsString) -> String {
    s.to_string_lossy()
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_fundamental(module, &by_id) {
        return None;
    }
    if !module.body.iter().all(|s| matches!(s, Stmt::Declare { .. } | Stmt::Expr { .. })) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    for loc in &module.locals {
        if let Some(b) = builtin_for_name(&loc.name) {
            env.insert(loc.id, match b {
                BuiltinId::Undefined => JsVal::Undef,
                other => JsVal::Builtin(other),
            });
        }
    }

    if eval_body(&module.body, &mut env).is_err() {
        return None;
    }

    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match env.get(local) {
                Some(v @ (JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_))) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String
                    ) {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(JsVal::Undef | JsVal::Builtin(_)) => {}
                None => return None,
            }
        }
    }

    if user_locals.is_empty() {
        return None;
    }

    Some(ModuleInfo {
        user_locals,
        values,
    })
}

fn builtin_for_name(name: &str) -> Option<BuiltinId> {
    match name {
        "undefined" => Some(BuiltinId::Undefined),
        "globalThis" => Some(BuiltinId::GlobalThis),
        "Object" => Some(BuiltinId::Object),
        "Function" => Some(BuiltinId::Function),
        "Array" => Some(BuiltinId::Array),
        "String" => Some(BuiltinId::String),
        "Boolean" => Some(BuiltinId::Boolean),
        _ => None,
    }
}

fn module_has_fundamental(module: &Module, by_id: &HashMap<LocalId, &Local>) -> bool {
    module.body.iter().any(|s| stmt_has_fundamental(s, by_id))
}

fn stmt_has_fundamental(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } => {
            expr_has_fundamental(e, by_id)
        }
        _ => false,
    }
}

fn expr_has_fundamental(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, .. } => by_id.get(id).is_some_and(|l| {
            matches!(
                l.name.as_str(),
                "Object" | "Function" | "Array" | "String" | "Boolean" | "globalThis"
            )
        }),
        Expr::Unary { arg, .. } => expr_has_fundamental(arg, by_id),
        Expr::Binary { left, right, .. } => {
            expr_has_fundamental(left, by_id) || expr_has_fundamental(right, by_id)
        }
        Expr::Member { object, property, .. } => {
            expr_has_fundamental(object, by_id) || expr_has_fundamental(property, by_id)
        }
        Expr::Call { callee, args, .. } => {
            expr_has_fundamental(callee, by_id)
                || args.iter().any(|a| match a {
                    draconic_ir::Arg::Expr(e) => expr_has_fundamental(e, by_id),
                    _ => false,
                })
        }
        _ => false,
    }
}

fn eval_body(body: &[Stmt], env: &mut HashMap<LocalId, JsVal>) -> Result<(), ()> {
    for stmt in body {
        eval_stmt(stmt, env)?;
    }
    Ok(())
}

fn eval_stmt(stmt: &Stmt, env: &mut HashMap<LocalId, JsVal>) -> Result<(), ()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => eval_expr(e, env)?,
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(())
        }
        Stmt::Expr { expr } => {
            let _ = eval_expr(expr, env)?;
            Ok(())
        }
        _ => Err(()),
    }
}

fn eval_expr(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| ())?;
            Ok(JsVal::Num(n))
        }
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::String { value, .. } => Ok(JsVal::Str(js_string_to_utf8(value))),
        Expr::Null { .. } => Err(()),
        Expr::Local { id, .. } => env.get(id).cloned().ok_or(()),
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            let v = eval_expr(arg, env)?;
            Ok(JsVal::Str(typeof_str(&v)))
        }
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            let l = eval_expr(left, env)?;
            let r = eval_expr(right, env)?;
            eval_binary(op, &l, &r)
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = eval_expr(object, env)?;
            let key = eval_key(property, env)?;
            member_get(&obj, &key)
        }
        _ => Err(()),
    }
}

fn eval_key(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<String, ()> {
    match expr {
        Expr::String { value, .. } => Ok(js_string_to_utf8(value)),
        e => match eval_expr(e, env)? {
            JsVal::Str(s) => Ok(s),
            JsVal::Num(n) => Ok(format!("{}", n as i64)),
            _ => Err(()),
        },
    }
}

fn member_get(obj: &JsVal, key: &str) -> Result<JsVal, ()> {
    match obj {
        JsVal::Builtin(BuiltinId::GlobalThis) => match key {
            "Object" => Ok(JsVal::Builtin(BuiltinId::Object)),
            "Function" => Ok(JsVal::Builtin(BuiltinId::Function)),
            "Array" => Ok(JsVal::Builtin(BuiltinId::Array)),
            "String" => Ok(JsVal::Builtin(BuiltinId::String)),
            "Boolean" => Ok(JsVal::Builtin(BuiltinId::Boolean)),
            "undefined" => Ok(JsVal::Undef),
            "globalThis" => Ok(JsVal::Builtin(BuiltinId::GlobalThis)),
            _ => Err(()),
        },
        JsVal::Builtin(BuiltinId::Object) if key == "prototype" => {
            Ok(JsVal::Builtin(BuiltinId::ObjectPrototype))
        }
        JsVal::Builtin(BuiltinId::Array) if key == "isArray" => {
            Ok(JsVal::Builtin(BuiltinId::ArrayIsArray))
        }
        _ => Err(()),
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::Builtin(BuiltinId::Undefined) => "undefined".into(),
        JsVal::Builtin(BuiltinId::GlobalThis | BuiltinId::ObjectPrototype) => "object".into(),
        JsVal::Builtin(
            BuiltinId::Object
            | BuiltinId::Function
            | BuiltinId::Array
            | BuiltinId::String
            | BuiltinId::Boolean
            | BuiltinId::ArrayIsArray,
        ) => "function".into(),
    }
}

fn strict_eq(l: &JsVal, r: &JsVal) -> bool {
    match (l, r) {
        (JsVal::Num(a), JsVal::Num(b)) => a == b,
        (JsVal::Bool(a), JsVal::Bool(b)) => a == b,
        (JsVal::Str(a), JsVal::Str(b)) => a == b,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Builtin(a), JsVal::Builtin(b)) => a == b,
        (JsVal::Undef, JsVal::Builtin(BuiltinId::Undefined))
        | (JsVal::Builtin(BuiltinId::Undefined), JsVal::Undef) => true,
        _ => false,
    }
}

fn eval_binary(op: &BinaryOp, l: &JsVal, r: &JsVal) -> Result<JsVal, ()> {
    match op {
        BinaryOp::EqEqEq => Ok(JsVal::Bool(strict_eq(l, r))),
        BinaryOp::NotEqEq => Ok(JsVal::Bool(!strict_eq(l, r))),
        BinaryOp::EqEq => Ok(JsVal::Bool(strict_eq(l, r))),
        BinaryOp::NotEq => Ok(JsVal::Bool(!strict_eq(l, r))),
        _ => Err(()),
    }
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

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_builtins: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => {
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                JsVal::Bool(b) => {
                    let s = if *b { "true" } else { "false" };
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                _ => return Err(diag("es_builtins: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.14.01 global builtins basics)"
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

    fn compile(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn global_basics_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/builtins/global_basics.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("undefined") && ir.contains("object") && ir.contains("function"),
            "should print typeof observations:\n{ir}"
        );
        assert!(
            ir.contains("true"),
            "should print boolean identity observations:\n{ir}"
        );
    }
}
