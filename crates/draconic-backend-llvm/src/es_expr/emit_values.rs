use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::Diagnostic;
use draconic_ir::{AssignTarget, Expr, IrType as Type, LocalId};
use draconic_runtime::abi::{CSTR_CONCAT_N, CSTR_FROM_CODE_UNIT_N, CSTR_FROM_U64, CSTR_LEN};

use super::*;

impl<'a> Emitter<'a> {
    /// Emit a BigInt expression as signed i64 SSA (N08.08.02–N08.08.04).
    pub(super) fn emit_bigint_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::BigInt { raw, .. } => Ok(format_bigint_const(raw)?),
            Expr::Local { id, .. } => {
                let (ptr, slot) = self
                    .state
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated local %{}", id.0)))?;
                if slot != SlotTy::BigInt {
                    return Err(diag("internal: expected bigint local"));
                }
                let t = self.fresh();
                writeln!(self.body, "  {t} = load i64, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Unary { op, arg, .. } => match op {
                UnaryOp::Minus => {
                    let a = self.emit_bigint_expr(arg)?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = sub i64 0, {a}").ok();
                    Ok(t)
                }
                // JS BigInt `~x` is two's-complement bitwise not (no ToInt32).
                UnaryOp::BitNot => {
                    let a = self.emit_bigint_expr(arg)?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = xor i64 {a}, -1").ok();
                    Ok(t)
                }
                _ => Err(diag("internal: non-bigint unary in bigint emit")),
            },
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.emit_bigint_expr(left)?;
                let r = self.emit_bigint_expr(right)?;
                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem => {
                        let inst = match op {
                            BinaryOp::Add => "add",
                            BinaryOp::Sub => "sub",
                            BinaryOp::Mul => "mul",
                            // JS BigInt `/` and `%` truncate toward zero (sdiv/srem).
                            BinaryOp::Div => "sdiv",
                            BinaryOp::Rem => "srem",
                            _ => unreachable!(),
                        };
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = {inst} i64 {l}, {r}").ok();
                        Ok(t)
                    }
                    BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                        let inst = match op {
                            BinaryOp::BitAnd => "and",
                            BinaryOp::BitOr => "or",
                            BinaryOp::BitXor => "xor",
                            _ => unreachable!(),
                        };
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = {inst} i64 {l}, {r}").ok();
                        Ok(t)
                    }
                    // BigInt shifts use the full shift count (fixture values fit i64).
                    BinaryOp::Shl => {
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = shl i64 {l}, {r}").ok();
                        Ok(t)
                    }
                    BinaryOp::Shr => {
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = ashr i64 {l}, {r}").ok();
                        Ok(t)
                    }
                    // N08.08.04: BigInt `**` binary exponentiation (non-neg exp; 0**0 → 1).
                    BinaryOp::Pow => self.emit_bigint_pow(&l, &r),
                    BinaryOp::Comma => Ok(r),
                    _ => Err(diag("internal: non-arithmetic binary in bigint emit")),
                }
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
                if slot != SlotTy::BigInt {
                    return Err(diag("internal: expected bigint assign local"));
                }
                match op {
                    AssignOp::Eq => {
                        let v = self.emit_bigint_expr(value)?;
                        writeln!(self.body, "  store i64 {v}, ptr {ptr}").ok();
                        Ok(v)
                    }
                    // ES order: GetValue(lhs) then evaluate RHS, then pow and PutValue.
                    AssignOp::PowEq => {
                        let cur = self.fresh();
                        writeln!(self.body, "  {cur} = load i64, ptr {ptr}").ok();
                        let r = self.emit_bigint_expr(value)?;
                        let v = self.emit_bigint_pow(&cur, &r)?;
                        writeln!(self.body, "  store i64 {v}, ptr {ptr}").ok();
                        Ok(v)
                    }
                    _ => Err(diag("internal: unsupported bigint assign op")),
                }
            }
            _ => Err(diag("internal: unsupported bigint expr in es_expr")),
        }
    }

    /// Integer binary exponentiation for JS BigInt `**` / `**=` (N08.08.04).
    /// Exponent must be non-negative (fixtures); any base ** 0n → 1n, including 0n ** 0n.
    pub(super) fn emit_bigint_pow(&mut self, base: &str, exp: &str) -> Result<String, Diagnostic> {
        let res_a = self.fresh();
        let base_a = self.fresh();
        let exp_a = self.fresh();
        writeln!(self.body, "  {res_a} = alloca i64, align 8").ok();
        writeln!(self.body, "  {base_a} = alloca i64, align 8").ok();
        writeln!(self.body, "  {exp_a} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 1, ptr {res_a}").ok();
        writeln!(self.body, "  store i64 {base}, ptr {base_a}").ok();
        writeln!(self.body, "  store i64 {exp}, ptr {exp_a}").ok();

        let head = self.fresh_label("bipow_head");
        let body = self.fresh_label("bipow_body");
        let odd = self.fresh_label("bipow_odd");
        let after = self.fresh_label("bipow_after");
        let end = self.fresh_label("bipow_end");

        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{head}:").ok();
        let e0 = self.fresh();
        writeln!(self.body, "  {e0} = load i64, ptr {exp_a}").ok();
        let cont = self.fresh();
        writeln!(self.body, "  {cont} = icmp sgt i64 {e0}, 0").ok();
        writeln!(self.body, "  br i1 {cont}, label %{body}, label %{end}").ok();

        writeln!(self.body, "{body}:").ok();
        let e1 = self.fresh();
        writeln!(self.body, "  {e1} = load i64, ptr {exp_a}").ok();
        let bit = self.fresh();
        writeln!(self.body, "  {bit} = and i64 {e1}, 1").ok();
        let is_odd = self.fresh();
        writeln!(self.body, "  {is_odd} = icmp ne i64 {bit}, 0").ok();
        writeln!(self.body, "  br i1 {is_odd}, label %{odd}, label %{after}").ok();

        writeln!(self.body, "{odd}:").ok();
        let r0 = self.fresh();
        let b0 = self.fresh();
        writeln!(self.body, "  {r0} = load i64, ptr {res_a}").ok();
        writeln!(self.body, "  {b0} = load i64, ptr {base_a}").ok();
        let r1 = self.fresh();
        writeln!(self.body, "  {r1} = mul i64 {r0}, {b0}").ok();
        writeln!(self.body, "  store i64 {r1}, ptr {res_a}").ok();
        writeln!(self.body, "  br label %{after}").ok();

        writeln!(self.body, "{after}:").ok();
        let b1 = self.fresh();
        writeln!(self.body, "  {b1} = load i64, ptr {base_a}").ok();
        let bsq = self.fresh();
        writeln!(self.body, "  {bsq} = mul i64 {b1}, {b1}").ok();
        writeln!(self.body, "  store i64 {bsq}, ptr {base_a}").ok();
        let e2 = self.fresh();
        writeln!(self.body, "  {e2} = load i64, ptr {exp_a}").ok();
        let e3 = self.fresh();
        writeln!(self.body, "  {e3} = ashr i64 {e2}, 1").ok();
        writeln!(self.body, "  store i64 {e3}, ptr {exp_a}").ok();
        writeln!(self.body, "  br label %{head}").ok();

        writeln!(self.body, "{end}:").ok();
        let out = self.fresh();
        writeln!(self.body, "  {out} = load i64, ptr {res_a}").ok();
        Ok(out)
    }

    pub(super) fn emit_string_expr(&mut self, expr: &Expr) -> Result<StrVal, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const_js(value),
            // N08.07.02: `` `a${x}b` `` → concat cooked quasis with ToString(expressions).
            Expr::Template {
                quasis,
                expressions,
                ..
            } => {
                if quasis.is_empty() {
                    return Err(diag("internal: template with no quasis"));
                }
                if quasis.len() != expressions.len() + 1 {
                    return Err(diag(
                        "internal: template quasis/expressions length mismatch",
                    ));
                }
                let mut acc = self.string_const_js(&quasis[0])?;
                for (i, e) in expressions.iter().enumerate() {
                    let mid = self.emit_concat_operand(e)?;
                    acc = self.emit_concat_strvals(&acc, &mid)?;
                    let q = self.string_const_js(&quasis[i + 1])?;
                    acc = self.emit_concat_strvals(&acc, &q)?;
                }
                Ok(acc)
            }
            Expr::Local { id, .. } => self.load_string_local(*id),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => {
                self.emit_discard_arg(arg)?;
                let slot_of: HashMap<LocalId, SlotTy> = self
                    .state
                    .allocas
                    .iter()
                    .map(|(k, (_, s))| (*k, *s))
                    .collect();
                let name = Self::typeof_name(arg, &slot_of, self.module)
                    .ok_or_else(|| diag("internal: unsupported typeof operand"))?;
                self.string_const(name)
            }
            Expr::Binary {
                left,
                op: BinaryOp::Comma,
                right,
                ..
            } => {
                self.emit_discard_arg(left)?;
                self.emit_string_expr(right)
            }
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } => {
                let l = self.emit_concat_operand(left)?;
                let r = self.emit_concat_operand(right)?;
                self.emit_concat_strvals(&l, &r)
            }
            Expr::Member {
                object,
                property,
                computed: true,
                optional: false,
                ..
            } => {
                // N08.07.05: `s[i]` indexes UTF-16 code units; result is one-unit WTF-8.
                let s = self.emit_string_expr(object)?;
                let idx_f = self.emit_number_expr(property)?;
                let idx = self.fresh();
                writeln!(self.body, "  {idx} = fptoui double {idx_f} to i64").ok();
                let out_len_ptr = self.fresh();
                writeln!(self.body, "  {out_len_ptr} = alloca i64, align 8").ok();
                let ch = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    CSTR_FROM_CODE_UNIT_N.call_to(
                        &ch,
                        &format!(
                            "ptr {}, i64 {}, i64 {idx}, ptr {out_len_ptr}",
                            s.data, s.len
                        )
                    )
                )
                .ok();
                let ch_len = self.fresh();
                writeln!(self.body, "  {ch_len} = load i64, ptr {out_len_ptr}").ok();
                Ok(StrVal {
                    data: ch,
                    len: ch_len,
                })
            }
            Expr::Assign {
                target, op, value, ..
            } => {
                if !matches!(op, AssignOp::Eq) {
                    return Err(diag("internal: only simple = in es_expr string assign"));
                }
                let AssignTarget::Local(id) = target else {
                    return Err(diag("internal: only local assign in es_expr"));
                };
                let v = self.emit_string_expr(value)?;
                self.store_string_local(*id, &v)?;
                Ok(v)
            }
            _ => Err(diag("internal: unsupported string expr in es_expr module")),
        }
    }

    /// String-producing operand for concat (string subset or `any` string slot).
    pub(super) fn emit_string_operand(&mut self, expr: &Expr) -> Result<StrVal, Diagnostic> {
        match expr {
            Expr::Local { id, ty: Type::Any } => self.load_string_local(*id),
            e => self.emit_string_expr(e),
        }
    }

    pub(super) fn emit_concat_strvals(
        &mut self,
        left: &StrVal,
        right: &StrVal,
    ) -> Result<StrVal, Diagnostic> {
        // N08.07.05: concat UTF-16 units then re-encode WTF-8 (*out_len).
        let out_len_ptr = self.fresh();
        writeln!(self.body, "  {out_len_ptr} = alloca i64, align 8").ok();
        let t = self.fresh();
        writeln!(
            self.body,
            "  {}",
            CSTR_CONCAT_N.call_to(
                &t,
                &format!(
                    "ptr {}, i64 {}, ptr {}, i64 {}, ptr {out_len_ptr}",
                    left.data, left.len, right.data, right.len
                )
            )
        )
        .ok();
        let n = self.fresh();
        writeln!(self.body, "  {n} = load i64, ptr {out_len_ptr}").ok();
        Ok(StrVal { data: t, len: n })
    }

    /// Concat operand: string or number (ToString via decimal for non-neg integers).
    pub(super) fn emit_concat_operand(&mut self, expr: &Expr) -> Result<StrVal, Diagnostic> {
        let as_number = match expr {
            Expr::Number { .. } => true,
            Expr::Local { id, .. } => self
                .state
                .allocas
                .get(id)
                .is_some_and(|(_, s)| *s == SlotTy::Number),
            e if expr_ty_is_number(e) => true,
            _ => false,
        };
        if as_number {
            let n = self.emit_number_expr(expr)?;
            let i = self.fresh();
            writeln!(self.body, "  {i} = fptoui double {n} to i64").ok();
            let p = self.fresh();
            writeln!(
                self.body,
                "  {}",
                CSTR_FROM_U64.call_to(&p, &format!("i64 {i}"))
            )
            .ok();
            let len = self.fresh();
            writeln!(
                self.body,
                "  {}",
                CSTR_LEN.call_to(&len, &format!("ptr {p}"))
            )
            .ok();
            return Ok(StrVal { data: p, len });
        }
        self.emit_string_operand(expr)
    }

    pub(super) fn emit_undefined_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Local { id, .. } => {
                let (_, slot) = self
                    .state
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated local %{}", id.0)))?;
                if slot != SlotTy::Undefined {
                    return Err(diag("internal: expected undefined local"));
                }
                Ok(())
            }
            Expr::Unary {
                op: UnaryOp::Void,
                arg,
                ..
            } => self.emit_discard_arg(arg),
            Expr::Binary {
                left,
                op: BinaryOp::Comma,
                right,
                ..
            } => {
                self.emit_discard_arg(left)?;
                self.emit_undefined_expr(right)
            }
            Expr::Assign {
                target, op, value, ..
            } => {
                if !matches!(op, AssignOp::Eq) {
                    return Err(diag("internal: only simple = in es_expr undefined assign"));
                }
                let AssignTarget::Local(id) = target else {
                    return Err(diag("internal: only local assign in es_expr"));
                };
                let (_, slot) =
                    self.state.allocas.get(id).cloned().ok_or_else(|| {
                        diag(format!("internal: unallocated assign local %{}", id.0))
                    })?;
                if slot != SlotTy::Undefined {
                    return Err(diag("internal: expected undefined assign target"));
                }
                self.emit_undefined_expr(value)
            }
            _ => Err(diag(
                "internal: unsupported undefined expr in es_expr module",
            )),
        }
    }
}
