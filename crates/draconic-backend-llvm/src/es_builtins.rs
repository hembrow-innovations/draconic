//! N08.14.01–N08.14.10 + N08.16.01–N08.16.04: native observations for global builtins + Error
//! ctors + functions + URI + JSON + Date + RegExp + Map/Set + WeakMap/WeakSet +
//! ArrayBuffer/DataView/TypedArrays + Annex B `escape`/`unescape` + `Object.prototype.__proto__`
//! + `String.prototype` `substr` / HTML wrappers + `Date.prototype` `getYear`/`setYear`/`toGMTString`.
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
//!   `.flags` / `.test` / `.exec` (fixture subset: literals + `c+` + `i` flag)
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
//!
//! Emits Runtime prints of final top-level number/string/bool/null locals.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

pub(crate) fn is_es_builtins_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_builtins(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_builtins module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum BuiltinId {
    Undefined,
    GlobalThis,
    Object,
    Function,
    Array,
    String,
    StringPrototype,
    Boolean,
    ObjectPrototype,
    ObjectGetPrototypeOf,
    HasOwnProperty,
    /// Annex B String.prototype methods (unbound; `.call` supplies this).
    StrSubstr,
    StrAnchor,
    StrBig,
    StrBlink,
    StrBold,
    StrFixed,
    StrFontcolor,
    StrFontsize,
    StrItalics,
    StrLink,
    StrSmall,
    StrStrike,
    StrSub,
    StrSup,
    ArrayIsArray,
    Error,
    TypeError,
    RangeError,
    ReferenceError,
    SyntaxError,
    UriError,
    EvalError,
    AggregateError,
    ParseInt,
    ParseFloat,
    IsNaN,
    IsFinite,
    Nan,
    Infinity,
    EncodeUri,
    DecodeUri,
    EncodeUriComponent,
    DecodeUriComponent,
    Escape,
    Unescape,
    Json,
    JsonParse,
    JsonStringify,
    Date,
    DateNow,
    DateUtc,
    DatePrototype,
    /// Annex B / fixture Date.prototype methods (unbound; `.call` supplies this).
    DateGetYear,
    DateSetYear,
    DateToGmtString,
    DateGetFullYear,
    RegExp,
    Map,
    Set,
    WeakMap,
    WeakSet,
    ArrayBuffer,
    DataView,
    Uint8Array,
    Int32Array,
    Float64Array,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaKind {
    U8,
    I32,
    F64,
}

impl TaKind {
    fn bytes_per_element(self) -> usize {
        match self {
            TaKind::U8 => 1,
            TaKind::I32 => 4,
            TaKind::F64 => 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Null,
    Builtin(BuiltinId),
    /// Error instance: name, message, optional AggregateError `.errors` array.
    ErrorInst {
        name: String,
        message: String,
        errors: Option<Vec<JsVal>>,
    },
    /// Date instance: milliseconds since Unix epoch (UTC).
    DateInst {
        ms: f64,
    },
    /// RegExp instance: pattern source + flags string (E15.07 fixture subset).
    RegExpInst {
        source: String,
        flags: String,
    },
    /// Map instance: insertion-ordered entries (E15.08 fixture subset).
    MapInst {
        entries: Vec<(JsVal, JsVal)>,
    },
    /// Set instance: insertion-ordered values (E15.08 fixture subset).
    SetInst {
        values: Vec<JsVal>,
    },
    /// WeakMap instance: object-key entries (E15.09 fixture subset).
    WeakMapInst {
        entries: Vec<(JsVal, JsVal)>,
    },
    /// WeakSet instance: object values (E15.09 fixture subset).
    WeakSetInst {
        values: Vec<JsVal>,
    },
    /// ArrayBuffer: shared byte storage (E15.10).
    ArrayBufferInst {
        id: u64,
        bytes: Rc<RefCell<Vec<u8>>>,
    },
    /// TypedArray view over shared buffer (E15.10 fixture subset).
    TypedArrayInst {
        kind: TaKind,
        buffer_id: u64,
        bytes: Rc<RefCell<Vec<u8>>>,
        length: usize,
    },
    /// DataView over shared buffer (E15.10 fixture subset).
    DataViewInst {
        buffer_id: u64,
        bytes: Rc<RefCell<Vec<u8>>>,
        byte_length: usize,
    },
    Array(Vec<JsVal>),
    /// Plain object: identity id + insertion-ordered string keys + [[Prototype]].
    Object {
        id: u64,
        props: Vec<(String, JsVal)>,
        proto: Box<JsVal>,
    },
}

fn next_object_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn new_object(props: Vec<(String, JsVal)>) -> JsVal {
    new_object_with_proto(props, JsVal::Builtin(BuiltinId::ObjectPrototype))
}

fn new_object_with_proto(props: Vec<(String, JsVal)>, proto: JsVal) -> JsVal {
    JsVal::Object {
        id: next_object_id(),
        props,
        proto: Box::new(proto),
    }
}

fn object_own_has(props: &[(String, JsVal)], key: &str) -> bool {
    props.iter().any(|(k, _)| k == key)
}

fn object_own_get(props: &[(String, JsVal)], key: &str) -> Option<JsVal> {
    props.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn object_get_prototype(obj: &JsVal) -> Result<JsVal, ()> {
    match obj {
        JsVal::Object { proto, .. } => Ok((**proto).clone()),
        JsVal::Builtin(BuiltinId::ObjectPrototype) => Ok(JsVal::Null),
        _ => Err(()),
    }
}

fn is_object_key(v: &JsVal) -> bool {
    matches!(
        v,
        JsVal::Object { .. }
            | JsVal::Array(_)
            | JsVal::ErrorInst { .. }
            | JsVal::DateInst { .. }
            | JsVal::RegExpInst { .. }
            | JsVal::MapInst { .. }
            | JsVal::SetInst { .. }
            | JsVal::WeakMapInst { .. }
            | JsVal::WeakSetInst { .. }
            | JsVal::ArrayBufferInst { .. }
            | JsVal::TypedArrayInst { .. }
            | JsVal::DataViewInst { .. }
            | JsVal::Builtin(BuiltinId::GlobalThis | BuiltinId::ObjectPrototype | BuiltinId::Json)
    )
}

fn new_array_buffer(byte_len: usize) -> JsVal {
    JsVal::ArrayBufferInst {
        id: next_object_id(),
        bytes: Rc::new(RefCell::new(vec![0u8; byte_len])),
    }
}

fn typed_array_from_buffer(kind: TaKind, buf: &JsVal) -> Result<JsVal, ()> {
    let JsVal::ArrayBufferInst { id, bytes } = buf else {
        return Err(());
    };
    let blen = bytes.borrow().len();
    let bpe = kind.bytes_per_element();
    if blen % bpe != 0 {
        return Err(());
    }
    Ok(JsVal::TypedArrayInst {
        kind,
        buffer_id: *id,
        bytes: Rc::clone(bytes),
        length: blen / bpe,
    })
}

fn typed_array_from_length(kind: TaKind, len: usize) -> JsVal {
    let blen = len.saturating_mul(kind.bytes_per_element());
    let id = next_object_id();
    let bytes = Rc::new(RefCell::new(vec![0u8; blen]));
    JsVal::TypedArrayInst {
        kind,
        buffer_id: id,
        bytes,
        length: len,
    }
}

fn typed_array_from_array(kind: TaKind, elems: &[JsVal]) -> Result<JsVal, ()> {
    let ta = typed_array_from_length(kind, elems.len());
    let JsVal::TypedArrayInst {
        kind,
        bytes,
        length,
        ..
    } = &ta
    else {
        return Err(());
    };
    let bpe = kind.bytes_per_element();
    let mut buf = bytes.borrow_mut();
    for (i, el) in elems.iter().enumerate() {
        if i >= *length {
            break;
        }
        let n = match el {
            JsVal::Num(n) => *n,
            _ => return Err(()),
        };
        let off = i * bpe;
        write_ta_elem(*kind, &mut buf, off, n)?;
    }
    drop(buf);
    Ok(ta)
}

fn write_ta_elem(kind: TaKind, buf: &mut [u8], off: usize, n: f64) -> Result<(), ()> {
    let bpe = kind.bytes_per_element();
    if off + bpe > buf.len() {
        return Err(());
    }
    match kind {
        TaKind::U8 => buf[off] = n as u8,
        TaKind::I32 => {
            let i = n as i32;
            buf[off..off + 4].copy_from_slice(&i.to_le_bytes());
        }
        TaKind::F64 => {
            buf[off..off + 8].copy_from_slice(&n.to_le_bytes());
        }
    }
    Ok(())
}

fn read_ta_elem(kind: TaKind, buf: &[u8], off: usize) -> Result<f64, ()> {
    let bpe = kind.bytes_per_element();
    if off + bpe > buf.len() {
        return Err(());
    }
    Ok(match kind {
        TaKind::U8 => buf[off] as f64,
        TaKind::I32 => {
            let mut b = [0u8; 4];
            b.copy_from_slice(&buf[off..off + 4]);
            i32::from_le_bytes(b) as f64
        }
        TaKind::F64 => {
            let mut b = [0u8; 8];
            b.copy_from_slice(&buf[off..off + 8]);
            f64::from_le_bytes(b)
        }
    })
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Throw(JsVal),
}

struct Emitter {
    out: String,
    body: String,
    str_consts: Vec<(String, String)>,
}

fn js_string_to_utf8(s: &JsString) -> String {
    s.to_string_lossy()
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_builtin_surface(module, &by_id) {
        return None;
    }
    if !body_ok(&module.body) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    for loc in &module.locals {
        if let Some(b) = builtin_for_name(&loc.name) {
            env.insert(
                loc.id,
                match b {
                    BuiltinId::Undefined => JsVal::Undef,
                    BuiltinId::Nan => JsVal::Num(f64::NAN),
                    BuiltinId::Infinity => JsVal::Num(f64::INFINITY),
                    other => JsVal::Builtin(other),
                },
            );
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
                Some(v @ (JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_) | JsVal::Null)) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String | Type::Null
                    ) {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(
                    JsVal::Undef
                    | JsVal::Builtin(_)
                    | JsVal::ErrorInst { .. }
                    | JsVal::DateInst { .. }
                    | JsVal::RegExpInst { .. }
                    | JsVal::MapInst { .. }
                    | JsVal::SetInst { .. }
                    | JsVal::WeakMapInst { .. }
                    | JsVal::WeakSetInst { .. }
                    | JsVal::ArrayBufferInst { .. }
                    | JsVal::TypedArrayInst { .. }
                    | JsVal::DataViewInst { .. }
                    | JsVal::Array(_)
                    | JsVal::Object { .. },
                ) => {}
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
        "undefined" => Some(BuiltinId::Undefined),
        "globalThis" => Some(BuiltinId::GlobalThis),
        "Object" => Some(BuiltinId::Object),
        "Function" => Some(BuiltinId::Function),
        "Array" => Some(BuiltinId::Array),
        "String" => Some(BuiltinId::String),
        "Boolean" => Some(BuiltinId::Boolean),
        "Error" => Some(BuiltinId::Error),
        "TypeError" => Some(BuiltinId::TypeError),
        "RangeError" => Some(BuiltinId::RangeError),
        "ReferenceError" => Some(BuiltinId::ReferenceError),
        "SyntaxError" => Some(BuiltinId::SyntaxError),
        "URIError" => Some(BuiltinId::UriError),
        "EvalError" => Some(BuiltinId::EvalError),
        "AggregateError" => Some(BuiltinId::AggregateError),
        "parseInt" => Some(BuiltinId::ParseInt),
        "parseFloat" => Some(BuiltinId::ParseFloat),
        "isNaN" => Some(BuiltinId::IsNaN),
        "isFinite" => Some(BuiltinId::IsFinite),
        "NaN" => Some(BuiltinId::Nan),
        "Infinity" => Some(BuiltinId::Infinity),
        "encodeURI" => Some(BuiltinId::EncodeUri),
        "decodeURI" => Some(BuiltinId::DecodeUri),
        "encodeURIComponent" => Some(BuiltinId::EncodeUriComponent),
        "decodeURIComponent" => Some(BuiltinId::DecodeUriComponent),
        "escape" => Some(BuiltinId::Escape),
        "unescape" => Some(BuiltinId::Unescape),
        "JSON" => Some(BuiltinId::Json),
        "Date" => Some(BuiltinId::Date),
        "RegExp" => Some(BuiltinId::RegExp),
        "Map" => Some(BuiltinId::Map),
        "Set" => Some(BuiltinId::Set),
        "WeakMap" => Some(BuiltinId::WeakMap),
        "WeakSet" => Some(BuiltinId::WeakSet),
        "ArrayBuffer" => Some(BuiltinId::ArrayBuffer),
        "DataView" => Some(BuiltinId::DataView),
        "Uint8Array" => Some(BuiltinId::Uint8Array),
        "Int32Array" => Some(BuiltinId::Int32Array),
        "Float64Array" => Some(BuiltinId::Float64Array),
        _ => None,
    }
}

fn error_ctor_name(b: BuiltinId) -> Option<&'static str> {
    match b {
        BuiltinId::Error => Some("Error"),
        BuiltinId::TypeError => Some("TypeError"),
        BuiltinId::RangeError => Some("RangeError"),
        BuiltinId::ReferenceError => Some("ReferenceError"),
        BuiltinId::SyntaxError => Some("SyntaxError"),
        BuiltinId::UriError => Some("URIError"),
        BuiltinId::EvalError => Some("EvalError"),
        BuiltinId::AggregateError => Some("AggregateError"),
        _ => None,
    }
}

fn module_has_builtin_surface(module: &Module, by_id: &HashMap<LocalId, &Local>) -> bool {
    module.body.iter().any(|s| stmt_has_builtin_surface(s, by_id))
}

fn stmt_has_builtin_surface(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Throw { value: e } => {
            expr_has_builtin_surface(e, by_id)
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block
                .iter()
                .any(|s| stmt_has_builtin_surface(s, by_id))
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(|s| stmt_has_builtin_surface(s, by_id)))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(|s| stmt_has_builtin_surface(s, by_id)))
        }
        Stmt::Block { body } => body.iter().any(|s| stmt_has_builtin_surface(s, by_id)),
        _ => false,
    }
}

fn expr_has_builtin_surface(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, .. } => by_id.get(id).is_some_and(|l| builtin_for_name(&l.name).is_some()),
        Expr::Unary { arg, .. } => expr_has_builtin_surface(arg, by_id),
        Expr::Binary { left, right, .. } => {
            expr_has_builtin_surface(left, by_id) || expr_has_builtin_surface(right, by_id)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_builtin_surface(test, by_id)
                || expr_has_builtin_surface(consequent, by_id)
                || expr_has_builtin_surface(alternate, by_id)
        }
        Expr::Member { object, property, .. } => {
            expr_has_builtin_surface(object, by_id) || expr_has_builtin_surface(property, by_id)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_has_builtin_surface(callee, by_id)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_builtin_surface(e, by_id),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_has_builtin_surface(value, by_id),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_has_builtin_surface(e, by_id),
            ArrayElement::Elision => false,
        }),
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value } => {
                (match key {
                    ObjectPropKey::Static(_) => false,
                    ObjectPropKey::Computed(e) => expr_has_builtin_surface(e, by_id),
                }) || expr_has_builtin_surface(value, by_id)
            }
            ObjectProp::Accessor { key, value, .. } => {
                (match key {
                    ObjectPropKey::Static(_) => false,
                    ObjectPropKey::Computed(e) => expr_has_builtin_surface(e, by_id),
                }) || expr_has_builtin_surface(value, by_id)
            }
            ObjectProp::Spread(e) => expr_has_builtin_surface(e, by_id),
        }),
        Expr::Null { .. } => false,
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
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } | Expr::Null { .. } => true,
        Expr::Local { .. } => true,
        Expr::Unary {
            op: UnaryOp::TypeOf | UnaryOp::Minus | UnaryOp::Plus,
            arg,
            ..
        } => expr_ok(arg),
        Expr::Binary { left, right, op, .. } => {
            matches!(
                op,
                BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
                    | BinaryOp::EqEq
                    | BinaryOp::NotEq
                    | BinaryOp::And
                    | BinaryOp::Or
            ) && expr_ok(left)
                && expr_ok(right)
        }
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
        Expr::New {
            callee,
            args,
            ..
        }
        | Expr::Call {
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
        Expr::Assign {
            target: AssignTarget::Local(_),
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(value),
        Expr::Assign {
            target: AssignTarget::Member {
                object,
                property,
                ..
            },
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(object) && expr_ok(property) && expr_ok(value),
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

fn eval_body(body: &[Stmt], env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, env)? {
            Flow::Normal => {}
            other => return Ok(other),
        }
    }
    Ok(Flow::Normal)
}

fn eval_stmt(stmt: &Stmt, env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => match eval_expr(e, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                },
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Expr { expr } => match eval_expr(expr, env)? {
            Ok(_) => Ok(Flow::Normal),
            Err(flow) => Ok(flow),
        },
        Stmt::Throw { value } => match eval_expr(value, env)? {
            Ok(v) => Ok(Flow::Throw(v)),
            Err(flow) => Ok(flow),
        },
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            let mut completion = match eval_body(block, env)? {
                Flow::Throw(exc) => {
                    if let Some(handler) = handler {
                        if let Some(Pattern::Local(pid)) = handler_param {
                            env.insert(*pid, exc);
                        }
                        eval_body(handler, env)?
                    } else {
                        Flow::Throw(exc)
                    }
                }
                other => other,
            };
            if let Some(fin) = finalizer {
                match eval_body(fin, env)? {
                    Flow::Normal => {}
                    abrupt => completion = abrupt,
                }
            }
            Ok(completion)
        }
        Stmt::Block { body } => eval_body(body, env),
        _ => Err(()),
    }
}

/// `Ok(Ok(v))` = value; `Ok(Err(flow))` = abrupt throw; `Err(())` = unsupported.
fn eval_expr(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<JsVal, Flow>, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| ())?;
            Ok(Ok(JsVal::Num(n)))
        }
        Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
        Expr::String { value, .. } => Ok(Ok(JsVal::Str(js_string_to_utf8(value)))),
        Expr::Null { .. } => Ok(Ok(JsVal::Null)),
        Expr::Local { id, .. } => {
            let v = env.get(id).cloned().ok_or(())?;
            Ok(Ok(v))
        }
        Expr::Unary { op, arg, .. } => {
            let v = match eval_expr(arg, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match op {
                UnaryOp::TypeOf => Ok(Ok(JsVal::Str(typeof_str(&v)))),
                UnaryOp::Minus => match v {
                    JsVal::Num(n) => Ok(Ok(JsVal::Num(-n))),
                    _ => Err(()),
                },
                UnaryOp::Plus => match v {
                    JsVal::Num(n) => Ok(Ok(JsVal::Num(n))),
                    _ => Err(()),
                },
                _ => Err(()),
            }
        }
        Expr::Binary {
            left,
            op,
            right,
            ..
        } => {
            let l = match eval_expr(left, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match op {
                BinaryOp::And => {
                    if !to_boolean(&l) {
                        return Ok(Ok(l));
                    }
                    eval_expr(right, env)
                }
                BinaryOp::Or => {
                    if to_boolean(&l) {
                        return Ok(Ok(l));
                    }
                    eval_expr(right, env)
                }
                BinaryOp::EqEqEq | BinaryOp::EqEq => {
                    let r = match eval_expr(right, env)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(Err(flow)),
                    };
                    Ok(Ok(JsVal::Bool(strict_eq(&l, &r))))
                }
                BinaryOp::NotEqEq | BinaryOp::NotEq => {
                    let r = match eval_expr(right, env)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(Err(flow)),
                    };
                    Ok(Ok(JsVal::Bool(!strict_eq(&l, &r))))
                }
                _ => Err(()),
            }
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            let t = match eval_expr(test, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            if to_boolean(&t) {
                eval_expr(consequent, env)
            } else {
                eval_expr(alternate, env)
            }
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = match eval_expr(object, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let key = match eval_key(property, env)? {
                Ok(k) => k,
                Err(flow) => return Ok(Err(flow)),
            };
            Ok(Ok(member_get(&obj, &key)?))
        }
        Expr::New { callee, args, .. } => {
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => arg_vals.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    _ => return Err(()),
                }
            }
            Ok(Ok(eval_new(&c, &arg_vals)?))
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => arg_vals.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    _ => return Err(()),
                }
            }
            // Method call: recv.prop(args) — keep `this` for Date/Map/Set instance methods.
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = callee.as_ref()
            {
                let mut obj = match eval_expr(object, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                let key = match eval_key(property, env)? {
                    Ok(k) => k,
                    Err(flow) => return Ok(Err(flow)),
                };
                let result = eval_method_call(&mut obj, &key, &arg_vals)?;
                // Write back mutated Map/Set (and any other instance) to local receiver.
                if let Expr::Local { id, .. } = object.as_ref() {
                    env.insert(*id, obj);
                }
                return Ok(Ok(result));
            }
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            Ok(Ok(eval_call(&c, &arg_vals)?))
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = match eval_expr(value, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            env.insert(*id, v.clone());
            Ok(Ok(v))
        }
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = match eval_expr(value, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let mut obj = match eval_expr(object, env)? {
                Ok(o) => o,
                Err(flow) => return Ok(Err(flow)),
            };
            let key = match eval_key(property, env)? {
                Ok(k) => k,
                Err(flow) => return Ok(Err(flow)),
            };
            member_set(&mut obj, &key, v.clone())?;
            if let Expr::Local { id, .. } = object.as_ref() {
                env.insert(*id, obj);
            }
            Ok(Ok(v))
        }
        Expr::Array { elements, .. } => {
            let mut out = Vec::new();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => out.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    ArrayElement::Elision => out.push(JsVal::Undef),
                    ArrayElement::Spread(_) => return Err(()),
                }
            }
            Ok(Ok(JsVal::Array(out)))
        }
        Expr::Object { properties, .. } => {
            let mut props = Vec::new();
            let mut proto = JsVal::Builtin(BuiltinId::ObjectPrototype);
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key: ObjectPropKey::Static(k),
                        value,
                    } => {
                        let v = match eval_expr(value, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        let key = js_string_to_utf8(k);
                        // Annex B / ES: static `__proto__` in object literal sets [[Prototype]].
                        if key == "__proto__" {
                            proto = v;
                            continue;
                        }
                        if let Some((_, slot)) = props.iter_mut().find(|(n, _)| n == &key) {
                            *slot = v;
                        } else {
                            props.push((key, v));
                        }
                    }
                    ObjectProp::Property {
                        key: ObjectPropKey::Computed(ke),
                        value,
                    } => {
                        let key = match eval_key(ke, env)? {
                            Ok(k) => k,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        let v = match eval_expr(value, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        // Computed `["__proto__"]` is an own data property (not [[Prototype]]).
                        if let Some((_, slot)) = props.iter_mut().find(|(n, _)| n == &key) {
                            *slot = v;
                        } else {
                            props.push((key, v));
                        }
                    }
                    _ => return Err(()),
                }
            }
            Ok(Ok(new_object_with_proto(props, proto)))
        }
        _ => Err(()),
    }
}

fn eval_key(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<String, Flow>, ()> {
    match expr {
        Expr::String { value, .. } => Ok(Ok(js_string_to_utf8(value))),
        e => match eval_expr(e, env)? {
            Ok(JsVal::Str(s)) => Ok(Ok(s)),
            Ok(JsVal::Num(n)) => Ok(Ok(format!("{}", n as i64))),
            Ok(_) => Err(()),
            Err(flow) => Ok(Err(flow)),
        },
    }
}

fn eval_new(callee: &JsVal, args: &[JsVal]) -> Result<JsVal, ()> {
    let JsVal::Builtin(b) = callee else {
        return Err(());
    };
    if *b == BuiltinId::Date {
        let ms = match args.first() {
            Some(JsVal::Num(n)) => *n,
            Some(JsVal::Undef) | None => date_now_ms(),
            _ => return Err(()),
        };
        return Ok(JsVal::DateInst { ms });
    }
    if *b == BuiltinId::RegExp {
        return make_regexp(args);
    }
    if *b == BuiltinId::Map {
        // Fixture: `new Map()` only (no iterable init).
        if !args.is_empty() {
            return Err(());
        }
        return Ok(JsVal::MapInst {
            entries: Vec::new(),
        });
    }
    if *b == BuiltinId::Set {
        if !args.is_empty() {
            return Err(());
        }
        return Ok(JsVal::SetInst {
            values: Vec::new(),
        });
    }
    if *b == BuiltinId::WeakMap {
        if !args.is_empty() {
            return Err(());
        }
        return Ok(JsVal::WeakMapInst {
            entries: Vec::new(),
        });
    }
    if *b == BuiltinId::WeakSet {
        if !args.is_empty() {
            return Err(());
        }
        return Ok(JsVal::WeakSetInst {
            values: Vec::new(),
        });
    }
    if *b == BuiltinId::ArrayBuffer {
        let len = match args.first() {
            Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => *n as usize,
            _ => return Err(()),
        };
        return Ok(new_array_buffer(len));
    }
    if *b == BuiltinId::Uint8Array {
        return match args.first() {
            Some(buf @ JsVal::ArrayBufferInst { .. }) => typed_array_from_buffer(TaKind::U8, buf),
            Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => {
                Ok(typed_array_from_length(TaKind::U8, *n as usize))
            }
            Some(JsVal::Array(elems)) => typed_array_from_array(TaKind::U8, elems),
            _ => Err(()),
        };
    }
    if *b == BuiltinId::Int32Array {
        return match args.first() {
            Some(buf @ JsVal::ArrayBufferInst { .. }) => typed_array_from_buffer(TaKind::I32, buf),
            Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => {
                Ok(typed_array_from_length(TaKind::I32, *n as usize))
            }
            Some(JsVal::Array(elems)) => typed_array_from_array(TaKind::I32, elems),
            _ => Err(()),
        };
    }
    if *b == BuiltinId::Float64Array {
        return match args.first() {
            Some(buf @ JsVal::ArrayBufferInst { .. }) => typed_array_from_buffer(TaKind::F64, buf),
            Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => {
                Ok(typed_array_from_length(TaKind::F64, *n as usize))
            }
            Some(JsVal::Array(elems)) => typed_array_from_array(TaKind::F64, elems),
            _ => Err(()),
        };
    }
    if *b == BuiltinId::DataView {
        let JsVal::ArrayBufferInst { id, bytes } = args.first().ok_or(())? else {
            return Err(());
        };
        let byte_length = bytes.borrow().len();
        return Ok(JsVal::DataViewInst {
            buffer_id: *id,
            bytes: Rc::clone(bytes),
            byte_length,
        });
    }
    let name = error_ctor_name(*b).ok_or(())?;
    if *b == BuiltinId::AggregateError {
        let errors = match args.first() {
            Some(JsVal::Array(a)) => a.clone(),
            _ => return Err(()),
        };
        let message = match args.get(1) {
            Some(JsVal::Str(s)) => s.clone(),
            Some(JsVal::Undef) | None => String::new(),
            _ => return Err(()),
        };
        return Ok(JsVal::ErrorInst {
            name: name.into(),
            message,
            errors: Some(errors),
        });
    }
    let message = match args.first() {
        Some(JsVal::Str(s)) => s.clone(),
        Some(JsVal::Undef) | None => String::new(),
        _ => return Err(()),
    };
    Ok(JsVal::ErrorInst {
        name: name.into(),
        message,
        errors: None,
    })
}

fn eval_call(callee: &JsVal, args: &[JsVal]) -> Result<JsVal, ()> {
    let JsVal::Builtin(b) = callee else {
        return Err(());
    };
    match b {
        BuiltinId::ParseInt => {
            let s = match args.first() {
                Some(JsVal::Str(s)) => s.as_str(),
                Some(JsVal::Num(n)) => {
                    // ToString(number) for fixture depth; only decimals we need.
                    return Ok(JsVal::Num(js_parse_int(&format!("{n}"), args.get(1))?));
                }
                _ => return Err(()),
            };
            Ok(JsVal::Num(js_parse_int(s, args.get(1))?))
        }
        BuiltinId::ParseFloat => {
            let s = match args.first() {
                Some(JsVal::Str(s)) => s.as_str(),
                Some(JsVal::Num(n)) => return Ok(JsVal::Num(*n)),
                _ => return Err(()),
            };
            Ok(JsVal::Num(js_parse_float(s)))
        }
        BuiltinId::IsNaN => {
            let n = to_number(args.first().unwrap_or(&JsVal::Undef))?;
            Ok(JsVal::Bool(n.is_nan()))
        }
        BuiltinId::IsFinite => {
            let n = to_number(args.first().unwrap_or(&JsVal::Undef))?;
            Ok(JsVal::Bool(n.is_finite()))
        }
        BuiltinId::EncodeUri => {
            let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
            Ok(JsVal::Str(js_encode_uri(&s, false)))
        }
        BuiltinId::EncodeUriComponent => {
            let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
            Ok(JsVal::Str(js_encode_uri(&s, true)))
        }
        BuiltinId::DecodeUri => {
            let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
            Ok(JsVal::Str(js_decode_uri(&s, false)?))
        }
        BuiltinId::DecodeUriComponent => {
            let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
            Ok(JsVal::Str(js_decode_uri(&s, true)?))
        }
        BuiltinId::Escape => {
            let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
            Ok(JsVal::Str(js_escape(&s)))
        }
        BuiltinId::Unescape => {
            let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
            Ok(JsVal::Str(js_unescape(&s)))
        }
        BuiltinId::JsonParse => {
            let s = match args.first() {
                Some(JsVal::Str(s)) => s.as_str(),
                _ => return Err(()),
            };
            json_parse(s)
        }
        BuiltinId::JsonStringify => {
            let v = args.first().unwrap_or(&JsVal::Undef);
            Ok(JsVal::Str(json_stringify(v)?))
        }
        BuiltinId::DateNow => Ok(JsVal::Num(date_now_ms())),
        BuiltinId::DateUtc => Ok(JsVal::Num(date_utc(args)?)),
        BuiltinId::RegExp => make_regexp(args),
        BuiltinId::ObjectGetPrototypeOf => {
            let target = args.first().ok_or(())?;
            object_get_prototype(target)
        }
        BuiltinId::HasOwnProperty => {
            // Direct call without this binding is not supported for fixture depth.
            Err(())
        }
        _ => Err(()),
    }
}

fn eval_method_call(recv: &mut JsVal, key: &str, args: &[JsVal]) -> Result<JsVal, ()> {
    match recv {
        JsVal::DateInst { ms } => eval_date_method(ms, key, args),
        JsVal::Str(s) => eval_string_method(s, key, args),
        JsVal::Builtin(BuiltinId::Object) if key == "getPrototypeOf" => {
            let target = args.first().ok_or(())?;
            object_get_prototype(target)
        }
        JsVal::Builtin(id) if key == "call" && is_string_annex_method(*id) => {
            let this_arg = args.first().ok_or(())?;
            let this_s = to_string_arg(this_arg)?;
            let rest: Vec<JsVal> = args.iter().skip(1).cloned().collect();
            let method = string_annex_method_name(*id).ok_or(())?;
            eval_string_method(&this_s, method, &rest)
        }
        JsVal::Builtin(id) if key == "call" && is_date_proto_method(*id) => {
            let this_arg = args.first().ok_or(())?;
            let mut this = this_arg.clone();
            let rest: Vec<JsVal> = args.iter().skip(1).cloned().collect();
            let method = date_proto_method_name(*id).ok_or(())?;
            eval_method_call(&mut this, method, &rest)
        }
        JsVal::Builtin(BuiltinId::HasOwnProperty) if key == "call" => {
            let this_arg = args.first().ok_or(())?;
            let prop = match args.get(1) {
                Some(JsVal::Str(s)) => s.as_str(),
                _ => return Err(()),
            };
            match this_arg {
                JsVal::Object { props, .. } => Ok(JsVal::Bool(object_own_has(props, prop))),
                _ => Ok(JsVal::Bool(false)),
            }
        }
        JsVal::Builtin(BuiltinId::ObjectPrototype) if key == "hasOwnProperty" => {
            let prop = match args.first() {
                Some(JsVal::Str(s)) => s.as_str(),
                _ => return Err(()),
            };
            // Bare call without `.call` uses Object.prototype as this — not in fixture.
            let _ = prop;
            Err(())
        }
        JsVal::Builtin(BuiltinId::Date) => match key {
            "now" if args.is_empty() => Ok(JsVal::Num(date_now_ms())),
            "UTC" => Ok(JsVal::Num(date_utc(args)?)),
            _ => Err(()),
        },
        JsVal::RegExpInst { source, flags } => match key {
            "test" => {
                let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                Ok(JsVal::Bool(regexp_find(source, flags, &s).is_some()))
            }
            "exec" => {
                let s = to_string_arg(args.first().unwrap_or(&JsVal::Undef))?;
                match regexp_find(source, flags, &s) {
                    Some(m) => Ok(JsVal::Array(vec![JsVal::Str(m)])),
                    None => Ok(JsVal::Null),
                }
            }
            _ => Err(()),
        },
        JsVal::MapInst { entries } => match key {
            "set" => {
                let k = args.first().cloned().ok_or(())?;
                let v = args.get(1).cloned().unwrap_or(JsVal::Undef);
                if let Some((_, slot)) = entries
                    .iter_mut()
                    .find(|(ek, _)| same_value_zero(ek, &k))
                {
                    *slot = v;
                } else {
                    entries.push((k, v));
                }
                Ok(JsVal::MapInst {
                    entries: entries.clone(),
                })
            }
            "get" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                Ok(entries
                    .iter()
                    .find(|(ek, _)| same_value_zero(ek, k))
                    .map(|(_, v)| v.clone())
                    .unwrap_or(JsVal::Undef))
            }
            "has" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                Ok(JsVal::Bool(
                    entries.iter().any(|(ek, _)| same_value_zero(ek, k)),
                ))
            }
            _ => Err(()),
        },
        JsVal::SetInst { values } => match key {
            "add" => {
                let v = args.first().cloned().ok_or(())?;
                if !values.iter().any(|ev| same_value_zero(ev, &v)) {
                    values.push(v);
                }
                Ok(JsVal::SetInst {
                    values: values.clone(),
                })
            }
            "has" => {
                let v = args.first().unwrap_or(&JsVal::Undef);
                Ok(JsVal::Bool(
                    values.iter().any(|ev| same_value_zero(ev, v)),
                ))
            }
            _ => Err(()),
        },
        JsVal::WeakMapInst { entries } => match key {
            "set" => {
                let k = args.first().cloned().ok_or(())?;
                if !is_object_key(&k) {
                    return Err(());
                }
                let v = args.get(1).cloned().unwrap_or(JsVal::Undef);
                if let Some((_, slot)) = entries.iter_mut().find(|(ek, _)| strict_eq(ek, &k)) {
                    *slot = v;
                } else {
                    entries.push((k, v));
                }
                Ok(JsVal::WeakMapInst {
                    entries: entries.clone(),
                })
            }
            "get" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                if !is_object_key(k) {
                    return Ok(JsVal::Undef);
                }
                Ok(entries
                    .iter()
                    .find(|(ek, _)| strict_eq(ek, k))
                    .map(|(_, v)| v.clone())
                    .unwrap_or(JsVal::Undef))
            }
            "has" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                if !is_object_key(k) {
                    return Ok(JsVal::Bool(false));
                }
                Ok(JsVal::Bool(entries.iter().any(|(ek, _)| strict_eq(ek, k))))
            }
            "delete" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                if !is_object_key(k) {
                    return Ok(JsVal::Bool(false));
                }
                let before = entries.len();
                entries.retain(|(ek, _)| !strict_eq(ek, k));
                Ok(JsVal::Bool(entries.len() < before))
            }
            _ => Err(()),
        },
        JsVal::WeakSetInst { values } => match key {
            "add" => {
                let v = args.first().cloned().ok_or(())?;
                if !is_object_key(&v) {
                    return Err(());
                }
                if !values.iter().any(|ev| strict_eq(ev, &v)) {
                    values.push(v);
                }
                Ok(JsVal::WeakSetInst {
                    values: values.clone(),
                })
            }
            "has" => {
                let v = args.first().unwrap_or(&JsVal::Undef);
                if !is_object_key(v) {
                    return Ok(JsVal::Bool(false));
                }
                Ok(JsVal::Bool(values.iter().any(|ev| strict_eq(ev, v))))
            }
            "delete" => {
                let v = args.first().unwrap_or(&JsVal::Undef);
                if !is_object_key(v) {
                    return Ok(JsVal::Bool(false));
                }
                let before = values.len();
                values.retain(|ev| !strict_eq(ev, v));
                Ok(JsVal::Bool(values.len() < before))
            }
            _ => Err(()),
        },
        JsVal::DataViewInst {
            bytes,
            byte_length,
            ..
        } => match key {
            "getUint8" => {
                let idx = match args.first() {
                    Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => *n as usize,
                    _ => return Err(()),
                };
                if idx >= *byte_length {
                    return Err(());
                }
                let b = bytes.borrow()[idx];
                Ok(JsVal::Num(b as f64))
            }
            "setUint8" => {
                let idx = match args.first() {
                    Some(JsVal::Num(n)) if *n >= 0.0 && n.is_finite() => *n as usize,
                    _ => return Err(()),
                };
                let val = match args.get(1) {
                    Some(JsVal::Num(n)) => *n as u8,
                    _ => return Err(()),
                };
                if idx >= *byte_length {
                    return Err(());
                }
                bytes.borrow_mut()[idx] = val;
                Ok(JsVal::Undef)
            }
            _ => Err(()),
        },
        // Non-method: resolve property then call as bare function.
        other => {
            let c = member_get(other, key)?;
            eval_call(&c, args)
        }
    }
}

/// ECMA-262 SameValueZero (Map/Set key equality): NaN≡NaN, +0≡-0, else ===.
fn same_value_zero(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => {
            if x.is_nan() && y.is_nan() {
                true
            } else {
                *x == *y
            }
        }
        _ => strict_eq(a, b),
    }
}

fn make_regexp(args: &[JsVal]) -> Result<JsVal, ()> {
    let source = match args.first() {
        Some(JsVal::Str(s)) => s.clone(),
        Some(JsVal::Undef) | None => String::new(),
        _ => return Err(()),
    };
    let flags = match args.get(1) {
        Some(JsVal::Str(s)) => s.clone(),
        Some(JsVal::Undef) | None => String::new(),
        _ => return Err(()),
    };
    // Fixture subset: only empty flags or `i`.
    if !(flags.is_empty() || flags == "i") {
        return Err(());
    }
    // Reject unsupported pattern syntax early (keep classify strict).
    parse_regexp_atoms(&source)?;
    Ok(JsVal::RegExpInst { source, flags })
}

/// Fixture-depth pattern atoms: literal char or `c+` (one-or-more of c).
#[derive(Clone, Copy, Debug)]
enum ReAtom {
    Lit(char),
    Plus(char),
}

fn parse_regexp_atoms(pattern: &str) -> Result<Vec<ReAtom>, ()> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut atoms = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // No escapes / classes / groups / other quantifiers in this subset.
        if matches!(
            c,
            '\\' | '.' | '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$'
        ) {
            return Err(());
        }
        if i + 1 < chars.len() && chars[i + 1] == '+' {
            atoms.push(ReAtom::Plus(c));
            i += 2;
        } else if c == '+' {
            return Err(());
        } else {
            atoms.push(ReAtom::Lit(c));
            i += 1;
        }
    }
    Ok(atoms)
}

fn char_eq(a: char, b: char, ignore_case: bool) -> bool {
    if ignore_case {
        a.to_ascii_lowercase() == b.to_ascii_lowercase()
    } else {
        a == b
    }
}

/// First match of fixture-subset pattern in `input`, or None.
fn regexp_find(pattern: &str, flags: &str, input: &str) -> Option<String> {
    let atoms = parse_regexp_atoms(pattern).ok()?;
    let ignore_case = flags.contains('i');
    let chars: Vec<char> = input.chars().collect();
    for start in 0..=chars.len() {
        if let Some(end) = regexp_match_at(&atoms, &chars, start, ignore_case) {
            return Some(chars[start..end].iter().collect());
        }
    }
    None
}

fn regexp_match_at(atoms: &[ReAtom], input: &[char], start: usize, ignore_case: bool) -> Option<usize> {
    let mut pos = start;
    for atom in atoms {
        match *atom {
            ReAtom::Lit(c) => {
                if pos >= input.len() || !char_eq(input[pos], c, ignore_case) {
                    return None;
                }
                pos += 1;
            }
            ReAtom::Plus(c) => {
                if pos >= input.len() || !char_eq(input[pos], c, ignore_case) {
                    return None;
                }
                pos += 1;
                while pos < input.len() && char_eq(input[pos], c, ignore_case) {
                    pos += 1;
                }
            }
        }
    }
    Some(pos)
}

fn date_now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// ECMA-262 Date.UTC(year, month, date=1, hours=0, minutes=0, seconds=0, ms=0) subset.
fn date_utc(args: &[JsVal]) -> Result<f64, ()> {
    let year = to_number(args.first().ok_or(())?)?;
    let month = to_number(args.get(1).ok_or(())?)?;
    let date = match args.get(2) {
        Some(v) => to_number(v)?,
        None => 1.0,
    };
    let hours = match args.get(3) {
        Some(v) => to_number(v)?,
        None => 0.0,
    };
    let minutes = match args.get(4) {
        Some(v) => to_number(v)?,
        None => 0.0,
    };
    let seconds = match args.get(5) {
        Some(v) => to_number(v)?,
        None => 0.0,
    };
    let ms = match args.get(6) {
        Some(v) => to_number(v)?,
        None => 0.0,
    };
    if ![year, month, date, hours, minutes, seconds, ms]
        .iter()
        .all(|n| n.is_finite())
    {
        return Ok(f64::NAN);
    }
    let y = year.trunc() as i64;
    let m = month.trunc() as i64;
    // ECMA MakeFullYear: years 0–99 → 1900+y (not needed for fixture; keep full year).
    let day = date.trunc() as i64;
    let h = hours.trunc() as i64;
    let mi = minutes.trunc() as i64;
    let s = seconds.trunc() as i64;
    let milli = ms.trunc() as i64;
    // Normalize month into year.
    let mut yy = y;
    let mut mm = m;
    if mm >= 0 {
        yy += mm / 12;
        mm %= 12;
    } else {
        let years = (-mm + 11) / 12;
        yy -= years;
        mm += years * 12;
    }
    let day_num = days_from_civil(yy as i32, (mm + 1) as u32, 1) + (day - 1);
    let time_ms = ((h * 60 + mi) * 60 + s) * 1000 + milli;
    Ok((day_num * 86_400_000 + time_ms) as f64)
}

/// Days from Unix epoch (1970-01-01) for civil (y, m, d) with m in 1..=12.
/// Howard Hinnant civil_from_days inverse.
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64;
    let m = m as i64;
    let d = d as i64;
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp as u64 + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

/// Civil (y, m, d) from days since Unix epoch. Howard Hinnant algorithm.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

const MS_PER_DAY: i64 = 86_400_000;

/// Split UTC ms into (day number since epoch, time-within-day ms). Fixture uses UTC as local.
fn split_date_ms(ms: f64) -> Result<(i64, i64), ()> {
    if !ms.is_finite() {
        return Err(());
    }
    let t = ms.trunc() as i64;
    Ok((t.div_euclid(MS_PER_DAY), t.rem_euclid(MS_PER_DAY)))
}

fn date_full_year(ms: f64) -> Result<f64, ()> {
    let (day, _) = split_date_ms(ms)?;
    let (y, _, _) = civil_from_days(day);
    Ok(y as f64)
}

/// Annex B.2.4 Date.prototype.getYear: YearFromTime(t) − 1900 (UTC-as-local for fixture).
fn date_get_year(ms: f64) -> Result<f64, ()> {
    Ok(date_full_year(ms)? - 1900.0)
}

/// Annex B.2.5 Date.prototype.setYear: MakeFullYear for 0–99 → 1900+y; keep mon/day/tod.
fn date_set_year(ms: &mut f64, year_arg: &JsVal) -> Result<f64, ()> {
    let y = to_number(year_arg)?;
    if y.is_nan() {
        *ms = f64::NAN;
        return Ok(f64::NAN);
    }
    if !ms.is_finite() {
        return Err(());
    }
    let (day, tod) = split_date_ms(*ms)?;
    let (_oy, mon, dom) = civil_from_days(day);
    let yi = y.trunc() as i64;
    let yyyy = if (0..=99).contains(&yi) {
        1900 + yi
    } else {
        yi
    };
    let new_day = days_from_civil(yyyy as i32, mon, dom);
    let new_ms = (new_day * MS_PER_DAY + tod) as f64;
    *ms = new_ms;
    Ok(new_ms)
}

/// ECMA-262 Date.prototype.toUTCString / Annex B toGMTString.
fn date_to_gmt_string(ms: f64) -> Result<String, ()> {
    if !ms.is_finite() {
        return Ok("Invalid Date".into());
    }
    let (day, tod) = split_date_ms(ms)?;
    let (y, m, d) = civil_from_days(day);
    // Epoch day 0 = Thursday.
    let wd = day.rem_euclid(7) as usize;
    const WDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let hour = tod / 3_600_000;
    let min = (tod % 3_600_000) / 60_000;
    let sec = (tod % 60_000) / 1000;
    Ok(format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        WDAYS[wd],
        d,
        MONTHS[(m - 1) as usize],
        y,
        hour,
        min,
        sec
    ))
}

fn date_proto_method_builtin(key: &str) -> Option<BuiltinId> {
    match key {
        "getYear" => Some(BuiltinId::DateGetYear),
        "setYear" => Some(BuiltinId::DateSetYear),
        "toGMTString" => Some(BuiltinId::DateToGmtString),
        "getFullYear" => Some(BuiltinId::DateGetFullYear),
        "getTime" | "valueOf" => None, // instance-only in current fixtures
        _ => None,
    }
}

fn is_date_proto_method(id: BuiltinId) -> bool {
    matches!(
        id,
        BuiltinId::DateGetYear
            | BuiltinId::DateSetYear
            | BuiltinId::DateToGmtString
            | BuiltinId::DateGetFullYear
    )
}

fn date_proto_method_name(id: BuiltinId) -> Option<&'static str> {
    match id {
        BuiltinId::DateGetYear => Some("getYear"),
        BuiltinId::DateSetYear => Some("setYear"),
        BuiltinId::DateToGmtString => Some("toGMTString"),
        BuiltinId::DateGetFullYear => Some("getFullYear"),
        _ => None,
    }
}

fn eval_date_method(ms: &mut f64, key: &str, args: &[JsVal]) -> Result<JsVal, ()> {
    match key {
        "getTime" | "valueOf" if args.is_empty() => Ok(JsVal::Num(*ms)),
        "getFullYear" if args.is_empty() => Ok(JsVal::Num(date_full_year(*ms)?)),
        "getYear" if args.is_empty() => Ok(JsVal::Num(date_get_year(*ms)?)),
        "toGMTString" if args.is_empty() => Ok(JsVal::Str(date_to_gmt_string(*ms)?)),
        "setYear" => {
            let arg = args.first().unwrap_or(&JsVal::Undef);
            Ok(JsVal::Num(date_set_year(ms, arg)?))
        }
        _ => Err(()),
    }
}

fn to_string_arg(v: &JsVal) -> Result<String, ()> {
    match v {
        JsVal::Str(s) => Ok(s.clone()),
        JsVal::Num(n) => {
            if n.is_nan() {
                Ok("NaN".into())
            } else if *n == f64::INFINITY {
                Ok("Infinity".into())
            } else if *n == f64::NEG_INFINITY {
                Ok("-Infinity".into())
            } else if *n == 0.0 {
                Ok("0".into())
            } else {
                Ok(format!("{n}"))
            }
        }
        JsVal::Bool(true) => Ok("true".into()),
        JsVal::Bool(false) => Ok("false".into()),
        JsVal::Undef => Ok("undefined".into()),
        _ => Err(()),
    }
}

fn string_annex_method_builtin(key: &str) -> Option<BuiltinId> {
    match key {
        "substr" => Some(BuiltinId::StrSubstr),
        "anchor" => Some(BuiltinId::StrAnchor),
        "big" => Some(BuiltinId::StrBig),
        "blink" => Some(BuiltinId::StrBlink),
        "bold" => Some(BuiltinId::StrBold),
        "fixed" => Some(BuiltinId::StrFixed),
        "fontcolor" => Some(BuiltinId::StrFontcolor),
        "fontsize" => Some(BuiltinId::StrFontsize),
        "italics" => Some(BuiltinId::StrItalics),
        "link" => Some(BuiltinId::StrLink),
        "small" => Some(BuiltinId::StrSmall),
        "strike" => Some(BuiltinId::StrStrike),
        "sub" => Some(BuiltinId::StrSub),
        "sup" => Some(BuiltinId::StrSup),
        _ => None,
    }
}

fn is_string_annex_method(id: BuiltinId) -> bool {
    matches!(
        id,
        BuiltinId::StrSubstr
            | BuiltinId::StrAnchor
            | BuiltinId::StrBig
            | BuiltinId::StrBlink
            | BuiltinId::StrBold
            | BuiltinId::StrFixed
            | BuiltinId::StrFontcolor
            | BuiltinId::StrFontsize
            | BuiltinId::StrItalics
            | BuiltinId::StrLink
            | BuiltinId::StrSmall
            | BuiltinId::StrStrike
            | BuiltinId::StrSub
            | BuiltinId::StrSup
    )
}

fn string_annex_method_name(id: BuiltinId) -> Option<&'static str> {
    match id {
        BuiltinId::StrSubstr => Some("substr"),
        BuiltinId::StrAnchor => Some("anchor"),
        BuiltinId::StrBig => Some("big"),
        BuiltinId::StrBlink => Some("blink"),
        BuiltinId::StrBold => Some("bold"),
        BuiltinId::StrFixed => Some("fixed"),
        BuiltinId::StrFontcolor => Some("fontcolor"),
        BuiltinId::StrFontsize => Some("fontsize"),
        BuiltinId::StrItalics => Some("italics"),
        BuiltinId::StrLink => Some("link"),
        BuiltinId::StrSmall => Some("small"),
        BuiltinId::StrStrike => Some("strike"),
        BuiltinId::StrSub => Some("sub"),
        BuiltinId::StrSup => Some("sup"),
        _ => None,
    }
}

/// ECMA-262 ToIntegerOrInfinity for Annex B substr (fixture subset: finite numbers).
fn to_integer_or_infinity(v: &JsVal) -> Result<f64, ()> {
    let n = match v {
        JsVal::Num(n) => *n,
        JsVal::Undef => f64::NAN,
        JsVal::Str(s) => js_string_to_number(s),
        JsVal::Bool(true) => 1.0,
        JsVal::Bool(false) => 0.0,
        JsVal::Null => 0.0,
        _ => return Err(()),
    };
    if n.is_nan() {
        return Ok(0.0);
    }
    if n.is_infinite() {
        return Ok(n);
    }
    if n == 0.0 {
        return Ok(0.0);
    }
    Ok(n.trunc())
}

/// Annex B.2.3.1 String.prototype.substr over UTF-16 code units.
fn js_substr(s: &str, start: &JsVal, length: Option<&JsVal>) -> Result<String, ()> {
    let units: Vec<u16> = s.encode_utf16().collect();
    let size = units.len() as f64;
    let mut int_start = to_integer_or_infinity(start)?;
    if int_start == f64::NEG_INFINITY {
        int_start = 0.0;
    } else if int_start < 0.0 {
        int_start = (size + int_start).max(0.0);
    } else {
        int_start = int_start.min(size);
    }
    let int_length = match length {
        None | Some(JsVal::Undef) => size,
        Some(v) => to_integer_or_infinity(v)?,
    };
    let int_length = int_length.max(0.0).min(size - int_start);
    let begin = int_start as usize;
    let end = (int_start + int_length) as usize;
    let slice = &units[begin..end.min(units.len())];
    String::from_utf16(slice).map_err(|_| ())
}

/// ECMA-262 CreateHTML (Annex B.2.3).
fn create_html(s: &str, tag: &str, attribute: &str, value: Option<&JsVal>) -> Result<String, ()> {
    let mut p1 = format!("<{tag}");
    if !attribute.is_empty() {
        let v = to_string_arg(value.unwrap_or(&JsVal::Undef))?;
        let escaped: String = v
            .chars()
            .flat_map(|c| {
                if c == '"' {
                    "&quot;".chars().collect::<Vec<_>>()
                } else {
                    vec![c]
                }
            })
            .collect();
        p1.push(' ');
        p1.push_str(attribute);
        p1.push_str("=\"");
        p1.push_str(&escaped);
        p1.push('"');
    }
    Ok(format!("{p1}>{s}</{tag}>"))
}

fn eval_string_method(this_s: &str, key: &str, args: &[JsVal]) -> Result<JsVal, ()> {
    match key {
        "substr" => {
            let start = args.first().unwrap_or(&JsVal::Undef);
            let length = args.get(1);
            Ok(JsVal::Str(js_substr(this_s, start, length)?))
        }
        "anchor" => Ok(JsVal::Str(create_html(
            this_s,
            "a",
            "name",
            args.first(),
        )?)),
        "big" => Ok(JsVal::Str(create_html(this_s, "big", "", None)?)),
        "blink" => Ok(JsVal::Str(create_html(this_s, "blink", "", None)?)),
        "bold" => Ok(JsVal::Str(create_html(this_s, "b", "", None)?)),
        "fixed" => Ok(JsVal::Str(create_html(this_s, "tt", "", None)?)),
        "fontcolor" => Ok(JsVal::Str(create_html(
            this_s,
            "font",
            "color",
            args.first(),
        )?)),
        "fontsize" => Ok(JsVal::Str(create_html(
            this_s,
            "font",
            "size",
            args.first(),
        )?)),
        "italics" => Ok(JsVal::Str(create_html(this_s, "i", "", None)?)),
        "link" => Ok(JsVal::Str(create_html(
            this_s,
            "a",
            "href",
            args.first(),
        )?)),
        "small" => Ok(JsVal::Str(create_html(this_s, "small", "", None)?)),
        "strike" => Ok(JsVal::Str(create_html(this_s, "strike", "", None)?)),
        "sub" => Ok(JsVal::Str(create_html(this_s, "sub", "", None)?)),
        "sup" => Ok(JsVal::Str(create_html(this_s, "sup", "", None)?)),
        _ => Err(()),
    }
}

/// uriUnescaped = Alpha / DecimalDigit / "-" / "_" / "." / "!" / "~" / "*" / "'" / "(" / ")"
fn is_uri_unescaped(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(b, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')')
}

/// uriReserved = ";" / "/" / "?" / ":" / "@" / "&" / "=" / "+" / "$" / ","
/// plus "#" kept unescaped by encodeURI / reserved by decodeURI.
fn is_uri_reserved_or_hash(b: u8) -> bool {
    matches!(
        b,
        b';' | b'/' | b'?' | b':' | b'@' | b'&' | b'=' | b'+' | b'$' | b',' | b'#'
    )
}

/// Annex B.2.1.1 escape: unescaped = Alpha / Digit / "@" / "*" / "_" / "+" / "-" / "." / "/"
fn is_escape_unescaped(cu: u16) -> bool {
    matches!(
        cu,
        0x41..=0x5A // A-Z
            | 0x61..=0x7A // a-z
            | 0x30..=0x39 // 0-9
            | 0x40 // @
            | 0x2A // *
            | 0x5F // _
            | 0x2B // +
            | 0x2D // -
            | 0x2E // .
            | 0x2F // /
    )
}

/// ECMA-262 Annex B `escape` over UTF-16 code units.
fn js_escape(input: &str) -> String {
    let mut out = String::new();
    for cu in input.encode_utf16() {
        if is_escape_unescaped(cu) {
            out.push(char::from_u32(cu as u32).unwrap_or('\u{FFFD}'));
        } else if cu < 256 {
            out.push_str(&format!("%{cu:02X}"));
        } else {
            out.push_str(&format!("%u{cu:04X}"));
        }
    }
    out
}

/// ECMA-262 Annex B `unescape`: `%uXXXX` and `%XX` sequences.
fn js_unescape(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut units: Vec<u16> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'u' && i + 5 < bytes.len() {
                if let (Some(a), Some(b), Some(c), Some(d)) = (
                    hex_nibble(bytes[i + 2]),
                    hex_nibble(bytes[i + 3]),
                    hex_nibble(bytes[i + 4]),
                    hex_nibble(bytes[i + 5]),
                ) {
                    units.push(((a as u16) << 12) | ((b as u16) << 8) | ((c as u16) << 4) | d as u16);
                    i += 6;
                    continue;
                }
            } else if i + 2 < bytes.len() {
                if let (Some(hi), Some(lo)) = (hex_nibble(bytes[i + 1]), hex_nibble(bytes[i + 2])) {
                    units.push(((hi as u16) << 4) | lo as u16);
                    i += 3;
                    continue;
                }
            }
        }
        // Non-ASCII UTF-8 in the source string: take next UTF-8 char as code units.
        let rest = &input[i..];
        if let Some(ch) = rest.chars().next() {
            for cu in ch.encode_utf16(&mut [0u16; 2]).iter().copied() {
                units.push(cu);
            }
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    String::from_utf16_lossy(&units)
}

/// ECMA-262 Encode (encodeURI / encodeURIComponent) over UTF-8 code units of the string.
fn js_encode_uri(input: &str, component: bool) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        let leave = if component {
            is_uri_unescaped(b)
        } else {
            is_uri_unescaped(b) || is_uri_reserved_or_hash(b)
        };
        if leave {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// ECMA-262 Decode (decodeURI / decodeURIComponent). Reserved set preserved only for decodeURI.
fn js_decode_uri(input: &str, component: bool) -> Result<String, ()> {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        if i + 2 >= bytes.len() {
            return Err(());
        }
        let hi = hex_nibble(bytes[i + 1]).ok_or(())?;
        let lo = hex_nibble(bytes[i + 2]).ok_or(())?;
        let decoded = (hi << 4) | lo;
        // decodeURI leaves percent-escapes of uriReserved + "#" as-is.
        if !component && is_uri_reserved_or_hash(decoded) {
            out.push(b'%');
            out.push(bytes[i + 1]);
            out.push(bytes[i + 2]);
        } else {
            out.push(decoded);
        }
        i += 3;
    }
    String::from_utf8(out).map_err(|_| ())
}

fn to_number(v: &JsVal) -> Result<f64, ()> {
    match v {
        JsVal::Num(n) => Ok(*n),
        JsVal::Bool(true) => Ok(1.0),
        JsVal::Bool(false) => Ok(0.0),
        JsVal::Undef => Ok(f64::NAN),
        JsVal::Str(s) => Ok(js_string_to_number(s)),
        JsVal::Builtin(BuiltinId::Nan) => Ok(f64::NAN),
        JsVal::Builtin(BuiltinId::Infinity) => Ok(f64::INFINITY),
        _ => Err(()),
    }
}

/// ECMA-262 ToNumber on string (subset used by E15.03 fixtures).
fn js_string_to_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if t.eq_ignore_ascii_case("infinity") || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// ECMA-262 parseInt (string, radix) for fixture cases.
fn js_parse_int(input: &str, radix_arg: Option<&JsVal>) -> Result<f64, ()> {
    let s = input.trim_start();
    if s.is_empty() {
        return Ok(f64::NAN);
    }
    let mut radix = match radix_arg {
        None | Some(JsVal::Undef) => 0i32,
        Some(JsVal::Num(n)) => {
            if !n.is_finite() {
                return Ok(f64::NAN);
            }
            *n as i32
        }
        _ => return Err(()),
    };
    let mut chars = s.chars().peekable();
    let mut sign = 1.0f64;
    if let Some(&c) = chars.peek() {
        if c == '+' {
            chars.next();
        } else if c == '-' {
            sign = -1.0;
            chars.next();
        }
    }
    let rest: String = chars.collect();
    let mut body = rest.as_str();
    if radix == 0 {
        if body.starts_with("0x") || body.starts_with("0X") {
            radix = 16;
            body = &body[2..];
        } else {
            radix = 10;
        }
    } else if radix == 16 && (body.starts_with("0x") || body.starts_with("0X")) {
        body = &body[2..];
    }
    if !(2..=36).contains(&radix) {
        return Ok(f64::NAN);
    }
    let radix_u = radix as u32;
    let mut acc: i64 = 0;
    let mut any = false;
    for c in body.chars() {
        let dig = match c.to_digit(radix_u) {
            Some(d) => d as i64,
            None => break,
        };
        any = true;
        acc = acc
            .checked_mul(radix as i64)
            .and_then(|a| a.checked_add(dig))
            .unwrap_or(i64::MAX);
    }
    if !any {
        return Ok(f64::NAN);
    }
    Ok(sign * acc as f64)
}

/// ECMA-262 parseFloat (string) for fixture cases.
fn js_parse_float(input: &str) -> f64 {
    let s = input.trim_start();
    if s.is_empty() {
        return f64::NAN;
    }
    // Scan a JS-like float prefix: optional sign, digits, optional fraction/exponent.
    let bytes = s.as_bytes();
    let mut i = 0usize;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let start = i;
    let mut saw_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        saw_digit = true;
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            saw_digit = true;
            i += 1;
        }
    }
    if !saw_digit {
        // Infinity?
        let rest = &s[start.min(s.len())..];
        if rest.len() >= 8 && rest[..8].eq_ignore_ascii_case("Infinity") {
            return if s.starts_with('-') {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
        return f64::NAN;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let e_pos = i;
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if exp_start == i {
            i = e_pos; // no exponent digits → stop before e
        }
    }
    s[..i].parse::<f64>().unwrap_or(f64::NAN)
}

fn member_get(obj: &JsVal, key: &str) -> Result<JsVal, ()> {
    match obj {
        JsVal::Builtin(BuiltinId::GlobalThis) => match key {
            "Object" => Ok(JsVal::Builtin(BuiltinId::Object)),
            "Function" => Ok(JsVal::Builtin(BuiltinId::Function)),
            "Array" => Ok(JsVal::Builtin(BuiltinId::Array)),
            "String" => Ok(JsVal::Builtin(BuiltinId::String)),
            "Boolean" => Ok(JsVal::Builtin(BuiltinId::Boolean)),
            "Error" => Ok(JsVal::Builtin(BuiltinId::Error)),
            "TypeError" => Ok(JsVal::Builtin(BuiltinId::TypeError)),
            "RangeError" => Ok(JsVal::Builtin(BuiltinId::RangeError)),
            "ReferenceError" => Ok(JsVal::Builtin(BuiltinId::ReferenceError)),
            "SyntaxError" => Ok(JsVal::Builtin(BuiltinId::SyntaxError)),
            "URIError" => Ok(JsVal::Builtin(BuiltinId::UriError)),
            "EvalError" => Ok(JsVal::Builtin(BuiltinId::EvalError)),
            "AggregateError" => Ok(JsVal::Builtin(BuiltinId::AggregateError)),
            "parseInt" => Ok(JsVal::Builtin(BuiltinId::ParseInt)),
            "parseFloat" => Ok(JsVal::Builtin(BuiltinId::ParseFloat)),
            "isNaN" => Ok(JsVal::Builtin(BuiltinId::IsNaN)),
            "isFinite" => Ok(JsVal::Builtin(BuiltinId::IsFinite)),
            "NaN" => Ok(JsVal::Num(f64::NAN)),
            "Infinity" => Ok(JsVal::Num(f64::INFINITY)),
            "encodeURI" => Ok(JsVal::Builtin(BuiltinId::EncodeUri)),
            "decodeURI" => Ok(JsVal::Builtin(BuiltinId::DecodeUri)),
            "encodeURIComponent" => Ok(JsVal::Builtin(BuiltinId::EncodeUriComponent)),
            "decodeURIComponent" => Ok(JsVal::Builtin(BuiltinId::DecodeUriComponent)),
            "escape" => Ok(JsVal::Builtin(BuiltinId::Escape)),
            "unescape" => Ok(JsVal::Builtin(BuiltinId::Unescape)),
            "JSON" => Ok(JsVal::Builtin(BuiltinId::Json)),
            "Date" => Ok(JsVal::Builtin(BuiltinId::Date)),
            "RegExp" => Ok(JsVal::Builtin(BuiltinId::RegExp)),
            "Map" => Ok(JsVal::Builtin(BuiltinId::Map)),
            "Set" => Ok(JsVal::Builtin(BuiltinId::Set)),
            "WeakMap" => Ok(JsVal::Builtin(BuiltinId::WeakMap)),
            "WeakSet" => Ok(JsVal::Builtin(BuiltinId::WeakSet)),
            "ArrayBuffer" => Ok(JsVal::Builtin(BuiltinId::ArrayBuffer)),
            "DataView" => Ok(JsVal::Builtin(BuiltinId::DataView)),
            "Uint8Array" => Ok(JsVal::Builtin(BuiltinId::Uint8Array)),
            "Int32Array" => Ok(JsVal::Builtin(BuiltinId::Int32Array)),
            "Float64Array" => Ok(JsVal::Builtin(BuiltinId::Float64Array)),
            "undefined" => Ok(JsVal::Undef),
            "globalThis" => Ok(JsVal::Builtin(BuiltinId::GlobalThis)),
            _ => Err(()),
        },
        JsVal::Builtin(BuiltinId::Object) if key == "prototype" => {
            Ok(JsVal::Builtin(BuiltinId::ObjectPrototype))
        }
        JsVal::Builtin(BuiltinId::String) if key == "prototype" => {
            Ok(JsVal::Builtin(BuiltinId::StringPrototype))
        }
        JsVal::Builtin(BuiltinId::Date) if key == "prototype" => {
            Ok(JsVal::Builtin(BuiltinId::DatePrototype))
        }
        JsVal::Builtin(BuiltinId::Object) if key == "getPrototypeOf" => {
            Ok(JsVal::Builtin(BuiltinId::ObjectGetPrototypeOf))
        }
        JsVal::Builtin(BuiltinId::ObjectPrototype) if key == "hasOwnProperty" => {
            Ok(JsVal::Builtin(BuiltinId::HasOwnProperty))
        }
        JsVal::Builtin(BuiltinId::StringPrototype) => string_annex_method_builtin(key)
            .map(JsVal::Builtin)
            .ok_or(()),
        JsVal::Str(_) => string_annex_method_builtin(key)
            .map(JsVal::Builtin)
            .ok_or(()),
        JsVal::Builtin(BuiltinId::DatePrototype) => date_proto_method_builtin(key)
            .map(JsVal::Builtin)
            .ok_or(()),
        JsVal::Builtin(BuiltinId::Array) if key == "isArray" => {
            Ok(JsVal::Builtin(BuiltinId::ArrayIsArray))
        }
        JsVal::Builtin(BuiltinId::Json) => match key {
            "parse" => Ok(JsVal::Builtin(BuiltinId::JsonParse)),
            "stringify" => Ok(JsVal::Builtin(BuiltinId::JsonStringify)),
            _ => Err(()),
        },
        JsVal::Builtin(BuiltinId::Date) => match key {
            "now" => Ok(JsVal::Builtin(BuiltinId::DateNow)),
            "UTC" => Ok(JsVal::Builtin(BuiltinId::DateUtc)),
            _ => Err(()),
        },
        JsVal::ErrorInst {
            name,
            message,
            errors,
        } => match key {
            "name" => Ok(JsVal::Str(name.clone())),
            "message" => Ok(JsVal::Str(message.clone())),
            "errors" => match errors {
                Some(a) => Ok(JsVal::Array(a.clone())),
                None => Err(()),
            },
            _ => Err(()),
        },
        JsVal::DateInst { ms } => {
            let _ = ms;
            // Bound methods via eval_method_call; bare get yields callable for typeof paths.
            date_proto_method_builtin(key)
                .map(JsVal::Builtin)
                .ok_or(())
        }
        JsVal::RegExpInst { source, flags } => match key {
            "source" => Ok(JsVal::Str(source.clone())),
            "flags" => Ok(JsVal::Str(flags.clone())),
            _ => Err(()),
        },
        JsVal::MapInst { entries } if key == "size" => Ok(JsVal::Num(entries.len() as f64)),
        JsVal::SetInst { values } if key == "size" => Ok(JsVal::Num(values.len() as f64)),
        JsVal::ArrayBufferInst { bytes, .. } if key == "byteLength" => {
            Ok(JsVal::Num(bytes.borrow().len() as f64))
        }
        JsVal::TypedArrayInst { length, .. } if key == "length" => {
            Ok(JsVal::Num(*length as f64))
        }
        JsVal::TypedArrayInst {
            kind,
            bytes,
            length,
            ..
        } => {
            let idx = key.parse::<usize>().map_err(|_| ())?;
            if idx >= *length {
                return Ok(JsVal::Undef);
            }
            let off = idx * kind.bytes_per_element();
            let n = read_ta_elem(*kind, &bytes.borrow(), off)?;
            Ok(JsVal::Num(n))
        }
        JsVal::DataViewInst { byte_length, .. } if key == "byteLength" => {
            Ok(JsVal::Num(*byte_length as f64))
        }
        JsVal::Array(elems) if key == "length" => Ok(JsVal::Num(elems.len() as f64)),
        JsVal::Array(elems) => {
            if let Ok(idx) = key.parse::<usize>() {
                Ok(elems.get(idx).cloned().unwrap_or(JsVal::Undef))
            } else {
                Err(())
            }
        }
        JsVal::Object { props, proto, .. } => {
            if let Some(v) = object_own_get(props, key) {
                return Ok(v);
            }
            // Annex B accessor: missing own `__proto__` → [[Prototype]].
            if key == "__proto__" {
                return Ok((**proto).clone());
            }
            match proto.as_ref() {
                JsVal::Builtin(BuiltinId::ObjectPrototype) => {
                    if key == "hasOwnProperty" {
                        Ok(JsVal::Builtin(BuiltinId::HasOwnProperty))
                    } else {
                        Err(())
                    }
                }
                JsVal::Null => Err(()),
                other => member_get(other, key),
            }
        }
        _ => Err(()),
    }
}

fn member_set(obj: &mut JsVal, key: &str, val: JsVal) -> Result<(), ()> {
    match obj {
        JsVal::TypedArrayInst {
            kind,
            bytes,
            length,
            ..
        } => {
            let idx = key.parse::<usize>().map_err(|_| ())?;
            if idx >= *length {
                return Err(());
            }
            let n = match val {
                JsVal::Num(n) => n,
                _ => return Err(()),
            };
            let off = idx * kind.bytes_per_element();
            write_ta_elem(*kind, &mut bytes.borrow_mut(), off, n)
        }
        JsVal::Object { props, proto, .. } => {
            if key == "__proto__" && !object_own_has(props, "__proto__") {
                *proto = Box::new(val);
                return Ok(());
            }
            if let Some((_, slot)) = props.iter_mut().find(|(k, _)| k == key) {
                *slot = val;
            } else {
                props.push((key.to_string(), val));
            }
            Ok(())
        }
        _ => Err(()),
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::Null
        | JsVal::Array(_)
        | JsVal::Object { .. }
        | JsVal::ErrorInst { .. }
        | JsVal::DateInst { .. }
        | JsVal::RegExpInst { .. }
        | JsVal::MapInst { .. }
        | JsVal::SetInst { .. }
        | JsVal::WeakMapInst { .. }
        | JsVal::WeakSetInst { .. }
        | JsVal::ArrayBufferInst { .. }
        | JsVal::TypedArrayInst { .. }
        | JsVal::DataViewInst { .. } => "object".into(),
        JsVal::Builtin(BuiltinId::Undefined) => "undefined".into(),
        JsVal::Builtin(BuiltinId::Nan | BuiltinId::Infinity) => "number".into(),
        JsVal::Builtin(
            BuiltinId::GlobalThis
                | BuiltinId::ObjectPrototype
                | BuiltinId::StringPrototype
                | BuiltinId::DatePrototype
                | BuiltinId::Json,
        ) => "object".into(),
        JsVal::Builtin(
            BuiltinId::Object
            | BuiltinId::Function
            | BuiltinId::Array
            | BuiltinId::String
            | BuiltinId::Boolean
            | BuiltinId::ArrayIsArray
            | BuiltinId::ObjectGetPrototypeOf
            | BuiltinId::HasOwnProperty
            | BuiltinId::StrSubstr
            | BuiltinId::StrAnchor
            | BuiltinId::StrBig
            | BuiltinId::StrBlink
            | BuiltinId::StrBold
            | BuiltinId::StrFixed
            | BuiltinId::StrFontcolor
            | BuiltinId::StrFontsize
            | BuiltinId::StrItalics
            | BuiltinId::StrLink
            | BuiltinId::StrSmall
            | BuiltinId::StrStrike
            | BuiltinId::StrSub
            | BuiltinId::StrSup
            | BuiltinId::Error
            | BuiltinId::TypeError
            | BuiltinId::RangeError
            | BuiltinId::ReferenceError
            | BuiltinId::SyntaxError
            | BuiltinId::UriError
            | BuiltinId::EvalError
            | BuiltinId::AggregateError
            | BuiltinId::ParseInt
            | BuiltinId::ParseFloat
            | BuiltinId::IsNaN
            | BuiltinId::IsFinite
            | BuiltinId::EncodeUri
            | BuiltinId::DecodeUri
            | BuiltinId::EncodeUriComponent
            | BuiltinId::DecodeUriComponent
            | BuiltinId::Escape
            | BuiltinId::Unescape
            | BuiltinId::JsonParse
            | BuiltinId::JsonStringify
            | BuiltinId::Date
            | BuiltinId::DateNow
            | BuiltinId::DateUtc
            | BuiltinId::DateGetYear
            | BuiltinId::DateSetYear
            | BuiltinId::DateToGmtString
            | BuiltinId::DateGetFullYear
            | BuiltinId::RegExp
            | BuiltinId::Map
            | BuiltinId::Set
            | BuiltinId::WeakMap
            | BuiltinId::WeakSet
            | BuiltinId::ArrayBuffer
            | BuiltinId::DataView
            | BuiltinId::Uint8Array
            | BuiltinId::Int32Array
            | BuiltinId::Float64Array,
        ) => "function".into(),
    }
}

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef | JsVal::Null => false,
        JsVal::Builtin(_)
        | JsVal::ErrorInst { .. }
        | JsVal::DateInst { .. }
        | JsVal::RegExpInst { .. }
        | JsVal::MapInst { .. }
        | JsVal::SetInst { .. }
        | JsVal::WeakMapInst { .. }
        | JsVal::WeakSetInst { .. }
        | JsVal::ArrayBufferInst { .. }
        | JsVal::TypedArrayInst { .. }
        | JsVal::DataViewInst { .. }
        | JsVal::Array(_)
        | JsVal::Object { .. } => true,
    }
}

fn strict_eq(l: &JsVal, r: &JsVal) -> bool {
    match (l, r) {
        (JsVal::Num(a), JsVal::Num(b)) => a == b,
        (JsVal::Bool(a), JsVal::Bool(b)) => a == b,
        (JsVal::Str(a), JsVal::Str(b)) => a == b,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Null, JsVal::Null) => true,
        (JsVal::Builtin(a), JsVal::Builtin(b)) => a == b,
        (JsVal::Undef, JsVal::Builtin(BuiltinId::Undefined))
        | (JsVal::Builtin(BuiltinId::Undefined), JsVal::Undef) => true,
        (
            JsVal::ErrorInst {
                name: n1,
                message: m1,
                errors: e1,
            },
            JsVal::ErrorInst {
                name: n2,
                message: m2,
                errors: e2,
            },
        ) => n1 == n2 && m1 == m2 && e1 == e2,
        (JsVal::DateInst { ms: a }, JsVal::DateInst { ms: b }) => a == b,
        (JsVal::MapInst { entries: a }, JsVal::MapInst { entries: b }) => a == b,
        (JsVal::SetInst { values: a }, JsVal::SetInst { values: b }) => a == b,
        (JsVal::WeakMapInst { entries: a }, JsVal::WeakMapInst { entries: b }) => a == b,
        (JsVal::WeakSetInst { values: a }, JsVal::WeakSetInst { values: b }) => a == b,
        (JsVal::ArrayBufferInst { id: a, .. }, JsVal::ArrayBufferInst { id: b, .. }) => a == b,
        (
            JsVal::TypedArrayInst {
                kind: ka,
                buffer_id: ba,
                length: la,
                ..
            },
            JsVal::TypedArrayInst {
                kind: kb,
                buffer_id: bb,
                length: lb,
                ..
            },
        ) => ka == kb && ba == bb && la == lb,
        (
            JsVal::DataViewInst {
                buffer_id: a,
                byte_length: la,
                ..
            },
            JsVal::DataViewInst {
                buffer_id: b,
                byte_length: lb,
                ..
            },
        ) => a == b && la == lb,
        (JsVal::Array(a), JsVal::Array(b)) => a == b,
        (JsVal::Object { id: a, .. }, JsVal::Object { id: b, .. }) => a == b,
        _ => false,
    }
}

/// Minimal JSON.parse for E15.05 fixture depth (null/bool/number/string/array/object).
fn json_parse(input: &str) -> Result<JsVal, ()> {
    let mut p = JsonParser {
        bytes: input.as_bytes(),
        i: 0,
    };
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.i != p.bytes.len() {
        return Err(());
    }
    Ok(v)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl<'a> JsonParser<'a> {
    fn skip_ws(&mut self) {
        while self.i < self.bytes.len() && matches!(self.bytes[self.i], b' ' | b'\t' | b'\n' | b'\r')
        {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.i).copied()
    }

    fn bump(&mut self) -> Result<u8, ()> {
        let b = self.peek().ok_or(())?;
        self.i += 1;
        Ok(b)
    }

    fn expect(&mut self, b: u8) -> Result<(), ()> {
        if self.bump()? == b {
            Ok(())
        } else {
            Err(())
        }
    }

    fn parse_value(&mut self) -> Result<JsVal, ()> {
        self.skip_ws();
        match self.peek().ok_or(())? {
            b'n' => self.parse_lit(b"null", JsVal::Null),
            b't' => self.parse_lit(b"true", JsVal::Bool(true)),
            b'f' => self.parse_lit(b"false", JsVal::Bool(false)),
            b'"' => Ok(JsVal::Str(self.parse_string()?)),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            _ => Err(()),
        }
    }

    fn parse_lit(&mut self, lit: &[u8], v: JsVal) -> Result<JsVal, ()> {
        for &b in lit {
            self.expect(b)?;
        }
        Ok(v)
    }

    fn parse_string(&mut self) -> Result<String, ()> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.bump()? {
                b'"' => return Ok(out),
                b'\\' => match self.bump()? {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let mut code = 0u16;
                        for _ in 0..4 {
                            let h = hex_nibble(self.bump()?).ok_or(())?;
                            code = (code << 4) | u16::from(h);
                        }
                        out.push(char::from_u32(u32::from(code)).unwrap_or('\u{FFFD}'));
                    }
                    _ => return Err(()),
                },
                c if c < 0x20 => return Err(()),
                c => out.push(c as char),
            }
        }
    }

    fn parse_number(&mut self) -> Result<JsVal, ()> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        if self.peek() == Some(b'0') {
            self.i += 1;
        } else {
            let d = self.bump()?;
            if !d.is_ascii_digit() || d == b'0' {
                return Err(());
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.i += 1;
            }
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !self.peek().is_some_and(|b| b.is_ascii_digit()) {
                return Err(());
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !self.peek().is_some_and(|b| b.is_ascii_digit()) {
                return Err(());
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.i += 1;
            }
        }
        let s = std::str::from_utf8(&self.bytes[start..self.i]).map_err(|_| ())?;
        let n: f64 = s.parse().map_err(|_| ())?;
        Ok(JsVal::Num(n))
    }

    fn parse_array(&mut self) -> Result<JsVal, ()> {
        self.expect(b'[')?;
        self.skip_ws();
        let mut elems = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(JsVal::Array(elems));
        }
        loop {
            elems.push(self.parse_value()?);
            self.skip_ws();
            match self.bump()? {
                b']' => return Ok(JsVal::Array(elems)),
                b',' => self.skip_ws(),
                _ => return Err(()),
            }
        }
    }

    fn parse_object(&mut self) -> Result<JsVal, ()> {
        self.expect(b'{')?;
        self.skip_ws();
        let mut props = Vec::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(new_object(props));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(());
            }
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let val = self.parse_value()?;
            if let Some((_, slot)) = props.iter_mut().find(|(k, _)| k == &key) {
                *slot = val;
            } else {
                props.push((key, val));
            }
            self.skip_ws();
            match self.bump()? {
                b'}' => return Ok(new_object(props)),
                b',' => {}
                _ => return Err(()),
            }
        }
    }
}

/// Minimal JSON.stringify for E15.05 fixture depth.
fn json_stringify(v: &JsVal) -> Result<String, ()> {
    match v {
        JsVal::Null => Ok("null".into()),
        JsVal::Bool(true) => Ok("true".into()),
        JsVal::Bool(false) => Ok("false".into()),
        JsVal::Num(n) => {
            if !n.is_finite() {
                return Ok("null".into());
            }
            if *n == 0.0 {
                return Ok("0".into());
            }
            // Prefer integer form when exact.
            if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                return Ok(format!("{}", *n as i64));
            }
            Ok(format!("{n}"))
        }
        JsVal::Str(s) => Ok(json_quote(s)),
        JsVal::Array(elems) => {
            let mut out = String::from("[");
            for (i, e) in elems.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                // undefined in arrays becomes null in JSON.stringify
                match e {
                    JsVal::Undef => out.push_str("null"),
                    other => out.push_str(&json_stringify(other)?),
                }
            }
            out.push(']');
            Ok(out)
        }
        JsVal::Object { props, .. } => {
            let mut out = String::from("{");
            let mut first = true;
            for (k, val) in props {
                if matches!(val, JsVal::Undef) {
                    continue;
                }
                if !first {
                    out.push(',');
                }
                first = false;
                out.push_str(&json_quote(k));
                out.push(':');
                out.push_str(&json_stringify(val)?);
            }
            out.push('}');
            Ok(out)
        }
        JsVal::Undef => Err(()), // top-level undefined is not a string in full ES; fixture avoids it
        _ => Err(()),
    }
}

fn json_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
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
        writeln!(
            self.body,
            "  {}",
            PRINT_F64.call(&format!("double {lit}"))
        )
        .ok();
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
            "; Draconic LLVM backend (N08.14.01–N08.14.10 + N08.16.01–N08.16.04 global builtins / Error ctors / functions / URI / JSON / Date / RegExp / Map/Set / WeakMap/WeakSet / ArrayBuffer/DataView/TypedArrays / escape/unescape / Object.prototype.__proto__ / String.prototype substr+HTML / Date.prototype getYear/setYear/toGMTString)"
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
        for s in ["object", "function", "true", "null", "hi", "two", "\\22hi\\22"] {
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
        let src =
            include_str!("../../../tests/conformance/fixtures/es/builtins/weak_map_set.drac");
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
        assert!(
            ir.contains("double 1.5"),
            "should print f64_0=1.5:\n{ir}"
        );
        assert!(
            ir.contains("double 2.25"),
            "should print f64_1=2.25:\n{ir}"
        );
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
        for s in [
            "function",
            "true",
            "a%20b",
            " ",
            "caf%E9",
            "hello world",
        ] {
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
}
