use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        has_process_args: false,
        has_env: false,
        has_exit: false,
        has_pid: false,
    };

    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }

    if !(ctx.has_process_args || ctx.has_env || ctx.has_exit || ctx.has_pid) {
        return None;
    }
    // Exit-only modules (e.g. `exit(7);`) have no print locals.
    if ctx.print_locals.is_empty() && !ctx.has_exit {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
        needs_argv: ctx.has_process_args,
        needs_env: ctx.has_env,
        needs_exit: ctx.has_exit,
        needs_pid: ctx.has_pid,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            if matches!(
                ty,
                SlotTy::Number | SlotTy::Bool | SlotTy::String | SlotTy::MaybeString
            ) {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => {
            classify_side_effect(expr, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. } => {
            let name = ident_name(callee)?;
            match name {
                "envSet" if args.len() == 2 => {
                    ctx.has_env = true;
                    classify_expr(arg_expr(&args[0])?, ctx)?;
                    classify_expr(arg_expr(&args[1])?, ctx)?;
                    Some(())
                }
                "envDelete" if args.len() == 1 => {
                    ctx.has_env = true;
                    classify_expr(arg_expr(&args[0])?, ctx)?;
                    Some(())
                }
                "exit" if args.is_empty() || args.len() == 1 => {
                    ctx.has_exit = true;
                    if args.len() == 1 {
                        classify_expr(arg_expr(&args[0])?, ctx)?;
                    }
                    Some(())
                }
                "setExitCode" if args.len() == 1 => {
                    ctx.has_exit = true;
                    classify_expr(arg_expr(&args[0])?, ctx)?;
                    Some(())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "processArgs") =>
        {
            ctx.has_process_args = true;
            Some(SlotTy::Array)
        }
        Expr::Call { callee, args, .. } if args.is_empty() && is_named_callee(callee, "pid") => {
            ctx.has_pid = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if args.is_empty() && is_named_callee(callee, "ppid") => {
            ctx.has_pid = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "envGet") => {
            ctx.has_env = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::MaybeString)
        }
        Expr::Binary {
            op: BinaryOp::Gt | BinaryOp::GtEq | BinaryOp::Lt | BinaryOp::LtEq,
            left,
            right,
            ..
        } => {
            let lt = classify_expr(left, ctx)?;
            let rt = classify_expr(right, ctx)?;
            if lt == SlotTy::Number && rt == SlotTy::Number {
                Some(SlotTy::Bool)
            } else {
                None
            }
        }
        Expr::Call { callee, args, .. } if args.len() == 2 && is_named_callee(callee, "envSet") => {
            ctx.has_env = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            classify_expr(arg_expr(&args[1])?, ctx)?;
            // Not assigned as value in fixtures; treat as void if ever used as expr.
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "envDelete") =>
        {
            ctx.has_env = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "exitCode") =>
        {
            ctx.has_exit = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if (args.is_empty() || args.len() == 1) && is_named_callee(callee, "exit") =>
        {
            ctx.has_exit = true;
            if args.len() == 1 {
                classify_expr(arg_expr(&args[0])?, ctx)?;
            }
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "setExitCode") =>
        {
            ctx.has_exit = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            let _ = classify_expr(arg, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let obj_ty = classify_expr(object, ctx)?;
            let prop = string_lit(property)?;
            if obj_ty == SlotTy::Array && prop.as_str() == "length" {
                Some(SlotTy::Number)
            } else {
                None
            }
        }
        Expr::Member {
            object,
            property,
            computed: true,
            ..
        } => {
            let obj_ty = classify_expr(object, ctx)?;
            let _idx = classify_expr(property, ctx)?;
            if obj_ty == SlotTy::Array {
                Some(SlotTy::String)
            } else {
                None
            }
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        _ => None,
    }
}
