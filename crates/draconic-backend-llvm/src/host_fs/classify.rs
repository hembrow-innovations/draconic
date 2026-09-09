use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        needs_text: false,
        needs_bytes: false,
        needs_write: false,
        needs_write_text: false,
        needs_append_text: false,
        needs_write_bytes: false,
        needs_append_bytes: false,
        needs_exists: false,
        needs_stat: false,
        needs_mkdir: false,
        needs_mkdir_all: false,
        needs_readdir: false,
        needs_rmdir: false,
        needs_remove_file: false,
        needs_rename_file: false,
        needs_copy_file: false,
        needs_open: false,
        needs_handle_read: false,
        needs_handle_write: false,
        needs_handle_seek: false,
        needs_close_file: false,
        has_fs: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_fs {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
        needs_text: ctx.needs_text,
        needs_bytes: ctx.needs_bytes,
        needs_write: ctx.needs_write,
        needs_write_text: ctx.needs_write_text,
        needs_append_text: ctx.needs_append_text,
        needs_write_bytes: ctx.needs_write_bytes,
        needs_append_bytes: ctx.needs_append_bytes,
        needs_exists: ctx.needs_exists,
        needs_stat: ctx.needs_stat,
        needs_mkdir: ctx.needs_mkdir,
        needs_mkdir_all: ctx.needs_mkdir_all,
        needs_readdir: ctx.needs_readdir,
        needs_rmdir: ctx.needs_rmdir,
        needs_remove_file: ctx.needs_remove_file,
        needs_rename_file: ctx.needs_rename_file,
        needs_copy_file: ctx.needs_copy_file,
        needs_open: ctx.needs_open,
        needs_handle_read: ctx.needs_handle_read,
        needs_handle_write: ctx.needs_handle_write,
        needs_handle_seek: ctx.needs_handle_seek,
        needs_close_file: ctx.needs_close_file,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            match ty {
                SlotTy::String | SlotTy::Number | SlotTy::Bool => {
                    ctx.print_locals.push((*local, ty));
                }
                SlotTy::DynBytes | SlotTy::Stat | SlotTy::Array | SlotTy::Handle => {}
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
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
        {
            ctx.needs_write = true;
            classify_write_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && (is_named_callee(callee, "readFileText")
                    || is_named_callee(callee, "readFileBytes")) =>
        {
            // bare call as statement (error path / discard)
            ctx.has_fs = true;
            if is_named_callee(callee, "readFileText") {
                ctx.needs_text = true;
            } else {
                ctx.needs_bytes = true;
            }
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "writeFileText") =>
        {
            ctx.has_fs = true;
            ctx.needs_write_text = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "appendFileText") =>
        {
            ctx.has_fs = true;
            ctx.needs_append_text = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "writeFileBytes") =>
        {
            ctx.has_fs = true;
            ctx.needs_write_bytes = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_or_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "appendFileBytes") =>
        {
            ctx.has_fs = true;
            ctx.needs_append_bytes = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_or_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && (is_named_callee(callee, "exists") || is_named_callee(callee, "stat")) =>
        {
            ctx.has_fs = true;
            if is_named_callee(callee, "exists") {
                ctx.needs_exists = true;
            } else {
                ctx.needs_stat = true;
            }
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "mkdir") => {
            ctx.has_fs = true;
            ctx.needs_mkdir = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "mkdirAll") =>
        {
            ctx.has_fs = true;
            ctx.needs_mkdir_all = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "rmdir") => {
            ctx.has_fs = true;
            ctx.needs_rmdir = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "removeFile") =>
        {
            ctx.has_fs = true;
            ctx.needs_remove_file = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "renameFile") =>
        {
            ctx.has_fs = true;
            ctx.needs_rename_file = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "copyFile") =>
        {
            ctx.has_fs = true;
            ctx.needs_copy_file = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readdir") =>
        {
            ctx.has_fs = true;
            ctx.needs_readdir = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "fileWrite") =>
        {
            ctx.has_fs = true;
            ctx.needs_handle_write = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_bytes_or_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if (args.len() == 2 || args.len() == 3) && is_named_callee(callee, "fileSeek") =>
        {
            ctx.has_fs = true;
            ctx.needs_handle_seek = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            if args.len() == 3 {
                classify_number_arg(arg_expr(&args[2])?, ctx)?;
            }
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "closeFile") =>
        {
            ctx.has_fs = true;
            ctx.needs_close_file = true;
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_handle_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Handle | SlotTy::Number => Some(()),
            _ => None,
        },
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
        _ => None,
    }
}

fn classify_write_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::DynBytes | SlotTy::String => Some(()),
            _ => None,
        },
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readFileText") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_text = true;
            Some(SlotTy::String)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readFileBytes") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_bytes = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "openFile") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_open = true;
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "fileRead") =>
        {
            classify_handle_arg(arg_expr(&args[0])?, ctx)?;
            classify_number_arg(arg_expr(&args[1])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_handle_read = true;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "exists") => {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_exists = true;
            Some(SlotTy::Bool)
        }
        Expr::Call { callee, args, .. } if args.len() == 1 && is_named_callee(callee, "stat") => {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_stat = true;
            Some(SlotTy::Stat)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "readdir") =>
        {
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            ctx.has_fs = true;
            ctx.needs_readdir = true;
            Some(SlotTy::Array)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let obj = match object.as_ref() {
                Expr::Local { id, .. } => ctx.slot_of.get(id).copied()?,
                _ => classify_expr(object, ctx)?,
            };
            let prop = string_lit(property)?;
            match (obj, prop.as_str()) {
                (SlotTy::DynBytes, "length") => Some(SlotTy::Number),
                (SlotTy::Array, "length") => Some(SlotTy::Number),
                (SlotTy::Stat, "size" | "mtime") => Some(SlotTy::Number),
                (SlotTy::Stat, "isFile" | "isDir") => Some(SlotTy::Bool),
                _ => None,
            }
        }
        Expr::Member {
            object,
            property,
            computed: true,
            ..
        } => {
            let obj = match object.as_ref() {
                Expr::Local { id, .. } => ctx.slot_of.get(id).copied()?,
                _ => classify_expr(object, ctx)?,
            };
            if obj != SlotTy::Array {
                return None;
            }
            let idx_ty = classify_expr(property, ctx)?;
            if idx_ty != SlotTy::Number {
                return None;
            }
            Some(SlotTy::String)
        }
        Expr::Binary {
            op: BinaryOp::Gt,
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
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
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
        _ => None,
    }
}

fn classify_bytes_or_string_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::String | SlotTy::DynBytes => Some(()),
            _ => None,
        },
        _ => None,
    }
}
