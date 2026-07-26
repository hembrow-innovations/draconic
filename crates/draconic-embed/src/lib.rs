//! Embed: compiler-in-runtime for `eval` / `Function` (ROADMAP N07).
//!
//! N07.01: compile simple expression source strings through Frontend → IR and
//! evaluate them with a minimal IR interpreter (completion value of the script).

use std::collections::HashMap;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_check::check;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{lower, Expr, LocalId, Module, Stmt};
use draconic_parser::parse;

/// JS-ish value produced by Embed eval (N07.01 subset).
#[derive(Debug, Clone, PartialEq)]
pub enum EmbedValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
}

impl EmbedValue {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            EmbedValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            EmbedValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn typeof_name(&self) -> &'static str {
        match self {
            EmbedValue::Undefined => "undefined",
            EmbedValue::Null => "object",
            EmbedValue::Boolean(_) => "boolean",
            EmbedValue::Number(_) => "number",
            EmbedValue::String(_) => "string",
        }
    }
}

/// Compile `source` as a Script and return its completion value.
///
/// N07.01 supports expression scripts built from literals, arithmetic, unary
/// `+/-`, grouping, and `typeof` on the supported value set (incl. `undefined`).
pub fn eval_source(source: &str) -> Result<EmbedValue, Diagnostic> {
    let program = parse(source)?;
    let checked = check(program)?;
    let module = lower(&checked);
    interpret_module(&module)
}

fn interpret_module(module: &Module) -> Result<EmbedValue, Diagnostic> {
    let mut env: HashMap<LocalId, EmbedValue> = HashMap::new();
    for local in &module.locals {
        if local.name == "undefined" {
            env.insert(local.id, EmbedValue::Undefined);
        }
    }
    let mut completion = EmbedValue::Undefined;
    for stmt in &module.body {
        completion = exec_stmt(stmt, &mut env, module)?;
    }
    Ok(completion)
}

fn exec_stmt(
    stmt: &Stmt,
    env: &mut HashMap<LocalId, EmbedValue>,
    module: &Module,
) -> Result<EmbedValue, Diagnostic> {
    match stmt {
        Stmt::Expr { expr } => eval_expr(expr, env, module),
        Stmt::Block { body } => {
            let mut last = EmbedValue::Undefined;
            for s in body {
                last = exec_stmt(s, env, module)?;
            }
            Ok(last)
        }
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => eval_expr(e, env, module)?,
                None => EmbedValue::Undefined,
            };
            env.insert(*local, v);
            Ok(EmbedValue::Undefined)
        }
        other => Err(diag(format!(
            "embed eval does not support statement: {other:?}"
        ))),
    }
}

fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<LocalId, EmbedValue>,
    module: &Module,
) -> Result<EmbedValue, Diagnostic> {
    match expr {
        Expr::Number { raw, .. } => {
            let n = parse_number_raw(raw)?;
            Ok(EmbedValue::Number(n))
        }
        Expr::String { value, .. } => Ok(EmbedValue::String(value.to_string_lossy())),
        Expr::Boolean { value, .. } => Ok(EmbedValue::Boolean(*value)),
        Expr::Null { .. } => Ok(EmbedValue::Null),
        Expr::Local { id, .. } => {
            if let Some(v) = env.get(id) {
                return Ok(v.clone());
            }
            if let Some(local) = module.locals.iter().find(|l| l.id == *id) {
                if local.name == "undefined" {
                    return Ok(EmbedValue::Undefined);
                }
            }
            Err(diag(format!(
                "embed eval: unbound local %{}",
                id.0
            )))
        }
        Expr::Unary { op, arg, .. } => {
            let v = eval_expr(arg, env, module)?;
            eval_unary(*op, v)
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let l = eval_expr(left, env, module)?;
            // Short-circuit not required for arithmetic subset; still evaluate both.
            let r = eval_expr(right, env, module)?;
            eval_binary(*op, l, r)
        }
        other => Err(diag(format!(
            "embed eval does not support expression: {other:?}"
        ))),
    }
}

fn eval_unary(op: UnaryOp, v: EmbedValue) -> Result<EmbedValue, Diagnostic> {
    match op {
        UnaryOp::Plus => Ok(EmbedValue::Number(to_number(&v)?)),
        UnaryOp::Minus => Ok(EmbedValue::Number(-to_number(&v)?)),
        UnaryOp::TypeOf => Ok(EmbedValue::String(v.typeof_name().to_string())),
        UnaryOp::Void => Ok(EmbedValue::Undefined),
        UnaryOp::Not => Ok(EmbedValue::Boolean(!to_boolean(&v))),
        other => Err(diag(format!(
            "embed eval does not support unary op: {other}"
        ))),
    }
}

fn eval_binary(op: BinaryOp, left: EmbedValue, right: EmbedValue) -> Result<EmbedValue, Diagnostic> {
    match op {
        BinaryOp::Add => {
            if matches!(left, EmbedValue::String(_)) || matches!(right, EmbedValue::String(_)) {
                Ok(EmbedValue::String(format!(
                    "{}{}",
                    to_string_js(&left),
                    to_string_js(&right)
                )))
            } else {
                Ok(EmbedValue::Number(to_number(&left)? + to_number(&right)?))
            }
        }
        BinaryOp::Sub => Ok(EmbedValue::Number(to_number(&left)? - to_number(&right)?)),
        BinaryOp::Mul => Ok(EmbedValue::Number(to_number(&left)? * to_number(&right)?)),
        BinaryOp::Div => Ok(EmbedValue::Number(to_number(&left)? / to_number(&right)?)),
        BinaryOp::Rem => Ok(EmbedValue::Number(to_number(&left)? % to_number(&right)?)),
        BinaryOp::Comma => Ok(right),
        other => Err(diag(format!(
            "embed eval does not support binary op: {other:?}"
        ))),
    }
}

fn to_number(v: &EmbedValue) -> Result<f64, Diagnostic> {
    match v {
        EmbedValue::Number(n) => Ok(*n),
        EmbedValue::Boolean(true) => Ok(1.0),
        EmbedValue::Boolean(false) => Ok(0.0),
        EmbedValue::Null => Ok(0.0),
        EmbedValue::Undefined => Ok(f64::NAN),
        EmbedValue::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                return Ok(0.0);
            }
            t.parse::<f64>()
                .map_err(|_| diag(format!("embed eval: cannot convert string to number: {s:?}")))
        }
    }
}

fn to_boolean(v: &EmbedValue) -> bool {
    match v {
        EmbedValue::Undefined | EmbedValue::Null => false,
        EmbedValue::Boolean(b) => *b,
        EmbedValue::Number(n) => *n != 0.0 && !n.is_nan(),
        EmbedValue::String(s) => !s.is_empty(),
    }
}

fn to_string_js(v: &EmbedValue) -> String {
    match v {
        EmbedValue::Undefined => "undefined".into(),
        EmbedValue::Null => "null".into(),
        EmbedValue::Boolean(b) => b.to_string(),
        EmbedValue::Number(n) => {
            if n.is_nan() {
                "NaN".into()
            } else if *n == f64::INFINITY {
                "Infinity".into()
            } else if *n == f64::NEG_INFINITY {
                "-Infinity".into()
            } else if *n == 0.0 {
                "0".into()
            } else {
                // Prefer integer form when exact.
                if n.fract() == 0.0 && n.abs() < 1e21 {
                    format!("{}", *n as i64)
                } else {
                    n.to_string()
                }
            }
        }
        EmbedValue::String(s) => s.clone(),
    }
}

fn parse_number_raw(raw: &str) -> Result<f64, Diagnostic> {
    let s = raw.replace('_', "");
    if let Some(hex) = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
    {
        return i64::from_str_radix(hex, 16)
            .map(|n| n as f64)
            .map_err(|_| diag(format!("embed eval: bad hex number: {raw}")));
    }
    if let Some(bin) = s
        .strip_prefix("0b")
        .or_else(|| s.strip_prefix("0B"))
    {
        return i64::from_str_radix(bin, 2)
            .map(|n| n as f64)
            .map_err(|_| diag(format!("embed eval: bad binary number: {raw}")));
    }
    if let Some(oct) = s
        .strip_prefix("0o")
        .or_else(|| s.strip_prefix("0O"))
    {
        return i64::from_str_radix(oct, 8)
            .map(|n| n as f64)
            .map_err(|_| diag(format!("embed eval: bad octal number: {raw}")));
    }
    s.parse::<f64>()
        .map_err(|_| diag(format!("embed eval: bad number: {raw}")))
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_number_literal() {
        let v = eval_source("42").unwrap();
        assert_eq!(v, EmbedValue::Number(42.0));
    }

    #[test]
    fn eval_arithmetic_add() {
        let v = eval_source("1 + 2").unwrap();
        assert_eq!(v, EmbedValue::Number(3.0));
    }

    #[test]
    fn eval_arithmetic_mul_div_rem_sub() {
        assert_eq!(eval_source("3 * 4").unwrap(), EmbedValue::Number(12.0));
        assert_eq!(eval_source("10 - 3").unwrap(), EmbedValue::Number(7.0));
        assert_eq!(eval_source("20 / 4").unwrap(), EmbedValue::Number(5.0));
        assert_eq!(eval_source("10 % 3").unwrap(), EmbedValue::Number(1.0));
    }

    #[test]
    fn eval_unary_plus_minus_grouping() {
        assert_eq!(eval_source("-3").unwrap(), EmbedValue::Number(-3.0));
        assert_eq!(eval_source("+5").unwrap(), EmbedValue::Number(5.0));
        assert_eq!(eval_source("(1 + 2) * 3").unwrap(), EmbedValue::Number(9.0));
    }

    #[test]
    fn eval_string_literal() {
        let v = eval_source("'hi'").unwrap();
        assert_eq!(v, EmbedValue::String("hi".into()));
    }

    #[test]
    fn eval_typeof_undefined() {
        let v = eval_source("typeof undefined").unwrap();
        assert_eq!(v, EmbedValue::String("undefined".into()));
    }

    #[test]
    fn eval_typeof_number() {
        let v = eval_source("typeof 1").unwrap();
        assert_eq!(v, EmbedValue::String("number".into()));
    }

    #[test]
    fn eval_direct_eval_expression_cases() {
        // Mirrors tests/conformance/fixtures/es/eval/direct_eval.drac string args.
        assert_eq!(eval_source("1 + 2").unwrap(), EmbedValue::Number(3.0));
        assert_eq!(
            eval_source("typeof undefined").unwrap(),
            EmbedValue::String("undefined".into())
        );
        assert_eq!(eval_source("3 * 4").unwrap(), EmbedValue::Number(12.0));
        assert_eq!(
            eval_source("'hi'").unwrap(),
            EmbedValue::String("hi".into())
        );
    }

    #[test]
    fn eval_completion_value_is_last_expr() {
        let v = eval_source("1 + 2; 3 * 4").unwrap();
        assert_eq!(v, EmbedValue::Number(12.0));
    }

    #[test]
    fn eval_string_concat() {
        let v = eval_source("'a' + 'b'").unwrap();
        assert_eq!(v, EmbedValue::String("ab".into()));
    }

    #[test]
    fn eval_rejects_unsupported_construct() {
        let err = eval_source("function f() {}").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("embed eval does not support") || msg.contains("function"),
            "msg={msg}"
        );
    }
}
