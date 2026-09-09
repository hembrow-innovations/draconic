use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_numbers: Vec::new(),
        has_h2: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_h2 {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_numbers: ctx.print_numbers,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            if ty == SlotTy::Number {
                ctx.print_numbers.push(*local);
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "closeTcp") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpWrite") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[1])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
        {
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "http2ParseRequest") =>
        {
            ctx.has_h2 = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "http2ParseResponse") =>
        {
            ctx.has_h2 = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
        {
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            if args.len() == 2 {
                classify_number_arg(arg_expr(&args[1])?, ctx)?;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpAccept") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpRead") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "http2ClientPreface") =>
        {
            ctx.has_h2 = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "http2ServerPreface") =>
        {
            ctx.has_h2 = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.is_empty() && is_named_callee(callee, "http2SettingsAck") =>
        {
            ctx.has_h2 = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "http2EncodeRequest") =>
        {
            ctx.has_h2 = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[2])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "http2ClientOpen") =>
        {
            ctx.has_h2 = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[2])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "http2EncodeResponse") =>
        {
            ctx.has_h2 = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "http2ServerReply") =>
        {
            ctx.has_h2 = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "http2ParseRequest") =>
        {
            ctx.has_h2 = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::H2Req)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "http2ParseResponse") =>
        {
            ctx.has_h2 = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::H2Res)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            match (ot, name.as_str()) {
                (SlotTy::DynBytes, "length") => Some(SlotTy::Number),
                (SlotTy::H2Req, "method" | "path" | "body") => Some(SlotTy::String),
                (SlotTy::H2Req, "streamId") => Some(SlotTy::Number),
                (SlotTy::H2Res, "body") => Some(SlotTy::String),
                (SlotTy::H2Res, "status" | "streamId") => Some(SlotTy::Number),
                _ => None,
            }
        }
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        _ => None,
    }
}

fn classify_handle_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Local { id, .. } => matches!(ctx.slot_of.get(id)?, SlotTy::Handle).then_some(()),
        Expr::Call { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::Handle).then_some(()),
        _ => None,
    }
}

fn classify_number_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Number { .. } => Some(()),
        Expr::Local { id, .. } => {
            matches!(ctx.slot_of.get(id)?, SlotTy::Number | SlotTy::Handle).then_some(())
        }
        Expr::Call { .. } => {
            matches!(classify_expr(expr, ctx)?, SlotTy::Number | SlotTy::Handle).then_some(())
        }
        Expr::Member { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::Number).then_some(()),
        _ => None,
    }
}

fn classify_string_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => matches!(ctx.slot_of.get(id)?, SlotTy::String).then_some(()),
        Expr::Call { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::String).then_some(()),
        Expr::Member { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::String).then_some(()),
        _ => None,
    }
}

fn classify_bytes_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => {
            matches!(ctx.slot_of.get(id)?, SlotTy::String | SlotTy::DynBytes).then_some(())
        }
        Expr::Call { .. } => {
            matches!(classify_expr(expr, ctx)?, SlotTy::DynBytes | SlotTy::String).then_some(())
        }
        Expr::Member { .. } => {
            matches!(classify_expr(expr, ctx)?, SlotTy::String | SlotTy::DynBytes).then_some(())
        }
        _ => None,
    }
}
