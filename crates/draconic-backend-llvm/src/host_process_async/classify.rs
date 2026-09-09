use super::*;

pub(super) fn try_classify(module: &Module) -> Result<ModuleInfo, String> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_ids = HashSet::new();
    collect_top_level_decl_ids(&module.body, &mut user_ids);

    let mut slot_of: HashMap<LocalId, SlotKind> = HashMap::new();
    let mut user_locals = Vec::new();
    let mut seen = HashSet::new();
    let mut uses_async = false;

    for stmt in &module.body {
        check_stmt(stmt, &mut uses_async, &mut slot_of)?;
        if let Stmt::Declare { local, init, .. } = stmt {
            if !seen.insert(*local) {
                continue;
            }
            if !user_ids.contains(local) {
                continue;
            }
            let kind = if let Some(e) = init {
                kind_from_expr(e, &slot_of).unwrap_or(SlotKind::Number)
            } else {
                SlotKind::Number
            };
            let _ = by_id.get(local);
            slot_of.insert(*local, kind);
            user_locals.push((*local, kind));
        }
    }

    Ok(ModuleInfo {
        uses_async,
        user_locals,
    })
}

fn collect_top_level_decl_ids(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Declare { local, .. } => {
                out.insert(*local);
            }
            Stmt::Block { body } => collect_top_level_decl_ids(body, out),
            _ => {}
        }
    }
}

fn kind_from_expr(expr: &Expr, slot_of: &HashMap<LocalId, SlotKind>) -> Option<SlotKind> {
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
        Expr::Call { callee, .. } if is_named_callee(callee, "processWaitAsync") => {
            Some(SlotKind::Promise)
        }
        Expr::Call { callee, .. }
            if is_named_callee(callee, "processSpawn")
                || is_named_callee(callee, "processClose") =>
        {
            Some(SlotKind::Number)
        }
        Expr::Number { .. } => Some(SlotKind::Number),
        Expr::Boolean { .. } => Some(SlotKind::Bool),
        Expr::String { .. } => Some(SlotKind::String),
        Expr::Local { id, .. } => slot_of.get(id).copied(),
        Expr::Member {
            property,
            computed: false,
            ..
        } => {
            let Expr::String { value, .. } = property.as_ref() else {
                return None;
            };
            let prop = value.to_string_lossy();
            if prop == "then" {
                return Some(SlotKind::Promise);
            }
            None
        }
        _ => None,
    }
}

fn check_stmt(
    stmt: &Stmt,
    uses: &mut bool,
    slot_of: &mut HashMap<LocalId, SlotKind>,
) -> Result<(), String> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            if let Some(e) = init {
                check_expr(e, uses, slot_of)?;
                if let Some(k) = kind_from_expr(e, slot_of) {
                    slot_of.insert(*local, k);
                }
            }
            Ok(())
        }
        Stmt::Expr { expr } => check_expr(expr, uses, slot_of),
        Stmt::Block { body } => {
            for s in body {
                check_stmt(s, uses, slot_of)?;
            }
            Ok(())
        }
        _ => Err("unsupported stmt in host_process_async".into()),
    }
}

fn check_expr(
    expr: &Expr,
    uses: &mut bool,
    slot_of: &mut HashMap<LocalId, SlotKind>,
) -> Result<(), String> {
    match expr {
        Expr::Call { callee, args, .. } => {
            if is_named_callee(callee, "processWaitAsync") {
                *uses = true;
            }
            if is_named_callee(callee, "processSpawn")
                || is_named_callee(callee, "processClose")
                || is_named_callee(callee, "processWaitAsync")
            {
                for a in args {
                    if let Arg::Expr(e) = a {
                        check_expr(e, uses, slot_of)?;
                    }
                }
                return Ok(());
            }
            if let Expr::Member {
                object,
                property,
                computed: false,
                ..
            } = callee.as_ref()
            {
                if let Expr::String { value, .. } = property.as_ref() {
                    if value.to_string_lossy() == "then" {
                        *uses = true;
                        check_expr(object, uses, slot_of)?;
                        for a in args {
                            if let Arg::Expr(e) = a {
                                check_expr(e, uses, slot_of)?;
                            }
                        }
                        return Ok(());
                    }
                }
            }
            Err("unsupported call in host_process_async".into())
        }
        Expr::Function { body, .. } => {
            for s in body {
                check_stmt(s, uses, slot_of)?;
            }
            Ok(())
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            value,
            ..
        } => {
            check_expr(value, uses, slot_of)?;
            if let Some(k) = kind_from_expr(value, slot_of) {
                slot_of.insert(*id, k);
            }
            Ok(())
        }
        Expr::Unary { arg, .. } => check_expr(arg, uses, slot_of),
        Expr::Binary { left, right, .. } => {
            check_expr(left, uses, slot_of)?;
            check_expr(right, uses, slot_of)
        }
        Expr::Local { .. }
        | Expr::Number { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::IdentName { .. }
        | Expr::Array { .. } => Ok(()),
        Expr::Member { object, .. } => check_expr(object, uses, slot_of),
        _ => Err("unsupported expr in host_process_async".into()),
    }
}
