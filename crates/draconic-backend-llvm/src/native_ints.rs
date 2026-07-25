//! N01: lower pure native-integer Programs to LLVM IR.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp, UpdateOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, NativeType, Param, Pattern,
    Stmt, UpdateTarget,
};

/// True when every **user-declared** local is a native integer or a function,
/// and the module has at least one native integer local (N01 surface).
///
/// Globals (Object/Function builtins) are ignored — they live in `module.locals`
/// for the JS world but are unused by pure native-int programs.
pub(crate) fn is_native_int_module(module: &Module) -> bool {
    let mut user = HashSet::new();
    collect_user_local_ids(&module.body, &mut user);
    if user.is_empty() {
        return false;
    }
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut has_int = false;
    for id in user {
        let Some(local) = by_id.get(&id) else {
            return false;
        };
        match local.ty {
            Type::Native(n) if n.is_int() => has_int = true,
            Type::Function => {}
            _ => return false,
        }
    }
    has_int
}

fn collect_user_local_ids(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        collect_user_local_ids_stmt(stmt, out);
    }
}

fn collect_user_local_ids_stmt(stmt: &Stmt, out: &mut HashSet<LocalId>) {
    match stmt {
        Stmt::Declare { local, .. } => {
            out.insert(*local);
        }
        Stmt::Function {
            local,
            params,
            body,
            ..
        } => {
            out.insert(*local);
            for p in params {
                collect_pattern_locals(&p.pattern, out);
            }
            collect_user_local_ids(body, out);
        }
        Stmt::Block { body } => collect_user_local_ids(body, out),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_user_local_ids_stmt(consequent, out);
            if let Some(a) = alternate {
                collect_user_local_ids_stmt(a, out);
            }
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } | Stmt::Labeled { body, .. } => {
            collect_user_local_ids_stmt(body, out);
        }
        Stmt::For {
            init,
            body,
            ..
        } => {
            if let Some(i) = init {
                collect_user_local_ids_stmt(i, out);
            }
            collect_user_local_ids_stmt(body, out);
        }
        _ => {}
    }
}

fn collect_pattern_locals(pat: &Pattern, out: &mut HashSet<LocalId>) {
    match pat {
        Pattern::Local(id) => {
            out.insert(*id);
        }
        Pattern::Array(els) => {
            for el in els {
                match el {
                    draconic_ir::ArrayPatternEl::Pattern { binding, .. } => {
                        collect_pattern_locals(binding, out);
                    }
                    draconic_ir::ArrayPatternEl::Rest(id) => {
                        out.insert(*id);
                    }
                }
            }
        }
        Pattern::Object(props) => {
            for p in props {
                match p {
                    draconic_ir::ObjectPatternEl::Prop { binding, .. } => {
                        collect_pattern_locals(binding, out);
                    }
                    draconic_ir::ObjectPatternEl::Rest(id) => {
                        out.insert(*id);
                    }
                }
            }
        }
    }
}

pub(crate) fn emit_native_ints(module: &Module) -> Result<String, Diagnostic> {
    let mut em = Emitter::new(module);
    em.emit_module()?;
    Ok(em.finish())
}

struct Emitter<'a> {
    module: &'a Module,
    locals: HashMap<LocalId, &'a Local>,
    /// Alloca pointer SSA name per local: `%l{id}`
    allocas: HashMap<LocalId, String>,
    /// Function IR local → LLVM function name
    fn_names: HashMap<LocalId, String>,
    /// Function param locals (no alloca; SSA param name)
    params: HashMap<LocalId, (String, NativeType)>,
    out: String,
    body: String,
    tmp: u32,
    label: u32,
    /// Top-level native int locals to print at end of main (declare order).
    print_order: Vec<LocalId>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        let locals: HashMap<LocalId, &'a Local> =
            module.locals.iter().map(|l| (l.id, l)).collect();
        let mut fn_names = HashMap::new();
        for stmt in &module.body {
            if let Stmt::Function { local, .. } = stmt {
                let name = locals
                    .get(local)
                    .map(|l| l.name.as_str())
                    .unwrap_or("fn");
                let safe: String = name
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                    .collect();
                fn_names.insert(*local, format!("d_{safe}_{}", local.0));
            }
        }
        Self {
            module,
            locals,
            allocas: HashMap::new(),
            fn_names,
            params: HashMap::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
            label: 0,
            print_order: Vec::new(),
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(self.out, "; Draconic LLVM backend (N01 native integers)").ok();
        writeln!(self.out, "declare void @draconic_rt_print_i64(i64)").ok();
        writeln!(self.out, "declare void @draconic_rt_print_u64(i64)").ok();
        writeln!(self.out).ok();

        // Emit nested function definitions first.
        for stmt in &self.module.body {
            if let Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
            } = stmt
            {
                if *is_async || *is_generator {
                    return Err(diag("native ints: async/generator functions not supported"));
                }
                self.emit_function(*local, params, body)?;
            }
        }

        // main
        self.body.clear();
        self.tmp = 0;
        self.label = 0;
        self.params.clear();
        self.allocas.clear();
        self.print_order.clear();

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();

        // Pre-declare allocas for all non-function, non-param top-level locals.
        for local in &self.module.locals {
            if matches!(local.ty, Type::Function) {
                continue;
            }
            if let Type::Native(n) = local.ty {
                if n.is_int() {
                    let ptr = format!("%l{}", local.id.0);
                    self.allocas.insert(local.id, ptr.clone());
                    writeln!(
                        self.out,
                        "  {ptr} = alloca {}, align {}",
                        n.llvm_ty(),
                        n.bit_width() / 8
                    )
                    .ok();
                }
            }
        }

        for stmt in &self.module.body {
            if matches!(stmt, Stmt::Function { .. }) {
                continue;
            }
            self.emit_stmt(stmt)?;
        }

        // Print top-level native int declares in source order.
        for id in self.print_order.clone() {
            self.emit_print_local(id)?;
        }

        writeln!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_function(
        &mut self,
        local: LocalId,
        params: &[Param],
        body: &[Stmt],
    ) -> Result<(), Diagnostic> {
        let fn_name = self
            .fn_names
            .get(&local)
            .cloned()
            .ok_or_else(|| diag("internal: missing function name"))?;

        let mut param_tys = Vec::new();
        let mut param_ids = Vec::new();
        for p in params {
            if p.rest || p.default.is_some() {
                return Err(diag("native ints: rest/default params not supported"));
            }
            let Pattern::Local(id) = &p.pattern else {
                return Err(diag("native ints: only simple ident params supported"));
            };
            let ty = self.local_native(*id)?;
            param_tys.push(ty);
            param_ids.push(*id);
        }

        // Infer return type from first Return with a value, else i32.
        let ret_ty = infer_return_native(body).unwrap_or(NativeType::I32);

        let mut sig = String::new();
        write!(sig, "define {} @{fn_name}(", ret_ty.llvm_ty()).ok();
        for (i, (id, ty)) in param_ids.iter().zip(param_tys.iter()).enumerate() {
            if i > 0 {
                sig.push_str(", ");
            }
            write!(sig, "{} %p{}", ty.llvm_ty(), id.0).ok();
        }
        sig.push(')');

        // Save main state.
        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_label = self.label;
        let saved_params = std::mem::take(&mut self.params);
        let saved_allocas = std::mem::take(&mut self.allocas);
        let saved_print = std::mem::take(&mut self.print_order);

        self.tmp = 0;
        self.label = 0;
        self.params.clear();
        self.allocas.clear();

        for (id, ty) in param_ids.iter().zip(param_tys.iter()) {
            self.params.insert(*id, (format!("%p{}", id.0), *ty));
        }

        // Allocas for locals declared inside the function (and copy params to allocas for mutability).
        let mut pre = String::new();
        for (id, ty) in param_ids.iter().zip(param_tys.iter()) {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            writeln!(
                pre,
                "  {ptr} = alloca {}, align {}",
                ty.llvm_ty(),
                ty.bit_width() / 8
            )
            .ok();
            writeln!(
                pre,
                "  store {} %p{}, ptr {ptr}",
                ty.llvm_ty(),
                id.0
            )
            .ok();
            // Params are also addressable via alloca now.
            self.params.remove(id);
        }

        // Collect locals used in body that aren't params.
        let mut body_locals = Vec::new();
        collect_declared_locals(body, &mut body_locals);
        for id in body_locals {
            if self.allocas.contains_key(&id) {
                continue;
            }
            let ty = self.local_native(id)?;
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(id, ptr.clone());
            writeln!(
                pre,
                "  {ptr} = alloca {}, align {}",
                ty.llvm_ty(),
                ty.bit_width() / 8
            )
            .ok();
        }

        for stmt in body {
            self.emit_stmt(stmt)?;
        }

        // Ensure terminator.
        if !self.body_ends_with_terminator() {
            // Default return 0 of return type.
            let zero = format!("{} 0", ret_ty.llvm_ty());
            // zero is "i32 0" — need just the value part for ret
            writeln!(self.body, "  ret {} 0", ret_ty.llvm_ty()).ok();
            let _ = zero;
        }

        writeln!(self.out, "{sig} {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{pre}").ok();
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.label = saved_label;
        self.params = saved_params;
        self.allocas = saved_allocas;
        self.print_order = saved_print;
        Ok(())
    }

    fn body_ends_with_terminator(&self) -> bool {
        for line in self.body.lines().rev() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            return t.starts_with("ret ") || t.starts_with("br ");
        }
        false
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let ty = self.local_native(*local)?;
                let ptr = self
                    .allocas
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("internal: missing alloca for local"))?;
                if let Some(init) = init {
                    let v = self.emit_expr(init, Some(ty))?;
                    writeln!(self.body, "  store {} {v}, ptr {ptr}", ty.llvm_ty()).ok();
                } else {
                    writeln!(self.body, "  store {} 0, ptr {ptr}", ty.llvm_ty()).ok();
                }
                // Main tracks declares for end-of-program print; function emit uses a
                // saved empty print_order that is discarded.
                if !self.print_order.contains(local) {
                    self.print_order.push(*local);
                }
                Ok(())
            }
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr, None)?;
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Return { value } => {
                if let Some(v) = value {
                    let nty = match v.ty() {
                        Type::Native(n) if n.is_int() => n,
                        _ => {
                            return Err(diag(
                                "native ints: return value must be a native integer",
                            ))
                        }
                    };
                    let val = self.emit_expr(v, Some(nty))?;
                    writeln!(self.body, "  ret {} {val}", nty.llvm_ty()).ok();
                } else {
                    writeln!(self.body, "  ret i32 0").ok();
                }
                // Unreachable padding label so subsequent code is valid if any.
                let lab = self.fresh_label("after_ret");
                writeln!(self.body, "{lab}:").ok();
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let cond = self.emit_bool(test)?;
                let then_l = self.fresh_label("then");
                let else_l = self.fresh_label("else");
                let end_l = self.fresh_label("endif");
                if alternate.is_some() {
                    writeln!(
                        self.body,
                        "  br i1 {cond}, label %{then_l}, label %{else_l}"
                    )
                    .ok();
                } else {
                    writeln!(
                        self.body,
                        "  br i1 {cond}, label %{then_l}, label %{end_l}"
                    )
                    .ok();
                }
                writeln!(self.body, "{then_l}:").ok();
                self.emit_stmt(consequent)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{end_l}").ok();
                }
                if let Some(alt) = alternate {
                    writeln!(self.body, "{else_l}:").ok();
                    self.emit_stmt(alt)?;
                    if !self.body_ends_with_terminator() {
                        writeln!(self.body, "  br label %{end_l}").ok();
                    }
                }
                writeln!(self.body, "{end_l}:").ok();
                Ok(())
            }
            Stmt::While { test, body } => {
                let head = self.fresh_label("while_head");
                let bod = self.fresh_label("while_body");
                let end = self.fresh_label("while_end");
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{head}:").ok();
                let cond = self.emit_bool(test)?;
                writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                writeln!(self.body, "{bod}:").ok();
                self.emit_stmt(body)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{head}").ok();
                }
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::Function { .. } => Ok(()), // emitted separately
            other => Err(diag(&format!(
                "native ints: unsupported statement {other:?}"
            ))),
        }
    }

    fn emit_print_local(&mut self, id: LocalId) -> Result<(), Diagnostic> {
        let ty = self.local_native(id)?;
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("internal: print missing alloca"))?;
        let v = self.fresh_tmp();
        writeln!(self.body, "  {v} = load {}, ptr {ptr}", ty.llvm_ty()).ok();
        let ext = self.fresh_tmp();
        if ty.bit_width() < 64 {
            if ty.is_signed() {
                writeln!(
                    self.body,
                    "  {ext} = sext {} {v} to i64",
                    ty.llvm_ty()
                )
                .ok();
            } else {
                writeln!(
                    self.body,
                    "  {ext} = zext {} {v} to i64",
                    ty.llvm_ty()
                )
                .ok();
            }
        } else {
            // already i64
            writeln!(self.body, "  {ext} = add i64 {v}, 0").ok();
        }
        if ty.is_signed() {
            writeln!(self.body, "  call void @draconic_rt_print_i64(i64 {ext})").ok();
        } else {
            writeln!(self.body, "  call void @draconic_rt_print_u64(i64 {ext})").ok();
        }
        Ok(())
    }

    fn emit_bool(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr.ty() {
            Type::Boolean => self.emit_expr(expr, None),
            Type::Native(n) if n.is_int() => {
                let v = self.emit_expr(expr, Some(n))?;
                let t = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {t} = icmp ne {} {v}, 0",
                    n.llvm_ty()
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("native ints: condition must be bool or native int")),
        }
    }

    fn emit_expr(
        &mut self,
        expr: &Expr,
        expect: Option<NativeType>,
    ) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, ty } => {
                if let Some((pname, _)) = self.params.get(id) {
                    return Ok(pname.clone());
                }
                if let Some(ptr) = self.allocas.get(id).cloned() {
                    let nty = match ty {
                        Type::Native(n) if n.is_int() => *n,
                        _ => self.local_native(*id)?,
                    };
                    let t = self.fresh_tmp();
                    writeln!(self.body, "  {t} = load {}, ptr {ptr}", nty.llvm_ty()).ok();
                    return Ok(t);
                }
                // Function reference used as value — not supported except as callee.
                Err(diag("native ints: bare function value not supported"))
            }
            Expr::Number { raw, ty } => {
                let nty = match (ty, expect) {
                    (Type::Native(n), _) if n.is_int() => *n,
                    (_, Some(n)) => n,
                    _ => {
                        return Err(diag(
                            "native ints: number literal needs native integer context",
                        ))
                    }
                };
                Ok(format_int_const(raw, nty)?)
            }
            Expr::Unary { op, arg, ty } => {
                let nty = match ty {
                    Type::Native(n) if n.is_int() => *n,
                    _ => {
                        return Err(diag("native ints: unary result must be native int"))
                    }
                };
                let a = self.emit_expr(arg, Some(nty))?;
                let t = self.fresh_tmp();
                match op {
                    UnaryOp::Minus => {
                        writeln!(
                            self.body,
                            "  {t} = sub {} 0, {a}",
                            nty.llvm_ty()
                        )
                        .ok();
                    }
                    UnaryOp::BitNot => {
                        writeln!(
                            self.body,
                            "  {t} = xor {} {a}, -1",
                            nty.llvm_ty()
                        )
                        .ok();
                    }
                    UnaryOp::Plus => {
                        writeln!(self.body, "  {t} = add {} {a}, 0", nty.llvm_ty()).ok();
                    }
                    _ => return Err(diag(&format!("native ints: unsupported unary {op}"))),
                }
                Ok(t)
            }
            Expr::Binary {
                left,
                op,
                right,
                ty,
            } => self.emit_binary(left, *op, right, ty, expect),
            Expr::Assign {
                target,
                op,
                value,
                ty,
            } => self.emit_assign(target, *op, value, ty),
            Expr::Update {
                op,
                target,
                prefix,
                ty,
            } => self.emit_update(*op, target, *prefix, ty),
            Expr::Call {
                callee,
                args,
                optional,
                ty,
            } => {
                if *optional {
                    return Err(diag("native ints: optional call not supported"));
                }
                let Expr::Local { id, .. } = callee.as_ref() else {
                    return Err(diag("native ints: only direct function calls supported"));
                };
                let fn_name = self
                    .fn_names
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("native ints: call to unknown function"))?;
                // Checker currently types non-generic calls as `Any`; prefer the
                // expression type, then expected context, then inferred signature.
                let ret_ty = match ty {
                    Type::Native(n) if n.is_int() => *n,
                    _ => match expect {
                        Some(n) => n,
                        None => self.function_sig(*id)?.1,
                    },
                };
                let (param_tys, _) = self.function_sig(*id)?;
                if param_tys.len() != args.len() {
                    return Err(diag("native ints: arity mismatch"));
                }
                let mut arg_parts = Vec::new();
                for (arg, pty) in args.iter().zip(param_tys.iter()) {
                    let Arg::Expr(e) = arg else {
                        return Err(diag("native ints: spread args not supported"));
                    };
                    let v = self.emit_expr(e, Some(*pty))?;
                    arg_parts.push(format!("{} {v}", pty.llvm_ty()));
                }
                let t = self.fresh_tmp();
                write!(
                    self.body,
                    "  {t} = call {} @{fn_name}(",
                    ret_ty.llvm_ty()
                )
                .ok();
                for (i, p) in arg_parts.iter().enumerate() {
                    if i > 0 {
                        self.body.push_str(", ");
                    }
                    self.body.push_str(p);
                }
                writeln!(self.body, ")").ok();
                Ok(t)
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ty,
            } => {
                let nty = match ty {
                    Type::Native(n) if n.is_int() => *n,
                    _ => expect.ok_or_else(|| {
                        diag("native ints: conditional needs native int type")
                    })?,
                };
                let cond = self.emit_bool(test)?;
                let then_l = self.fresh_label("sel_then");
                let else_l = self.fresh_label("sel_else");
                let end_l = self.fresh_label("sel_end");
                let slot = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {slot} = alloca {}, align {}",
                    nty.llvm_ty(),
                    nty.bit_width() / 8
                )
                .ok();
                writeln!(
                    self.body,
                    "  br i1 {cond}, label %{then_l}, label %{else_l}"
                )
                .ok();
                writeln!(self.body, "{then_l}:").ok();
                let c = self.emit_expr(consequent, Some(nty))?;
                writeln!(self.body, "  store {} {c}, ptr {slot}", nty.llvm_ty()).ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{else_l}:").ok();
                let a = self.emit_expr(alternate, Some(nty))?;
                writeln!(self.body, "  store {} {a}, ptr {slot}", nty.llvm_ty()).ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{end_l}:").ok();
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = load {}, ptr {slot}", nty.llvm_ty()).ok();
                Ok(t)
            }
            _ => Err(diag(&format!(
                "native ints: unsupported expression {expr:?}"
            ))),
        }
    }

    fn emit_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        ty: &Type,
        expect: Option<NativeType>,
    ) -> Result<String, Diagnostic> {
        // Comparisons → i1
        if matches!(
            op,
            BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
        ) {
            let nty = native_operand_ty(left, right, expect)?;
            let l = self.emit_expr(left, Some(nty))?;
            let r = self.emit_expr(right, Some(nty))?;
            let pred = match op {
                BinaryOp::EqEq | BinaryOp::EqEqEq => "eq",
                BinaryOp::NotEq | BinaryOp::NotEqEq => "ne",
                BinaryOp::Lt => {
                    if nty.is_signed() {
                        "slt"
                    } else {
                        "ult"
                    }
                }
                BinaryOp::LtEq => {
                    if nty.is_signed() {
                        "sle"
                    } else {
                        "ule"
                    }
                }
                BinaryOp::Gt => {
                    if nty.is_signed() {
                        "sgt"
                    } else {
                        "ugt"
                    }
                }
                BinaryOp::GtEq => {
                    if nty.is_signed() {
                        "sge"
                    } else {
                        "uge"
                    }
                }
                _ => unreachable!(),
            };
            let t = self.fresh_tmp();
            writeln!(
                self.body,
                "  {t} = icmp {pred} {} {l}, {r}",
                nty.llvm_ty()
            )
            .ok();
            return Ok(t);
        }

        if matches!(op, BinaryOp::Comma) {
            let _ = self.emit_expr(left, expect)?;
            return self.emit_expr(right, expect);
        }

        let nty = match ty {
            Type::Native(n) if n.is_int() => *n,
            _ => native_operand_ty(left, right, expect)?,
        };
        let l = self.emit_expr(left, Some(nty))?;
        let r = self.emit_expr(right, Some(nty))?;
        let t = self.fresh_tmp();
        let ll = nty.llvm_ty();
        match op {
            BinaryOp::Add => writeln!(self.body, "  {t} = add {ll} {l}, {r}").ok(),
            BinaryOp::Sub => writeln!(self.body, "  {t} = sub {ll} {l}, {r}").ok(),
            BinaryOp::Mul => writeln!(self.body, "  {t} = mul {ll} {l}, {r}").ok(),
            BinaryOp::Div => {
                if nty.is_signed() {
                    writeln!(self.body, "  {t} = sdiv {ll} {l}, {r}").ok()
                } else {
                    writeln!(self.body, "  {t} = udiv {ll} {l}, {r}").ok()
                }
            }
            BinaryOp::Rem => {
                if nty.is_signed() {
                    writeln!(self.body, "  {t} = srem {ll} {l}, {r}").ok()
                } else {
                    writeln!(self.body, "  {t} = urem {ll} {l}, {r}").ok()
                }
            }
            BinaryOp::BitAnd => writeln!(self.body, "  {t} = and {ll} {l}, {r}").ok(),
            BinaryOp::BitOr => writeln!(self.body, "  {t} = or {ll} {l}, {r}").ok(),
            BinaryOp::BitXor => writeln!(self.body, "  {t} = xor {ll} {l}, {r}").ok(),
            BinaryOp::Shl => writeln!(self.body, "  {t} = shl {ll} {l}, {r}").ok(),
            BinaryOp::Shr => {
                if nty.is_signed() {
                    writeln!(self.body, "  {t} = ashr {ll} {l}, {r}").ok()
                } else {
                    writeln!(self.body, "  {t} = lshr {ll} {l}, {r}").ok()
                }
            }
            _ => {
                return Err(diag(&format!(
                    "native ints: unsupported binary operator {op}"
                )))
            }
        };
        Ok(t)
    }

    fn emit_assign(
        &mut self,
        target: &AssignTarget,
        op: AssignOp,
        value: &Expr,
        ty: &Type,
    ) -> Result<String, Diagnostic> {
        let AssignTarget::Local(id) = target else {
            return Err(diag("native ints: only local assignment supported"));
        };
        let nty = match ty {
            Type::Native(n) if n.is_int() => *n,
            _ => self.local_native(*id)?,
        };
        let ptr = self
            .allocas
            .get(id)
            .cloned()
            .ok_or_else(|| diag("internal: assign missing alloca"))?;

        let rhs = if matches!(op, AssignOp::Eq) {
            self.emit_expr(value, Some(nty))?
        } else {
            let cur = self.fresh_tmp();
            writeln!(self.body, "  {cur} = load {}, ptr {ptr}", nty.llvm_ty()).ok();
            let rhs_v = self.emit_expr(value, Some(nty))?;
            let t = self.fresh_tmp();
            let ll = nty.llvm_ty();
            match op {
                AssignOp::AddEq => writeln!(self.body, "  {t} = add {ll} {cur}, {rhs_v}").ok(),
                AssignOp::SubEq => writeln!(self.body, "  {t} = sub {ll} {cur}, {rhs_v}").ok(),
                AssignOp::MulEq => writeln!(self.body, "  {t} = mul {ll} {cur}, {rhs_v}").ok(),
                AssignOp::DivEq => {
                    if nty.is_signed() {
                        writeln!(self.body, "  {t} = sdiv {ll} {cur}, {rhs_v}").ok()
                    } else {
                        writeln!(self.body, "  {t} = udiv {ll} {cur}, {rhs_v}").ok()
                    }
                }
                AssignOp::RemEq => {
                    if nty.is_signed() {
                        writeln!(self.body, "  {t} = srem {ll} {cur}, {rhs_v}").ok()
                    } else {
                        writeln!(self.body, "  {t} = urem {ll} {cur}, {rhs_v}").ok()
                    }
                }
                AssignOp::BitAndEq => {
                    writeln!(self.body, "  {t} = and {ll} {cur}, {rhs_v}").ok()
                }
                AssignOp::BitOrEq => writeln!(self.body, "  {t} = or {ll} {cur}, {rhs_v}").ok(),
                AssignOp::BitXorEq => {
                    writeln!(self.body, "  {t} = xor {ll} {cur}, {rhs_v}").ok()
                }
                AssignOp::ShlEq => writeln!(self.body, "  {t} = shl {ll} {cur}, {rhs_v}").ok(),
                AssignOp::ShrEq => {
                    if nty.is_signed() {
                        writeln!(self.body, "  {t} = ashr {ll} {cur}, {rhs_v}").ok()
                    } else {
                        writeln!(self.body, "  {t} = lshr {ll} {cur}, {rhs_v}").ok()
                    }
                }
                _ => {
                    return Err(diag(&format!(
                        "native ints: unsupported compound assign {op:?}"
                    )))
                }
            };
            t
        };
        writeln!(self.body, "  store {} {rhs}, ptr {ptr}", nty.llvm_ty()).ok();
        Ok(rhs)
    }

    fn emit_update(
        &mut self,
        op: UpdateOp,
        target: &UpdateTarget,
        prefix: bool,
        ty: &Type,
    ) -> Result<String, Diagnostic> {
        let UpdateTarget::Local(id) = target else {
            return Err(diag("native ints: only local ++/-- supported"));
        };
        let nty = match ty {
            Type::Native(n) if n.is_int() => *n,
            _ => self.local_native(*id)?,
        };
        let ptr = self
            .allocas
            .get(id)
            .cloned()
            .ok_or_else(|| diag("internal: update missing alloca"))?;
        let cur = self.fresh_tmp();
        writeln!(self.body, "  {cur} = load {}, ptr {ptr}", nty.llvm_ty()).ok();
        let next = self.fresh_tmp();
        match op {
            UpdateOp::Inc => {
                writeln!(self.body, "  {next} = add {} {cur}, 1", nty.llvm_ty()).ok()
            }
            UpdateOp::Dec => {
                writeln!(self.body, "  {next} = sub {} {cur}, 1", nty.llvm_ty()).ok()
            }
        };
        writeln!(self.body, "  store {} {next}, ptr {ptr}", nty.llvm_ty()).ok();
        if prefix {
            Ok(next)
        } else {
            Ok(cur)
        }
    }

    fn function_sig(&self, id: LocalId) -> Result<(Vec<NativeType>, NativeType), Diagnostic> {
        for stmt in &self.module.body {
            if let Stmt::Function {
                local,
                params,
                body,
                ..
            } = stmt
            {
                if *local == id {
                    let mut ptys = Vec::new();
                    for p in params {
                        let Pattern::Local(pid) = &p.pattern else {
                            return Err(diag("native ints: only simple params"));
                        };
                        ptys.push(self.local_native(*pid)?);
                    }
                    let ret = infer_return_native(body).unwrap_or(NativeType::I32);
                    return Ok((ptys, ret));
                }
            }
        }
        Err(diag("native ints: function not found for signature"))
    }

    fn local_native(&self, id: LocalId) -> Result<NativeType, Diagnostic> {
        let local = self
            .locals
            .get(&id)
            .ok_or_else(|| diag("internal: unknown local"))?;
        match local.ty {
            Type::Native(n) if n.is_int() => Ok(n),
            _ => Err(diag(&format!(
                "native ints: local `{}` is not a native integer",
                local.name
            ))),
        }
    }

    fn fresh_tmp(&mut self) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("%t{n}")
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label;
        self.label += 1;
        format!("{prefix}{n}")
    }
}

fn native_operand_ty(
    left: &Expr,
    right: &Expr,
    expect: Option<NativeType>,
) -> Result<NativeType, Diagnostic> {
    if let Type::Native(n) = left.ty() {
        if n.is_int() {
            return Ok(n);
        }
    }
    if let Type::Native(n) = right.ty() {
        if n.is_int() {
            return Ok(n);
        }
    }
    if let Some(n) = expect {
        return Ok(n);
    }
    Err(diag(
        "native ints: cannot determine integer type for operands",
    ))
}

fn format_int_const(raw: &str, ty: NativeType) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let (neg, digits) = if let Some(rest) = cleaned.strip_prefix('-') {
        (true, rest)
    } else {
        (false, cleaned.as_str())
    };

    let bits = parse_int_bits(digits)?;
    let width = ty.bit_width();
    let mask = if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    let mut v = bits & mask;
    if neg {
        // two's complement negate within width
        v = (!v).wrapping_add(1) & mask;
    }

    if ty.is_signed() {
        // sign-extend interpretation for printing as LLVM signed const
        let sign_bit = 1u64 << (width - 1);
        let signed = if v & sign_bit != 0 && width < 64 {
            (v | !mask) as i64
        } else if v & sign_bit != 0 && width == 64 {
            v as i64
        } else {
            v as i64
        };
        Ok(format!("{signed}"))
    } else {
        Ok(format!("{v}"))
    }
}

fn parse_int_bits(digits: &str) -> Result<u64, Diagnostic> {
    if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16)
            .map_err(|_| diag(&format!("invalid hex literal {digits}")));
    }
    if let Some(bin) = digits
        .strip_prefix("0b")
        .or_else(|| digits.strip_prefix("0B"))
    {
        return u64::from_str_radix(bin, 2)
            .map_err(|_| diag(&format!("invalid binary literal {digits}")));
    }
    if let Some(oct) = digits
        .strip_prefix("0o")
        .or_else(|| digits.strip_prefix("0O"))
    {
        return u64::from_str_radix(oct, 8)
            .map_err(|_| diag(&format!("invalid octal literal {digits}")));
    }
    // decimal — allow float-looking only if integral
    if digits.contains('.') || digits.contains('e') || digits.contains('E') {
        let f: f64 = digits
            .parse()
            .map_err(|_| diag(&format!("invalid numeric literal {digits}")))?;
        if f.fract() != 0.0 || f < 0.0 || f > u64::MAX as f64 {
            return Err(diag(&format!(
                "native ints: non-integral literal {digits}"
            )));
        }
        return Ok(f as u64);
    }
    digits
        .parse::<u64>()
        .map_err(|_| diag(&format!("invalid integer literal {digits}")))
}

fn infer_return_native(body: &[Stmt]) -> Option<NativeType> {
    for stmt in body {
        if let Some(n) = infer_return_native_stmt(stmt) {
            return Some(n);
        }
    }
    None
}

fn infer_return_native_stmt(stmt: &Stmt) -> Option<NativeType> {
    match stmt {
        Stmt::Return {
            value: Some(v), ..
        } => match v.ty() {
            Type::Native(n) if n.is_int() => Some(n),
            _ => None,
        },
        Stmt::Block { body } => infer_return_native(body),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => infer_return_native_stmt(consequent)
            .or_else(|| alternate.as_ref().and_then(|a| infer_return_native_stmt(a))),
        Stmt::While { body, .. } => infer_return_native_stmt(body),
        _ => None,
    }
}

fn collect_declared_locals(body: &[Stmt], out: &mut Vec<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Declare { local, .. } => out.push(*local),
            Stmt::Block { body } => collect_declared_locals(body, out),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                collect_declared_locals_stmt(consequent, out);
                if let Some(a) = alternate {
                    collect_declared_locals_stmt(a, out);
                }
            }
            Stmt::While { body, .. } => collect_declared_locals_stmt(body, out),
            _ => {}
        }
    }
}

fn collect_declared_locals_stmt(stmt: &Stmt, out: &mut Vec<LocalId>) {
    match stmt {
        Stmt::Declare { local, .. } => out.push(*local),
        Stmt::Block { body } => collect_declared_locals(body, out),
        other => collect_declared_locals(std::slice::from_ref(other), out),
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

