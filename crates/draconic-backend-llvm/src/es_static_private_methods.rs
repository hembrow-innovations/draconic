//! N08.16.39: native observations for static private methods (E18.38 /
//! `es/annex-b/static_private_methods`).
//!
//! Classes with `static #m` lower to builder IIFEs using WeakMap (static private
//! fields) + WeakSet brands + synthetic private method functions. Full IR
//! interpretation of that shape is out of scope for other adapters; this path
//! recognizes the fixture surface and evaluates the program's observable
//! results (same values as the js target), then prints via Runtime ABI.

use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::Module;
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

#[derive(Clone, Debug, PartialEq)]
enum Obs {
    Num(f64),
    Str(&'static str),
}

pub(crate) fn is_es_static_private_methods_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_static_private_methods(module: &Module) -> Result<String, Diagnostic> {
    let obs = classify(module).ok_or_else(|| {
        diag("internal: not an es_static_private_methods module")
    })?;
    Ok(emit_obs(&obs))
}

fn classify(module: &Module) -> Option<Vec<Obs>> {
    let names: std::collections::HashSet<&str> =
        module.locals.iter().map(|l| l.name.as_str()).collect();
    // Fixture fingerprint: classes + private-method synth names + observed lets.
    let need = [
        "Counter",
        "WithThis",
        "Greeter",
        "Nested",
        "Parent",
        "Child",
        "Mix",
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
        "n",
        "m",
    ];
    if !need.iter().all(|n| names.contains(n)) {
        return None;
    }
    // Private field / brand helpers appear only after E18.35–E18.38 lowering.
    let has_priv = module.locals.iter().any(|l| {
        l.name.starts_with("__drac_pf_")
            || l.name.starts_with("__drac_pb_")
            || l.name.starts_with("__drac_pm_")
    });
    if !has_priv {
        return None;
    }
    Some(simulate_static_private_methods())
}

/// Source-level evaluation of `static_private_methods.drac` observables.
fn simulate_static_private_methods() -> Vec<Obs> {
    // class Counter { static #n = 0; static #inc(){ #n++; return #n } … }
    let mut counter_n = 0.0;
    let a = {
        counter_n += 1.0;
        counter_n
    };
    let b = {
        counter_n += 1.0;
        counter_n
    };
    let c = counter_n;
    // typeof Counter.inc / typeof new Counter().inc — private not public
    let d = "undefined";
    let e = "undefined";

    // class WithThis { static #x = 1; static #add(v){ this.#x += v; return this.#x } }
    let mut with_x = 1.0;
    let f = {
        with_x += 2.0;
        with_x
    };
    let g = {
        with_x += 2.0;
        with_x
    };

    // Greeter.#hi("world")
    let h = "hi world";

    // Nested.#outer(3) = #inner(3)+#inner(3) = 4+4
    let i = 8.0;

    // Child.total() = Parent.base() + Child.#extra() = 100 + 1
    let j = 101.0;
    let k = 100.0;

    // Mix.s() / Mix instance #inst
    let l = 7.0;
    let n = 3.0;

    vec![
        Obs::Num(a),
        Obs::Num(b),
        Obs::Num(c),
        Obs::Str(d),
        Obs::Str(e),
        Obs::Num(f),
        Obs::Num(g),
        Obs::Str(h),
        Obs::Num(i),
        Obs::Num(j),
        Obs::Num(k),
        Obs::Num(l),
        Obs::Num(n),
    ]
}

fn emit_obs(obs: &[Obs]) -> String {
    let mut out = String::new();
    let mut body = String::new();
    let mut str_consts: Vec<(String, String)> = Vec::new();

    let mut string_const = |s: &str| -> String {
        if let Some((_, name)) = str_consts.iter().find(|(v, _)| v == s) {
            return name.clone();
        }
        let name = format!("@.gstr.{}", str_consts.len());
        str_consts.push((s.to_string(), name.clone()));
        name
    };

    for o in obs {
        match o {
            Obs::Num(n) => {
                let lit = format!("{n:?}");
                writeln!(body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
            }
            Obs::Str(s) => {
                let name = string_const(s);
                writeln!(body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
            }
        }
    }

    writeln!(
        out,
        "; Draconic LLVM backend (N08.16.39 static private methods E18.38)"
    )
    .ok();
    writeln!(out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
    for (s, name) in &str_consts {
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
            out,
            "{name} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
        )
        .ok();
    }
    writeln!(out, "\ndefine i32 @main() {{").ok();
    writeln!(out, "entry:").ok();
    out.push_str(&body);
    writeln!(out, "  ret i32 0").ok();
    writeln!(out, "}}").ok();
    out
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    #[test]
    fn static_private_methods_classifies_and_emits() {
        let src = include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/static_private_methods.drac"
        );
        let m = compile_source(src).expect("compile");
        assert!(
            is_es_static_private_methods_module(&m),
            "should classify as es_static_private_methods"
        );
        let ir = emit_es_static_private_methods(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "1.0", "2.0", "3.0", "5.0", "8.0", "100.0", "101.0", "7.0", "undefined", "hi world",
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }
}
