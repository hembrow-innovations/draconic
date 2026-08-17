//! Embed: compiler-in-runtime for `eval` / `Function` (ROADMAP N07).
//!
//! N07.01: compile simple expression source strings through Frontend → IR and
//! evaluate them with a minimal IR interpreter (completion value of the script).
//! N07.03: evaluate `Function` bodies (`return expr`) with bound parameter values.
//! N07.04: evaluate with injected name bindings (direct lexical / indirect global).
//! Outer Program fold (N07.02–N07.04 native): [`fold_eval_program`].

mod fold;

pub use fold::{fold_eval_program, is_eval_fold_module, Observation};

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_frontend::compile_source;
use draconic_ir::{Expr, LocalId, Module, Stmt};

/// Maximum UTF-8 byte length of source accepted by embed `eval` / `Function` (R01.01).
///
/// Checked before compile so oversize input fails closed without parsing.
pub const MAX_EVAL_SOURCE_BYTES: usize = 1_048_576; // 1 MiB

/// Reject `source` when longer than [`MAX_EVAL_SOURCE_BYTES`].
fn check_eval_source_size(source: &str) -> Result<(), Diagnostic> {
    let len = source.len();
    if len > MAX_EVAL_SOURCE_BYTES {
        return Err(diag(format!(
            "embed eval: source exceeds maximum source size ({len} > {MAX_EVAL_SOURCE_BYTES} bytes)"
        )));
    }
    Ok(())
}

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
    check_eval_source_size(source)?;
    let module = compile_source(source)?;
    interpret_module(&module)
}

/// Like [`eval_source`], but prepends `let` bindings for free names (N07.04).
///
/// Used for direct eval (caller lexical names) and indirect eval (global object
/// properties). Each binding name must appear at most once (caller merges
/// shadowing: lexical over global).
pub fn eval_source_with_bindings(
    source: &str,
    bindings: &[(String, EmbedValue)],
) -> Result<EmbedValue, Diagnostic> {
    if bindings.is_empty() {
        return eval_source(source);
    }
    let mut seen = std::collections::HashSet::new();
    for (name, _) in bindings {
        if !seen.insert(name.as_str()) {
            return Err(diag(format!(
                "embed eval: duplicate binding name {name:?}"
            )));
        }
    }
    let mut script = String::new();
    for (name, val) in bindings {
        validate_param_name(name)?;
        write_let_binding(&mut script, name, val)?;
    }
    script.push_str(source);
    eval_source(&script)
}

/// Evaluate a `Function` body with bound parameters (N07.03).
///
/// `params` are simple identifier names; `body` is the function body source
/// (typically a single `return <expr>;`). `args` are bound positionally; missing
/// args become `undefined`. Extra args are ignored.
pub fn eval_function_call(
    params: &[&str],
    body: &str,
    args: &[EmbedValue],
) -> Result<EmbedValue, Diagnostic> {
    // Body is Function source text; reject before strip/compile (R01.01).
    check_eval_source_size(body)?;
    let expr_src = function_body_completion_expr(body)?;
    let mut bindings = Vec::with_capacity(params.len());
    for (i, name) in params.iter().enumerate() {
        validate_param_name(name)?;
        let val = args.get(i).cloned().unwrap_or(EmbedValue::Undefined);
        bindings.push(((*name).to_string(), val));
    }
    eval_source_with_bindings(&expr_src, &bindings)
}

fn function_body_completion_expr(body: &str) -> Result<String, Diagnostic> {
    let t = body.trim();
    // Single-statement `return <expr>;` (N07.03 fixture subset).
    let rest = if let Some(r) = t.strip_prefix("return") {
        let r = r.trim_start();
        if r.is_empty() {
            return Ok("undefined".into());
        }
        // `return` must be followed by expr or end; require boundary.
        r
    } else {
        // Bare expression body (not used by fixtures, but harmless).
        t
    };
    let rest = rest.trim_end_matches(';').trim();
    if rest.is_empty() {
        return Ok("undefined".into());
    }
    Ok(rest.to_string())
}

fn validate_param_name(name: &str) -> Result<(), Diagnostic> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(diag("embed Function: empty parameter name"));
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return Err(diag(format!(
            "embed Function: invalid parameter name {name:?}"
        )));
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '$') {
            return Err(diag(format!(
                "embed Function: invalid parameter name {name:?}"
            )));
        }
    }
    Ok(())
}

fn write_let_binding(out: &mut String, name: &str, val: &EmbedValue) -> Result<(), Diagnostic> {
    match val {
        EmbedValue::Undefined => {
            let _ = write!(out, "let {name} = undefined; ");
        }
        EmbedValue::Null => {
            let _ = write!(out, "let {name} = null; ");
        }
        EmbedValue::Boolean(true) => {
            let _ = write!(out, "let {name} = true; ");
        }
        EmbedValue::Boolean(false) => {
            let _ = write!(out, "let {name} = false; ");
        }
        EmbedValue::Number(n) => {
            if n.is_nan() {
                let _ = write!(out, "let {name} = NaN; ");
            } else if *n == f64::INFINITY {
                let _ = write!(out, "let {name} = Infinity; ");
            } else if *n == f64::NEG_INFINITY {
                let _ = write!(out, "let {name} = -Infinity; ");
            } else {
                let _ = write!(out, "let {name} = {n}; ");
            }
        }
        EmbedValue::String(s) => {
            let _ = write!(out, "let {name} = {}; ", js_string_literal(s));
        }
    }
    Ok(())
}

fn js_string_literal(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
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

    #[test]
    fn eval_function_call_return_add() {
        let v = eval_function_call(
            &["a", "b"],
            "return a + b",
            &[EmbedValue::Number(1.0), EmbedValue::Number(2.0)],
        )
        .unwrap();
        assert_eq!(v, EmbedValue::Number(3.0));
    }

    #[test]
    fn eval_function_call_return_mul() {
        let v = eval_function_call(&["x"], "return x * 2", &[EmbedValue::Number(3.0)]).unwrap();
        assert_eq!(v, EmbedValue::Number(6.0));
    }

    #[test]
    fn eval_function_call_return_constant() {
        let v = eval_function_call(&[], "return 7", &[]).unwrap();
        assert_eq!(v, EmbedValue::Number(7.0));
    }

    #[test]
    fn eval_function_call_missing_arg_is_undefined() {
        let v = eval_function_call(&["a"], "return typeof a", &[]).unwrap();
        assert_eq!(v, EmbedValue::String("undefined".into()));
    }

    #[test]
    fn eval_source_with_bindings_resolves_free_ident() {
        let v = eval_source_with_bindings(
            "gx",
            &[("gx".into(), EmbedValue::Number(200.0))],
        )
        .unwrap();
        assert_eq!(v, EmbedValue::Number(200.0));
    }

    #[test]
    fn eval_source_with_bindings_global_style() {
        let v = eval_source_with_bindings(
            "gx",
            &[("gx".into(), EmbedValue::Number(100.0))],
        )
        .unwrap();
        assert_eq!(v, EmbedValue::Number(100.0));
    }

    #[test]
    fn eval_source_accepts_source_at_max_size() {
        // One-byte expression under the cap; pad with spaces after a valid expr.
        let mut src = String::from("1");
        src.push_str(&" ".repeat(MAX_EVAL_SOURCE_BYTES - src.len()));
        assert_eq!(src.len(), MAX_EVAL_SOURCE_BYTES);
        let v = eval_source(&src).unwrap();
        assert_eq!(v, EmbedValue::Number(1.0));
    }

    #[test]
    fn eval_source_rejects_oversize_source() {
        let src = "1".to_string() + &" ".repeat(MAX_EVAL_SOURCE_BYTES);
        assert_eq!(src.len(), MAX_EVAL_SOURCE_BYTES + 1);
        let err = eval_source(&src).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("maximum source size") || msg.contains("exceeds"),
            "msg={msg}"
        );
        assert!(
            msg.contains(&MAX_EVAL_SOURCE_BYTES.to_string()),
            "msg should mention limit; msg={msg}"
        );
    }

    #[test]
    fn eval_source_with_bindings_rejects_when_combined_script_oversize() {
        // Source alone is small; bindings + source exceed the cap.
        let pad = " ".repeat(MAX_EVAL_SOURCE_BYTES);
        let err = eval_source_with_bindings(
            &pad,
            &[("gx".into(), EmbedValue::Number(1.0))],
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("maximum source size") || msg.contains("exceeds"),
            "msg={msg}"
        );
    }

    #[test]
    fn eval_function_call_rejects_oversize_body() {
        let body = format!("return {}", "1".to_string() + &" ".repeat(MAX_EVAL_SOURCE_BYTES));
        let err = eval_function_call(&[], &body, &[]).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("maximum source size") || msg.contains("exceeds"),
            "msg={msg}"
        );
    }
}
