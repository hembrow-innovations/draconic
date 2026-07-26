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
        return Err(
            "invalid regular expression flags: 'u' and 'v' are mutually exclusive".into(),
        );
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
    match regress::Regex::with_flags(pattern, pattern_flags.as_str()) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("invalid regular expression pattern: {e}")),
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
}
