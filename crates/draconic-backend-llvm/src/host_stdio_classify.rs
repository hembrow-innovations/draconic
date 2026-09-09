use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        module,
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        needs_stdin_line: false,
        needs_stdin_bytes: false,
        needs_write: false,
        has_stdio: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_stdio {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
        needs_stdin_line: ctx.needs_stdin_line,
        needs_stdin_bytes: ctx.needs_stdin_bytes,
        needs_write: ctx.needs_write,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            match ty {
                SlotTy::MaybeString | SlotTy::Number => {
                    ctx.print_locals.push((*local, ty));
                }
                SlotTy::Bytes(_) | SlotTy::DynBytes => {}
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && (is_named_callee(callee, "stdoutWrite", ctx.module)
                    || is_named_callee(callee, "stderrWrite", ctx.module)) =>
        {
            ctx.has_stdio = true;
            ctx.needs_write = true;
            classify_write_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    computed: true,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let obj_ty = classify_expr(object, ctx)?;
            let idx = number_lit_usize(property)?;
            let _byte = number_lit_u8(value)?;
            match obj_ty {
                SlotTy::Bytes(n) if idx < n => Some(()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_write_arg(expr: &Expr, ctx: &mut ClassifyCtx<'_>) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Bytes(_) | SlotTy::DynBytes => Some(()),
            _ => None,
        },
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx<'_>) -> Option<SlotTy> {
    match expr {
        Expr::New { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "Uint8Array", ctx.module) =>
        {
            let n = number_lit_usize(arg_expr(&args[0])?)?;
            Some(SlotTy::Bytes(n))
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "stdinReadLine", ctx.module) =>
        {
            ctx.has_stdio = true;
            ctx.needs_stdin_line = true;
            Some(SlotTy::MaybeString)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdinReadBytes", ctx.module) =>
        {
            let n = number_lit_usize(arg_expr(&args[0])?)?;
            if n > (isize::MAX as usize) {
                return None;
            }
            ctx.has_stdio = true;
            ctx.needs_stdin_bytes = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let obj = classify_expr(object, ctx)?;
            let prop = string_lit(property)?;
            if prop == "length" {
                match obj {
                    SlotTy::Bytes(_) | SlotTy::DynBytes => Some(SlotTy::Number),
                    _ => None,
                }
            } else {
                None
            }
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            classify_expr(arg, ctx)?;
            // typeof result is a string; print as string via typeof emit path.
            // Store as MaybeString? Better: treat as String printed via PRINT_STR.
            // Use Number path won't work. Reuse MaybeString only for nullability.
            // Host_process uses String slot for typeof. Add String = always present.
            // Simpler: typeof of maybe-string → store as MaybeString no - typeof never null.
            // Use a dedicated approach: classify as Number is wrong.
            // I'll emit typeof into a string global and store as "string slot" via MaybeString
            // but always non-null. PRINT_STR works.
            let _ = arg;
            Some(SlotTy::MaybeString) // reused: non-null cstr from typeof
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::MaybeString),
        _ => None,
    }
}
