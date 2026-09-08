use draconic_diagnostics::Span;

/// Type annotation — named (T01), object (T02), union/intersection (T03), generic app (T04),
/// tuple / fixed array (N03.02), pointer `*T` (N03.03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeAnn {
    /// `number`, `string`, user alias name, etc.
    Named { name: String, span: Span },
    /// `Foo<T, U>` — generic type application (T04).
    GenericApp {
        name: String,
        args: Vec<TypeAnn>,
        span: Span,
    },
    /// `{ a: T; b: U }` (`;` or `,` separators).
    Object { props: Vec<TypeProp>, span: Span },
    /// `[T, U, V]` fixed-length tuple / fixed array type (N03.02).
    Tuple { elements: Vec<TypeAnn>, span: Span },
    /// `*T` — pointer to `T` (N03.03 native).
    Pointer { inner: Box<TypeAnn>, span: Span },
    /// `A | B | C` (flattened left-associative).
    Union { types: Vec<TypeAnn>, span: Span },
    /// `A & B & C` (flattened left-associative).
    Intersection { types: Vec<TypeAnn>, span: Span },
}

/// One property in a structural object type (`name: Type`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeProp {
    pub name: String,
    pub ty: TypeAnn,
    pub span: Span,
}

impl TypeAnn {
    pub fn span(&self) -> Span {
        match self {
            TypeAnn::Named { span, .. }
            | TypeAnn::GenericApp { span, .. }
            | TypeAnn::Object { span, .. }
            | TypeAnn::Tuple { span, .. }
            | TypeAnn::Pointer { span, .. }
            | TypeAnn::Union { span, .. }
            | TypeAnn::Intersection { span, .. } => *span,
        }
    }
}
