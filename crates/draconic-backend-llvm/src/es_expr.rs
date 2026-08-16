//! N08.01 + N08.02.01–N08.02.09: emit native observations for ES expression Programs,
//! `if`/`else`, `while`, `do`/`while`, `for`, `for-in`/`for-of` (strings), `break`/`continue`
//! (incl. labeled), `switch`, labeled statements, and `const` declarations
//! (E01.01 arithmetic, E01.02 comparison, E01.03 logical, E01.04.01 bitwise, E01.04.02 `**`,
//! E01.04.03 conditional `?:`, E01.04.04 simple `=` assignment, E01.04.05 prefix/postfix `++`/`--`,
//! E01.04.06 comma `,`, E01.04.07 unary keywords `typeof`/`void`/`delete`,
//! E01.04.08 compound assignment `+=` `-=` `*=` `/=` `%=` `**=` `<<=` `>>=` `>>>=` `&=` `^=` `|=`,
//! E02.01 `if` / `else` (incl. block bodies; ToBoolean on number/boolean tests),
//! E02.02 `while` loops (incl. block bodies; ToBoolean on number/boolean tests),
//! E02.03 `do` / `while` loops (incl. block bodies; ToBoolean on number/boolean tests),
//! E02.04 `for` loops (`for (init; test; update)`; `let`/`const` init; omitted clauses; block bodies),
//! E02.05 unlabeled `break` / `continue` in loops,
//! E02.06 `switch` / `case` / `default` (number discriminant; fall-through; unlabeled `break`),
//! E02.07 labeled statements + labeled `break` / `continue`,
//! E02.08 `for-in` / `for-of` over strings (`let`/`const`/assign binding; string concat `+`),
//! E02.09 `const` declarations (required init; `for`/`for-of`/`for-in` binding),
//! E07.01 string lit + concat (incl. number ToString) + `.length` + index (N08.07.01),
//! E07.02 untagged template literals (N08.07.02; cooked quasis + ToString interpolations),
//! E07.03 unicode escapes `\x`/`\u`/`\u{}` cooked into strings; `.length` is UTF-16 units (N08.07.03),
//! E07.05 UTF-16 code-unit semantics: index/concat/eq over WTF-8 storage (N08.07.05).
//! E08.01 number literals: decimal/hex/bin/oct/separators/scientific (N08.08.01).
//! E08.02 BigInt integer literals + same-type arithmetic (N08.08.02; i64-range values).
//! E08.03 BigInt comparison & bitwise: `<` `<=` `>` `>=` `==` `!=` `===` `!==` `&` `|` `^` `~` `<<` `>>` (N08.08.03; no `>>>`).
//! E08.04 BigInt exponentiation: `**` (right-associative) and `**=` (same-type BigInt; non-neg exp; N08.08.04).
//! E08.05 Global `Math`: constants (`E`, `PI`, `LN2`, `LOG2E`) + methods (`abs`, `floor`, `ceil`,
//! `round`, `min`, `max`, `pow`, `sqrt`, `sign`) via `.` / `[]` and calls (N08.08.05).
//! E08.06 Global `NaN` / `Infinity` + `Number`: constants + static methods (`isNaN`, `isFinite`,
//! `isInteger`, `isSafeInteger`) via `.` / `[]` and calls (N08.08.06).
//! N08.01.04.09 nullish/logical-assign lives in `es_nullish`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp, UpdateOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Stmt, UpdateTarget,
};
use draconic_runtime::abi::{
    llvm_declares, CSTR_CONCAT_N, CSTR_EQ_N, CSTR_FROM_CODE_UNIT_N, CSTR_FROM_U64, CSTR_LEN,
    ES_EXPR_DECLARES, PRINT_BOOL, PRINT_BYTES, PRINT_F64, PRINT_I64, UTF16_LEN,
};

/// True when this module is a supported ES expression / control-flow subset
/// (E01.* / E02.01–E02.09 / E07.01–E07.05 / E08.01–E08.06 / N08.01.* / N08.02.01–N08.02.09 /
/// N08.07.01–N08.07.05 / N08.08.01–N08.08.06):
/// top-level `let`/`const` declares over JS numbers, BigInts (i64-range), booleans, strings,
/// undefined (`void`), and/or untyped `any` string/number/boolean slots with arithmetic, unary
/// `+`/`-`/`!`/`~`/`typeof`/`void`/`delete`, comparison, equality, logical, bitwise,
/// exponentiation, conditional, simple/compound assignment, prefix/postfix `++`/`--`, comma,
/// grouping, local refs, string concat `+` (incl. number ToString), untagged templates,
/// unicode-escape string lits, UTF-16 index/length, number literals
/// (decimal/hex/bin/oct/separators/scientific), BigInt literals + same-type `+` `-` `*` `/` `%`
/// unary `-`/`~`, comparison/equality, bitwise `&` `|` `^` `<<` `>>` (no `>>>`),
/// BigInt `**` / `**=` (non-negative exponents; values fit i64),
/// global `Math` constants/methods (`. ` / `[]` + call; `typeof Math` → `"object"`),
/// global `NaN`/`Infinity`/`Number` constants/static methods (`. ` / `[]` + call;
/// `typeof Number` → `"function"`; `typeof NaN`/`Infinity` → `"number"`),
/// `if`/`else`, `while`, `do`/`while`, `for` (incl. `let`/`const` init; block or
/// expression bodies), `for-in`/`for-of` over strings (`let`/`const`/assign left), `break`/`continue`
/// (unlabeled or labeled), labeled statements (incl. labeled blocks), and `switch`/`case`/`default`
/// (number discriminant; fall-through; unlabeled `break`).
/// Expression statements may be assigns or updates.
pub(crate) fn is_es_expr_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_expr(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_expr module"))?;
    let mut em = Emitter::new(module);
    em.emit_module(&info.alloc_locals, &info.user_locals)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    /// JS BigInt as signed i64 (N08.08.02 fixture range).
    BigInt,
    Boolean,
    String,
    /// JS `undefined` from `void` (checker maps void → `Type::Null`).
    Undefined,
}

/// Top-level user locals in declaration order (observation/print order).
/// `alloc_locals` also includes `for (let …)` bindings (not printed).
struct ModuleInfo {
    user_locals: Vec<(LocalId, SlotTy)>,
    alloc_locals: Vec<(LocalId, SlotTy)>,
}

fn slot_for_declare(
    local: LocalId,
    init: &Option<Expr>,
    by_id: &HashMap<LocalId, &Local>,
) -> Option<SlotTy> {
    let loc = by_id.get(&local)?;
    match loc.ty {
        Type::Number => {
            if let Some(init) = init {
                if !expr_is_number_subset(init, by_id) {
                    return None;
                }
            }
            Some(SlotTy::Number)
        }
        Type::BigInt => {
            if let Some(init) = init {
                if !expr_is_bigint_subset(init, by_id) {
                    return None;
                }
            }
            Some(SlotTy::BigInt)
        }
        Type::Boolean => {
            if let Some(init) = init {
                if !expr_is_boolean_subset(init, by_id) {
                    return None;
                }
            }
            Some(SlotTy::Boolean)
        }
        Type::String => {
            if let Some(init) = init {
                if !expr_is_string_subset(init, by_id) {
                    return None;
                }
            }
            Some(SlotTy::String)
        }
        Type::Null => {
            if let Some(init) = init {
                if !expr_is_undefined_subset(init, by_id) {
                    return None;
                }
            }
            Some(SlotTy::Undefined)
        }
        // Untyped: string index / for-in-of (string), `.length` (number), Math, or Number (N08.02.08 / N08.07.01 / N08.08.05–06).
        Type::Any => {
            if let Some(init) = init {
                if expr_is_string_length(init, by_id) {
                    return Some(SlotTy::Number);
                }
                if expr_is_math_number(init, by_id) {
                    return Some(SlotTy::Number);
                }
                if expr_is_number_ctor_const(init, by_id) {
                    return Some(SlotTy::Number);
                }
                if expr_is_number_ctor_bool(init, by_id) {
                    return Some(SlotTy::Boolean);
                }
                if expr_is_string_subset(init, by_id) {
                    return Some(SlotTy::String);
                }
                return None;
            }
            Some(SlotTy::String)
        }
        _ => None,
    }
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_locals = Vec::new();
    let mut alloc_locals = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let slot = slot_for_declare(*local, init, &by_id)?;
                if seen.insert(*local) {
                    user_locals.push((*local, slot));
                    alloc_locals.push((*local, slot));
                }
            }
            Stmt::Expr { .. }
            | Stmt::Block { .. }
            | Stmt::If { .. }
            | Stmt::While { .. }
            | Stmt::DoWhile { .. }
            | Stmt::For { .. }
            | Stmt::ForIn { .. }
            | Stmt::ForOf { .. }
            | Stmt::Switch { .. }
            | Stmt::Labeled { .. } => {
                if !stmt_is_subset(stmt, &by_id) {
                    return None;
                }
                collect_for_init_allocs(stmt, &by_id, &mut alloc_locals, &mut seen)?;
            }
            _ => return None,
        }
    }
    if user_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        user_locals,
        alloc_locals,
    })
}

/// Collect `for (let|const x = …)` / `for (let|const k in/of …)` locals into alloc slots (not prints).
fn collect_for_init_allocs(
    stmt: &Stmt,
    by_id: &HashMap<LocalId, &Local>,
    alloc_locals: &mut Vec<(LocalId, SlotTy)>,
    seen: &mut std::collections::HashSet<LocalId>,
) -> Option<()> {
    match stmt {
        Stmt::For { init, body, .. } => {
            if let Some(i) = init.as_ref() {
                if let Stmt::Declare { local, init, .. } = i.as_ref() {
                    let slot = slot_for_declare(*local, init, by_id)?;
                    if seen.insert(*local) {
                        alloc_locals.push((*local, slot));
                    }
                } else {
                    collect_for_init_allocs(i, by_id, alloc_locals, seen)?;
                }
            }
            collect_for_init_allocs(body, by_id, alloc_locals, seen)
        }
        Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
            if let Stmt::Declare { local, init, .. } = left.as_ref() {
                let slot = slot_for_declare(*local, init, by_id)?;
                if seen.insert(*local) {
                    alloc_locals.push((*local, slot));
                }
            } else {
                collect_for_init_allocs(left, by_id, alloc_locals, seen)?;
            }
            collect_for_init_allocs(body, by_id, alloc_locals, seen)
        }
        Stmt::Block { body } => {
            for s in body {
                collect_for_init_allocs(s, by_id, alloc_locals, seen)?;
            }
            Some(())
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_for_init_allocs(consequent, by_id, alloc_locals, seen)?;
            if let Some(a) = alternate {
                collect_for_init_allocs(a, by_id, alloc_locals, seen)?;
            }
            Some(())
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
            collect_for_init_allocs(body, by_id, alloc_locals, seen)
        }
        Stmt::Switch { cases, .. } => {
            for c in cases {
                for s in &c.body {
                    collect_for_init_allocs(s, by_id, alloc_locals, seen)?;
                }
            }
            Some(())
        }
        Stmt::Labeled { body, .. } => collect_for_init_allocs(body, by_id, alloc_locals, seen),
        _ => Some(()),
    }
}

/// Nested statement subset for control-flow bodies and blocks.
/// `for` / `for-in` / `for-of` may introduce nested `let` bindings.
/// Labeled statements + labeled/unlabeled `break`/`continue` (N08.02.07).
/// `for-in`/`for-of` iterate strings only this Loop (N08.02.08).
/// `switch` discriminant and case tests are number subset only.
fn stmt_is_subset(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Expr { expr } => match expr.ty() {
            Type::Number => expr_is_number_subset(expr, by_id),
            Type::BigInt => expr_is_bigint_subset(expr, by_id),
            Type::Boolean => expr_is_boolean_subset(expr, by_id),
            Type::String => expr_is_string_subset(expr, by_id),
            Type::Null => expr_is_undefined_subset(expr, by_id),
            // Assignment-form for-in/of left: bare local ref.
            Type::Any => matches!(
                expr,
                Expr::Local { id, .. } if by_id.get(id).is_some_and(|l| l.ty == Type::Any)
            ),
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
        Stmt::While { test, body } | Stmt::DoWhile { test, body } => {
            (expr_is_boolean_subset(test, by_id) || expr_is_number_subset(test, by_id))
                && stmt_is_subset(body, by_id)
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            let init_ok = init
                .as_ref()
                .map(|i| match i.as_ref() {
                    Stmt::Declare { local, init, .. } => {
                        slot_for_declare(*local, init, by_id).is_some()
                    }
                    other => stmt_is_subset(other, by_id),
                })
                .unwrap_or(true);
            let test_ok = test
                .as_ref()
                .map(|t| expr_is_boolean_subset(t, by_id) || expr_is_number_subset(t, by_id))
                .unwrap_or(true);
            let update_ok = update
                .as_ref()
                .map(|u| match u.ty() {
                    Type::Number => expr_is_number_subset(u, by_id),
                    Type::Boolean => expr_is_boolean_subset(u, by_id),
                    Type::String => expr_is_string_subset(u, by_id),
                    Type::Null => expr_is_undefined_subset(u, by_id),
                    _ => false,
                })
                .unwrap_or(true);
            init_ok && test_ok && update_ok && stmt_is_subset(body, by_id)
        }
        Stmt::ForIn { left, right, body } => {
            for_in_of_left_ok(left, by_id)
                && expr_is_string_subset(right, by_id)
                && stmt_is_subset(body, by_id)
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
        } => {
            !*is_await
                && for_in_of_left_ok(left, by_id)
                && expr_is_string_subset(right, by_id)
                && stmt_is_subset(body, by_id)
        }
        Stmt::Switch {
            discriminant,
            cases,
        } => {
            if !expr_is_number_subset(discriminant, by_id) {
                return false;
            }
            cases.iter().all(|c| {
                let test_ok = c
                    .test
                    .as_ref()
                    .map(|t| expr_is_number_subset(t, by_id))
                    .unwrap_or(true);
                test_ok && c.body.iter().all(|s| stmt_is_subset(s, by_id))
            })
        }
        Stmt::Labeled { body, .. } => stmt_is_subset(body, by_id),
        Stmt::Break { .. } | Stmt::Continue { .. } => true,
        _ => false,
    }
}

fn for_in_of_left_ok(left: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match left {
        Stmt::Declare { local, init, .. } => slot_for_declare(*local, init, by_id).is_some(),
        Stmt::Expr {
            expr: Expr::Local { id, ty },
        } => {
            (*ty == Type::Any || *ty == Type::String)
                && by_id.get(id).is_some_and(|l| l.ty == *ty || l.ty == Type::Any)
        }
        _ => false,
    }
}

/// Operand of `typeof` / `void` / `delete` in the supported subset.
fn expr_is_unary_keyword_arg(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { ty, .. } => *ty == Type::Number,
        Expr::BigInt { ty, .. } => *ty == Type::BigInt,
        Expr::String { ty, .. } => *ty == Type::String,
        Expr::Boolean { ty, .. } => *ty == Type::Boolean,
        Expr::Null { ty } => *ty == Type::Null,
        Expr::Local { id, ty } => {
            if is_math_global_local(*id, *ty, by_id) {
                return true;
            }
            if is_number_ctor_local(*id, *ty, by_id) {
                return true;
            }
            if is_nan_or_infinity_local(*id, *ty, by_id) {
                return true;
            }
            matches!(
                ty,
                Type::Number
                    | Type::BigInt
                    | Type::String
                    | Type::Boolean
                    | Type::Null
                    | Type::Any
            ) && by_id.get(id).is_some_and(|l| l.ty == *ty)
        }
        e if expr_is_number_subset(e, by_id) => true,
        e if expr_is_bigint_subset(e, by_id) => true,
        e if expr_is_boolean_subset(e, by_id) => true,
        e if expr_is_string_subset(e, by_id) => true,
        e if expr_is_undefined_subset(e, by_id) => true,
        _ => false,
    }
}

/// Global `Math` binding (host builtin local; N08.08.05).
fn is_math_global_local(id: LocalId, ty: Type, by_id: &HashMap<LocalId, &Local>) -> bool {
    ty == Type::Object
        && by_id
            .get(&id)
            .is_some_and(|l| l.name == "Math" && l.ty == Type::Object)
}

fn is_math_global_expr(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, ty } => is_math_global_local(*id, *ty, by_id),
        _ => false,
    }
}

/// `Math.prop` / `Math["prop"]` → property name when object is the Math global.
fn math_member_name<'a>(expr: &'a Expr, by_id: &HashMap<LocalId, &Local>) -> Option<String> {
    match expr {
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } if is_math_global_expr(object, by_id) => match property.as_ref() {
            Expr::String { value, .. } => Some(value.to_string_lossy()),
            _ => None,
        },
        _ => None,
    }
}

fn is_math_const_name(name: &str) -> bool {
    matches!(name, "E" | "PI" | "LN2" | "LOG2E")
}

fn is_math_method_name(name: &str) -> bool {
    matches!(
        name,
        "abs" | "floor" | "ceil" | "round" | "min" | "max" | "pow" | "sqrt" | "sign"
    )
}

/// Number-producing `Math` member or call (E08.05 / N08.08.05).
fn expr_is_math_number(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    if let Some(name) = math_member_name(expr, by_id) {
        return is_math_const_name(&name);
    }
    match expr {
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let Some(name) = math_member_name(callee, by_id) else {
                return false;
            };
            if !is_math_method_name(&name) {
                return false;
            }
            let n = args.len();
            let arity_ok = match name.as_str() {
                "min" | "max" => n >= 1,
                "pow" => n == 2,
                _ => n == 1,
            };
            arity_ok
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_is_number_subset(e, by_id),
                    _ => false,
                })
        }
        _ => false,
    }
}

/// Global `Number` constructor binding (host builtin local; N08.08.06).
fn is_number_ctor_local(id: LocalId, ty: Type, by_id: &HashMap<LocalId, &Local>) -> bool {
    ty == Type::Function
        && by_id
            .get(&id)
            .is_some_and(|l| l.name == "Number" && l.ty == Type::Function)
}

fn is_number_ctor_expr(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, ty } => is_number_ctor_local(*id, *ty, by_id),
        _ => false,
    }
}

/// Global `NaN` / `Infinity` number bindings (host builtins; N08.08.06).
fn is_nan_or_infinity_local(id: LocalId, ty: Type, by_id: &HashMap<LocalId, &Local>) -> bool {
    ty == Type::Number
        && by_id.get(&id).is_some_and(|l| {
            l.ty == Type::Number && (l.name == "NaN" || l.name == "Infinity")
        })
}

fn nan_or_infinity_name(id: LocalId, by_id: &HashMap<LocalId, &Local>) -> Option<&'static str> {
    let l = by_id.get(&id)?;
    if l.ty != Type::Number {
        return None;
    }
    match l.name.as_str() {
        "NaN" => Some("NaN"),
        "Infinity" => Some("Infinity"),
        _ => None,
    }
}

/// `Number.prop` / `Number["prop"]` → property name when object is the Number constructor.
fn number_ctor_member_name<'a>(expr: &'a Expr, by_id: &HashMap<LocalId, &Local>) -> Option<String> {
    match expr {
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } if is_number_ctor_expr(object, by_id) => match property.as_ref() {
            Expr::String { value, .. } => Some(value.to_string_lossy()),
            _ => None,
        },
        _ => None,
    }
}

fn is_number_ctor_const_name(name: &str) -> bool {
    matches!(
        name,
        "NaN"
            | "POSITIVE_INFINITY"
            | "NEGATIVE_INFINITY"
            | "MAX_VALUE"
            | "MIN_VALUE"
            | "EPSILON"
            | "MAX_SAFE_INTEGER"
            | "MIN_SAFE_INTEGER"
    )
}

fn is_number_ctor_method_name(name: &str) -> bool {
    matches!(name, "isNaN" | "isFinite" | "isInteger" | "isSafeInteger")
}

/// Number-producing `Number.*` constant member (E08.06 / N08.08.06).
fn expr_is_number_ctor_const(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    if let Some(name) = number_ctor_member_name(expr, by_id) {
        return is_number_ctor_const_name(&name);
    }
    false
}

/// Boolean-producing `Number.isNaN` / `isFinite` / `isInteger` / `isSafeInteger` call.
fn expr_is_number_ctor_bool(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let Some(name) = number_ctor_member_name(callee, by_id) else {
                return false;
            };
            if !is_number_ctor_method_name(&name) {
                return false;
            }
            args.len() == 1
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_is_number_subset(e, by_id),
                    _ => false,
                })
        }
        _ => false,
    }
}

/// Same-type BigInt subset (E08.02–E08.04 / N08.08.02–N08.08.04): literals, unary `-`/`~`,
/// `+` `-` `*` `/` `%`, bitwise `&` `|` `^` `<<` `>>` (no `>>>`), `**` / `**=`, locals.
/// Values must fit signed i64 at emit; `**` exponents non-negative in fixtures.
fn expr_is_bigint_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::BigInt { ty, .. } => *ty == Type::BigInt,
        Expr::Local { id, ty } => {
            *ty == Type::BigInt && by_id.get(id).is_some_and(|l| l.ty == Type::BigInt)
        }
        Expr::Unary { op, arg, ty } => {
            *ty == Type::BigInt
                && matches!(op, UnaryOp::Minus | UnaryOp::BitNot)
                && expr_is_bigint_subset(arg, by_id)
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            *ty == Type::BigInt
                && matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Rem
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Shl
                        | BinaryOp::Shr
                        | BinaryOp::Pow
                        | BinaryOp::Comma
                )
                && expr_is_bigint_subset(left, by_id)
                && expr_is_bigint_subset(right, by_id)
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::BigInt
                && matches!(op, AssignOp::Eq | AssignOp::PowEq)
                && matches!(
                    target,
                    AssignTarget::Local(id) if by_id.get(id).is_some_and(|l| l.ty == Type::BigInt)
                )
                && expr_is_bigint_subset(value, by_id)
        }
        _ => false,
    }
}

fn expr_is_string_subset(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::String { ty, .. } => *ty == Type::String,
        // N08.07.02: untagged template → string (cooked quasis + ToString interpolations).
        Expr::Template {
            expressions, ty, ..
        } => {
            *ty == Type::String
                && expressions
                    .iter()
                    .all(|e| expr_is_concat_operand(e, by_id))
        }
        Expr::Local { id, ty } => {
            (*ty == Type::String
                && by_id
                    .get(id)
                    .is_some_and(|l| l.ty == Type::String || l.ty == Type::Any))
                || (*ty == Type::Any && by_id.get(id).is_some_and(|l| l.ty == Type::Any))
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
                && match op {
                    BinaryOp::Comma => {
                        expr_is_unary_keyword_arg(left, by_id) && expr_is_string_subset(right, by_id)
                    }
                    // String concat (N08.02.08 / N08.07.01); number operand → ToString.
                    BinaryOp::Add => {
                        expr_is_concat_operand(left, by_id) && expr_is_concat_operand(right, by_id)
                            && (expr_is_string_operand(left, by_id)
                                || expr_is_string_operand(right, by_id))
                    }
                    _ => false,
                }
        }
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ty,
        } => {
            !*optional
                && (*ty == Type::String || *ty == Type::Any)
                && expr_is_string_subset(object, by_id)
                && *computed
                && expr_is_number_subset(property, by_id)
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            *ty == Type::String
                && matches!(op, AssignOp::Eq)
                && matches!(
                    target,
                    AssignTarget::Local(id)
                        if by_id.get(id).is_some_and(|l| {
                            l.ty == Type::String || l.ty == Type::Any
                        })
                )
                && expr_is_string_subset(value, by_id)
        }
        _ => false,
    }
}

/// `s.length` → number (IR types Member as `any`).
fn expr_is_string_length(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } => {
            !*optional
                && !*computed
                && expr_is_string_subset(object, by_id)
                && matches!(
                    property.as_ref(),
                    Expr::String { value, .. } if value.to_string_lossy() == "length"
                )
        }
        _ => false,
    }
}

fn expr_is_string_operand(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, ty } if *ty == Type::Any => {
            by_id.get(id).is_some_and(|l| l.ty == Type::Any)
        }
        e => expr_is_string_subset(e, by_id),
    }
}

fn expr_is_concat_operand(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    expr_is_string_operand(expr, by_id) || expr_is_number_subset(expr, by_id)
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
        // N08.07.01: `s.length` (Member typed `any`).
        e if expr_is_string_length(e, by_id) => true,
        // N08.08.05: `Math.*` constants/methods (Member/Call typed `any`).
        e if expr_is_math_number(e, by_id) => true,
        // N08.08.06: `Number.*` constants (Member typed `any`).
        e if expr_is_number_ctor_const(e, by_id) => true,
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
    // N08.08.06: `Number.isNaN(…)` etc. are typed `any` but produce boolean.
    if expr_is_number_ctor_bool(expr, by_id) {
        return true;
    }
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
                    (expr_is_number_subset(left, by_id) && expr_is_number_subset(right, by_id))
                        || (expr_is_bigint_subset(left, by_id)
                            && expr_is_bigint_subset(right, by_id))
                }
                BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq => {
                    (expr_is_number_subset(left, by_id) && expr_is_number_subset(right, by_id))
                        || (expr_is_bigint_subset(left, by_id)
                            && expr_is_bigint_subset(right, by_id))
                        || (expr_is_boolean_subset(left, by_id)
                            && expr_is_boolean_subset(right, by_id))
                        || (expr_is_string_subset(left, by_id)
                            && expr_is_string_subset(right, by_id))
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

/// Targets for `break` / `continue` (unlabeled = innermost; labeled = matching `names`).
/// `continue_label` is `None` for `switch` and labeled non-iteration statements.
struct CtrlFrame {
    names: Vec<String>,
    break_label: String,
    continue_label: Option<String>,
}

/// Length-aware string SSA value (N08.07.01; supports embedded NUL).
struct StrVal {
    data: String,
    len: String,
}

struct Emitter<'a> {
    module: &'a Module,
    /// local id → (alloca ptr name, slot type)
    allocas: HashMap<LocalId, (String, SlotTy)>,
    /// string local id → length alloca ptr name (`i64`)
    string_lens: HashMap<LocalId, String>,
    /// WTF-8 string content → global name (e.g. `.str.0`)
    str_globals: HashMap<Vec<u8>, String>,
    out: String,
    body: String,
    tmp: u32,
    ctrls: Vec<CtrlFrame>,
    /// Labels from enclosing `label:` wrappers applied to the next loop/frame.
    pending_names: Vec<String>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        Self {
            module,
            allocas: HashMap::new(),
            string_lens: HashMap::new(),
            str_globals: HashMap::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
            ctrls: Vec::new(),
            pending_names: Vec::new(),
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

    fn emit_module(
        &mut self,
        alloc: &[(LocalId, SlotTy)],
        user: &[(LocalId, SlotTy)],
    ) -> Result<(), Diagnostic> {
        // Body first so string globals are collected, then header + globals + main.
        for (id, slot) in alloc {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, (ptr.clone(), *slot));
            match slot {
                SlotTy::Number => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                }
                SlotTy::BigInt => {
                    writeln!(self.body, "  {ptr} = alloca i64, align 8").ok();
                }
                SlotTy::Boolean => {
                    writeln!(self.body, "  {ptr} = alloca i1, align 1").ok();
                }
                SlotTy::String => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    let len_ptr = format!("%l{}_len", id.0);
                    writeln!(self.body, "  {len_ptr} = alloca i64, align 8").ok();
                    self.string_lens.insert(*id, len_ptr);
                }
                // No runtime payload; print always emits `undefined`.
                SlotTy::Undefined => {}
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        // Print top-level user locals in declaration order (not for-init bindings).
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
                SlotTy::BigInt => {
                    let (ptr, _) = self
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("internal: print missing alloca"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {v}"))).ok();
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
                    let len_ptr = self
                        .string_lens
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("internal: print missing string len"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    let n = self.fresh();
                    writeln!(self.body, "  {n} = load i64, ptr {len_ptr}").ok();
                    writeln!(
                        self.body,
                        "  {}",
                        PRINT_BYTES.call(&format!("ptr {v}, i64 {n}"))
                    )
                    .ok();
                }
                SlotTy::Undefined => {
                    let p = self.string_const("undefined")?;
                    writeln!(
                        self.body,
                        "  {}",
                        PRINT_BYTES.call(&format!("ptr {}, i64 {}", p.data, p.len))
                    )
                    .ok();
                }
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.01/N08.02/N08.07.01–N08.07.05 ES expressions + control + strings via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        // JS `**` / Math.pow + Math.* methods (IEEE f64 intrinsics; no extra libm flags).
        writeln!(self.out, "declare double @llvm.pow.f64(double, double)").ok();
        writeln!(self.out, "declare double @llvm.fabs.f64(double)").ok();
        writeln!(self.out, "declare double @llvm.floor.f64(double)").ok();
        writeln!(self.out, "declare double @llvm.ceil.f64(double)").ok();
        writeln!(self.out, "declare double @llvm.round.f64(double)").ok();
        writeln!(self.out, "declare double @llvm.sqrt.f64(double)").ok();
        writeln!(self.out).ok();

        for (content, gname) in &self.str_globals {
            let n = content.len() + 1;
            let esc = escape_llvm_bytes(content);
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
                    (SlotTy::BigInt, Some(init)) => {
                        let v = self.emit_bigint_expr(init)?;
                        writeln!(self.body, "  store i64 {v}, ptr {ptr}").ok();
                    }
                    (SlotTy::Boolean, Some(init)) => {
                        let v = self.emit_bool_expr(init)?;
                        writeln!(self.body, "  store i1 {v}, ptr {ptr}").ok();
                    }
                    (SlotTy::String, Some(init)) => {
                        let v = self.emit_string_expr(init)?;
                        self.store_string_local(*local, &v)?;
                    }
                    (SlotTy::Undefined, Some(init)) => {
                        self.emit_undefined_expr(init)?;
                    }
                    // Uninitialized string `let` (incl. any) — empty until assigned.
                    (SlotTy::String, None) => {
                        let v = self.string_const("")?;
                        self.store_string_local(*local, &v)?;
                    }
                    // Uninitialized number/bigint/bool/undefined — leave alloca undef until assigned.
                    (_, None) => {}
                }
                Ok(())
            }
            Stmt::Expr { expr } => match expr.ty() {
                Type::Number => {
                    let _ = self.emit_number_expr(expr)?;
                    Ok(())
                }
                Type::BigInt => {
                    let _ = self.emit_bigint_expr(expr)?;
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
                    if self.body_ends_with_terminator() {
                        break;
                    }
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
            Stmt::While { test, body } => {
                let names = std::mem::take(&mut self.pending_names);
                let head = self.fresh_label("while_head");
                let bod = self.fresh_label("while_body");
                let end = self.fresh_label("while_end");
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{head}:").ok();
                let cond = self.emit_to_boolean(test)?;
                writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                writeln!(self.body, "{bod}:").ok();
                self.ctrls.push(CtrlFrame {
                    names,
                    break_label: end.clone(),
                    continue_label: Some(head.clone()),
                });
                self.emit_stmt(body)?;
                self.ctrls.pop();
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{head}").ok();
                }
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::DoWhile { body, test } => {
                let names = std::mem::take(&mut self.pending_names);
                let bod = self.fresh_label("do_body");
                let head = self.fresh_label("do_test");
                let end = self.fresh_label("do_end");
                writeln!(self.body, "  br label %{bod}").ok();
                writeln!(self.body, "{bod}:").ok();
                self.ctrls.push(CtrlFrame {
                    names,
                    break_label: end.clone(),
                    continue_label: Some(head.clone()),
                });
                self.emit_stmt(body)?;
                self.ctrls.pop();
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{head}").ok();
                }
                writeln!(self.body, "{head}:").ok();
                let cond = self.emit_to_boolean(test)?;
                writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::For {
                init,
                test,
                update,
                body,
            } => {
                let names = std::mem::take(&mut self.pending_names);
                if let Some(i) = init {
                    self.emit_stmt(i)?;
                }
                let head = self.fresh_label("for_head");
                let bod = self.fresh_label("for_body");
                let upd = self.fresh_label("for_update");
                let end = self.fresh_label("for_end");
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{head}:").ok();
                if let Some(t) = test {
                    let cond = self.emit_to_boolean(t)?;
                    writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                } else {
                    writeln!(self.body, "  br label %{bod}").ok();
                }
                writeln!(self.body, "{bod}:").ok();
                self.ctrls.push(CtrlFrame {
                    names,
                    break_label: end.clone(),
                    continue_label: Some(upd.clone()),
                });
                self.emit_stmt(body)?;
                self.ctrls.pop();
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{upd}").ok();
                }
                writeln!(self.body, "{upd}:").ok();
                if let Some(u) = update {
                    match u.ty() {
                        Type::Number => {
                            let _ = self.emit_number_expr(u)?;
                        }
                        Type::Boolean => {
                            let _ = self.emit_bool_expr(u)?;
                        }
                        Type::String => {
                            let _ = self.emit_string_expr(u)?;
                        }
                        Type::Null => {
                            self.emit_undefined_expr(u)?;
                        }
                        _ => {
                            return Err(diag("internal: unsupported for update ty in es_expr"));
                        }
                    }
                }
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::Switch {
                discriminant,
                cases,
            } => {
                let names = std::mem::take(&mut self.pending_names);
                let disc = self.emit_number_expr(discriminant)?;
                let end = self.fresh_label("switch_end");
                let case_labels: Vec<String> = (0..cases.len())
                    .map(|i| self.fresh_label(&format!("case{i}_")))
                    .collect();
                let default_idx = cases.iter().position(|c| c.test.is_none());
                let default_target = default_idx
                    .map(|i| case_labels[i].clone())
                    .unwrap_or_else(|| end.clone());

                // Match chain: first case with Strict Equality (===) on numbers.
                for (i, c) in cases.iter().enumerate() {
                    if let Some(test) = &c.test {
                        let try_l = self.fresh_label("sw_try");
                        writeln!(self.body, "  br label %{try_l}").ok();
                        writeln!(self.body, "{try_l}:").ok();
                        let tv = self.emit_number_expr(test)?;
                        let eq = self.fresh();
                        writeln!(self.body, "  {eq} = fcmp oeq double {disc}, {tv}").ok();
                        let next = self.fresh_label("sw_next");
                        writeln!(
                            self.body,
                            "  br i1 {eq}, label %{}, label %{next}",
                            case_labels[i]
                        )
                        .ok();
                        writeln!(self.body, "{next}:").ok();
                    }
                }
                writeln!(self.body, "  br label %{default_target}").ok();

                self.ctrls.push(CtrlFrame {
                    names,
                    break_label: end.clone(),
                    continue_label: None,
                });
                for (i, c) in cases.iter().enumerate() {
                    writeln!(self.body, "{}:", case_labels[i]).ok();
                    for s in &c.body {
                        if self.body_ends_with_terminator() {
                            break;
                        }
                        self.emit_stmt(s)?;
                    }
                    if !self.body_ends_with_terminator() {
                        if i + 1 < cases.len() {
                            writeln!(self.body, "  br label %{}", case_labels[i + 1]).ok();
                        } else {
                            writeln!(self.body, "  br label %{end}").ok();
                        }
                    }
                }
                self.ctrls.pop();
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::ForIn { left, right, body } => {
                self.emit_for_in_of(left, right, body, /* is_of */ false)
            }
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
            } => {
                if *is_await {
                    return Err(diag("internal: for-await-of not in es_expr subset"));
                }
                self.emit_for_in_of(left, right, body, /* is_of */ true)
            }
            Stmt::Labeled { label, body } => match body.as_ref() {
                Stmt::While { .. }
                | Stmt::DoWhile { .. }
                | Stmt::For { .. }
                | Stmt::ForIn { .. }
                | Stmt::ForOf { .. }
                | Stmt::Switch { .. }
                | Stmt::Labeled { .. } => {
                    self.pending_names.push(label.clone());
                    self.emit_stmt(body)
                }
                _ => {
                    let end = self.fresh_label("lbl_end");
                    let mut names = std::mem::take(&mut self.pending_names);
                    names.push(label.clone());
                    self.ctrls.push(CtrlFrame {
                        names,
                        break_label: end.clone(),
                        continue_label: None,
                    });
                    self.emit_stmt(body)?;
                    self.ctrls.pop();
                    if !self.body_ends_with_terminator() {
                        writeln!(self.body, "  br label %{end}").ok();
                    }
                    writeln!(self.body, "{end}:").ok();
                    Ok(())
                }
            },
            Stmt::Break { label: None } => {
                let frame = self
                    .ctrls
                    .last()
                    .ok_or_else(|| diag("internal: break outside loop/switch in es_expr"))?;
                let end = frame.break_label.clone();
                writeln!(self.body, "  br label %{end}").ok();
                Ok(())
            }
            Stmt::Break { label: Some(name) } => {
                let end = self
                    .ctrls
                    .iter()
                    .rev()
                    .find(|f| f.names.iter().any(|n| n == name))
                    .map(|f| f.break_label.clone())
                    .ok_or_else(|| diag("internal: labeled break target missing in es_expr"))?;
                writeln!(self.body, "  br label %{end}").ok();
                Ok(())
            }
            Stmt::Continue { label: None } => {
                let cont = self
                    .ctrls
                    .iter()
                    .rev()
                    .find_map(|f| f.continue_label.clone())
                    .ok_or_else(|| diag("internal: continue outside loop in es_expr"))?;
                writeln!(self.body, "  br label %{cont}").ok();
                Ok(())
            }
            Stmt::Continue { label: Some(name) } => {
                let cont = self
                    .ctrls
                    .iter()
                    .rev()
                    .find(|f| f.names.iter().any(|n| n == name))
                    .and_then(|f| f.continue_label.clone())
                    .ok_or_else(|| {
                        diag("internal: labeled continue target missing or not iteration")
                    })?;
                writeln!(self.body, "  br label %{cont}").ok();
                Ok(())
            }
            _ => Err(diag("internal: unsupported stmt in es_expr module")),
        }
    }

    /// `for (let k in s)` / `for (let c of s)` over strings (N08.02.08).
    fn emit_for_in_of(
        &mut self,
        left: &Stmt,
        right: &Expr,
        body: &Stmt,
        is_of: bool,
    ) -> Result<(), Diagnostic> {
        let names = std::mem::take(&mut self.pending_names);
        // Ensure for-in/of `let` binding has an alloca (also collected in classify).
        if let Stmt::Declare { local, init, .. } = left {
            if init.is_some() {
                return Err(diag("internal: for-in/of left declare must not have init"));
            }
            if !self.allocas.contains_key(local) {
                let ptr = format!("%l{}", local.0);
                writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                self.allocas.insert(*local, (ptr, SlotTy::String));
                let len_ptr = format!("%l{}_len", local.0);
                writeln!(self.body, "  {len_ptr} = alloca i64, align 8").ok();
                self.string_lens.insert(*local, len_ptr);
            }
        }
        let s = self.emit_string_expr(right)?;
        let idx_ptr = self.fresh();
        writeln!(self.body, "  {idx_ptr} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {idx_ptr}").ok();
        let head = self.fresh_label(if is_of { "forof_head" } else { "forin_head" });
        let bod = self.fresh_label(if is_of { "forof_body" } else { "forin_body" });
        let cont = self.fresh_label(if is_of { "forof_cont" } else { "forin_cont" });
        let end = self.fresh_label(if is_of { "forof_end" } else { "forin_end" });
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{head}:").ok();
        let idx = self.fresh();
        writeln!(self.body, "  {idx} = load i64, ptr {idx_ptr}").ok();
        // N08.07.05: for-in/of over strings iterates UTF-16 code units.
        let units = self.fresh();
        writeln!(
            self.body,
            "  {}",
            UTF16_LEN.call_to(&units, &format!("ptr {}, i64 {}", s.data, s.len))
        )
        .ok();
        let cmp = self.fresh();
        writeln!(self.body, "  {cmp} = icmp ult i64 {idx}, {units}").ok();
        writeln!(self.body, "  br i1 {cmp}, label %{bod}, label %{end}").ok();
        writeln!(self.body, "{bod}:").ok();
        let bound = if is_of {
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
            StrVal {
                data: ch,
                len: ch_len,
            }
        } else {
            let key = self.fresh();
            writeln!(
                self.body,
                "  {}",
                CSTR_FROM_U64.call_to(&key, &format!("i64 {idx}"))
            )
            .ok();
            let key_len = self.fresh();
            writeln!(
                self.body,
                "  {}",
                CSTR_LEN.call_to(&key_len, &format!("ptr {key}"))
            )
            .ok();
            StrVal {
                data: key,
                len: key_len,
            }
        };
        self.store_for_in_of_left(left, &bound)?;
        self.ctrls.push(CtrlFrame {
            names,
            break_label: end.clone(),
            continue_label: Some(cont.clone()),
        });
        self.emit_stmt(body)?;
        self.ctrls.pop();
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  br label %{cont}").ok();
        }
        writeln!(self.body, "{cont}:").ok();
        let idx2 = self.fresh();
        writeln!(self.body, "  {idx2} = load i64, ptr {idx_ptr}").ok();
        let next = self.fresh();
        writeln!(self.body, "  {next} = add i64 {idx2}, 1").ok();
        writeln!(self.body, "  store i64 {next}, ptr {idx_ptr}").ok();
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{end}:").ok();
        Ok(())
    }

    fn store_for_in_of_left(&mut self, left: &Stmt, value: &StrVal) -> Result<(), Diagnostic> {
        let id = match left {
            Stmt::Declare { local, .. } => *local,
            Stmt::Expr {
                expr: Expr::Local { id, .. },
            } => *id,
            _ => return Err(diag("internal: unsupported for-in/of left")),
        };
        self.store_string_local(id, value)
    }

    fn store_string_local(&mut self, id: LocalId, value: &StrVal) -> Result<(), Diagnostic> {
        let (ptr, slot) = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag(format!("internal: unallocated string local %{}", id.0)))?;
        if slot != SlotTy::String {
            return Err(diag("internal: expected string slot"));
        }
        let len_ptr = self
            .string_lens
            .get(&id)
            .cloned()
            .ok_or_else(|| diag(format!("internal: missing string len alloca %{}", id.0)))?;
        writeln!(self.body, "  store ptr {}, ptr {ptr}", value.data).ok();
        writeln!(self.body, "  store i64 {}, ptr {len_ptr}", value.len).ok();
        Ok(())
    }

    fn load_string_local(&mut self, id: LocalId) -> Result<StrVal, Diagnostic> {
        let (ptr, slot) = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag(format!("internal: unallocated local %{}", id.0)))?;
        if slot != SlotTy::String {
            return Err(diag("internal: expected string local"));
        }
        let len_ptr = self
            .string_lens
            .get(&id)
            .cloned()
            .ok_or_else(|| diag(format!("internal: missing string len %{}", id.0)))?;
        let data = self.fresh();
        writeln!(self.body, "  {data} = load ptr, ptr {ptr}").ok();
        let len = self.fresh();
        writeln!(self.body, "  {len} = load i64, ptr {len_ptr}").ok();
        Ok(StrVal { data, len })
    }

    fn string_const(&mut self, s: &str) -> Result<StrVal, Diagnostic> {
        self.string_const_bytes(s.as_bytes())
    }

    fn string_const_js(&mut self, value: &JsString) -> Result<StrVal, Diagnostic> {
        let bytes = jsstring_to_wtf8(value);
        self.string_const_bytes(&bytes)
    }

    fn string_const_bytes(&mut self, bytes: &[u8]) -> Result<StrVal, Diagnostic> {
        let gname = if let Some(g) = self.str_globals.get(bytes) {
            g.clone()
        } else {
            let g = format!(".str.{}", self.str_globals.len());
            self.str_globals.insert(bytes.to_vec(), g.clone());
            g
        };
        let t = self.fresh();
        let n = bytes.len() + 1;
        writeln!(
            self.body,
            "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
        )
        .ok();
        Ok(StrVal {
            data: t,
            len: format!("{}", bytes.len()),
        })
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
            Expr::BigInt { .. } => {
                let _ = self.emit_bigint_expr(expr)?;
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
            Expr::Local { id, ty } => {
                // N08.08.05–06: host globals are not stack-allocated.
                if *ty == Type::Object
                    && self
                        .module
                        .locals
                        .iter()
                        .any(|l| l.id == *id && l.name == "Math")
                {
                    return Ok(());
                }
                if *ty == Type::Function
                    && self
                        .module
                        .locals
                        .iter()
                        .any(|l| l.id == *id && l.name == "Number")
                {
                    return Ok(());
                }
                if *ty == Type::Number
                    && self.module.locals.iter().any(|l| {
                        l.id == *id && (l.name == "NaN" || l.name == "Infinity")
                    })
                {
                    return Ok(());
                }
                let (_, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag(format!("internal: unallocated discard local %{}", id.0)))?;
                match slot {
                    SlotTy::Number => {
                        let _ = self.emit_number_expr(expr)?;
                    }
                    SlotTy::BigInt => {
                        let _ = self.emit_bigint_expr(expr)?;
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
            e if matches!(e.ty(), Type::BigInt) => {
                let _ = self.emit_bigint_expr(e)?;
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

    fn typeof_name(
        expr: &Expr,
        slot_of: &HashMap<LocalId, SlotTy>,
        module: &Module,
    ) -> Option<&'static str> {
        match expr {
            Expr::Number { .. } => Some("number"),
            Expr::BigInt { .. } => Some("bigint"),
            Expr::String { .. } => Some("string"),
            Expr::Boolean { .. } => Some("boolean"),
            Expr::Null { .. } => Some("object"),
            Expr::Local { id, .. } => {
                if module.locals.iter().any(|l| l.id == *id && l.name == "Math") {
                    return Some("object");
                }
                if module
                    .locals
                    .iter()
                    .any(|l| l.id == *id && l.name == "Number")
                {
                    return Some("function");
                }
                if module.locals.iter().any(|l| {
                    l.id == *id && (l.name == "NaN" || l.name == "Infinity")
                }) {
                    return Some("number");
                }
                match slot_of.get(id)? {
                    SlotTy::Number => Some("number"),
                    SlotTy::BigInt => Some("bigint"),
                    SlotTy::String => Some("string"), // includes untyped any string slots
                    SlotTy::Boolean => Some("boolean"),
                    SlotTy::Undefined => Some("undefined"),
                }
            }
            Expr::Unary {
                op: UnaryOp::Void, ..
            } => Some("undefined"),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                ..
            } => Some("string"),
            // Number/bool/bigint-producing ops → type name strings
            e if matches!(e.ty(), Type::Number) => Some("number"),
            e if matches!(e.ty(), Type::BigInt) => Some("bigint"),
            e if matches!(e.ty(), Type::Boolean) => Some("boolean"),
            e if matches!(e.ty(), Type::String) => Some("string"),
            e if matches!(e.ty(), Type::Null) => Some("undefined"),
            _ => None,
        }
    }

    /// Emit a BigInt expression as signed i64 SSA (N08.08.02–N08.08.04).
    fn emit_bigint_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::BigInt { raw, .. } => Ok(format_bigint_const(raw)?),
            Expr::Local { id, .. } => {
                let (ptr, slot) = self
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
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
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
    fn emit_bigint_pow(&mut self, base: &str, exp: &str) -> Result<String, Diagnostic> {
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

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<StrVal, Diagnostic> {
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
                    return Err(diag("internal: template quasis/expressions length mismatch"));
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
                let slot_of: HashMap<LocalId, SlotTy> =
                    self.allocas.iter().map(|(k, (_, s))| (*k, *s)).collect();
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
                let v = self.emit_string_expr(value)?;
                self.store_string_local(*id, &v)?;
                Ok(v)
            }
            _ => Err(diag("internal: unsupported string expr in es_expr module")),
        }
    }

    /// String-producing operand for concat (string subset or `any` string slot).
    fn emit_string_operand(&mut self, expr: &Expr) -> Result<StrVal, Diagnostic> {
        match expr {
            Expr::Local { id, ty: Type::Any } => self.load_string_local(*id),
            e => self.emit_string_expr(e),
        }
    }

    fn emit_concat_strvals(&mut self, left: &StrVal, right: &StrVal) -> Result<StrVal, Diagnostic> {
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
    fn emit_concat_operand(&mut self, expr: &Expr) -> Result<StrVal, Diagnostic> {
        let as_number = match expr {
            Expr::Number { .. } => true,
            Expr::Local { id, .. } => self
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
                return Ok(format_math_const(&name)?);
            }
        }
        // N08.08.06: `Number.NaN` / `Number.MAX_VALUE` / …
        if let Some(name) = self.number_ctor_member_name_emit(expr) {
            if is_number_ctor_const_name(&name) {
                return Ok(format_number_ctor_const(&name)?);
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
                    return Ok(format_number_ctor_const(name)?);
                }
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

    /// Resolve `Math.prop` / `Math["prop"]` property name during emit.
    fn math_member_name_emit(&self, expr: &Expr) -> Option<String> {
        let by_id: HashMap<LocalId, &Local> =
            self.module.locals.iter().map(|l| (l.id, l)).collect();
        math_member_name(expr, &by_id)
    }

    /// Resolve `Number.prop` / `Number["prop"]` property name during emit.
    fn number_ctor_member_name_emit(&self, expr: &Expr) -> Option<String> {
        let by_id: HashMap<LocalId, &Local> =
            self.module.locals.iter().map(|l| (l.id, l)).collect();
        number_ctor_member_name(expr, &by_id)
    }

    /// True when `emit_number_expr` can lower this (Number ty, Math, Number consts, or `.length`).
    fn expr_emits_as_number(&self, expr: &Expr) -> bool {
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
    fn emit_number_ctor_bool_call(
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
                writeln!(self.body, "  {abs} = call double @llvm.fabs.f64(double {x})").ok();
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
                writeln!(self.body, "  {abs} = call double @llvm.fabs.f64(double {x})").ok();
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
                writeln!(self.body, "  {abs} = call double @llvm.fabs.f64(double {x})").ok();
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
    fn emit_math_call(&mut self, method: &str, args: &[Arg]) -> Result<String, Diagnostic> {
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

fn expr_ty_is_bigint(expr: &Expr) -> bool {
    matches!(expr.ty(), Type::BigInt)
}

fn expr_ty_is_string(expr: &Expr) -> bool {
    matches!(expr.ty(), Type::String)
}

/// Format a Math constant name as an LLVM `double` (N08.08.05 / E08.05).
fn format_math_const(name: &str) -> Result<String, Diagnostic> {
    let f = match name {
        "E" => std::f64::consts::E,
        "PI" => std::f64::consts::PI,
        "LN2" => std::f64::consts::LN_2,
        "LOG2E" => std::f64::consts::LOG2_E,
        _ => return Err(diag(format!("internal: unknown Math const {name}"))),
    };
    Ok(format!("{f:.17e}"))
}

/// Format a Number/NaN/Infinity constant as an LLVM `double` (N08.08.06 / E08.06).
fn format_number_ctor_const(name: &str) -> Result<String, Diagnostic> {
    match name {
        // Quiet NaN / ±Infinity as bit patterns (decimal `NaN` is not valid LLVM double text).
        "NaN" => Ok("0x7FF8000000000000".into()),
        "POSITIVE_INFINITY" | "Infinity" => Ok("0x7FF0000000000000".into()),
        "NEGATIVE_INFINITY" => Ok("0xFFF0000000000000".into()),
        "MAX_VALUE" => Ok(format!("{:.17e}", f64::MAX)),
        "MIN_VALUE" => Ok(format!("{:.17e}", f64::MIN_POSITIVE)),
        "EPSILON" => Ok(format!("{:.17e}", f64::EPSILON)),
        "MAX_SAFE_INTEGER" => Ok("9.0071992547409910e+15".into()),
        "MIN_SAFE_INTEGER" => Ok("-9.0071992547409910e+15".into()),
        _ => Err(diag(format!("internal: unknown Number const {name}"))),
    }
}

/// Format a JS number literal as an LLVM `double` constant (decimal/hex/bin/oct,
/// numeric separators, round-trip safe). N08.08.01 / E08.01.
fn format_number_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f = parse_js_number_literal(&cleaned)
        .ok_or_else(|| diag(format!("invalid number literal {raw}")))?;
    Ok(format!("{f:.17e}"))
}

/// Format a JS BigInt literal as an LLVM `i64` constant (decimal/hex/bin/oct,
/// numeric separators, trailing `n`). N08.08.02 / E08.02.
fn format_bigint_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let n = parse_js_bigint_literal(&cleaned)
        .ok_or_else(|| diag(format!("invalid BigInt literal {raw}")))?;
    Ok(n.to_string())
}

/// Parse ECMAScript BigInt literal text (no `_` separators; optional trailing `n`) to `i64`.
fn parse_js_bigint_literal(s: &str) -> Option<i64> {
    let s = s.strip_suffix('n').unwrap_or(s);
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok();
    }
    if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        return i64::from_str_radix(bin, 2).ok();
    }
    if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        return i64::from_str_radix(oct, 8).ok();
    }
    s.parse().ok()
}

/// Parse ECMAScript numeric literal text (no `_` separators) to `f64`.
fn parse_js_number_literal(s: &str) -> Option<f64> {
    if let Some(hex) = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16).ok().map(|n| n as f64);
    }
    if let Some(bin) = s
        .strip_prefix("0b")
        .or_else(|| s.strip_prefix("0B"))
    {
        return u64::from_str_radix(bin, 2).ok().map(|n| n as f64);
    }
    if let Some(oct) = s
        .strip_prefix("0o")
        .or_else(|| s.strip_prefix("0O"))
    {
        return u64::from_str_radix(oct, 8).ok().map(|n| n as f64);
    }
    s.parse().ok()
}

fn escape_llvm_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for b in bytes {
        match *b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) && c != b'\\' => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

/// Encode JS UTF-16 code units as WTF-8 (UTF-8 + unpaired surrogates as 3-byte sequences).
fn jsstring_to_wtf8(value: &JsString) -> Vec<u8> {
    let units = value.units();
    let mut out = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        if (0xD800..=0xDBFF).contains(&u) && i + 1 < units.len() {
            let v = units[i + 1];
            if (0xDC00..=0xDFFF).contains(&v) {
                let cp = 0x10000 + (((u as u32) - 0xD800) << 10) + ((v as u32) - 0xDC00);
                out.push(0xF0 | (cp >> 18) as u8);
                out.push(0x80 | ((cp >> 12) & 0x3F) as u8);
                out.push(0x80 | ((cp >> 6) & 0x3F) as u8);
                out.push(0x80 | (cp & 0x3F) as u8);
                i += 2;
                continue;
            }
        }
        if u < 0x80 {
            out.push(u as u8);
        } else if u < 0x800 {
            out.push(0xC0 | (u >> 6) as u8);
            out.push(0x80 | (u & 0x3F) as u8);
        } else {
            out.push(0xE0 | (u >> 12) as u8);
            out.push(0x80 | ((u >> 6) & 0x3F) as u8);
            out.push(0x80 | (u & 0x3F) as u8);
        }
        i += 1;
    }
    out
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
