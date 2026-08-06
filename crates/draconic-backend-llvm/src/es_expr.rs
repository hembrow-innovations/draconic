//! N08.01: emit native observations for ES expression Programs
//! (E01.01 arithmetic, E01.02 comparison, E01.03 logical, E01.04.01 bitwise, E01.04.02 `**`,
//! E01.04.03 conditional `?:`).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Expr, IrType as Type, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BOOL, PRINT_F64};

/// True when this module is a supported ES expression subset (E01.01–E01.04.03 / N08.01.*):
/// top-level `let` declares over JS numbers and/or booleans with arithmetic, unary `+`/`-`/`!`/`~`,
/// comparison (`<` `<=` `>` `>=`), equality (`==` `!=` `===` `!==`), logical (`&&` `||`),
/// bitwise (`&` `|` `^` `<<` `>>` `>>>`), exponentiation (`**`), conditional (`?:`), grouping,
/// and local refs. Value-preserving `&&`/`||` on numbers is included.
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
                && matches!(op, UnaryOp::Minus | UnaryOp::Plus | UnaryOp::BitNot)
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
                    BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Rem
                        | BinaryOp::And
                        | BinaryOp::Or
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Shl
                        | BinaryOp::Shr
                        | BinaryOp::UShr
                        | BinaryOp::Pow
                )
                && expr_is_number_subset(left, by_id)
                && expr_is_number_subset(right, by_id)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ty,
        } => {
            *ty == Type::Number
                && (expr_is_boolean_subset(test, by_id) || expr_is_number_subset(test, by_id))
                && expr_is_number_subset(consequent, by_id)
                && expr_is_number_subset(alternate, by_id)
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
        Expr::Unary { op, arg, ty } => {
            *ty == Type::Boolean && matches!(op, UnaryOp::Not) && expr_is_boolean_subset(arg, by_id)
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
                BinaryOp::And | BinaryOp::Or => {
                    expr_is_boolean_subset(left, by_id) && expr_is_boolean_subset(right, by_id)
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
        // JS `**` → IEEE pow (Math.pow); intrinsic available without libm link flags.
        writeln!(self.out, "declare double @llvm.pow.f64(double, double)").ok();
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
                    // JS `~`: ToInt32 then bitwise not; result as Number.
                    UnaryOp::BitNot => {
                        let i = self.fresh();
                        writeln!(self.body, "  {i} = fptosi double {a} to i32").ok();
                        let n = self.fresh();
                        writeln!(self.body, "  {n} = xor i32 {i}, -1").ok();
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = sitofp i32 {n} to double").ok();
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
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                        let inst = match op {
                            BinaryOp::Add => "fadd",
                            BinaryOp::Sub => "fsub",
                            BinaryOp::Mul => "fmul",
                            BinaryOp::Div => "fdiv",
                            BinaryOp::Rem => "frem",
                            _ => unreachable!(),
                        };
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                        Ok(t)
                    }
                    // Value-preserving JS && / || on numbers (ToBoolean via nonzero).
                    BinaryOp::And | BinaryOp::Or => {
                        let truthy = self.fresh();
                        // +0/-0/NaN are falsy; `one` is ordered-and-unequal.
                        writeln!(self.body, "  {truthy} = fcmp one double {l}, 0.00000000000000000e+00")
                            .ok();
                        let t = self.fresh();
                        match op {
                            BinaryOp::And => {
                                writeln!(
                                    self.body,
                                    "  {t} = select i1 {truthy}, double {r}, double {l}"
                                )
                                .ok();
                            }
                            BinaryOp::Or => {
                                writeln!(
                                    self.body,
                                    "  {t} = select i1 {truthy}, double {l}, double {r}"
                                )
                                .ok();
                            }
                            _ => unreachable!(),
                        }
                        Ok(t)
                    }
                    // JS bitwise on Numbers: ToInt32 (or ToUint32 for >>>), then int op.
                    BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr
                    | BinaryOp::UShr => self.emit_bitwise_number(op, &l, &r),
                    // JS `**` (Math.pow): IEEE floating pow on Number doubles.
                    BinaryOp::Pow => {
                        let t = self.fresh();
                        writeln!(
                            self.body,
                            "  {t} = call double @llvm.pow.f64(double {l}, double {r})"
                        )
                        .ok();
                        Ok(t)
                    }
                    _ => Err(diag("internal: non-arithmetic binary in number emit")),
                }
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                let cond = self.emit_to_boolean(test)?;
                let c = self.emit_number_expr(consequent)?;
                let a = self.emit_number_expr(alternate)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {t} = select i1 {cond}, double {c}, double {a}"
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("internal: unsupported number expr in es_expr module")),
        }
    }

    /// JS ToBoolean for number/boolean tests (ternary / value-preserving branches).
    fn emit_to_boolean(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr.ty() {
            Type::Boolean => self.emit_bool_expr(expr),
            Type::Number => {
                let n = self.emit_number_expr(expr)?;
                let t = self.fresh();
                // +0/-0/NaN falsy; nonzero truthy (`one` = ordered-and-unequal).
                writeln!(
                    self.body,
                    "  {t} = fcmp one double {n}, 0.00000000000000000e+00"
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("internal: ToBoolean expects number or boolean")),
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
            Expr::Unary { op, arg, .. } => match op {
                UnaryOp::Not => {
                    let a = self.emit_bool_expr(arg)?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = xor i1 {a}, true").ok();
                    Ok(t)
                }
                _ => Err(diag("internal: non-logical unary in bool emit")),
            },
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
                BinaryOp::And | BinaryOp::Or => {
                    let l = self.emit_bool_expr(left)?;
                    let r = self.emit_bool_expr(right)?;
                    let inst = match op {
                        BinaryOp::And => "and",
                        BinaryOp::Or => "or",
                        _ => unreachable!(),
                    };
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = {inst} i1 {l}, {r}").ok();
                    Ok(t)
                }
                _ => Err(diag("internal: non-comparison binary in bool emit")),
            },
            _ => Err(diag("internal: unsupported bool expr in es_expr module")),
        }
    }

    /// Emit JS bitwise op on two number SSA values (doubles). Shift count masked to 5 bits.
    fn emit_bitwise_number(
        &mut self,
        op: &BinaryOp,
        l: &str,
        r: &str,
    ) -> Result<String, Diagnostic> {
        let li = self.fresh();
        writeln!(self.body, "  {li} = fptosi double {l} to i32").ok();
        let ri = self.fresh();
        writeln!(self.body, "  {ri} = fptosi double {r} to i32").ok();
        match op {
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                let inst = match op {
                    BinaryOp::BitAnd => "and",
                    BinaryOp::BitOr => "or",
                    BinaryOp::BitXor => "xor",
                    _ => unreachable!(),
                };
                let n = self.fresh();
                writeln!(self.body, "  {n} = {inst} i32 {li}, {ri}").ok();
                let t = self.fresh();
                writeln!(self.body, "  {t} = sitofp i32 {n} to double").ok();
                Ok(t)
            }
            BinaryOp::Shl | BinaryOp::Shr | BinaryOp::UShr => {
                let shift = self.fresh();
                writeln!(self.body, "  {shift} = and i32 {ri}, 31").ok();
                let n = self.fresh();
                match op {
                    BinaryOp::Shl => {
                        writeln!(self.body, "  {n} = shl i32 {li}, {shift}").ok();
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = sitofp i32 {n} to double").ok();
                        Ok(t)
                    }
                    BinaryOp::Shr => {
                        writeln!(self.body, "  {n} = ashr i32 {li}, {shift}").ok();
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = sitofp i32 {n} to double").ok();
                        Ok(t)
                    }
                    BinaryOp::UShr => {
                        writeln!(self.body, "  {n} = lshr i32 {li}, {shift}").ok();
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = uitofp i32 {n} to double").ok();
                        Ok(t)
                    }
                    _ => unreachable!(),
                }
            }
            _ => Err(diag("internal: not a bitwise op")),
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
