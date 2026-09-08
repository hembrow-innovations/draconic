use draconic_ast::{AccessorKind, AssignOp, BinaryOp, UnaryOp, UpdateOp};
use draconic_check::{SymbolId as LocalId, Type};

use crate::Stmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Local {
        id: LocalId,
        ty: Type,
    },
    /// Bare identifier for `with` Object Environment chain (not a static Local).
    IdentName {
        name: String,
        ty: Type,
    },
    Number {
        raw: String,
        ty: Type,
    },
    BigInt {
        raw: String,
        ty: Type,
    },
    String {
        value: draconic_ast::JsString,
        ty: Type,
    },
    /// `/pattern/flags` regular expression literal.
    RegExp {
        pattern: String,
        flags: String,
        ty: Type,
    },
    /// Untagged template literal (cooked quasis + interpolations).
    Template {
        quasis: Vec<draconic_ast::JsString>,
        expressions: Vec<Expr>,
        ty: Type,
    },
    /// Tagged template: `` tag`a${x}b` ``.
    TaggedTemplate {
        tag: Box<Expr>,
        quasis: Vec<draconic_ast::JsString>,
        expressions: Vec<Expr>,
        ty: Type,
    },
    Boolean {
        value: bool,
        ty: Type,
    },
    Null {
        ty: Type,
    },
    /// `this` binding.
    This {
        ty: Type,
    },
    /// `new.target` meta-property.
    NewTarget {
        ty: Type,
    },
    /// `import.meta` meta-property.
    ImportMeta {
        ty: Type,
    },
    /// Dynamic `import(specifier)` / `import.defer(…)` / `import.source(…)`.
    ImportCall {
        phase: draconic_ast::ImportPhase,
        source: Box<Expr>,
        options: Option<Box<Expr>>,
        ty: Type,
    },
    Unary {
        op: UnaryOp,
        arg: Box<Expr>,
        ty: Type,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        ty: Type,
    },
    Conditional {
        test: Box<Expr>,
        consequent: Box<Expr>,
        alternate: Box<Expr>,
        ty: Type,
    },
    /// `target = value` or compound `op=` — local or member target.
    Assign {
        target: AssignTarget,
        op: AssignOp,
        value: Box<Expr>,
        ty: Type,
    },
    /// Prefix or postfix `++` / `--` on a local or with-chain name.
    Update {
        op: UpdateOp,
        target: UpdateTarget,
        prefix: bool,
        ty: Type,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
        /// `true` for optional call `callee?.(args)`.
        optional: bool,
        ty: Type,
    },
    /// `new callee(args?)`.
    New {
        callee: Box<Expr>,
        args: Vec<Arg>,
        ty: Type,
    },
    /// `async? function *? name? (params) { body }` or arrow expression value.
    Function {
        /// Named function expression binding (local to the body), if any.
        name: Option<LocalId>,
        params: Vec<Param>,
        body: Vec<Stmt>,
        is_async: bool,
        is_generator: bool,
        /// `true` for `(params) => …` — lexical `this` / `new.target`.
        is_arrow: bool,
        /// `true` for method definitions (`{ m() {} }`) — JS emit as method form (home object / `super`).
        is_method: bool,
        ty: Type,
    },
    /// Bare `super` (only valid as `super.prop` / `super[expr]` / `super(...)` object after check).
    /// Kept when lowering object methods so the JS backend can emit home-object `super`.
    Super {
        ty: Type,
    },
    /// `{ key: value, … }` object literal.
    Object {
        properties: Vec<ObjectProp>,
        ty: Type,
    },
    /// `[elem, …]` array literal (may include spread elements).
    Array {
        elements: Vec<ArrayElement>,
        ty: Type,
    },
    /// `obj.prop` / `obj[expr]` / optional `obj?.prop` / `obj?.[expr]` property read.
    Member {
        object: Box<Expr>,
        /// Non-computed: string key name as `String` expr. Computed: any expr.
        property: Box<Expr>,
        computed: bool,
        /// `true` for optional chaining (`?.` / `?.[]`).
        optional: bool,
        ty: Type,
    },
}

/// One element of an array literal after lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayElement {
    Expr(Expr),
    Spread(Expr),
    Elision,
}

/// One argument of a call or `new` after lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Expr(Expr),
    Spread(Expr),
}

/// Object literal property key after lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectPropKey {
    /// Static string key (`a` or `"a"`).
    Static(draconic_ast::JsString),
    /// Computed key `[expr]`.
    Computed(Expr),
}

/// Object literal property after lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectProp {
    Property {
        key: ObjectPropKey,
        value: Expr,
    },
    /// `get key() { … }` / `set key(v) { … }` — value is the accessor function.
    Accessor {
        kind: AccessorKind,
        key: ObjectPropKey,
        value: Expr,
    },
    Spread(Expr),
}

/// LHS of an assignment after lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Local(LocalId),
    /// Bare name for `with` Object Environment assign.
    Name(String),
    Member {
        object: Box<Expr>,
        property: Box<Expr>,
        computed: bool,
    },
    /// `*ptr = …` store through native pointer (N03.03).
    Deref(Box<Expr>),
    /// `[a, b, ...rest] = …`
    ArrayPattern {
        elements: Vec<ArrayPatternEl>,
    },
    /// `{ a, b: c, ...rest } = …`
    ObjectPattern {
        properties: Vec<ObjectPatternEl>,
    },
}

/// Target of `++` / `--` after lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateTarget {
    Local(LocalId),
    Name(String),
    /// Property update `obj.prop++` / `obj[k]++` (E19.13).
    Member {
        object: Box<Expr>,
        property: Box<Expr>,
        computed: bool,
    },
}

/// One element of an array destructuring pattern in IR.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayPatternEl {
    /// Hole / elision (`,`).
    Elision,
    /// Simple or nested binding, optional default (`pat = expr`).
    Pattern {
        binding: Pattern,
        default: Option<Expr>,
    },
    Rest(Pattern),
}

/// One property of an object destructuring pattern in IR.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectPatternEl {
    /// `key` / `key: pattern` / `[expr]: pattern` / defaults.
    Prop {
        key: ObjectPropKey,
        binding: Pattern,
        shorthand: bool,
        default: Option<Expr>,
    },
    Rest(Pattern),
}

/// Binding pattern after lowering (ident, nested array/object, or assignment member).
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Local(LocalId),
    /// Free / with-chain name (assignment only).
    Name(String),
    /// Assignment-only property target.
    Member {
        object: Box<Expr>,
        property: Box<Expr>,
        computed: bool,
    },
    Array(Vec<ArrayPatternEl>),
    Object(Vec<ObjectPatternEl>),
}

/// Formal parameter in IR, optionally with a default initializer or rest flag.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub pattern: Pattern,
    pub default: Option<Expr>,
    pub rest: bool,
}

impl Expr {
    pub fn ty(&self) -> Type {
        match self {
            Expr::Local { ty, .. }
            | Expr::IdentName { ty, .. }
            | Expr::Number { ty, .. }
            | Expr::BigInt { ty, .. }
            | Expr::String { ty, .. }
            | Expr::RegExp { ty, .. }
            | Expr::Template { ty, .. }
            | Expr::TaggedTemplate { ty, .. }
            | Expr::Boolean { ty, .. }
            | Expr::Null { ty }
            | Expr::This { ty }
            | Expr::NewTarget { ty }
            | Expr::ImportMeta { ty }
            | Expr::ImportCall { ty, .. }
            | Expr::Super { ty }
            | Expr::Unary { ty, .. }
            | Expr::Binary { ty, .. }
            | Expr::Conditional { ty, .. }
            | Expr::Assign { ty, .. }
            | Expr::Update { ty, .. }
            | Expr::Call { ty, .. }
            | Expr::New { ty, .. }
            | Expr::Function { ty, .. }
            | Expr::Object { ty, .. }
            | Expr::Array { ty, .. }
            | Expr::Member { ty, .. } => *ty,
        }
    }
}
