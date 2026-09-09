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
                Expr::Local { id, .. } => ctx.slot_of.get(id) == Some(&SlotTy::DynBytes),
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
            if matches!(ty, SlotTy::Bool | SlotTy::String)
                || (ty == SlotTy::Number && is_dynbytes_length(init, ctx))
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
            if ht != SlotTy::Handle {
                return None;
            }
            if !matches!(dt, SlotTy::String | SlotTy::DynBytes) {
                return None;
            }
            Some(())
        }
        Expr::Call { callee, args, .. }
            if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpShutdown") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            if args.len() == 2 {
                let ht2 = classify_expr(arg_expr(&args[1])?, ctx)?;
                if ht2 != SlotTy::Number {
                    return None;
                }
            }
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
        {
            let t = classify_expr(arg_expr(&args[0])?, ctx)?;
            if matches!(t, SlotTy::String | SlotTy::DynBytes) {
                Some(())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "tlsClientWrap") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            let nt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if nt != SlotTy::String {
                return None;
            }
            let it = classify_expr(arg_expr(&args[2])?, ctx)?;
            if it != SlotTy::Number {
                return None;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 3 && is_named_callee(callee, "tlsServerWrap") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            let ct = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ct != SlotTy::String {
                return None;
            }
            let kt = classify_expr(arg_expr(&args[2])?, ctx)?;
            if kt != SlotTy::String {
                return None;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tlsRead") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            let mt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if mt != SlotTy::Number {
                return None;
            }
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
        {
            ctx.has_tcp = true;
            classify_expr(arg_expr(&args[0])?, ctx)?;
            if args.len() == 2 {
                classify_expr(arg_expr(&args[1])?, ctx)?;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpAccept") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            let pt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ht != SlotTy::String || pt != SlotTy::Number {
                return None;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpPeerAddress") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpPeerPort") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpRead") =>
        {
            ctx.has_tcp = true;
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            let mt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ht != SlotTy::Handle || mt != SlotTy::Number {
                return None;
            }
            Some(SlotTy::DynBytes)
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
            if matches!(lt, SlotTy::Number | SlotTy::Handle)
                && matches!(rt, SlotTy::Number | SlotTy::Handle)
            {
                Some(SlotTy::Bool)
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
            Some(SlotTy::String)
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        _ => None,
    }
}
