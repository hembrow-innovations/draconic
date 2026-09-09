use super::{CType, FnDecl, Header, Item, StructDecl, TypedefDecl};

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

#[cfg(test)]
mod tests {
    use super::super::parse_header;
    use super::emit_externs;

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
}
