//! L08.01: portable URL parse — scheme / host / path / query / hash.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUrl {
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub query: String,
    pub hash: String,
}

pub fn parse_url(input: &str) -> Result<ParsedUrl, ()> {
    let bytes = input.as_bytes();
    let mut i = 0;
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return Err(());
    }
    while i < bytes.len() {
        let b = bytes[i];
        if b == b':' {
            break;
        }
        if !(b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')) {
            return Err(());
        }
        i += 1;
    }
    if i == 0 || i >= bytes.len() || bytes[i] != b':' {
        return Err(());
    }
    let scheme = input[..i].to_ascii_lowercase();
    i += 1;
    if i + 1 >= bytes.len() || bytes[i] != b'/' || bytes[i + 1] != b'/' {
        return Err(());
    }
    i += 2;
    let auth_start = i;
    while i < bytes.len() && !matches!(bytes[i], b'/' | b'?' | b'#') {
        i += 1;
    }
    if i == auth_start {
        return Err(());
    }
    let host = input[auth_start..i].to_string();
    let mut path = String::new();
    if i < bytes.len() && bytes[i] == b'/' {
        let path_start = i;
        i += 1;
        while i < bytes.len() && !matches!(bytes[i], b'?' | b'#') {
            i += 1;
        }
        path = input[path_start..i].to_string();
    }
    let mut query = String::new();
    if i < bytes.len() && bytes[i] == b'?' {
        i += 1;
        let q_start = i;
        while i < bytes.len() && bytes[i] != b'#' {
            i += 1;
        }
        query = input[q_start..i].to_string();
    }
    let mut hash = String::new();
    if i < bytes.len() && bytes[i] == b'#' {
        i += 1;
        hash = input[i..].to_string();
    }
    Ok(ParsedUrl {
        scheme,
        host,
        path,
        query,
        hash,
    })
}

pub fn parse_url_js_polyfill() -> &'static str {
    r#"function parseUrl(input) {
  if (typeof input !== "string") input = String(input);
  var s = input;
  var i = 0;
  var n = s.length;
  if (n === 0) throw new TypeError("Invalid URL");
  var c0 = s.charCodeAt(0);
  if (!((c0 >= 65 && c0 <= 90) || (c0 >= 97 && c0 <= 122))) throw new TypeError("Invalid URL");
  while (i < n) {
    var b = s.charCodeAt(i);
    if (b === 58) break;
    var ok = (b >= 65 && b <= 90) || (b >= 97 && b <= 122) || (b >= 48 && b <= 57)
      || b === 43 || b === 45 || b === 46;
    if (!ok) throw new TypeError("Invalid URL");
    i++;
  }
  if (i === 0 || i >= n || s.charCodeAt(i) !== 58) throw new TypeError("Invalid URL");
  var scheme = s.slice(0, i).toLowerCase();
  i++;
  if (i + 1 >= n || s.charCodeAt(i) !== 47 || s.charCodeAt(i + 1) !== 47) throw new TypeError("Invalid URL");
  i += 2;
  var authStart = i;
  while (i < n) {
    var ch = s.charCodeAt(i);
    if (ch === 47 || ch === 63 || ch === 35) break;
    i++;
  }
  if (i === authStart) throw new TypeError("Invalid URL");
  var host = s.slice(authStart, i);
  var path = "";
  if (i < n && s.charCodeAt(i) === 47) {
    var pathStart = i;
    i++;
    while (i < n) {
      var ch2 = s.charCodeAt(i);
      if (ch2 === 63 || ch2 === 35) break;
      i++;
    }
    path = s.slice(pathStart, i);
  }
  var query = "";
  if (i < n && s.charCodeAt(i) === 63) {
    i++;
    var qStart = i;
    while (i < n && s.charCodeAt(i) !== 35) i++;
    query = s.slice(qStart, i);
  }
  var hash = "";
  if (i < n && s.charCodeAt(i) === 35) {
    i++;
    hash = s.slice(i);
  }
  return { scheme: scheme, host: host, path: path, query: query, hash: hash };
}
if (typeof globalThis !== "undefined") globalThis.parseUrl = parseUrl;
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_full_url() {
        let u = parse_url("https://example.com/path?q=1#frag").unwrap();
        assert_eq!(u.scheme, "https");
        assert_eq!(u.host, "example.com");
        assert_eq!(u.path, "/path");
        assert_eq!(u.query, "q=1");
        assert_eq!(u.hash, "frag");
    }
    #[test]
    fn parses_port_and_root_path() {
        let u = parse_url("http://localhost:8080/").unwrap();
        assert_eq!(u.host, "localhost:8080");
        assert_eq!(u.path, "/");
    }
    #[test]
    fn empty_path_when_absent() {
        assert_eq!(parse_url("https://example.com").unwrap().path, "");
    }
    #[test]
    fn authority_with_userinfo() {
        let u = parse_url("https://user:pass@example.com:443/a/b?x=1&y=2#top").unwrap();
        assert_eq!(u.host, "user:pass@example.com:443");
        assert_eq!(u.path, "/a/b");
        assert_eq!(u.query, "x=1&y=2");
        assert_eq!(u.hash, "top");
    }
    #[test]
    fn rejects_relative() {
        assert!(parse_url("/path").is_err());
        assert!(parse_url("").is_err());
    }
    #[test]
    fn lowercases_scheme() {
        let u = parse_url("HTTPS://Example.COM/x").unwrap();
        assert_eq!(u.scheme, "https");
        assert_eq!(u.host, "Example.COM");
    }
}

/// L08.02: parse `application/x-www-form-urlencoded` query text.
/// Last duplicate key wins; empty keys skipped; optional leading `?`.
pub fn parse_query(input: &str) -> Vec<(String, String)> {
    let s = input.strip_prefix('?').unwrap_or(input);
    let mut out: Vec<(String, String)> = Vec::new();
    if s.is_empty() {
        return out;
    }
    for part in s.split('&') {
        if part.is_empty() {
            continue;
        }
        let (k, v) = match part.split_once('=') {
            Some((k, v)) => (k, v),
            None => (part, ""),
        };
        let k = query_decode(k);
        if k.is_empty() {
            continue;
        }
        let v = query_decode(v);
        if let Some(existing) = out.iter_mut().find(|(ek, _)| *ek == k) {
            existing.1 = v;
        } else {
            out.push((k, v));
        }
    }
    out
}

/// L08.02: serialize key/value pairs to query text (insertion order).
pub fn serialize_query(pairs: &[(String, String)]) -> String {
    let mut out = String::new();
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        out.push_str(&query_encode(k));
        out.push('=');
        out.push_str(&query_encode(v));
    }
    out
}

fn query_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                    out.push((h << 4) | l);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn query_encode(s: &str) -> String {
    let mut out = String::new();
    for &b in s.as_bytes() {
        if matches!(
            b,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
        ) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub fn query_js_polyfill() -> &'static str {
    r#"function queryDecode(s) {
  s = String(s).replace(/\+/g, " ");
  try { return decodeURIComponent(s); } catch (e) { return s; }
}
function queryEncode(s) {
  s = String(s);
  var utf8 = unescape(encodeURIComponent(s));
  var out = "";
  for (var i = 0; i < utf8.length; i++) {
    var c = utf8.charCodeAt(i);
    var ok = (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || (c >= 48 && c <= 57)
      || c === 45 || c === 95 || c === 46 || c === 126;
    if (ok) out += utf8.charAt(i);
    else {
      var hex = c.toString(16).toUpperCase();
      if (hex.length < 2) hex = "0" + hex;
      out += "%" + hex;
    }
  }
  return out;
}
function parseQuery(input) {
  if (typeof input !== "string") input = String(input);
  if (input.charCodeAt(0) === 63) input = input.slice(1);
  var result = {};
  if (input.length === 0) return result;
  var parts = input.split("&");
  for (var i = 0; i < parts.length; i++) {
    var part = parts[i];
    if (part.length === 0) continue;
    var eq = part.indexOf("=");
    var k = eq < 0 ? part : part.slice(0, eq);
    var v = eq < 0 ? "" : part.slice(eq + 1);
    k = queryDecode(k);
    if (k.length === 0) continue;
    result[k] = queryDecode(v);
  }
  return result;
}
function serializeQuery(obj) {
  if (obj === null || typeof obj !== "object" || Array.isArray(obj)) {
    throw new TypeError("serializeQuery expects an object");
  }
  var keys = Object.keys(obj);
  var parts = [];
  for (var i = 0; i < keys.length; i++) {
    var k = keys[i];
    var v = obj[k];
    if (v === undefined) continue;
    parts.push(queryEncode(k) + "=" + queryEncode(v));
  }
  return parts.join("&");
}
if (typeof globalThis !== "undefined") {
  globalThis.parseQuery = parseQuery;
  globalThis.serializeQuery = serializeQuery;
}
"#
}

#[cfg(test)]
mod query_tests {
    use super::*;

    #[test]
    fn parses_pairs_last_wins() {
        let q = parse_query("a=1&b=2");
        assert_eq!(q, vec![("a".into(), "1".into()), ("b".into(), "2".into())]);
        let last = parse_query("a=1&a=2");
        assert_eq!(last, vec![("a".into(), "2".into())]);
    }

    #[test]
    fn decodes_percent_and_plus() {
        assert_eq!(
            parse_query("q=hello%20world"),
            vec![("q".into(), "hello world".into())]
        );
        assert_eq!(
            parse_query("q=hello+world"),
            vec![("q".into(), "hello world".into())]
        );
    }

    #[test]
    fn empty_and_bare_and_roundtrip() {
        assert!(parse_query("").is_empty());
        assert_eq!(parse_query("flag"), vec![("flag".into(), "".into())]);
        assert_eq!(parse_query("a="), vec![("a".into(), "".into())]);
        assert_eq!(serialize_query(&parse_query("x=1&y=2")), "x=1&y=2");
        assert_eq!(
            serialize_query(&[("q".into(), "hello world".into())]),
            "q=hello%20world"
        );
    }
}
