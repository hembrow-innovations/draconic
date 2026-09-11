use super::collect::collect_all_functions;
use super::ok::{bool_expr_ok, fn_body_ok, number_expr_ok};
use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
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
    let mut obj_methods = HashMap::new();
    record_obj_methods(&module.body, &functions, &mut obj_methods);
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
            &obj_methods,
            &rest_locals,
        ) {
            return None;
        }
        for e in f.defaults.iter().flatten() {
            if !number_expr_ok(
                e,
                &by_id,
                &fn_arities,
                &functions,
                &fn_binding,
                &obj_methods,
            ) {
                return None;
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
            &obj_methods,
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
        obj_methods,
    })
}

/// Collect `{ m: function… }` static methods on object-literal declares.
fn record_obj_methods(
    stmts: &[Stmt],
    functions: &[FnInfo],
    out: &mut HashMap<LocalId, HashMap<String, usize>>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Declare {
                local,
                init: Some(Expr::Object { properties, .. }),
                ..
            } => {
                let mut map = HashMap::new();
                for p in properties {
                    if let ObjectProp::Property {
                        key: ObjectPropKey::Static(name),
                        value: Expr::Function { params, .. },
                    } = p
                    {
                        if let Some(idx) = find_fn_idx_by_param_patterns(params, functions) {
                            map.insert(name.to_string_lossy(), idx);
                        }
                    }
                }
                if !map.is_empty() {
                    out.insert(*local, map);
                }
            }
            Stmt::Block { body } => record_obj_methods(body, functions, out),
            Stmt::Labeled { body, .. } => {
                record_obj_methods(std::slice::from_ref(body), functions, out)
            }
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                record_obj_methods(std::slice::from_ref(consequent), functions, out);
                if let Some(a) = alternate {
                    record_obj_methods(std::slice::from_ref(a), functions, out);
                }
            }
            _ => {}
        }
    }
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
                        let same = by_id
                            .get(&cl)
                            .is_some_and(|l| by_id.get(&al).is_some_and(|r| l.name == r.name));
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
    obj_methods: &HashMap<LocalId, HashMap<String, usize>>,
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
            obj_methods,
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
                obj_methods,
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
            bool_expr_ok(test, by_id, fn_arities, functions, fn_binding, obj_methods)
                && classify_top_stmt(
                    consequent,
                    by_id,
                    fn_arities,
                    functions,
                    fn_binding,
                    if_fn_primary,
                    if_fn_slots,
                    var_primary,
                    obj_methods,
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
                        obj_methods,
                        has_fn,
                        user_locals,
                        string_locals,
                        observed,
                        false,
                    )
                })
        }
        Stmt::Declare { local, init, kind } => {
            if init
                .as_ref()
                .is_some_and(|e| crate::es_console::is_global_this_console(e, by_id))
            {
                return true;
            }
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
                        if !number_expr_ok(
                            init,
                            by_id,
                            fn_arities,
                            functions,
                            fn_binding,
                            obj_methods,
                        ) {
                            // Call of if-clause function uses dynamic slot — still ok if callee is if-fn.
                            if !call_if_fn_ok(
                                init,
                                by_id,
                                fn_arities,
                                functions,
                                fn_binding,
                                if_fn_slots,
                                obj_methods,
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
                    if observe_declares && observed.insert(*local) {
                        user_locals.push(*local);
                        string_locals.insert(*local);
                    }
                    true
                }
                Type::Function => {
                    let Some(init) = init.as_ref() else {
                        return false;
                    };
                    matches!(init, Expr::Function { .. })
                }
                Type::Shape(_) | Type::Object => {
                    // Object literal holding only static method functions (not observed).
                    obj_methods.contains_key(local)
                        && init
                            .as_ref()
                            .is_some_and(|e| matches!(e, Expr::Object { .. }))
                }
                _ => false,
            }
        }
        Stmt::Expr { expr } => {
            if crate::es_console::console_log_string_arg(expr, by_id).is_some() {
                return true;
            }
            match expr {
                Expr::Assign {
                    target: AssignTarget::Local(_),
                    op: AssignOp::Eq,
                    value,
                    ..
                } => number_expr_ok(value, by_id, fn_arities, functions, fn_binding, obj_methods),
                _ => false,
            }
        }
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
    let id = match arg.as_ref() {
        Expr::Local { id, .. } => *id,
        Expr::IdentName { name, .. } => {
            let Some(id) = fn_id_for_name(name, by_id, fn_binding).or_else(|| {
                by_id
                    .iter()
                    .filter(|(id, loc)| {
                        loc.name == *name
                            && (if_fn_slots.contains(id)
                                || if_fn_primary.contains_key(id)
                                || var_primary.contains_key(id))
                    })
                    .map(|(id, _)| *id)
                    .max_by_key(|id| id.0)
            }) else {
                return false;
            };
            id
        }
        _ => return false,
    };
    if if_fn_slots.contains(&id) || if_fn_primary.contains_key(&id) || fn_binding.contains_key(&id)
    {
        return true;
    }
    // `typeof` of a number/any local (incl. hoisted `var` that may be undefined).
    if var_primary.contains_key(&id) {
        return true;
    }
    by_id
        .get(&id)
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
    obj_methods: &HashMap<LocalId, HashMap<String, usize>>,
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
        Arg::Expr(e) => number_expr_ok(e, by_id, fn_arities, functions, fn_binding, obj_methods),
        Arg::Spread(_) => false,
    }) {
        return false;
    }
    let Some(id) = callee_fn_id(callee, by_id, fn_binding).or_else(|| match callee.as_ref() {
        Expr::Local { id, .. } => Some(*id),
        Expr::IdentName { name, .. } => by_id
            .iter()
            .filter(|(id, loc)| loc.name == *name && if_fn_slots.contains(id))
            .map(|(id, _)| *id)
            .max_by_key(|id| id.0),
        _ => None,
    }) else {
        return false;
    };
    if !if_fn_slots.contains(&id) && !fn_binding.contains_key(&id) {
        return false;
    }
    // Arity: use any candidate with matching fixed arity, or static binding.
    if let Some(&idx) = fn_binding.get(&id) {
        return call_arity_ok(&functions[idx], args.len());
    }
    true
}
