use std::fmt;

use draconic_diagnostics::Span;
use draconic_lexer::JsString;

use crate::{
    AccessorKind, ArrayPatternElement, BindingPattern, ClassElement, ObjectPatternProp, Stmt,
    TypeAnn,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(Ident),
    Number(NumberLit),
    BigInt(BigIntLit),
    String(StringLit),
    /// `/pattern/flags` regular expression literal.
    RegExp {
        pattern: String,
        flags: String,
        span: Span,
    },
    /// Untagged template literal: `` `a${x}b` ``.
    TemplateLiteral {
        /// Cooked quasi strings; length is always `expressions.len() + 1`.
        quasis: Vec<TemplateElement>,
        expressions: Vec<Expr>,
        span: Span,
    },
    /// Tagged template: `` tag`a${x}b` ``.
    TaggedTemplate {
        tag: Box<Expr>,
        /// Cooked quasi strings; length is always `expressions.len() + 1`.
        quasis: Vec<TemplateElement>,
        expressions: Vec<Expr>,
        span: Span,
    },
    Boolean {
        value: bool,
        span: Span,
    },
    Null {
        span: Span,
    },
    /// `this` binding (method/call-site determined).
    This {
        span: Span,
    },
    /// `super` (constructor call or parent property access in class body).
    Super {
        span: Span,
    },
    /// `new.target` meta-property (active construct target; `undefined` if not `new`).
    NewTarget {
        span: Span,
    },
    /// `import.meta` meta-property (Module goal only).
    ImportMeta {
        span: Span,
    },
    /// Dynamic `import(specifier)` / `import.defer(…)` / `import.source(…)` (ImportCall).
    ImportCall {
        /// Evaluation phase (`import()`), deferred (`import.defer()`), or source (`import.source()`).
        phase: ImportPhase,
        source: Box<Expr>,
        /// Optional second argument (import attributes / options). Only for [`ImportPhase::Evaluation`].
        options: Option<Box<Expr>>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        arg: Box<Expr>,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    /// `test ? consequent : alternate`
    Conditional {
        test: Box<Expr>,
        consequent: Box<Expr>,
        alternate: Box<Expr>,
        span: Span,
    },
    /// `target = value` or compound `target op= value`
    Assign {
        target: Box<Expr>,
        op: AssignOp,
        value: Box<Expr>,
        span: Span,
    },
    /// Prefix or postfix `++` / `--`.
    Update {
        op: UpdateOp,
        arg: Box<Expr>,
        prefix: bool,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
        /// `true` for optional call `callee?.(args)`.
        optional: bool,
        span: Span,
    },
    /// `new callee` or `new callee(args)`.
    New {
        callee: Box<Expr>,
        args: Vec<Arg>,
        span: Span,
    },
    /// `async? function *? name? (params): ret? { body }` as an expression value.
    FunctionExpression {
        name: Option<Ident>,
        params: Vec<Param>,
        /// Optional return type annotation (`: T` after the parameter list).
        return_type: Option<TypeAnn>,
        body: Box<Stmt>,
        is_async: bool,
        is_generator: bool,
        /// True for method definitions (`{ m() {} }`, `{ [e]() {} }`), not `m: function(){}`.
        is_method: bool,
        span: Span,
    },
    /// `class Name? extends Super? { … }` as an expression value (E18.33).
    ClassExpression {
        name: Option<Ident>,
        /// Present when `extends SuperClass`.
        super_class: Option<Box<Expr>>,
        body: Vec<ClassElement>,
        span: Span,
    },
    /// `async? (params): ret? => body` or bare `async? param => body` (simple ident params only).
    ArrowFunction {
        params: Vec<Param>,
        /// Optional return type annotation (`: T` after `)` before `=>`).
        return_type: Option<TypeAnn>,
        body: ArrowBody,
        is_async: bool,
        span: Span,
    },
    /// `{ key: value, … }` — data properties only.
    ObjectExpression {
        properties: Vec<ObjectProp>,
        span: Span,
    },
    /// `[elem, …]` array literal (spread elements and holes/elisions allowed).
    ArrayExpression {
        elements: Vec<ArrayElement>,
        /// True when a comma followed the last element before `]` (e.g. `[a,]` / `[...x,]`).
        /// Distinguishes trailing comma after rest (invalid assignment pattern) from bare rest.
        trailing_comma: bool,
        span: Span,
    },
    /// `obj.prop` / `obj.#prop` / `obj[expr]` / optional `obj?.prop` / `obj?.[expr]` (property read).
    MemberExpression {
        object: Box<Expr>,
        /// Non-computed: `Expr::Ident`. Computed: any expression.
        property: Box<Expr>,
        computed: bool,
        /// `true` for optional chaining (`?.` / `?.[]`).
        optional: bool,
        /// `true` for private field access `obj.#name` (E18.35).
        private: bool,
        span: Span,
    },
    /// Private brand check: `#name in object` (E18.40).
    PrivateIn {
        /// Private name without `#` (dump shows `#name`).
        name: Ident,
        object: Box<Expr>,
        span: Span,
    },
    /// Parenthesized expression — preserved for dump fidelity.
    Paren {
        expr: Box<Expr>,
        span: Span,
    },
    /// Dual-worlds / type boundary: `expr as T` (T06). Erased at emit.
    As {
        expr: Box<Expr>,
        ty: TypeAnn,
        span: Span,
    },
    /// Array destructuring pattern used as assignment target: `[a, b, ...rest]`.
    ArrayPattern {
        elements: Vec<ArrayPatternElement>,
        span: Span,
    },
    /// Object destructuring pattern used as assignment target: `{ a, b: c, ...rest }`.
    ObjectPattern {
        properties: Vec<ObjectPatternProp>,
        span: Span,
    },
}

/// One element of an array literal: value, `...spread`, or hole (elision).
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayElement {
    Expr(Expr),
    Spread(Expr),
    /// Hole from elision (`,`) — contributes `undefined` / empty slot.
    Elision,
}

/// One argument of a call or `new`: value or `...spread`.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Expr(Expr),
    Spread(Expr),
}

/// One property in an object literal (`key: value`, shorthand, method, accessor, or spread).
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectProp {
    /// `key: value`, shorthand `{ a }`, or method `{ m() {} }`.
    Property {
        key: ObjectKey,
        value: Expr,
        /// True for property shorthand `{ a }` (value is the same Ident as key).
        shorthand: bool,
        span: Span,
    },
    /// `get key() { … }` / `set key(v) { … }` (incl. computed keys).
    Accessor {
        kind: AccessorKind,
        key: ObjectKey,
        params: Vec<Param>,
        body: Box<Stmt>,
        span: Span,
    },
    /// `...expr` spread element.
    Spread { expr: Expr, span: Span },
}

/// Object literal property key (ident, string, or computed `[expr]`).
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectKey {
    Ident(Ident),
    String(StringLit),
    Computed(Box<Expr>),
}

/// Concise expression body or block body of an arrow function.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrowBody {
    Expr(Box<Expr>),
    Block(Box<Stmt>),
}

/// Formal parameter: binding pattern (ident / object / array), optional type + default, or rest.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// `name`, `{ a, b }`, `[a, b]`, etc. Rest params use a simple ident binding.
    pub binding: BindingPattern,
    /// Optional type annotation after the parameter binding.
    pub type_ann: Option<TypeAnn>,
    pub default: Option<Expr>,
    /// `true` for a rest parameter (`...name`). Must be last; no default.
    pub rest: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberLit {
    /// Canonical source text (e.g. `1.0`, `42`).
    pub raw: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BigIntLit {
    /// Canonical source text including `n` suffix (e.g. `1n`, `0xffn`).
    pub raw: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLit {
    pub value: JsString,
    pub span: Span,
}

/// One cooked quasi span of a template literal (`cooked` text between interpolations).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateElement {
    pub cooked: JsString,
    /// True for the final quasi (after the last `${…}` or the sole quasi of `` `…` ``).
    pub tail: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    BitNot,
    TypeOf,
    Void,
    Delete,
    Await,
    Yield,
    /// `yield* AssignmentExpression` (delegate).
    YieldStar,
    /// `&expr` — address-of (N03.03 native pointer).
    Ref,
    /// `*expr` — dereference (N03.03 native pointer).
    Deref,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOp {
    Inc,
    Dec,
}

/// Simple `=` or compound assignment operator (`+=`, `-=`, …).
/// Phase of a dynamic `import` call (`import()` / `import.defer()` / `import.source()`)
/// or static deferred namespace import (`import defer * as ns from`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportPhase {
    /// `import(specifier)` / normal static import — load and evaluate.
    #[default]
    Evaluation,
    /// `import.defer(specifier)` / `import defer * as ns from` — deferred evaluation namespace.
    Defer,
    /// `import.source(specifier)` — source-phase module source.
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Eq,
    AddEq,
    SubEq,
    MulEq,
    DivEq,
    RemEq,
    PowEq,
    ShlEq,
    ShrEq,
    UShrEq,
    BitAndEq,
    BitOrEq,
    BitXorEq,
    AndAndEq,
    OrOrEq,
    NullishEq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    EqEq,
    NotEq,
    EqEqEq,
    NotEqEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
    And,
    Or,
    Nullish,
    Comma,
    In,
    InstanceOf,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UnaryOp::Plus => "+",
            UnaryOp::Minus => "-",
            UnaryOp::Not => "!",
            UnaryOp::BitNot => "~",
            UnaryOp::TypeOf => "typeof",
            UnaryOp::Void => "void",
            UnaryOp::Delete => "delete",
            UnaryOp::Await => "await",
            UnaryOp::Yield => "yield",
            UnaryOp::YieldStar => "yield*",
            UnaryOp::Ref => "&",
            UnaryOp::Deref => "*",
        };
        write!(f, "{s}")
    }
}

impl fmt::Display for UpdateOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UpdateOp::Inc => "++",
            UpdateOp::Dec => "--",
        };
        write!(f, "{s}")
    }
}

impl fmt::Display for AssignOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AssignOp::Eq => "=",
            AssignOp::AddEq => "+=",
            AssignOp::SubEq => "-=",
            AssignOp::MulEq => "*=",
            AssignOp::DivEq => "/=",
            AssignOp::RemEq => "%=",
            AssignOp::PowEq => "**=",
            AssignOp::ShlEq => "<<=",
            AssignOp::ShrEq => ">>=",
            AssignOp::UShrEq => ">>>=",
            AssignOp::BitAndEq => "&=",
            AssignOp::BitOrEq => "|=",
            AssignOp::BitXorEq => "^=",
            AssignOp::AndAndEq => "&&=",
            AssignOp::OrOrEq => "||=",
            AssignOp::NullishEq => "??=",
        };
        write!(f, "{s}")
    }
}

impl AssignOp {
    /// Binary operator for compound assignment, if any.
    pub fn binary_op(self) -> Option<BinaryOp> {
        match self {
            AssignOp::Eq => None,
            AssignOp::AddEq => Some(BinaryOp::Add),
            AssignOp::SubEq => Some(BinaryOp::Sub),
            AssignOp::MulEq => Some(BinaryOp::Mul),
            AssignOp::DivEq => Some(BinaryOp::Div),
            AssignOp::RemEq => Some(BinaryOp::Rem),
            AssignOp::PowEq => Some(BinaryOp::Pow),
            AssignOp::ShlEq => Some(BinaryOp::Shl),
            AssignOp::ShrEq => Some(BinaryOp::Shr),
            AssignOp::UShrEq => Some(BinaryOp::UShr),
            AssignOp::BitAndEq => Some(BinaryOp::BitAnd),
            AssignOp::BitOrEq => Some(BinaryOp::BitOr),
            AssignOp::BitXorEq => Some(BinaryOp::BitXor),
            AssignOp::AndAndEq => Some(BinaryOp::And),
            AssignOp::OrOrEq => Some(BinaryOp::Or),
            AssignOp::NullishEq => Some(BinaryOp::Nullish),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::Pow => "**",
            BinaryOp::EqEq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::EqEqEq => "===",
            BinaryOp::NotEqEq => "!==",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
            BinaryOp::UShr => ">>>",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::Nullish => "??",
            BinaryOp::Comma => ",",
            BinaryOp::In => "in",
            BinaryOp::InstanceOf => "instanceof",
        };
        write!(f, "{s}")
    }
}
