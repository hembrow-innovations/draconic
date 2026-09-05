//! L07.01 / L07.02: parse long/short flags, typed options, and designed help text.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlagValue {
    Present,
    Value(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedFlags {
    pub flags: Vec<(String, FlagValue)>,
    pub positionals: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    Boolean,
    String,
    Number,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagSpec {
    pub name: String,
    pub kind: OptionKind,
    pub short: Option<char>,
    pub help: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedValue {
    Bool(bool),
    Str(String),
    Num(f64),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParsedTypedFlags {
    pub flags: Vec<(String, TypedValue)>,
    pub positionals: Vec<String>,
}

fn set_flag(flags: &mut Vec<(String, FlagValue)>, name: String, value: FlagValue) {
    if name.is_empty() {
        return;
    }
    if let Some((_, slot)) = flags.iter_mut().find(|(n, _)| n == &name) {
        *slot = value;
    } else {
        flags.push((name, value));
    }
}

fn set_typed(flags: &mut Vec<(String, TypedValue)>, name: String, value: TypedValue) {
    if name.is_empty() {
        return;
    }
    if let Some((_, slot)) = flags.iter_mut().find(|(n, _)| n == &name) {
        *slot = value;
    } else {
        flags.push((name, value));
    }
}

fn parse_flag_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        0.0
    } else {
        t.parse::<f64>().unwrap_or(f64::NAN)
    }
}

fn coerce(kind: OptionKind, raw: &str) -> TypedValue {
    match kind {
        OptionKind::Boolean => TypedValue::Bool(true),
        OptionKind::String => TypedValue::Str(raw.to_string()),
        OptionKind::Number => TypedValue::Num(parse_flag_number(raw)),
    }
}

fn is_value_token(tok: &str) -> bool {
    !(tok.starts_with('-') && tok != "-")
}

/// Schema-free parse: `--name` / `-n` / `-abc` are present; `--name=v` / `-n=v`
/// take an inline string; `--` ends options; leftover tokens are positionals.
pub fn parse_flags(argv: &[String]) -> ParsedFlags {
    let mut out = ParsedFlags::default();
    let mut i = 0;
    while i < argv.len() {
        let tok = argv[i].as_str();
        if tok == "--" {
            out.positionals.extend(argv[i + 1..].iter().cloned());
            break;
        }
        if let Some(body) = tok.strip_prefix("--") {
            if let Some((name, val)) = body.split_once('=') {
                set_flag(
                    &mut out.flags,
                    name.to_string(),
                    FlagValue::Value(val.to_string()),
                );
            } else {
                set_flag(&mut out.flags, body.to_string(), FlagValue::Present);
            }
            i += 1;
            continue;
        }
        if tok.starts_with('-') && tok != "-" {
            let body = &tok[1..];
            if let Some((name, val)) = body.split_once('=') {
                if name.len() == 1 {
                    set_flag(
                        &mut out.flags,
                        name.to_string(),
                        FlagValue::Value(val.to_string()),
                    );
                } else {
                    let chars: Vec<char> = name.chars().collect();
                    for c in chars.iter().take(chars.len().saturating_sub(1)) {
                        set_flag(&mut out.flags, c.to_string(), FlagValue::Present);
                    }
                    if let Some(last) = chars.last() {
                        set_flag(
                            &mut out.flags,
                            last.to_string(),
                            FlagValue::Value(val.to_string()),
                        );
                    }
                }
            } else {
                for c in body.chars() {
                    set_flag(&mut out.flags, c.to_string(), FlagValue::Present);
                }
            }
            i += 1;
            continue;
        }
        out.positionals.push(tok.to_string());
        i += 1;
    }
    out
}

/// Spec-driven parse: boolean / string / number options; missing booleans are false.
pub fn parse_flags_typed(argv: &[String], spec: &[FlagSpec]) -> ParsedTypedFlags {
    let mut by_long: HashMap<&str, &FlagSpec> = HashMap::new();
    let mut by_short: HashMap<char, &FlagSpec> = HashMap::new();
    for s in spec {
        by_long.insert(s.name.as_str(), s);
        if let Some(c) = s.short {
            by_short.insert(c, s);
        }
    }

    let mut flags = Vec::new();
    let mut positionals = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        let tok = argv[i].as_str();
        if tok == "--" {
            positionals.extend(argv[i + 1..].iter().cloned());
            break;
        }
        if let Some(body) = tok.strip_prefix("--") {
            if let Some((name, val)) = body.split_once('=') {
                if let Some(sp) = by_long.get(name) {
                    if sp.kind == OptionKind::Boolean {
                        set_typed(&mut flags, sp.name.clone(), TypedValue::Bool(true));
                    } else {
                        set_typed(&mut flags, sp.name.clone(), coerce(sp.kind, val));
                    }
                } else if !name.is_empty() {
                    set_typed(
                        &mut flags,
                        name.to_string(),
                        TypedValue::Str(val.to_string()),
                    );
                }
            } else if let Some(sp) = by_long.get(body) {
                match sp.kind {
                    OptionKind::Boolean => {
                        set_typed(&mut flags, sp.name.clone(), TypedValue::Bool(true));
                    }
                    OptionKind::String | OptionKind::Number => {
                        if i + 1 < argv.len() && is_value_token(&argv[i + 1]) {
                            i += 1;
                            set_typed(&mut flags, sp.name.clone(), coerce(sp.kind, &argv[i]));
                        }
                    }
                }
            } else if !body.is_empty() {
                set_typed(&mut flags, body.to_string(), TypedValue::Bool(true));
            }
            i += 1;
            continue;
        }
        if tok.starts_with('-') && tok != "-" {
            let body = &tok[1..];
            if let Some((name, val)) = body.split_once('=') {
                let chars: Vec<char> = name.chars().collect();
                if chars.len() == 1 {
                    apply_short_inline(&mut flags, chars[0], val, &by_short);
                } else {
                    for c in chars.iter().take(chars.len().saturating_sub(1)) {
                        apply_short_present(&mut flags, *c, &by_short);
                    }
                    if let Some(last) = chars.last() {
                        apply_short_inline(&mut flags, *last, val, &by_short);
                    }
                }
            } else {
                let chars: Vec<char> = body.chars().collect();
                for (idx, c) in chars.iter().enumerate() {
                    let is_last = idx + 1 == chars.len();
                    if let Some(sp) = by_short.get(c) {
                        match sp.kind {
                            OptionKind::Boolean => {
                                set_typed(&mut flags, sp.name.clone(), TypedValue::Bool(true));
                            }
                            OptionKind::String | OptionKind::Number => {
                                if is_last && i + 1 < argv.len() && is_value_token(&argv[i + 1]) {
                                    i += 1;
                                    set_typed(
                                        &mut flags,
                                        sp.name.clone(),
                                        coerce(sp.kind, &argv[i]),
                                    );
                                }
                            }
                        }
                    } else {
                        set_typed(&mut flags, c.to_string(), TypedValue::Bool(true));
                    }
                }
            }
            i += 1;
            continue;
        }
        positionals.push(tok.to_string());
        i += 1;
    }

    for s in spec {
        if s.kind == OptionKind::Boolean && !flags.iter().any(|(n, _)| n == &s.name) {
            flags.push((s.name.clone(), TypedValue::Bool(false)));
        }
    }

    ParsedTypedFlags { flags, positionals }
}

fn apply_short_present(
    flags: &mut Vec<(String, TypedValue)>,
    c: char,
    by_short: &HashMap<char, &FlagSpec>,
) {
    if let Some(sp) = by_short.get(&c) {
        set_typed(flags, sp.name.clone(), TypedValue::Bool(true));
    } else {
        set_typed(flags, c.to_string(), TypedValue::Bool(true));
    }
}

fn apply_short_inline(
    flags: &mut Vec<(String, TypedValue)>,
    c: char,
    val: &str,
    by_short: &HashMap<char, &FlagSpec>,
) {
    if let Some(sp) = by_short.get(&c) {
        if sp.kind == OptionKind::Boolean {
            set_typed(flags, sp.name.clone(), TypedValue::Bool(true));
        } else {
            set_typed(flags, sp.name.clone(), coerce(sp.kind, val));
        }
    } else {
        set_typed(flags, c.to_string(), TypedValue::Str(val.to_string()));
    }
}

/// Designed help text: one line per spec entry (`-s, --long  help` or `      --long  help`).
pub fn flag_help(spec: &[FlagSpec]) -> String {
    let mut out = String::new();
    for s in spec {
        if let Some(c) = s.short {
            out.push_str("  -");
            out.push(c);
            out.push_str(", --");
        } else {
            out.push_str("      --");
        }
        out.push_str(&s.name);
        out.push_str("  ");
        out.push_str(&s.help);
        out.push('\n');
    }
    out
}

pub fn parse_flags_js_polyfill() -> &'static str {
    r#"function parseFlagNumber(s) {
  var t = String(s).replace(/^\s+|\s+$/g, "");
  if (t === "") return 0;
  return Number(t);
}
function readFlagSpec(spec) {
  if (spec === null || typeof spec !== "object" || Array.isArray(spec)) {
    throw new TypeError("parseFlags spec must be an object");
  }
  var list = [];
  var keys = Object.keys(spec);
  var i;
  for (i = 0; i < keys.length; i++) {
    var name = keys[i];
    var opt = spec[name];
    if (opt === null || typeof opt !== "object" || Array.isArray(opt)) {
      throw new TypeError("invalid flag spec");
    }
    var t = opt.type;
    if (t !== "boolean" && t !== "string" && t !== "number") {
      throw new TypeError("invalid flag type");
    }
    var short = null;
    if (typeof opt.short === "string" && opt.short.length === 1) short = opt.short;
    var help = typeof opt.help === "string" ? opt.help : "";
    list.push({ name: name, type: t, short: short, help: help });
  }
  return list;
}
function isFlagValueToken(tok) {
  return !(tok.charAt(0) === "-" && tok !== "-");
}
function parseFlags(argv, spec) {
  if (!Array.isArray(argv)) throw new TypeError("parseFlags expects an array");
  if (spec === undefined) {
    var flags = {};
    var positionals = [];
    var i = 0;
    while (i < argv.length) {
      var tok = argv[i];
      if (typeof tok !== "string") tok = String(tok);
      if (tok === "--") {
        i++;
        while (i < argv.length) {
          var rest = argv[i];
          if (typeof rest !== "string") rest = String(rest);
          positionals.push(rest);
          i++;
        }
        break;
      }
      if (tok.length > 2 && tok.charCodeAt(0) === 45 && tok.charCodeAt(1) === 45) {
        var body = tok.slice(2);
        var eq = body.indexOf("=");
        if (eq >= 0) {
          var name = body.slice(0, eq);
          if (name) flags[name] = body.slice(eq + 1);
        } else if (body) {
          flags[body] = true;
        }
        i++;
        continue;
      }
      if (tok.length > 1 && tok.charCodeAt(0) === 45 && tok !== "-") {
        var sbody = tok.slice(1);
        var seq = sbody.indexOf("=");
        if (seq >= 0) {
          var sname = sbody.slice(0, seq);
          var sval = sbody.slice(seq + 1);
          if (sname.length === 1) {
            flags[sname] = sval;
          } else {
            var k;
            for (k = 0; k < sname.length - 1; k++) flags[sname.charAt(k)] = true;
            if (sname.length > 0) flags[sname.charAt(sname.length - 1)] = sval;
          }
        } else {
          var j;
          for (j = 0; j < sbody.length; j++) flags[sbody.charAt(j)] = true;
        }
        i++;
        continue;
      }
      positionals.push(tok);
      i++;
    }
    return { flags: flags, positionals: positionals };
  }
  var list = readFlagSpec(spec);
  var byLong = {};
  var byShort = {};
  var si;
  for (si = 0; si < list.length; si++) {
    byLong[list[si].name] = list[si];
    if (list[si].short) byShort[list[si].short] = list[si];
  }
  function setTyped(flagsObj, key, value) {
    flagsObj[key] = value;
  }
  function coerceTyped(kind, raw) {
    if (kind === "boolean") return true;
    if (kind === "string") return String(raw);
    return parseFlagNumber(raw);
  }
  var tflags = {};
  var tpos = [];
  var ti = 0;
  while (ti < argv.length) {
    var ttok = argv[ti];
    if (typeof ttok !== "string") ttok = String(ttok);
    if (ttok === "--") {
      ti++;
      while (ti < argv.length) {
        var trest = argv[ti];
        if (typeof trest !== "string") trest = String(trest);
        tpos.push(trest);
        ti++;
      }
      break;
    }
    if (ttok.length > 2 && ttok.charCodeAt(0) === 45 && ttok.charCodeAt(1) === 45) {
      var tbody = ttok.slice(2);
      var teq = tbody.indexOf("=");
      if (teq >= 0) {
        var tname = tbody.slice(0, teq);
        var tval = tbody.slice(teq + 1);
        var tsp = byLong[tname];
        if (tsp) {
          if (tsp.type === "boolean") setTyped(tflags, tsp.name, true);
          else setTyped(tflags, tsp.name, coerceTyped(tsp.type, tval));
        } else if (tname) {
          setTyped(tflags, tname, tval);
        }
      } else {
        var tsp2 = byLong[tbody];
        if (tsp2) {
          if (tsp2.type === "boolean") {
            setTyped(tflags, tsp2.name, true);
          } else if (ti + 1 < argv.length && isFlagValueToken(String(argv[ti + 1]))) {
            ti++;
            setTyped(tflags, tsp2.name, coerceTyped(tsp2.type, argv[ti]));
          }
        } else if (tbody) {
          setTyped(tflags, tbody, true);
        }
      }
      ti++;
      continue;
    }
    if (ttok.length > 1 && ttok.charCodeAt(0) === 45 && ttok !== "-") {
      var tsbody = ttok.slice(1);
      var tseq = tsbody.indexOf("=");
      if (tseq >= 0) {
        var tsname = tsbody.slice(0, tseq);
        var tsval = tsbody.slice(tseq + 1);
        var tci;
        for (tci = 0; tci < tsname.length; tci++) {
          var tc = tsname.charAt(tci);
          var last = tci === tsname.length - 1;
          var ssp = byShort[tc];
          if (!last) {
            if (ssp) setTyped(tflags, ssp.name, true);
            else setTyped(tflags, tc, true);
          } else if (ssp) {
            if (ssp.type === "boolean") setTyped(tflags, ssp.name, true);
            else setTyped(tflags, ssp.name, coerceTyped(ssp.type, tsval));
          } else {
            setTyped(tflags, tc, tsval);
          }
        }
      } else {
        var tsi;
        for (tsi = 0; tsi < tsbody.length; tsi++) {
          var sc = tsbody.charAt(tsi);
          var slast = tsi === tsbody.length - 1;
          var ssp2 = byShort[sc];
          if (ssp2) {
            if (ssp2.type === "boolean") {
              setTyped(tflags, ssp2.name, true);
            } else if (slast && ti + 1 < argv.length && isFlagValueToken(String(argv[ti + 1]))) {
              ti++;
              setTyped(tflags, ssp2.name, coerceTyped(ssp2.type, argv[ti]));
            }
          } else {
            setTyped(tflags, sc, true);
          }
        }
      }
      ti++;
      continue;
    }
    tpos.push(ttok);
    ti++;
  }
  for (si = 0; si < list.length; si++) {
    if (list[si].type === "boolean" && tflags[list[si].name] === undefined) {
      tflags[list[si].name] = false;
    }
  }
  return { flags: tflags, positionals: tpos };
}
function flagHelp(spec) {
  var list = readFlagSpec(spec);
  var lines = [];
  var hi;
  for (hi = 0; hi < list.length; hi++) {
    var h = list[hi];
    if (h.short) lines.push("  -" + h.short + ", --" + h.name + "  " + h.help);
    else lines.push("      --" + h.name + "  " + h.help);
  }
  return lines.length ? lines.join("\n") + "\n" : "";
}
if (typeof globalThis !== "undefined") {
  globalThis.parseFlags = parseFlags;
  globalThis.flagHelp = flagHelp;
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    fn sample_spec() -> Vec<FlagSpec> {
        vec![
            FlagSpec {
                name: "verbose".into(),
                kind: OptionKind::Boolean,
                short: Some('v'),
                help: "verbose output".into(),
            },
            FlagSpec {
                name: "count".into(),
                kind: OptionKind::Number,
                short: Some('c'),
                help: "repeat count".into(),
            },
            FlagSpec {
                name: "name".into(),
                kind: OptionKind::String,
                short: None,
                help: "user name".into(),
            },
        ]
    }

    #[test]
    fn long_and_short_presence_and_positionals() {
        let p = parse_flags(&argv(&["--verbose", "-n", "file.txt"]));
        assert_eq!(
            p.flags,
            vec![
                ("verbose".into(), FlagValue::Present),
                ("n".into(), FlagValue::Present),
            ]
        );
        assert_eq!(p.positionals, vec!["file.txt".to_string()]);
    }

    #[test]
    fn inline_values() {
        let p = parse_flags(&argv(&["--name=alice", "-o=out", "in.txt"]));
        assert_eq!(
            p.flags,
            vec![
                ("name".into(), FlagValue::Value("alice".into())),
                ("o".into(), FlagValue::Value("out".into())),
            ]
        );
        assert_eq!(p.positionals, vec!["in.txt".to_string()]);
    }

    #[test]
    fn clustered_shorts_and_terminator() {
        let p = parse_flags(&argv(&["-abc", "--", "--still"]));
        assert_eq!(
            p.flags,
            vec![
                ("a".into(), FlagValue::Present),
                ("b".into(), FlagValue::Present),
                ("c".into(), FlagValue::Present),
            ]
        );
        assert_eq!(p.positionals, vec!["--still".to_string()]);
    }

    #[test]
    fn empty_argv() {
        let p = parse_flags(&argv(&[]));
        assert!(p.flags.is_empty());
        assert!(p.positionals.is_empty());
    }

    #[test]
    fn typed_bool_string_number_and_positionals() {
        let p = parse_flags_typed(
            &argv(&["-v", "--count", "3", "--name", "alice", "file.txt"]),
            &sample_spec(),
        );
        assert_eq!(
            p.flags,
            vec![
                ("verbose".into(), TypedValue::Bool(true)),
                ("count".into(), TypedValue::Num(3.0)),
                ("name".into(), TypedValue::Str("alice".into())),
            ]
        );
        assert_eq!(p.positionals, vec!["file.txt".to_string()]);
    }

    #[test]
    fn typed_inline_and_missing_bool_false() {
        let p = parse_flags_typed(&argv(&["--count=4.5", "--name=bob"]), &sample_spec());
        assert_eq!(
            p.flags,
            vec![
                ("count".into(), TypedValue::Num(4.5)),
                ("name".into(), TypedValue::Str("bob".into())),
                ("verbose".into(), TypedValue::Bool(false)),
            ]
        );
        assert!(p.positionals.is_empty());
    }

    #[test]
    fn typed_short_number_next_token() {
        let p = parse_flags_typed(&argv(&["-c", "9"]), &sample_spec());
        assert_eq!(
            p.flags,
            vec![
                ("count".into(), TypedValue::Num(9.0)),
                ("verbose".into(), TypedValue::Bool(false)),
            ]
        );
    }

    #[test]
    fn designed_help_text() {
        assert_eq!(
            flag_help(&sample_spec()),
            "  -v, --verbose  verbose output\n  -c, --count  repeat count\n      --name  user name\n"
        );
    }

    #[test]
    fn polyfill_defines_parse_flags_and_flag_help() {
        let s = parse_flags_js_polyfill();
        assert!(s.contains("function parseFlags("), "{s}");
        assert!(s.contains("function flagHelp("), "{s}");
        assert!(s.contains("globalThis.parseFlags = parseFlags"), "{s}");
        assert!(s.contains("globalThis.flagHelp = flagHelp"), "{s}");
        assert!(s.contains("parseFlags expects an array"), "{s}");
        assert!(s.contains("invalid flag type"), "{s}");
    }
}
