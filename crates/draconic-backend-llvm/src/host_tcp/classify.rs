use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_tcp: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_tcp {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
    })
}

fn is_dynbytes_length(expr: &Expr, ctx: &ClassifyCtx) -> bool {
    match expr {
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let name = match string_lit(property) {
                Some(n) => n,
                None => return false,
            };
            if name != "length" {
                return false;
            }
            match object.as_ref() {
                Expr::Local { id, .. } => ctx.slot_of.get(id) == Some(&LocalSlot::DynBytes),
                _ => false,
            }
        }
        _ => false,
    }
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            // Auto-print bools/strings always; numbers only from DynBytes.length
            // (ports etc. must not pollute native.stdout).
            if matches!(ty, LocalSlot::Bool | LocalSlot::String)
                || (ty == LocalSlot::Number && is_dynbytes_length(init, ctx))
            {
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
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && (is_named_callee(callee, "closeTcp") || is_named_callee(callee, "closeTls")) =>
        {
            ctx.has_tcp = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2
                && (is_named_callee(callee, "tcpWrite") || is_named_callee(callee, "tlsWrite")) =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            let dt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ht != LocalSlot::Handle {
                return None;
            }
            if !matches!(dt, LocalSlot::String | LocalSlot::DynBytes) {
                return None;
            }
            Some(())
        }
        Expr::Call { callee, args, .. }
            if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpShutdown") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != LocalSlot::Handle {
                return None;
            }
            if args.len() == 2 {
                let ht2 = classify_expr(arg_expr(&args[1])?, ctx)?;
                if ht2 != LocalSlot::Number {
                    return None;
                }
            }
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
        {
            let t = classify_expr(arg_expr(&args[0])?, ctx)?;
            if matches!(t, LocalSlot::String | LocalSlot::DynBytes) {
                Some(())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<LocalSlot> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "tlsClientWrap") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != LocalSlot::Handle {
                return None;
            }
            let nt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if nt != LocalSlot::String {
                return None;
            }
            let it = classify_expr(arg_expr(&args[2])?, ctx)?;
            if it != LocalSlot::Number {
                return None;
            }
            Some(LocalSlot::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "tlsServerWrap") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != LocalSlot::Handle {
                return None;
            }
            let ct = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ct != LocalSlot::String {
                return None;
            }
            let kt = classify_expr(arg_expr(&args[2])?, ctx)?;
            if kt != LocalSlot::String {
                return None;
            }
            Some(LocalSlot::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tlsRead") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != LocalSlot::Handle {
                return None;
            }
            let mt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if mt != LocalSlot::Number {
                return None;
            }
            Some(LocalSlot::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
        {
            ctx.has_tcp = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            if args.len() == 2 {
                classify_expr(arg_expr(&args[1])?, ctx)?;
            }
            Some(LocalSlot::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != LocalSlot::Handle {
                return None;
            }
            Some(LocalSlot::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpAccept") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != LocalSlot::Handle {
                return None;
            }
            Some(LocalSlot::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            let pt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ht != LocalSlot::String || pt != LocalSlot::Number {
                return None;
            }
            Some(LocalSlot::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpPeerAddress") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != LocalSlot::Handle {
                return None;
            }
            Some(LocalSlot::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpPeerPort") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != LocalSlot::Handle {
                return None;
            }
            Some(LocalSlot::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpRead") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            let mt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ht != LocalSlot::Handle || mt != LocalSlot::Number {
                return None;
            }
            Some(LocalSlot::DynBytes)
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
                (LocalSlot::DynBytes, "length") => Some(LocalSlot::Number),
                _ => None,
            }
        }
        Expr::Binary {
            op:
                BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEq
                | BinaryOp::NotEqEq,
            left,
            right,
            ..
        } => {
            let lt = classify_expr(left, ctx)?;
            let rt = classify_expr(right, ctx)?;
            if matches!(lt, LocalSlot::Number | LocalSlot::Handle)
                && matches!(rt, LocalSlot::Number | LocalSlot::Handle)
            {
                Some(LocalSlot::Bool)
            } else {
                None
            }
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            let _ = classify_expr(arg, ctx)?;
            Some(LocalSlot::String)
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(LocalSlot::Number),
        Expr::String { .. } => Some(LocalSlot::String),
        _ => None,
    }
}
