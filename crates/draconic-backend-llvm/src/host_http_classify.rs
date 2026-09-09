use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_http: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_http {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            if ty == SlotTy::String || ty == SlotTy::Number {
                ctx.print_locals.push((*local, ty));
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
            if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpRequestHeader") =>
        {
            ctx.has_http = true;
            classify_req_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
        {
            ctx.has_http = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            ctx.has_http = true;
            classify_res_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::HttpReq)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpRequestHeader") =>
        {
            ctx.has_http = true;
            classify_req_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
        {
            ctx.has_http = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::HttpRes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            ctx.has_http = true;
            classify_res_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::String)
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
                (SlotTy::HttpReq, "method" | "path" | "version" | "body") => Some(SlotTy::String),
                (SlotTy::HttpRes, "version" | "reason" | "body") => Some(SlotTy::String),
                (SlotTy::HttpRes, "status") => Some(SlotTy::Number),
                _ => None,
            }
        }
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
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
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            match (ot, name.as_str()) {
                (SlotTy::HttpReq, "method" | "path" | "version" | "body") => Some(()),
                (SlotTy::HttpRes, "version" | "reason" | "body") => Some(()),
                _ => None,
            }
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
        {
            ctx.has_http = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpRequestHeader") =>
        {
            ctx.has_http = true;
            classify_req_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            ctx.has_http = true;
            classify_res_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_number_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Number { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Number => Some(()),
            _ => None,
        },
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            match (ot, name.as_str()) {
                (SlotTy::HttpRes, "status") => Some(()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_req_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::HttpReq => Some(()),
            _ => None,
        },
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_res_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::HttpRes => Some(()),
            _ => None,
        },
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        _ => None,
    }
}
