//! N08.03.01: emit native observations for ES function declaration + return + call
//! (no params) — E03.01 / `es/functions/decl_return_call`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Expr, IrType as Type, Local, LocalId, Module, Param, Stmt};
use draconic_runtime::abi::{llvm_declares, PRINT_F64};

/// True when this module is the supported ES function subset (E03.01 / N08.03.01):
/// top-level `function f() { return <number>; }` + `let x = f()` (number/any slot).
pub(crate) fn is_es_functions_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_functions(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_functions module"))?;
    let mut em = Emitter::new(module);
    em.emit_module(&info)?;
    Ok(em.finish())
}

struct ModuleInfo {
    /// Function local → body (no params; number return).
    functions: Vec<(LocalId, Vec<Stmt>)>,
    /// Top-level user locals to print (declare order).
    user_locals: Vec<LocalId>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut fn_ids = std::collections::HashSet::new();
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
                if !params_ok(params) {
                    return None;
                }
                if !fn_body_ok(body, &by_id, &fn_ids) {
                    return None;
                }
                has_fn = true;
                fn_ids.insert(*local);
                functions.push((*local, body.clone()));
            }
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                match loc.ty {
                    Type::Number | Type::Any => {
                        let init = init.as_ref()?;
                        if !number_expr_ok(init, &by_id, &fn_ids) {
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

fn params_ok(params: &[Param]) -> bool {
    // N08.03.01: no parameters.
    params.is_empty()
}

fn fn_body_ok(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_ids: &std::collections::HashSet<LocalId>,
) -> bool {
    body.iter().all(|s| match s {
        Stmt::Return { value: Some(v) } => number_expr_ok(v, by_id, fn_ids),
        Stmt::Return { value: None } => false,
        Stmt::Block { body } => fn_body_ok(body, by_id, fn_ids),
        _ => false,
    })
}

fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_ids: &std::collections::HashSet<LocalId>,
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
        } => number_expr_ok(arg, by_id, fn_ids),
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            use draconic_ast::BinaryOp::*;
            matches!(op, Add | Sub | Mul | Div | Rem)
                && number_expr_ok(left, by_id, fn_ids)
                && number_expr_ok(right, by_id, fn_ids)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional || !args.is_empty() {
                return false;
            }
            match callee.as_ref() {
                Expr::Local { id, .. } => fn_ids.contains(id),
                _ => false,
            }
        }
        _ => false,
    }
}

struct Emitter<'a> {
    module: &'a Module,
    fn_names: HashMap<LocalId, String>,
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
            "; Draconic LLVM backend (N08.03.01 ES function decl/return/call via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(&[PRINT_F64])).ok();
        writeln!(self.out).ok();

        for (local, body) in &info.functions {
            self.emit_function(*local, body)?;
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

    fn emit_function(&mut self, local: LocalId, body: &[Stmt]) -> Result<(), Diagnostic> {
        let fn_name = self
            .fn_names
            .get(&local)
            .cloned()
            .ok_or_else(|| diag("internal: missing function name"))?;

        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_allocas = std::mem::take(&mut self.allocas);

        self.tmp = 0;
        self.allocas.clear();

        for stmt in body {
            self.emit_fn_stmt(stmt)?;
        }
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
        }

        writeln!(self.out, "define double @{fn_name}() {{").ok();
        writeln!(self.out, "entry:").ok();
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
                if !args.is_empty() {
                    return Err(diag("es_functions: only zero-arg calls supported"));
                }
                let Expr::Local { id, .. } = callee.as_ref() else {
                    return Err(diag("es_functions: only direct function calls supported"));
                };
                let fn_name = self
                    .fn_names
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_functions: call to unknown function"))?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = call double @{fn_name}()").ok();
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
