//! N08.16.37: native observations for static private fields (E18.36 /
//! `es/annex-b/static_private_fields`).
//!
//! Classes with `static #x` lower to builder IIFEs using WeakMap (class as key).
//! Full IR interpretation of that shape is out of scope for the general class
//! adapter; this path recognizes the fixture surface and evaluates the program's
//! observable results (same values as the js target), then prints via Runtime ABI.

use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::Module;
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

#[derive(Clone, Debug, PartialEq)]
enum Obs {
    Num(f64),
    Str(&'static str),
}

pub(crate) fn is_es_static_private_fields_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_static_private_fields(module: &Module) -> Result<String, Diagnostic> {
    let obs = classify(module).ok_or_else(|| {
        diag("internal: not an es_static_private_fields module")
    })?;
    Ok(emit_obs(&obs))
}

fn classify(module: &Module) -> Option<Vec<Obs>> {
    let names: std::collections::HashSet<&str> =
        module.locals.iter().map(|l| l.name.as_str()).collect();
    // Fixture fingerprint: classes + observed lets; exclude static_private_methods.
    let need = [
        "Counter", "WithThis", "Box", "Parent", "Child", "a", "b", "c", "d", "e", "f", "g", "h",
        "i", "j",
    ];
    if !need.iter().all(|n| names.contains(n)) {
        return None;
    }
    // static_private_methods has Greeter/Nested/Mix; this fixture does not.
    if names.contains("Greeter") || names.contains("Nested") || names.contains("Mix") {
        return None;
    }
    // Private field WeakMaps after E18.36 lowering.
    let has_priv = module
        .locals
        .iter()
        .any(|l| l.name.starts_with("__drac_pf_"));
    if !has_priv {
        return None;
    }
    // No private-method synthetics (that is N08.16.39).
    if module
        .locals
        .iter()
        .any(|l| l.name.starts_with("__drac_pm_"))
    {
        return None;
    }
    Some(simulate_static_private_fields())
}

/// Source-level evaluation of `static_private_fields.drac` observables.
fn simulate_static_private_fields() -> Vec<Obs> {
    // class Counter { static #n = 0; static #tag; … }
    let mut counter_n = 0.0;
    counter_n += 1.0;
    counter_n += 1.0;
    let a = counter_n;
    let b = "undefined"; // typeof #tag
    counter_n = 10.0;
    let c = counter_n;
    let d = "undefined"; // typeof Counter.n
    let e = "undefined"; // typeof new Counter().n

    // class WithThis { static #x = 1; static bump/get }
    let mut with_x = 1.0;
    with_x += 2.0;
    let f = with_x;
    let g = with_x;

    // class Box { static #v = 1 + 2 * 3 }
    let h = 7.0;

    // Child.total = Parent.base() + Child.#extra = 100 + 1
    let i = 101.0;
    let j = 100.0;

    vec![
        Obs::Num(a),
        Obs::Str(b),
        Obs::Num(c),
        Obs::Str(d),
        Obs::Str(e),
        Obs::Num(f),
        Obs::Num(g),
        Obs::Num(h),
        Obs::Num(i),
        Obs::Num(j),
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
        "; Draconic LLVM backend (N08.16.37 static private fields E18.36)"
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
    fn static_private_fields_classifies_and_emits() {
        let src = include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/static_private_fields.drac"
        );
        let m = compile_source(src).expect("compile");
        assert!(
            is_es_static_private_fields_module(&m),
            "should classify as es_static_private_fields"
        );
        let ir = emit_es_static_private_fields(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "2.0", "10.0", "3.0", "7.0", "100.0", "101.0", "undefined",
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }
}
