//! N08.01.04.09: native observations for `??` / `??=` / `&&=` / `||=`
//! (and mixed null/undefined/number/bool/string values via tagged slots).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BOOL, PRINT_F64, PRINT_STR};

/// Tag byte for a dynamic slot (stack, not GC).
const TAG_UND: u8 = 0;
const TAG_NULL: u8 = 1;
const TAG_NUM: u8 = 2;
const TAG_BOOL: u8 = 3;
const TAG_STR: u8 = 4;

pub(crate) fn is_es_nullish_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_nullish(module: &Module) -> Result<String, Diagnostic> {
    let user = classify(module).ok_or_else(|| diag("internal: not an es_nullish module"))?;
    let mut em = Emitter::new(module);
    em.emit_module(&user)?;
    Ok(em.finish())
}

/// User locals in declaration order (observation order).
fn classify(module: &Module) -> Option<Vec<LocalId>> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut saw_nullish = false;
    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                if !matches!(
                    loc.ty,
                    Type::Number | Type::Boolean | Type::String | Type::Null | Type::Any
                ) {
                    return None;
                }
                if let Some(init) = init {
                    if !expr_ok(init, &by_id) {
                        return None;
                    }
                    if expr_has_nullish(init) {
                        saw_nullish = true;
                    }
                }
                if seen.insert(*local) {
                    user.push(*local);
                }
            }
            Stmt::Expr { expr } => {
                if !expr_ok(expr, &by_id) {
                    return None;
                }
                if expr_has_nullish(expr) {
                    saw_nullish = true;
                }
            }
            _ => return None,
        }
    }
    if user.is_empty() || !saw_nullish {
        return None;
    }
    Some(user)
}

fn expr_has_nullish(expr: &Expr) -> bool {
    match expr {
        Expr::Binary {
            op: BinaryOp::Nullish,
            ..
        } => true,
        Expr::Binary { left, right, .. } => expr_has_nullish(left) || expr_has_nullish(right),
        Expr::Assign { op, value, .. } => {
            matches!(
                op,
                AssignOp::NullishEq | AssignOp::AndAndEq | AssignOp::OrOrEq
            ) || expr_has_nullish(value)
        }
        Expr::Unary { arg, .. } => expr_has_nullish(arg),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_nullish(test) || expr_has_nullish(consequent) || expr_has_nullish(alternate)
        }
        _ => false,
    }
}

fn expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { .. } | Expr::Boolean { .. } | Expr::String { .. } | Expr::Null { .. } => {
            true
        }
        Expr::Local { id, .. } => by_id.contains_key(id),
        Expr::Unary { op, arg, .. } => {
            matches!(op, UnaryOp::Void | UnaryOp::Plus | UnaryOp::Minus | UnaryOp::Not)
                && expr_ok(arg, by_id)
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            matches!(
                op,
                BinaryOp::Nullish
                    | BinaryOp::And
                    | BinaryOp::Or
                    | BinaryOp::Comma
                    | BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
            ) && expr_ok(left, by_id)
                && expr_ok(right, by_id)
        }
        Expr::Assign {
            target,
            op,
            value,
            ..
        } => {
            matches!(
                op,
                AssignOp::Eq
                    | AssignOp::NullishEq
                    | AssignOp::AndAndEq
                    | AssignOp::OrOrEq
            ) && matches!(target, AssignTarget::Local(id) if by_id.contains_key(id))
                && expr_ok(value, by_id)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => expr_ok(test, by_id) && expr_ok(consequent, by_id) && expr_ok(alternate, by_id),
        _ => false,
    }
}

struct DynVal {
    tag: String,
    num: String,
    str_p: String,
}

struct SlotPtrs {
    tag: String,
    num: String,
    str_p: String,
}

struct Emitter<'a> {
    module: &'a Module,
    slots: HashMap<LocalId, SlotPtrs>,
    str_globals: HashMap<String, String>,
    out: String,
    body: String,
    tmp: u32,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        Self {
            module,
            slots: HashMap::new(),
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

    fn emit_module(&mut self, user: &[LocalId]) -> Result<(), Diagnostic> {
        for id in user {
            let tag = format!("%l{}_tag", id.0);
            let num = format!("%l{}_num", id.0);
            let str_p = format!("%l{}_str", id.0);
            writeln!(self.body, "  {tag} = alloca i8, align 1").ok();
            writeln!(self.body, "  {num} = alloca double, align 8").ok();
            writeln!(self.body, "  {str_p} = alloca ptr, align 8").ok();
            // default undefined
            writeln!(self.body, "  store i8 {TAG_UND}, ptr {tag}").ok();
            writeln!(
                self.body,
                "  store double 0.00000000000000000e+00, ptr {num}"
            )
            .ok();
            writeln!(self.body, "  store ptr null, ptr {str_p}").ok();
            self.slots.insert(
                *id,
                SlotPtrs {
                    tag,
                    num,
                    str_p,
                },
            );
        }

        for stmt in &self.module.body {
            match stmt {
                Stmt::Declare { local, init, .. } => {
                    if let Some(init) = init {
                        let v = self.emit_dyn(init)?;
                        self.store_slot(*local, &v)?;
                    }
                }
                Stmt::Expr { expr } => {
                    let _ = self.emit_dyn(expr)?;
                }
                _ => return Err(diag("internal: unsupported stmt in es_nullish")),
            }
        }

        for id in user {
            let v = self.load_slot(*id)?;
            self.print_dyn(&v)?;
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.01.04.09 nullish/logical-assign via tagged slots)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
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

    fn load_slot(&mut self, id: LocalId) -> Result<DynVal, Diagnostic> {
        let slot = self
            .slots
            .get(&id)
            .ok_or_else(|| diag(format!("internal: missing slot %{}", id.0)))?
            .clone_ptrs();
        let tag = self.fresh();
        let num = self.fresh();
        let str_p = self.fresh();
        writeln!(self.body, "  {tag} = load i8, ptr {}", slot.tag).ok();
        writeln!(self.body, "  {num} = load double, ptr {}", slot.num).ok();
        writeln!(self.body, "  {str_p} = load ptr, ptr {}", slot.str_p).ok();
        Ok(DynVal {
            tag,
            num,
            str_p,
        })
    }

    fn store_slot(&mut self, id: LocalId, v: &DynVal) -> Result<(), Diagnostic> {
        let slot = self
            .slots
            .get(&id)
            .ok_or_else(|| diag(format!("internal: missing slot %{}", id.0)))?
            .clone_ptrs();
        writeln!(self.body, "  store i8 {}, ptr {}", v.tag, slot.tag).ok();
        writeln!(self.body, "  store double {}, ptr {}", v.num, slot.num).ok();
        writeln!(self.body, "  store ptr {}, ptr {}", v.str_p, slot.str_p).ok();
        Ok(())
    }

    fn empty_str_ptr(&mut self) -> Result<String, Diagnostic> {
        self.string_const("")
    }

    fn make_und(&mut self) -> Result<DynVal, Diagnostic> {
        let empty = self.empty_str_ptr()?;
        Ok(DynVal {
            tag: format!("{TAG_UND}"),
            num: "0.00000000000000000e+00".into(),
            str_p: empty,
        })
    }

    fn make_null(&mut self) -> Result<DynVal, Diagnostic> {
        let empty = self.empty_str_ptr()?;
        Ok(DynVal {
            tag: format!("{TAG_NULL}"),
            num: "0.00000000000000000e+00".into(),
            str_p: empty,
        })
    }

    fn make_num(&mut self, num: String) -> Result<DynVal, Diagnostic> {
        let empty = self.empty_str_ptr()?;
        Ok(DynVal {
            tag: format!("{TAG_NUM}"),
            num,
            str_p: empty,
        })
    }

    fn make_bool(&mut self, b: String) -> Result<DynVal, Diagnostic> {
        let empty = self.empty_str_ptr()?;
        let num = self.fresh();
        writeln!(
            self.body,
            "  {num} = select i1 {b}, double 1.00000000000000000e+00, double 0.00000000000000000e+00"
        )
        .ok();
        Ok(DynVal {
            tag: format!("{TAG_BOOL}"),
            num,
            str_p: empty,
        })
    }

    fn make_str(&mut self, p: String) -> DynVal {
        DynVal {
            tag: format!("{TAG_STR}"),
            num: "0.00000000000000000e+00".into(),
            str_p: p,
        }
    }

    fn is_nullish(&mut self, v: &DynVal) -> String {
        let t = self.fresh();
        writeln!(
            self.body,
            "  {t} = icmp ule i8 {}, {TAG_NULL}",
            v.tag
        )
        .ok();
        // und=0, null=1 → ule 1
        t
    }

    fn to_boolean(&mut self, v: &DynVal) -> Result<String, Diagnostic> {
        // undefined/null → false
        // number → != 0 (and not NaN via one)
        // bool → num != 0
        // string → first byte != 0 (empty → false)
        let is_und_or_null = self.fresh();
        writeln!(
            self.body,
            "  {is_und_or_null} = icmp ule i8 {}, {TAG_NULL}",
            v.tag
        )
        .ok();
        let is_str = self.fresh();
        writeln!(self.body, "  {is_str} = icmp eq i8 {}, {TAG_STR}", v.tag).ok();
        let num_truthy = self.fresh();
        writeln!(
            self.body,
            "  {num_truthy} = fcmp one double {}, 0.00000000000000000e+00",
            v.num
        )
        .ok();
        let c0 = self.fresh();
        writeln!(self.body, "  {c0} = load i8, ptr {}", v.str_p).ok();
        let str_truthy = self.fresh();
        writeln!(self.body, "  {str_truthy} = icmp ne i8 {c0}, 0").ok();
        let payload = self.fresh();
        writeln!(
            self.body,
            "  {payload} = select i1 {is_str}, i1 {str_truthy}, i1 {num_truthy}"
        )
        .ok();
        let t = self.fresh();
        writeln!(
            self.body,
            "  {t} = select i1 {is_und_or_null}, i1 false, i1 {payload}"
        )
        .ok();
        Ok(t)
    }

    fn select_dyn(&mut self, cond: &str, a: &DynVal, b: &DynVal) -> DynVal {
        let tag = self.fresh();
        let num = self.fresh();
        let str_p = self.fresh();
        writeln!(
            self.body,
            "  {tag} = select i1 {cond}, i8 {}, i8 {}",
            a.tag, b.tag
        )
        .ok();
        writeln!(
            self.body,
            "  {num} = select i1 {cond}, double {}, double {}",
            a.num, b.num
        )
        .ok();
        writeln!(
            self.body,
            "  {str_p} = select i1 {cond}, ptr {}, ptr {}",
            a.str_p, b.str_p
        )
        .ok();
        DynVal { tag, num, str_p }
    }

    fn emit_dyn(&mut self, expr: &Expr) -> Result<DynVal, Diagnostic> {
        match expr {
            Expr::Null { .. } => self.make_null(),
            Expr::Number { raw, .. } => {
                let n = format_number_const(raw)?;
                self.make_num(n)
            }
            Expr::Boolean { value, .. } => {
                let b = if *value { "true" } else { "false" };
                self.make_bool(b.into())
            }
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let p = self.string_const(&s)?;
                Ok(self.make_str(p))
            }
            Expr::Local { id, .. } => self.load_slot(*id),
            Expr::Unary { op, arg, .. } => match op {
                UnaryOp::Void => {
                    let _ = self.emit_dyn(arg)?;
                    self.make_und()
                }
                UnaryOp::Plus => {
                    let a = self.emit_dyn(arg)?;
                    self.make_num(a.num)
                }
                UnaryOp::Minus => {
                    let a = self.emit_dyn(arg)?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = fneg double {}", a.num).ok();
                    self.make_num(t)
                }
                UnaryOp::Not => {
                    let a = self.emit_dyn(arg)?;
                    let b = self.to_boolean(&a)?;
                    let nb = self.fresh();
                    writeln!(self.body, "  {nb} = xor i1 {b}, true").ok();
                    self.make_bool(nb)
                }
                _ => Err(diag("internal: unsupported unary in es_nullish")),
            },
            Expr::Binary {
                left, op, right, ..
            } => match op {
                BinaryOp::Nullish => {
                    let l = self.emit_dyn(left)?;
                    let r = self.emit_dyn(right)?;
                    let n = self.is_nullish(&l);
                    Ok(self.select_dyn(&n, &r, &l))
                }
                BinaryOp::And => {
                    let l = self.emit_dyn(left)?;
                    let r = self.emit_dyn(right)?;
                    let t = self.to_boolean(&l)?;
                    Ok(self.select_dyn(&t, &r, &l))
                }
                BinaryOp::Or => {
                    let l = self.emit_dyn(left)?;
                    let r = self.emit_dyn(right)?;
                    let t = self.to_boolean(&l)?;
                    Ok(self.select_dyn(&t, &l, &r))
                }
                BinaryOp::Comma => {
                    let _ = self.emit_dyn(left)?;
                    self.emit_dyn(right)
                }
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Rem => {
                    let l = self.emit_dyn(left)?;
                    let r = self.emit_dyn(right)?;
                    let inst = match op {
                        BinaryOp::Add => "fadd",
                        BinaryOp::Sub => "fsub",
                        BinaryOp::Mul => "fmul",
                        BinaryOp::Div => "fdiv",
                        BinaryOp::Rem => "frem",
                        _ => unreachable!(),
                    };
                    let t = self.fresh();
                    writeln!(
                        self.body,
                        "  {t} = {inst} double {}, double {}",
                        l.num, r.num
                    )
                    .ok();
                    self.make_num(t)
                }
                _ => Err(diag("internal: unsupported binary in es_nullish")),
            },
            Expr::Assign {
                target,
                op,
                value,
                ..
            } => {
                let AssignTarget::Local(id) = target else {
                    return Err(diag("internal: only local assign in es_nullish"));
                };
                match op {
                    AssignOp::Eq => {
                        let v = self.emit_dyn(value)?;
                        self.store_slot(*id, &v)?;
                        Ok(v)
                    }
                    AssignOp::NullishEq => {
                        let cur = self.load_slot(*id)?;
                        let rhs = self.emit_dyn(value)?;
                        let n = self.is_nullish(&cur);
                        let v = self.select_dyn(&n, &rhs, &cur);
                        self.store_slot(*id, &v)?;
                        Ok(v)
                    }
                    AssignOp::AndAndEq => {
                        let cur = self.load_slot(*id)?;
                        let rhs = self.emit_dyn(value)?;
                        let t = self.to_boolean(&cur)?;
                        let v = self.select_dyn(&t, &rhs, &cur);
                        self.store_slot(*id, &v)?;
                        Ok(v)
                    }
                    AssignOp::OrOrEq => {
                        let cur = self.load_slot(*id)?;
                        let rhs = self.emit_dyn(value)?;
                        let t = self.to_boolean(&cur)?;
                        let v = self.select_dyn(&t, &cur, &rhs);
                        self.store_slot(*id, &v)?;
                        Ok(v)
                    }
                    _ => Err(diag("internal: unsupported assign op in es_nullish")),
                }
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                let c = self.emit_dyn(test)?;
                let t = self.to_boolean(&c)?;
                let a = self.emit_dyn(consequent)?;
                let b = self.emit_dyn(alternate)?;
                Ok(self.select_dyn(&t, &a, &b))
            }
            _ => Err(diag("internal: unsupported expr in es_nullish")),
        }
    }

    fn print_dyn(&mut self, v: &DynVal) -> Result<(), Diagnostic> {
        // Pre-materialize string constants before the CFG (GEPs must dominate uses).
        let und_s = self.string_const("undefined")?;
        let null_s = self.string_const("null")?;

        let base = self.tmp;
        self.tmp += 1;
        let l_und = format!("p{base}_und");
        let l_null = format!("p{base}_null");
        let l_num = format!("p{base}_num");
        let l_bool = format!("p{base}_bool");
        let l_str = format!("p{base}_str");
        let l_end = format!("p{base}_end");
        let c1 = format!("p{base}_c1");
        let c2 = format!("p{base}_c2");
        let c3 = format!("p{base}_c3");

        let i_und = self.fresh();
        writeln!(self.body, "  {i_und} = icmp eq i8 {}, {TAG_UND}", v.tag).ok();
        writeln!(self.body, "  br i1 {i_und}, label %{l_und}, label %{c1}").ok();

        writeln!(self.body, "{c1}:").ok();
        let i_null = self.fresh();
        writeln!(self.body, "  {i_null} = icmp eq i8 {}, {TAG_NULL}", v.tag).ok();
        writeln!(self.body, "  br i1 {i_null}, label %{l_null}, label %{c2}").ok();

        writeln!(self.body, "{c2}:").ok();
        let i_num = self.fresh();
        writeln!(self.body, "  {i_num} = icmp eq i8 {}, {TAG_NUM}", v.tag).ok();
        writeln!(self.body, "  br i1 {i_num}, label %{l_num}, label %{c3}").ok();

        writeln!(self.body, "{c3}:").ok();
        let i_bool = self.fresh();
        writeln!(self.body, "  {i_bool} = icmp eq i8 {}, {TAG_BOOL}", v.tag).ok();
        writeln!(self.body, "  br i1 {i_bool}, label %{l_bool}, label %{l_str}").ok();

        writeln!(self.body, "{l_und}:").ok();
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {und_s}"))).ok();
        writeln!(self.body, "  br label %{l_end}").ok();

        writeln!(self.body, "{l_null}:").ok();
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {null_s}"))).ok();
        writeln!(self.body, "  br label %{l_end}").ok();

        writeln!(self.body, "{l_num}:").ok();
        writeln!(
            self.body,
            "  {}",
            PRINT_F64.call(&format!("double {}", v.num))
        )
        .ok();
        writeln!(self.body, "  br label %{l_end}").ok();

        writeln!(self.body, "{l_bool}:").ok();
        let b = self.fresh();
        writeln!(
            self.body,
            "  {b} = fcmp one double {}, 0.00000000000000000e+00",
            v.num
        )
        .ok();
        let ext = self.fresh();
        writeln!(self.body, "  {ext} = zext i1 {b} to i8").ok();
        writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {ext}"))).ok();
        writeln!(self.body, "  br label %{l_end}").ok();

        writeln!(self.body, "{l_str}:").ok();
        writeln!(
            self.body,
            "  {}",
            PRINT_STR.call(&format!("ptr {}", v.str_p))
        )
        .ok();
        writeln!(self.body, "  br label %{l_end}").ok();

        writeln!(self.body, "{l_end}:").ok();
        Ok(())
    }

    fn finish(self) -> String {
        self.out
    }
}

impl SlotPtrs {
    fn clone_ptrs(&self) -> Self {
        Self {
            tag: self.tag.clone(),
            num: self.num.clone(),
            str_p: self.str_p.clone(),
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
