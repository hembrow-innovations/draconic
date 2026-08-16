//! N08.03.01–N08.03.05: native observations for ES function declarations,
//! expressions, and arrows (simple ident params) — E03.01–E03.05 /
//! `es/functions/decl_return_call`, `params_call`, `nested_capture`,
//! `function_expr`, `arrow`.
//!
//! Nested/non-escaping decls use extra by-value capture params. Function
//! expressions and arrows are first-class as fn-id doubles; returned closures
//! stash captures in a small return buffer for immediate call (`make(10)(7)`).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, IrType as Type, Local, LocalId, Module, Param, Pattern, Stmt};
use draconic_runtime::abi::{llvm_declares, PRINT_F64};

const MAX_CAPS: usize = 8;

pub(crate) fn is_es_functions_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_functions(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_functions module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone)]
struct FnInfo {
    /// Stable index → LLVM name `d_fn_{idx}`.
    idx: usize,
    params: Vec<LocalId>,
    captures: Vec<LocalId>,
    body: Vec<Stmt>,
    /// Named function expression recursive binding.
    name_local: Option<LocalId>,
}

struct ModuleInfo {
    functions: Vec<FnInfo>,
    /// Locals statically bound to a function index (decl / expr assign / name).
    fn_binding: HashMap<LocalId, usize>,
    /// Top-level number/any user locals to print (declare order).
    user_locals: Vec<LocalId>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut fn_binding = HashMap::new();
    let mut user_locals = Vec::new();

    // Collect every function (decl + expr) first so arities are known.
    collect_all_functions(&module.body, &by_id, &mut functions, &mut fn_binding)?;

    let mut fn_arities: HashMap<LocalId, usize> = HashMap::new();
    for (loc, idx) in &fn_binding {
        fn_arities.insert(*loc, functions[*idx].params.len());
    }
    // Also map by internal name bindings.
    for f in &functions {
        if let Some(n) = f.name_local {
            fn_arities.insert(n, f.params.len());
        }
    }

    for f in &functions {
        if !fn_body_ok(&f.body, &by_id, &fn_arities, &functions, &fn_binding) {
            return None;
        }
    }

    let mut has_fn = !functions.is_empty();
    for stmt in &module.body {
        match stmt {
            Stmt::Function { .. } => {
                has_fn = true;
            }
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                match loc.ty {
                    Type::Number | Type::Any => {
                        let init = init.as_ref()?;
                        if matches!(init, Expr::Function { .. }) {
                            // function value in any/number slot — still ok if bound
                            if !fn_binding.contains_key(local) {
                                return None;
                            }
                            continue;
                        }
                        if !number_expr_ok(init, &by_id, &fn_arities, &functions, &fn_binding) {
                            return None;
                        }
                        user_locals.push(*local);
                    }
                    Type::Function => {
                        let init = init.as_ref()?;
                        if !matches!(init, Expr::Function { .. }) {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
            _ => return None,
        }
    }

    if !has_fn || user_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        functions,
        fn_binding,
        user_locals,
    })
}

fn collect_all_functions(
    stmts: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    out: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
) -> Option<()> {
    for stmt in stmts {
        match stmt {
            Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
            } => {
                if *is_async || *is_generator {
                    return None;
                }
                let param_ids = simple_param_locals(params, by_id)?;
                // Nested first.
                collect_all_functions(body, by_id, out, fn_binding)?;
                collect_exprs_in_body(body, by_id, out, fn_binding)?;
                let idx = push_fn(None, param_ids, body, by_id, out)?;
                fn_binding.insert(*local, idx);
            }
            Stmt::Declare { local, init, .. } => {
                if let Some(e) = init {
                    collect_expr_fns(e, by_id, out, fn_binding)?;
                    if let Expr::Function { name, params, .. } = e {
                        if let Some(idx) = find_fn_idx_by_param_patterns(params, out) {
                            fn_binding.insert(*local, idx);
                            if let Some(n) = name {
                                fn_binding.insert(*n, idx);
                            }
                        }
                    }
                }
            }
            Stmt::Block { body } => collect_all_functions(body, by_id, out, fn_binding)?,
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                collect_all_functions(std::slice::from_ref(consequent), by_id, out, fn_binding)?;
                if let Some(a) = alternate {
                    collect_all_functions(std::slice::from_ref(a), by_id, out, fn_binding)?;
                }
            }
            Stmt::Return { value: Some(v) } => {
                collect_expr_fns(v, by_id, out, fn_binding)?;
            }
            _ => {}
        }
    }
    Some(())
}

fn collect_exprs_in_body(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    out: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
) -> Option<()> {
    for stmt in body {
        match stmt {
            Stmt::Return { value: Some(v) } => collect_expr_fns(v, by_id, out, fn_binding)?,
            Stmt::Declare { init: Some(e), .. } => collect_expr_fns(e, by_id, out, fn_binding)?,
            Stmt::Block { body } => collect_exprs_in_body(body, by_id, out, fn_binding)?,
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                collect_expr_fns(test, by_id, out, fn_binding)?;
                collect_all_functions(std::slice::from_ref(consequent), by_id, out, fn_binding)?;
                if let Some(a) = alternate {
                    collect_all_functions(std::slice::from_ref(a), by_id, out, fn_binding)?;
                }
            }
            Stmt::Function { .. } => {}
            _ => {}
        }
    }
    Some(())
}

fn collect_expr_fns(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    out: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
) -> Option<()> {
    match expr {
        Expr::Function {
            name,
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if *is_async || *is_generator {
                return None;
            }
            let param_ids = simple_param_locals(params, by_id)?;
            collect_all_functions(body, by_id, out, fn_binding)?;
            collect_exprs_in_body(body, by_id, out, fn_binding)?;
            let idx = push_fn(*name, param_ids, body, by_id, out)?;
            if let Some(n) = name {
                fn_binding.insert(*n, idx);
            }
            Some(())
        }
        Expr::Unary { arg, .. } => collect_expr_fns(arg, by_id, out, fn_binding),
        Expr::Binary { left, right, .. } => {
            collect_expr_fns(left, by_id, out, fn_binding)?;
            collect_expr_fns(right, by_id, out, fn_binding)
        }
        Expr::Call { callee, args, .. } => {
            collect_expr_fns(callee, by_id, out, fn_binding)?;
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_expr_fns(e, by_id, out, fn_binding)?;
                }
            }
            Some(())
        }
        _ => Some(()),
    }
}

/// Match a FunctionExpr to its `FnInfo` by param local ids (unique per lower).
fn find_fn_idx_by_param_patterns(params: &[Param], out: &[FnInfo]) -> Option<usize> {
    let ids: Vec<LocalId> = params
        .iter()
        .filter_map(|p| match &p.pattern {
            Pattern::Local(id) => Some(*id),
            _ => None,
        })
        .collect();
    if ids.len() != params.len() {
        return None;
    }
    out.iter().find(|f| f.params == ids).map(|f| f.idx)
}

fn push_fn(
    name_local: Option<LocalId>,
    params: Vec<LocalId>,
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    out: &mut Vec<FnInfo>,
) -> Option<usize> {
    let bound = bound_in_fn(&params, body);
    if let Some(n) = name_local {
        // name is bound inside the function for recursion
        let mut bound = bound.clone();
        bound.insert(n);
        return push_fn_with_bound(name_local, params, body, by_id, &bound, out);
    }
    push_fn_with_bound(name_local, params, body, by_id, &bound, out)
}

fn push_fn_with_bound(
    name_local: Option<LocalId>,
    params: Vec<LocalId>,
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    bound: &HashSet<LocalId>,
    out: &mut Vec<FnInfo>,
) -> Option<usize> {
    let mut free = HashSet::new();
    collect_free_in_body(body, bound, &mut free);
    // Nested free through nested Function decls/exprs already in body free collection
    // for exprs; nested Stmt::Function free handled via collect_free that skips nested
    // function bodies — re-walk nested decls:
    for stmt in body {
        collect_nested_free_through(stmt, bound, by_id, &mut free)?;
    }
    let mut captures: Vec<LocalId> = free.into_iter().collect();
    captures.sort_by_key(|id| id.0);
    if captures.len() > MAX_CAPS {
        return None;
    }
    for id in &captures {
        let loc = by_id.get(id)?;
        if !matches!(loc.ty, Type::Number | Type::Any) {
            return None;
        }
    }
    let idx = out.len();
    out.push(FnInfo {
        idx,
        params,
        captures,
        body: body.to_vec(),
        name_local,
    });
    Some(idx)
}

fn bound_in_fn(params: &[LocalId], body: &[Stmt]) -> HashSet<LocalId> {
    let mut bound: HashSet<LocalId> = params.iter().copied().collect();
    collect_bound_in_body(body, &mut bound);
    bound
}

fn collect_bound_in_body(body: &[Stmt], bound: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Declare { local, .. } => {
                bound.insert(*local);
            }
            Stmt::Function { local, .. } => {
                bound.insert(*local);
            }
            Stmt::Block { body } => collect_bound_in_body(body, bound),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                collect_bound_in_body(std::slice::from_ref(consequent), bound);
                if let Some(a) = alternate {
                    collect_bound_in_body(std::slice::from_ref(a), bound);
                }
            }
            _ => {}
        }
    }
}

fn collect_free_in_body(body: &[Stmt], bound: &HashSet<LocalId>, free: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Return { value: Some(v) } => collect_free_in_expr(v, bound, free),
            Stmt::Declare { init, .. } => {
                if let Some(e) = init {
                    collect_free_in_expr(e, bound, free);
                }
            }
            Stmt::Block { body } => collect_free_in_body(body, bound, free),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                collect_free_in_expr(test, bound, free);
                collect_free_in_body(std::slice::from_ref(consequent), bound, free);
                if let Some(a) = alternate {
                    collect_free_in_body(std::slice::from_ref(a), bound, free);
                }
            }
            Stmt::Function { .. } => {}
            _ => {}
        }
    }
}

fn collect_free_in_expr(expr: &Expr, bound: &HashSet<LocalId>, free: &mut HashSet<LocalId>) {
    match expr {
        Expr::Local { id, .. } => {
            if !bound.contains(id) {
                free.insert(*id);
            }
        }
        Expr::Unary { arg, .. } => collect_free_in_expr(arg, bound, free),
        Expr::Binary { left, right, .. } => {
            collect_free_in_expr(left, bound, free);
            collect_free_in_expr(right, bound, free);
        }
        Expr::Call { callee, args, .. } => {
            collect_free_in_expr(callee, bound, free);
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_free_in_expr(e, bound, free);
                }
            }
        }
        Expr::Function {
            name,
            params,
            body,
            ..
        } => {
            let mut nested_bound = bound_in_fn(
                &params
                    .iter()
                    .filter_map(|p| match &p.pattern {
                        Pattern::Local(id) => Some(*id),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                body,
            );
            if let Some(n) = name {
                nested_bound.insert(*n);
            }
            let mut nested_free = HashSet::new();
            collect_free_in_body(body, &nested_bound, &mut nested_free);
            for id in nested_free {
                if !bound.contains(&id) {
                    free.insert(id);
                }
            }
        }
        _ => {}
    }
}

fn collect_nested_free_through(
    stmt: &Stmt,
    outer_bound: &HashSet<LocalId>,
    by_id: &HashMap<LocalId, &Local>,
    free: &mut HashSet<LocalId>,
) -> Option<()> {
    match stmt {
        Stmt::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if *is_async || *is_generator {
                return None;
            }
            let param_ids = simple_param_locals(params, by_id)?;
            let nested_bound = bound_in_fn(&param_ids, body);
            let mut nested_free = HashSet::new();
            collect_free_in_body(body, &nested_bound, &mut nested_free);
            for s in body {
                collect_nested_free_through(s, &nested_bound, by_id, &mut nested_free)?;
            }
            for id in nested_free {
                if !outer_bound.contains(&id) {
                    free.insert(id);
                }
            }
            Some(())
        }
        Stmt::Block { body } => {
            for s in body {
                collect_nested_free_through(s, outer_bound, by_id, free)?;
            }
            Some(())
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_nested_free_through(consequent, outer_bound, by_id, free)?;
            if let Some(a) = alternate {
                collect_nested_free_through(a, outer_bound, by_id, free)?;
            }
            Some(())
        }
        _ => Some(()),
    }
}

fn simple_param_locals(
    params: &[Param],
    by_id: &HashMap<LocalId, &Local>,
) -> Option<Vec<LocalId>> {
    let mut out = Vec::with_capacity(params.len());
    for p in params {
        if p.rest || p.default.is_some() {
            return None;
        }
        let Pattern::Local(id) = &p.pattern else {
            return None;
        };
        let loc = by_id.get(id)?;
        if !matches!(loc.ty, Type::Number | Type::Any) {
            return None;
        }
        out.push(*id);
    }
    Some(out)
}

fn body_returns_fn(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::Return {
            value: Some(Expr::Function { .. }),
        } => true,
        Stmt::Block { body } => body_returns_fn(body),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            body_returns_fn(std::slice::from_ref(consequent))
                || alternate
                    .as_ref()
                    .is_some_and(|a| body_returns_fn(std::slice::from_ref(a)))
        }
        _ => false,
    })
}

fn fn_body_ok(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    body.iter().all(|s| match s {
        Stmt::Return { value: Some(v) } => match v {
            Expr::Function {
                is_async,
                is_generator,
                params,
                body,
                ..
            } => {
                !*is_async
                    && !*is_generator
                    && simple_param_locals(params, by_id).is_some()
                    && fn_body_ok(body, by_id, fn_arities, functions, fn_binding)
            }
            _ => number_expr_ok(v, by_id, fn_arities, functions, fn_binding),
        },
        Stmt::Return { value: None } => false,
        Stmt::Block { body } => fn_body_ok(body, by_id, fn_arities, functions, fn_binding),
        Stmt::Declare { local, init, .. } => {
            let Some(loc) = by_id.get(local) else {
                return false;
            };
            if !matches!(loc.ty, Type::Number | Type::Any | Type::Function) {
                return false;
            }
            match init {
                Some(Expr::Function {
                    is_async,
                    is_generator,
                    params,
                    body,
                    ..
                }) => {
                    !*is_async
                        && !*is_generator
                        && simple_param_locals(params, by_id).is_some()
                        && fn_body_ok(body, by_id, fn_arities, functions, fn_binding)
                }
                Some(e) => number_expr_ok(e, by_id, fn_arities, functions, fn_binding),
                None => true,
            }
        }
        Stmt::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if *is_async || *is_generator {
                return false;
            }
            simple_param_locals(params, by_id).is_some()
                && fn_body_ok(body, by_id, fn_arities, functions, fn_binding)
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            bool_expr_ok(test, by_id, fn_arities, functions, fn_binding)
                && fn_body_ok(
                    std::slice::from_ref(consequent),
                    by_id,
                    fn_arities,
                    functions,
                    fn_binding,
                )
                && alternate.as_ref().is_none_or(|a| {
                    fn_body_ok(
                        std::slice::from_ref(a),
                        by_id,
                        fn_arities,
                        functions,
                        fn_binding,
                    )
                })
        }
        _ => false,
    })
}

fn bool_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            use draconic_ast::BinaryOp::*;
            matches!(
                op,
                Lt | LtEq | Gt | GtEq | EqEq | NotEq | EqEqEq | NotEqEq
            ) && number_expr_ok(left, by_id, fn_arities, functions, fn_binding)
                && number_expr_ok(right, by_id, fn_arities, functions, fn_binding)
        }
        _ => number_expr_ok(expr, by_id, fn_arities, functions, fn_binding),
    }
}

fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            if fn_arities.contains_key(id) {
                return false;
            }
            matches!(ty, Type::Number | Type::Any)
                && by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::Unary {
            op: draconic_ast::UnaryOp::Plus | draconic_ast::UnaryOp::Minus,
            arg,
            ..
        } => number_expr_ok(arg, by_id, fn_arities, functions, fn_binding),
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            use draconic_ast::BinaryOp::*;
            matches!(op, Add | Sub | Mul | Div | Rem)
                && number_expr_ok(left, by_id, fn_arities, functions, fn_binding)
                && number_expr_ok(right, by_id, fn_arities, functions, fn_binding)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return false;
            }
            if !args.iter().all(|a| match a {
                Arg::Expr(e) => number_expr_ok(e, by_id, fn_arities, functions, fn_binding),
                Arg::Spread(_) => false,
            }) {
                return false;
            }
            match callee.as_ref() {
                Expr::Local { id, .. } => {
                    fn_arities.get(id).is_some_and(|n| *n == args.len())
                }
                Expr::Function {
                    params,
                    is_async,
                    is_generator,
                    body,
                    ..
                } => {
                    !*is_async
                        && !*is_generator
                        && simple_param_locals(params, by_id)
                            .is_some_and(|p| p.len() == args.len())
                        && fn_body_ok(body, by_id, fn_arities, functions, fn_binding)
                }
                Expr::Call {
                    callee: inner,
                    args: inner_args,
                    optional: opt2,
                    ..
                } => {
                    if *opt2 {
                        return false;
                    }
                    if !inner_args.iter().all(|a| match a {
                        Arg::Expr(e) => {
                            number_expr_ok(e, by_id, fn_arities, functions, fn_binding)
                        }
                        Arg::Spread(_) => false,
                    }) {
                        return false;
                    }
                    let Expr::Local { id, .. } = inner.as_ref() else {
                        return false;
                    };
                    let Some(&caller_idx) = fn_binding.get(id) else {
                        return false;
                    };
                    let f = &functions[caller_idx];
                    body_returns_fn(&f.body)
                        && returned_fn_arity(&f.body).is_some_and(|n| n == args.len())
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn returned_fn_arity(body: &[Stmt]) -> Option<usize> {
    for s in body {
        match s {
            Stmt::Return {
                value: Some(Expr::Function { params, .. }),
            } => return Some(params.len()),
            Stmt::Block { body } => {
                if let Some(n) = returned_fn_arity(body) {
                    return Some(n);
                }
            }
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                if let Some(n) = returned_fn_arity(std::slice::from_ref(consequent)) {
                    return Some(n);
                }
                if let Some(a) = alternate {
                    if let Some(n) = returned_fn_arity(std::slice::from_ref(a)) {
                        return Some(n);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn returned_fn_idx_in_body(body: &[Stmt], functions: &[FnInfo]) -> Option<usize> {
    for s in body {
        match s {
            Stmt::Return {
                value: Some(Expr::Function { params, .. }),
            } => {
                return find_fn_idx_by_param_patterns(params, functions);
            }
            Stmt::Block { body } => {
                if let Some(i) = returned_fn_idx_in_body(body, functions) {
                    return Some(i);
                }
            }
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                if let Some(i) = returned_fn_idx_in_body(std::slice::from_ref(consequent), functions)
                {
                    return Some(i);
                }
                if let Some(a) = alternate {
                    if let Some(i) = returned_fn_idx_in_body(std::slice::from_ref(a), functions) {
                        return Some(i);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    /// fn idx → LLVM name
    fn_names: HashMap<usize, String>,
    allocas: HashMap<LocalId, String>,
    out: String,
    body: String,
    tmp: u32,
    label: u32,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        let mut fn_names = HashMap::new();
        for f in &info.functions {
            fn_names.insert(f.idx, format!("d_fn_{}", f.idx));
        }
        Self {
            module,
            info,
            fn_names,
            allocas: HashMap::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
            label: 0,
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("%t{t}")
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label;
        self.label += 1;
        format!("{prefix}{n}")
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.03.05 ES functions + expressions + arrows via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(&[PRINT_F64])).ok();
        writeln!(self.out, "@es_ret_fn = private global i32 -1").ok();
        writeln!(
            self.out,
            "@es_ret_cap = private global [{MAX_CAPS} x double] zeroinitializer"
        )
        .ok();
        writeln!(self.out).ok();

        for f in &info.functions {
            self.emit_function(f)?;
        }

        self.body.clear();
        self.tmp = 0;
        self.label = 0;
        self.allocas.clear();

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();

        for id in &info.user_locals {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            writeln!(self.out, "  {ptr} = alloca double, align 8").ok();
        }
        // Function-binding locals that are only used as call targets need no storage
        // when statically bound; assigns of FunctionExpr to unused-as-value slots skip.

        for stmt in &self.module.body {
            if matches!(stmt, Stmt::Function { .. }) {
                continue;
            }
            self.emit_top_stmt(stmt)?;
        }

        for id in &info.user_locals {
            let ptr = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("internal: print missing alloca"))?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
        }

        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_function(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let fn_name = self.fn_names.get(&f.idx).cloned().unwrap();

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_label = self.label;
        let saved_allocas = std::mem::take(&mut self.allocas);

        self.tmp = 0;
        self.label = 0;
        self.allocas.clear();

        let mut sig_parts = Vec::new();
        for (i, _) in f.params.iter().enumerate() {
            sig_parts.push(format!("double %p{i}"));
        }
        for (i, _) in f.captures.iter().enumerate() {
            sig_parts.push(format!("double %c{i}"));
        }
        let sig = sig_parts.join(", ");

        let mut entry = String::new();
        for (i, pid) in f.params.iter().enumerate() {
            let ptr = format!("%l{}", pid.0);
            self.allocas.insert(*pid, ptr.clone());
            writeln!(entry, "  {ptr} = alloca double, align 8").ok();
            writeln!(entry, "  store double %p{i}, ptr {ptr}").ok();
        }
        for (i, cid) in f.captures.iter().enumerate() {
            let ptr = format!("%l{}", cid.0);
            self.allocas.insert(*cid, ptr.clone());
            writeln!(entry, "  {ptr} = alloca double, align 8").ok();
            writeln!(entry, "  store double %c{i}, ptr {ptr}").ok();
        }

        for stmt in &f.body {
            self.emit_fn_stmt(stmt)?;
        }
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
        }

        writeln!(self.out, "define double @{fn_name}({sig}) {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{entry}").ok();
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.label = saved_label;
        self.allocas = saved_allocas;
        Ok(())
    }

    fn body_ends_with_terminator(&self) -> bool {
        for line in self.body.lines().rev() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            return t.starts_with("ret ") || t.starts_with("br ");
        }
        false
    }

    fn emit_top_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                if self.info.fn_binding.contains_key(local) {
                    // Function binding — no number storage required for static calls.
                    return Ok(());
                }
                let ptr = self
                    .allocas
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("internal: missing alloca"))?;
                let init = init
                    .as_ref()
                    .ok_or_else(|| diag("es_functions: declare requires init"))?;
                let v = self.emit_number_expr(init)?;
                writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                Ok(())
            }
            Stmt::Function { .. } => Ok(()),
            _ => Err(diag("es_functions: unsupported top-level stmt")),
        }
    }

    fn emit_fn_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(v) } => {
                if let Expr::Function { .. } = v {
                    return self.emit_return_fn(v);
                }
                let n = self.emit_number_expr(v)?;
                writeln!(self.body, "  ret double {n}").ok();
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    if self.body_ends_with_terminator() {
                        break;
                    }
                    self.emit_fn_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Declare { local, init, .. } => {
                if self.info.fn_binding.contains_key(local) {
                    return Ok(());
                }
                let ptr = format!("%l{}", local.0);
                self.allocas.insert(*local, ptr.clone());
                writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                if let Some(e) = init {
                    if matches!(e, Expr::Function { .. }) {
                        writeln!(
                            self.body,
                            "  store double 0.00000000000000000e+00, ptr {ptr}"
                        )
                        .ok();
                    } else {
                        let v = self.emit_number_expr(e)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                } else {
                    writeln!(
                        self.body,
                        "  store double 0.00000000000000000e+00, ptr {ptr}"
                    )
                    .ok();
                }
                Ok(())
            }
            Stmt::Function { .. } => Ok(()),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let cond = self.emit_bool_expr(test)?;
                let then_l = self.fresh_label("then");
                let else_l = self.fresh_label("else");
                let end_l = self.fresh_label("endif");
                if alternate.is_some() {
                    writeln!(
                        self.body,
                        "  br i1 {cond}, label %{then_l}, label %{else_l}"
                    )
                    .ok();
                } else {
                    writeln!(
                        self.body,
                        "  br i1 {cond}, label %{then_l}, label %{end_l}"
                    )
                    .ok();
                }
                writeln!(self.body, "{then_l}:").ok();
                self.emit_fn_stmt(consequent)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{end_l}").ok();
                }
                if let Some(alt) = alternate {
                    writeln!(self.body, "{else_l}:").ok();
                    self.emit_fn_stmt(alt)?;
                    if !self.body_ends_with_terminator() {
                        writeln!(self.body, "  br label %{end_l}").ok();
                    }
                }
                writeln!(self.body, "{end_l}:").ok();
                Ok(())
            }
            _ => Err(diag("es_functions: unsupported stmt in function body")),
        }
    }

    fn emit_return_fn(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        let Expr::Function { params, .. } = expr else {
            return Err(diag("internal: emit_return_fn"));
        };
        let idx = find_fn_idx_by_param_patterns(params, &self.info.functions)
            .ok_or_else(|| diag("es_functions: return unknown FunctionExpr"))?;
        let f = &self.info.functions[idx];
        writeln!(self.body, "  store i32 {idx}, ptr @es_ret_fn").ok();
        for (i, cid) in f.captures.iter().enumerate() {
            let ptr = self.allocas.get(cid).cloned().ok_or_else(|| {
                diag(format!(
                    "es_functions: return capture %{} not in frame",
                    cid.0
                ))
            })?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            let gep = self.fresh();
            writeln!(
                self.body,
                "  {gep} = getelementptr inbounds [{MAX_CAPS} x double], ptr @es_ret_cap, i64 0, i64 {i}"
            )
            .ok();
            writeln!(self.body, "  store double {v}, ptr {gep}").ok();
        }
        // Return fn idx as double for chaining.
        let d = self.fresh();
        writeln!(self.body, "  {d} = sitofp i32 {idx} to double").ok();
        writeln!(self.body, "  ret double {d}").ok();
        Ok(())
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Binary {
                left, op, right, ..
            } => {
                use draconic_ast::BinaryOp::*;
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let pred = match op {
                    Lt => "olt",
                    LtEq => "ole",
                    Gt => "ogt",
                    GtEq => "oge",
                    EqEq | EqEqEq => "oeq",
                    NotEq | NotEqEq => "one",
                    _ => return Err(diag("es_functions: unsupported compare")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = fcmp {pred} double {l}, {r}").ok();
                Ok(t)
            }
            _ => {
                // ToBoolean on number: != 0
                let n = self.emit_number_expr(expr)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {t} = fcmp one double {n}, 0.00000000000000000e+00"
                )
                .ok();
                Ok(t)
            }
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => Ok(format_number_const(raw)?),
            Expr::Local { id, .. } => {
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated local %{}", id.0)))?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Unary {
                op: draconic_ast::UnaryOp::Plus,
                arg,
                ..
            } => self.emit_number_expr(arg),
            Expr::Unary {
                op: draconic_ast::UnaryOp::Minus,
                arg,
                ..
            } => {
                let a = self.emit_number_expr(arg)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = fneg double {a}").ok();
                Ok(t)
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                use draconic_ast::BinaryOp::*;
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let inst = match op {
                    Add => "fadd",
                    Sub => "fsub",
                    Mul => "fmul",
                    Div => "fdiv",
                    Rem => "frem",
                    _ => return Err(diag("es_functions: unsupported binary")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_functions: optional call not supported"));
                }
                self.emit_call(callee, args)
            }
            _ => Err(diag("es_functions: unsupported number expr")),
        }
    }

    fn emit_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let mut arg_vals = Vec::with_capacity(args.len());
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_functions: spread args not supported"));
                }
            }
        }

        match callee {
            Expr::Local { id, .. } => {
                let idx = *self
                    .info
                    .fn_binding
                    .get(id)
                    .ok_or_else(|| diag("es_functions: call to unbound function local"))?;
                self.emit_direct_call(idx, &arg_vals)
            }
            Expr::Function { params, .. } => {
                let idx = find_fn_idx_by_param_patterns(params, &self.info.functions)
                    .ok_or_else(|| diag("es_functions: IIFE unknown FunctionExpr"))?;
                self.emit_direct_call(idx, &arg_vals)
            }
            Expr::Call {
                callee: inner,
                args: inner_args,
                ..
            } => {
                // Higher-order: evaluate inner call (sets @es_ret_fn / caps if returns fn).
                let _inner_ret = self.emit_call(inner, inner_args)?;
                let idx = match inner.as_ref() {
                    Expr::Local { id, .. } => {
                        let caller_idx = *self.info.fn_binding.get(id).ok_or_else(|| {
                            diag("es_functions: higher-order call unbound callee")
                        })?;
                        returned_fn_idx_in_body(
                            &self.info.functions[caller_idx].body,
                            &self.info.functions,
                        )
                        .ok_or_else(|| diag("es_functions: callee does not return function"))?
                    }
                    _ => return Err(diag("es_functions: unsupported higher-order callee")),
                };
                // Load captures from return buffer (set by the returning call).
                let f = &self.info.functions[idx];
                let mut full_args = arg_vals;
                for i in 0..f.captures.len() {
                    let gep = self.fresh();
                    writeln!(
                        self.body,
                        "  {gep} = getelementptr inbounds [{MAX_CAPS} x double], ptr @es_ret_cap, i64 0, i64 {i}"
                    )
                    .ok();
                    let c = self.fresh();
                    writeln!(self.body, "  {c} = load double, ptr {gep}").ok();
                    full_args.push(c);
                }
                // Direct call without re-loading captures from frame.
                let fn_name = self.fn_names.get(&idx).cloned().unwrap();
                let t = self.fresh();
                if full_args.is_empty() {
                    writeln!(self.body, "  {t} = call double @{fn_name}()").ok();
                } else {
                    let parts: Vec<String> = full_args
                        .iter()
                        .map(|v| format!("double {v}"))
                        .collect();
                    writeln!(
                        self.body,
                        "  {t} = call double @{fn_name}({})",
                        parts.join(", ")
                    )
                    .ok();
                }
                Ok(t)
            }
            _ => Err(diag("es_functions: unsupported call callee")),
        }
    }

    fn emit_direct_call(&mut self, idx: usize, arg_vals: &[String]) -> Result<String, Diagnostic> {
        let f = &self.info.functions[idx];
        if arg_vals.len() != f.params.len() {
            return Err(diag("es_functions: call arity mismatch"));
        }
        let fn_name = self.fn_names.get(&idx).cloned().unwrap();
        let mut all = arg_vals.to_vec();
        for cid in &f.captures {
            let ptr = self.allocas.get(cid).cloned().ok_or_else(|| {
                diag(format!(
                    "es_functions: capture local %{} not in caller frame",
                    cid.0
                ))
            })?;
            let t = self.fresh();
            writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
            all.push(t);
        }
        let t = self.fresh();
        if all.is_empty() {
            writeln!(self.body, "  {t} = call double @{fn_name}()").ok();
        } else {
            let parts: Vec<String> = all.iter().map(|v| format!("double {v}")).collect();
            writeln!(
                self.body,
                "  {t} = call double @{fn_name}({})",
                parts.join(", ")
            )
            .ok();
        }
        Ok(t)
    }
}

fn format_number_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(format!("invalid number literal {raw}")))?;
    Ok(format!("{f:.17e}"))
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
