//! N08.03.01–N08.03.07 + N08.16.11–N08.16.14: native observations for ES function
//! declarations, expressions, and arrows (simple ident params + defaults + rest) —
//! E03.01–E03.07 / `es/functions/*`, Annex B labelled function declarations —
//! E18.11 / `es/annex-b/labelled_function`, Annex B FunctionDeclarations in
//! `if` — E18.12 / `es/annex-b/if_function`, block-level function declarations
//! — E18.13 / `es/annex-b/block_function`, and `var` declarations (hoist, redeclare,
//! uninit → undefined) — E18.14 / `es/annex-b/var_decl`.
//!
//! Nested/non-escaping decls use extra by-value capture params. Function
//! expressions and arrows are first-class as fn-id doubles; returned closures
//! stash captures in a small return buffer for immediate call (`make(10)(7)`).
//! Missing/undefined args use a NaN payload sentinel; callee applies defaults.
//! Rest params pack trailing args into a stack buffer of doubles; `for-of` over
//! the rest local iterates that buffer (no full JS array heap).
//! Labelled function declarations (`L: function f(){…}`) unwrap to ordinary decls.
//! If-clause function decls (Annex B.3.4) bind only when the branch runs; same-name
//! then/else share one slot; `typeof` on an unbound name is `"undefined"`.
//! Block-level function decls (Annex B.3.2) activate when the block runs; same-name
//! redecls share one outer slot (last activation wins).
//! `var` is function/script-scoped: slots hoist to entry as undefined; same-name
//! redecls share one primary; `typeof` of uninit is `"undefined"`.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::AssignOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, BindingKind, Expr, IrType as Type, Local, LocalId, Module, Param, Pattern,
    Stmt,
};
use draconic_runtime::abi::{llvm_declares, PRINT_F64, PRINT_STR};

const MAX_CAPS: usize = 8;
/// Max trailing rest arguments packed into the stack buffer (fixture uses ≤3).
const MAX_REST: usize = 8;
/// qNaN payload marking JS `undefined` for default-parameter application.
const UNDEF_BITS: u64 = 0x7FF8_0000_0000_0001;

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
    /// Fixed (non-rest) params only.
    params: Vec<LocalId>,
    /// Parallel to `params`; `Some` → apply when arg missing/undefined.
    defaults: Vec<Option<Expr>>,
    /// Last param `...rest` local, if any.
    rest: Option<LocalId>,
    captures: Vec<LocalId>,
    body: Vec<Stmt>,
    /// Named function expression recursive binding.
    name_local: Option<LocalId>,
}

struct ModuleInfo {
    functions: Vec<FnInfo>,
    /// Locals statically bound to a function index (decl / expr assign / name).
    fn_binding: HashMap<LocalId, usize>,
    /// Top-level user locals to print (declare order): numbers and typeof-strings.
    user_locals: Vec<LocalId>,
    /// Subset of `user_locals` holding typeof string observations.
    string_locals: HashSet<LocalId>,
    /// Annex B if-clause primary binding locals (runtime i32 fn-idx slot; -1 = unbound).
    if_fn_slots: HashSet<LocalId>,
    /// If-clause Function local → primary slot local (then/else same name share primary).
    if_fn_primary: HashMap<LocalId, LocalId>,
    /// Primary slot → possible fn idxs that may be stored there (for dynamic dispatch).
    if_fn_candidates: HashMap<LocalId, Vec<usize>>,
    /// `var` redeclare/use local → primary storage (same name, script/function scope).
    var_primary: HashMap<LocalId, LocalId>,
    /// Top-level (script) hoisted `var` primary slots.
    top_var_slots: HashSet<LocalId>,
    /// Per-function idx → hoisted `var` primary slots in that body.
    fn_var_slots: HashMap<usize, HashSet<LocalId>>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut fn_binding = HashMap::new();
    let mut user_locals = Vec::new();
    let mut string_locals = HashSet::new();
    let mut if_fn_primary = HashMap::new();
    let mut if_fn_slots = HashSet::new();
    let mut var_primary = HashMap::new();
    let mut top_var_slots = HashSet::new();
    let mut fn_var_slots: HashMap<usize, HashSet<LocalId>> = HashMap::new();

    // Collect every function (decl + expr) first so arities are known.
    collect_all_functions(&module.body, &by_id, &mut functions, &mut fn_binding)?;
    record_if_fn_bindings(
        &module.body,
        &by_id,
        &fn_binding,
        &mut if_fn_primary,
        &mut if_fn_slots,
    );

    // Script-scope `var` hoist + same-name redecl share (E18.14).
    collect_var_slots_in_stmts(&module.body, &by_id, &mut var_primary, &mut top_var_slots);
    for f in &functions {
        let mut slots = HashSet::new();
        collect_var_slots_in_stmts(&f.body, &by_id, &mut var_primary, &mut slots);
        if !slots.is_empty() {
            fn_var_slots.insert(f.idx, slots);
        }
    }

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
        let mut rest_locals = HashSet::new();
        if let Some(r) = f.rest {
            rest_locals.insert(r);
        }
        if !fn_body_ok(
            &f.body,
            &by_id,
            &fn_arities,
            &functions,
            &fn_binding,
            &rest_locals,
        ) {
            return None;
        }
        for d in &f.defaults {
            if let Some(e) = d {
                if !number_expr_ok(e, &by_id, &fn_arities, &functions, &fn_binding) {
                    return None;
                }
            }
        }
    }

    let mut has_fn = !functions.is_empty();
    let mut observed = HashSet::new();
    for stmt in &module.body {
        if !classify_top_stmt(
            stmt,
            &by_id,
            &fn_arities,
            &functions,
            &fn_binding,
            &if_fn_primary,
            &if_fn_slots,
            &var_primary,
            &mut has_fn,
            &mut user_locals,
            &mut string_locals,
            &mut observed,
            true,
        ) {
            return None;
        }
    }

    if !has_fn || user_locals.is_empty() {
        return None;
    }

    let mut if_fn_candidates: HashMap<LocalId, Vec<usize>> = HashMap::new();
    for (loc, primary) in &if_fn_primary {
        if let Some(&idx) = fn_binding.get(loc) {
            let c = if_fn_candidates.entry(*primary).or_default();
            if !c.contains(&idx) {
                c.push(idx);
            }
        }
    }
    for c in if_fn_candidates.values_mut() {
        c.sort_unstable();
    }

    Some(ModuleInfo {
        functions,
        fn_binding,
        user_locals,
        string_locals,
        if_fn_slots,
        if_fn_primary,
        if_fn_candidates,
        var_primary,
        top_var_slots,
        fn_var_slots,
    })
}

/// Register a `var` local into a scope's primary-slot set (same name → share primary).
fn register_var_slot(
    local: LocalId,
    by_id: &HashMap<LocalId, &Local>,
    var_primary: &mut HashMap<LocalId, LocalId>,
    var_slots: &mut HashSet<LocalId>,
) {
    if var_primary.contains_key(&local) {
        return;
    }
    let name = by_id.get(&local).map(|l| l.name.as_str());
    if let Some(name) = name {
        let mut shared: Option<LocalId> = None;
        for &primary in var_slots.iter() {
            if by_id.get(&primary).is_some_and(|l| l.name == name) {
                shared = Some(primary);
                break;
            }
        }
        if let Some(primary) = shared {
            var_primary.insert(local, primary);
            return;
        }
    }
    var_primary.insert(local, local);
    var_slots.insert(local);
}

/// Collect `var` declares in `stmts` (does not enter nested function bodies).
fn collect_var_slots_in_stmts(
    stmts: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    var_primary: &mut HashMap<LocalId, LocalId>,
    var_slots: &mut HashSet<LocalId>,
) {
    for stmt in stmts {
        collect_var_slots_in_stmt(stmt, by_id, var_primary, var_slots);
    }
}

fn collect_var_slots_in_stmt(
    stmt: &Stmt,
    by_id: &HashMap<LocalId, &Local>,
    var_primary: &mut HashMap<LocalId, LocalId>,
    var_slots: &mut HashSet<LocalId>,
) {
    match stmt {
        Stmt::Declare {
            local,
            kind: BindingKind::Var,
            ..
        } => {
            register_var_slot(*local, by_id, var_primary, var_slots);
        }
        Stmt::Block { body } => collect_var_slots_in_stmts(body, by_id, var_primary, var_slots),
        Stmt::Labeled { body, .. } => {
            collect_var_slots_in_stmt(body, by_id, var_primary, var_slots)
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_var_slots_in_stmt(consequent, by_id, var_primary, var_slots);
            if let Some(a) = alternate {
                collect_var_slots_in_stmt(a, by_id, var_primary, var_slots);
            }
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
            collect_var_slots_in_stmt(body, by_id, var_primary, var_slots)
        }
        Stmt::For { init, body, .. } => {
            if let Some(i) = init {
                collect_var_slots_in_stmt(i, by_id, var_primary, var_slots);
            }
            collect_var_slots_in_stmt(body, by_id, var_primary, var_slots);
        }
        Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
            collect_var_slots_in_stmt(left, by_id, var_primary, var_slots);
            collect_var_slots_in_stmt(body, by_id, var_primary, var_slots);
        }
        Stmt::Switch { cases, .. } => {
            for c in cases {
                collect_var_slots_in_stmts(&c.body, by_id, var_primary, var_slots);
            }
        }
        // Nested functions have their own var environment.
        Stmt::Function { .. } => {}
        Stmt::Declare {
            init: Some(Expr::Function { .. }),
            ..
        } => {}
        _ => {}
    }
}

/// Unwrap `L: …: function f` / bare `function f` as if/else clause.
fn unwrap_if_fn_local(stmt: &Stmt) -> Option<LocalId> {
    let mut s = stmt;
    while let Stmt::Labeled { body, .. } = s {
        s = body;
    }
    match s {
        Stmt::Function { local, .. } => Some(*local),
        _ => None,
    }
}

/// Annex B.3.2 / B.3.4: deferred function binding slot; same-name redecls share the
/// first primary (outer use binding). Last activation wins at runtime.
fn register_annex_b_fn_slot(
    local: LocalId,
    by_id: &HashMap<LocalId, &Local>,
    if_fn_primary: &mut HashMap<LocalId, LocalId>,
    if_fn_slots: &mut HashSet<LocalId>,
) {
    if if_fn_primary.contains_key(&local) {
        return;
    }
    let name = by_id.get(&local).map(|l| l.name.as_str());
    if let Some(name) = name {
        let mut shared: Option<LocalId> = None;
        for &primary in if_fn_slots.iter() {
            if by_id.get(&primary).is_some_and(|l| l.name == name) {
                shared = Some(primary);
                break;
            }
        }
        if let Some(primary) = shared {
            if_fn_primary.insert(local, primary);
            return;
        }
    }
    if_fn_primary.insert(local, local);
    if_fn_slots.insert(local);
}

/// Register a block-level (or labelled) `function` as an Annex B deferred slot.
fn register_block_level_fn(
    stmt: &Stmt,
    by_id: &HashMap<LocalId, &Local>,
    if_fn_primary: &mut HashMap<LocalId, LocalId>,
    if_fn_slots: &mut HashSet<LocalId>,
) {
    let mut s = stmt;
    while let Stmt::Labeled { body, .. } = s {
        s = body;
    }
    if let Stmt::Function { local, .. } = s {
        register_annex_b_fn_slot(*local, by_id, if_fn_primary, if_fn_slots);
    }
}

/// Annex B.3.4: if-clause function decls bind only when the branch runs; then/else
/// same name share the consequent (first) local as the primary use binding.
/// Annex B.3.2: block-level `function` decls bind when the block runs; same-name
/// redecls share one outer slot.
fn record_if_fn_bindings(
    stmts: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    if_fn_primary: &mut HashMap<LocalId, LocalId>,
    if_fn_slots: &mut HashSet<LocalId>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                let c = unwrap_if_fn_local(consequent);
                let a = alternate.as_ref().and_then(|s| unwrap_if_fn_local(s));
                match (c, a) {
                    (Some(cl), Some(al)) => {
                        let same = by_id.get(&cl).is_some_and(|l| {
                            by_id.get(&al).is_some_and(|r| l.name == r.name)
                        });
                        if same {
                            // Uses resolve to the first (consequent) binding.
                            register_annex_b_fn_slot(cl, by_id, if_fn_primary, if_fn_slots);
                            if_fn_primary.insert(al, cl);
                        } else {
                            register_annex_b_fn_slot(cl, by_id, if_fn_primary, if_fn_slots);
                            register_annex_b_fn_slot(al, by_id, if_fn_primary, if_fn_slots);
                        }
                    }
                    (Some(cl), None) => {
                        register_annex_b_fn_slot(cl, by_id, if_fn_primary, if_fn_slots);
                    }
                    (None, Some(al)) => {
                        register_annex_b_fn_slot(al, by_id, if_fn_primary, if_fn_slots);
                    }
                    (None, None) => {
                        record_if_fn_bindings(
                            std::slice::from_ref(consequent.as_ref()),
                            by_id,
                            fn_binding,
                            if_fn_primary,
                            if_fn_slots,
                        );
                        if let Some(alt) = alternate {
                            record_if_fn_bindings(
                                std::slice::from_ref(alt.as_ref()),
                                by_id,
                                fn_binding,
                                if_fn_primary,
                                if_fn_slots,
                            );
                        }
                    }
                }
            }
            Stmt::Function { body, .. } => {
                record_if_fn_bindings(body, by_id, fn_binding, if_fn_primary, if_fn_slots);
            }
            Stmt::Block { body } => {
                for s in body {
                    register_block_level_fn(s, by_id, if_fn_primary, if_fn_slots);
                }
                record_if_fn_bindings(body, by_id, fn_binding, if_fn_primary, if_fn_slots);
            }
            Stmt::Labeled { body, .. } => {
                record_if_fn_bindings(
                    std::slice::from_ref(body.as_ref()),
                    by_id,
                    fn_binding,
                    if_fn_primary,
                    if_fn_slots,
                );
            }
            Stmt::Declare {
                init: Some(Expr::Function { body, .. }),
                ..
            } => {
                record_if_fn_bindings(body, by_id, fn_binding, if_fn_primary, if_fn_slots);
            }
            _ => {}
        }
    }
    let _ = fn_binding;
}

fn classify_top_stmt(
    stmt: &Stmt,
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    if_fn_primary: &HashMap<LocalId, LocalId>,
    if_fn_slots: &HashSet<LocalId>,
    var_primary: &HashMap<LocalId, LocalId>,
    has_fn: &mut bool,
    user_locals: &mut Vec<LocalId>,
    string_locals: &mut HashSet<LocalId>,
    observed: &mut HashSet<LocalId>,
    observe_declares: bool,
) -> bool {
    match stmt {
        Stmt::Function { .. } => {
            *has_fn = true;
            true
        }
        Stmt::Labeled { body, .. } => classify_top_stmt(
            body,
            by_id,
            fn_arities,
            functions,
            fn_binding,
            if_fn_primary,
            if_fn_slots,
            var_primary,
            has_fn,
            user_locals,
            string_locals,
            observed,
            observe_declares,
        ),
        Stmt::Block { body } => body.iter().all(|s| {
            classify_top_stmt(
                s,
                by_id,
                fn_arities,
                functions,
                fn_binding,
                if_fn_primary,
                if_fn_slots,
                var_primary,
                has_fn,
                user_locals,
                string_locals,
                observed,
                false,
            )
        }),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            bool_expr_ok(test, by_id, fn_arities, functions, fn_binding)
                && classify_top_stmt(
                    consequent,
                    by_id,
                    fn_arities,
                    functions,
                    fn_binding,
                    if_fn_primary,
                    if_fn_slots,
                    var_primary,
                    has_fn,
                    user_locals,
                    string_locals,
                    observed,
                    false,
                )
                && alternate.as_ref().is_none_or(|a| {
                    classify_top_stmt(
                        a,
                        by_id,
                        fn_arities,
                        functions,
                        fn_binding,
                        if_fn_primary,
                        if_fn_slots,
                        var_primary,
                        has_fn,
                        user_locals,
                        string_locals,
                        observed,
                        false,
                    )
                })
        }
        Stmt::Declare { local, init, kind } => {
            let Some(loc) = by_id.get(local) else {
                return false;
            };
            match loc.ty {
                Type::Number | Type::Any => {
                    if let Some(init) = init.as_ref() {
                        if matches!(init, Expr::Function { .. }) {
                            if !fn_binding.contains_key(local) {
                                return false;
                            }
                            return true;
                        }
                        if !number_expr_ok(init, by_id, fn_arities, functions, fn_binding) {
                            // Call of if-clause function uses dynamic slot — still ok if callee is if-fn.
                            if !call_if_fn_ok(
                                init,
                                by_id,
                                fn_arities,
                                functions,
                                fn_binding,
                                if_fn_slots,
                            ) {
                                return false;
                            }
                        }
                    } else if *kind != BindingKind::Var {
                        return false;
                    }
                    if observe_declares {
                        // Same-name `var` redeclares share one observation slot (primary).
                        let obs = var_primary.get(local).copied().unwrap_or(*local);
                        if observed.insert(obs) {
                            user_locals.push(obs);
                        }
                    }
                    true
                }
                Type::String => {
                    let Some(init) = init.as_ref() else {
                        return false;
                    };
                    if !typeof_local_ok(
                        init,
                        if_fn_primary,
                        if_fn_slots,
                        fn_binding,
                        var_primary,
                        by_id,
                    ) {
                        return false;
                    }
                    if observe_declares {
                        if observed.insert(*local) {
                            user_locals.push(*local);
                            string_locals.insert(*local);
                        }
                    }
                    true
                }
                Type::Function => {
                    let Some(init) = init.as_ref() else {
                        return false;
                    };
                    matches!(init, Expr::Function { .. })
                }
                _ => false,
            }
        }
        Stmt::Expr { expr } => match expr {
            Expr::Assign {
                target: AssignTarget::Local(_),
                op: AssignOp::Eq,
                value,
                ..
            } => number_expr_ok(value, by_id, fn_arities, functions, fn_binding),
            _ => false,
        },
        _ => false,
    }
}

fn typeof_local_ok(
    expr: &Expr,
    if_fn_primary: &HashMap<LocalId, LocalId>,
    if_fn_slots: &HashSet<LocalId>,
    fn_binding: &HashMap<LocalId, usize>,
    var_primary: &HashMap<LocalId, LocalId>,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    let Expr::Unary {
        op: draconic_ast::UnaryOp::TypeOf,
        arg,
        ..
    } = expr
    else {
        return false;
    };
    let Expr::Local { id, .. } = arg.as_ref() else {
        return false;
    };
    if if_fn_slots.contains(id)
        || if_fn_primary.contains_key(id)
        || fn_binding.contains_key(id)
    {
        return true;
    }
    // `typeof` of a number/any local (incl. hoisted `var` that may be undefined).
    if var_primary.contains_key(id) {
        return true;
    }
    by_id
        .get(id)
        .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
}

/// `f()` where `f` is an Annex B if-clause binding (may be undefined until branch runs).
fn call_if_fn_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    if_fn_slots: &HashSet<LocalId>,
) -> bool {
    let Expr::Call {
        callee,
        args,
        optional,
        ..
    } = expr
    else {
        return false;
    };
    if *optional {
        return false;
    }
    if !args.iter().all(|a| match a {
        Arg::Expr(e) => number_expr_ok(e, by_id, fn_arities, functions, fn_binding),
        Arg::Spread(_) => false,
    }) {
        return false;
    }
    let Expr::Local { id, .. } = callee.as_ref() else {
        return false;
    };
    if !if_fn_slots.contains(id) && !fn_binding.contains_key(id) {
        return false;
    }
    // Arity: use any candidate with matching fixed arity, or static binding.
    if let Some(&idx) = fn_binding.get(id) {
        return call_arity_ok(&functions[idx], args.len());
    }
    true
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
                let (param_ids, defaults, rest) = simple_params(params, by_id)?;
                // Nested first.
                collect_all_functions(body, by_id, out, fn_binding)?;
                collect_exprs_in_body(body, by_id, out, fn_binding)?;
                let idx = push_fn(None, param_ids, defaults, rest, body, by_id, out)?;
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
            Stmt::Labeled { body, .. } => {
                collect_all_functions(std::slice::from_ref(body), by_id, out, fn_binding)?
            }
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
            Stmt::Labeled { body, .. } => {
                collect_exprs_in_body(std::slice::from_ref(body), by_id, out, fn_binding)?
            }
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
            } => {
                if *is_await {
                    return None;
                }
                collect_expr_fns(right, by_id, out, fn_binding)?;
                collect_exprs_in_body(std::slice::from_ref(left), by_id, out, fn_binding)?;
                collect_exprs_in_body(std::slice::from_ref(body), by_id, out, fn_binding)?;
            }
            Stmt::Expr { expr } => collect_expr_fns(expr, by_id, out, fn_binding)?,
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
            let (param_ids, defaults, rest) = simple_params(params, by_id)?;
            collect_all_functions(body, by_id, out, fn_binding)?;
            collect_exprs_in_body(body, by_id, out, fn_binding)?;
            let idx = push_fn(*name, param_ids, defaults, rest, body, by_id, out)?;
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
        Expr::Assign { value, .. } => collect_expr_fns(value, by_id, out, fn_binding),
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
    out.iter()
        .find(|f| {
            let mut all = f.params.clone();
            if let Some(r) = f.rest {
                all.push(r);
            }
            all == ids
        })
        .map(|f| f.idx)
}

fn push_fn(
    name_local: Option<LocalId>,
    params: Vec<LocalId>,
    defaults: Vec<Option<Expr>>,
    rest: Option<LocalId>,
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    out: &mut Vec<FnInfo>,
) -> Option<usize> {
    let bound = bound_in_fn(&params, rest, body);
    if let Some(n) = name_local {
        // name is bound inside the function for recursion
        let mut bound = bound.clone();
        bound.insert(n);
        return push_fn_with_bound(name_local, params, defaults, rest, body, by_id, &bound, out);
    }
    push_fn_with_bound(name_local, params, defaults, rest, body, by_id, &bound, out)
}

fn push_fn_with_bound(
    name_local: Option<LocalId>,
    params: Vec<LocalId>,
    defaults: Vec<Option<Expr>>,
    rest: Option<LocalId>,
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    bound: &HashSet<LocalId>,
    out: &mut Vec<FnInfo>,
) -> Option<usize> {
    let mut free = HashSet::new();
    collect_free_in_body(body, bound, &mut free);
    for d in &defaults {
        if let Some(e) = d {
            collect_free_in_expr(e, bound, &mut free);
        }
    }
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
        defaults,
        rest,
        captures,
        body: body.to_vec(),
        name_local,
    });
    Some(idx)
}

fn bound_in_fn(params: &[LocalId], rest: Option<LocalId>, body: &[Stmt]) -> HashSet<LocalId> {
    let mut bound: HashSet<LocalId> = params.iter().copied().collect();
    if let Some(r) = rest {
        bound.insert(r);
    }
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
            Stmt::Labeled { body, .. } => {
                collect_bound_in_body(std::slice::from_ref(body), bound)
            }
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
            Stmt::ForOf { left, body, .. } | Stmt::ForIn { left, body, .. } => {
                collect_bound_in_body(std::slice::from_ref(left), bound);
                collect_bound_in_body(std::slice::from_ref(body), bound);
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
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
            } => {
                if *is_await {
                    continue;
                }
                collect_free_in_expr(right, bound, free);
                collect_free_in_body(std::slice::from_ref(left), bound, free);
                collect_free_in_body(std::slice::from_ref(body), bound, free);
            }
            Stmt::Expr { expr } => collect_free_in_expr(expr, bound, free),
            Stmt::Function { .. } => {}
            Stmt::Labeled { body, .. } => {
                collect_free_in_body(std::slice::from_ref(body), bound, free)
            }
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
        Expr::Assign { value, .. } => collect_free_in_expr(value, bound, free),
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
            let fixed: Vec<LocalId> = params
                .iter()
                .filter(|p| !p.rest)
                .filter_map(|p| match &p.pattern {
                    Pattern::Local(id) => Some(*id),
                    _ => None,
                })
                .collect();
            let rest = params.iter().find(|p| p.rest).and_then(|p| match &p.pattern {
                Pattern::Local(id) => Some(*id),
                _ => None,
            });
            let mut nested_bound = bound_in_fn(&fixed, rest, body);
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
            let (param_ids, _, rest) = simple_params(params, by_id)?;
            let nested_bound = bound_in_fn(&param_ids, rest, body);
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
        Stmt::Labeled { body, .. } => collect_nested_free_through(body, outer_bound, by_id, free),
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
        Stmt::ForOf { left, body, .. } => {
            collect_nested_free_through(left, outer_bound, by_id, free)?;
            collect_nested_free_through(body, outer_bound, by_id, free)
        }
        _ => Some(()),
    }
}

fn simple_params(
    params: &[Param],
    by_id: &HashMap<LocalId, &Local>,
) -> Option<(Vec<LocalId>, Vec<Option<Expr>>, Option<LocalId>)> {
    let mut ids = Vec::with_capacity(params.len());
    let mut defaults = Vec::with_capacity(params.len());
    let mut rest = None;
    for (i, p) in params.iter().enumerate() {
        let Pattern::Local(id) = &p.pattern else {
            return None;
        };
        let loc = by_id.get(id)?;
        if !matches!(loc.ty, Type::Number | Type::Any) {
            return None;
        }
        if p.rest {
            if i != params.len() - 1 || p.default.is_some() || rest.is_some() {
                return None;
            }
            rest = Some(*id);
        } else {
            ids.push(*id);
            defaults.push(p.default.clone());
        }
    }
    Some((ids, defaults, rest))
}

fn call_arity_ok(f: &FnInfo, args_len: usize) -> bool {
    if f.rest.is_some() {
        if args_len >= f.params.len() {
            args_len - f.params.len() <= MAX_REST
        } else {
            f.defaults[args_len..].iter().all(|d| d.is_some())
        }
    } else if args_len > f.params.len() {
        false
    } else {
        f.defaults[args_len..].iter().all(|d| d.is_some())
    }
}

fn call_arity_ok_params(defaults: &[Option<Expr>], has_rest: bool, args_len: usize) -> bool {
    if has_rest {
        if args_len >= defaults.len() {
            args_len - defaults.len() <= MAX_REST
        } else {
            defaults[args_len..].iter().all(|d| d.is_some())
        }
    } else if args_len > defaults.len() {
        false
    } else {
        defaults[args_len..].iter().all(|d| d.is_some())
    }
}

fn undef_double_const() -> String {
    format!("bitcast (i64 {UNDEF_BITS} to double)")
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

fn nested_rest_locals(
    params: &[Param],
    by_id: &HashMap<LocalId, &Local>,
) -> Option<HashSet<LocalId>> {
    let (_, _, rest) = simple_params(params, by_id)?;
    let mut s = HashSet::new();
    if let Some(r) = rest {
        s.insert(r);
    }
    Some(s)
}

fn fn_body_ok(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    rest_locals: &HashSet<LocalId>,
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
                    && simple_params(params, by_id).is_some()
                    && nested_rest_locals(params, by_id).is_some_and(|rl| {
                        fn_body_ok(body, by_id, fn_arities, functions, fn_binding, &rl)
                    })
            }
            _ => number_expr_ok(v, by_id, fn_arities, functions, fn_binding),
        },
        Stmt::Return { value: None } => false,
        Stmt::Block { body } => {
            fn_body_ok(body, by_id, fn_arities, functions, fn_binding, rest_locals)
        }
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
                        && simple_params(params, by_id).is_some()
                        && nested_rest_locals(params, by_id).is_some_and(|rl| {
                            fn_body_ok(body, by_id, fn_arities, functions, fn_binding, &rl)
                        })
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
            simple_params(params, by_id).is_some()
                && nested_rest_locals(params, by_id).is_some_and(|rl| {
                    fn_body_ok(body, by_id, fn_arities, functions, fn_binding, &rl)
                })
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
                    rest_locals,
                )
                && alternate.as_ref().is_none_or(|a| {
                    fn_body_ok(
                        std::slice::from_ref(a),
                        by_id,
                        fn_arities,
                        functions,
                        fn_binding,
                        rest_locals,
                    )
                })
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
        } => {
            if *is_await {
                return false;
            }
            let Expr::Local { id, .. } = right else {
                return false;
            };
            if !rest_locals.contains(id) {
                return false;
            }
            matches!(
                left.as_ref(),
                Stmt::Declare {
                    init: None,
                    ..
                }
            ) && fn_body_ok(
                std::slice::from_ref(body),
                by_id,
                fn_arities,
                functions,
                fn_binding,
                rest_locals,
            )
        }
        Stmt::Expr { expr } => match expr {
            Expr::Assign {
                target: AssignTarget::Local(_),
                op: AssignOp::Eq,
                value,
                ..
            } => number_expr_ok(value, by_id, fn_arities, functions, fn_binding),
            _ => false,
        },
        Stmt::Labeled { body, .. } => fn_body_ok(
            std::slice::from_ref(body),
            by_id,
            fn_arities,
            functions,
            fn_binding,
            rest_locals,
        ),
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
        Expr::Boolean { .. } => true,
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
            if functions.iter().any(|f| f.rest == Some(*id)) {
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
        Expr::Unary {
            op: draconic_ast::UnaryOp::Void,
            arg,
            ..
        } => number_expr_ok(arg, by_id, fn_arities, functions, fn_binding)
            || matches!(arg.as_ref(), Expr::Number { .. } | Expr::Local { .. }),
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
                    let Some(&idx) = fn_binding.get(id) else {
                        return fn_arities.get(id).is_some_and(|n| args.len() <= *n);
                    };
                    call_arity_ok(&functions[idx], args.len())
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
                        && simple_params(params, by_id).is_some_and(|(_, defaults, rest)| {
                            call_arity_ok_params(&defaults, rest.is_some(), args.len())
                                && nested_rest_locals(params, by_id).is_some_and(|rl| {
                                    fn_body_ok(
                                        body,
                                        by_id,
                                        fn_arities,
                                        functions,
                                        fn_binding,
                                        &rl,
                                    )
                                })
                        })
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
                    if !body_returns_fn(&f.body) {
                        return false;
                    }
                    let Some(ret_idx) = returned_fn_idx_in_body(&f.body, functions) else {
                        return false;
                    };
                    call_arity_ok(&functions[ret_idx], args.len())
                }
                _ => false,
            }
        }
        _ => false,
    }
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
    /// Rest local → (buf ptr alloca, len i64 alloca).
    rest_slots: HashMap<LocalId, (String, String)>,
    /// Annex B if-fn primary → i32 alloca (fn idx or -1).
    if_fn_slot_ptrs: HashMap<LocalId, String>,
    /// String typeof obs local → i32 alloca (0 = "undefined", 1 = "function").
    typeof_code_ptrs: HashMap<LocalId, String>,
    str_globals: HashMap<String, String>,
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
            rest_slots: HashMap::new(),
            if_fn_slot_ptrs: HashMap::new(),
            typeof_code_ptrs: HashMap::new(),
            str_globals: HashMap::new(),
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

    /// Same-name `var` redecls / uses share one primary storage slot.
    fn resolve_var_slot(&self, id: LocalId) -> LocalId {
        self.info.var_primary.get(&id).copied().unwrap_or(id)
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.03.07 ES functions + defaults/rest via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(&[PRINT_F64, PRINT_STR])).ok();
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
        self.rest_slots.clear();
        self.if_fn_slot_ptrs.clear();
        self.typeof_code_ptrs.clear();

        // String globals for typeof observations (emitted before main).
        let mut prelude = String::new();

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();

        // Hoisted script-scope `var` slots (init undefined).
        let mut top_vars: Vec<LocalId> = info.top_var_slots.iter().copied().collect();
        top_vars.sort_by_key(|id| id.0);
        for id in top_vars {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(id, ptr.clone());
            writeln!(self.out, "  {ptr} = alloca double, align 8").ok();
            writeln!(
                self.out,
                "  store double {}, ptr {ptr}",
                undef_double_const()
            )
            .ok();
        }

        for id in &info.user_locals {
            if info.string_locals.contains(id) {
                let ptr = format!("%typeof{}", id.0);
                self.typeof_code_ptrs.insert(*id, ptr.clone());
                writeln!(self.out, "  {ptr} = alloca i32, align 4").ok();
                writeln!(self.out, "  store i32 0, ptr {ptr}").ok();
            } else if !self.allocas.contains_key(id) {
                let ptr = format!("%l{}", id.0);
                self.allocas.insert(*id, ptr.clone());
                writeln!(self.out, "  {ptr} = alloca double, align 8").ok();
            }
        }
        // Annex B if-fn binding slots (top-level).
        let mut slot_ids: Vec<LocalId> = info.if_fn_slots.iter().copied().collect();
        slot_ids.sort_by_key(|id| id.0);
        for id in slot_ids {
            // Only slots whose Function is not nested inside another function body.
            if self.if_fn_slot_owned_by_top(id) {
                let ptr = format!("%iffn{}", id.0);
                self.if_fn_slot_ptrs.insert(id, ptr.clone());
                writeln!(self.out, "  {ptr} = alloca i32, align 4").ok();
                writeln!(self.out, "  store i32 -1, ptr {ptr}").ok();
            }
        }
        // Function-binding locals that are only used as call targets need no storage
        // when statically bound; assigns of FunctionExpr to unused-as-value slots skip.
        // Block-scoped number locals allocate on demand in emit_top_stmt.

        for stmt in &self.module.body {
            self.emit_top_stmt(stmt)?;
        }

        for id in &info.user_locals {
            if info.string_locals.contains(id) {
                let code_ptr = self
                    .typeof_code_ptrs
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("internal: typeof print missing"))?;
                let code = self.fresh();
                writeln!(self.body, "  {code} = load i32, ptr {code_ptr}").ok();
                let is_fn = self.fresh();
                writeln!(self.body, "  {is_fn} = icmp eq i32 {code}, 1").ok();
                let then_l = self.fresh_label("ty_fn");
                let else_l = self.fresh_label("ty_und");
                let end_l = self.fresh_label("ty_end");
                writeln!(
                    self.body,
                    "  br i1 {is_fn}, label %{then_l}, label %{else_l}"
                )
                .ok();
                writeln!(self.body, "{then_l}:").ok();
                self.emit_print_str("function")?;
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{else_l}:").ok();
                self.emit_print_str("undefined")?;
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{end_l}:").ok();
            } else {
                // Number / `var` observations: print "undefined" for the undef sentinel.
                let slot = self.resolve_var_slot(*id);
                let ptr = self
                    .allocas
                    .get(&slot)
                    .cloned()
                    .ok_or_else(|| diag("internal: print missing alloca"))?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                let bits = self.fresh();
                writeln!(self.body, "  {bits} = bitcast double {v} to i64").ok();
                let is_u = self.fresh();
                writeln!(
                    self.body,
                    "  {is_u} = icmp eq i64 {bits}, {UNDEF_BITS}"
                )
                .ok();
                let und_l = self.fresh_label("print_und");
                let num_l = self.fresh_label("print_num");
                let end_l = self.fresh_label("print_end");
                writeln!(
                    self.body,
                    "  br i1 {is_u}, label %{und_l}, label %{num_l}"
                )
                .ok();
                writeln!(self.body, "{und_l}:").ok();
                self.emit_print_str("undefined")?;
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{num_l}:").ok();
                writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{end_l}:").ok();
            }
        }

        // Emit string globals before main definition.
        for (s, gname) in &self.str_globals {
            let n = s.len() + 1;
            let esc = escape_llvm_string(s);
            writeln!(
                prelude,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\""
            )
            .ok();
        }
        if !prelude.is_empty() {
            // Insert globals before `define i32 @main` — rewrite out.
            let main_def = "define i32 @main()";
            if let Some(pos) = self.out.find(main_def) {
                let mut new_out = String::new();
                new_out.push_str(&self.out[..pos]);
                new_out.push_str(&prelude);
                new_out.push('\n');
                new_out.push_str(&self.out[pos..]);
                self.out = new_out;
            }
        }

        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    /// True when `id` is an if-fn primary whose Function stmts appear only at top level
    /// (not nested inside another function). Nested slots are allocated in that function.
    fn if_fn_slot_owned_by_top(&self, id: LocalId) -> bool {
        !self.if_fn_nested_in_any_function(id)
    }

    fn if_fn_nested_in_any_function(&self, id: LocalId) -> bool {
        for f in &self.info.functions {
            if stmt_list_mentions_if_fn(&f.body, id) {
                return true;
            }
        }
        false
    }

    fn emit_print_str(&mut self, s: &str) -> Result<(), Diagnostic> {
        let gname = if let Some(g) = self.str_globals.get(s) {
            g.clone()
        } else {
            let g = format!(".esfn.str.{}", self.str_globals.len());
            self.str_globals.insert(s.to_string(), g.clone());
            g
        };
        let t = self.fresh();
        let n = s.len() + 1;
        writeln!(
            self.body,
            "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
        )
        .ok();
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {t}"))).ok();
        Ok(())
    }

    fn emit_function(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let fn_name = self.fn_names.get(&f.idx).cloned().unwrap();

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_label = self.label;
        let saved_allocas = std::mem::take(&mut self.allocas);
        let saved_rest = std::mem::take(&mut self.rest_slots);
        let saved_if_slots = std::mem::take(&mut self.if_fn_slot_ptrs);

        self.tmp = 0;
        self.label = 0;
        self.allocas.clear();
        self.rest_slots.clear();
        self.if_fn_slot_ptrs.clear();

        let mut sig_parts = Vec::new();
        for (i, _) in f.params.iter().enumerate() {
            sig_parts.push(format!("double %p{i}"));
        }
        if f.rest.is_some() {
            sig_parts.push("ptr %rest_buf".into());
            sig_parts.push("i64 %rest_len".into());
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
        if let Some(rid) = f.rest {
            let buf_slot = format!("%rest_buf_slot{}", rid.0);
            let len_slot = format!("%rest_len_slot{}", rid.0);
            writeln!(entry, "  {buf_slot} = alloca ptr, align 8").ok();
            writeln!(entry, "  {len_slot} = alloca i64, align 8").ok();
            writeln!(entry, "  store ptr %rest_buf, ptr {buf_slot}").ok();
            writeln!(entry, "  store i64 %rest_len, ptr {len_slot}").ok();
            self.rest_slots.insert(rid, (buf_slot, len_slot));
        }
        for (i, cid) in f.captures.iter().enumerate() {
            let ptr = format!("%l{}", cid.0);
            self.allocas.insert(*cid, ptr.clone());
            writeln!(entry, "  {ptr} = alloca double, align 8").ok();
            writeln!(entry, "  store double %c{i}, ptr {ptr}").ok();
        }
        // Hoisted function-scope `var` slots (init undefined).
        if let Some(slots) = self.info.fn_var_slots.get(&f.idx) {
            let mut ids: Vec<LocalId> = slots.iter().copied().collect();
            ids.sort_by_key(|id| id.0);
            for id in ids {
                if self.allocas.contains_key(&id) {
                    continue;
                }
                let ptr = format!("%l{}", id.0);
                self.allocas.insert(id, ptr.clone());
                writeln!(entry, "  {ptr} = alloca double, align 8").ok();
                writeln!(
                    entry,
                    "  store double {}, ptr {ptr}",
                    undef_double_const()
                )
                .ok();
            }
        }
        // Nested Annex B if-fn slots for this function body.
        let mut nested_slots: Vec<LocalId> = self
            .info
            .if_fn_slots
            .iter()
            .copied()
            .filter(|id| stmt_list_mentions_if_fn(&f.body, *id))
            .collect();
        nested_slots.sort_by_key(|id| id.0);
        for id in nested_slots {
            let ptr = format!("%iffn{}", id.0);
            self.if_fn_slot_ptrs.insert(id, ptr.clone());
            writeln!(entry, "  {ptr} = alloca i32, align 4").ok();
            writeln!(entry, "  store i32 -1, ptr {ptr}").ok();
        }

        writeln!(self.out, "define double @{fn_name}({sig}) {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{entry}").ok();

        // Apply defaults left-to-right when arg is missing/undefined sentinel.
        let defaults = f.defaults.clone();
        let param_ids = f.params.clone();
        for (i, pid) in param_ids.iter().enumerate() {
            if let Some(def) = &defaults[i] {
                self.emit_param_default(*pid, def)?;
            }
        }

        for stmt in &f.body {
            self.emit_fn_stmt(stmt)?;
        }
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
        }

        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.label = saved_label;
        self.allocas = saved_allocas;
        self.rest_slots = saved_rest;
        self.if_fn_slot_ptrs = saved_if_slots;
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
            Stmt::Declare { local, init, kind } => {
                if self.info.fn_binding.contains_key(local) {
                    // Function binding — no number storage required for static calls.
                    return Ok(());
                }
                if self.info.string_locals.contains(local) {
                    let init = init
                        .as_ref()
                        .ok_or_else(|| diag("es_functions: typeof declare requires init"))?;
                    return self.emit_typeof_declare(*local, init);
                }
                // `var` is hoisted to entry as undefined; bare `var x` is a no-op store.
                let is_var = *kind == BindingKind::Var || self.info.var_primary.contains_key(local);
                let slot = self.resolve_var_slot(*local);
                if is_var {
                    let ptr = self
                        .allocas
                        .get(&slot)
                        .cloned()
                        .ok_or_else(|| diag("es_functions: var slot missing alloca"))?;
                    if let Some(init) = init.as_ref() {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    return Ok(());
                }
                let init = init
                    .as_ref()
                    .ok_or_else(|| diag("es_functions: declare requires init"))?;
                let ptr = if let Some(p) = self.allocas.get(&slot).cloned() {
                    p
                } else {
                    let p = format!("%l{}", slot.0);
                    self.allocas.insert(slot, p.clone());
                    writeln!(self.body, "  {p} = alloca double, align 8").ok();
                    p
                };
                let v = self.emit_number_expr(init)?;
                writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                Ok(())
            }
            Stmt::Function { local, .. } => self.emit_if_fn_activate(*local),
            Stmt::Labeled { body, .. } => self.emit_top_stmt(body),
            Stmt::Block { body } => {
                for s in body {
                    self.emit_top_stmt(s)?;
                }
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => self.emit_if_stmt(test, consequent, alternate, true),
            Stmt::Expr { expr } => match expr {
                Expr::Assign {
                    target: AssignTarget::Local(id),
                    op: AssignOp::Eq,
                    value,
                    ..
                } => {
                    let slot = self.resolve_var_slot(*id);
                    let ptr = self
                        .allocas
                        .get(&slot)
                        .cloned()
                        .ok_or_else(|| diag("es_functions: top assign missing alloca"))?;
                    let v = self.emit_number_expr(value)?;
                    writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    Ok(())
                }
                _ => Err(diag("es_functions: unsupported top-level expr stmt")),
            },
            _ => Err(diag("es_functions: unsupported top-level stmt")),
        }
    }

    fn emit_typeof_declare(&mut self, local: LocalId, init: &Expr) -> Result<(), Diagnostic> {
        let Expr::Unary {
            op: draconic_ast::UnaryOp::TypeOf,
            arg,
            ..
        } = init
        else {
            return Err(diag("es_functions: string local must be typeof"));
        };
        let Expr::Local { id, .. } = arg.as_ref() else {
            return Err(diag("es_functions: typeof arg must be local"));
        };
        let code_ptr = self
            .typeof_code_ptrs
            .get(&local)
            .cloned()
            .ok_or_else(|| diag("es_functions: typeof code slot missing"))?;
        let primary = self
            .info
            .if_fn_primary
            .get(id)
            .copied()
            .unwrap_or(*id);
        if let Some(slot) = self.if_fn_slot_ptrs.get(&primary).cloned() {
            let idx = self.fresh();
            writeln!(self.body, "  {idx} = load i32, ptr {slot}").ok();
            let bound = self.fresh();
            writeln!(self.body, "  {bound} = icmp ne i32 {idx}, -1").ok();
            let t = self.fresh();
            writeln!(self.body, "  {t} = zext i1 {bound} to i32").ok();
            writeln!(self.body, "  store i32 {t}, ptr {code_ptr}").ok();
        } else if self.info.fn_binding.contains_key(id) {
            // Always-bound function decl.
            writeln!(self.body, "  store i32 1, ptr {code_ptr}").ok();
        } else {
            // Unbound / hoisted-uninit `var` typeof → "undefined" (code 0).
            // Number typeof string obs is out of scope for this path's table.
            writeln!(self.body, "  store i32 0, ptr {code_ptr}").ok();
        }
        Ok(())
    }

    /// Activate Annex B if-clause function: store its fn idx into the primary slot.
    fn emit_if_fn_activate(&mut self, local: LocalId) -> Result<(), Diagnostic> {
        let Some(primary) = self.info.if_fn_primary.get(&local).copied() else {
            // Ordinary function decl (not if-clause) — always available via fn_binding.
            return Ok(());
        };
        let Some(&idx) = self.info.fn_binding.get(&local) else {
            return Ok(());
        };
        let Some(slot) = self.if_fn_slot_ptrs.get(&primary).cloned() else {
            return Err(diag(format!(
                "es_functions: if-fn slot missing for %{}",
                primary.0
            )));
        };
        writeln!(self.body, "  store i32 {idx}, ptr {slot}").ok();
        Ok(())
    }

    fn emit_if_stmt(
        &mut self,
        test: &Expr,
        consequent: &Stmt,
        alternate: &Option<Box<Stmt>>,
        top: bool,
    ) -> Result<(), Diagnostic> {
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
        if top {
            self.emit_top_stmt(consequent)?;
        } else {
            self.emit_fn_stmt(consequent)?;
        }
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  br label %{end_l}").ok();
        }
        if let Some(alt) = alternate {
            writeln!(self.body, "{else_l}:").ok();
            if top {
                self.emit_top_stmt(alt)?;
            } else {
                self.emit_fn_stmt(alt)?;
            }
            if !self.body_ends_with_terminator() {
                writeln!(self.body, "  br label %{end_l}").ok();
            }
        }
        writeln!(self.body, "{end_l}:").ok();
        Ok(())
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
            Stmt::Declare { local, init, kind } => {
                if self.info.fn_binding.contains_key(local) {
                    return Ok(());
                }
                // Hoisted `var`: store init into primary (bare `var` already undef at entry).
                let is_var = *kind == BindingKind::Var || self.info.var_primary.contains_key(local);
                if is_var {
                    let slot = self.resolve_var_slot(*local);
                    let ptr = self
                        .allocas
                        .get(&slot)
                        .cloned()
                        .ok_or_else(|| diag("es_functions: fn var slot missing alloca"))?;
                    if let Some(e) = init {
                        if !matches!(e, Expr::Function { .. }) {
                            let v = self.emit_number_expr(e)?;
                            writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                        }
                    }
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
            Stmt::Function { local, .. } => self.emit_if_fn_activate(*local),
            Stmt::Labeled { body, .. } => self.emit_fn_stmt(body),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => self.emit_if_stmt(test, consequent, alternate, false),
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
            } => {
                if *is_await {
                    return Err(diag("es_functions: for-await-of not supported"));
                }
                self.emit_for_of_rest(left, right, body)
            }
            Stmt::Expr { expr } => match expr {
                Expr::Assign {
                    target: AssignTarget::Local(id),
                    op: AssignOp::Eq,
                    value,
                    ..
                } => {
                    let slot = self.resolve_var_slot(*id);
                    let ptr = self
                        .allocas
                        .get(&slot)
                        .cloned()
                        .ok_or_else(|| diag("es_functions: assign missing alloca"))?;
                    let v = self.emit_number_expr(value)?;
                    writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    Ok(())
                }
                _ => Err(diag("es_functions: unsupported expr stmt")),
            },
            _ => Err(diag("es_functions: unsupported stmt in function body")),
        }
    }

    fn emit_for_of_rest(
        &mut self,
        left: &Stmt,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), Diagnostic> {
        let Expr::Local { id: rest_id, .. } = right else {
            return Err(diag("es_functions: for-of right must be rest local"));
        };
        let (buf_slot, len_slot) = self
            .rest_slots
            .get(rest_id)
            .cloned()
            .ok_or_else(|| diag("es_functions: for-of rest slot missing"))?;
        let Stmt::Declare {
            local: bind_id,
            init: None,
            ..
        } = left
        else {
            return Err(diag("es_functions: for-of left must be bare let binding"));
        };
        let bind_ptr = format!("%l{}", bind_id.0);
        self.allocas.insert(*bind_id, bind_ptr.clone());
        writeln!(self.body, "  {bind_ptr} = alloca double, align 8").ok();

        let buf = self.fresh();
        let len = self.fresh();
        writeln!(self.body, "  {buf} = load ptr, ptr {buf_slot}").ok();
        writeln!(self.body, "  {len} = load i64, ptr {len_slot}").ok();
        let idx_ptr = self.fresh();
        writeln!(self.body, "  {idx_ptr} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {idx_ptr}").ok();

        let head = self.fresh_label("forof_head");
        let bod = self.fresh_label("forof_body");
        let cont = self.fresh_label("forof_cont");
        let end = self.fresh_label("forof_end");
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{head}:").ok();
        let idx = self.fresh();
        writeln!(self.body, "  {idx} = load i64, ptr {idx_ptr}").ok();
        let cmp = self.fresh();
        writeln!(self.body, "  {cmp} = icmp ult i64 {idx}, {len}").ok();
        writeln!(self.body, "  br i1 {cmp}, label %{bod}, label %{end}").ok();
        writeln!(self.body, "{bod}:").ok();
        let gep = self.fresh();
        writeln!(
            self.body,
            "  {gep} = getelementptr inbounds double, ptr {buf}, i64 {idx}"
        )
        .ok();
        let elem = self.fresh();
        writeln!(self.body, "  {elem} = load double, ptr {gep}").ok();
        writeln!(self.body, "  store double {elem}, ptr {bind_ptr}").ok();
        self.emit_fn_stmt(body)?;
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  br label %{cont}").ok();
        }
        writeln!(self.body, "{cont}:").ok();
        let idx2 = self.fresh();
        writeln!(self.body, "  {idx2} = load i64, ptr {idx_ptr}").ok();
        let next = self.fresh();
        writeln!(self.body, "  {next} = add i64 {idx2}, 1").ok();
        writeln!(self.body, "  store i64 {next}, ptr {idx_ptr}").ok();
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{end}:").ok();
        Ok(())
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

    fn emit_param_default(&mut self, pid: LocalId, def: &Expr) -> Result<(), Diagnostic> {
        let ptr = self
            .allocas
            .get(&pid)
            .cloned()
            .ok_or_else(|| diag("es_functions: default param missing alloca"))?;
        let cur = self.fresh();
        writeln!(self.body, "  {cur} = load double, ptr {ptr}").ok();
        let bits = self.fresh();
        writeln!(self.body, "  {bits} = bitcast double {cur} to i64").ok();
        let is_u = self.fresh();
        writeln!(
            self.body,
            "  {is_u} = icmp eq i64 {bits}, {UNDEF_BITS}"
        )
        .ok();
        let then_l = self.fresh_label("def");
        let end_l = self.fresh_label("defend");
        writeln!(
            self.body,
            "  br i1 {is_u}, label %{then_l}, label %{end_l}"
        )
        .ok();
        writeln!(self.body, "{then_l}:").ok();
        let v = self.emit_number_expr(def)?;
        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
        writeln!(self.body, "  br label %{end_l}").ok();
        writeln!(self.body, "{end_l}:").ok();
        Ok(())
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Boolean { value, .. } => {
                let t = self.fresh();
                let bit = if *value { 1 } else { 0 };
                writeln!(self.body, "  {t} = add i1 0, {bit}").ok();
                Ok(t)
            }
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
                let slot = self.resolve_var_slot(*id);
                let ptr = self
                    .allocas
                    .get(&slot)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated local %{}", slot.0)))?;
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
            Expr::Unary {
                op: draconic_ast::UnaryOp::Void,
                ..
            } => Ok(undef_double_const()),
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
                let primary = self
                    .info
                    .if_fn_primary
                    .get(id)
                    .copied()
                    .or_else(|| {
                        if self.info.if_fn_slots.contains(id) {
                            Some(*id)
                        } else {
                            None
                        }
                    });
                if let Some(primary) = primary {
                    if self.if_fn_slot_ptrs.contains_key(&primary) {
                        return self.emit_dynamic_if_fn_call(primary, &arg_vals);
                    }
                }
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
                    // Pad defaults / pack rest, then load captures from return buffer.
                    let f = &self.info.functions[idx];
                    if !call_arity_ok(f, arg_vals.len()) {
                        return Err(diag("es_functions: higher-order call arity mismatch"));
                    }
                    let mut caps = Vec::new();
                    for i in 0..f.captures.len() {
                        let gep = self.fresh();
                        writeln!(
                            self.body,
                            "  {gep} = getelementptr inbounds [{MAX_CAPS} x double], ptr @es_ret_cap, i64 0, i64 {i}"
                        )
                        .ok();
                        let c = self.fresh();
                        writeln!(self.body, "  {c} = load double, ptr {gep}").ok();
                        caps.push(c);
                    }
                    self.emit_call_args(idx, &arg_vals, &caps)
                }
            _ => Err(diag("es_functions: unsupported call callee")),
        }
    }

    fn emit_direct_call(&mut self, idx: usize, arg_vals: &[String]) -> Result<String, Diagnostic> {
        let f = &self.info.functions[idx];
        if !call_arity_ok(f, arg_vals.len()) {
            return Err(diag("es_functions: call arity mismatch"));
        }
        let mut caps = Vec::new();
        for cid in &f.captures.clone() {
            let ptr = self.allocas.get(cid).cloned().ok_or_else(|| {
                diag(format!(
                    "es_functions: capture local %{} not in caller frame",
                    cid.0
                ))
            })?;
            let t = self.fresh();
            writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
            caps.push(t);
        }
        self.emit_call_args(idx, arg_vals, &caps)
    }

    /// Call through Annex B if-fn slot (i32 idx, -1 = unbound).
    fn emit_dynamic_if_fn_call(
        &mut self,
        primary: LocalId,
        arg_vals: &[String],
    ) -> Result<String, Diagnostic> {
        let slot = self
            .if_fn_slot_ptrs
            .get(&primary)
            .cloned()
            .ok_or_else(|| diag("es_functions: dynamic call missing slot"))?;
        let candidates = self
            .info
            .if_fn_candidates
            .get(&primary)
            .cloned()
            .unwrap_or_default();
        if candidates.is_empty() {
            return Err(diag("es_functions: dynamic call has no candidates"));
        }
        // Precompute captures per candidate (same frame).
        let idx_v = self.fresh();
        writeln!(self.body, "  {idx_v} = load i32, ptr {slot}").ok();
        let end_l = self.fresh_label("dyn_end");
        let bad_l = self.fresh_label("dyn_bad");
        let mut case_labels = Vec::new();
        for &cidx in &candidates {
            case_labels.push((cidx, self.fresh_label(&format!("dyn_c{cidx}"))));
        }
        // switch
        let mut sw = format!("  switch i32 {idx_v}, label %{bad_l} [");
        for (cidx, lab) in &case_labels {
            sw.push_str(&format!(" i32 {cidx}, label %{lab}"));
        }
        sw.push_str(" ]");
        writeln!(self.body, "{sw}").ok();

        let mut phi_pairs = Vec::new();
        for (cidx, lab) in &case_labels {
            writeln!(self.body, "{lab}:").ok();
            let ret = self.emit_direct_call(*cidx, arg_vals)?;
            phi_pairs.push((ret, lab.clone()));
            writeln!(self.body, "  br label %{end_l}").ok();
        }
        writeln!(self.body, "{bad_l}:").ok();
        // Unbound / bad idx — return 0 (should not be observed in fixtures).
        let bad_ret = "0.00000000000000000e+00".to_string();
        writeln!(self.body, "  br label %{end_l}").ok();
        writeln!(self.body, "{end_l}:").ok();
        let phi = self.fresh();
        let mut phi_src = String::from(&format!("  {phi} = phi double "));
        let mut first = true;
        for (ret, lab) in &phi_pairs {
            if !first {
                phi_src.push_str(", ");
            }
            first = false;
            phi_src.push_str(&format!("[ {ret}, %{lab} ]"));
        }
        if !first {
            phi_src.push_str(", ");
        }
        phi_src.push_str(&format!("[ {bad_ret}, %{bad_l} ]"));
        writeln!(self.body, "{phi_src}").ok();
        Ok(phi)
    }

    /// Build fixed params (pad defaults), optional rest buffer, then captures.
    fn emit_call_args(
        &mut self,
        idx: usize,
        arg_vals: &[String],
        caps: &[String],
    ) -> Result<String, Diagnostic> {
        let f = &self.info.functions[idx];
        let n_fixed = f.params.len();
        let undef = undef_double_const();
        let mut fixed: Vec<String> = arg_vals.iter().take(n_fixed).cloned().collect();
        while fixed.len() < n_fixed {
            fixed.push(undef.clone());
        }

        let mut call_parts: Vec<String> = fixed.iter().map(|v| format!("double {v}")).collect();

        if f.rest.is_some() {
            let rest_vals: Vec<&String> = arg_vals.iter().skip(n_fixed).collect();
            let rest_len = rest_vals.len();
            if rest_len > MAX_REST {
                return Err(diag("es_functions: too many rest args"));
            }
            let buf = self.fresh();
            writeln!(
                self.body,
                "  {buf} = alloca [{MAX_REST} x double], align 8"
            )
            .ok();
            for (i, v) in rest_vals.iter().enumerate() {
                let gep = self.fresh();
                writeln!(
                    self.body,
                    "  {gep} = getelementptr inbounds [{MAX_REST} x double], ptr {buf}, i64 0, i64 {i}"
                )
                .ok();
                writeln!(self.body, "  store double {v}, ptr {gep}").ok();
            }
            let buf_ptr = self.fresh();
            writeln!(
                self.body,
                "  {buf_ptr} = getelementptr inbounds [{MAX_REST} x double], ptr {buf}, i64 0, i64 0"
            )
            .ok();
            call_parts.push(format!("ptr {buf_ptr}"));
            call_parts.push(format!("i64 {rest_len}"));
        }

        for c in caps {
            call_parts.push(format!("double {c}"));
        }

        let fn_name = self.fn_names.get(&idx).cloned().unwrap();
        let t = self.fresh();
        if call_parts.is_empty() {
            writeln!(self.body, "  {t} = call double @{fn_name}()").ok();
        } else {
            writeln!(
                self.body,
                "  {t} = call double @{fn_name}({})",
                call_parts.join(", ")
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

/// Whether `body` contains an if-clause Function for primary/local `id`.
fn stmt_list_mentions_if_fn(body: &[Stmt], id: LocalId) -> bool {
    for stmt in body {
        if stmt_mentions_if_fn(stmt, id) {
            return true;
        }
    }
    false
}

fn stmt_mentions_if_fn(stmt: &Stmt, id: LocalId) -> bool {
    match stmt {
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            if unwrap_if_fn_local(consequent) == Some(id)
                || alternate
                    .as_ref()
                    .and_then(|a| unwrap_if_fn_local(a))
                    == Some(id)
            {
                return true;
            }
            // Primary may be consequent while else has different local aliased to primary.
            if let Some(cl) = unwrap_if_fn_local(consequent) {
                if cl == id {
                    return true;
                }
            }
            if let Some(alt) = alternate {
                if let Some(al) = unwrap_if_fn_local(alt) {
                    // Caller checks primary set membership separately; match either local.
                    if al == id {
                        return true;
                    }
                }
                if stmt_mentions_if_fn(alt, id) {
                    return true;
                }
            }
            stmt_mentions_if_fn(consequent, id)
        }
        Stmt::Block { body } => stmt_list_mentions_if_fn(body, id),
        Stmt::Labeled { body, .. } => stmt_mentions_if_fn(body, id),
        Stmt::Function { body, local, .. } => *local == id || stmt_list_mentions_if_fn(body, id),
        _ => false,
    }
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
