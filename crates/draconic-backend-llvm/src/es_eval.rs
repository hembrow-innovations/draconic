//! N07.02–N07.04: emit native observations for `eval` / `Function` (fold via Embed).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::BinaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_embed::{fold_eval_program, is_eval_fold_module, Observation};
use draconic_ir::{Expr, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, ES_EVAL_DECLARES, GC_INIT, PRINT_BOOL, PRINT_I64, PRINT_STR,
};

/// True when this module is the supported eval/Function subset (E16 / N07.02–N07.04).
pub(crate) fn is_es_eval_module(module: &Module) -> bool {
    is_eval_fold_module(module)
}

pub(crate) fn emit_es_eval(module: &Module) -> Result<String, Diagnostic> {
    let obs = fold_eval_program(module)?;
    if obs.is_empty() {
        return Err(diag("internal: not an eval/Function module"));
    }
    emit_observations(&obs, classify_tag(module))
}

fn classify_tag(module: &Module) -> &'static str {
    if module_has_indirect(module) {
        "N07.04 indirect eval via Embed"
    } else if module_has_function_ctor(module) && module_has_direct_eval(module) {
        "N07.02/N07.03 eval+Function via Embed"
    } else if module_has_function_ctor(module) {
        "N07.03 Function via Embed"
    } else {
        "N07.02 direct eval via Embed"
    }
}

fn module_has_direct_eval(module: &Module) -> bool {
    let eval_id = module.locals.iter().find(|l| l.name == "eval").map(|l| l.id);
    fn walk(stmt: &Stmt, eval_id: Option<LocalId>) -> bool {
        match stmt {
            Stmt::Declare { init: Some(e), .. }
            | Stmt::Expr { expr: e }
            | Stmt::Return { value: Some(e) } => expr_has_direct_eval(e, eval_id),
            Stmt::Function { body, .. } | Stmt::Block { body } => {
                body.iter().any(|s| walk(s, eval_id))
            }
            _ => false,
        }
    }
    fn expr_has_direct_eval(e: &Expr, eval_id: Option<LocalId>) -> bool {
        match e {
            Expr::Call { callee, .. } => {
                if let Expr::Local { id, .. } = callee.as_ref() {
                    if Some(*id) == eval_id {
                        return true;
                    }
                }
                false
            }
            Expr::Unary { arg, .. } => expr_has_direct_eval(arg, eval_id),
            Expr::Binary { left, right, .. } => {
                expr_has_direct_eval(left, eval_id) || expr_has_direct_eval(right, eval_id)
            }
            _ => false,
        }
    }
    module.body.iter().any(|s| walk(s, eval_id))
}

fn module_has_function_ctor(module: &Module) -> bool {
    let function_id = module
        .locals
        .iter()
        .find(|l| l.name == "Function")
        .map(|l| l.id);
    fn walk(stmt: &Stmt, function_id: Option<LocalId>) -> bool {
        match stmt {
            Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } => {
                expr_has_fn_ctor(e, function_id)
            }
            Stmt::Block { body } | Stmt::Function { body, .. } => {
                body.iter().any(|s| walk(s, function_id))
            }
            _ => false,
        }
    }
    fn expr_has_fn_ctor(e: &Expr, function_id: Option<LocalId>) -> bool {
        match e {
            Expr::New { callee, .. } | Expr::Call { callee, .. } => {
                matches!(callee.as_ref(), Expr::Local { id, .. } if Some(*id) == function_id)
            }
            _ => false,
        }
    }
    module.body.iter().any(|s| walk(s, function_id))
}

fn module_has_indirect(module: &Module) -> bool {
    fn walk(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Declare { init: Some(e), .. }
            | Stmt::Expr { expr: e }
            | Stmt::Return { value: Some(e) } => expr_indirect(e),
            Stmt::Function { body, .. } | Stmt::Block { body } => body.iter().any(walk),
            _ => false,
        }
    }
    fn expr_indirect(e: &Expr) -> bool {
        match e {
            Expr::Call { callee, .. } => match callee.as_ref() {
                Expr::Binary {
                    op: BinaryOp::Comma,
                    ..
                } => true,
                Expr::Member {
                    object,
                    property,
                    computed,
                    optional,
                    ..
                } if !*computed && !*optional => {
                    if let (Expr::Local { .. }, Expr::String { value, .. }) =
                        (object.as_ref(), property.as_ref())
                    {
                        value.to_string_lossy() == "eval"
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Expr::Unary { arg, .. } => expr_indirect(arg),
            Expr::Binary { left, right, .. } => expr_indirect(left) || expr_indirect(right),
            _ => false,
        }
    }
    module.body.iter().any(walk)
}

fn emit_observations(obs: &[Observation], tag: &str) -> Result<String, Diagnostic> {
    let mut out = String::new();
    let mut body = String::new();
    let mut str_globals: HashMap<String, String> = HashMap::new();
    let mut tmp = 0u32;

    writeln!(out, "; Draconic LLVM backend ({tag})").ok();
    writeln!(out, "{}", llvm_declares(ES_EVAL_DECLARES)).ok();
    writeln!(out).ok();

    for o in obs {
        match o {
            Observation::Number(n) => {
                if n.fract() != 0.0 || !n.is_finite() || n.abs() >= (i64::MAX as f64) {
                    return Err(diag(format!("number not representable as i64: {n}")));
                }
                let v = *n as i64;
                writeln!(body, "  {}", PRINT_I64.call(&format!("i64 {v}"))).ok();
            }
            Observation::Bool(b) => {
                let v = if *b { 1 } else { 0 };
                writeln!(body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
            }
            Observation::String(s) => {
                emit_print_str(&mut body, &mut str_globals, &mut tmp, s);
            }
            Observation::Function => {
                emit_print_str(&mut body, &mut str_globals, &mut tmp, "function");
            }
        }
    }

    for (content, gname) in &str_globals {
        let n = content.len() + 1;
        let esc = escape_llvm_string(content);
        writeln!(
            out,
            "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
        )
        .ok();
    }
    if !str_globals.is_empty() {
        writeln!(out).ok();
    }

    writeln!(out, "define i32 @main() {{").ok();
    writeln!(out, "entry:").ok();
    writeln!(out, "  {}", GC_INIT.call("")).ok();
    out.push_str(&body);
    writeln!(out, "  ret i32 0").ok();
    writeln!(out, "}}").ok();
    Ok(out)
}

fn emit_print_str(
    body: &mut String,
    str_globals: &mut HashMap<String, String>,
    tmp: &mut u32,
    s: &str,
) {
    let gname = if let Some(g) = str_globals.get(s) {
        g.clone()
    } else {
        let g = format!(".str.{}", str_globals.len());
        str_globals.insert(s.to_string(), g.clone());
        g
    };
    let t = format!("%t{tmp}");
    *tmp += 1;
    let n = s.len() + 1;
    writeln!(
        body,
        "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
    )
    .ok();
    writeln!(body, "  {}", PRINT_STR.call(&format!("ptr {t}"))).ok();
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) && c != b'\\' => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
