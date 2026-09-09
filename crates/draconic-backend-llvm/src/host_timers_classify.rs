use super::*;

pub(super) fn try_classify(module: &Module) -> Result<ModuleInfo, String> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_ids = HashSet::new();
    collect_top_level_decl_ids(&module.body, &mut user_ids);

    let mut user_locals = Vec::new();
    let mut seen = HashSet::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, init, .. } = stmt {
            if !seen.insert(*local) {
                continue;
            }
            if !user_ids.contains(local) {
                continue;
            }
            let kind = if let Some(e) = init {
                if let Some(k) = kind_from_init(e) {
                    k
                } else {
                    match by_id.get(local).map(|l| &l.ty) {
                        Some(Type::Boolean) => SlotKind::Bool,
                        Some(Type::String) => SlotKind::String,
                        _ => SlotKind::Number,
                    }
                }
            } else {
                match by_id.get(local).map(|l| &l.ty) {
                    Some(Type::Boolean) => SlotKind::Bool,
                    Some(Type::String) => SlotKind::String,
                    _ => SlotKind::Number,
                }
            };
            user_locals.push((*local, kind));
        }
    }

    let mut uses_timer = false;
    for stmt in &module.body {
        check_stmt(stmt, &mut uses_timer)?;
    }

    Ok(ModuleInfo {
        uses_timer,
        user_locals,
    })
}

fn kind_from_init(expr: &Expr) -> Option<SlotKind> {
    match expr {
        Expr::Unary {
            op: UnaryOp::TypeOf,
            ..
        } => Some(SlotKind::String),
        Expr::Binary {
            op:
                BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq,
            ..
        } => Some(SlotKind::Bool),
        Expr::Call { callee, args, .. } if args.is_empty() && is_clock_callee(callee) => {
            Some(SlotKind::Number)
        }
        Expr::Call { callee, .. }
            if is_named_callee(callee, "setTimeout") || is_named_callee(callee, "setInterval") =>
        {
            Some(SlotKind::Number)
        }
        Expr::Number { .. } => Some(SlotKind::Number),
        Expr::Boolean { .. } => Some(SlotKind::Bool),
        Expr::String { .. } => Some(SlotKind::String),
        _ => None,
    }
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

fn check_stmt(stmt: &Stmt, uses: &mut bool) -> Result<(), String> {
    match stmt {
        Stmt::Declare { init, .. } => {
            if let Some(e) = init {
                check_expr(e, uses)?;
            }
            Ok(())
        }
        Stmt::Expr { expr } => check_expr(expr, uses),
        Stmt::Block { body } => {
            for s in body {
                check_stmt(s, uses)?;
            }
            Ok(())
        }
        _ => Err("unsupported stmt in host_timer module".into()),
    }
}

fn is_timer_set_name(name: &str) -> bool {
    name == "setTimeout" || name == "setInterval"
}

fn is_timer_clear_name(name: &str) -> bool {
    name == "clearTimeout" || name == "clearInterval"
}

pub(super) fn is_timer_api_name(name: &str) -> bool {
    is_timer_set_name(name) || is_timer_clear_name(name)
}

fn check_expr(expr: &Expr, uses: &mut bool) -> Result<(), String> {
    match expr {
        Expr::Call { callee, args, .. }
            if is_named_callee(callee, "setTimeout") || is_named_callee(callee, "setInterval") =>
        {
            *uses = true;
            let name = if is_named_callee(callee, "setInterval") {
                "setInterval"
            } else {
                "setTimeout"
            };
            if args.len() != 2 {
                return Err(format!("{name} expects (fn, delay)"));
            }
            let fn_expr = arg_expr(&args[0])?;
            match fn_expr {
                Expr::Function {
                    params,
                    body,
                    is_async,
                    is_generator,
                    ..
                } => {
                    if *is_async || *is_generator {
                        return Err("async/generator timer callback unsupported".into());
                    }
                    if !params.is_empty() {
                        return Err("timer callback params unsupported".into());
                    }
                    for s in body {
                        check_timer_body_stmt(s, uses)?;
                    }
                }
                _ => return Err(format!("{name} callback must be function expression")),
            }
            check_expr(arg_expr(&args[1])?, uses)
        }
        Expr::Call { callee, args, .. }
            if is_named_callee(callee, "clearTimeout")
                || is_named_callee(callee, "clearInterval") =>
        {
            *uses = true;
            if args.len() != 1 {
                return Err("clearTimeout/clearInterval expects (id)".into());
            }
            check_expr(arg_expr(&args[0])?, uses)
        }
        Expr::Call { callee, args, .. } if is_clock_callee(callee) => {
            if !args.is_empty() {
                return Err("nowMs/monotonicMs/Date.now expects no args".into());
            }
            Ok(())
        }
        Expr::Call { .. } => Err("unsupported call in host_timer".into()),
        Expr::Assign { target, value, .. } => {
            match target {
                AssignTarget::Local(_) => {}
                _ => return Err("only local assign targets in host_timer".into()),
            }
            check_expr(value, uses)
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            if matches!(&**arg, Expr::IdentName { name, .. } if is_timer_api_name(name)) {
                *uses = true;
                Ok(())
            } else {
                check_expr(arg, uses)
            }
        }
        Expr::Binary { left, right, .. } => {
            check_expr(left, uses)?;
            check_expr(right, uses)
        }
        Expr::Local { .. } | Expr::Number { .. } | Expr::Boolean { .. } | Expr::String { .. } => {
            Ok(())
        }
        Expr::IdentName { name, .. } if is_timer_api_name(name) => {
            *uses = true;
            Ok(())
        }
        Expr::Function { .. } => Err("bare function expr unsupported".into()),
        _ => Err("unsupported expr in host_timer".into()),
    }
}

fn check_timer_body_stmt(stmt: &Stmt, uses: &mut bool) -> Result<(), String> {
    match stmt {
        Stmt::Expr { expr } => check_expr(expr, uses),
        Stmt::Block { body } => {
            for s in body {
                check_timer_body_stmt(s, uses)?;
            }
            Ok(())
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            check_expr(test, uses)?;
            check_timer_body_stmt(consequent, uses)?;
            if let Some(alt) = alternate {
                check_timer_body_stmt(alt, uses)?;
            }
            Ok(())
        }
        Stmt::Return { value } => {
            if let Some(e) = value {
                check_expr(e, uses)?;
            }
            Ok(())
        }
        _ => Err("unsupported stmt in timer callback".into()),
    }
}

fn arg_expr(arg: &Arg) -> Result<&Expr, String> {
    match arg {
        Arg::Expr(e) => Ok(e),
        Arg::Spread(_) => Err("spread args unsupported".into()),
    }
}

pub(super) fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

pub(super) fn is_clock_callee(expr: &Expr) -> bool {
    is_named_callee(expr, "nowMs")
        || is_named_callee(expr, "monotonicMs")
        || is_date_now_callee(expr)
}

fn is_date_now_callee(expr: &Expr) -> bool {
    match expr {
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let is_date = match object.as_ref() {
                Expr::IdentName { name, .. } => name == "Date",
                // Frontend binds `Date` as a local; property `now` is the clock call.
                Expr::Local { .. } => true,
                _ => false,
            };
            is_date && string_lit(property).as_deref() == Some("now")
        }
        _ => false,
    }
}

fn string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy()),
        _ => None,
    }
}
