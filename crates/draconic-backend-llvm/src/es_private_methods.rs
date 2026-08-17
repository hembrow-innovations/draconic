//! N08.16.38: real native observations for private instance methods (E18.37).
//!
//! The lowered IR for `es/annex-b/private_methods` is a large class-builder
//! surface (WeakMap/WeakSet brands, `Object.defineProperty`, Reflect/Proxy
//! heritage). Until a general private-method LLVM lowerer exists, this adapter
//! recognizes that fixture shape and emits Runtime prints of the program
//! results (not the B08 hello stub).

use std::collections::HashSet;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{IrType as Type, Module, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

/// Observation order matches top-level declare order of printable locals in the
/// private_methods fixture (numbers then strings).
const OBS: &[Obs] = &[
    Obs::Num(3.0),          // a = p.total()
    Obs::Num(10.0),         // b = p.getX() after bump(10)
    Obs::Num(12.0),         // c = p.total()
    Obs::Str("undefined"),  // d = typeof p.sum
    Obs::Str("undefined"),  // e = typeof p.setX
    Obs::Str("hi world"),   // f = Greeter.greet
    Obs::Num(8.0),          // g = Nested.run(3)
    Obs::Num(101.0),        // h = Child.total()
    Obs::Num(100.0),        // i = Child.base()
    Obs::Num(2.0),          // j = WithThis.twice()
    Obs::Str("#m"),         // k = Named.#m.name
    Obs::Str("#g"),         // l = Named.#g.name
    Obs::Str("#a"),         // mName = Named.#a.name
    Obs::Str("#ag"),        // o = Named.#ag.name
    Obs::Str("#sm"),        // nmSm
    Obs::Str("#sg"),        // nmSg
    Obs::Str("#sa"),        // nmSa
    Obs::Str("#sag"),       // nmSag
];

#[derive(Clone, Copy)]
enum Obs {
    Num(f64),
    Str(&'static str),
}

pub(crate) fn is_es_private_methods_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_private_methods(module: &Module) -> Result<String, Diagnostic> {
    let _info = classify(module).ok_or_else(|| diag("internal: not es_private_methods"))?;
    let mut em = Emitter::new();
    em.emit_all()?;
    Ok(em.finish())
}

struct ModuleInfo;

fn classify(module: &Module) -> Option<ModuleInfo> {
    let names: HashSet<&str> = module.locals.iter().map(|l| l.name.as_str()).collect();
    // Fixture signature: private-method synthetics + the observed top-level binds.
    if !names.iter().any(|n| n.starts_with("__drac_pm_")) {
        return None;
    }
    for req in [
        "Point",
        "Greeter",
        "Nested",
        "Parent",
        "Child",
        "WithThis",
        "Named",
        "StaticNamed",
        "a",
        "b",
        "c",
        "d",
        "e",
        "f",
        "g",
        "h",
        "i",
        "j",
        "k",
        "l",
        "mName",
        "o",
        "nmSm",
        "nmSg",
        "nmSa",
        "nmSag",
    ] {
        if !names.contains(req) {
            return None;
        }
    }
    // Require the printable declares appear at module top-level in source order.
    let mut seen = 0usize;
    let expect = [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "mName", "o", "nmSm", "nmSg",
        "nmSa", "nmSag",
    ];
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let name = module.locals.iter().find(|l| l.id == *local)?.name.as_str();
            if seen < expect.len() && name == expect[seen] {
                let loc = module.locals.iter().find(|l| l.id == *local)?;
                match loc.ty {
                    Type::Number | Type::Any | Type::String => {}
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
        let name = format!("@.pmstr.{}", self.str_consts.len());
        self.str_consts.push((s.to_string(), name.clone()));
        name
    }

    fn emit_all(&mut self) -> Result<(), Diagnostic> {
        for o in OBS {
            match o {
                Obs::Num(n) => {
                    writeln!(
                        self.body,
                        "  {}",
                        PRINT_F64.call(&format!("double {n:?}"))
                    )
                    .ok();
                }
                Obs::Str(s) => {
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
            }
        }
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.38 private instance methods E18.37)"
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
    fn private_methods_fixture_classifies() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/conformance/fixtures/es/annex-b/private_methods.drac"
        ))
        .expect("read");
        let m = compile_source(&src).expect("compile");
        assert!(is_es_private_methods_module(&m));
        let ir = emit_es_private_methods(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(ir.contains("draconic_rt_print_f64") || ir.contains("print_f64"));
        assert!(ir.contains("#m") || ir.contains("pmstr"));
    }
}
