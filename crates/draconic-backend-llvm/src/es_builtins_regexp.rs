use super::*;

/// Annex B RegExp constructor statics (B.2.5) — updated on successful match only.
#[derive(Clone, Debug, Default)]
pub(super) struct RegExpStatics {
    input: String,
    /// `$1`…`$9`
    dollar: [String; 9],
    last_match: String,
    last_paren: String,
    left_context: String,
    right_context: String,
}

thread_local! {
    pub(super) static REGEXP_STATICS: std::cell::RefCell<RegExpStatics> =
        std::cell::RefCell::new(RegExpStatics::default());
}

pub(super) fn reset_regexp_statics() {
    REGEXP_STATICS.with(|cell| {
        *cell.borrow_mut() = RegExpStatics::default();
    });
}

pub(super) fn update_regexp_statics(m: &ReMatch, input: &str) {
    let chars: Vec<char> = input.chars().collect();
    REGEXP_STATICS.with(|cell| {
        let mut s = cell.borrow_mut();
        s.input = input.to_string();
        s.last_match = m.full.clone();
        s.left_context = chars[..m.start].iter().collect();
        s.right_context = chars[m.end..].iter().collect();
        for i in 0..9 {
            s.dollar[i] = m.captures.get(i).cloned().unwrap_or_default();
        }
        s.last_paren = m.captures.last().cloned().unwrap_or_default();
    });
}

pub(super) fn regexp_static_get(key: &str) -> Option<String> {
    REGEXP_STATICS.with(|cell| {
        let s = cell.borrow();
        match key {
            "input" | "$_" => Some(s.input.clone()),
            "$1" => Some(s.dollar[0].clone()),
            "$2" => Some(s.dollar[1].clone()),
            "$3" => Some(s.dollar[2].clone()),
            "$4" => Some(s.dollar[3].clone()),
            "$5" => Some(s.dollar[4].clone()),
            "$6" => Some(s.dollar[5].clone()),
            "$7" => Some(s.dollar[6].clone()),
            "$8" => Some(s.dollar[7].clone()),
            "$9" => Some(s.dollar[8].clone()),
            "lastMatch" | "$&" => Some(s.last_match.clone()),
            "lastParen" | "$+" => Some(s.last_paren.clone()),
            "leftContext" | "$`" => Some(s.left_context.clone()),
            "rightContext" | "$'" => Some(s.right_context.clone()),
            _ => None,
        }
    })
}

pub(super) fn regexp_flags_ok(flags: &str) -> bool {
    // Fixture subset: empty or any combo of `i` / `g` (order preserved as given).
    flags.chars().all(|c| matches!(c, 'i' | 'g'))
}

pub(super) fn regexp_compile_args(args: &[JsVal]) -> Result<(String, String), ()> {
    match args.first() {
        Some(JsVal::RegExpInst {
            source: s,
            flags: f,
        }) => {
            let flags = match args.get(1) {
                Some(JsVal::Undef) | None => f.clone(),
                Some(JsVal::Str(fs)) => {
                    if !regexp_flags_ok(fs) {
                        return Err(());
                    }
                    fs.clone()
                }
                _ => return Err(()),
            };
            Ok((s.clone(), flags))
        }
        Some(JsVal::Str(s)) => {
            let flags = match args.get(1) {
                Some(JsVal::Str(fs)) => {
                    if !regexp_flags_ok(fs) {
                        return Err(());
                    }
                    fs.clone()
                }
                Some(JsVal::Undef) | None => String::new(),
                _ => return Err(()),
            };
            Ok((s.clone(), flags))
        }
        Some(JsVal::Undef) | None => {
            let flags = match args.get(1) {
                Some(JsVal::Str(fs)) => {
                    if !regexp_flags_ok(fs) {
                        return Err(());
                    }
                    fs.clone()
                }
                Some(JsVal::Undef) | None => String::new(),
                _ => return Err(()),
            };
            Ok((String::new(), flags))
        }
        _ => Err(()),
    }
}

pub(super) fn make_regexp(args: &[JsVal]) -> Result<JsVal, ()> {
    let source = match args.first() {
        Some(JsVal::Str(s)) => s.clone(),
        Some(JsVal::RegExpInst { source, flags }) => {
            // `new RegExp(re)` / `RegExp(re)` copy when flags omitted.
            let fl = match args.get(1) {
                Some(JsVal::Undef) | None => flags.clone(),
                Some(JsVal::Str(fs)) => {
                    if !regexp_flags_ok(fs) {
                        return Err(());
                    }
                    fs.clone()
                }
                _ => return Err(()),
            };
            parse_regexp_atoms(source)?;
            return Ok(JsVal::RegExpInst {
                source: source.clone(),
                flags: fl,
            });
        }
        Some(JsVal::Undef) | None => String::new(),
        _ => return Err(()),
    };
    let flags = match args.get(1) {
        Some(JsVal::Str(s)) => s.clone(),
        Some(JsVal::Undef) | None => String::new(),
        _ => return Err(()),
    };
    if !regexp_flags_ok(&flags) {
        return Err(());
    }
    // Reject unsupported pattern syntax early (keep classify strict).
    parse_regexp_atoms(&source)?;
    Ok(JsVal::RegExpInst { source, flags })
}

pub(super) fn regexp_proto_method_builtin(key: &str) -> Option<BuiltinId> {
    match key {
        "compile" => Some(BuiltinId::RegExpCompile),
        _ => None,
    }
}

pub(super) fn is_regexp_proto_method(id: BuiltinId) -> bool {
    matches!(id, BuiltinId::RegExpCompile)
}

pub(super) fn regexp_proto_method_name(id: BuiltinId) -> Option<&'static str> {
    match id {
        BuiltinId::RegExpCompile => Some("compile"),
        _ => None,
    }
}

/// Fixture-depth pattern atoms: literal char, `c+` (one-or-more of c), capturing `(…)`,
/// identity escapes (`\/`), or simple `[…]` character classes.
#[derive(Clone, Debug)]
pub(super) enum ReAtom {
    Lit(char),
    Plus(char),
    /// Capturing group; numbered left-to-right by appearance.
    Group(Vec<ReAtom>),
    /// Simple character class `[abc]` (no ranges/escapes/negation in this subset).
    Class(Vec<char>),
}

/// Successful match result for `exec`/`test` + Annex B statics.
#[derive(Clone, Debug)]
pub(super) struct ReMatch {
    pub(super) full: String,
    /// Capturing groups in order (`$1`…); always defined strings in this subset.
    pub(super) captures: Vec<String>,
    /// Char indices into the input string.
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn parse_regexp_atoms(pattern: &str) -> Result<Vec<ReAtom>, ()> {
    let chars: Vec<char> = pattern.chars().collect();
    let (atoms, i) = parse_atoms_until(&chars, 0, false)?;
    if i != chars.len() {
        return Err(());
    }
    Ok(atoms)
}

/// Parse atoms until end (or `)` when `stop_on_close`).
/// Returns `(atoms, index)` where index is at end or at the closing `)`.
pub(super) fn parse_atoms_until(
    chars: &[char],
    mut i: usize,
    stop_on_close: bool,
) -> Result<(Vec<ReAtom>, usize), ()> {
    let mut atoms = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c == ')' {
            if stop_on_close {
                return Ok((atoms, i));
            }
            return Err(());
        }
        if c == '(' {
            i += 1;
            let (inner, ni) = parse_atoms_until(chars, i, true)?;
            if ni >= chars.len() || chars[ni] != ')' {
                return Err(());
            }
            atoms.push(ReAtom::Group(inner));
            i = ni + 1;
            continue;
        }
        // Identity escapes (fixture: `\/`); store escaped char as literal.
        if c == '\\' {
            if i + 1 >= chars.len() {
                return Err(());
            }
            let esc = chars[i + 1];
            // Reject other metachar quantifiers after escape in this subset.
            if i + 2 < chars.len() && chars[i + 2] == '+' {
                atoms.push(ReAtom::Plus(esc));
                i += 3;
            } else {
                atoms.push(ReAtom::Lit(esc));
                i += 2;
            }
            continue;
        }
        // Simple character class `[…]` (no `^` negation, ranges, or escapes).
        if c == '[' {
            i += 1;
            let mut class_chars = Vec::new();
            while i < chars.len() && chars[i] != ']' {
                let cc = chars[i];
                if matches!(cc, '\\' | '[' | '^') {
                    return Err(());
                }
                class_chars.push(cc);
                i += 1;
            }
            if i >= chars.len() || chars[i] != ']' || class_chars.is_empty() {
                return Err(());
            }
            i += 1; // closing `]`
            atoms.push(ReAtom::Class(class_chars));
            continue;
        }
        // No other classes / other quantifiers in this subset.
        if matches!(c, '.' | '*' | '?' | ']' | '{' | '}' | '|' | '^' | '$') {
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
    if stop_on_close {
        // Unclosed `(`.
        return Err(());
    }
    Ok((atoms, i))
}

pub(super) fn char_eq(a: char, b: char, ignore_case: bool) -> bool {
    if ignore_case {
        a.eq_ignore_ascii_case(&b)
    } else {
        a == b
    }
}

/// First match of fixture-subset pattern in `input`, or None.
pub(super) fn regexp_find(pattern: &str, flags: &str, input: &str) -> Option<ReMatch> {
    let atoms = parse_regexp_atoms(pattern).ok()?;
    let ignore_case = flags.contains('i');
    let chars: Vec<char> = input.chars().collect();
    for start in 0..=chars.len() {
        let mut caps = Vec::new();
        if let Some(end) = regexp_match_at(&atoms, &chars, start, ignore_case, &mut caps) {
            let full: String = chars[start..end].iter().collect();
            let captures: Vec<String> = caps
                .into_iter()
                .map(|(s, e)| chars[s..e].iter().collect())
                .collect();
            return Some(ReMatch {
                full,
                captures,
                start,
                end,
            });
        }
    }
    None
}

/// Match `atoms` at `start`. Appends capture spans `(start,end)` left-to-right.
pub(super) fn regexp_match_at(
    atoms: &[ReAtom],
    input: &[char],
    start: usize,
    ignore_case: bool,
    captures: &mut Vec<(usize, usize)>,
) -> Option<usize> {
    let mut pos = start;
    for atom in atoms {
        match atom {
            ReAtom::Lit(c) => {
                if pos >= input.len() || !char_eq(input[pos], *c, ignore_case) {
                    return None;
                }
                pos += 1;
            }
            ReAtom::Plus(c) => {
                if pos >= input.len() || !char_eq(input[pos], *c, ignore_case) {
                    return None;
                }
                pos += 1;
                while pos < input.len() && char_eq(input[pos], *c, ignore_case) {
                    pos += 1;
                }
            }
            ReAtom::Class(set) => {
                if pos >= input.len() {
                    return None;
                }
                let ch = input[pos];
                let ok = set.iter().any(|c| char_eq(ch, *c, ignore_case));
                if !ok {
                    return None;
                }
                pos += 1;
            }
            ReAtom::Group(inner) => {
                // Number this group before descending so outer groups get lower indices.
                let idx = captures.len();
                captures.push((0, 0));
                let gstart = pos;
                let end = regexp_match_at(inner, input, pos, ignore_case, captures)?;
                captures[idx] = (gstart, end);
                pos = end;
            }
        }
    }
    Some(pos)
}
