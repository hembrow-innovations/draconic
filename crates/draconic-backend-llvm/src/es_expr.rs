//! N08.01 + N08.02.01–N08.02.09: emit native observations for ES expression Programs,
//! `if`/`else`, `while`, `do`/`while`, `for`, `for-in`/`for-of` (strings), `break`/`continue`
//! (incl. labeled), `switch`, labeled statements, and `const` declarations
//! (E01.01 arithmetic, E01.02 comparison, E01.03 logical, E01.04.01 bitwise, E01.04.02 `**`,
//! E01.04.03 conditional `?:`, E01.04.04 simple `=` assignment, E01.04.05 prefix/postfix `++`/`--`,
//! E01.04.06 comma `,`, E01.04.07 unary keywords `typeof`/`void`/`delete`,
//! E01.04.08 compound assignment `+=` `-=` `*=` `/=` `%=` `**=` `<<=` `>>=` `>>>=` `&=` `^=` `|=`,
//! E02.01 `if` / `else` (incl. block bodies; ToBoolean on number/boolean tests),
//! E02.02 `while` loops (incl. block bodies; ToBoolean on number/boolean tests),
//! E02.03 `do` / `while` loops (incl. block bodies; ToBoolean on number/boolean tests),
//! E02.04 `for` loops (`for (init; test; update)`; `let`/`const` init; omitted clauses; block bodies),
//! E02.05 unlabeled `break` / `continue` in loops,
//! E02.06 `switch` / `case` / `default` (number discriminant; fall-through; unlabeled `break`),
//! E02.07 labeled statements + labeled `break` / `continue`,
//! E02.08 `for-in` / `for-of` over strings (`let`/`const`/assign binding; string concat `+`),
//! E02.09 `const` declarations (required init; `for`/`for-of`/`for-in` binding),
//! E07.01 string lit + concat (incl. number ToString) + `.length` + index (N08.07.01),
//! E07.02 untagged template literals (N08.07.02; cooked quasis + ToString interpolations),
//! E07.03 unicode escapes `\x`/`\u`/`\u{}` cooked into strings; `.length` is UTF-16 units (N08.07.03),
//! E07.05 UTF-16 code-unit semantics: index/concat/eq over WTF-8 storage (N08.07.05).
//! E08.01 number literals: decimal/hex/bin/oct/separators/scientific (N08.08.01).
//! E08.02 BigInt integer literals + same-type arithmetic (N08.08.02; i64-range values).
//! E08.03 BigInt comparison & bitwise: `<` `<=` `>` `>=` `==` `!=` `===` `!==` `&` `|` `^` `~` `<<` `>>` (N08.08.03; no `>>>`).
//! E08.04 BigInt exponentiation: `**` (right-associative) and `**=` (same-type BigInt; non-neg exp; N08.08.04).
//! E08.05 Global `Math`: constants (`E`, `PI`, `LN2`, `LOG2E`) + methods (`abs`, `floor`, `ceil`,
//! `round`, `min`, `max`, `pow`, `sqrt`, `sign`) via `.` / `[]` and calls (N08.08.05).
//! E08.06 Global `NaN` / `Infinity` + `Number`: constants + static methods (`isNaN`, `isFinite`,
//! `isInteger`, `isSafeInteger`) via `.` / `[]` and calls (N08.08.06).
//! N08.01.04.09 nullish/logical-assign lives in `es_nullish`.

use std::collections::HashMap;

use crate::emitter::{Emitter as IrEmitter, SlotTy};
use draconic_ast::JsString;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Expr, IrType as Type, LocalId, Module};

mod classify;
use classify::*;

mod emit;
mod emit_number;
mod emit_values;

/// True when this module is a supported ES expression / control-flow subset
/// (E01.* / E02.01–E02.09 / E07.01–E07.05 / E08.01–E08.06 / N08.01.* / N08.02.01–N08.02.09 /
/// N08.07.01–N08.07.05 / N08.08.01–N08.08.06):
/// top-level `let`/`const` declares over JS numbers, BigInts (i64-range), booleans, strings,
/// undefined (`void`), and/or untyped `any` string/number/boolean slots with arithmetic, unary
/// `+`/`-`/`!`/`~`/`typeof`/`void`/`delete`, comparison, equality, logical, bitwise,
/// exponentiation, conditional, simple/compound assignment, prefix/postfix `++`/`--`, comma,
/// grouping, local refs, string concat `+` (incl. number ToString), untagged templates,
/// unicode-escape string lits, UTF-16 index/length, number literals
/// (decimal/hex/bin/oct/separators/scientific), BigInt literals + same-type `+` `-` `*` `/` `%`
/// unary `-`/`~`, comparison/equality, bitwise `&` `|` `^` `<<` `>>` (no `>>>`),
/// BigInt `**` / `**=` (non-negative exponents; values fit i64),
/// global `Math` constants/methods (`. ` / `[]` + call; `typeof Math` → `"object"`),
/// global `NaN`/`Infinity`/`Number` constants/static methods (`. ` / `[]` + call;
/// `typeof Number` → `"function"`; `typeof NaN`/`Infinity` → `"number"`),
/// `if`/`else`, `while`, `do`/`while`, `for` (incl. `let`/`const` init; block or
/// expression bodies), `for-in`/`for-of` over strings (`let`/`const`/assign left), `break`/`continue`
/// (unlabeled or labeled), labeled statements (incl. labeled blocks), and `switch`/`case`/`default`
/// (number discriminant; fall-through; unlabeled `break`).
/// Expression statements may be assigns or updates.
pub(crate) fn is_es_expr_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_expr(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_expr module"))?;
    emit_es_expr_with(module, &info)
}

pub(crate) fn walk_es_expr(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_expr_module(module) {
        return None;
    }
    Some(emit_es_expr(module))
}

pub(crate) fn emit_es_expr_walk(module: &Module) -> Result<String, Diagnostic> {
    if let Some(result) = try_folded_walks(module) {
        return result;
    }
    let info = classify_body(module).ok_or_else(|| diag("unsupported IR node"))?;
    emit_es_expr_with(module, &info)
}

fn try_folded_walks(module: &Module) -> Option<Result<String, Diagnostic>> {
    crate::host_process::walk_host_process(module)
        .or_else(|| crate::host_os::walk_host_os(module))
        .or_else(|| crate::host_process_async::walk_host_process_async(module))
        .or_else(|| crate::host_subprocess::walk_host_subprocess(module))
        .or_else(|| crate::host_signals::walk_host_signals(module))
        .or_else(|| crate::host_stdio::walk_host_stdio(module))
        .or_else(|| crate::host_path::walk_host_path(module))
        .or_else(|| crate::host_docs::walk_host_docs(module))
        .or_else(|| crate::host_tcp_async::walk_host_tcp_async(module))
        .or_else(|| crate::host_udp::walk_host_udp(module))
        .or_else(|| crate::host_dns::walk_host_dns(module))
        .or_else(|| crate::host_ws_e2e::walk_host_ws_e2e(module))
        .or_else(|| crate::host_http2::walk_host_http2(module))
        .or_else(|| crate::host_http_server::walk_host_http_server(module))
        .or_else(|| crate::host_ws::walk_host_ws(module))
        .or_else(|| crate::host_http::walk_host_http(module))
        .or_else(|| crate::host_tcp::walk_host_tcp(module))
        .or_else(|| crate::host_time::walk_host_time(module))
        .or_else(|| crate::host_timers::walk_host_timers(module))
        .or_else(|| crate::host_atomics::walk_host_atomics(module))
        .or_else(|| crate::host_worker_channels::walk_host_worker_channels(module))
        .or_else(|| crate::host_once::walk_host_once(module))
        .or_else(|| crate::host_cancel::walk_host_cancel(module))
        .or_else(|| crate::host_workers::walk_host_workers(module))
        .or_else(|| crate::host_channels::walk_host_channels(module))
        .or_else(|| crate::es_promise::walk_es_promise(module))
        .or_else(|| crate::es_eval::walk_es_eval(module))
        .or_else(|| crate::es_private_in::walk_es_private_in(module))
        .or_else(|| crate::es_proxies::walk_es_proxies(module))
        .or_else(|| crate::es_testing::walk_es_testing(module))
        .or_else(|| crate::es_logging::walk_es_logging(module))
        .or_else(|| crate::es_mime::walk_es_mime(module))
        .or_else(|| crate::es_collections::walk_es_collections(module))
        .or_else(|| crate::es_encoding::walk_es_encoding(module))
        .or_else(|| crate::es_new_target::walk_es_new_target(module))
        .or_else(|| crate::es_private_accessors::walk_es_private_accessors(module))
        .or_else(|| crate::es_instanceof::walk_es_instanceof(module))
        .or_else(|| crate::es_generators::walk_es_generators(module))
        .or_else(|| crate::es_modules::walk_es_modules(module))
        .or_else(|| crate::es_exceptions::walk_es_exceptions(module))
        .or_else(|| crate::es_legacy::walk_es_legacy(module))
        .or_else(|| crate::es_optional_chain::walk_es_optional_chain(module))
        .or_else(|| crate::es_static_blocks::walk_es_static_blocks(module))
        .or_else(|| crate::es_nullish::walk_es_nullish(module))
        .or_else(|| crate::es_to_primitive::walk_es_to_primitive(module))
        .or_else(|| crate::es_coercion::walk_es_coercion(module))
        .or_else(|| crate::es_values::walk_es_values(module))
        .or_else(|| crate::es_call_spread::walk_es_call_spread(module))
        .or_else(|| crate::es_tagged_template::walk_es_tagged_template(module))
        .or_else(|| crate::es_param_dstr::walk_es_param_dstr(module))
        .or_else(|| crate::es_var_for::walk_es_var_for(module))
        .or_else(|| crate::es_class_expr_name::walk_es_class_expr_name(module))
        .or_else(|| crate::es_static_private_methods::walk_es_static_private_methods(module))
        .or_else(|| crate::es_object_destructure::walk_es_object_destructure(module))
        .or_else(|| crate::es_destructure_defaults::walk_es_destructure_defaults(module))
        .or_else(|| crate::es_builtins::walk_es_builtins(module))
        .or_else(|| crate::es_objects::walk_es_objects(module))
        .or_else(|| crate::es_arrays::walk_es_arrays(module))
        .or_else(|| walk_es_expr(module))
        .or_else(|| crate::host_fs::walk_host_fs(module))
        .or_else(|| crate::es_classes::walk_es_classes(module))
        .or_else(|| crate::es_functions::walk_es_functions(module))
}

fn emit_es_expr_with(module: &Module, info: &ModuleInfo) -> Result<String, Diagnostic> {
    let mut em = Emitter::new(module, ExprState::default());
    em.emit_module(&info.alloc_locals, &info.user_locals)?;
    Ok(em.finish())
}
/// Top-level user locals in declaration order (observation/print order).
/// `alloc_locals` also includes `for (let …)` bindings (not printed).
pub(super) struct ModuleInfo {
    user_locals: Vec<(LocalId, SlotTy)>,
    alloc_locals: Vec<(LocalId, SlotTy)>,
}

/// Targets for `break` / `continue` (unlabeled = innermost; labeled = matching `names`).
/// `continue_label` is `None` for `switch` and labeled non-iteration statements.
pub(super) struct CtrlFrame {
    names: Vec<String>,
    break_label: String,
    continue_label: Option<String>,
}

/// Length-aware string SSA value (N08.07.01; supports embedded NUL).
pub(super) struct StrVal {
    data: String,
    len: String,
}

#[derive(Default)]
pub(super) struct ExprState {
    allocas: HashMap<LocalId, (String, SlotTy)>,
    string_lens: HashMap<LocalId, String>,
    str_globals: HashMap<Vec<u8>, String>,
    ctrls: Vec<CtrlFrame>,
    pending_names: Vec<String>,
}

pub(super) type Emitter<'a> = IrEmitter<'a, ExprState>;

pub(super) fn expr_ty_is_number(expr: &Expr) -> bool {
    matches!(expr.ty(), Type::Number)
}

pub(super) fn expr_ty_is_bigint(expr: &Expr) -> bool {
    matches!(expr.ty(), Type::BigInt)
}

pub(super) fn expr_ty_is_string(expr: &Expr) -> bool {
    matches!(expr.ty(), Type::String)
}

/// Format a Math constant name as an LLVM `double` (N08.08.05 / E08.05).
pub(super) fn format_math_const(name: &str) -> Result<String, Diagnostic> {
    let f = match name {
        "E" => std::f64::consts::E,
        "PI" => std::f64::consts::PI,
        "LN2" => std::f64::consts::LN_2,
        "LOG2E" => std::f64::consts::LOG2_E,
        _ => return Err(diag(format!("internal: unknown Math const {name}"))),
    };
    Ok(format!("{f:.17e}"))
}

/// Format a Number/NaN/Infinity constant as an LLVM `double` (N08.08.06 / E08.06).
pub(super) fn format_number_ctor_const(name: &str) -> Result<String, Diagnostic> {
    match name {
        // Quiet NaN / ±Infinity as bit patterns (decimal `NaN` is not valid LLVM double text).
        "NaN" => Ok("0x7FF8000000000000".into()),
        "POSITIVE_INFINITY" | "Infinity" => Ok("0x7FF0000000000000".into()),
        "NEGATIVE_INFINITY" => Ok("0xFFF0000000000000".into()),
        "MAX_VALUE" => Ok(format!("{:.17e}", f64::MAX)),
        "MIN_VALUE" => Ok(format!("{:.17e}", f64::MIN_POSITIVE)),
        "EPSILON" => Ok(format!("{:.17e}", f64::EPSILON)),
        "MAX_SAFE_INTEGER" => Ok("9.0071992547409910e+15".into()),
        "MIN_SAFE_INTEGER" => Ok("-9.0071992547409910e+15".into()),
        _ => Err(diag(format!("internal: unknown Number const {name}"))),
    }
}

/// Format a JS number literal as an LLVM `double` constant (decimal/hex/bin/oct,
/// numeric separators, round-trip safe). N08.08.01 / E08.01.
pub(super) fn format_number_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f = parse_js_number_literal(&cleaned)
        .ok_or_else(|| diag(format!("invalid number literal {raw}")))?;
    Ok(format!("{f:.17e}"))
}

/// Format a JS BigInt literal as an LLVM `i64` constant (decimal/hex/bin/oct,
/// numeric separators, trailing `n`). N08.08.02 / E08.02.
pub(super) fn format_bigint_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let n = parse_js_bigint_literal(&cleaned)
        .ok_or_else(|| diag(format!("invalid BigInt literal {raw}")))?;
    Ok(n.to_string())
}

/// Parse ECMAScript BigInt literal text (no `_` separators; optional trailing `n`) to `i64`.
pub(super) fn parse_js_bigint_literal(s: &str) -> Option<i64> {
    let s = s.strip_suffix('n').unwrap_or(s);
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok();
    }
    if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        return i64::from_str_radix(bin, 2).ok();
    }
    if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        return i64::from_str_radix(oct, 8).ok();
    }
    s.parse().ok()
}

/// Parse ECMAScript numeric literal text (no `_` separators) to `f64`.
pub(super) fn parse_js_number_literal(s: &str) -> Option<f64> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16).ok().map(|n| n as f64);
    }
    if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        return u64::from_str_radix(bin, 2).ok().map(|n| n as f64);
    }
    if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        return u64::from_str_radix(oct, 8).ok().map(|n| n as f64);
    }
    s.parse().ok()
}

/// Encode JS UTF-16 code units as WTF-8 (UTF-8 + unpaired surrogates as 3-byte sequences).
pub(super) fn jsstring_to_wtf8(value: &JsString) -> Vec<u8> {
    let units = value.units();
    let mut out = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        if (0xD800..=0xDBFF).contains(&u) && i + 1 < units.len() {
            let v = units[i + 1];
            if (0xDC00..=0xDFFF).contains(&v) {
                let cp = 0x10000 + (((u as u32) - 0xD800) << 10) + ((v as u32) - 0xDC00);
                out.push(0xF0 | (cp >> 18) as u8);
                out.push(0x80 | ((cp >> 12) & 0x3F) as u8);
                out.push(0x80 | ((cp >> 6) & 0x3F) as u8);
                out.push(0x80 | (cp & 0x3F) as u8);
                i += 2;
                continue;
            }
        }
        if u < 0x80 {
            out.push(u as u8);
        } else if u < 0x800 {
            out.push(0xC0 | (u >> 6) as u8);
            out.push(0x80 | (u & 0x3F) as u8);
        } else {
            out.push(0xE0 | (u >> 12) as u8);
            out.push(0x80 | ((u >> 6) & 0x3F) as u8);
            out.push(0x80 | (u & 0x3F) as u8);
        }
        i += 1;
    }
    out
}

pub(super) fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
