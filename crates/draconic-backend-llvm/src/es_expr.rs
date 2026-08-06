//! N08.01: emit native observations for ES expression Programs (E01.01 arithmetic, E01.02 comparison).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Expr, IrType as Type, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BOOL, PRINT_F64};

/// True when this module is a supported ES expression subset (E01.01 / E01.02 / N08.01.*):
/// top-level `let` declares over JS numbers and/or booleans with arithmetic, unary `+`/`-`,
/// comparison (`<` `<=` `>` `>=`), equality (`==` `!=` `===` `!==`), grouping, and local refs.
pub(crate) fn is_es_expr_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_expr(module: &Module) -> Result<String, Diagnostic> {
    let user = classify(module).ok_or_else(|| diag("internal: not an es_expr module"))?;
    let mut em = Emitter::new(module);
    em.emit_module(&user.user_locals)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    Boolean,
}

/// Top-level user locals in declaration order (observation order).
struct ModuleInfo {
    user_locals: Vec<(LocalId, SlotTy)>,
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
        let slot = match loc.ty {
            Type::Number => {
                if !expr_is_number_subset(init, &by_id) {
                    return None;
                }
                SlotTy::Number
            }
            Type::Boolean => {
                if !expr_is_boolean_subset(init, &by_id) {
                    return None;
                }
                SlotTy::Boolean
            }
            _ => return None,
        };
        user_locals.push((*local, slot));
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

fn expr_is_boolean_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Boolean { ty, .. } => *ty == Type::Boolean,
        Expr::Local { id, ty } => {
            *ty == Type::Boolean && by_id.get(id).is_some_and(|l| l.ty == Type::Boolean)
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            if *ty != Type::Boolean {
                return false;
            }
            match op {
                BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                    expr_is_number_subset(left, by_id) && expr_is_number_subset(right, by_id)
                }
                BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq => {
                    (expr_is_number_subset(left, by_id) && expr_is_number_subset(right, by_id))
                        || (expr_is_boolean_subset(left, by_id)
                            && expr_is_boolean_subset(right, by_id))
                }
                _ => false,
            }
        }
        _ => false,
    }
}

struct Emitter<'a> {
    module: &'a Module,
    /// local id → (alloca ptr name, slot type)
    allocas: HashMap<LocalId, (String, SlotTy)>,
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

    fn emit_module(&mut self, user: &[(LocalId, SlotTy)]) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.01 ES expressions via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        writeln!(self.out).ok();

        for (id, slot) in user {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, (ptr.clone(), *slot));
            match slot {
                SlotTy::Number => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                }
                SlotTy::Boolean => {
                    writeln!(self.body, "  {ptr} = alloca i1, align 1").ok();
                }
            }
        }

        for stmt in &self.module.body {
            let Stmt::Declare { local, init, .. } = stmt else {
                return Err(diag("internal: non-declare in es_expr module"));
            };
            let Some(init) = init else {
                return Err(diag("internal: declare without init in es_expr module"));
            };
            let (ptr, slot) = self
                .allocas
                .get(local)
                .cloned()
                .ok_or_else(|| diag("internal: missing alloca"))?;
            match slot {
                SlotTy::Number => {
                    let v = self.emit_number_expr(init)?;
                    writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                }
                SlotTy::Boolean => {
                    let v = self.emit_bool_expr(init)?;
                    writeln!(self.body, "  store i1 {v}, ptr {ptr}").ok();
                }
            }
        }

        // Print top-level user locals in declaration order.
        for (id, slot) in user {
            let (ptr, _) = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("internal: print missing alloca"))?;
            match slot {
                SlotTy::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::Boolean => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i1, ptr {ptr}").ok();
                    let ext = self.fresh();
                    writeln!(self.body, "  {ext} = zext i1 {v} to i8").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {ext}"))).ok();
                }
            }
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => Ok(format_number_const(raw)?),
            Expr::Local { id, .. } => {
                let (ptr, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated local %{}", id.0)))?;
                if slot != SlotTy::Number {
                    return Err(diag("internal: expected number local"));
                }
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Unary { op, arg, .. } => {
                let a = self.emit_number_expr(arg)?;
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
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let inst = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    BinaryOp::Rem => "frem",
                    _ => return Err(diag("internal: non-arithmetic binary in number emit")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            _ => Err(diag("internal: unsupported number expr in es_expr module")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Boolean { value, .. } => Ok(if *value { "true".into() } else { "false".into() }),
            Expr::Local { id, .. } => {
                let (ptr, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated local %{}", id.0)))?;
                if slot != SlotTy::Boolean {
                    return Err(diag("internal: expected boolean local"));
                }
                let t = self.fresh();
                writeln!(self.body, "  {t} = load i1, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Binary {
                left, op, right, ..
            } => match op {
                BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
                    if expr_ty_is_number(left) =>
                {
                    let l = self.emit_number_expr(left)?;
                    let r = self.emit_number_expr(right)?;
                    let pred = match op {
                        BinaryOp::EqEq | BinaryOp::EqEqEq => "oeq",
                        BinaryOp::NotEq | BinaryOp::NotEqEq => "one",
                        BinaryOp::Lt => "olt",
                        BinaryOp::LtEq => "ole",
                        BinaryOp::Gt => "ogt",
                        BinaryOp::GtEq => "oge",
                        _ => unreachable!(),
                    };
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = fcmp {pred} double {l}, {r}").ok();
                    Ok(t)
                }
                BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq => {
                    let l = self.emit_bool_expr(left)?;
                    let r = self.emit_bool_expr(right)?;
                    let pred = match op {
                        BinaryOp::EqEq | BinaryOp::EqEqEq => "eq",
                        BinaryOp::NotEq | BinaryOp::NotEqEq => "ne",
                        _ => unreachable!(),
                    };
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = icmp {pred} i1 {l}, {r}").ok();
                    Ok(t)
                }
                _ => Err(diag("internal: non-comparison binary in bool emit")),
            },
            _ => Err(diag("internal: unsupported bool expr in es_expr module")),
        }
    }

    fn finish(self) -> String {
        self.out
    }
}

fn expr_ty_is_number(expr: &Expr) -> bool {
    matches!(expr.ty(), Type::Number)
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
