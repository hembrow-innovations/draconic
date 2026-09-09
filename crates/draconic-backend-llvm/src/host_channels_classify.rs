use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        queues: HashMap::new(),
        uses_make: false,
        uses_send: false,
        uses_recv: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !(ctx.uses_make || ctx.uses_send || ctx.uses_recv) || ctx.print_locals.is_empty() {
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
            ctx.slots.push((*local, ty.clone()));
            ctx.slot_of.insert(*local, ty.clone());
            if is_make_channel_call(init) {
                ctx.queues.insert(*local, VecDeque::new());
            }
            if is_scalar_print(&ty) {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => {
            let _ = classify_expr(expr, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn is_make_channel_call(expr: &Expr) -> bool {
    matches!(expr, Expr::Call { callee, args, .. } if is_named_callee(callee, "makeChannel") && args.len() <= 1)
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. } if is_named_callee(callee, "makeChannel") => {
            if args.len() > 1 {
                return None;
            }
            if args.len() == 1 {
                let cap = arg_expr(&args[0])?;
                if classify_expr(cap, ctx)? != SlotTy::Number {
                    return None;
                }
            }
            ctx.uses_make = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "channelSend") => {
            if args.len() != 2 {
                return None;
            }
            let handle = arg_expr(&args[0])?;
            let value = arg_expr(&args[1])?;
            let ht = classify_expr(handle, ctx)?;
            if ht != SlotTy::Number {
                return None;
            }
            let vt = classify_expr(value, ctx)?;
            if let Expr::Local { id, .. } = handle {
                if let Some(q) = ctx.queues.get_mut(id) {
                    q.push_back(vt);
                }
            }
            ctx.uses_send = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "channelRecv") => {
            if args.len() != 1 {
                return None;
            }
            let handle = arg_expr(&args[0])?;
            let ht = classify_expr(handle, ctx)?;
            if ht != SlotTy::Number {
                return None;
            }
            ctx.uses_recv = true;
            if let Expr::Local { id, .. } = handle {
                if let Some(q) = ctx.queues.get_mut(id) {
                    return q.pop_front();
                }
            }
            None
        }
        Expr::Binary {
            op, left, right, ..
        } if matches!(
            op,
            BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
        ) =>
        {
            let lt = classify_expr(left, ctx)?;
            let rt = classify_expr(right, ctx)?;
            if lt == SlotTy::Number && rt == SlotTy::Number {
                Some(SlotTy::Bool)
            } else if matches!(
                op,
                BinaryOp::EqEqEq | BinaryOp::NotEqEq | BinaryOp::EqEq | BinaryOp::NotEq
            ) && matches!(lt, SlotTy::Object(_))
                && matches!(rt, SlotTy::Object(_))
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
            if is_named_ident(arg, "makeChannel") {
                ctx.uses_make = true;
                Some(SlotTy::String)
            } else if is_named_ident(arg, "channelSend") {
                ctx.uses_send = true;
                Some(SlotTy::String)
            } else if is_named_ident(arg, "channelRecv") {
                ctx.uses_recv = true;
                Some(SlotTy::String)
            } else {
                let _ = classify_expr(arg, ctx)?;
                Some(SlotTy::String)
            }
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).cloned(),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Boolean { .. } => Some(SlotTy::Bool),
        Expr::Object { properties, .. } => classify_object_lit(properties, ctx),
        Expr::Member {
            object, property, ..
        } => classify_member(object, property, ctx),
        Expr::Assign {
            target: AssignTarget::Member {
                object, property, ..
            },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let key = static_prop_key(property)?;
            let vt = classify_expr(value, ctx)?;
            match ot {
                SlotTy::Object(mut shape) => {
                    shape.insert(key, vt.clone());
                    if let Expr::Local { id, .. } = object.as_ref() {
                        ctx.slot_of.insert(*id, SlotTy::Object(shape));
                    }
                    Some(vt)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_object_lit(properties: &[ObjectProp], ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    let mut shape = HashMap::new();
    for p in properties {
        let ObjectProp::Property {
            key: ObjectPropKey::Static(k),
            value,
        } = p
        else {
            return None;
        };
        let ty = classify_expr(value, ctx)?;
        shape.insert(k.to_string_lossy(), ty);
    }
    Some(SlotTy::Object(shape))
}

fn classify_member(object: &Expr, property: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    let ot = classify_expr(object, ctx)?;
    let key = static_prop_key(property)?;
    match ot {
        SlotTy::Object(shape) => shape.get(&key).cloned(),
        _ => None,
    }
}
