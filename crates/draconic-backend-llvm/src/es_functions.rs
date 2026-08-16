//! N08.03.01–N08.03.02: emit native observations for ES function declaration +
//! return + call (simple ident params) — E03.01–E03.02 /
//! `es/functions/decl_return_call`, `es/functions/params_call`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, IrType as Type, Local, LocalId, Module, Param, Pattern, Stmt};
use draconic_runtime::abi::{llvm_declares, PRINT_F64};

/// True when this module is the supported ES function subset (E03.01–E03.02 /
/// N08.03.01–N08.03.02): top-level `function f(a, b) { return <number>; }` +
/// `let x = f(...)` (number/any slots; simple ident params only).
pub(crate) fn is_es_functions_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_functions(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_functions module"))?;
    let mut em = Emitter::new(module);
    em.emit_module(&info)?;
    Ok(em.finish())
}

struct FnInfo {
    local: LocalId,
    /// Simple ident param locals (order).
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

struct ModuleInfo {
    functions: Vec<FnInfo>,
    /// Top-level user locals to print (declare order).
    user_locals: Vec<LocalId>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut fn_arities: HashMap<LocalId, usize> = HashMap::new();
    let mut user_locals = Vec::new();
    let mut has_fn = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
            } => {
                if *is_async || *is_generator {
                    return None;
                }
                let param_ids = simple_param_locals(params, &by_id)?;
                if !fn_body_ok(body, &by_id, &fn_arities) {
                    return None;
                }
                has_fn = true;
                fn_arities.insert(*local, param_ids.len());
                functions.push(FnInfo {
                    local: *local,
                    params: param_ids,
                    body: body.clone(),
                });
            }
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                match loc.ty {
                    Type::Number | Type::Any => {
                        let init = init.as_ref()?;
                        if !number_expr_ok(init, &by_id, &fn_arities) {
                            return None;
                        }
                        user_locals.push(*local);
                    }
                    _ => return None,
                }
            }
            _ => return None,
        }
    }

    if !has_fn || user_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        functions,
        user_locals,
    })
}

/// N08.03.02: simple ident params only (no default, rest, or destructure).
fn simple_param_locals(
    params: &[Param],
    by_id: &HashMap<LocalId, &Local>,
) -> Option<Vec<LocalId>> {
    let mut out = Vec::with_capacity(params.len());
    for p in params {
        if p.rest || p.default.is_some() {
            return None;
        }
        let Pattern::Local(id) = &p.pattern else {
            return None;
        };
        let loc = by_id.get(id)?;
        if !matches!(loc.ty, Type::Number | Type::Any) {
            return None;
        }
        out.push(*id);
    }
    Some(out)
}

fn fn_body_ok(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
) -> bool {
    body.iter().all(|s| match s {
        Stmt::Return { value: Some(v) } => number_expr_ok(v, by_id, fn_arities),
        Stmt::Return { value: None } => false,
        Stmt::Block { body } => fn_body_ok(body, by_id, fn_arities),
        _ => false,
    })
}

fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::Number | Type::Any)
                && by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::Unary {
            op: draconic_ast::UnaryOp::Plus | draconic_ast::UnaryOp::Minus,
            arg,
            ..
        } => number_expr_ok(arg, by_id, fn_arities),
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            use draconic_ast::BinaryOp::*;
            matches!(op, Add | Sub | Mul | Div | Rem)
                && number_expr_ok(left, by_id, fn_arities)
                && number_expr_ok(right, by_id, fn_arities)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return false;
            }
            if !args.iter().all(|a| match a {
                Arg::Expr(e) => number_expr_ok(e, by_id, fn_arities),
                Arg::Spread(_) => false,
            }) {
                return false;
            }
            match callee.as_ref() {
                Expr::Local { id, .. } => fn_arities.get(id).is_some_and(|n| *n == args.len()),
                _ => false,
            }
        }
        _ => false,
    }
}

struct Emitter<'a> {
    module: &'a Module,
    fn_names: HashMap<LocalId, String>,
    /// Function local → param locals (for call arity / signature).
    fn_params: HashMap<LocalId, Vec<LocalId>>,
    allocas: HashMap<LocalId, String>,
    out: String,
    body: String,
    tmp: u32,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        let locals: HashMap<LocalId, &Local> =
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
            fn_names,
            fn_params: HashMap::new(),
            allocas: HashMap::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("%t{t}")
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.03.02 ES function params/call via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(&[PRINT_F64])).ok();
        writeln!(self.out).ok();

        for f in &info.functions {
            self.fn_params.insert(f.local, f.params.clone());
        }

        for f in &info.functions {
            self.emit_function(f)?;
        }

        self.body.clear();
        self.tmp = 0;
        self.allocas.clear();

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();

        for id in &info.user_locals {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            writeln!(self.out, "  {ptr} = alloca double, align 8").ok();
        }

        for stmt in &self.module.body {
            if matches!(stmt, Stmt::Function { .. }) {
                continue;
            }
            self.emit_stmt(stmt)?;
        }

        for id in &info.user_locals {
            let ptr = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("internal: print missing alloca"))?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
        }

        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_function(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let fn_name = self
            .fn_names
            .get(&f.local)
            .cloned()
            .ok_or_else(|| diag("internal: missing function name"))?;

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_allocas = std::mem::take(&mut self.allocas);

        self.tmp = 0;
        self.allocas.clear();

        // Param signature: double %p0, double %p1, ...
        let mut sig_parts = Vec::new();
        for (i, _) in f.params.iter().enumerate() {
            sig_parts.push(format!("double %p{i}"));
        }
        let sig = sig_parts.join(", ");

        // Entry: alloca + store each param, then body.
        let mut entry = String::new();
        for (i, pid) in f.params.iter().enumerate() {
            let ptr = format!("%l{}", pid.0);
            self.allocas.insert(*pid, ptr.clone());
            writeln!(entry, "  {ptr} = alloca double, align 8").ok();
            writeln!(entry, "  store double %p{i}, ptr {ptr}").ok();
        }

        for stmt in &f.body {
            self.emit_fn_stmt(stmt)?;
        }
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
        }

        writeln!(self.out, "define double @{fn_name}({sig}) {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{entry}").ok();
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.allocas = saved_allocas;
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

    fn emit_fn_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(v) } => {
                let n = self.emit_number_expr(v)?;
                writeln!(self.body, "  ret double {n}").ok();
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    if self.body_ends_with_terminator() {
                        break;
                    }
                    self.emit_fn_stmt(s)?;
                }
                Ok(())
            }
            _ => Err(diag("es_functions: unsupported stmt in function body")),
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let ptr = self
                    .allocas
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("internal: missing alloca"))?;
                let init = init
                    .as_ref()
                    .ok_or_else(|| diag("es_functions: declare requires init"))?;
                let v = self.emit_number_expr(init)?;
                writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                Ok(())
            }
            Stmt::Function { .. } => Ok(()),
            _ => Err(diag("es_functions: unsupported top-level stmt")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => Ok(format_number_const(raw)?),
            Expr::Local { id, .. } => {
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated local %{}", id.0)))?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Unary {
                op: draconic_ast::UnaryOp::Plus,
                arg,
                ..
            } => self.emit_number_expr(arg),
            Expr::Unary {
                op: draconic_ast::UnaryOp::Minus,
                arg,
                ..
            } => {
                let a = self.emit_number_expr(arg)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = fneg double {a}").ok();
                Ok(t)
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                use draconic_ast::BinaryOp::*;
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let inst = match op {
                    Add => "fadd",
                    Sub => "fsub",
                    Mul => "fmul",
                    Div => "fdiv",
                    Rem => "frem",
                    _ => return Err(diag("es_functions: unsupported binary")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_functions: optional call not supported"));
                }
                let Expr::Local { id, .. } = callee.as_ref() else {
                    return Err(diag("es_functions: only direct function calls supported"));
                };
                let fn_name = self
                    .fn_names
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_functions: call to unknown function"))?;
                let expected = self
                    .fn_params
                    .get(id)
                    .map(|p| p.len())
                    .ok_or_else(|| diag("es_functions: call to unknown function"))?;
                if args.len() != expected {
                    return Err(diag("es_functions: call arity mismatch"));
                }
                let mut arg_vals = Vec::with_capacity(args.len());
                for a in args {
                    match a {
                        Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                        Arg::Spread(_) => {
                            return Err(diag("es_functions: spread args not supported"));
                        }
                    }
                }
                let t = self.fresh();
                if arg_vals.is_empty() {
                    writeln!(self.body, "  {t} = call double @{fn_name}()").ok();
                } else {
                    let parts: Vec<String> = arg_vals
                        .iter()
                        .map(|v| format!("double {v}"))
                        .collect();
                    writeln!(
                        self.body,
                        "  {t} = call double @{fn_name}({})",
                        parts.join(", ")
                    )
                    .ok();
                }
                Ok(t)
            }
            _ => Err(diag("es_functions: unsupported number expr")),
        }
    }
}

fn format_number_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(format!("invalid number literal {raw}")))?;
    Ok(format!("{f:.17e}"))
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
