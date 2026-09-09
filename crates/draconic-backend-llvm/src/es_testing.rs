//! L05 / L05.01 / L05.02 / L05.03: native observations for `describe` / `it` / `expect` + hooks.
//!
//! Compile-time evaluation: `describe` runs its callback; `it` runs its callback
//! and yields `true` on success or `false` if the callback throws. `expect`
//! matchers throw a string message on failure. Nested `describe` plus `before` /
//! `after` / `beforeEach` / `afterEach` share a suite stack. Emits Runtime prints
//! of final top-level number/string/bool locals.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Pattern, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

#[path = "es_testing_eval.rs"]
mod eval;

use eval::eval_body;

pub(crate) fn is_es_testing_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_testing(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_testing module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_testing(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_testing_module(module) {
        return None;
    }
    Some(emit_es_testing(module))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum BuiltinId {
    GlobalThis,
    Describe,
    It,
    Expect,
    Before,
    After,
    BeforeEach,
    AfterEach,
}

#[derive(Clone, Debug, Default)]
struct SuiteHooks {
    before_each: Vec<JsVal>,
    after_each: Vec<JsVal>,
    after: Vec<JsVal>,
}

struct EvalCtx {
    env: HashMap<LocalId, JsVal>,
    suites: Vec<SuiteHooks>,
}

impl EvalCtx {
    fn new() -> Self {
        Self {
            env: HashMap::new(),
            suites: vec![SuiteHooks::default()],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum MatcherKind {
    ToBe,
    ToBeTruthy,
    ToBeFalsy,
}

#[derive(Clone, Debug, PartialEq)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Builtin(BuiltinId),
    Closure {
        body: Vec<Stmt>,
    },
    Matcher {
        actual: Box<JsVal>,
    },
    BoundMatcher {
        kind: MatcherKind,
        actual: Box<JsVal>,
    },
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Return(JsVal),
    Throw(JsVal),
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_testing_surface(module, &by_id) {
        return None;
    }
    if !body_ok(&module.body) {
        return None;
    }
    let mut ctx = EvalCtx::new();
    for loc in &module.locals {
        if loc.name == "globalThis" {
            ctx.env
                .insert(loc.id, JsVal::Builtin(BuiltinId::GlobalThis));
        }
    }
    match eval_body(&module.body, &mut ctx) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }
    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match ctx.env.get(local) {
                Some(v @ (JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_))) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String
                    ) {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(_) => {}
                None => return None,
            }
        }
    }
    if user_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        user_locals,
        values,
    })
}

fn ident_builtin(name: &str) -> Option<BuiltinId> {
    match name {
        "globalThis" => Some(BuiltinId::GlobalThis),
        "describe" => Some(BuiltinId::Describe),
        "it" => Some(BuiltinId::It),
        "expect" => Some(BuiltinId::Expect),
        "before" => Some(BuiltinId::Before),
        "after" => Some(BuiltinId::After),
        "beforeEach" => Some(BuiltinId::BeforeEach),
        "afterEach" => Some(BuiltinId::AfterEach),
        _ => None,
    }
}

fn module_has_testing_surface(module: &Module, _by_id: &HashMap<LocalId, &Local>) -> bool {
    module.body.iter().any(stmt_has_testing_surface)
}

fn stmt_has_testing_surface(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Throw { value: e } => {
            expr_has_testing_surface(e)
        }
        Stmt::Return { value: Some(e) } => expr_has_testing_surface(e),
        Stmt::Block { body } => body.iter().any(stmt_has_testing_surface),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_has_testing_surface(test)
                || stmt_has_testing_surface(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_has_testing_surface(a))
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(stmt_has_testing_surface)
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(stmt_has_testing_surface))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(stmt_has_testing_surface))
        }
        _ => false,
    }
}

fn expr_has_testing_surface(expr: &Expr) -> bool {
    match expr {
        Expr::IdentName { name, .. } => matches!(
            name.as_str(),
            "describe" | "it" | "expect" | "before" | "after" | "beforeEach" | "afterEach"
        ),
        Expr::Unary { arg, .. } => expr_has_testing_surface(arg),
        Expr::Binary { left, right, .. } => {
            expr_has_testing_surface(left) || expr_has_testing_surface(right)
        }
        Expr::Call { callee, args, .. } => {
            expr_has_testing_surface(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_testing_surface(e),
                    _ => false,
                })
        }
        Expr::Member {
            object, property, ..
        } => expr_has_testing_surface(object) || expr_has_testing_surface(property),
        Expr::Assign { value, .. } => expr_has_testing_surface(value),
        Expr::Function { body, .. } => body.iter().any(stmt_has_testing_surface),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_testing_surface(test)
                || expr_has_testing_surface(consequent)
                || expr_has_testing_surface(alternate)
        }
        _ => false,
    }
}

fn body_ok(body: &[Stmt]) -> bool {
    body.iter().all(stmt_ok)
}

fn stmt_ok(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init, .. } => match init {
            None => true,
            Some(e) => expr_ok(e),
        },
        Stmt::Expr { expr } => expr_ok(expr),
        Stmt::Throw { value } => expr_ok(value),
        Stmt::Return { value } => value.as_ref().is_none_or(expr_ok),
        Stmt::Block { body } => body_ok(body),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => expr_ok(test) && stmt_ok(consequent) && alternate.as_ref().is_none_or(|a| stmt_ok(a)),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            body_ok(block)
                && handler.as_ref().is_none_or(|h| body_ok(h))
                && finalizer.as_ref().is_none_or(|f| body_ok(f))
        }
        _ => false,
    }
}

fn expr_ok(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Null { .. }
        | Expr::Local { .. } => true,
        Expr::IdentName { name, .. } => ident_builtin(name).is_some(),
        Expr::Unary {
            op: UnaryOp::TypeOf | UnaryOp::Minus | UnaryOp::Plus,
            arg,
            ..
        } => expr_ok(arg),
        Expr::Binary {
            op:
                BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div,
            left,
            right,
            ..
        } => expr_ok(left) && expr_ok(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => expr_ok(test) && expr_ok(consequent) && expr_ok(alternate),
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => expr_ok(object) && expr_ok(property),
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            expr_ok(callee)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_ok(value),
        Expr::Function {
            is_async: false,
            is_generator: false,
            body,
            ..
        } => body_ok(body),
        _ => false,
    }
}

struct Emitter {
    out: String,
    body: String,
    str_consts: Vec<(String, String)>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
            str_consts: Vec::new(),
        }
    }

    fn string_const(&mut self, s: &str) -> String {
        if let Some((_, name)) = self.str_consts.iter().find(|(v, _)| v == s) {
            return name.clone();
        }
        let name = format!("@.gstr.{}", self.str_consts.len());
        self.str_consts.push((s.to_string(), name.clone()));
        name
    }

    fn emit_num(&mut self, n: f64) {
        let lit = format!("{n:?}");
        writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_testing: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => {
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                JsVal::Bool(b) => {
                    let s = if *b { "true" } else { "false" };
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                _ => return Err(diag("es_testing: non-printable value")),
            }
        }
        writeln!(
            self.out,
            "; Draconic LLVM backend (L05.03 describe/it/expect/hooks)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        for (s, name) in &self.str_consts {
            let n = s.len() + 1;
            let mut esc = String::new();
            for b in s.bytes() {
                match b {
                    b'\\' => esc.push_str("\\5C"),
                    b'"' => esc.push_str("\\22"),
                    c if (0x20..0x7f).contains(&c) => esc.push(c as char),
                    c => esc.push_str(&format!("\\{c:02X}")),
                }
            }
            writeln!(
                self.out,
                "{name} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        writeln!(self.out, "\ndefine i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        self.out.push_str(&self.body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn finish(self) -> String {
        self.out
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn compile_src(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn classifies_describe_it_run() {
        let m = compile_src(
            r#"
            let ran = 0;
            let passed;
            describe("suite", () => {
              passed = it("case", () => {
                ran = 1;
              });
            });
            "#,
        );
        assert!(is_es_testing_module(&m));
        let ir = emit_es_testing(&m).expect("emit");
        assert!(ir.contains("define i32 @main()"), "{ir}");
        assert!(ir.contains("1"), "{ir}");
    }

    #[test]
    fn classifies_expect_matchers() {
        let m = compile_src(
            r#"
            let te = typeof expect;
            let ok;
            describe("e", () => {
              ok = it("eq", () => {
                expect(1).toBe(1);
                expect(1).toBeTruthy();
                expect(0).toBeFalsy();
              });
            });
            "#,
        );
        assert!(is_es_testing_module(&m));
        let ir = emit_es_testing(&m).expect("emit");
        assert!(ir.contains("function"), "{ir}");
        assert!(ir.contains("true"), "{ir}");
    }

    #[test]
    fn classifies_expect_fail_messages() {
        let m = compile_src(
            r#"
            let eqHas = false;
            try {
              expect(1).toBe(2);
            } catch (e) {
              eqHas = e === "expected 1 to be 2";
            }
            "#,
        );
        assert!(is_es_testing_module(&m));
        let ir = emit_es_testing(&m).expect("emit");
        assert!(ir.contains("true"), "{ir}");
    }

    #[test]
    fn classifies_nested_hooks() {
        let m = compile_src(
            r#"
            let nestedOk;
            let hookOk;
            let order = "";
            describe("outer", () => {
              before(() => { order = order + "B"; });
              after(() => { order = order + "A"; });
              beforeEach(() => { order = order + "b"; });
              afterEach(() => { order = order + "a"; });
              describe("inner", () => {
                beforeEach(() => { order = order + "i"; });
                afterEach(() => { order = order + "j"; });
                nestedOk = it("case", () => {
                  order = order + "T";
                });
              });
            });
            hookOk = order === "BbiTjaA";
            "#,
        );
        assert!(is_es_testing_module(&m));
        let ir = emit_es_testing(&m).expect("emit");
        assert!(ir.contains("true"), "{ir}");
        assert!(ir.contains("BbiTjaA"), "{ir}");
    }
}
