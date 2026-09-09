use super::*;

pub(super) fn collect_all_functions(
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
        Expr::Member {
            object, property, ..
        } => {
            collect_expr_fns(object, by_id, out, fn_binding)?;
            collect_expr_fns(property, by_id, out, fn_binding)
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                        collect_expr_fns(value, by_id, out, fn_binding)?;
                    }
                    ObjectProp::Spread(e) => collect_expr_fns(e, by_id, out, fn_binding)?,
                }
            }
            Some(())
        }
        _ => Some(()),
    }
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
    let arguments = find_arguments_local(body, by_id);
    let mut bound_ext = bound.clone();
    if let Some(a) = arguments {
        bound_ext.insert(a);
    }
    let mut free = HashSet::new();
    collect_free_in_body(body, &bound_ext, by_id, &mut free);
    collect_ident_free_in_body(body, &bound_ext, by_id, &mut free);
    for e in defaults.iter().flatten() {
        collect_free_in_expr(e, &bound_ext, by_id, &mut free);
        collect_ident_free_in_expr(e, &bound_ext, by_id, &mut free);
    }
    // Nested free through nested Function decls/exprs already in body free collection
    // for exprs; nested Stmt::Function free handled via collect_free that skips nested
    // function bodies — re-walk nested decls:
    for stmt in body {
        collect_nested_free_through(stmt, &bound_ext, by_id, &mut free)?;
    }
    // Never capture the implicit `arguments` object.
    free.retain(|id| by_id.get(id).map(|l| l.name.as_str()) != Some("arguments"));
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
        arguments,
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
            Stmt::Labeled { body, .. } => collect_bound_in_body(std::slice::from_ref(body), bound),
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

fn collect_free_in_body(
    body: &[Stmt],
    bound: &HashSet<LocalId>,
    by_id: &HashMap<LocalId, &Local>,
    free: &mut HashSet<LocalId>,
) {
    for stmt in body {
        match stmt {
            Stmt::Return { value: Some(v) } => collect_free_in_expr(v, bound, by_id, free),
            Stmt::Declare { init, .. } => {
                if let Some(e) = init {
                    collect_free_in_expr(e, bound, by_id, free);
                }
            }
            Stmt::Block { body } => collect_free_in_body(body, bound, by_id, free),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                collect_free_in_expr(test, bound, by_id, free);
                collect_free_in_body(std::slice::from_ref(consequent), bound, by_id, free);
                if let Some(a) = alternate {
                    collect_free_in_body(std::slice::from_ref(a), bound, by_id, free);
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
                collect_free_in_expr(right, bound, by_id, free);
                collect_free_in_body(std::slice::from_ref(left), bound, by_id, free);
                collect_free_in_body(std::slice::from_ref(body), bound, by_id, free);
            }
            Stmt::Expr { expr } => collect_free_in_expr(expr, bound, by_id, free),
            Stmt::Function { .. } => {}
            Stmt::Labeled { body, .. } => {
                collect_free_in_body(std::slice::from_ref(body), bound, by_id, free)
            }
            _ => {}
        }
    }
}

fn collect_free_in_expr(
    expr: &Expr,
    bound: &HashSet<LocalId>,
    by_id: &HashMap<LocalId, &Local>,
    free: &mut HashSet<LocalId>,
) {
    match expr {
        Expr::Local { id, .. } => {
            if !bound.contains(id) {
                free.insert(*id);
            }
        }
        Expr::Unary { arg, .. } => collect_free_in_expr(arg, bound, by_id, free),
        Expr::Binary { left, right, .. } => {
            collect_free_in_expr(left, bound, by_id, free);
            collect_free_in_expr(right, bound, by_id, free);
        }
        Expr::Assign { value, .. } => collect_free_in_expr(value, bound, by_id, free),
        Expr::Call { callee, args, .. } => {
            collect_free_in_expr(callee, bound, by_id, free);
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_free_in_expr(e, bound, by_id, free);
                }
            }
        }
        Expr::Member {
            object, property, ..
        } => {
            collect_free_in_expr(object, bound, by_id, free);
            collect_free_in_expr(property, bound, by_id, free);
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                        collect_free_in_expr(value, bound, by_id, free);
                    }
                    ObjectProp::Spread(e) => collect_free_in_expr(e, bound, by_id, free),
                }
            }
        }
        Expr::Function {
            name, params, body, ..
        } => {
            let fixed: Vec<LocalId> = params
                .iter()
                .filter(|p| !p.rest)
                .filter_map(|p| match &p.pattern {
                    Pattern::Local(id) => Some(*id),
                    _ => None,
                })
                .collect();
            let rest = params
                .iter()
                .find(|p| p.rest)
                .and_then(|p| match &p.pattern {
                    Pattern::Local(id) => Some(*id),
                    _ => None,
                });
            let mut nested_bound = bound_in_fn(&fixed, rest, body);
            if let Some(n) = name {
                nested_bound.insert(*n);
            }
            let mut nested_free = HashSet::new();
            collect_free_in_body(body, &nested_bound, by_id, &mut nested_free);
            collect_ident_free_in_body(body, &nested_bound, by_id, &mut nested_free);
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
            collect_free_in_body(body, &nested_bound, by_id, &mut nested_free);
            collect_ident_free_in_body(body, &nested_bound, by_id, &mut nested_free);
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
fn collect_ident_free_in_body(
    body: &[Stmt],
    bound: &HashSet<LocalId>,
    by_id: &HashMap<LocalId, &Local>,
    free: &mut HashSet<LocalId>,
) {
    for stmt in body {
        collect_ident_free_in_stmt(stmt, bound, by_id, free);
    }
}

fn collect_ident_free_in_stmt(
    stmt: &Stmt,
    bound: &HashSet<LocalId>,
    by_id: &HashMap<LocalId, &Local>,
    free: &mut HashSet<LocalId>,
) {
    match stmt {
        Stmt::Return { value: Some(v) } => collect_ident_free_in_expr(v, bound, by_id, free),
        Stmt::Declare { init: Some(e), .. } => collect_ident_free_in_expr(e, bound, by_id, free),
        Stmt::Expr { expr } => collect_ident_free_in_expr(expr, bound, by_id, free),
        Stmt::Block { body } => collect_ident_free_in_body(body, bound, by_id, free),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            collect_ident_free_in_expr(test, bound, by_id, free);
            collect_ident_free_in_stmt(consequent, bound, by_id, free);
            if let Some(a) = alternate {
                collect_ident_free_in_stmt(a, bound, by_id, free);
            }
        }
        Stmt::Labeled { body, .. } => collect_ident_free_in_stmt(body, bound, by_id, free),
        Stmt::Function { .. } => {}
        _ => {}
    }
}

fn collect_ident_free_in_expr(
    expr: &Expr,
    bound: &HashSet<LocalId>,
    by_id: &HashMap<LocalId, &Local>,
    free: &mut HashSet<LocalId>,
) {
    match expr {
        Expr::IdentName { name, .. } => {
            if let Some(id) = number_id_named(name, by_id) {
                if !bound.contains(&id) {
                    free.insert(id);
                }
            }
        }
        Expr::Unary { arg, .. } => collect_ident_free_in_expr(arg, bound, by_id, free),
        Expr::Binary { left, right, .. } => {
            collect_ident_free_in_expr(left, bound, by_id, free);
            collect_ident_free_in_expr(right, bound, by_id, free);
        }
        Expr::Assign { value, .. } => collect_ident_free_in_expr(value, bound, by_id, free),
        Expr::Call { callee, args, .. } => {
            collect_ident_free_in_expr(callee, bound, by_id, free);
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_ident_free_in_expr(e, bound, by_id, free);
                }
            }
        }
        Expr::Member {
            object, property, ..
        } => {
            collect_ident_free_in_expr(object, bound, by_id, free);
            collect_ident_free_in_expr(property, bound, by_id, free);
        }
        _ => {}
    }
}
