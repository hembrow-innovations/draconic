use crate::lexer::{hex_digit, Lexer};
use crate::scan_ident::is_ident_start_char;
use crate::token::TokenKind;
use draconic_diagnostics::{Diagnostic, Span};

impl Lexer<'_> {
    pub(crate) fn number_literal(&mut self) -> Result<TokenKind, Diagnostic> {
        let start = self.pos;
        // Non-decimal integer: 0x / 0b / 0o (case-insensitive prefix).
        if self.peek() == b'0' {
            match self.peek_at(1) {
                Some(b'x' | b'X') => {
                    self.bump(); // 0
                    self.bump(); // x
                    self.scan_radix_digits(16, start)?;
                    return self.finish_number_or_bigint(start, true);
                }
                Some(b'b' | b'B') => {
                    self.bump();
                    self.bump();
                    self.scan_radix_digits(2, start)?;
                    return self.finish_number_or_bigint(start, true);
                }
                Some(b'o' | b'O') => {
                    self.bump();
                    self.bump();
                    self.scan_radix_digits(8, start)?;
                    return self.finish_number_or_bigint(start, true);
                }
                // Annex B.1.1: `0` + digit → LegacyOctalIntegerLiteral or NonOctalDecimalIntegerLiteral.
                // Also reject numeric separators after a lone leading `0` (`0_1`).
                Some(b'0'..=b'9' | b'_') => {
                    return self.scan_zero_prefixed_decimal_or_legacy_octal(start);
                }
                _ => {}
            }
        }

        // DecimalIntegerLiteral (with optional numeric separators).
        self.scan_decimal_integer_digits(start)?;

        // Optional fractional part: `.` DecimalDigits_opt (then ExponentPart_opt).
        // ECMA-262: DecimalIntegerLiteral `.` DecimalDigits_opt — empty fraction is valid
        // (`1.` `10.` `0.`). Always consume one `.` here so `1..x` is `1.` + `.` + `x`.
        let mut is_integer = true;
        if !self.is_eof() && self.peek() == b'.' {
            self.bump(); // .
                         // DecimalDigits_opt — empty ok; `_` alone / leading `_` invalid.
            if !self.is_eof() && (self.peek().is_ascii_digit() || self.peek() == b'_') {
                self.scan_decimal_digits_required(start)?;
            }
            is_integer = false;
        }

        let had_exponent = self.scan_exponent_opt(start)?;
        if had_exponent {
            is_integer = false;
        }
        self.finish_number_or_bigint(start, is_integer)
    }

    /// Annex B.1.1 / DecimalIntegerLiteral NonOctalDecimal branch after a leading `0`
    /// when the next character is a digit or `_`.
    ///
    /// - Pure octal digits → LegacyOctalIntegerLiteral (MV base-8); no `.`/`e`/`n`.
    /// - Any `8`/`9` → NonOctalDecimalIntegerLiteral (MV decimal); `.`/`e` allowed; no `n`.
    /// - `_` after leading `0` is always invalid.
    pub(crate) fn scan_zero_prefixed_decimal_or_legacy_octal(
        &mut self,
        start: usize,
    ) -> Result<TokenKind, Diagnostic> {
        debug_assert_eq!(self.peek(), b'0');
        self.bump(); // leading 0

        if !self.is_eof() && self.peek() == b'_' {
            return Err(Diagnostic::new(
                "numeric separator cannot be used after leading 0",
                Span::new(start as u32, (self.pos + 1) as u32),
            ));
        }

        let mut has_non_octal = false;
        while !self.is_eof() {
            let b = self.peek();
            if b.is_ascii_digit() {
                if b == b'8' || b == b'9' {
                    has_non_octal = true;
                }
                self.bump();
                continue;
            }
            if b == b'_' {
                return Err(Diagnostic::new(
                    "numeric separator cannot be used after leading 0",
                    Span::new(start as u32, (self.pos + 1) as u32),
                ));
            }
            break;
        }

        if has_non_octal {
            // NonOctalDecimalIntegerLiteral: optional fraction + exponent (decimal MV).
            // Empty fraction is valid (`089.`); always consume one `.` when present.
            if !self.is_eof() && self.peek() == b'.' {
                self.bump(); // .
                if !self.is_eof() && (self.peek().is_ascii_digit() || self.peek() == b'_') {
                    self.scan_decimal_digits_required(start)?;
                }
            }
            self.scan_exponent_opt(start)?;
            // BigInt suffix not allowed on zero-prefixed multi-digit forms.
            if !self.is_eof() && self.peek() == b'n' && !self.is_ident_continue_at(self.pos + 1) {
                return Err(Diagnostic::new(
                    "Invalid BigInt literal",
                    Span::new(start as u32, (self.pos + 1) as u32),
                ));
            }
            self.reject_numeric_followed_by_ident(start)?;
            let raw = self.src[start..self.pos].to_string();
            // E19.69: NonOctalDecimalIntegerLiteral is SyntaxError in strict mode.
            self.pending_legacy_octal = true;
            return Ok(TokenKind::Number(canonicalize_leading_zero_decimal(&raw)));
        }

        // LegacyOctalIntegerLiteral: do not consume `.`/`e` as part of this token.
        if !self.is_eof() && self.peek() == b'n' && !self.is_ident_continue_at(self.pos + 1) {
            return Err(Diagnostic::new(
                "Invalid BigInt literal",
                Span::new(start as u32, (self.pos + 1) as u32),
            ));
        }
        self.reject_numeric_followed_by_ident(start)?;
        let raw = &self.src[start..self.pos];
        let mv = legacy_octal_mv(raw);
        // E19.69: LegacyOctalIntegerLiteral is SyntaxError in strict mode.
        self.pending_legacy_octal = true;
        Ok(TokenKind::Number(mv))
    }

    /// Leading-dot decimal: `.` DecimalDigits ExponentPart_opt
    pub(crate) fn number_literal_leading_dot(&mut self) -> Result<TokenKind, Diagnostic> {
        let start = self.pos;
        self.bump(); // .
        self.scan_decimal_digits_required(start)?;
        self.scan_exponent_opt(start)?;
        // Leading-dot forms are never BigInt (`n` after a float is invalid).
        self.finish_number_or_bigint(start, false)
    }

    /// Finish a numeric token; optional `n` suffix yields BigInt when `allow_bigint`.
    pub(crate) fn finish_number_or_bigint(
        &mut self,
        start: usize,
        allow_bigint: bool,
    ) -> Result<TokenKind, Diagnostic> {
        if !self.is_eof() && self.peek() == b'n' && !self.is_ident_continue_at(self.pos + 1) {
            if !allow_bigint {
                return Err(Diagnostic::new(
                    "Invalid BigInt literal",
                    Span::new(start as u32, (self.pos + 1) as u32),
                ));
            }
            self.bump(); // n
                         // E19.67: NumericLiteral must not be followed by IdentifierStart.
            self.reject_numeric_followed_by_ident(start)?;
            let raw = self.src[start..self.pos].to_string();
            return Ok(TokenKind::BigInt(raw));
        }
        // E19.67: NumericLiteral must not be followed by IdentifierStart (`3in`).
        self.reject_numeric_followed_by_ident(start)?;
        let raw = self.src[start..self.pos].to_string();
        Ok(TokenKind::Number(raw))
    }

    /// ECMA-262: The SourceCharacter immediately following a NumericLiteral must not
    /// be an IdentifierStart or DecimalDigit.
    pub(crate) fn reject_numeric_followed_by_ident(&self, start: usize) -> Result<(), Diagnostic> {
        if self.is_eof() {
            return Ok(());
        }
        let ch = self.peek_char();
        if is_ident_start_char(ch) {
            return Err(Diagnostic::new(
                "numeric literal cannot be immediately followed by identifier".to_string(),
                Span::new(start as u32, (self.pos + ch.len_utf8()) as u32),
            ));
        }
        Ok(())
    }

    /// Returns `true` if an exponent part was consumed.
    pub(crate) fn scan_exponent_opt(&mut self, start: usize) -> Result<bool, Diagnostic> {
        if self.is_eof() {
            return Ok(false);
        }
        let e = self.peek();
        if e != b'e' && e != b'E' {
            return Ok(false);
        }
        self.bump();
        if !self.is_eof() && (self.peek() == b'+' || self.peek() == b'-') {
            self.bump();
        }
        self.scan_decimal_digits_required(start)?;
        Ok(true)
    }

    /// Decimal integer digits with optional `_` separators (at least one digit already at pos).
    pub(crate) fn scan_decimal_integer_digits(&mut self, start: usize) -> Result<(), Diagnostic> {
        if self.is_eof() || !self.peek().is_ascii_digit() {
            return Err(Diagnostic::new(
                "invalid number literal",
                Span::new(start as u32, self.pos as u32),
            ));
        }
        self.scan_decimal_digits_required(start)
    }

    /// One or more decimal digits with optional `_` between digits (not leading/trailing/adjacent).
    pub(crate) fn scan_decimal_digits_required(&mut self, start: usize) -> Result<(), Diagnostic> {
        if self.is_eof() || !self.peek().is_ascii_digit() {
            return Err(Diagnostic::new(
                "invalid number literal",
                Span::new(start as u32, self.pos as u32),
            ));
        }
        self.bump();
        loop {
            if self.is_eof() {
                break;
            }
            if self.peek().is_ascii_digit() {
                self.bump();
                continue;
            }
            if self.peek() == b'_' {
                let after = self.peek_at(1);
                if after.is_some_and(|b| b.is_ascii_digit()) {
                    self.bump(); // _
                    self.bump(); // digit
                    continue;
                }
                return Err(Diagnostic::new(
                    "invalid numeric separator in number literal",
                    Span::new(start as u32, (self.pos + 1) as u32),
                ));
            }
            break;
        }
        Ok(())
    }

    /// Radix digits after `0x`/`0b`/`0o` prefix; requires ≥1 digit; allows `_` separators.
    pub(crate) fn scan_radix_digits(&mut self, radix: u32, start: usize) -> Result<(), Diagnostic> {
        let is_digit = |b: u8| -> bool {
            match radix {
                2 => b == b'0' || b == b'1',
                8 => (b'0'..=b'7').contains(&b),
                16 => hex_digit(b).is_some(),
                _ => false,
            }
        };

        if self.is_eof() || !is_digit(self.peek()) {
            return Err(Diagnostic::new(
                "invalid number literal",
                Span::new(start as u32, self.pos as u32),
            ));
        }
        self.bump();
        loop {
            if self.is_eof() {
                break;
            }
            if is_digit(self.peek()) {
                self.bump();
                continue;
            }
            if self.peek() == b'_' {
                let after = self.peek_at(1);
                if after.is_some_and(&is_digit) {
                    self.bump(); // _
                    self.bump(); // digit
                    continue;
                }
                return Err(Diagnostic::new(
                    "invalid numeric separator in number literal",
                    Span::new(start as u32, (self.pos + 1) as u32),
                ));
            }
            break;
        }
        Ok(())
    }
}

/// Annex B.1.1 LegacyOctalIntegerLiteral mathematical value as a decimal digit string.
fn legacy_octal_mv(raw: &str) -> String {
    let mut val: u64 = 0;
    let mut overflow = false;
    for b in raw.bytes() {
        debug_assert!((b'0'..=b'7').contains(&b));
        match val
            .checked_mul(8)
            .and_then(|v| v.checked_add(u64::from(b - b'0')))
        {
            Some(next) => val = next,
            None => {
                overflow = true;
                break;
            }
        }
    }
    if !overflow {
        return val.to_string();
    }
    // Past u64: f64 MV (JS Number semantics for large integers).
    let mut f = 0.0f64;
    for b in raw.bytes() {
        f = f * 8.0 + f64::from(b - b'0');
    }
    if f.is_finite() && f.fract() == 0.0 && f.abs() <= (1u64 << 53) as f64 {
        format!("{}", f as u64)
    } else {
        format!("{f}")
    }
}

/// Strip redundant leading zeros from a NonOctalDecimalIntegerLiteral (and optional frac/exp).
fn canonicalize_leading_zero_decimal(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() && bytes[i] == b'0' && bytes[i + 1].is_ascii_digit() {
        i += 1;
    }
    raw[i..].to_string()
}

#[cfg(test)]
mod tests {
    use crate::{Lexer, TokenKind};

    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::new(src)
            .tokenize()
            .expect("lex")
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn lex_float() {
        assert_eq!(
            kinds("3.14"),
            vec![TokenKind::Number("3.14".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn lex_number_scientific() {
        assert_eq!(
            kinds("1e3 1.5E+2 2e-1"),
            vec![
                TokenKind::Number("1e3".into()),
                TokenKind::Number("1.5E+2".into()),
                TokenKind::Number("2e-1".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_number_radix() {
        assert_eq!(
            kinds("0xff 0b1010 0o17 0XFF 0B10 0O7"),
            vec![
                TokenKind::Number("0xff".into()),
                TokenKind::Number("0b1010".into()),
                TokenKind::Number("0o17".into()),
                TokenKind::Number("0XFF".into()),
                TokenKind::Number("0B10".into()),
                TokenKind::Number("0O7".into()),
                TokenKind::Eof,
            ]
        );
    }

    /// Annex B.1.1: legacy octal MV rewritten to decimal; NonOctalDecimal stays decimal.
    #[test]
    fn lex_legacy_octal_numeric_literals() {
        assert_eq!(
            kinds("010 077 00 0010 0123"),
            vec![
                TokenKind::Number("8".into()),
                TokenKind::Number("63".into()),
                TokenKind::Number("0".into()),
                TokenKind::Number("8".into()),
                TokenKind::Number("83".into()),
                TokenKind::Eof,
            ]
        );
        // E19.69: zero-prefixed multi-digit forms set legacy_octal for strict rejection.
        let toks = Lexer::new("00 08 1").tokenize().unwrap();
        assert!(toks[0].legacy_octal, "00");
        assert!(toks[1].legacy_octal, "08");
        assert!(!toks[2].legacy_octal, "1");
        assert_eq!(
            kinds("08 09 089 0008 08.5 08e2"),
            vec![
                TokenKind::Number("8".into()),
                TokenKind::Number("9".into()),
                TokenKind::Number("89".into()),
                TokenKind::Number("8".into()),
                TokenKind::Number("8.5".into()),
                TokenKind::Number("8e2".into()),
                TokenKind::Eof,
            ]
        );
        // Pure legacy octal does not swallow `.digit` (next token is leading-dot number).
        assert_eq!(
            kinds("010.5"),
            vec![
                TokenKind::Number("8".into()),
                TokenKind::Number(".5".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_number_leading_dot() {
        assert_eq!(
            kinds(".5 .25e1"),
            vec![
                TokenKind::Number(".5".into()),
                TokenKind::Number(".25e1".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_number_separators() {
        assert_eq!(
            kinds("1_000 0xFF_FF 0b1010_0001 1_000.5_00 1e1_0"),
            vec![
                TokenKind::Number("1_000".into()),
                TokenKind::Number("0xFF_FF".into()),
                TokenKind::Number("0b1010_0001".into()),
                TokenKind::Number("1_000.5_00".into()),
                TokenKind::Number("1e1_0".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_bigint_literals() {
        assert_eq!(
            kinds("1n 0n 0xffn 0b1010n 0o17n 1_000n 0xFF_FFn"),
            vec![
                TokenKind::BigInt("1n".into()),
                TokenKind::BigInt("0n".into()),
                TokenKind::BigInt("0xffn".into()),
                TokenKind::BigInt("0b1010n".into()),
                TokenKind::BigInt("0o17n".into()),
                TokenKind::BigInt("1_000n".into()),
                TokenKind::BigInt("0xFF_FFn".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_bigint_rejects_float_suffix() {
        let err = Lexer::new("1.0n").tokenize().unwrap_err();
        assert!(
            err.message.contains("Invalid BigInt"),
            "unexpected: {}",
            err.message
        );
        let err = Lexer::new("1e2n").tokenize().unwrap_err();
        assert!(
            err.message.contains("Invalid BigInt"),
            "unexpected: {}",
            err.message
        );
    }

    /// E19.51: trailing-dot DecimalLiteral (`1.` / `0.` / `10.`).
    #[test]
    fn lex_number_trailing_dot() {
        assert_eq!(
            kinds("1. 0. 10."),
            vec![
                TokenKind::Number("1.".into()),
                TokenKind::Number("0.".into()),
                TokenKind::Number("10.".into()),
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("1.;"),
            vec![
                TokenKind::Number("1.".into()),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
        // Double-dot: trailing-dot number then member `.`.
        assert_eq!(
            kinds("1..toString"),
            vec![
                TokenKind::Number("1.".into()),
                TokenKind::Dot,
                TokenKind::Ident("toString".into()),
                TokenKind::Eof,
            ]
        );
        // `1.toString`: NumericLiteral must not be followed by IdentifierStart (E19.67).
        assert!(
            Lexer::new("1.toString").tokenize().is_err(),
            "1.toString must be a lexical early error"
        );
        assert!(
            Lexer::new("3in").tokenize().is_err(),
            "3in must be a lexical early error"
        );
        // Space allows integer + member access.
        assert_eq!(
            kinds("1 .toString"),
            vec![
                TokenKind::Number("1".into()),
                TokenKind::Dot,
                TokenKind::Ident("toString".into()),
                TokenKind::Eof,
            ]
        );
        // `10.e1` is a DecimalLiteral with empty fraction + exponent.
        assert_eq!(
            kinds("10.e1"),
            vec![TokenKind::Number("10.e1".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn lex_number_rejects_separator_after_dot() {
        // `10._` / `10._e1` / `10._1`: `_` cannot start DecimalDigits after `.`.
        for src in ["10._", "10._e1", "10._1"] {
            let err = Lexer::new(src).tokenize().unwrap_err();
            assert!(
                err.message.contains("invalid number") || err.message.contains("numeric separator"),
                "src={src:?} unexpected: {}",
                err.message
            );
        }
    }
}
