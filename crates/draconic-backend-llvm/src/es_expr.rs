//! N08.01 + N08.02.01: emit native observations for ES expression Programs and `if`/`else`
//! (E01.01 arithmetic, E01.02 comparison, E01.03 logical, E01.04.01 bitwise, E01.04.02 `**`,
//! E01.04.03 conditional `?:`, E01.04.04 simple `=` assignment, E01.04.05 prefix/postfix `++`/`--`,
//! E01.04.06 comma `,`, E01.04.07 unary keywords `typeof`/`void`/`delete`,
//! E01.04.08 compound assignment `+=` `-=` `*=` `/=` `%=` `**=` `<<=` `>>=` `>>>=` `&=` `^=` `|=`,
//! E02.01 `if` / `else` (incl. block bodies; ToBoolean on number/boolean tests).
//! N08.01.04.09 nullish/logical-assign lives in `es_nullish`.)

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp, UpdateOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Stmt, UpdateTarget};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BOOL, PRINT_F64, PRINT_STR};

/// True when this module is a supported ES expression / `if` subset (E01.* / E02.01 / N08.01.* / N08.02.01):
/// top-level `let` declares over JS numbers, booleans, strings, and/or undefined (`void`) with
/// arithmetic, unary `+`/`-`/`!`/`~`/`typeof`/`void`/`delete`, comparison, equality, logical,
/// bitwise, exponentiation, conditional, simple/compound assignment, prefix/postfix `++`/`--`,
/// comma, grouping, local refs, and `if`/`else` (block or expression bodies). Expression
/// statements may be assigns or updates.
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
    String,
    /// JS `undefined` from `void` (checker maps void → `Type::Null`).
    Undefined,
}

/// Top-level user locals in declaration order (observation order).
struct ModuleInfo {
    user_locals: Vec<(LocalId, SlotTy)>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_locals = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                let slot = match loc.ty {
                    Type::Number => {
                        if let Some(init) = init {
                            if !expr_is_number_subset(init, &by_id) {
                                return None;
                            }
                        }
                        SlotTy::Number
                    }
                    Type::Boolean => {
                        if let Some(init) = init {
                            if !expr_is_boolean_subset(init, &by_id) {
                                return None;
                            }
                        }
                        SlotTy::Boolean
                    }
                    Type::String => {
                        if let Some(init) = init {
                            if !expr_is_string_subset(init, &by_id) {
                                return None;
                            }
                        }
                        SlotTy::String
                    }
                    Type::Null => {
                        if let Some(init) = init {
                            if !expr_is_undefined_subset(init, &by_id) {
                                return None;
                            }
                        }
                        SlotTy::Undefined
                    }
                    _ => return None,
                };
                if seen.insert(*local) {
                    user_locals.push((*local, slot));
                }
            }
            Stmt::Expr { .. } | Stmt::Block { .. } | Stmt::If { .. } => {
                if !stmt_is_subset(stmt, &by_id) {
                    return None;
                }
            }
            _ => return None,
        }
    }
    if user_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo { user_locals })
}

/// Nested statement subset for `if`/`else` bodies and blocks (no nested `let` in this slice).
fn stmt_is_subset(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Expr { expr } => match expr.ty() {
            Type::Number => expr_is_number_subset(expr, by_id),
            Type::Boolean => expr_is_boolean_subset(expr, by_id),
            Type::String => expr_is_string_subset(expr, by_id),
            Type::Null => expr_is_undefined_subset(expr, by_id),
            _ => false,
        },
        Stmt::Block { body } => body.iter().all(|s| stmt_is_subset(s, by_id)),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            (expr_is_boolean_subset(test, by_id) || expr_is_number_subset(test, by_id))
                && stmt_is_subset(consequent, by_id)
                && alternate
                    .as_ref()
                    .map(|a| stmt_is_subset(a, by_id))
                    .unwrap_or(true)
        }
        _ => false,
    }
}

/// Operand of `typeof` / `void` / `delete` in the supported subset.
fn expr_is_unary_keyword_arg(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { ty, .. } => *ty == Type::Number,
        Expr::String { ty, .. } => *ty == Type::String,
        Expr::Boolean { ty, .. } => *ty == Type::Boolean,
        Expr::Null { ty } => *ty == Type::Null,
        Expr::Local { id, ty } => {
            matches!(ty, Type::Number | Type::String | Type::Boolean | Type::Null)
                && by_id.get(id).is_some_and(|l| l.ty == *ty)
        }
        e if expr_is_number_subset(e, by_id) => true,
        e if expr_is_boolean_subset(e, by_id) => true,
        e if expr_is_string_subset(e, by_id) => true,
        e if expr_is_undefined_subset(e, by_id) => true,
        _ => false,
    }
}

fn expr_is_string_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::String { ty, .. } => *ty == Type::String,
        Expr::Local { id, ty } => {
            *ty == Type::String && by_id.get(id).is_some_and(|l| l.ty == Type::String)
        }
        Expr::Unary { op, arg, ty } => {
            *ty == Type::String
                && matches!(op, UnaryOp::TypeOf)
                && expr_is_unary_keyword_arg(arg, by_id)
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            *ty == Type::String
                && matches!(op, BinaryOp::Comma)
                && expr_is_unary_keyword_arg(left, by_id)
                && expr_is_string_subset(right, by_id)
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::String
                && matches!(op, AssignOp::Eq)
                && matches!(target, AssignTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::String))
                && expr_is_string_subset(value, by_id)
        }
        _ => false,
    }
}

fn expr_is_undefined_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, ty } => {
            *ty == Type::Null && by_id.get(id).is_some_and(|l| l.ty == Type::Null)
        }
        Expr::Unary { op, arg, ty } => {
            *ty == Type::Null
                && matches!(op, UnaryOp::Void)
                && expr_is_unary_keyword_arg(arg, by_id)
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            *ty == Type::Null
                && matches!(op, BinaryOp::Comma)
                && expr_is_unary_keyword_arg(left, by_id)
                && expr_is_undefined_subset(right, by_id)
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::Null
                && matches!(op, AssignOp::Eq)
                && matches!(target, AssignTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::Null))
                && expr_is_undefined_subset(value, by_id)
        }
        _ => false,
    }
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
                        | BinaryOp::Comma
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
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::Number
                && is_number_assign_op(*op)
                && matches!(target, AssignTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::Number))
                && expr_is_number_subset(value, by_id)
        }
        Expr::Update {
            target,
            ty,
            ..
        } => {
            *ty == Type::Number
                && matches!(
                    target,
                    UpdateTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::Number)
                )
        }
        _ => false,
    }
}

/// Simple `=` plus numeric compound ops (not logical `&&=`/`||=`/`??=` — N08.01.04.09).
fn is_number_assign_op(op: AssignOp) -> bool {
    matches!(
        op,
        AssignOp::Eq
            | AssignOp::AddEq
            | AssignOp::SubEq
            | AssignOp::MulEq
            | AssignOp::DivEq
            | AssignOp::RemEq
            | AssignOp::PowEq
            | AssignOp::ShlEq
            | AssignOp::ShrEq
            | AssignOp::UShrEq
            | AssignOp::BitAndEq
            | AssignOp::BitOrEq
            | AssignOp::BitXorEq
    )
}

fn expr_is_boolean_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Boolean { ty, .. } => *ty == Type::Boolean,
        Expr::Local { id, ty } => {
            *ty == Type::Boolean && by_id.get(id).is_some_and(|l| l.ty == Type::Boolean)
        }
        Expr::Unary { op, arg, ty } => {
            if *ty != Type::Boolean {
                return false;
            }
            match op {
                UnaryOp::Not => expr_is_boolean_subset(arg, by_id),
                // `delete` of a non-reference (literal/expr) is always `true` in non-strict.
                UnaryOp::Delete => expr_is_unary_keyword_arg(arg, by_id),
                _ => false,
            }
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
                BinaryOp::Comma => {
                    expr_is_unary_keyword_arg(left, by_id) && expr_is_boolean_subset(right, by_id)
                }
                _ => false,
            }
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::Boolean
                && matches!(op, AssignOp::Eq)
                && matches!(target, AssignTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::Boolean))
                && expr_is_boolean_subset(value, by_id)
        }
        _ => false,
    }
}

struct Emitter<'a> {
    module: &'a Module,
    /// local id → (alloca ptr name, slot type)
    allocas: HashMap<LocalId, (String, SlotTy)>,
    /// string content → global name (e.g. `.str.0`)
    str_globals: HashMap<String, String>,
    out: String,
    body: String,
    tmp: u32,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        Self {
            module,
            allocas: HashMap::new(),
            str_globals: HashMap::new(),
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

    fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("{prefix}{n}")
    }

    fn body_ends_with_terminator(&self) -> bool {
        self.body
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .is_some_and(|l| {
                let t = l.trim_start();
                t.starts_with("br ")
                    || t.starts_with("ret ")
                    || t.starts_with("unreachable")
                    || t.starts_with("switch ")
                    || t.starts_with("indirectbr ")
            })
    }

    fn emit_module(&mut self, user: &[(LocalId, SlotTy)]) -> Result<(), Diagnostic> {
        // Body first so string globals are collected, then header + globals + main.
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
                SlotTy::String => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                }
                // No runtime payload; print always emits `undefined`.
                SlotTy::Undefined => {}
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        // Print top-level user locals in declaration order.
        for (id, slot) in user {
            match slot {
                SlotTy::Number => {
                    let (ptr, _) = self
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("internal: print missing alloca"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::Boolean => {
                    let (ptr, _) = self
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("internal: print missing alloca"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i1, ptr {ptr}").ok();
                    let ext = self.fresh();
                    writeln!(self.body, "  {ext} = zext i1 {v} to i8").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {ext}"))).ok();
                }
                SlotTy::String => {
                    let (ptr, _) = self
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("internal: print missing alloca"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotTy::Undefined => {
                    let p = self.string_const("undefined")?;
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {p}"))).ok();
                }
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.01/N08.02.01 ES expressions + if/else via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        // JS `**` → IEEE pow (Math.pow); intrinsic available without libm link flags.
        writeln!(self.out, "declare double @llvm.pow.f64(double, double)").ok();
        writeln!(self.out).ok();

        for (content, gname) in &self.str_globals {
            let n = content.len() + 1;
            let esc = escape_llvm_string(content);
            writeln!(
                self.out,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        if !self.str_globals.is_empty() {
            writeln!(self.out).ok();
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let (ptr, slot) = self
                    .allocas
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("internal: missing alloca"))?;
                match (slot, init) {
                    (SlotTy::Number, Some(init)) => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    (SlotTy::Boolean, Some(init)) => {
                        let v = self.emit_bool_expr(init)?;
                        writeln!(self.body, "  store i1 {v}, ptr {ptr}").ok();
                    }
                    (SlotTy::String, Some(init)) => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    (SlotTy::Undefined, Some(init)) => {
                        self.emit_undefined_expr(init)?;
                    }
                    // Uninitialized `let` — leave alloca undef until assigned.
                    (_, None) => {}
                }
                Ok(())
            }
            Stmt::Expr { expr } => match expr.ty() {
                Type::Number => {
                    let _ = self.emit_number_expr(expr)?;
                    Ok(())
                }
                Type::Boolean => {
                    let _ = self.emit_bool_expr(expr)?;
                    Ok(())
                }
                Type::String => {
                    let _ = self.emit_string_expr(expr)?;
                    Ok(())
                }
                Type::Null => self.emit_undefined_expr(expr),
                _ => Err(diag("internal: unsupported expr stmt ty in es_expr module")),
            },
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let cond = self.emit_to_boolean(test)?;
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
            _ => Err(diag("internal: unsupported stmt in es_expr module")),
        }
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some(g) = self.str_globals.get(s) {
            g.clone()
        } else {
            let g = format!(".str.{}", self.str_globals.len());
            self.str_globals.insert(s.to_string(), g.clone());
            g
        };
        let t = self.fresh();
        let n = s.len() + 1;
        writeln!(
            self.body,
            "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
        )
        .ok();
        Ok(t)
    }

    /// Evaluate a typeof/void/delete operand for side effects only.
    fn emit_discard_arg(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            // `null` literal and pure undefined locals have no side effects.
            Expr::Null { .. } => Ok(()),
            Expr::Number { .. } => {
                let _ = self.emit_number_expr(expr)?;
                Ok(())
            }
            Expr::Boolean { .. } => {
                let _ = self.emit_bool_expr(expr)?;
                Ok(())
            }
            Expr::String { .. } => {
                let _ = self.emit_string_expr(expr)?;
                Ok(())
            }
            Expr::Local { id, .. } => {
                let (_, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated discard local %{}", id.0)))?;
                match slot {
                    SlotTy::Number => {
                        let _ = self.emit_number_expr(expr)?;
                    }
                    SlotTy::Boolean => {
                        let _ = self.emit_bool_expr(expr)?;
                    }
                    SlotTy::String => {
                        let _ = self.emit_string_expr(expr)?;
                    }
                    SlotTy::Undefined => {}
                }
                Ok(())
            }
            e if matches!(e.ty(), Type::Number) => {
                let _ = self.emit_number_expr(e)?;
                Ok(())
            }
            e if matches!(e.ty(), Type::Boolean) => {
                let _ = self.emit_bool_expr(e)?;
                Ok(())
            }
            e if matches!(e.ty(), Type::String) => {
                let _ = self.emit_string_expr(e)?;
                Ok(())
            }
            e if matches!(e.ty(), Type::Null) => self.emit_undefined_expr(e),
            _ => Err(diag("internal: unsupported typeof/void/delete arg")),
        }
    }

    fn typeof_name(expr: &Expr, slot_of: &HashMap<LocalId, SlotTy>) -> Option<&'static str> {
        match expr {
            Expr::Number { .. } => Some("number"),
            Expr::String { .. } => Some("string"),
            Expr::Boolean { .. } => Some("boolean"),
            Expr::Null { .. } => Some("object"),
            Expr::Local { id, .. } => match slot_of.get(id)? {
                SlotTy::Number => Some("number"),
                SlotTy::String => Some("string"),
                SlotTy::Boolean => Some("boolean"),
                SlotTy::Undefined => Some("undefined"),
            },
            Expr::Unary {
                op: UnaryOp::Void, ..
            } => Some("undefined"),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                ..
            } => Some("string"),
            // Number/bool-producing ops → "number" / "boolean"
            e if matches!(e.ty(), Type::Number) => Some("number"),
            e if matches!(e.ty(), Type::Boolean) => Some("boolean"),
            e if matches!(e.ty(), Type::String) => Some("string"),
            e if matches!(e.ty(), Type::Null) => Some("undefined"),
            _ => None,
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                self.string_const(&s)
            }
            Expr::Local { id, .. } => {
                let (ptr, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated local %{}", id.0)))?;
                if slot != SlotTy::String {
                    return Err(diag("internal: expected string local"));
                }
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => {
                self.emit_discard_arg(arg)?;
                let slot_of: HashMap<LocalId, SlotTy> =
                    self.allocas.iter().map(|(k, (_, s))| (*k, *s)).collect();
                let name = Self::typeof_name(arg, &slot_of)
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
            Expr::Assign {
                target,
                op,
                value,
                ..
            } => {
                if !matches!(op, AssignOp::Eq) {
                    return Err(diag("internal: only simple = in es_expr string assign"));
                }
                let AssignTarget::Local(id) = target else {
                    return Err(diag("internal: only local assign in es_expr"));
                };
                let (ptr, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated assign local %{}", id.0)))?;
                if slot != SlotTy::String {
                    return Err(diag("internal: expected string assign target"));
                }
                let v = self.emit_string_expr(value)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("internal: unsupported string expr in es_expr module")),
        }
    }

    fn emit_undefined_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Local { id, .. } => {
                let (_, slot) = self
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
                target,
                op,
                value,
                ..
            } => {
                if !matches!(op, AssignOp::Eq) {
                    return Err(diag("internal: only simple = in es_expr undefined assign"));
                }
                let AssignTarget::Local(id) = target else {
                    return Err(diag("internal: only local assign in es_expr"));
                };
                let (_, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated assign local %{}", id.0)))?;
                if slot != SlotTy::Undefined {
                    return Err(diag("internal: expected undefined assign target"));
                }
                self.emit_undefined_expr(value)
            }
            _ => Err(diag("internal: unsupported undefined expr in es_expr module")),
        }
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
                target,
                op,
                value,
                ..
            } => {
                let AssignTarget::Local(id) = target else {
                    return Err(diag("internal: only local assign in es_expr"));
                };
                let (ptr, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated assign local %{}", id.0)))?;
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
                    AssignOp::AddEq | AssignOp::SubEq | AssignOp::MulEq | AssignOp::DivEq
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
                op,
                target,
                prefix,
                ..
            } => {
                let UpdateTarget::Local(id) = target else {
                    return Err(diag("internal: only local ++/-- in es_expr"));
                };
                let (ptr, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated update local %{}", id.0)))?;
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
                BinaryOp::Comma => {
                    self.emit_discard_arg(left)?;
                    self.emit_bool_expr(right)
                }
                _ => Err(diag("internal: non-comparison binary in bool emit")),
            },
            Expr::Assign {
                target,
                op,
                value,
                ..
            } => {
                if !matches!(op, AssignOp::Eq) {
                    return Err(diag("internal: only simple = in es_expr bool assign"));
                }
                let AssignTarget::Local(id) = target else {
                    return Err(diag("internal: only local assign in es_expr"));
                };
                let (ptr, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated assign local %{}", id.0)))?;
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

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) && c != b'\\' => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
