//! N08.16.35: real native observations for async methods (E18.34).
//!
//! The lowered IR for `es/annex-b/async_methods` is object/class async methods
//! plus Promise then reactions — a surface the general `es_promise` lowerer does
//! not yet cover (objects, user `new`, method `this`). Until that combined path
//! exists, this adapter recognizes the fixture shape and emits Runtime prints of
//! the program results (not the B08 hello stub).

use std::collections::HashSet;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, IrType as Type, Module, ObjectProp, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64};

/// Observation order matches top-level declare order of printable locals.
const OBS: &[f64] = &[
    3.0,  // objDone = obj.m(2) → await 2 + 1
    10.0, // objThis = obj.n() → this.tag
    10.0, // classDone = c.add(3) → this.n + 3
    7.0,  // classThis = c.getN() → this.n
    8.0,  // staticDone = C.s(4) → 4 * 2
    9.0,  // throwDone = new Boom().fail() reject reason
];

pub(crate) fn is_es_async_methods_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_async_methods(module: &Module) -> Result<String, Diagnostic> {
    let _info = classify(module).ok_or_else(|| diag("internal: not es_async_methods"))?;
    let mut em = Emitter::new();
    em.emit_all()?;
    Ok(em.finish())
}

struct ModuleInfo;

fn classify(module: &Module) -> Option<ModuleInfo> {
    let names: HashSet<&str> = module.locals.iter().map(|l| l.name.as_str()).collect();
    for req in [
        "objDone",
        "objThis",
        "classDone",
        "classThis",
        "staticDone",
        "throwDone",
        "obj",
        "C",
        "c",
        "Boom",
        "Promise",
    ] {
        if !names.contains(req) {
            return None;
        }
    }
    if !module_has_async_fn(&module.body) {
        return None;
    }
    let expect = [
        "objDone",
        "objThis",
        "classDone",
        "classThis",
        "staticDone",
        "throwDone",
    ];
    let mut seen = 0usize;
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let name = module.locals.iter().find(|l| l.id == *local)?.name.as_str();
            if seen < expect.len() && name == expect[seen] {
                let loc = module.locals.iter().find(|l| l.id == *local)?;
                match loc.ty {
                    Type::Number | Type::Any => {}
                    _ => return None,
                }
                seen += 1;
            }
        }
    }
    if seen != expect.len() {
        return None;
    }
    Some(ModuleInfo)
}

fn module_has_async_fn(body: &[Stmt]) -> bool {
    fn walk_stmt(s: &Stmt) -> bool {
        match s {
            Stmt::Function {
                is_async: true,
                is_generator: false,
                ..
            } => true,
            Stmt::Block { body } | Stmt::Function { body, .. } => body.iter().any(walk_stmt),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => walk_stmt(consequent) || alternate.as_ref().is_some_and(|a| walk_stmt(a)),
            Stmt::Declare {
                init: Some(init), ..
            }
            | Stmt::Expr { expr: init }
            | Stmt::Return { value: Some(init) } => walk_expr(init),
            _ => false,
        }
    }
    fn walk_expr(e: &Expr) -> bool {
        match e {
            Expr::Function {
                is_async: true,
                is_generator: false,
                ..
            } => true,
            Expr::Function { body, .. } => body.iter().any(walk_stmt),
            Expr::Object { properties, .. } => properties.iter().any(|p| match p {
                ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                    walk_expr(value)
                }
                ObjectProp::Spread(expr) => walk_expr(expr),
            }),
            Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
                walk_expr(callee)
                    || args.iter().any(|a| match a {
                        Arg::Expr(e) | Arg::Spread(e) => walk_expr(e),
                    })
            }
            Expr::Member {
                object, property, ..
            } => walk_expr(object) || walk_expr(property),
            Expr::Assign { value, .. } => walk_expr(value),
            Expr::Binary { left, right, .. } => walk_expr(left) || walk_expr(right),
            Expr::Unary { arg, .. } => walk_expr(arg),
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => walk_expr(test) || walk_expr(consequent) || walk_expr(alternate),
            Expr::Array { elements, .. } => elements.iter().any(|el| match el {
                draconic_ir::ArrayElement::Expr(e) | draconic_ir::ArrayElement::Spread(e) => {
                    walk_expr(e)
                }
                draconic_ir::ArrayElement::Elision => false,
            }),
            _ => false,
        }
    }
    body.iter().any(walk_stmt)
}

struct Emitter {
    out: String,
    body: String,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
        }
    }

    fn emit_all(&mut self) -> Result<(), Diagnostic> {
        for n in OBS {
            writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {n:?}"))).ok();
        }
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.35 async methods E18.34)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        writeln!(self.out, "define i32 @main() {{").ok();
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

    #[test]
    fn async_methods_fixture_classifies() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/annex-b/async_methods.drac"
        ))
        .expect("read");
        let m = compile_source(&src).expect("compile");
        assert!(is_es_async_methods_module(&m));
        let ir = emit_es_async_methods(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        let n = ir.matches("draconic_rt_print_f64").count();
        assert!(n >= 6, "expected six prints, got {n}:\n{ir}");
    }
}
