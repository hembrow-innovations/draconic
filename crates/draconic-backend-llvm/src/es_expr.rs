//! N08.01.01: emit native observations for numeric-arithmetic Programs (E01.01).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Expr, IrType as Type, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64};

/// True when this module is the supported numeric-arithmetic subset (E01.01 / N08.01.01):
/// top-level `let` declares with `+` `-` `*` `/` `%`, unary `+`/`-`, grouping and
/// local references, all over JS numbers.
pub(crate) fn is_es_expr_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_expr(module: &Module) -> Result<String, Diagnostic> {
    let user = classify(module).ok_or_else(|| diag("internal: not an es_expr module"))?;
    let mut em = Emitter::new(module);
    em.emit_module(&user.user_locals)?;
    Ok(em.finish())
}

/// Top-level user number locals in declaration order (observation order).
struct ModuleInfo {
    user_locals: Vec<LocalId>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_locals = Vec::new();
    for stmt in &module.body {
        let Stmt::Declare {
            local,
            init: Some(init),
            ..
        } = stmt
        else {
            return None;
        };
        let loc = by_id.get(local)?;
        if loc.ty != Type::Number {
            return None;
        }
        if !expr_is_number_subset(init, &by_id) {
            return None;
        }
        user_locals.push(*local);
    }
    if user_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo { user_locals })
}

fn expr_is_number_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { ty, .. } => *ty == Type::Number,
        Expr::Local { id, ty } => {
            *ty == Type::Number && by_id.get(id).is_some_and(|l| l.ty == Type::Number)
        }
        Expr::Unary { op, arg, ty } => {
            *ty == Type::Number
                && matches!(op, UnaryOp::Minus | UnaryOp::Plus)
                && expr_is_number_subset(arg, by_id)
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            *ty == Type::Number
                && matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
                )
                && expr_is_number_subset(left, by_id)
                && expr_is_number_subset(right, by_id)
        }
        _ => false,
    }
}

struct Emitter<'a> {
    module: &'a Module,
    /// local id → alloca ptr name (`%lN`)
    allocas: HashMap<LocalId, String>,
    out: String,
    body: String,
    tmp: u32,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        Self {
            module,
            allocas: HashMap::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
        }
    }

    fn fresh(&mut self) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("%t{n}")
    }

    fn emit_module(&mut self, user: &[LocalId]) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.01.01 numeric arithmetic via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        writeln!(self.out).ok();

        for id in user {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
        }

        for stmt in &self.module.body {
            let Stmt::Declare { local, init, .. } = stmt else {
                return Err(diag("internal: non-declare in es_expr module"));
            };
            let Some(init) = init else {
                return Err(diag("internal: declare without init in es_expr module"));
            };
            let ptr = self
                .allocas
                .get(local)
                .cloned()
                .ok_or_else(|| diag("internal: missing alloca"))?;
            let v = self.emit_expr(init)?;
            writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
        }

        // Print top-level user number locals in declaration order.
        for id in user {
            let ptr = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("internal: print missing alloca"))?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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
            Expr::Unary { op, arg, .. } => {
                let a = self.emit_expr(arg)?;
                match op {
                    UnaryOp::Plus => Ok(a),
                    UnaryOp::Minus => {
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = fneg double {a}").ok();
                        Ok(t)
                    }
                    _ => Err(diag("internal: non-arithmetic unary in es_expr module")),
                }
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.emit_expr(left)?;
                let r = self.emit_expr(right)?;
                let inst = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    BinaryOp::Rem => "frem",
                    _ => return Err(diag("internal: non-arithmetic binary in es_expr module")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            _ => Err(diag("internal: unsupported expr in es_expr module")),
        }
    }

    fn finish(self) -> String {
        self.out
    }
}

/// Format a JS number literal as an LLVM `double` constant (decimal, round-trip safe).
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
