//! F07.01: parse a C header subset — function decls with scalar/pointer params.
//! F07.02: emit Draconic `extern "C"` decls from a parsed header.
//! F07.03: default extern-module path for `draconic bindgen`.
//! F07.04: simple structs + typedef names (no full C).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub functions: Vec<FnDecl>,
    items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Item {
    Struct(StructDecl),
    Typedef(TypedefDecl),
    Function(FnDecl),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StructDecl {
    name: String,
    fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    name: String,
    ty: CType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TypedefDecl {
    name: String,
    ty: CType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnDecl {
    pub name: String,
    pub return_ty: CType,
    pub params: Vec<Param>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Option<String>,
    pub ty: CType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CType {
    Void,
    Char,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    LongLong,
    ULongLong,
    Float,
    Double,
    Pointer(Box<CType>),
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse_header(src: &str) -> Result<Header, ParseError> {
    let tokens = tokenize(src)?;
    let mut i = 0;
    let mut functions = Vec::new();
    let mut items = Vec::new();
    while !matches!(peek(&tokens, i), Tok::Eof) {
        while ident_eq(peek(&tokens, i), "extern")
            || ident_eq(peek(&tokens, i), "static")
            || ident_eq(peek(&tokens, i), "inline")
        {
            i += 1;
        }
        if ident_eq(peek(&tokens, i), "typedef") {
            items.push(parse_typedef(&tokens, &mut i)?);
            continue;
        }
        if is_struct_def(&tokens, i) {
            items.push(parse_struct_item(&tokens, &mut i)?);
            continue;
        }
        let f = parse_fn_decl(&tokens, &mut i)?;
        functions.push(f.clone());
        items.push(Item::Function(f));
    }
    Ok(Header { functions, items })
}

pub fn emit_externs(header: &Header) -> String {
    let mut out = String::new();
    let mut emitted_types: Vec<String> = Vec::new();
    for item in &header.items {
        match item {
            Item::Struct(s) => {
                if emitted_types.iter().any(|n| n == &s.name) {
                    continue;
                }
                out.push_str(&emit_struct(s));
                emitted_types.push(s.name.clone());
            }
            Item::Typedef(t) => {
                if emitted_types.iter().any(|n| n == &t.name) {
                    continue;
                }
                if matches!(&t.ty, CType::Named(n) if n == &t.name) {
                    continue;
                }
                out.push_str(&emit_typedef(t));
                emitted_types.push(t.name.clone());
            }
            Item::Function(f) => {
                out.push_str(&emit_fn(f));
                out.push('\n');
            }
        }
    }
    out
}

/// Sibling `.drac` path for `draconic bindgen <header>` when `-o` is omitted.
pub fn default_extern_module_path(header: &Path) -> PathBuf {
    header.with_extension("drac")
}

fn emit_struct(s: &StructDecl) -> String {
    let fields = s
        .fields
        .iter()
        .map(|f| format!("{}: {}", f.name, emit_ty(&f.ty)))
        .collect::<Vec<_>>()
        .join("; ");
    format!("type {} = {{ {fields} }};\n", s.name)
}

fn emit_typedef(t: &TypedefDecl) -> String {
    format!("type {} = {};\n", t.name, emit_ty(&t.ty))
}

fn emit_fn(f: &FnDecl) -> String {
    let params = f
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let name = p.name.clone().unwrap_or_else(|| format!("p{i}"));
            format!("{}: {}", name, emit_ty(&p.ty))
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "extern \"C\" function {}({}): {};",
        f.name,
        params,
        emit_ty(&f.return_ty)
    )
}

fn emit_ty(ty: &CType) -> String {
    match ty {
        CType::Void => "void".into(),
        CType::Char => "i8".into(),
        CType::UChar => "u8".into(),
        CType::Short => "i16".into(),
        CType::UShort => "u16".into(),
        CType::Int => "i32".into(),
        CType::UInt => "u32".into(),
        CType::Long => "i64".into(),
        CType::ULong => "u64".into(),
        CType::LongLong => "i64".into(),
        CType::ULongLong => "u64".into(),
        CType::Float => "f32".into(),
        CType::Double => "f64".into(),
        CType::Pointer(inner) => match inner.as_ref() {
            CType::Void | CType::Char | CType::UChar => "*u8".into(),
            other => format!("*{}", emit_ty(other)),
        },
        CType::Named(n) => n.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Ident(String),
    Star,
    LParen,
    RParen,
    Comma,
    Semi,
    LBrace,
    RBrace,
    Colon,
    Eof,
}

fn err(msg: impl Into<String>) -> ParseError {
    ParseError {
        message: msg.into(),
    }
}

fn tokenize(src: &str) -> Result<Vec<Tok>, ParseError> {
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            loop {
                if i + 1 >= bytes.len() {
                    return Err(err("unterminated block comment"));
                }
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }
        match c {
            b'*' => {
                out.push(Tok::Star);
                i += 1;
            }
            b'(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            b')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            b',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            b';' => {
                out.push(Tok::Semi);
                i += 1;
            }
            b'{' => {
                out.push(Tok::LBrace);
                i += 1;
            }
            b'}' => {
                out.push(Tok::RBrace);
                i += 1;
            }
            b':' => {
                out.push(Tok::Colon);
                i += 1;
            }
            _ if is_ident_start(c) => {
                let start = i;
                i += 1;
                while i < bytes.len() && is_ident_continue(bytes[i]) {
                    i += 1;
                }
                out.push(Tok::Ident(src[start..i].to_string()));
            }
            _ => {
                i += 1;
            }
        }
    }
    out.push(Tok::Eof);
    Ok(out)
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn is_ident_continue(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn peek(tokens: &[Tok], i: usize) -> &Tok {
    tokens.get(i).unwrap_or(&Tok::Eof)
}

fn ident_eq(tok: &Tok, s: &str) -> bool {
    matches!(tok, Tok::Ident(n) if n == s)
}

fn is_struct_def(tokens: &[Tok], i: usize) -> bool {
    if !ident_eq(peek(tokens, i), "struct") {
        return false;
    }
    match peek(tokens, i + 1) {
        Tok::LBrace => true,
        Tok::Ident(_) => matches!(peek(tokens, i + 2), Tok::LBrace),
        _ => false,
    }
}

fn parse_struct_item(tokens: &[Tok], i: &mut usize) -> Result<Item, ParseError> {
    let (tag, fields) = parse_struct_def(tokens, i)?;
    let name = tag.ok_or_else(|| err("anonymous struct requires a typedef name"))?;
    expect_semi(tokens, i)?;
    Ok(Item::Struct(StructDecl { name, fields }))
}

fn parse_struct_def(
    tokens: &[Tok],
    i: &mut usize,
) -> Result<(Option<String>, Vec<Field>), ParseError> {
    if !ident_eq(peek(tokens, *i), "struct") {
        return Err(err("expected struct"));
    }
    *i += 1;
    let tag = match peek(tokens, *i) {
        Tok::Ident(n) => {
            let n = n.clone();
            *i += 1;
            Some(n)
        }
        _ => None,
    };
    if !matches!(peek(tokens, *i), Tok::LBrace) {
        return Err(err("expected '{' in struct definition"));
    }
    *i += 1;
    let fields = parse_struct_fields(tokens, i)?;
    if !matches!(peek(tokens, *i), Tok::RBrace) {
        return Err(err("expected '}' after struct fields"));
    }
    *i += 1;
    Ok((tag, fields))
}

fn parse_struct_fields(tokens: &[Tok], i: &mut usize) -> Result<Vec<Field>, ParseError> {
    let mut fields = Vec::new();
    while !matches!(peek(tokens, *i), Tok::RBrace | Tok::Eof) {
        reject_unsupported(peek(tokens, *i))?;
        let mut ty = parse_base_type(tokens, i)?;
        ty = parse_stars(tokens, i, ty);
        let name = match peek(tokens, *i) {
            Tok::Ident(n) if !is_type_keyword(n) => {
                let n = n.clone();
                *i += 1;
                n
            }
            other => return Err(err(format!("expected field name, found {other:?}"))),
        };
        if matches!(peek(tokens, *i), Tok::Colon) {
            return Err(err("bitfield not supported in this header subset"));
        }
        expect_semi(tokens, i)?;
        fields.push(Field { name, ty });
    }
    Ok(fields)
}

fn parse_typedef(tokens: &[Tok], i: &mut usize) -> Result<Item, ParseError> {
    if !ident_eq(peek(tokens, *i), "typedef") {
        return Err(err("expected typedef"));
    }
    *i += 1;
    if ident_eq(peek(tokens, *i), "struct") && is_struct_def(tokens, *i) {
        let (tag, fields) = parse_struct_def(tokens, i)?;
        let name = match peek(tokens, *i) {
            Tok::Ident(n) if !is_type_keyword(n) => {
                let n = n.clone();
                *i += 1;
                n
            }
            other => {
                return Err(err(format!(
                    "expected typedef name after struct, found {other:?}"
                )));
            }
        };
        expect_semi(tokens, i)?;
        let struct_name = tag.unwrap_or_else(|| name.clone());
        if struct_name == name {
            return Ok(Item::Struct(StructDecl { name, fields }));
        }
        return Ok(Item::Struct(StructDecl {
            name: struct_name,
            fields,
        }));
    }
    if ident_eq(peek(tokens, *i), "struct") {
        *i += 1;
        let tag = match peek(tokens, *i) {
            Tok::Ident(n) => {
                let n = n.clone();
                *i += 1;
                n
            }
            other => {
                return Err(err(format!(
                    "expected struct tag in typedef, found {other:?}"
                )));
            }
        };
        let name = match peek(tokens, *i) {
            Tok::Ident(n) if !is_type_keyword(n) => {
                let n = n.clone();
                *i += 1;
                n
            }
            other => {
                return Err(err(format!("expected typedef name, found {other:?}")));
            }
        };
        expect_semi(tokens, i)?;
        return Ok(Item::Typedef(TypedefDecl {
            name,
            ty: CType::Named(tag),
        }));
    }
    reject_unsupported(peek(tokens, *i))?;
    let mut ty = parse_base_type(tokens, i)?;
    ty = parse_stars(tokens, i, ty);
    let name = match peek(tokens, *i) {
        Tok::Ident(n) if !is_type_keyword(n) => {
            let n = n.clone();
            *i += 1;
            n
        }
        other => return Err(err(format!("expected typedef name, found {other:?}"))),
    };
    expect_semi(tokens, i)?;
    Ok(Item::Typedef(TypedefDecl { name, ty }))
}

fn expect_semi(tokens: &[Tok], i: &mut usize) -> Result<(), ParseError> {
    if !matches!(peek(tokens, *i), Tok::Semi) {
        return Err(err(format!("expected ';' , found {:?}", peek(tokens, *i))));
    }
    *i += 1;
    Ok(())
}

fn parse_fn_decl(tokens: &[Tok], i: &mut usize) -> Result<FnDecl, ParseError> {
    while ident_eq(peek(tokens, *i), "extern")
        || ident_eq(peek(tokens, *i), "static")
        || ident_eq(peek(tokens, *i), "inline")
    {
        *i += 1;
    }
    reject_unsupported(peek(tokens, *i))?;
    let mut ty = parse_base_type(tokens, i)?;
    ty = parse_stars(tokens, i, ty);
    let name = match peek(tokens, *i) {
        Tok::Ident(n) => {
            let n = n.clone();
            *i += 1;
            n
        }
        other => return Err(err(format!("expected function name, found {other:?}"))),
    };
    if !matches!(peek(tokens, *i), Tok::LParen) {
        return Err(err(format!("expected '(' after function name '{name}'")));
    }
    *i += 1;
    let params = parse_params(tokens, i)?;
    if !matches!(peek(tokens, *i), Tok::RParen) {
        return Err(err("expected ')' after parameter list"));
    }
    *i += 1;
    match peek(tokens, *i) {
        Tok::Semi => {
            *i += 1;
        }
        Tok::LBrace => {
            return Err(err("function body not supported in header subset"));
        }
        other => {
            return Err(err(format!(
                "expected ';' after function declaration, found {other:?}"
            )));
        }
    }
    Ok(FnDecl {
        name,
        return_ty: ty,
        params,
    })
}

fn reject_unsupported(tok: &Tok) -> Result<(), ParseError> {
    if let Tok::Ident(n) = tok {
        match n.as_str() {
            "enum" => return Err(err("enum not supported in this header subset")),
            "union" => return Err(err("union not supported in this header subset")),
            _ => {}
        }
    }
    Ok(())
}

fn parse_base_type(tokens: &[Tok], i: &mut usize) -> Result<CType, ParseError> {
    while ident_eq(peek(tokens, *i), "const")
        || ident_eq(peek(tokens, *i), "volatile")
        || ident_eq(peek(tokens, *i), "restrict")
    {
        *i += 1;
    }
    if ident_eq(peek(tokens, *i), "struct") {
        *i += 1;
        if matches!(peek(tokens, *i), Tok::LBrace) {
            return Err(err("anonymous struct not supported here"));
        }
        return match peek(tokens, *i) {
            Tok::Ident(n) => {
                let n = n.clone();
                *i += 1;
                Ok(CType::Named(n))
            }
            other => Err(err(format!("expected struct tag, found {other:?}"))),
        };
    }
    reject_unsupported(peek(tokens, *i))?;
    let mut signed = false;
    let mut unsigned = false;
    let mut longs = 0u8;
    let mut shorts = 0u8;
    let mut core: Option<&str> = None;
    loop {
        match peek(tokens, *i) {
            Tok::Ident(n) if n == "const" || n == "volatile" || n == "restrict" => {
                *i += 1;
            }
            Tok::Ident(n) if n == "signed" => {
                signed = true;
                *i += 1;
            }
            Tok::Ident(n) if n == "unsigned" => {
                unsigned = true;
                *i += 1;
            }
            Tok::Ident(n) if n == "long" => {
                longs += 1;
                *i += 1;
            }
            Tok::Ident(n) if n == "short" => {
                shorts += 1;
                *i += 1;
            }
            Tok::Ident(n) if n == "int" => {
                core = Some("int");
                *i += 1;
            }
            Tok::Ident(n) if n == "char" => {
                core = Some("char");
                *i += 1;
            }
            Tok::Ident(n) if n == "float" => {
                core = Some("float");
                *i += 1;
            }
            Tok::Ident(n) if n == "double" => {
                core = Some("double");
                *i += 1;
            }
            Tok::Ident(n) if n == "void" => {
                core = Some("void");
                *i += 1;
            }
            Tok::Ident(n) if n == "struct" || n == "typedef" || n == "enum" || n == "union" => {
                reject_unsupported(peek(tokens, *i))?;
                return Err(err(format!("unexpected {n} in type")));
            }
            _ => break,
        }
    }
    if signed && unsigned {
        return Err(err("type cannot be both signed and unsigned"));
    }
    match core {
        Some("void") => {
            if signed || unsigned || longs > 0 || shorts > 0 {
                return Err(err("invalid void type"));
            }
            Ok(CType::Void)
        }
        Some("float") => {
            if signed || unsigned || longs > 0 || shorts > 0 {
                return Err(err("invalid float type"));
            }
            Ok(CType::Float)
        }
        Some("double") => {
            if signed || unsigned || shorts > 0 || longs > 1 {
                return Err(err("invalid double type"));
            }
            Ok(CType::Double)
        }
        Some("char") => {
            if longs > 0 || shorts > 0 {
                return Err(err("invalid char type"));
            }
            if unsigned {
                Ok(CType::UChar)
            } else {
                Ok(CType::Char)
            }
        }
        Some("int") | None => {
            if longs > 0 && shorts > 0 {
                return Err(err("invalid integer type"));
            }
            if core.is_none() && longs == 0 && shorts == 0 && !signed && !unsigned {
                return match peek(tokens, *i) {
                    Tok::Ident(n) if !is_type_keyword(n) => {
                        let n = n.clone();
                        *i += 1;
                        Ok(CType::Named(n))
                    }
                    _ => Err(err("expected type specifier")),
                };
            }
            if shorts > 0 {
                if unsigned {
                    Ok(CType::UShort)
                } else {
                    Ok(CType::Short)
                }
            } else if longs >= 2 {
                if unsigned {
                    Ok(CType::ULongLong)
                } else {
                    Ok(CType::LongLong)
                }
            } else if longs == 1 {
                if unsigned {
                    Ok(CType::ULong)
                } else {
                    Ok(CType::Long)
                }
            } else if unsigned {
                Ok(CType::UInt)
            } else {
                Ok(CType::Int)
            }
        }
        _ => Err(err("unsupported type")),
    }
}

fn parse_stars(tokens: &[Tok], i: &mut usize, mut ty: CType) -> CType {
    while ident_eq(peek(tokens, *i), "const")
        || ident_eq(peek(tokens, *i), "volatile")
        || ident_eq(peek(tokens, *i), "restrict")
    {
        *i += 1;
    }
    while matches!(peek(tokens, *i), Tok::Star) {
        *i += 1;
        ty = CType::Pointer(Box::new(ty));
        while ident_eq(peek(tokens, *i), "const")
            || ident_eq(peek(tokens, *i), "volatile")
            || ident_eq(peek(tokens, *i), "restrict")
        {
            *i += 1;
        }
    }
    ty
}

fn parse_params(tokens: &[Tok], i: &mut usize) -> Result<Vec<Param>, ParseError> {
    if matches!(peek(tokens, *i), Tok::RParen) {
        return Ok(Vec::new());
    }
    if ident_eq(peek(tokens, *i), "void") && matches!(peek(tokens, *i + 1), Tok::RParen) {
        *i += 1;
        return Ok(Vec::new());
    }
    let mut params = Vec::new();
    loop {
        reject_unsupported(peek(tokens, *i))?;
        let mut ty = parse_base_type(tokens, i)?;
        ty = parse_stars(tokens, i, ty);
        let name = match peek(tokens, *i) {
            Tok::Ident(n) if !is_type_keyword(n) => {
                let n = n.clone();
                *i += 1;
                Some(n)
            }
            _ => None,
        };
        params.push(Param { name, ty });
        match peek(tokens, *i) {
            Tok::Comma => {
                *i += 1;
            }
            Tok::RParen => break,
            other => {
                return Err(err(format!(
                    "expected ',' or ')' in parameter list, found {other:?}"
                )));
            }
        }
    }
    Ok(params)
}

fn is_type_keyword(n: &str) -> bool {
    matches!(
        n,
        "void"
            | "char"
            | "short"
            | "int"
            | "long"
            | "float"
            | "double"
            | "signed"
            | "unsigned"
            | "const"
            | "volatile"
            | "restrict"
            | "struct"
            | "typedef"
            | "enum"
            | "union"
            | "extern"
            | "static"
            | "inline"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ptr(inner: CType) -> CType {
        CType::Pointer(Box::new(inner))
    }

    #[test]
    fn parse_scalar_binary_fn() {
        let h = parse_header("int add(int a, int b);").unwrap();
        assert_eq!(h.functions.len(), 1);
        let f = &h.functions[0];
        assert_eq!(f.name, "add");
        assert_eq!(f.return_ty, CType::Int);
        assert_eq!(
            f.params,
            vec![
                Param {
                    name: Some("a".into()),
                    ty: CType::Int,
                },
                Param {
                    name: Some("b".into()),
                    ty: CType::Int,
                },
            ]
        );
    }

    #[test]
    fn parse_void_return_pointer_param() {
        let h = parse_header("void free(void *p);").unwrap();
        let f = &h.functions[0];
        assert_eq!(f.name, "free");
        assert_eq!(f.return_ty, CType::Void);
        assert_eq!(
            f.params,
            vec![Param {
                name: Some("p".into()),
                ty: ptr(CType::Void),
            }]
        );
    }

    #[test]
    fn parse_const_char_pointer() {
        let h = parse_header("int puts(const char *s);").unwrap();
        let f = &h.functions[0];
        assert_eq!(f.name, "puts");
        assert_eq!(f.return_ty, CType::Int);
        assert_eq!(
            f.params,
            vec![Param {
                name: Some("s".into()),
                ty: ptr(CType::Char),
            }]
        );
    }

    #[test]
    fn parse_float_double() {
        let h = parse_header("double sqrt(double x);").unwrap();
        let f = &h.functions[0];
        assert_eq!(f.return_ty, CType::Double);
        assert_eq!(f.params[0].ty, CType::Double);
        let h = parse_header("float fma(float a, float b, float c);").unwrap();
        assert_eq!(h.functions[0].return_ty, CType::Float);
        assert_eq!(h.functions[0].params.len(), 3);
    }

    #[test]
    fn parse_unsigned_and_long() {
        let h = parse_header("unsigned int len(unsigned long n);").unwrap();
        let f = &h.functions[0];
        assert_eq!(f.return_ty, CType::UInt);
        assert_eq!(f.params[0].ty, CType::ULong);
    }

    #[test]
    fn parse_pointer_return_and_star_on_name() {
        let h = parse_header("void *memcpy(void *dst, const void *src, unsigned long n);").unwrap();
        let f = &h.functions[0];
        assert_eq!(f.name, "memcpy");
        assert_eq!(f.return_ty, ptr(CType::Void));
        assert_eq!(f.params[0].ty, ptr(CType::Void));
        assert_eq!(f.params[1].ty, ptr(CType::Void));
        assert_eq!(f.params[2].ty, CType::ULong);
        let h = parse_header("char *strdup(const char *s);").unwrap();
        assert_eq!(h.functions[0].return_ty, ptr(CType::Char));
    }

    #[test]
    fn parse_void_params_and_unnamed() {
        let h = parse_header("int getpid(void);").unwrap();
        assert!(h.functions[0].params.is_empty());
        let h = parse_header("int abs(int);").unwrap();
        assert_eq!(
            h.functions[0].params,
            vec![Param {
                name: None,
                ty: CType::Int,
            }]
        );
    }

    #[test]
    fn parse_multiple_and_extern_and_comments() {
        let src = r#"
            // comment
            extern int add(int a, int b);
            /* block */
            unsigned long long ull(unsigned long long x);
            #include <stdio.h>
            short sh(signed short s);
        "#;
        let h = parse_header(src).unwrap();
        assert_eq!(h.functions.len(), 3);
        assert_eq!(h.functions[0].name, "add");
        assert_eq!(h.functions[1].return_ty, CType::ULongLong);
        assert_eq!(h.functions[2].return_ty, CType::Short);
        assert_eq!(h.functions[2].params[0].ty, CType::Short);
    }

    #[test]
    fn reject_function_body() {
        let err = parse_header("int add(int a, int b) { return a + b; }").unwrap_err();
        assert!(
            err.message.contains("body") || err.message.contains("{"),
            "{err}"
        );
    }

    #[test]
    fn emit_scalar_binary_fn() {
        let h = parse_header("int add(int a, int b);").unwrap();
        assert_eq!(
            emit_externs(&h),
            "extern \"C\" function add(a: i32, b: i32): i32;\n"
        );
    }

    #[test]
    fn emit_void_and_pointer() {
        let h = parse_header("void free(void *p);\nchar *strdup(const char *s);").unwrap();
        assert_eq!(
            emit_externs(&h),
            "extern \"C\" function free(p: *u8): void;\nextern \"C\" function strdup(s: *u8): *u8;\n"
        );
    }

    #[test]
    fn emit_unnamed_and_void_params() {
        let h = parse_header("int getpid(void);\nint abs(int);").unwrap();
        assert_eq!(
            emit_externs(&h),
            "extern \"C\" function getpid(): i32;\nextern \"C\" function abs(p0: i32): i32;\n"
        );
    }

    #[test]
    fn emit_unsigned_long_float() {
        let h = parse_header(
            "unsigned int len(unsigned long n);\ndouble sqrt(double x);\nfloat fma(float a, float b, float c);",
        )
        .unwrap();
        assert_eq!(
            emit_externs(&h),
            concat!(
                "extern \"C\" function len(n: u64): u32;\n",
                "extern \"C\" function sqrt(x: f64): f64;\n",
                "extern \"C\" function fma(a: f32, b: f32, c: f32): f32;\n",
            )
        );
    }

    #[test]
    fn emit_int_pointer_and_short() {
        let h = parse_header("int load(int *p);\nshort sh(signed short s);").unwrap();
        assert_eq!(
            emit_externs(&h),
            "extern \"C\" function load(p: *i32): i32;\nextern \"C\" function sh(s: i16): i16;\n"
        );
    }

    #[test]
    fn default_extern_module_path_replaces_extension() {
        assert_eq!(
            default_extern_module_path(Path::new("api.h")),
            PathBuf::from("api.drac")
        );
        assert_eq!(
            default_extern_module_path(Path::new("/tmp/foo.H")),
            PathBuf::from("/tmp/foo.drac")
        );
    }

    #[test]
    fn emit_simple_struct() {
        let h = parse_header("struct Point { int x; int y; };").unwrap();
        assert_eq!(emit_externs(&h), "type Point = { x: i32; y: i32 };\n");
    }

    #[test]
    fn emit_typedef_scalar() {
        let h = parse_header("typedef int Int;\ntypedef unsigned int u32_t;").unwrap();
        assert_eq!(emit_externs(&h), "type Int = i32;\ntype u32_t = u32;\n");
    }

    #[test]
    fn emit_typedef_anonymous_struct() {
        let h = parse_header("typedef struct { int x; int y; } Point;").unwrap();
        assert_eq!(emit_externs(&h), "type Point = { x: i32; y: i32 };\n");
    }

    #[test]
    fn emit_typedef_struct_tag() {
        let h = parse_header("typedef struct Point { int x; int y; } Point;").unwrap();
        assert_eq!(emit_externs(&h), "type Point = { x: i32; y: i32 };\n");
    }

    #[test]
    fn emit_fn_using_struct_and_typedef() {
        let h = parse_header(
            r#"
            struct Point { int x; int y; };
            typedef int Int;
            int take(struct Point p);
            Int ident(Int n);
            struct Point *origin(void);
            "#,
        )
        .unwrap();
        assert_eq!(
            emit_externs(&h),
            concat!(
                "type Point = { x: i32; y: i32 };\n",
                "type Int = i32;\n",
                "extern \"C\" function take(p: Point): i32;\n",
                "extern \"C\" function ident(n: Int): Int;\n",
                "extern \"C\" function origin(): *Point;\n",
            )
        );
    }

    #[test]
    fn reject_union_enum_bitfield() {
        let err = parse_header("union U { int x; };").unwrap_err();
        assert!(err.message.contains("union"), "{err}");
        let err = parse_header("enum E { A };").unwrap_err();
        assert!(err.message.contains("enum"), "{err}");
        let err = parse_header("struct S { int x : 3; };").unwrap_err();
        assert!(
            err.message.contains("bitfield") || err.message.contains(":"),
            "{err}"
        );
    }
}
