//! N06.03–N06.11: lower Promise + async/await (incl. async arrows) to Runtime ABI.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Param, Pattern,
    Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, ES_PROMISE_DECLARES, GC_INIT,
    JOB_DRAIN, OBJECT_GET, PRINT_I64, PRINT_STR, PROMISE_ALL, PROMISE_ALL_SETTLED, PROMISE_ANY,
    PROMISE_AWAIT, PROMISE_CONSTRUCT, PROMISE_FINALLY, PROMISE_NEW, PROMISE_RACE, PROMISE_REJECT,
    PROMISE_RESOLVE, PROMISE_THEN,
};

#[path = "es_promise_emit.rs"]
mod emit;
#[path = "es_promise_async.rs"]
mod async_fn;
#[path = "es_promise_reaction.rs"]
mod reaction;

/// True when this module is the supported Promise/async subset (E12.01–E12.09 / N06.03–N06.11).
pub(crate) fn is_es_promise_module(module: &Module) -> bool {
    match try_classify(module) {
        Ok(info) => info.uses_promise,
        Err(_) => false,
    }
}

pub(crate) fn emit_es_promise(module: &Module) -> Result<String, Diagnostic> {
    let info = try_classify(module).map_err(diag)?;
    if !info.uses_promise {
        return Err(diag("internal: not a Promise module"));
    }
    let mut em = Emitter::new(module, info);
    em.emit_module()?;
    Ok(em.finish())
}

pub(crate) fn walk_es_promise(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_promise_module(module) {
        return None;
    }
    Some(emit_es_promise(module))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotKind {
    Number,
    String,
    Object,
}

struct ModuleInfo {
    uses_promise: bool,
    promise_id: Option<LocalId>,
    /// Top-level user locals to allocate / print (source order).
    user_locals: Vec<(LocalId, SlotKind)>,
}

fn try_classify(module: &Module) -> Result<ModuleInfo, String> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let promise_id = module
        .locals
        .iter()
        .find(|l| l.name == "Promise")
        .map(|l| l.id);

    let mut user_ids = HashSet::new();
    collect_top_level_decl_ids(&module.body, &mut user_ids);

    let mut user_locals = Vec::new();
    let mut seen = HashSet::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            if !seen.insert(*local) {
                continue;
            }
            if !user_ids.contains(local) {
                continue;
            }
            let Some(loc) = by_id.get(local) else {
                continue;
            };
            // Skip nested function param / arguments bindings that appear as declares.
            if loc.name == "arguments" {
                continue;
            }
            let kind = match loc.ty {
                Type::Number => SlotKind::Number,
                Type::String => SlotKind::String,
                Type::Object | Type::Any | Type::Function => SlotKind::Object,
                _ => return Err(format!("unsupported local type for `{}`", loc.name)),
            };
            user_locals.push((*local, kind));
        }
    }

    // Async function declarations bind a function local without a `Declare`.
    for stmt in &module.body {
        if let Stmt::Function {
            local, is_async, ..
        } = stmt
        {
            if !*is_async {
                return Err("only async function declarations supported in Promise path".into());
            }
            if seen.insert(*local) {
                user_locals.push((*local, SlotKind::Object));
            }
        }
    }

    let mut uses_promise = false;
    for stmt in &module.body {
        check_stmt(stmt, promise_id, &mut uses_promise)?;
    }

    Ok(ModuleInfo {
        uses_promise,
        promise_id,
        user_locals,
    })
}

fn check_simple_params(params: &[Param]) -> Result<(), String> {
    for p in params {
        if p.rest || p.default.is_some() {
            return Err("rest/default params not supported".into());
        }
        if !matches!(p.pattern, Pattern::Local(_)) {
            return Err("only simple params supported".into());
        }
    }
    Ok(())
}

fn collect_top_level_decl_ids(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Declare { local, .. } => {
                out.insert(*local);
            }
            Stmt::Expr { .. } => {}
            Stmt::Block { body } => collect_top_level_decl_ids(body, out),
            _ => {}
        }
    }
}

fn check_stmt(stmt: &Stmt, promise_id: Option<LocalId>, uses: &mut bool) -> Result<(), String> {
    match stmt {
        Stmt::Declare { init, .. } => {
            if let Some(e) = init {
                check_expr(e, promise_id, uses)?;
            }
            Ok(())
        }
        Stmt::Expr { expr } => check_expr(expr, promise_id, uses),
        Stmt::Return { value } => {
            if let Some(e) = value {
                check_expr(e, promise_id, uses)?;
            }
            Ok(())
        }
        Stmt::Throw { value } => check_expr(value, promise_id, uses),
        Stmt::Block { body } => {
            for s in body {
                check_stmt(s, promise_id, uses)?;
            }
            Ok(())
        }
        Stmt::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if !*is_async || *is_generator {
                return Err("only async function declarations supported in Promise path".into());
            }
            check_simple_params(params)?;
            *uses = true;
            for s in body {
                check_stmt(s, promise_id, uses)?;
            }
            Ok(())
        }
        other => Err(format!("unsupported statement in Promise path: {other:?}")),
    }
}

fn check_expr(expr: &Expr, promise_id: Option<LocalId>, uses: &mut bool) -> Result<(), String> {
    match expr {
        Expr::Local { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. } => Ok(()),
        Expr::Unary { op, arg, .. } => {
            match op {
                UnaryOp::TypeOf | UnaryOp::Minus | UnaryOp::Plus | UnaryOp::Not => {}
                UnaryOp::Await => {
                    *uses = true;
                }
                _ => return Err(format!("unsupported unary {op:?}")),
            }
            check_expr(arg, promise_id, uses)
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            match op {
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {}
                _ => return Err(format!("unsupported binary {op:?}")),
            }
            check_expr(left, promise_id, uses)?;
            check_expr(right, promise_id, uses)
        }
        Expr::Assign { target, value, .. } => {
            match target {
                AssignTarget::Local(_) => {}
                _ => return Err("only local assignment supported in Promise path".into()),
            }
            check_expr(value, promise_id, uses)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return Err("optional call not supported".into());
            }
            check_expr(callee, promise_id, uses)?;
            for a in args {
                match a {
                    Arg::Expr(e) => check_expr(e, promise_id, uses)?,
                    Arg::Spread(_) => return Err("spread args not supported".into()),
                }
            }
            Ok(())
        }
        Expr::New { callee, args, .. } => {
            if let Expr::Local { id, .. } = callee.as_ref() {
                if Some(*id) == promise_id {
                    *uses = true;
                } else {
                    return Err("only `new Promise` supported".into());
                }
            } else {
                return Err("only `new Promise` supported".into());
            }
            for a in args {
                match a {
                    Arg::Expr(e) => check_expr(e, promise_id, uses)?,
                    Arg::Spread(_) => return Err("spread args not supported".into()),
                }
            }
            Ok(())
        }
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } => {
            if *optional {
                return Err("optional member not supported in Promise path".into());
            }
            check_expr(object, promise_id, uses)?;
            if *computed {
                check_expr(property, promise_id, uses)?;
                return Ok(());
            }
            let Expr::String { value, .. } = property.as_ref() else {
                return Err("only string property keys supported".into());
            };
            let prop = value.to_string_lossy();
            match prop.as_ref() {
                "then" | "catch" | "finally" => {
                    *uses = true;
                    Ok(())
                }
                "length" | "status" | "value" | "reason" | "name" | "errors" => Ok(()),
                "resolve" | "reject" | "all" | "race" | "allSettled" | "any" => {
                    if let Expr::Local { id, .. } = object.as_ref() {
                        if Some(*id) == promise_id {
                            *uses = true;
                            return Ok(());
                        }
                    }
                    Err(
                        "only Promise.resolve / Promise.reject / Promise.all / Promise.race / Promise.allSettled / Promise.any supported"
                            .into(),
                    )
                }
                _ => Err(format!("unsupported property `{}` in Promise path", prop)),
            }
        }
        Expr::Array { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => check_expr(e, promise_id, uses)?,
                    ArrayElement::Spread(_) => {
                        return Err("spread in array not supported in Promise path".into());
                    }
                    ArrayElement::Elision => {}
                }
            }
            Ok(())
        }
        Expr::Function {
            name,
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if *is_generator {
                return Err("generator functions not supported in Promise path".into());
            }
            if *is_async {
                *uses = true;
            }
            if name.is_some() {
                return Err("named function expressions not supported".into());
            }
            check_simple_params(params)?;
            for s in body {
                check_stmt(s, promise_id, uses)?;
            }
            Ok(())
        }
        other => Err(format!("unsupported expr in Promise path: {other:?}")),
    }
}

struct Emitter<'a> {
    module: &'a Module,
    info: ModuleInfo,
    out: String,
    body: String,
    tmp: u32,
    next_fn: u32,
    /// local id → alloca ptr name (`%lN`)
    allocas: HashMap<LocalId, String>,
    /// string constants: content → global name
    str_globals: HashMap<String, String>,
    /// emitted helper function IR (appended before main)
    helpers: String,
    /// While lowering an executor: param local → (settle_fn_ssa, cap_ssa)
    executor_params: HashMap<LocalId, (String, String)>,
    /// While lowering a reaction: param local → value ssa (ptr)
    reaction_params: HashMap<LocalId, String>,
    /// Capture slots passed as reaction `data` (env of alloca ptrs, or single).
    reaction_captures: Vec<LocalId>,
    /// Known async function locals → LLVM function name (0-arg, returns Promise ptr).
    async_fns: HashMap<LocalId, String>,
}

fn collect_assigned_locals(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Expr { expr } => {
                if let Expr::Assign {
                    target: AssignTarget::Local(id),
                    ..
                } = expr
                {
                    out.insert(*id);
                }
            }
            Stmt::Block { body } => collect_assigned_locals(body, out),
            Stmt::Return { .. } => {}
            _ => {}
        }
    }
}

fn match_await_declare(stmt: &Stmt) -> Option<(LocalId, &Expr)> {
    match stmt {
        Stmt::Declare {
            local,
            init:
                Some(Expr::Unary {
                    op: UnaryOp::Await,
                    arg,
                    ..
                }),
            ..
        } => Some((*local, arg.as_ref())),
        _ => None,
    }
}

fn stmt_contains_await(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init, .. } => init.as_ref().is_some_and(expr_contains_await),
        Stmt::Expr { expr } => expr_contains_await(expr),
        Stmt::Return { value } => value.as_ref().is_some_and(expr_contains_await),
        Stmt::Throw { value } => expr_contains_await(value),
        Stmt::Block { body } => body.iter().any(stmt_contains_await),
        _ => false,
    }
}

fn expr_contains_await(expr: &Expr) -> bool {
    match expr {
        Expr::Unary {
            op: UnaryOp::Await, ..
        } => true,
        Expr::Unary { arg, .. } => expr_contains_await(arg),
        Expr::Binary { left, right, .. } => expr_contains_await(left) || expr_contains_await(right),
        Expr::Assign { value, .. } => expr_contains_await(value),
        Expr::Call { callee, args, .. } => {
            expr_contains_await(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_await(e),
                })
        }
        Expr::New { callee, args, .. } => {
            expr_contains_await(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_await(e),
                })
        }
        Expr::Member {
            object, property, ..
        } => expr_contains_await(object) || expr_contains_await(property),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_await(e),
            ArrayElement::Elision => false,
        }),
        Expr::Function { body, .. } => body.iter().any(stmt_contains_await),
        _ => false,
    }
}

fn format_ptr_args(args: &[String]) -> String {
    let mut out = String::new();
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write!(out, "ptr {a}").ok();
    }
    out
}

fn parse_number(raw: &str) -> Result<i64, Diagnostic> {
    let s = raw.trim();
    if let Ok(n) = s.parse::<i64>() {
        return Ok(n);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Ok(f as i64);
    }
    Err(diag(format!("bad number literal `{raw}`")))
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                write!(out, "\\{b:02X}").ok();
            }
        }
    }
    out
}

fn diag(msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(msg.into(), Span::dummy())
}
