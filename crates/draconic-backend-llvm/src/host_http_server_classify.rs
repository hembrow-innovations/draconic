use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut local_name = HashMap::new();
    for Local { id, name, .. } in &module.locals {
        local_name.insert(*id, name.clone());
    }
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_tcp: false,
        has_http: false,
        has_client: false,
        string_fns: HashMap::new(),
        fn_names: HashMap::new(),
        local_name,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !(ctx.has_tcp && ctx.has_http) {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
        client_print: ctx.has_client,
        string_fns: ctx.string_fns,
        fn_names: ctx.fn_names,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            // H10.05: only auto-print response field / header observations.
            if is_client_observation(init, ctx) {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        // H17.01: accept-loop server body (`while (true) { … }`).
        Stmt::Block { body, .. } => {
            for s in body {
                classify_stmt(s, ctx)?;
            }
            Some(())
        }
        Stmt::While { test, body, .. } => {
            classify_while_test(test)?;
            classify_stmt(body, ctx)
        }
        // P04: linked package function (`greet`) — record simple string returns.
        Stmt::Function {
            local,
            params,
            body,
            is_async: false,
            is_generator: false,
        } => {
            if params.len() == 1 && !params[0].rest && params[0].default.is_none() {
                if let Pattern::Local(pid) = &params[0].pattern {
                    if let Some(ret) = simple_string_return(body) {
                        ctx.string_fns.insert(*local, (*pid, ret));
                        if let Some(name) = ctx.local_name.get(local) {
                            ctx.fn_names.insert(name.clone(), *local);
                        }
                    }
                }
            }
            Some(())
        }
        _ => None,
    }
}

fn simple_string_return(body: &[Stmt]) -> Option<Expr> {
    match body {
        [Stmt::Return { value: Some(e) }] => Some(e.clone()),
        [Stmt::Block { body, .. }] => simple_string_return(body),
        _ => None,
    }
}

pub(super) fn subst_local(expr: &Expr, from: LocalId, to: &Expr) -> Expr {
    match expr {
        Expr::Local { id, .. } if *id == from => to.clone(),
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => Expr::Binary {
            left: Box::new(subst_local(left, from, to)),
            op: *op,
            right: Box::new(subst_local(right, from, to)),
            ty: *ty,
        },
        other => other.clone(),
    }
}

fn classify_while_test(test: &Expr) -> Option<()> {
    match test {
        Expr::Boolean { value: true, .. } => Some(()),
        Expr::Number { raw, .. } if raw == "1" => Some(()),
        _ => None,
    }
}

fn is_client_observation(expr: &Expr, ctx: &ClassifyCtx) -> bool {
    match expr {
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let Some(name) = string_lit(property) else {
                return false;
            };
            match object.as_ref() {
                Expr::Local { id, .. } => match (ctx.slot_of.get(id), name.as_str()) {
                    (Some(SlotTy::HttpRes), "version" | "reason" | "body" | "status") => true,
                    _ => false,
                },
                _ => false,
            }
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            true
        }
        _ => false,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && (is_named_callee(callee, "closeTcp") || is_named_callee(callee, "closeTls")) =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2
                && (is_named_callee(callee, "tcpWrite") || is_named_callee(callee, "tlsWrite")) =>
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
            if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
        {
            ctx.has_http = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseResponse") =>
        {
            ctx.has_http = true;
            ctx.has_client = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
        {
            ctx.has_http = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
        {
            ctx.has_http = true;
            ctx.has_client = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            ctx.has_http = true;
            ctx.has_client = true;
            classify_res_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)
        }
        // H17.03: static file serve on accepted TCP connection.
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpServeStatic") =>
        {
            ctx.has_tcp = true;
            ctx.has_http = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)
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
            if args.len() == 2
                && (is_named_callee(callee, "tcpRead") || is_named_callee(callee, "tlsRead")) =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "tlsClientWrap") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_number_arg(arg_expr(&args[2])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "tlsServerWrap") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
        {
            ctx.has_http = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::HttpReq)
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
            ctx.has_client = true;
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
            ctx.has_client = true;
            classify_bytes_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::HttpRes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            ctx.has_http = true;
            ctx.has_client = true;
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
                (SlotTy::DynBytes, "length") => Some(SlotTy::Number),
                _ => None,
            }
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readFileText") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_string_fn_callee(callee, ctx) => {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add,
            right,
            ..
        } => {
            classify_string_arg(left, ctx)?;
            classify_string_arg(right, ctx)?;
            Some(SlotTy::String)
        }
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        _ => None,
    }
}

fn is_string_fn_callee(callee: &Expr, ctx: &ClassifyCtx) -> bool {
    match callee {
        Expr::Local { id, .. } => ctx.string_fns.contains_key(id),
        Expr::IdentName { name, .. } => ctx.fn_names.contains_key(name),
        _ => false,
    }
}

fn classify_handle_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match classify_expr(expr, ctx)? {
        SlotTy::Handle => Some(()),
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
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
        {
            ctx.has_tcp = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)
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
                (SlotTy::HttpRes, "status") => Some(()),
                _ => None,
            }
        }
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
            classify_string_arg(arg_expr(&args[3])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            classify_string_arg(arg_expr(&args[2])?, ctx)?;
            classify_string_arg(arg_expr(&args[3])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpResponseHeader") =>
        {
            ctx.has_http = true;
            classify_res_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readFileText") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_string_fn_callee(callee, ctx) => {
            classify_string_arg(arg_expr(&args[0])?, ctx)
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add,
            right,
            ..
        } => {
            classify_string_arg(left, ctx)?;
            classify_string_arg(right, ctx)
        }
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
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteResponse") =>
        {
            classify_string_arg(expr, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 4 && is_named_callee(callee, "httpWriteRequest") =>
        {
            classify_string_arg(expr, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsHandshakeResponse") =>
        {
            classify_string_arg(expr, ctx)
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
                (SlotTy::HttpReq, "method" | "path" | "version" | "body") => Some(()),
                (SlotTy::HttpRes, "version" | "reason" | "body") => Some(()),
                _ => None,
            }
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
            classify_bytes_arg(arg_expr(&args[0])?, ctx)
        }
        _ => None,
    }
}
