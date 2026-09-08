use std::fmt;

/// ECMAScript string value: a sequence of UTF-16 code units (may include unpaired surrogates).
#[derive(Clone, PartialEq, Eq, Default)]
pub struct JsString {
    units: Vec<u16>,
}

impl JsString {
    pub fn new() -> Self {
        Self { units: Vec::new() }
    }

    pub fn units(&self) -> &[u16] {
        &self.units
    }

    pub fn push_code_unit(&mut self, unit: u16) {
        self.units.push(unit);
    }

    /// Push a Unicode scalar value as one or two UTF-16 code units.
    pub fn push_scalar(&mut self, c: char) {
        let mut buf = [0u16; 2];
        for u in c.encode_utf16(&mut buf) {
            self.units.push(*u);
        }
    }

    /// Push a code point from `\xHH` / `\uXXXX` (any 16-bit unit, incl. surrogates)
    /// or `\u{…}` scalar (validated by caller for braced form).
    pub fn push_code_point_unit(&mut self, cp: u32) -> Result<(), ()> {
        if cp <= 0xFFFF {
            self.units.push(cp as u16);
            Ok(())
        } else if cp <= 0x10FFFF {
            let c = cp - 0x10000;
            self.units.push(0xD800 + ((c >> 10) as u16));
            self.units.push(0xDC00 + ((c & 0x3FF) as u16));
            Ok(())
        } else {
            Err(())
        }
    }

    /// Lossy UTF-8 for diagnostics/dumps (unpaired surrogates → U+FFFD).
    pub fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(&self.units)
    }

    /// Well-formed UTF-16 only; `None` if unpaired surrogates present.
    pub fn to_string_strict(&self) -> Option<String> {
        String::from_utf16(&self.units).ok()
    }
}

impl From<&str> for JsString {
    fn from(s: &str) -> Self {
        Self {
            units: s.encode_utf16().collect(),
        }
    }
}

impl From<String> for JsString {
    fn from(s: String) -> Self {
        JsString::from(s.as_str())
    }
}

impl fmt::Debug for JsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("JsString")
            .field(&self.to_string_lossy())
            .finish()
    }
}

impl fmt::Display for JsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}
