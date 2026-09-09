use super::*;

pub(super) fn to_string_arg(v: &JsVal) -> Result<String, ()> {
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

pub(super) fn string_annex_method_builtin(key: &str) -> Option<BuiltinId> {
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
        // Annex B aliases share identity with trimStart/trimEnd.
        "trimStart" | "trimLeft" => Some(BuiltinId::StrTrimStart),
        "trimEnd" | "trimRight" => Some(BuiltinId::StrTrimEnd),
        _ => None,
    }
}

pub(super) fn is_string_annex_method(id: BuiltinId) -> bool {
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
            | BuiltinId::StrTrimStart
            | BuiltinId::StrTrimEnd
    )
}

pub(super) fn string_annex_method_name(id: BuiltinId) -> Option<&'static str> {
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
        // Canonical names for `.call` dispatch (aliases share BuiltinId).
        BuiltinId::StrTrimStart => Some("trimStart"),
        BuiltinId::StrTrimEnd => Some("trimEnd"),
        _ => None,
    }
}

/// ECMA-262 ToIntegerOrInfinity for Annex B substr (fixture subset: finite numbers).
pub(super) fn to_integer_or_infinity(v: &JsVal) -> Result<f64, ()> {
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
pub(super) fn js_substr(s: &str, start: &JsVal, length: Option<&JsVal>) -> Result<String, ()> {
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
pub(super) fn create_html(
    s: &str,
    tag: &str,
    attribute: &str,
    value: Option<&JsVal>,
) -> Result<String, ()> {
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

/// ECMA-262 TrimString whitespace (fixture subset + common WhiteSpace/LineTerminator).
pub(super) fn is_trim_ws(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{FEFF}'
            | '\u{000A}'
            | '\u{000D}'
            | '\u{2028}'
            | '\u{2029}'
    ) || c.is_whitespace()
}

pub(super) fn js_trim_start(s: &str) -> String {
    s.trim_start_matches(is_trim_ws).to_string()
}

pub(super) fn js_trim_end(s: &str) -> String {
    s.trim_end_matches(is_trim_ws).to_string()
}

pub(super) fn eval_string_method(this_s: &str, key: &str, args: &[JsVal]) -> Result<JsVal, ()> {
    match key {
        "substr" => {
            let start = args.first().unwrap_or(&JsVal::Undef);
            let length = args.get(1);
            Ok(JsVal::Str(js_substr(this_s, start, length)?))
        }
        "anchor" => Ok(JsVal::Str(create_html(this_s, "a", "name", args.first())?)),
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
        "link" => Ok(JsVal::Str(create_html(this_s, "a", "href", args.first())?)),
        "small" => Ok(JsVal::Str(create_html(this_s, "small", "", None)?)),
        "strike" => Ok(JsVal::Str(create_html(this_s, "strike", "", None)?)),
        "sub" => Ok(JsVal::Str(create_html(this_s, "sub", "", None)?)),
        "sup" => Ok(JsVal::Str(create_html(this_s, "sup", "", None)?)),
        "trimStart" | "trimLeft" => Ok(JsVal::Str(js_trim_start(this_s))),
        "trimEnd" | "trimRight" => Ok(JsVal::Str(js_trim_end(this_s))),
        _ => Err(()),
    }
}

/// uriUnescaped = Alpha / DecimalDigit / "-" / "_" / "." / "!" / "~" / "*" / "'" / "(" / ")"
pub(super) fn is_uri_unescaped(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
        )
}

/// uriReserved = ";" / "/" / "?" / ":" / "@" / "&" / "=" / "+" / "$" / ","
/// plus "#" kept unescaped by encodeURI / reserved by decodeURI.
pub(super) fn is_uri_reserved_or_hash(b: u8) -> bool {
    matches!(
        b,
        b';' | b'/' | b'?' | b':' | b'@' | b'&' | b'=' | b'+' | b'$' | b',' | b'#'
    )
}

/// Annex B.2.1.1 escape: unescaped = Alpha / Digit / "@" / "*" / "_" / "+" / "-" / "." / "/"
pub(super) fn is_escape_unescaped(cu: u16) -> bool {
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
pub(super) fn js_escape(input: &str) -> String {
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
pub(super) fn js_unescape(input: &str) -> String {
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
                    units.push(
                        ((a as u16) << 12) | ((b as u16) << 8) | ((c as u16) << 4) | d as u16,
                    );
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
pub(super) fn js_encode_uri(input: &str, component: bool) -> String {
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

pub(super) fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// ECMA-262 Decode (decodeURI / decodeURIComponent). Reserved set preserved only for decodeURI.
pub(super) fn js_decode_uri(input: &str, component: bool) -> Result<String, ()> {
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

pub(super) fn to_number(v: &JsVal) -> Result<f64, ()> {
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
pub(super) fn js_string_to_number(s: &str) -> f64 {
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
pub(super) fn js_parse_int(input: &str, radix_arg: Option<&JsVal>) -> Result<f64, ()> {
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
pub(super) fn js_parse_float(input: &str) -> f64 {
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
