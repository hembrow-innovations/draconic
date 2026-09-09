use super::*;

/// Minimal JSON.parse for E15.05 fixture depth (null/bool/number/string/array/object).
pub(super) fn json_parse(input: &str) -> Result<JsVal, ()> {
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

pub(super) struct JsonParser<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl<'a> JsonParser<'a> {
    fn skip_ws(&mut self) {
        while self.i < self.bytes.len()
            && matches!(self.bytes[self.i], b' ' | b'\t' | b'\n' | b'\r')
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
            object_set_data(&mut props, key, val);
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
pub(super) fn json_stringify(v: &JsVal) -> Result<String, ()> {
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
            for (k, slot) in props.borrow().iter() {
                let PropSlot::Data(val) = slot else {
                    continue;
                };
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

pub(super) fn json_quote(s: &str) -> String {
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
