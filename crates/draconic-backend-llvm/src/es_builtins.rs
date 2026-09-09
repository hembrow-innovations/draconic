//! N08.14.01–N08.14.10 + N08.16.01–N08.16.07 + N08.16.16 + N08.16.18 + N08.16.46: native
//! observations for global builtins + Error ctors + functions + URI + JSON + Date + RegExp +
//! Map/Set + WeakMap/WeakSet + ArrayBuffer/DataView/TypedArrays + Annex B `escape`/`unescape` +
//! `Object.prototype.__proto__` + `String.prototype` `substr` / HTML wrappers + `Date.prototype`
//! `getYear`/`setYear`/`toGMTString` + `Object.prototype` `__defineGetter__`/`__defineSetter__`/
//! `__lookupGetter__`/`__lookupSetter__` + RegExp constructor Annex B statics (`$1`–`$9`,
//! `input`/`$_`, `lastMatch`/`$&`, …) + regexp literals + private residual (nested-class private
//! access, compound/logical assign + update on private fields via WeakMap desugar).
//!
//! Compile-time evaluation of:
//! - E15.01: `undefined`, `globalThis`, `Object`/`Function`/`Array`/`String`/`Boolean`
//! - E15.02: `Error` / `TypeError` / `RangeError` / `ReferenceError` / `SyntaxError` /
//!   `URIError` / `EvalError` / `AggregateError` (`typeof`, `globalThis` identity,
//!   `new …(msg)`, `.name`/`.message`/`.errors.length`, throw+catch)
//! - E15.03: `parseInt` / `parseFloat` / `isNaN` / `isFinite` (`typeof`, `globalThis`
//!   identity, basic call behavior; `NaN` / `Infinity` globals)
//! - E15.04: `encodeURI` / `decodeURI` / `encodeURIComponent` / `decodeURIComponent`
//! - E15.05: `JSON` / `JSON.parse` / `JSON.stringify` (primitives, objects, arrays)
//! - E15.06: `Date` / `Date.now` / `Date.UTC` / `new Date(ms)` / `.getTime()` / `.valueOf()`
//! - E15.07: `RegExp` / `new RegExp(pattern[, flags])` / call without `new` / `.source` /
//!   `.flags` / `.test` / `.exec` (fixture subset: literals + `c+` + capturing groups + `i`)
//! - E15.08: `Map` / `Set` — `new Map`/`new Set`, `.set`/`.get`/`.has`/`.size`,
//!   `.add`/`.has`/`.size` (fixture subset; SameValueZero keys for num/str)
//! - E15.09: `WeakMap` / `WeakSet` — `new WeakMap`/`new WeakSet`, `.set`/`.get`/`.has`/
//!   `.delete`, `.add`/`.has`/`.delete` (object keys only; identity equality)
//! - E15.10: `ArrayBuffer` / `DataView` / `Uint8Array` / `Int32Array` / `Float64Array`
//!   (`new`, `.byteLength`/`.length`, index get/set, `getUint8`/`setUint8`; shared buffer)
//! - E18.01: `escape` / `unescape` (`typeof`, `globalThis` identity, basic call behavior)
//! - E18.02: `Object.prototype.__proto__` get/set; object-literal `__proto__` vs computed
//!   `["__proto__"]`; `Object.getPrototypeOf`; `hasOwnProperty.call`
//! - E18.03: `String.prototype.substr` + HTML wrappers (`anchor`/`big`/…/`sup`);
//!   `typeof` method; `String.prototype.substr.call`
//! - E18.04: `Date.prototype.getYear` / `setYear` / `toGMTString` (+ `getFullYear` for
//!   fixture); `typeof` on `Date.prototype.*`; `.call` this-binding
//! - E18.07: `Object.prototype.__defineGetter__` / `__defineSetter__` /
//!   `__lookupGetter__` / `__lookupSetter__` (install/lookup accessors; `.call` this-binding;
//!   simple function expressions as getter/setter)
//! - E18.16: RegExp constructor Annex B statics (B.2.5): `$1`–`$9`, `input`/`$_`,
//!   `lastMatch`/`$&`, `lastParen`/`$+`, `leftContext`/`$\``, `rightContext`/`$'`
//!   after successful `exec`/`test` (unchanged on failure)
//! - E18.18: regexp literals `/pattern/` / `/pattern/flags`; `typeof` `"object"`;
//!   `.source`/`.flags`/`.test`/`.exec` parity with `new RegExp` (subset: `c+`, `\/`,
//!   simple `[…]` classes, `i`)
//! - E18.22: accessor properties — object/class `get`/`set` (incl. computed keys, static),
//!   read/write via property access; class lowering via `Object.defineProperty` /
//!   `getOwnPropertyDescriptor`
//!
//! Emits Runtime prints of final top-level number/string/bool/null locals.

use std::cell::{Cell, RefCell};
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::Module;
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

#[path = "es_builtins_values.rs"]
mod values;
use values::*;

#[path = "es_builtins_regexp.rs"]
mod regexp;
use regexp::*;

#[path = "es_builtins_date.rs"]
mod date;
use date::*;

#[path = "es_builtins_string.rs"]
mod string;
use string::*;

#[path = "es_builtins_json.rs"]
mod json;
use json::*;

#[path = "es_builtins_classify.rs"]
mod classify;
use classify::*;

#[path = "es_builtins_call.rs"]
mod call;
#[path = "es_builtins_eval.rs"]
mod eval;
#[path = "es_builtins_member.rs"]
mod member;

struct Interp {
    regexp: RefCell<RegExpStatics>,
    next: Cell<u64>,
}

impl Interp {
    fn new() -> Self {
        Self {
            regexp: RefCell::new(RegExpStatics::default()),
            next: Cell::new(1),
        }
    }

    fn alloc_id(&self) -> u64 {
        let id = self.next.get();
        self.next.set(id + 1);
        id
    }
}

pub(crate) fn is_es_builtins_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_builtins(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_builtins module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_builtins(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_builtins_module(module) {
        return None;
    }
    Some(emit_es_builtins(module))
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
        let lit = if n.is_nan() {
            "0x7FF8000000000000".to_string()
        } else if n.is_infinite() {
            if n.is_sign_negative() {
                "0xFFF0000000000000".into()
            } else {
                "0x7FF0000000000000".into()
            }
        } else {
            format!("{n:?}")
        };
        writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_builtins: missing value"))?;
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
                JsVal::Null => {
                    let name = self.string_const("null");
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                _ => return Err(diag("es_builtins: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.14.01–N08.14.10 + N08.16.01–N08.16.07 + N08.16.16 + N08.16.18 + N08.16.22 global builtins / Error ctors / functions / URI / JSON / Date / RegExp / Map/Set / WeakMap/WeakSet / ArrayBuffer/DataView/TypedArrays / escape/unescape / Object.prototype.__proto__ / String.prototype substr+HTML / Date.prototype getYear/setYear/toGMTString / RegExp.prototype.compile / String.prototype trimLeft/trimRight / Object.prototype defineGetter/defineSetter/lookupGetter/lookupSetter / RegExp Annex B statics / regexp literals / accessor properties)"
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

    fn compile(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn global_basics_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/builtins/global_basics.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("undefined") && ir.contains("object") && ir.contains("function"),
            "should print typeof observations:\n{ir}"
        );
        assert!(
            ir.contains("true"),
            "should print boolean identity observations:\n{ir}"
        );
    }

    #[test]
    fn error_ctors_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/builtins/error_ctors.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "function",
            "true",
            "Error",
            "msg",
            "TypeError",
            "RangeError",
            "ReferenceError",
            "SyntaxError",
            "URIError",
            "EvalError",
            "AggregateError",
            "a",
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
        // thr final value 1 and agl 2 as f64 prints
        assert!(
            ir.contains("double 1") || ir.contains("double 1.0"),
            "should print thr=1:\n{ir}"
        );
        assert!(
            ir.contains("double 2") || ir.contains("double 2.0"),
            "should print agl=2:\n{ir}"
        );
    }

    #[test]
    fn global_functions_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/builtins/global_functions.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["function", "true", "false"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
        assert!(
            ir.contains("double 42") || ir.contains("double 42.0"),
            "should print parseInt 42:\n{ir}"
        );
        assert!(
            ir.contains("double 16") || ir.contains("double 16.0"),
            "should print parseInt hex 16:\n{ir}"
        );
        assert!(
            ir.contains("double 3.14") || ir.contains("3.14"),
            "should print parseFloat 3.14:\n{ir}"
        );
        assert!(
            ir.contains("double 100") || ir.contains("double 100.0"),
            "should print parseFloat 1e2 → 100:\n{ir}"
        );
    }

    #[test]
    fn uri_functions_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/builtins/uri_functions.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "function",
            "true",
            "https://example.com/a%20b",
            "https://example.com/a b",
            "a%20b%26c%3Dd",
            "a b&c=d",
            "caf%C3%A9",
            "x/y?z=1",
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }

    #[test]
    fn json_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/builtins/json.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "object",
            "function",
            "true",
            "null",
            "hi",
            "two",
            "\\22hi\\22",
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
        assert!(
            ir.contains("double 1") || ir.contains("double 1.0"),
            "should print numeric observations:\n{ir}"
        );
        assert!(
            ir.contains("double 2") || ir.contains("double 2.0"),
            "should print ox/a1=2:\n{ir}"
        );
    }

    #[test]
    fn date_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/builtins/date.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["function", "true", "number"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
        assert!(
            ir.contains("double 0") || ir.contains("double 0.0"),
            "should print getTime/valueOf/UTC zeros:\n{ir}"
        );
    }

    #[test]
    fn regexp_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/builtins/regexp.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["function", "true", "false", "a+b", "foo", "i", "FOO", "bar"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }

    #[test]
    fn regexp_literal_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/annex-b/regexp_literal.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "a+b",
            "true",
            "false",
            "foo",
            "i",
            "FOO",
            "[a/]",
            "object",
            // src3 = a\/b → LLVM c"a\5C/b\00"
            r#"c"a\5C/b\00""#,
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }

    #[test]
    fn map_set_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/builtins/map_set.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["function", "true", "false", "two"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
        assert!(
            ir.contains("double 1") || ir.contains("double 1.0"),
            "should print mGet=1 / sizes:\n{ir}"
        );
        assert!(
            ir.contains("double 2") || ir.contains("double 2.0"),
            "should print mSize2/sSize3=2:\n{ir}"
        );
    }

    #[test]
    fn weak_map_set_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/builtins/weak_map_set.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["function", "true", "false", "two"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
        assert!(
            ir.contains("double 1") || ir.contains("double 1.0"),
            "should print wmGet=1:\n{ir}"
        );
    }

    #[test]
    fn arraybuffer_typedarrays_classifies_and_emits() {
        let src = include_str!(
            "../../../tests/conformance/fixtures/es/builtins/arraybuffer_typedarrays.drac"
        );
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["function", "true"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
        assert!(
            ir.contains("double 8") || ir.contains("double 8.0"),
            "should print blen/u8len=8:\n{ir}"
        );
        assert!(
            ir.contains("double 42") || ir.contains("double 42.0"),
            "should print i32_0=42:\n{ir}"
        );
        assert!(
            ir.contains("double -7") || ir.contains("double -7.0"),
            "should print i32_1=-7:\n{ir}"
        );
        assert!(ir.contains("double 1.5"), "should print f64_0=1.5:\n{ir}");
        assert!(ir.contains("double 2.25"), "should print f64_1=2.25:\n{ir}");
    }

    #[test]
    fn escape_unescape_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/annex-b/escape_unescape.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["function", "true", "a%20b", " ", "caf%E9", "hello world"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }

    #[test]
    fn object_proto_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/annex-b/object_proto.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        // a,d,g,i,j,k true; h false; b,e = 9; c = 1; f = 2
        assert!(ir.contains("true"), "missing true:\n{ir}");
        assert!(ir.contains("false"), "missing false:\n{ir}");
        assert!(
            ir.contains("double 9") || ir.contains("double 9.0"),
            "missing 9:\n{ir}"
        );
        assert!(
            ir.contains("double 1") || ir.contains("double 1.0"),
            "missing 1:\n{ir}"
        );
        assert!(
            ir.contains("double 2") || ir.contains("double 2.0"),
            "missing 2:\n{ir}"
        );
    }

    #[test]
    fn string_proto_annex_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/annex-b/string_proto_annex.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "function",
            "hello",
            "ell",
            "ello",
            "lo",
            "af",
            "he",
            // Quotes in HTML attrs are LLVM-escaped as \\22
            r#"<a name=\22n\22>x</a>"#,
            "<big>x</big>",
            "<blink>x</blink>",
            "<b>x</b>",
            "<tt>x</tt>",
            r#"<font color=\22red\22>x</font>"#,
            r#"<font size=\223\22>x</font>"#,
            "<i>x</i>",
            r#"<a href=\22u\22>x</a>"#,
            "<small>x</small>",
            "<strike>x</strike>",
            "<sub>x</sub>",
            "<sup>x</sup>",
            // via = "b"
            r#"c"b\00""#,
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }

    #[test]
    fn date_proto_annex_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/annex-b/date_proto_annex.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "function",
            "Thu, 01 Jan 1970 00:00:00 GMT",
            "double 70",
            "double 1970",
            "double 1999",
            "double 2000",
            "double -1",
        ] {
            assert!(
                ir.contains(s)
                    || (s.starts_with("double ")
                        && ir.contains(&s["double ".len()..])
                        && ir.contains("double")),
                "missing {s:?} in emit:\n{ir}"
            );
        }
    }

    #[test]
    fn regexp_compile_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/annex-b/regexp_compile.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "function", "a+", "b+", "true", "false", "bar", "w", "y+", "i", "g",
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }

    #[test]
    fn regexp_statics_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/annex-b/regexp_statics.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "abcd",
            "xyabcdz",
            "false",
            "xx",
            "yy",
            "xxyy",
            "c\"b\\00\"",
            "c\"c\\00\"",
            "c\"xy\\00\"",
            "c\"z\\00\"",
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
        assert!(
            ir.contains("c\"\\00\""),
            "should emit empty string for $3:\n{ir}"
        );
    }

    #[test]
    fn string_trim_left_right_classifies_and_emits() {
        let src = include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/string_trim_left_right.drac"
        );
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "function",
            "true",
            "hi  ",
            "  hi",
            "nospace",
            // viaL/viaR
            r#"c"z\00""#,
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }

    #[test]
    fn accessors_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/annex-b/accessors.drac");
        let m = draconic_frontend::compile_source(src).expect("compile");
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(ir.contains("draconic_rt_print"), "emits prints");
        // Observed: a=0 b=10 c=11 d=7 e=3 f=1 g=9 h=C i=D
        for needle in [
            "double 0.0",
            "double 10.0",
            "double 11.0",
            "double 7.0",
            "double 3.0",
            "double 1.0",
            "double 9.0",
        ] {
            assert!(ir.contains(needle), "missing {needle} in {ir}");
        }
        assert!(
            ir.contains("C") && ir.contains("D"),
            "missing string tags in {ir}"
        );
    }

    #[test]
    fn object_accessor_legacy_classifies_and_emits() {
        let src = include_str!(
            "../../../tests/conformance/fixtures/es/annex-b/object_accessor_legacy.drac"
        );
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["function", "42", "7", "9", "undefined", "ok"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }

    #[test]
    fn parse_flags_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/stdlib/flags/parse_long_short.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(!ir.contains("draconic_rt_hello"), "hello stub");
        for s in [
            "function", "true", "file.txt", "alice", "out", "in.txt", "--still",
        ] {
            assert!(ir.contains(s), "missing {s:?}");
        }
    }

    #[test]
    fn typed_options_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/stdlib/flags/typed_options.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(!ir.contains("draconic_rt_hello"), "hello stub");
        for s in [
            "function", "boolean", "number", "string", "true", "alice", "file.txt", "bob",
        ] {
            assert!(ir.contains(s), "missing {s:?}");
        }
    }

    #[test]
    fn flags_surface_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/stdlib/flags/surface.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(ir.contains("true"), "missing printed true:\n{ir}");
    }

    #[test]
    fn parse_url_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/stdlib/url/parse_basics.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(!ir.contains("draconic_rt_hello"), "hello stub");
        for s in [
            "function",
            "true",
            "https",
            "example.com",
            "/path",
            "q=1",
            "frag",
            "http",
            "localhost:8080",
            "user:pass@example.com:443",
            "/a/b",
            "x=1&y=2",
            "top",
        ] {
            assert!(ir.contains(s), "missing {s:?}");
        }
    }

    #[test]
    fn query_roundtrip_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/stdlib/url/query_roundtrip.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(!ir.contains("draconic_rt_hello"), "hello stub");
        for s in [
            "function",
            "true",
            "1",
            "2",
            "hello world",
            "x=1&y=2",
            "q=hello%20world",
        ] {
            assert!(ir.contains(s), "missing {s:?}");
        }
    }

    #[test]
    fn private_residual_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/annex-b/private_residual.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["test262", "42", "3", "7", "9", "4", "5"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }
}
