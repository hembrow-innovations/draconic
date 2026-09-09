use std::collections::HashMap;
use std::fmt::Write as _;

use crate::emitter::SlotTy;
use draconic_ast::{AssignOp, BinaryOp, UnaryOp, UpdateOp};
use draconic_diagnostics::Diagnostic;
use draconic_ir::{Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, UpdateTarget};
use draconic_runtime::abi::{CSTR_EQ_N, UTF16_LEN};

use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        // N08.07.01: `s.length`
        if let Expr::Member {
            object,
            property,
            computed: false,
            optional: false,
            ..
        } = expr
        {
            if matches!(
                property.as_ref(),
                Expr::String { value, .. } if value.to_string_lossy() == "length"
            ) {
                let s = self.emit_string_expr(object)?;
                // N08.07.03: JS `.length` is UTF-16 code units (storage remains UTF-8 bytes).
                let units = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    UTF16_LEN.call_to(&units, &format!("ptr {}, i64 {}", s.data, s.len))
                )
                .ok();
                let t = self.fresh();
                writeln!(self.body, "  {t} = uitofp i64 {units} to double").ok();
                return Ok(t);
            }
        }
        // N08.08.05: `Math.PI` / `Math.E` / …
        if let Some(name) = self.math_member_name_emit(expr) {
            if is_math_const_name(&name) {
                return format_math_const(&name);
            }
        }
        // N08.08.06: `Number.NaN` / `Number.MAX_VALUE` / …
        if let Some(name) = self.number_ctor_member_name_emit(expr) {
            if is_number_ctor_const_name(&name) {
                return format_number_ctor_const(&name);
            }
        }
        // N08.08.05: `Math.abs(…)` / `Math["abs"](…)` / …
        if let Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } = expr
        {
            if let Some(name) = self.math_member_name_emit(callee) {
                if is_math_method_name(&name) {
                    return self.emit_math_call(&name, args);
                }
            }
        }
        match expr {
            Expr::Number { raw, .. } => Ok(format_number_const(raw)?),
            Expr::Local { id, .. } => {
                // N08.08.06: host `NaN` / `Infinity` are not stack-allocated.
                let by_id: HashMap<LocalId, &Local> =
                    self.module.locals.iter().map(|l| (l.id, l)).collect();
                if let Some(name) = nan_or_infinity_name(*id, &by_id) {
                    return format_number_ctor_const(name);
                }
                let (ptr, slot) = self
                    .state
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
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem => {
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
                        writeln!(
                            self.body,
                            "  {truthy} = fcmp one double {l}, 0.00000000000000000e+00"
                        )
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
                    // Comma: evaluate LHS for effects, yield RHS (left already emitted above).
                    BinaryOp::Comma => Ok(r),
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
            Expr::Assign {
                target, op, value, ..
            } => {
                let AssignTarget::Local(id) = target else {
                    return Err(diag("internal: only local assign in es_expr"));
                };
                let (ptr, slot) =
                    self.state.allocas.get(id).cloned().ok_or_else(|| {
                        diag(format!("internal: unallocated assign local %{}", id.0))
                    })?;
                if slot != SlotTy::Number {
                    return Err(diag("internal: expected number assign target"));
                }
                if matches!(op, AssignOp::Eq) {
                    let v = self.emit_number_expr(value)?;
                    writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    return Ok(v);
                }
                if !is_number_assign_op(*op) {
                    return Err(diag(
                        "internal: unsupported assign op in es_expr number assign",
                    ));
                }
                // ES order: GetValue(lhs) then evaluate RHS, then apply op and PutValue.
                let cur = self.fresh();
                writeln!(self.body, "  {cur} = load double, ptr {ptr}").ok();
                let r = self.emit_number_expr(value)?;
                let v = match op {
                    AssignOp::AddEq
                    | AssignOp::SubEq
                    | AssignOp::MulEq
                    | AssignOp::DivEq
                    | AssignOp::RemEq => {
                        let inst = match op {
                            AssignOp::AddEq => "fadd",
                            AssignOp::SubEq => "fsub",
                            AssignOp::MulEq => "fmul",
                            AssignOp::DivEq => "fdiv",
                            AssignOp::RemEq => "frem",
                            _ => unreachable!(),
                        };
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = {inst} double {cur}, {r}").ok();
                        t
                    }
                    AssignOp::PowEq => {
                        let t = self.fresh();
                        writeln!(
                            self.body,
                            "  {t} = call double @llvm.pow.f64(double {cur}, double {r})"
                        )
                        .ok();
                        t
                    }
                    AssignOp::BitAndEq
                    | AssignOp::BitOrEq
                    | AssignOp::BitXorEq
                    | AssignOp::ShlEq
                    | AssignOp::ShrEq
                    | AssignOp::UShrEq => {
                        let bop = match op {
                            AssignOp::BitAndEq => BinaryOp::BitAnd,
                            AssignOp::BitOrEq => BinaryOp::BitOr,
                            AssignOp::BitXorEq => BinaryOp::BitXor,
                            AssignOp::ShlEq => BinaryOp::Shl,
                            AssignOp::ShrEq => BinaryOp::Shr,
                            AssignOp::UShrEq => BinaryOp::UShr,
                            _ => unreachable!(),
                        };
                        self.emit_bitwise_number(&bop, &cur, &r)?
                    }
                    _ => {
                        return Err(diag(
                            "internal: unsupported compound assign in es_expr number assign",
                        ))
                    }
                };
                writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Update {
                op, target, prefix, ..
            } => {
                let UpdateTarget::Local(id) = target else {
                    return Err(diag("internal: only local ++/-- in es_expr"));
                };
                let (ptr, slot) =
                    self.state.allocas.get(id).cloned().ok_or_else(|| {
                        diag(format!("internal: unallocated update local %{}", id.0))
                    })?;
                if slot != SlotTy::Number {
                    return Err(diag("internal: expected number update target"));
                }
                let cur = self.fresh();
                writeln!(self.body, "  {cur} = load double, ptr {ptr}").ok();
                let next = self.fresh();
                match op {
                    UpdateOp::Inc => {
                        writeln!(
                            self.body,
                            "  {next} = fadd double {cur}, 1.00000000000000000e+00"
                        )
                        .ok();
                    }
                    UpdateOp::Dec => {
                        writeln!(
                            self.body,
                            "  {next} = fsub double {cur}, 1.00000000000000000e+00"
                        )
                        .ok();
                    }
                }
                writeln!(self.body, "  store double {next}, ptr {ptr}").ok();
                if *prefix {
                    Ok(next)
                } else {
                    Ok(cur)
                }
            }
            _ => Err(diag("internal: unsupported number expr in es_expr module")),
        }
    }

    /// JS ToBoolean for number/boolean tests (ternary / value-preserving branches).
    pub(super) fn emit_to_boolean(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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

    pub(super) fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        // N08.08.06: `Number.isNaN(…)` / `isFinite` / `isInteger` / `isSafeInteger`.
        if let Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } = expr
        {
            if let Some(name) = self.number_ctor_member_name_emit(callee) {
                if is_number_ctor_method_name(&name) {
                    return self.emit_number_ctor_bool_call(&name, args);
                }
            }
        }
        match expr {
            Expr::Boolean { value, .. } => Ok(if *value {
                "true".into()
            } else {
                "false".into()
            }),
            Expr::Local { id, .. } => {
                let (ptr, slot) = self
                    .state
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
                // `delete` non-reference → true (evaluate arg for effects).
                UnaryOp::Delete => {
                    self.emit_discard_arg(arg)?;
                    Ok("true".into())
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
                    if expr_ty_is_number(left) || self.expr_emits_as_number(left) =>
                {
                    let l = self.emit_number_expr(left)?;
                    let r = self.emit_number_expr(right)?;
                    // `une` so NaN !== NaN is true (ES Number equality; `one` is false for NaN).
                    let pred = match op {
                        BinaryOp::EqEq | BinaryOp::EqEqEq => "oeq",
                        BinaryOp::NotEq | BinaryOp::NotEqEq => "une",
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
                // N08.08.03: same-type BigInt comparison / equality (signed i64).
                BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
                    if expr_ty_is_bigint(left) =>
                {
                    let l = self.emit_bigint_expr(left)?;
                    let r = self.emit_bigint_expr(right)?;
                    let pred = match op {
                        BinaryOp::EqEq | BinaryOp::EqEqEq => "eq",
                        BinaryOp::NotEq | BinaryOp::NotEqEq => "ne",
                        BinaryOp::Lt => "slt",
                        BinaryOp::LtEq => "sle",
                        BinaryOp::Gt => "sgt",
                        BinaryOp::GtEq => "sge",
                        _ => unreachable!(),
                    };
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = icmp {pred} i64 {l}, {r}").ok();
                    Ok(t)
                }
                BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq
                    if expr_ty_is_string(left) || matches!(left.ty(), Type::String) =>
                {
                    let l = self.emit_string_expr(left)?;
                    let r = self.emit_string_expr(right)?;
                    let eq = self.fresh();
                    writeln!(
                        self.body,
                        "  {}",
                        CSTR_EQ_N.call_to(
                            &eq,
                            &format!(
                                "ptr {}, i64 {}, ptr {}, i64 {}",
                                l.data, l.len, r.data, r.len
                            )
                        )
                    )
                    .ok();
                    let t = self.fresh();
                    match op {
                        BinaryOp::EqEq | BinaryOp::EqEqEq => {
                            writeln!(self.body, "  {t} = icmp ne i32 {eq}, 0").ok();
                        }
                        BinaryOp::NotEq | BinaryOp::NotEqEq => {
                            writeln!(self.body, "  {t} = icmp eq i32 {eq}, 0").ok();
                        }
                        _ => unreachable!(),
                    }
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
                BinaryOp::Comma => {
                    self.emit_discard_arg(left)?;
                    self.emit_bool_expr(right)
                }
                _ => Err(diag("internal: non-comparison binary in bool emit")),
            },
            Expr::Assign {
                target, op, value, ..
            } => {
                if !matches!(op, AssignOp::Eq) {
                    return Err(diag("internal: only simple = in es_expr bool assign"));
                }
                let AssignTarget::Local(id) = target else {
                    return Err(diag("internal: only local assign in es_expr"));
                };
                let (ptr, slot) =
                    self.state.allocas.get(id).cloned().ok_or_else(|| {
                        diag(format!("internal: unallocated assign local %{}", id.0))
                    })?;
                if slot != SlotTy::Boolean {
                    return Err(diag("internal: expected boolean assign target"));
                }
                let v = self.emit_bool_expr(value)?;
                writeln!(self.body, "  store i1 {v}, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("internal: unsupported bool expr in es_expr module")),
        }
    }

    /// Resolve `Math.prop` / `Math["prop"]` property name during emit.
    pub(super) fn math_member_name_emit(&self, expr: &Expr) -> Option<String> {
        let by_id: HashMap<LocalId, &Local> =
            self.module.locals.iter().map(|l| (l.id, l)).collect();
        math_member_name(expr, &by_id)
    }

    /// Resolve `Number.prop` / `Number["prop"]` property name during emit.
    pub(super) fn number_ctor_member_name_emit(&self, expr: &Expr) -> Option<String> {
        let by_id: HashMap<LocalId, &Local> =
            self.module.locals.iter().map(|l| (l.id, l)).collect();
        number_ctor_member_name(expr, &by_id)
    }

    /// True when `emit_number_expr` can lower this (Number ty, Math, Number consts, or `.length`).
    pub(super) fn expr_emits_as_number(&self, expr: &Expr) -> bool {
        if matches!(expr.ty(), Type::Number) {
            return true;
        }
        let by_id: HashMap<LocalId, &Local> =
            self.module.locals.iter().map(|l| (l.id, l)).collect();
        expr_is_math_number(expr, &by_id)
            || expr_is_number_ctor_const(expr, &by_id)
            || expr_is_string_length(expr, &by_id)
    }

    /// Emit `Number.isNaN` / `isFinite` / `isInteger` / `isSafeInteger` as i1 SSA (N08.08.06).
    pub(super) fn emit_number_ctor_bool_call(
        &mut self,
        method: &str,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        if args.len() != 1 {
            return Err(diag(format!("internal: Number.{method} arity")));
        }
        let Arg::Expr(e) = &args[0] else {
            return Err(diag("internal: Number method expects expression arg"));
        };
        let x = self.emit_number_expr(e)?;
        match method {
            "isNaN" => {
                // Number.isNaN: true iff unordered (NaN); no ToNumber coerce.
                let t = self.fresh();
                writeln!(self.body, "  {t} = fcmp uno double {x}, {x}").ok();
                Ok(t)
            }
            "isFinite" => {
                // finite ≡ ordered and not ±Infinity.
                let ord = self.fresh();
                writeln!(self.body, "  {ord} = fcmp ord double {x}, {x}").ok();
                let abs = self.fresh();
                writeln!(
                    self.body,
                    "  {abs} = call double @llvm.fabs.f64(double {x})"
                )
                .ok();
                let is_inf = self.fresh();
                writeln!(
                    self.body,
                    "  {is_inf} = fcmp oeq double {abs}, 0x7FF0000000000000"
                )
                .ok();
                let not_inf = self.fresh();
                writeln!(self.body, "  {not_inf} = xor i1 {is_inf}, true").ok();
                let t = self.fresh();
                writeln!(self.body, "  {t} = and i1 {ord}, {not_inf}").ok();
                Ok(t)
            }
            "isInteger" => {
                // finite && floor(x) === x
                let ord = self.fresh();
                writeln!(self.body, "  {ord} = fcmp ord double {x}, {x}").ok();
                let abs = self.fresh();
                writeln!(
                    self.body,
                    "  {abs} = call double @llvm.fabs.f64(double {x})"
                )
                .ok();
                let is_inf = self.fresh();
                writeln!(
                    self.body,
                    "  {is_inf} = fcmp oeq double {abs}, 0x7FF0000000000000"
                )
                .ok();
                let not_inf = self.fresh();
                writeln!(self.body, "  {not_inf} = xor i1 {is_inf}, true").ok();
                let finite = self.fresh();
                writeln!(self.body, "  {finite} = and i1 {ord}, {not_inf}").ok();
                let flo = self.fresh();
                writeln!(
                    self.body,
                    "  {flo} = call double @llvm.floor.f64(double {x})"
                )
                .ok();
                let eq = self.fresh();
                writeln!(self.body, "  {eq} = fcmp oeq double {flo}, {x}").ok();
                let t = self.fresh();
                writeln!(self.body, "  {t} = and i1 {finite}, {eq}").ok();
                Ok(t)
            }
            "isSafeInteger" => {
                // isInteger && abs(x) <= MAX_SAFE_INTEGER
                let ord = self.fresh();
                writeln!(self.body, "  {ord} = fcmp ord double {x}, {x}").ok();
                let abs = self.fresh();
                writeln!(
                    self.body,
                    "  {abs} = call double @llvm.fabs.f64(double {x})"
                )
                .ok();
                let is_inf = self.fresh();
                writeln!(
                    self.body,
                    "  {is_inf} = fcmp oeq double {abs}, 0x7FF0000000000000"
                )
                .ok();
                let not_inf = self.fresh();
                writeln!(self.body, "  {not_inf} = xor i1 {is_inf}, true").ok();
                let finite = self.fresh();
                writeln!(self.body, "  {finite} = and i1 {ord}, {not_inf}").ok();
                let flo = self.fresh();
                writeln!(
                    self.body,
                    "  {flo} = call double @llvm.floor.f64(double {x})"
                )
                .ok();
                let eq = self.fresh();
                writeln!(self.body, "  {eq} = fcmp oeq double {flo}, {x}").ok();
                let is_int = self.fresh();
                writeln!(self.body, "  {is_int} = and i1 {finite}, {eq}").ok();
                let le = self.fresh();
                writeln!(
                    self.body,
                    "  {le} = fcmp ole double {abs}, 9.0071992547409910e+15"
                )
                .ok();
                let t = self.fresh();
                writeln!(self.body, "  {t} = and i1 {is_int}, {le}").ok();
                Ok(t)
            }
            _ => Err(diag(format!("internal: unsupported Number.{method}"))),
        }
    }

    /// Emit `Math.<method>(…args)` as f64 SSA (N08.08.05).
    pub(super) fn emit_math_call(
        &mut self,
        method: &str,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let mut nums = Vec::with_capacity(args.len());
        for a in args {
            let Arg::Expr(e) = a else {
                return Err(diag("internal: Math call expects expression args"));
            };
            nums.push(self.emit_number_expr(e)?);
        }
        match method {
            "abs" | "floor" | "ceil" | "round" | "sqrt" => {
                if nums.len() != 1 {
                    return Err(diag(format!("internal: Math.{method} arity")));
                }
                let intrinsic = match method {
                    "abs" => "llvm.fabs.f64",
                    "floor" => "llvm.floor.f64",
                    "ceil" => "llvm.ceil.f64",
                    "round" => "llvm.round.f64",
                    "sqrt" => "llvm.sqrt.f64",
                    _ => unreachable!(),
                };
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {t} = call double @{intrinsic}(double {})",
                    nums[0]
                )
                .ok();
                Ok(t)
            }
            "pow" => {
                if nums.len() != 2 {
                    return Err(diag("internal: Math.pow arity"));
                }
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {t} = call double @llvm.pow.f64(double {}, double {})",
                    nums[0], nums[1]
                )
                .ok();
                Ok(t)
            }
            "sign" => {
                if nums.len() != 1 {
                    return Err(diag("internal: Math.sign arity"));
                }
                // ES Math.sign: NaN→NaN; +0/−0 preserve; else ±1 by sign bit via compares.
                let x = &nums[0];
                let is_nan = self.fresh();
                writeln!(self.body, "  {is_nan} = fcmp uno double {x}, {x}").ok();
                let is_neg = self.fresh();
                writeln!(
                    self.body,
                    "  {is_neg} = fcmp olt double {x}, 0.00000000000000000e+00"
                )
                .ok();
                let is_pos = self.fresh();
                writeln!(
                    self.body,
                    "  {is_pos} = fcmp ogt double {x}, 0.00000000000000000e+00"
                )
                .ok();
                let neg1 = self.fresh();
                writeln!(
                    self.body,
                    "  {neg1} = select i1 {is_neg}, double -1.0000000000000000e+00, double {x}"
                )
                .ok();
                let pos1 = self.fresh();
                writeln!(
                    self.body,
                    "  {pos1} = select i1 {is_pos}, double 1.0000000000000000e+00, double {neg1}"
                )
                .ok();
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {t} = select i1 {is_nan}, double {x}, double {pos1}"
                )
                .ok();
                Ok(t)
            }
            "min" | "max" => {
                if nums.is_empty() {
                    return Err(diag(format!("internal: Math.{method} arity")));
                }
                let mut acc = nums[0].clone();
                for n in nums.iter().skip(1) {
                    let cmp = self.fresh();
                    let pred = if method == "min" { "olt" } else { "ogt" };
                    writeln!(self.body, "  {cmp} = fcmp {pred} double {acc}, {n}").ok();
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {t} = select i1 {cmp}, double {acc}, double {n}"
                    )
                    .ok();
                    acc = t;
                }
                Ok(acc)
            }
            _ => Err(diag(format!("internal: unsupported Math.{method}"))),
        }
    }

    /// Emit JS bitwise op on two number SSA values (doubles). Shift count masked to 5 bits.
    pub(super) fn emit_bitwise_number(
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
}
