//! L09: MIME multipart parse/serialize (`parseMultipart` / `serializeMultipart`).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimePart {
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimeErrorKind {
    Type,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimeError {
    pub kind: MimeErrorKind,
    pub message: &'static str,
}

impl MimeError {
    fn ty(message: &'static str) -> Self {
        Self {
            kind: MimeErrorKind::Type,
            message,
        }
    }

    fn invalid(message: &'static str) -> Self {
        Self {
            kind: MimeErrorKind::Invalid,
            message,
        }
    }
}

fn validate_boundary(boundary: &str) -> Result<(), MimeError> {
    if boundary.is_empty() || boundary.contains('\r') || boundary.contains('\n') {
        return Err(MimeError::ty("invalid multipart boundary"));
    }
    Ok(())
}

fn is_boundary_at(input: &str, pos: usize, dash: &str) -> bool {
    if !input[pos..].starts_with(dash) {
        return false;
    }
    let after = pos + dash.len();
    if after >= input.len() {
        return true;
    }
    matches!(input.as_bytes()[after], b'-' | b' ' | b'\t' | b'\r' | b'\n')
}

fn is_line_start(input: &str, pos: usize) -> bool {
    pos == 0 || input.as_bytes()[pos - 1] == b'\n'
}

fn find_dash(input: &str, dash: &str, from: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        if is_line_start(input, i) && is_boundary_at(input, i, dash) {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn skip_newline(input: &str, i: usize) -> Result<usize, MimeError> {
    let bytes = input.as_bytes();
    if i < bytes.len() && bytes[i] == b'\r' {
        if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            return Ok(i + 2);
        }
        return Ok(i + 1);
    }
    if i < bytes.len() && bytes[i] == b'\n' {
        return Ok(i + 1);
    }
    Err(MimeError::invalid("truncated multipart"))
}

fn find_next_delim(input: &str, from: usize, dash: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let dash_pos = i + 1;
            if is_boundary_at(input, dash_pos, dash) {
                let body_end = if i > 0 && bytes[i - 1] == b'\r' {
                    i - 1
                } else {
                    i
                };
                return Some((body_end, dash_pos));
            }
        }
        i += 1;
    }
    None
}

fn parse_part(text: &str) -> Result<MimePart, MimeError> {
    let sep = if let Some(p) = text.find("\r\n\r\n") {
        (p, 4)
    } else if let Some(p) = text.find("\n\n") {
        (p, 2)
    } else {
        return Err(MimeError::invalid("truncated multipart part"));
    };
    let header_block = &text[..sep.0];
    let body = text[sep.0 + sep.1..].to_string();
    let mut headers = Vec::new();
    if !header_block.is_empty() {
        for line in header_block.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.is_empty() {
                continue;
            }
            let Some((name, value)) = line.split_once(':') else {
                return Err(MimeError::invalid("invalid multipart header"));
            };
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    Ok(MimePart { headers, body })
}

pub fn parse_multipart(input: &str, boundary: &str) -> Result<Vec<MimePart>, MimeError> {
    validate_boundary(boundary)?;
    let dash = format!("--{boundary}");
    let mut pos = find_dash(input, &dash, 0)
        .ok_or_else(|| MimeError::invalid("missing multipart boundary"))?;
    let mut parts = Vec::new();
    loop {
        let after_dash = pos + dash.len();
        if input[after_dash..].starts_with("--") {
            return Ok(parts);
        }
        let mut i = after_dash;
        let bytes = input.as_bytes();
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        i = skip_newline(input, i)?;
        let (body_end, next_dash) = find_next_delim(input, i, &dash)
            .ok_or_else(|| MimeError::invalid("truncated multipart"))?;
        parts.push(parse_part(&input[i..body_end])?);
        pos = next_dash;
    }
}

pub fn serialize_multipart(parts: &[MimePart], boundary: &str) -> Result<String, MimeError> {
    validate_boundary(boundary)?;
    let mut out = String::new();
    for part in parts {
        out.push_str("--");
        out.push_str(boundary);
        out.push_str("\r\n");
        for (name, value) in &part.headers {
            out.push_str(name);
            out.push_str(": ");
            out.push_str(value);
            out.push_str("\r\n");
        }
        out.push_str("\r\n");
        out.push_str(&part.body);
        out.push_str("\r\n");
    }
    out.push_str("--");
    out.push_str(boundary);
    out.push_str("--");
    Ok(out)
}

pub fn mime_js_polyfill() -> &'static str {
    r#"function parseMultipart(input, boundary) {
  if (typeof input !== "string" || typeof boundary !== "string") {
    throw new TypeError("parseMultipart expects string body and boundary");
  }
  if (boundary.length === 0 || boundary.indexOf("\r") >= 0 || boundary.indexOf("\n") >= 0) {
    throw new TypeError("invalid multipart boundary");
  }
  var dash = "--" + boundary;
  function isBoundaryAt(s, pos) {
    if (s.slice(pos, pos + dash.length) !== dash) return false;
    var after = pos + dash.length;
    if (after >= s.length) return true;
    var c = s.charCodeAt(after);
    return c === 45 || c === 32 || c === 9 || c === 13 || c === 10;
  }
  function isLineStart(s, pos) {
    return pos === 0 || s.charCodeAt(pos - 1) === 10;
  }
  function findDash(s, from) {
    var i = from;
    while (i < s.length) {
      if (isLineStart(s, i) && isBoundaryAt(s, i)) return i;
      i++;
    }
    return -1;
  }
  function skipNewline(s, i) {
    if (i < s.length && s.charCodeAt(i) === 13) {
      if (i + 1 < s.length && s.charCodeAt(i + 1) === 10) return i + 2;
      return i + 1;
    }
    if (i < s.length && s.charCodeAt(i) === 10) return i + 1;
    throw new Error("truncated multipart");
  }
  function findNextDelim(s, from) {
    var i = from;
    while (i < s.length) {
      if (s.charCodeAt(i) === 10) {
        var dashPos = i + 1;
        if (isBoundaryAt(s, dashPos)) {
          var bodyEnd = (i > 0 && s.charCodeAt(i - 1) === 13) ? i - 1 : i;
          return [bodyEnd, dashPos];
        }
      }
      i++;
    }
    return null;
  }
  function parsePart(text) {
    var p = text.indexOf("\r\n\r\n");
    var n = 4;
    if (p < 0) {
      p = text.indexOf("\n\n");
      n = 2;
    }
    if (p < 0) throw new Error("truncated multipart part");
    var headerBlock = text.slice(0, p);
    var body = text.slice(p + n);
    var headers = {};
    if (headerBlock.length > 0) {
      var lines = headerBlock.split("\n");
      var li;
      for (li = 0; li < lines.length; li++) {
        var line = lines[li];
        if (line.length > 0 && line.charCodeAt(line.length - 1) === 13) {
          line = line.slice(0, line.length - 1);
        }
        if (line.length === 0) continue;
        var colon = line.indexOf(":");
        if (colon < 0) throw new Error("invalid multipart header");
        var name = line.slice(0, colon).replace(/^\s+|\s+$/g, "");
        var value = line.slice(colon + 1).replace(/^\s+|\s+$/g, "");
        headers[name] = value;
      }
    }
    return { headers: headers, body: body };
  }
  var pos = findDash(input, 0);
  if (pos < 0) throw new Error("missing multipart boundary");
  var parts = [];
  while (true) {
    var afterDash = pos + dash.length;
    if (input.slice(afterDash, afterDash + 2) === "--") return parts;
    var i = afterDash;
    while (i < input.length && (input.charCodeAt(i) === 32 || input.charCodeAt(i) === 9)) i++;
    i = skipNewline(input, i);
    var next = findNextDelim(input, i);
    if (!next) throw new Error("truncated multipart");
    parts.push(parsePart(input.slice(i, next[0])));
    pos = next[1];
  }
}
function serializeMultipart(parts, boundary) {
  if (!Array.isArray(parts) || typeof boundary !== "string") {
    throw new TypeError("serializeMultipart expects parts array and string boundary");
  }
  if (boundary.length === 0 || boundary.indexOf("\r") >= 0 || boundary.indexOf("\n") >= 0) {
    throw new TypeError("invalid multipart boundary");
  }
  var out = "";
  var pi;
  for (pi = 0; pi < parts.length; pi++) {
    var part = parts[pi];
    if (!part || typeof part !== "object") {
      throw new TypeError("serializeMultipart expects part objects");
    }
    if (typeof part.body !== "string") {
      throw new TypeError("serializeMultipart expects string part body");
    }
    var headers = part.headers;
    if (headers == null) headers = {};
    if (typeof headers !== "object") {
      throw new TypeError("serializeMultipart expects part headers object");
    }
    out += "--" + boundary + "\r\n";
    var k;
    for (k in headers) {
      if (Object.prototype.hasOwnProperty.call(headers, k)) {
        out += k + ": " + String(headers[k]) + "\r\n";
      }
    }
    out += "\r\n" + part.body + "\r\n";
  }
  out += "--" + boundary + "--";
  return out;
}
if (typeof globalThis !== "undefined") {
  globalThis.parseMultipart = parseMultipart;
  globalThis.serializeMultipart = serializeMultipart;
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_parts() {
        let body = "--abc\r\nContent-Disposition: form-data; name=\"x\"\r\n\r\nhello\r\n--abc\r\nContent-Type: text/plain\r\n\r\nworld\r\n--abc--";
        let parts = parse_multipart(body, "abc").unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].headers[0].0, "Content-Disposition");
        assert_eq!(parts[0].headers[0].1, "form-data; name=\"x\"");
        assert_eq!(parts[0].body, "hello");
        assert_eq!(parts[1].headers[0].1, "text/plain");
        assert_eq!(parts[1].body, "world");
    }

    #[test]
    fn round_trips_common_case() {
        let parts = vec![MimePart {
            headers: vec![("Content-Type".into(), "text/plain".into())],
            body: "hello".into(),
        }];
        let text = serialize_multipart(&parts, "xyz").unwrap();
        let back = parse_multipart(&text, "xyz").unwrap();
        assert_eq!(back, parts);
    }

    #[test]
    fn rejects_truncated() {
        let err = parse_multipart("--b\r\nContent-Type: text/plain\r\n\r\nhello", "b").unwrap_err();
        assert_eq!(err.kind, MimeErrorKind::Invalid);
    }

    #[test]
    fn rejects_missing_boundary() {
        let err = parse_multipart("not a multipart body", "bound").unwrap_err();
        assert_eq!(err.kind, MimeErrorKind::Invalid);
    }

    #[test]
    fn rejects_empty_boundary() {
        let err = parse_multipart("--\r\n\r\n--", "").unwrap_err();
        assert_eq!(err.kind, MimeErrorKind::Type);
    }
}
