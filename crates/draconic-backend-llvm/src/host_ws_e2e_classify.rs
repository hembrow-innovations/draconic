use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        has_tcp: false,
        has_ws_client: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    // TCP optional: `wsClientCheckAccept` alone is valid (H12.03 negative path).
    if !ctx.has_ws_client {
        return None;
    }
    Some(ModuleInfo { slots: ctx.slots })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
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
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpWrite") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_arg(arg_expr(&args[1])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
        {
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "wsClientCheckAccept") =>
        {
            ctx.has_ws_client = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsDecodeFrame") =>
        {
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
            ctx.has_tcp = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            if args.len() == 2 {
                classify_number_arg(arg_expr(&args[1])?, ctx)?;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpAccept") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
        {
            ctx.has_tcp = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpRead") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "wsClientHandshakeRequest") =>
        {
            ctx.has_ws_client = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            // Allowed in e2e when paired with client APIs; does not claim alone.
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodeTextClient") =>
        {
            ctx.has_ws_client = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodeText") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodeBinary") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "wsEncodeClose") =>
        {
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodePing") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodePong") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsDecodeFrame") =>
        {
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::WsFrame)
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
                (SlotTy::WsFrame, "fin" | "opcode" | "closeCode") => Some(SlotTy::Number),
                (SlotTy::WsFrame, "payload") => Some(SlotTy::String),
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
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Handle => Some(()),
            _ => None,
        },
        Expr::Call { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::Handle).then_some(()),
        _ => None,
    }
}

fn classify_number_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Number { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Number | SlotTy::Handle => Some(()),
            _ => None,
        },
        Expr::Call { .. } => {
            let ty = classify_expr(expr, ctx)?;
            matches!(ty, SlotTy::Number | SlotTy::Handle).then_some(())
        }
        Expr::Member { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::Number).then_some(()),
        _ => None,
    }
}

fn classify_string_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::String => Some(()),
            _ => None,
        },
        Expr::Call { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::String).then_some(()),
        Expr::Member { .. } => matches!(classify_expr(expr, ctx)?, SlotTy::String).then_some(()),
        _ => None,
    }
}

fn classify_bytes_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::String | SlotTy::DynBytes => Some(()),
            _ => None,
        },
        Expr::Call { .. } => {
            let ty = classify_expr(expr, ctx)?;
            matches!(ty, SlotTy::DynBytes | SlotTy::String).then_some(())
        }
        Expr::Member { .. } => {
            let ty = classify_expr(expr, ctx)?;
            matches!(ty, SlotTy::String | SlotTy::DynBytes).then_some(())
        }
        _ => None,
    }
}
