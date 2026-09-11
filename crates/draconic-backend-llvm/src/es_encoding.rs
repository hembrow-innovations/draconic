//! L01 / L01.01 / L01.02 / L01.03 / L03.01 / L03.02 / L04 / L10.01 / L10.02: native
//! observations for UTF-8 TextEncoder / TextDecoder, Uint8Array Base64
//! (`toBase64` / `fromBase64`), hex (`toHex` / `fromHex`), SHA-256 (`sha256`),
//! `randomBytes`, HMAC-SHA256 (`hmacSha256`), AES-256-GCM AEAD (`aeadEncrypt` /
//! `aeadDecrypt`), and gzip/deflate.
//!
//! L01 parent: one Program combining UTF-8 bytes↔string, Base64, and hex,
//! with invalid input as catchable errors rather than silent corruption.
//! Compile-time evaluation of TextEncoder/TextDecoder encode/decode plus
//! fatal invalid UTF-8 TypeError. Emits Runtime prints of final top-level
//! number/string/bool locals.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

use crate::aead;
use crate::base64;
use crate::compression;
use crate::hex;
use crate::hmac;
use crate::sha256;
mod eval;

use eval::eval_body;

pub(crate) fn is_es_encoding_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_encoding(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_encoding module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_encoding(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_encoding_module(module) {
        return None;
    }
    Some(emit_es_encoding(module))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum BuiltinId {
    GlobalThis,
    TextEncoder,
    TextDecoder,
    Uint8Array,
    TypeError,
    Sha256,
    RandomBytes,
    HmacSha256,
    AeadEncrypt,
    AeadDecrypt,
    Gzip,
    Gunzip,
    Deflate,
    Inflate,
}

#[derive(Clone, Debug, PartialEq)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Builtin(BuiltinId),
    ErrorInst { name: String, message: String },
    TextEncoderInst,
    TextDecoderInst { fatal: bool },
    Uint8ArrayInst { bytes: Rc<RefCell<Vec<u8>>> },
    Object { props: Vec<(String, JsVal)> },
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Throw(JsVal),
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_encoding_surface(module, &by_id) {
        return None;
    }
    if !body_ok(&module.body) {
        return None;
    }
    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    for loc in &module.locals {
        if let Some(b) = builtin_for_name(&loc.name) {
            env.insert(loc.id, JsVal::Builtin(b));
        }
    }
    match eval_body(&module.body, &mut env) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }
    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match env.get(local) {
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

fn builtin_for_name(name: &str) -> Option<BuiltinId> {
    match name {
        "globalThis" => Some(BuiltinId::GlobalThis),
        "TextEncoder" => Some(BuiltinId::TextEncoder),
        "TextDecoder" => Some(BuiltinId::TextDecoder),
        "Uint8Array" => Some(BuiltinId::Uint8Array),
        "TypeError" => Some(BuiltinId::TypeError),
        "sha256" => Some(BuiltinId::Sha256),
        "randomBytes" => Some(BuiltinId::RandomBytes),
        "hmacSha256" => Some(BuiltinId::HmacSha256),
        "aeadEncrypt" => Some(BuiltinId::AeadEncrypt),
        "aeadDecrypt" => Some(BuiltinId::AeadDecrypt),
        "gzip" => Some(BuiltinId::Gzip),
        "gunzip" => Some(BuiltinId::Gunzip),
        "deflate" => Some(BuiltinId::Deflate),
        "inflate" => Some(BuiltinId::Inflate),
        _ => None,
    }
}

fn module_has_encoding_surface(module: &Module, by_id: &HashMap<LocalId, &Local>) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_has_encoding_surface(s, by_id))
}

fn stmt_has_encoding_surface(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Throw { value: e } => {
            expr_has_encoding_surface(e, by_id)
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(|s| stmt_has_encoding_surface(s, by_id))
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(|s| stmt_has_encoding_surface(s, by_id)))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(|s| stmt_has_encoding_surface(s, by_id)))
        }
        Stmt::Block { body } => body.iter().any(|s| stmt_has_encoding_surface(s, by_id)),
        _ => false,
    }
}

fn expr_has_encoding_surface(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, .. } => by_id.get(id).is_some_and(|l| {
            matches!(
                l.name.as_str(),
                "TextEncoder"
                    | "TextDecoder"
                    | "sha256"
                    | "randomBytes"
                    | "hmacSha256"
                    | "aeadEncrypt"
                    | "aeadDecrypt"
                    | "gzip"
                    | "gunzip"
                    | "deflate"
                    | "inflate"
            )
        }),
        Expr::Unary { arg, .. } => expr_has_encoding_surface(arg, by_id),
        Expr::Binary { left, right, .. } => {
            expr_has_encoding_surface(left, by_id) || expr_has_encoding_surface(right, by_id)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_encoding_surface(test, by_id)
                || expr_has_encoding_surface(consequent, by_id)
                || expr_has_encoding_surface(alternate, by_id)
        }
        Expr::Member {
            object, property, ..
        } => {
            is_encoding_method_key(property)
                || expr_has_encoding_surface(object, by_id)
                || expr_has_encoding_surface(property, by_id)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_has_encoding_surface(callee, by_id)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_encoding_surface(e, by_id),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_has_encoding_surface(value, by_id),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_has_encoding_surface(e, by_id),
            ArrayElement::Elision => false,
        }),
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                expr_has_encoding_surface(value, by_id)
            }
            ObjectProp::Spread(e) => expr_has_encoding_surface(e, by_id),
        }),
        _ => false,
    }
}

fn is_encoding_method_key(expr: &Expr) -> bool {
    match expr {
        Expr::String { value, .. } => {
            let s = js_string_to_utf8(value);
            matches!(s.as_str(), "toBase64" | "fromBase64" | "toHex" | "fromHex")
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
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            match (handler.is_some(), handler_param) {
                (true, None) | (true, Some(Pattern::Local(_))) | (false, None) => {}
                _ => return false,
            }
            body_ok(block)
                && handler.as_ref().is_none_or(|h| body_ok(h))
                && finalizer.as_ref().is_none_or(|f| body_ok(f))
        }
        Stmt::Block { body } => body_ok(body),
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
        }
        | Expr::New { callee, args, .. } => {
            expr_ok(callee)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_ok(value),
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => expr_ok(e),
            ArrayElement::Elision => true,
            ArrayElement::Spread(_) => false,
        }),
        Expr::Object { properties, .. } => properties.iter().all(|p| match p {
            ObjectProp::Property {
                key: ObjectPropKey::Static(_),
                value,
            } => expr_ok(value),
            ObjectProp::Property {
                key: ObjectPropKey::Computed(k),
                value,
            } => expr_ok(k) && expr_ok(value),
            _ => false,
        }),
        _ => false,
    }
}

fn js_string_to_utf8(s: &JsString) -> String {
    s.to_string_lossy()
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
                .ok_or_else(|| diag("es_encoding: missing value"))?;
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
                _ => return Err(diag("es_encoding: non-printable value")),
            }
        }
        writeln!(
            self.out,
            "; Draconic LLVM backend (L01 + L03 + L04 UTF-8 + Base64 + hex + SHA-256 + gzip)"
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
    fn classifies_text_encoder_roundtrip() {
        let m = compile_src(
            r#"
            let bytes = new TextEncoder().encode("hi");
            let len = bytes.length;
            let s = new TextDecoder().decode(bytes);
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_fatal_invalid_utf8() {
        let m = compile_src(
            r#"
            let ok = 0;
            try {
              new TextDecoder("utf-8", { fatal: true }).decode(new Uint8Array([255]));
              ok = -1;
            } catch (e) {
              ok = e.name === "TypeError" ? 1 : -2;
            }
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_base64_roundtrip() {
        let m = compile_src(
            r#"
            let hi = new Uint8Array([104, 105]).toBase64();
            let v = Uint8Array.fromBase64("aGk=");
            let n = v.length;
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_base64_invalid() {
        let m = compile_src(
            r#"
            let ok = 0;
            try {
              Uint8Array.fromBase64("!!!");
              ok = -1;
            } catch (e) {
              ok = e.name === "SyntaxError" ? 1 : -2;
            }
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_hex_roundtrip() {
        let m = compile_src(
            r#"
            let hi = new Uint8Array([104, 105]).toHex();
            let v = Uint8Array.fromHex("6869");
            let n = v.length;
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_sha256_vectors() {
        let m = compile_src(
            r#"
            let empty = sha256(new Uint8Array([])).toHex();
            let abc = sha256(new TextEncoder().encode("abc")).toHex();
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_sha256_invalid() {
        let m = compile_src(
            r#"
            let ok = 0;
            try {
              sha256("abc");
              ok = -1;
            } catch (e) {
              ok = e.name === "TypeError" ? 1 : -2;
            }
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_random_bytes() {
        let m = compile_src(
            r#"
            let tp = typeof randomBytes;
            let zlen = randomBytes(0).length;
            let alen = randomBytes(8).length;
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_random_bytes_invalid() {
        let m = compile_src(
            r#"
            let ok = 0;
            try {
              randomBytes("abc");
              ok = -1;
            } catch (e) {
              ok = e.name === "TypeError" ? 1 : -2;
            }
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_hmac_sha256_vectors() {
        let m = compile_src(
            r#"
            let t1 = hmacSha256(
              Uint8Array.fromHex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b"),
              new TextEncoder().encode("Hi There")
            ).toHex();
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_hmac_sha256_invalid() {
        let m = compile_src(
            r#"
            let ok = 0;
            try {
              hmacSha256("key", new Uint8Array([]));
              ok = -1;
            } catch (e) {
              ok = e.name === "TypeError" ? 1 : -2;
            }
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_hex_invalid() {
        let m = compile_src(
            r#"
            let ok = 0;
            try {
              Uint8Array.fromHex("zzz");
              ok = -1;
            } catch (e) {
              ok = e.name === "SyntaxError" ? 1 : -2;
            }
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_combined_encoding_surface() {
        let m = compile_src(
            r#"
            let s = new TextDecoder().decode(new TextEncoder().encode("café"));
            let b64 = new Uint8Array([104, 105]).toBase64();
            let hx = new Uint8Array([104, 105]).toHex();
            let b64s = new TextDecoder().decode(Uint8Array.fromBase64(b64));
            let hxs = new TextDecoder().decode(Uint8Array.fromHex(hx));
            let utf8_bad = 0;
            try {
              new TextDecoder("utf-8", { fatal: true }).decode(new Uint8Array([255]));
              utf8_bad = -1;
            } catch (e) {
              utf8_bad = e.name === "TypeError" ? 1 : -2;
            }
            let b64_bad = 0;
            try {
              Uint8Array.fromBase64("!!!");
              b64_bad = -1;
            } catch (e) {
              b64_bad = e.name === "SyntaxError" ? 1 : -2;
            }
            let hex_bad = 0;
            try {
              Uint8Array.fromHex("zzz");
              hex_bad = -1;
            } catch (e) {
              hex_bad = e.name === "SyntaxError" ? 1 : -2;
            }
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"), "{ir}");
        assert!(ir.contains("aGk="), "{ir}");
        assert!(ir.contains("6869"), "{ir}");
        assert!(ir.contains("c\"hi\\00"), "{ir}");
    }

    #[test]
    fn classifies_gzip_roundtrip() {
        let m = compile_src(
            r#"
            let src = new TextEncoder().encode("hello");
            let round = gunzip(gzip(src)).toHex() === src.toHex();
            let m0 = gzip(src)[0] === 31;
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_deflate_roundtrip() {
        let m = compile_src(
            r#"
            let src = new TextEncoder().encode("hello");
            let round = inflate(deflate(src)).toHex() === src.toHex();
            let z0 = deflate(src)[0] === 120;
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_gzip_invalid() {
        let m = compile_src(
            r#"
            let ok = 0;
            try {
              gzip("abc");
              ok = -1;
            } catch (e) {
              ok = e.name === "TypeError" ? 1 : -2;
            }
            let trunc = 0;
            try {
              gunzip(new Uint8Array([31, 139, 8]));
              trunc = -1;
            } catch (e) {
              trunc = e.name === "Error" ? 1 : -2;
            }
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_aead_vectors() {
        let m = compile_src(
            r#"
            let t13 = aeadEncrypt(
              Uint8Array.fromHex("0000000000000000000000000000000000000000000000000000000000000000"),
              Uint8Array.fromHex("000000000000000000000000"),
              new Uint8Array([])
            ).toHex();
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_aead_invalid() {
        let m = compile_src(
            r#"
            let ok = 0;
            try {
              aeadEncrypt("key", new Uint8Array(12), new Uint8Array([]));
              ok = -1;
            } catch (e) {
              ok = e.name === "TypeError" ? 1 : -2;
            }
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }
}
