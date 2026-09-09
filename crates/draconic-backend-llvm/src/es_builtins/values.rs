use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::Interp;

use draconic_ast::JsString;
use draconic_ir::{LocalId, Stmt};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum BuiltinId {
    Undefined,
    GlobalThis,
    Object,
    Function,
    FunctionPrototype,
    Array,
    String,
    StringPrototype,
    Boolean,
    ObjectPrototype,
    ObjectGetPrototypeOf,
    HasOwnProperty,
    /// Annex B Object.prototype accessor legacy (unbound; `.call` supplies this).
    DefineGetter,
    DefineSetter,
    LookupGetter,
    LookupSetter,
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
    /// ES2019 `trimStart` / Annex B `trimLeft` (same function object).
    StrTrimStart,
    /// ES2019 `trimEnd` / Annex B `trimRight` (same function object).
    StrTrimEnd,
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
    RegExpPrototype,
    /// Annex B RegExp.prototype.compile (unbound; `.call` supplies this).
    RegExpCompile,
    Map,
    Set,
    WeakMap,
    WeakSet,
    ArrayBuffer,
    DataView,
    Uint8Array,
    Int32Array,
    Float64Array,
    /// L08.01
    ParseUrl,
    /// L08.02
    ParseQuery,
    SerializeQuery,
    /// L07.01 / L07.02
    ParseFlags,
    /// L07.02
    FlagHelp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TaKind {
    U8,
    I32,
    F64,
}

impl TaKind {
    pub(super) fn bytes_per_element(self) -> usize {
        match self {
            TaKind::U8 => 1,
            TaKind::I32 => 4,
            TaKind::F64 => 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum JsVal {
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
    /// User function expression (fixture subset: simple params + body).
    /// `props` holds own data/accessors (incl. `.prototype` for constructors) — Rc so
    /// `Object.defineProperty` mutations stick across clones.
    UserFn {
        params: Vec<LocalId>,
        body: Vec<Stmt>,
        props: Rc<RefCell<Vec<(String, PropSlot)>>>,
    },
    /// Plain object: identity id + insertion-ordered string keys + [[Prototype]].
    /// `props` is Rc so `this.x = …` / defineProperty on clones share mutations.
    Object {
        id: u64,
        props: Rc<RefCell<Vec<(String, PropSlot)>>>,
        proto: Box<JsVal>,
    },
}

/// Own property: data value or accessor pair (E18.07).
#[derive(Clone, Debug, PartialEq)]
pub(super) enum PropSlot {
    Data(JsVal),
    Accessor {
        get: Option<JsVal>,
        set: Option<JsVal>,
    },
}

pub(super) fn next_object_id(interp: &Interp) -> u64 {
    interp.alloc_id()
}

pub(super) fn new_object(interp: &Interp, props: Vec<(String, PropSlot)>) -> JsVal {
    new_object_with_proto(interp, props, JsVal::Builtin(BuiltinId::ObjectPrototype))
}

pub(super) fn new_object_with_proto(
    interp: &Interp,
    props: Vec<(String, PropSlot)>,
    proto: JsVal,
) -> JsVal {
    JsVal::Object {
        id: next_object_id(interp),
        props: Rc::new(RefCell::new(props)),
        proto: Box::new(proto),
    }
}

pub(super) fn new_user_fn(interp: &Interp, params: Vec<LocalId>, body: Vec<Stmt>) -> JsVal {
    let proto_obj = new_object(interp, Vec::new());
    JsVal::UserFn {
        params,
        body,
        props: Rc::new(RefCell::new(vec![(
            "prototype".into(),
            PropSlot::Data(proto_obj),
        )])),
    }
}

pub(super) fn object_own_has(props: &[(String, PropSlot)], key: &str) -> bool {
    props.iter().any(|(k, _)| k == key)
}

pub(super) fn object_own_slot<'a>(
    props: &'a [(String, PropSlot)],
    key: &str,
) -> Option<&'a PropSlot> {
    props.iter().find(|(k, _)| k == key).map(|(_, s)| s)
}

pub(super) fn object_own_slot_mut<'a>(
    props: &'a mut [(String, PropSlot)],
    key: &str,
) -> Option<&'a mut PropSlot> {
    props.iter_mut().find(|(k, _)| k == key).map(|(_, s)| s)
}

pub(super) fn object_set_data(props: &mut Vec<(String, PropSlot)>, key: String, val: JsVal) {
    if let Some(slot) = object_own_slot_mut(props, &key) {
        *slot = PropSlot::Data(val);
    } else {
        props.push((key, PropSlot::Data(val)));
    }
}

pub(super) fn object_define_getter(
    props: &mut Vec<(String, PropSlot)>,
    key: String,
    getter: JsVal,
) {
    match object_own_slot_mut(props, &key) {
        Some(PropSlot::Accessor { get, .. }) => {
            *get = Some(getter);
        }
        Some(slot) => {
            *slot = PropSlot::Accessor {
                get: Some(getter),
                set: None,
            };
        }
        None => props.push((
            key,
            PropSlot::Accessor {
                get: Some(getter),
                set: None,
            },
        )),
    }
}

pub(super) fn object_define_setter(
    props: &mut Vec<(String, PropSlot)>,
    key: String,
    setter: JsVal,
) {
    match object_own_slot_mut(props, &key) {
        Some(PropSlot::Accessor { set, .. }) => {
            *set = Some(setter);
        }
        Some(slot) => {
            *slot = PropSlot::Accessor {
                get: None,
                set: Some(setter),
            };
        }
        None => props.push((
            key,
            PropSlot::Accessor {
                get: None,
                set: Some(setter),
            },
        )),
    }
}

pub(super) fn object_lookup_getter(props: &[(String, PropSlot)], key: &str) -> JsVal {
    match object_own_slot(props, key) {
        Some(PropSlot::Accessor { get: Some(g), .. }) => g.clone(),
        _ => JsVal::Undef,
    }
}

pub(super) fn proto_lookup_setter(proto: &JsVal, key: &str) -> Option<JsVal> {
    match proto {
        JsVal::Object { props, proto, .. } => {
            let props = props.borrow();
            if let Some(PropSlot::Accessor { set: Some(s), .. }) = object_own_slot(&props, key) {
                return Some(s.clone());
            }
            if let Some(PropSlot::Data(_)) = object_own_slot(&props, key) {
                return None;
            }
            drop(props);
            proto_lookup_setter(proto, key)
        }
        _ => None,
    }
}

pub(super) fn write_back_value(env: &mut HashMap<LocalId, JsVal>, fresh: &JsVal) {
    let Some(id) = object_id(fresh) else {
        // UserFn: match by pointer identity of props Rc
        if let JsVal::UserFn { props, .. } = fresh {
            for v in env.values_mut() {
                if let JsVal::UserFn {
                    props: p2,
                    params,
                    body,
                } = v
                {
                    if Rc::ptr_eq(props, p2) {
                        *v = fresh.clone();
                    } else {
                        let _ = (params, body);
                    }
                }
            }
        }
        return;
    };
    for v in env.values_mut() {
        replace_object_id(v, id, fresh);
    }
}

pub(super) fn object_id(v: &JsVal) -> Option<u64> {
    match v {
        JsVal::Object { id, .. } => Some(*id),
        _ => None,
    }
}

pub(super) fn replace_object_id(v: &mut JsVal, id: u64, fresh: &JsVal) {
    match v {
        JsVal::Object { id: oid, .. } if *oid == id => {
            *v = fresh.clone();
        }
        JsVal::Object { props, proto, .. } => {
            for (_, slot) in props.borrow_mut().iter_mut() {
                if let PropSlot::Data(inner) = slot {
                    replace_object_id(inner, id, fresh);
                } else if let PropSlot::Accessor { get, set } = slot {
                    if let Some(g) = get {
                        replace_object_id(g, id, fresh);
                    }
                    if let Some(s) = set {
                        replace_object_id(s, id, fresh);
                    }
                }
            }
            replace_object_id(proto, id, fresh);
        }
        JsVal::UserFn { props, .. } => {
            for (_, slot) in props.borrow_mut().iter_mut() {
                if let PropSlot::Data(inner) = slot {
                    replace_object_id(inner, id, fresh);
                } else if let PropSlot::Accessor { get, set } = slot {
                    if let Some(g) = get {
                        replace_object_id(g, id, fresh);
                    }
                    if let Some(s) = set {
                        replace_object_id(s, id, fresh);
                    }
                }
            }
        }
        _ => {}
    }
}

pub(super) fn object_get_own_property_descriptor(
    interp: &Interp,
    target: &JsVal,
    key: &str,
) -> Result<JsVal, ()> {
    let slot = match target {
        JsVal::Object { props, .. } => object_own_slot(&props.borrow(), key).cloned(),
        JsVal::UserFn { props, .. } => props
            .borrow()
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, s)| s.clone()),
        _ => None,
    };
    let Some(slot) = slot else {
        return Ok(JsVal::Undef);
    };
    let mut desc_props = Vec::new();
    match slot {
        PropSlot::Data(v) => {
            desc_props.push(("value".into(), PropSlot::Data(v)));
            desc_props.push(("writable".into(), PropSlot::Data(JsVal::Bool(true))));
            desc_props.push(("enumerable".into(), PropSlot::Data(JsVal::Bool(true))));
            desc_props.push(("configurable".into(), PropSlot::Data(JsVal::Bool(true))));
        }
        PropSlot::Accessor { get, set } => {
            desc_props.push(("get".into(), PropSlot::Data(get.unwrap_or(JsVal::Undef))));
            desc_props.push(("set".into(), PropSlot::Data(set.unwrap_or(JsVal::Undef))));
            desc_props.push(("enumerable".into(), PropSlot::Data(JsVal::Bool(true))));
            desc_props.push(("configurable".into(), PropSlot::Data(JsVal::Bool(true))));
        }
    }
    Ok(new_object(interp, desc_props))
}

pub(super) fn object_define_property(
    target: &mut JsVal,
    key: String,
    desc: &JsVal,
    _env: &mut HashMap<LocalId, JsVal>,
) -> Result<(), ()> {
    let JsVal::Object { props: dprops, .. } = desc else {
        return Err(());
    };
    let dprops = dprops.borrow();
    let get_data = |k: &str| -> Option<JsVal> {
        match object_own_slot(&dprops, k) {
            Some(PropSlot::Data(v)) => Some(v.clone()),
            _ => None,
        }
    };
    let has = |k: &str| object_own_has(&dprops, k);
    match target {
        JsVal::Object { props, .. } => {
            apply_define_property(&mut props.borrow_mut(), key, &dprops, has, get_data)
        }
        JsVal::UserFn { props, .. } => {
            apply_define_property(&mut props.borrow_mut(), key, &dprops, has, get_data)
        }
        _ => Err(()),
    }
}

pub(super) fn apply_define_property(
    props: &mut Vec<(String, PropSlot)>,
    key: String,
    _dprops: &[(String, PropSlot)],
    has: impl Fn(&str) -> bool,
    get_data: impl Fn(&str) -> Option<JsVal>,
) -> Result<(), ()> {
    let has_get = has("get");
    let has_set = has("set");
    let has_value = has("value");
    if has_get || has_set {
        let get = if has_get {
            match get_data("get") {
                Some(JsVal::Undef) | None => None,
                Some(v) => Some(v),
            }
        } else {
            // Preserve existing getter when only setter provided.
            match object_own_slot(props, &key) {
                Some(PropSlot::Accessor { get, .. }) => get.clone(),
                _ => None,
            }
        };
        let set = if has_set {
            match get_data("set") {
                Some(JsVal::Undef) | None => None,
                Some(v) => Some(v),
            }
        } else {
            match object_own_slot(props, &key) {
                Some(PropSlot::Accessor { set, .. }) => set.clone(),
                _ => None,
            }
        };
        // When only get is defined, keep existing set and vice versa (already handled).
        if let Some(slot) = object_own_slot_mut(props, &key) {
            *slot = PropSlot::Accessor { get, set };
        } else {
            props.push((key, PropSlot::Accessor { get, set }));
        }
        Ok(())
    } else if has_value {
        let v = get_data("value").unwrap_or(JsVal::Undef);
        object_set_data(props, key, v);
        Ok(())
    } else {
        // enumerable-only touch etc. — ignore for fixture subset
        Ok(())
    }
}

pub(super) fn object_lookup_setter(props: &[(String, PropSlot)], key: &str) -> JsVal {
    match object_own_slot(props, key) {
        Some(PropSlot::Accessor { set: Some(s), .. }) => s.clone(),
        _ => JsVal::Undef,
    }
}

pub(super) fn object_get_prototype(obj: &JsVal) -> Result<JsVal, ()> {
    match obj {
        JsVal::Object { proto, .. } => Ok((**proto).clone()),
        JsVal::Builtin(BuiltinId::ObjectPrototype) => Ok(JsVal::Null),
        _ => Err(()),
    }
}

pub(super) fn is_object_key(v: &JsVal) -> bool {
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

pub(super) fn new_array_buffer(interp: &Interp, byte_len: usize) -> JsVal {
    JsVal::ArrayBufferInst {
        id: next_object_id(interp),
        bytes: Rc::new(RefCell::new(vec![0u8; byte_len])),
    }
}

pub(super) fn typed_array_from_buffer(kind: TaKind, buf: &JsVal) -> Result<JsVal, ()> {
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

pub(super) fn typed_array_from_length(interp: &Interp, kind: TaKind, len: usize) -> JsVal {
    let blen = len.saturating_mul(kind.bytes_per_element());
    let id = next_object_id(interp);
    let bytes = Rc::new(RefCell::new(vec![0u8; blen]));
    JsVal::TypedArrayInst {
        kind,
        buffer_id: id,
        bytes,
        length: len,
    }
}

pub(super) fn typed_array_from_array(
    interp: &Interp,
    kind: TaKind,
    elems: &[JsVal],
) -> Result<JsVal, ()> {
    let ta = typed_array_from_length(interp, kind, elems.len());
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

pub(super) fn write_ta_elem(kind: TaKind, buf: &mut [u8], off: usize, n: f64) -> Result<(), ()> {
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

pub(super) fn read_ta_elem(kind: TaKind, buf: &[u8], off: usize) -> Result<f64, ()> {
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

pub(super) struct ModuleInfo {
    pub(super) user_locals: Vec<LocalId>,
    pub(super) values: HashMap<LocalId, JsVal>,
}

#[derive(Debug)]
pub(super) enum Flow {
    Normal,
    Throw(JsVal),
    Return(JsVal),
}

thread_local! {
    pub(super) static CURRENT_THIS: std::cell::RefCell<JsVal> = const { std::cell::RefCell::new(JsVal::Undef) };
    pub(super) static CURRENT_NEW_TARGET: std::cell::RefCell<JsVal> = const { std::cell::RefCell::new(JsVal::Undef) };
}

pub(super) fn with_this<R>(this: JsVal, f: impl FnOnce() -> R) -> R {
    CURRENT_THIS.with(|cell| {
        let prev = cell.replace(this);
        let out = f();
        cell.replace(prev);
        out
    })
}

pub(super) fn current_this() -> JsVal {
    CURRENT_THIS.with(|cell| cell.borrow().clone())
}

pub(super) fn with_new_target<R>(nt: JsVal, f: impl FnOnce() -> R) -> R {
    CURRENT_NEW_TARGET.with(|cell| {
        let prev = cell.replace(nt);
        let out = f();
        cell.replace(prev);
        out
    })
}

pub(super) fn current_new_target() -> JsVal {
    CURRENT_NEW_TARGET.with(|cell| cell.borrow().clone())
}

pub(super) fn js_string_to_utf8(s: &JsString) -> String {
    s.to_string_lossy()
}

pub(super) fn error_ctor_name(b: BuiltinId) -> Option<&'static str> {
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

pub(super) fn is_object_accessor_legacy(id: BuiltinId) -> bool {
    matches!(
        id,
        BuiltinId::DefineGetter
            | BuiltinId::DefineSetter
            | BuiltinId::LookupGetter
            | BuiltinId::LookupSetter
    )
}

pub(super) fn object_accessor_legacy_name(id: BuiltinId) -> Option<&'static str> {
    match id {
        BuiltinId::DefineGetter => Some("__defineGetter__"),
        BuiltinId::DefineSetter => Some("__defineSetter__"),
        BuiltinId::LookupGetter => Some("__lookupGetter__"),
        BuiltinId::LookupSetter => Some("__lookupSetter__"),
        _ => None,
    }
}

pub(super) fn object_accessor_legacy_builtin(key: &str) -> Option<BuiltinId> {
    match key {
        "__defineGetter__" => Some(BuiltinId::DefineGetter),
        "__defineSetter__" => Some(BuiltinId::DefineSetter),
        "__lookupGetter__" => Some(BuiltinId::LookupGetter),
        "__lookupSetter__" => Some(BuiltinId::LookupSetter),
        _ => None,
    }
}

/// ECMA-262 SameValueZero (Map/Set key equality): NaN≡NaN, +0≡-0, else ===.
pub(super) fn same_value_zero(a: &JsVal, b: &JsVal) -> bool {
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

pub(super) fn typeof_str(v: &JsVal) -> String {
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
        JsVal::UserFn { .. } => "function".into(),
        JsVal::Builtin(BuiltinId::Undefined) => "undefined".into(),
        JsVal::Builtin(BuiltinId::Nan | BuiltinId::Infinity) => "number".into(),
        JsVal::Builtin(
            BuiltinId::GlobalThis
            | BuiltinId::ObjectPrototype
            | BuiltinId::FunctionPrototype
            | BuiltinId::StringPrototype
            | BuiltinId::DatePrototype
            | BuiltinId::RegExpPrototype
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
            | BuiltinId::DefineGetter
            | BuiltinId::DefineSetter
            | BuiltinId::LookupGetter
            | BuiltinId::LookupSetter
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
            | BuiltinId::StrTrimStart
            | BuiltinId::StrTrimEnd
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
            | BuiltinId::RegExpCompile
            | BuiltinId::Map
            | BuiltinId::Set
            | BuiltinId::WeakMap
            | BuiltinId::WeakSet
            | BuiltinId::ArrayBuffer
            | BuiltinId::DataView
            | BuiltinId::Uint8Array
            | BuiltinId::Int32Array
            | BuiltinId::Float64Array
            | BuiltinId::ParseUrl
            | BuiltinId::ParseQuery
            | BuiltinId::SerializeQuery
            | BuiltinId::ParseFlags
            | BuiltinId::FlagHelp,
        ) => "function".into(),
    }
}

pub(super) fn to_boolean(v: &JsVal) -> bool {
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
        | JsVal::UserFn { .. }
        | JsVal::Object { .. } => true,
    }
}

pub(super) fn strict_eq(l: &JsVal, r: &JsVal) -> bool {
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
