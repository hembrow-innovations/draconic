//! L07.01: parse long/short flags + leftover positionals from a string array.

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

pub fn parse_flags_js_polyfill() -> &'static str {
    r#"function parseFlags(argv) {
  if (!Array.isArray(argv)) throw new TypeError("parseFlags expects an array");
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
if (typeof globalThis !== "undefined") globalThis.parseFlags = parseFlags;
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
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
    fn polyfill_defines_parse_flags() {
        let s = parse_flags_js_polyfill();
        assert!(s.contains("function parseFlags("), "{s}");
        assert!(s.contains("globalThis.parseFlags = parseFlags"), "{s}");
        assert!(s.contains("parseFlags expects an array"), "{s}");
    }
}
