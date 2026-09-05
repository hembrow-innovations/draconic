//! RegExp literal early-error checks (ECMA-262 PrimaryExpression : RegularExpressionLiteral).

/// Validate pattern + flags for a RegExp literal. Errors are early SyntaxErrors.
pub fn validate_regexp_literal(pattern: &str, flags: &str) -> Result<(), String> {
    validate_regexp_flags(flags)?;
    validate_regexp_pattern(pattern, flags)?;
    Ok(())
}

/// FlagText: only `d g i m s u v y`, each at most once; `u` and `v` exclusive.
pub fn validate_regexp_flags(flags: &str) -> Result<(), String> {
    let mut seen = [false; 128];
    let mut has_u = false;
    let mut has_v = false;
    for c in flags.chars() {
        if !c.is_ascii() || (c as u32) >= 128 {
            return Err(format!(
                "invalid regular expression flag '{}'",
                c.escape_default()
            ));
        }
        let idx = c as usize;
        let ok = matches!(c, 'd' | 'g' | 'i' | 'm' | 's' | 'u' | 'v' | 'y');
        if !ok {
            return Err(format!("invalid regular expression flag '{c}'"));
        }
        if seen[idx] {
            return Err(format!("duplicate regular expression flag '{c}'"));
        }
        seen[idx] = true;
        if c == 'u' {
            has_u = true;
        }
        if c == 'v' {
            has_v = true;
        }
    }
    if has_u && has_v {
        return Err("invalid regular expression flags: 'u' and 'v' are mutually exclusive".into());
    }
    Ok(())
}

/// BodyText must be a valid Pattern (with flag-dependent grammar, e.g. unicode mode).
fn validate_regexp_pattern(pattern: &str, flags: &str) -> Result<(), String> {
    // Only flags that affect Pattern parse/semantics matter for early errors.
    let mut pattern_flags = String::new();
    for c in flags.chars() {
        if matches!(c, 'i' | 'm' | 's' | 'u' | 'v') {
            pattern_flags.push(c);
        }
    }
    // ECMA-262 + UTS#24: Script / Script_Extensions accept special value Unknown (Zzzz).
    // regress omits these; rewrite only that value so the rest of the pattern still validates.
    let normalized = rewrite_script_unknown_for_validate(pattern);
    match regress::Regex::with_flags(normalized.as_str(), pattern_flags.as_str()) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("invalid regular expression pattern: {e}")),
    }
}

/// Map `Script` / `Script_Extensions` special value `Unknown`/`Zzzz` → `Latin` for regress.
///
/// Only rewrites well-formed `\p{…}` / `\P{…}` property escapes; other text is unchanged.
fn rewrite_script_unknown_for_validate(pattern: &str) -> String {
    let bytes = pattern.as_bytes();
    let mut out = String::with_capacity(pattern.len());
    let mut i = 0;
    while i < bytes.len() {
        // Look for \p{ or \P{
        if bytes[i] == b'\\'
            && i + 2 < bytes.len()
            && (bytes[i + 1] == b'p' || bytes[i + 1] == b'P')
            && bytes[i + 2] == b'{'
        {
            if let Some(end) = pattern[i + 3..].find('}') {
                let inner = &pattern[i + 3..i + 3 + end];
                out.push('\\');
                out.push(bytes[i + 1] as char);
                out.push('{');
                out.push_str(&rewrite_script_unknown_inner(inner));
                out.push('}');
                i = i + 3 + end + 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn rewrite_script_unknown_inner(inner: &str) -> String {
    // Loose match is not applied here; ECMA uses exact names after UnicodeMatchProperty.
    // Accept both canonical names and short aliases for the Script* properties.
    let eq = match inner.find('=') {
        Some(i) => i,
        None => return inner.to_string(),
    };
    let name = inner[..eq].trim();
    let value = inner[eq + 1..].trim();
    let is_script = matches!(name, "Script" | "sc" | "Script_Extensions" | "scx");
    let is_unknown = matches!(value, "Unknown" | "Zzzz");
    if is_script && is_unknown {
        format!("{name}=Latin")
    } else {
        inner.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_ok() {
        assert!(validate_regexp_flags("").is_ok());
        assert!(validate_regexp_flags("i").is_ok());
        assert!(validate_regexp_flags("gimsuy").is_ok());
        assert!(validate_regexp_flags("dgimsvy").is_ok());
        assert!(validate_regexp_flags("dgimsuy").is_ok());
    }

    #[test]
    fn flags_reject_bad() {
        assert!(validate_regexp_flags("G").is_err());
        assert!(validate_regexp_flags("x").is_err());
        assert!(validate_regexp_flags("z").is_err());
    }

    #[test]
    fn flags_reject_duplicate() {
        assert!(validate_regexp_flags("gig").is_err());
        assert!(validate_regexp_flags("ii").is_err());
        assert!(validate_regexp_flags("uyy").is_err());
    }

    #[test]
    fn flags_reject_u_and_v() {
        assert!(validate_regexp_flags("uv").is_err());
        assert!(validate_regexp_flags("vu").is_err());
    }

    #[test]
    fn pattern_reject_invalid() {
        assert!(validate_regexp_literal("?", "").is_err());
        assert!(validate_regexp_literal("+", "").is_err());
        assert!(validate_regexp_literal("(", "").is_err());
        assert!(validate_regexp_literal("a{2,1}", "").is_err());
    }

    #[test]
    fn pattern_ok_basic() {
        assert!(validate_regexp_literal("a+b", "i").is_ok());
        assert!(validate_regexp_literal(".", "").is_ok());
        assert!(validate_regexp_literal("[a/]", "").is_ok());
        assert!(validate_regexp_literal(r"a\/b", "").is_ok());
    }

    #[test]
    fn property_escapes_ok() {
        assert!(validate_regexp_literal(r"\p{ASCII}", "u").is_ok());
        assert!(validate_regexp_literal(r"\P{ASCII}", "u").is_ok());
        assert!(validate_regexp_literal(r"^\p{ASCII}+$", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{Script=Latin}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{gc=Nd}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{Any}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{Emoji}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{General_Category=Letter}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{Basic_Emoji}", "v").is_ok());
    }

    #[test]
    fn property_escapes_script_unknown_zzzz() {
        // UTS#24 special value; required by Test262 special-property-value-Script_* tests.
        assert!(validate_regexp_literal(r"\p{Script_Extensions=Unknown}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{Script_Extensions=Zzzz}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{scx=Unknown}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{scx=Zzzz}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{Script=Unknown}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{Script=Zzzz}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{sc=Unknown}", "u").is_ok());
        assert!(validate_regexp_literal(r"\p{sc=Zzzz}", "u").is_ok());
    }

    #[test]
    fn property_escapes_still_reject_bogus() {
        assert!(validate_regexp_literal(r"\p{NotARealProperty}", "u").is_err());
        assert!(validate_regexp_literal(r"\p{Script=NotARealScript}", "u").is_err());
        // Without u/v, `\p` is not a property escape (IdentityEscape / literal path).
        assert!(validate_regexp_literal(r"\p{ASCII}", "").is_ok());
    }

    #[test]
    fn rewrite_unknown_preserves_other_text() {
        let s = rewrite_script_unknown_for_validate(r"a\p{scx=Unknown}b\p{ASCII}c");
        assert_eq!(s, r"a\p{scx=Latin}b\p{ASCII}c");
        assert_eq!(
            rewrite_script_unknown_for_validate(r"\P{Script=Zzzz}"),
            r"\P{Script=Latin}"
        );
    }
}
