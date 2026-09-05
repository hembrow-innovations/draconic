use draconic_diagnostics::Span;
pub use draconic_lexer::JsString;
use std::fmt;

mod print;
pub use print::print_program;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// Binding kind for `let` / `const` / `var` / function / `using` declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingKind {
    Let,
    Const,
    /// Function-scoped `var` (hoisted; redeclarable; no TDZ).
    Var,
    /// Function/class declaration or named expression binding (hoisted for decls).
    /// Reassignment is a runtime concern (immutable FE/class names; mutable decls).
    Function,
    /// `using x = expr` (explicit resource management; const-like + dispose).
    Using,
    /// `await using x = expr` (async dispose).
    AwaitUsing,
}

impl BindingKind {
    /// Lexical (block-scoped) binding — not `var` / function.
    pub fn is_lexical(self) -> bool {
        matches!(
            self,
            BindingKind::Let | BindingKind::Const | BindingKind::Using | BindingKind::AwaitUsing
        )
    }

    /// Immutable binding (`const` / `using` / `await using`).
    pub fn is_const_like(self) -> bool {
        matches!(
            self,
            BindingKind::Const | BindingKind::Using | BindingKind::AwaitUsing
        )
    }
}

/// Binding target for `let` / `const`: simple name or destructuring pattern.
/// Assignment patterns may also use [`BindingPattern::Member`] (LHS property ref).
#[derive(Debug, Clone, PartialEq)]
pub enum BindingPattern {
    Ident(Ident),
    /// `[a, b = d, ...rest]` (elision holes allowed).
    Array {
        elements: Vec<ArrayPatternElement>,
        span: Span,
    },
    /// `{ a, b = d, c: e = f, ...rest }`.
    Object {
        properties: Vec<ObjectPatternProp>,
        span: Span,
    },
    /// Assignment-only LHS member: `obj.prop` / `obj[key]` (not valid in declarations).
    Member(Box<Expr>),
}

/// One element of an array binding/assignment pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayPatternElement {
    /// Hole / elision (`,`) — skips one iterator step; no binding.
    Elision,
    /// Nested or simple binding (`a` or `[a, b]`), optional default (`pat = expr`).
    Pattern {
        binding: BindingPattern,
        default: Option<Expr>,
    },
    /// `...target` rest (must be last; ident, nested pattern, or assignment member).
    Rest(BindingPattern),
}

/// One property of an object binding/assignment pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectPatternProp {
    /// `key` shorthand, `key = default`, `key: nested`, or `key: name = default`.
    Prop {
        /// PropertyName: IdentifierName, StringLiteral, NumericLiteral, or `[AssignmentExpression]`.
        key: ObjectKey,
        /// Binding target for the property value.
        binding: BindingPattern,
        /// True when written as shorthand `{ a }` / `{ a = d }` (binding is the same Ident as key).
        shorthand: bool,
        /// Default when the property value is `undefined` (`pat = expr`).
        default: Option<Expr>,
        span: Span,
    },
    /// `...target` rest (must be last; ident, nested pattern, or assignment member).
    Rest(BindingPattern),
}

impl BindingPattern {
    pub fn span(&self) -> Span {
        match self {
            BindingPattern::Ident(id) => id.span,
            BindingPattern::Array { span, .. } => *span,
            BindingPattern::Object { span, .. } => *span,
            BindingPattern::Member(expr) => expr_span_of(expr),
        }
    }

    /// Visit every identifier bound by this pattern (declaration names).
    pub fn for_each_ident(&self, f: &mut dyn FnMut(&Ident)) {
        match self {
            BindingPattern::Ident(id) => f(id),
            BindingPattern::Member(_) => {}
            BindingPattern::Array { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Elision => {}
                        ArrayPatternElement::Pattern { binding, .. } => binding.for_each_ident(f),
                        ArrayPatternElement::Rest(binding) => binding.for_each_ident(f),
                    }
                }
            }
            BindingPattern::Object { properties, .. } => {
                for p in properties {
                    match p {
                        ObjectPatternProp::Prop { binding, .. } => binding.for_each_ident(f),
                        ObjectPatternProp::Rest(binding) => binding.for_each_ident(f),
                    }
                }
            }
        }
    }
}

fn expr_span_of(expr: &Expr) -> Span {
    match expr {
        Expr::Ident(i) => i.span,
        Expr::Number(n) => n.span,
        Expr::BigInt(n) => n.span,
        Expr::String(s) => s.span,
        Expr::RegExp { span, .. } => *span,
        Expr::Boolean { span, .. }
        | Expr::Null { span }
        | Expr::This { span }
        | Expr::Super { span }
        | Expr::NewTarget { span }
        | Expr::ImportMeta { span }
        | Expr::ImportCall { span, .. }
        | Expr::TemplateLiteral { span, .. }
        | Expr::TaggedTemplate { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Conditional { span, .. }
        | Expr::Assign { span, .. }
        | Expr::Update { span, .. }
        | Expr::Call { span, .. }
        | Expr::New { span, .. }
        | Expr::FunctionExpression { span, .. }
        | Expr::ArrowFunction { span, .. }
        | Expr::ClassExpression { span, .. }
        | Expr::ObjectExpression { span, .. }
        | Expr::ArrayExpression { span, .. }
        | Expr::MemberExpression { span, .. }
        | Expr::PrivateIn { span, .. }
        | Expr::Paren { span, .. }
        | Expr::As { span, .. }
        | Expr::ArrayPattern { span, .. }
        | Expr::ObjectPattern { span, .. } => *span,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expression {
        expr: Expr,
        span: Span,
    },
    /// `let name = init;`, `let name;`, `const name = init;`, or array destructuring.
    /// Optional `type_ann` is the TS-inspired annotation on a simple binding (`let x: T`).
    Let {
        kind: BindingKind,
        binding: BindingPattern,
        type_ann: Option<TypeAnn>,
        init: Option<Expr>,
        span: Span,
    },
    Empty {
        span: Span,
    },
    /// `{ statements }`
    Block {
        body: Vec<Stmt>,
        span: Span,
    },
    /// `if (test) consequent` or `if (test) consequent else alternate`
    If {
        test: Expr,
        consequent: Box<Stmt>,
        alternate: Option<Box<Stmt>>,
        span: Span,
    },
    /// `while (test) body`
    While {
        test: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    /// `do body while (test);`
    DoWhile {
        body: Box<Stmt>,
        test: Expr,
        span: Span,
    },
    /// `for (init; test; update) body` — each of init/test/update may be omitted.
    /// `init` is `Let` or `Expression` when present.
    For {
        init: Option<Box<Stmt>>,
        test: Option<Expr>,
        update: Option<Expr>,
        body: Box<Stmt>,
        span: Span,
    },
    /// `for (left in right) body` — `left` is `Let` or assignable `Expression`.
    ForIn {
        left: Box<Stmt>,
        right: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    /// `for await? (left of right) body` — `left` is `Let` or assignable `Expression`.
    ForOf {
        left: Box<Stmt>,
        right: Expr,
        body: Box<Stmt>,
        /// `for await (… of …)` (async iteration; only valid in async functions).
        is_await: bool,
        span: Span,
    },
    /// `break;` or `break label;`
    Break {
        label: Option<Ident>,
        span: Span,
    },
    /// `continue;` or `continue label;`
    Continue {
        label: Option<Ident>,
        span: Span,
    },
    /// `label: body`
    Labeled {
        label: Ident,
        body: Box<Stmt>,
        span: Span,
    },
    /// `switch (discriminant) { case test: … default: … }`
    Switch {
        discriminant: Expr,
        cases: Vec<SwitchCase>,
        span: Span,
    },
    /// `async? function *? name <T…>? (params): ret? { body }`
    FunctionDeclaration {
        name: Ident,
        /// Type parameters (`function f<T, U>(…)`); empty when absent (T04).
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        /// Optional return type annotation (`: T` after the parameter list).
        return_type: Option<TypeAnn>,
        body: Box<Stmt>,
        is_async: bool,
        is_generator: bool,
        span: Span,
    },
    /// `class name extends? super { constructor? methods… }`
    ClassDeclaration {
        name: Ident,
        /// Present when `extends SuperClass`.
        super_class: Option<Box<Expr>>,
        body: Vec<ClassElement>,
        span: Span,
    },
    /// `return;` or `return expr;`
    Return {
        argument: Option<Expr>,
        span: Span,
    },
    /// `throw expr;`
    Throw {
        argument: Expr,
        span: Span,
    },
    /// `try { … } catch (param)? { … }? finally { … }?` (at least one of catch/finally)
    Try {
        block: Box<Stmt>,
        /// Catch parameter when present (`catch (e)` / `catch ([a])` / `catch ({x})`).
        handler_param: Option<BindingPattern>,
        /// Catch body when a `catch` clause is present.
        handler: Option<Box<Stmt>>,
        /// `finally` block when present.
        finalizer: Option<Box<Stmt>>,
        span: Span,
    },
    /// `with (object) body` — non-strict Object Environment (ECMA-262).
    With {
        object: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    /// `import { a, b as c } from "mod"` / `import d from "mod"` / `import d, { a } from "mod"`
    /// / `import * as ns from "mod"` / `import d, * as ns from "mod"` / `import "mod"`
    /// / `import defer * as ns from "mod"` (E19.42).
    /// / `import type { a }` / `import type d from` / `import type * as ns from`.
    /// Default import is a specifier with `imported.name == "default"`.
    /// Namespace import binds `namespace` to a module namespace object.
    /// Optional `with {…}` / `assert {…}` import attributes (E19.38).
    ImportDeclaration {
        specifiers: Vec<ImportSpecifier>,
        /// `import * as name` binding, when present.
        namespace: Option<Ident>,
        source: StringLit,
        attributes: Vec<ImportAttribute>,
        /// Evaluation (default) or `import defer * as ns` deferred namespace (E19.42).
        phase: ImportPhase,
        /// `import type …` (type-only; no runtime local bindings).
        type_only: bool,
        span: Span,
    },
    /// `export let/const/function …` or `export { a, b as c }` or `export { a } from "mod"`
    ExportNamedDeclaration {
        /// Present for `export let` / `export const` / `export function`.
        declaration: Option<Box<Stmt>>,
        /// Present for `export { … }` (and empty when declaration carries the names).
        specifiers: Vec<ExportSpecifier>,
        /// Present for `export { … } from "mod"` (named re-export; no local bindings).
        source: Option<StringLit>,
        attributes: Vec<ImportAttribute>,
        span: Span,
    },
    /// `export default function …` / `export default expr`
    /// Always carries a declaration that binds `local` (function/class or synthetic `let`).
    ExportDefaultDeclaration {
        declaration: Box<Stmt>,
        /// Local binding name of the default export value.
        local: Ident,
        span: Span,
    },
    /// `export * from "mod"` / `export * as ns from "mod"`.
    /// Without `exported`: re-export all named exports (not `default`) from `source`.
    /// With `exported`: re-export the module namespace object as that name (includes `default`).
    /// Optional `with {…}` / `assert {…}` after the module specifier (E19.38).
    ExportAllDeclaration {
        /// `export * as ns` binding name, when present.
        exported: Option<Ident>,
        source: StringLit,
        attributes: Vec<ImportAttribute>,
        span: Span,
    },
    /// `type Name <T…>? = Type;` — TS-inspired type alias (erased at emit; T02/T04).
    TypeAlias {
        name: Ident,
        /// Type parameters (`type Box<T> = …`); empty when absent (T04).
        type_params: Vec<TypeParam>,
        ty: TypeAnn,
        span: Span,
    },
    /// `extern "C" function name(params): ret?;` — FFI function declaration (no body; F06).
    ExternFunctionDeclaration {
        /// ABI string literal (v1: `"C"`).
        abi: StringLit,
        name: Ident,
        params: Vec<Param>,
        /// Optional return type annotation (`: T` after the parameter list).
        return_type: Option<TypeAnn>,
        span: Span,
    },
}

/// One type parameter: `T` in `function f<T>` / `type Box<T>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    pub name: Ident,
}

/// One binding of `import { imported as local }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportSpecifier {
    /// Exported name in the source module.
    pub imported: Ident,
    /// Local binding name in this module.
    pub local: Ident,
    /// Inline `type` specifier (`import { type foo }`).
    pub is_type: bool,
}

/// One entry of `with { key: "value" }` / `assert { key: "value" }` (import attributes).
#[derive(Debug, Clone, PartialEq)]
pub struct ImportAttribute {
    pub key: ImportAttributeKey,
    pub value: StringLit,
    pub span: Span,
}

/// Attribute key: IdentifierName or StringLiteral (StringValue identity for dup checks).
#[derive(Debug, Clone, PartialEq)]
pub enum ImportAttributeKey {
    Ident(Ident),
    String(StringLit),
}

/// One binding of `export { local as exported }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSpecifier {
    /// Local name in this module.
    pub local: Ident,
    /// Name under which it is exported.
    pub exported: Ident,
}

/// One element of a class body (`constructor`, method, accessor, or field).
#[derive(Debug, Clone, PartialEq)]
pub enum ClassElement {
    /// `constructor(params) { body }`
    Constructor {
        params: Vec<Param>,
        body: Box<Stmt>,
        span: Span,
    },
    /// `static? async? *? #? name(params) { body }` instance or static method (optional async/generator/private/computed)
    Method {
        /// Ident / string / computed `[expr]`; private methods use `Ident` + `is_private`.
        key: ObjectKey,
        params: Vec<Param>,
        body: Box<Stmt>,
        is_static: bool,
        is_async: bool,
        is_generator: bool,
        /// `true` for `#name(...)` / `static #name(...)` private methods (E18.37 / E18.38).
        is_private: bool,
        span: Span,
    },
    /// `static? get #? name() { body }` / `static? set #? name(v) { body }` (E18.22 public; E18.39 private; computed keys)
    Accessor {
        kind: AccessorKind,
        /// Ident / string / computed `[expr]`; private accessors use `Ident` + `is_private`.
        key: ObjectKey,
        params: Vec<Param>,
        body: Box<Stmt>,
        is_static: bool,
        /// `true` for `get #name` / `set #name` private accessors.
        is_private: bool,
        span: Span,
    },
    /// `static? #? name = expr;` / `static? #? name;` / computed `[expr]` field (E18.26 public; E18.35 private).
    Field {
        /// Ident / string / computed `[expr]`; private fields use `Ident` + `is_private`.
        key: ObjectKey,
        /// Absent when the field has no initializer (`name;`).
        value: Option<Expr>,
        is_static: bool,
        /// `true` for `#name` private fields.
        is_private: bool,
        span: Span,
    },
    /// `static { … }` static initialization block (E18.41).
    StaticBlock { body: Box<Stmt>, span: Span },
}

/// Object/class accessor kind (`get` / `set`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessorKind {
    Get,
    Set,
}

/// One `case test:` or `default:` clause and its statement list.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    /// `None` means `default`.
    pub test: Option<Expr>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

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

/// Stable, indentation-based AST dump for snapshots and `draconic parse`.
pub fn dump_program(program: &Program) -> String {
    let mut out = String::new();
    out.push_str("Program\n");
    for stmt in &program.body {
        dump_stmt(stmt, 1, &mut out);
    }
    out
}

fn indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

/// Compact single-line-ish dump for catch param headers (`catch (e)` / `catch ([a, b])`).
fn dump_binding_pattern_inline(pat: &BindingPattern, out: &mut String) {
    match pat {
        BindingPattern::Ident(name) => out.push_str(&name.name),
        BindingPattern::Member(_) => out.push_str("<member>"),
        BindingPattern::Array { elements, .. } => {
            out.push('[');
            for (i, el) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        dump_binding_pattern_inline(binding, out);
                        if default.is_some() {
                            out.push_str(" = …");
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        out.push_str("...");
                        dump_binding_pattern_inline(binding, out);
                    }
                }
            }
            out.push(']');
        }
        BindingPattern::Object { properties, .. } => {
            out.push('{');
            for (i, p) in properties.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        shorthand,
                        default,
                        ..
                    } => {
                        if *shorthand {
                            dump_object_key_inline(key, out);
                        } else {
                            dump_object_key_inline(key, out);
                            out.push_str(": ");
                            dump_binding_pattern_inline(binding, out);
                        }
                        if default.is_some() {
                            out.push_str(" = …");
                        }
                    }
                    ObjectPatternProp::Rest(binding) => {
                        out.push_str("...");
                        dump_binding_pattern_inline(binding, out);
                    }
                }
            }
            out.push('}');
        }
    }
}

fn dump_binding_pattern(pat: &BindingPattern, level: usize, out: &mut String) {
    match pat {
        BindingPattern::Ident(name) => {
            indent(level, out);
            out.push_str(&format!("name: {}\n", name.name));
        }
        BindingPattern::Member(expr) => {
            indent(level, out);
            out.push_str("MemberTarget\n");
            dump_expr(expr, level + 1, out);
        }
        BindingPattern::Array { elements, .. } => {
            indent(level, out);
            out.push_str("ArrayPattern\n");
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {
                        indent(level + 1, out);
                        out.push_str("elision\n");
                    }
                    ArrayPatternElement::Pattern { binding, default } => {
                        dump_binding_pattern(binding, level + 1, out);
                        if let Some(def) = default {
                            indent(level + 1, out);
                            out.push_str("default:\n");
                            dump_expr(def, level + 2, out);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        indent(level + 1, out);
                        out.push_str("rest:\n");
                        dump_binding_pattern(binding, level + 2, out);
                    }
                }
            }
        }
        BindingPattern::Object { properties, .. } => {
            indent(level, out);
            out.push_str("ObjectPattern\n");
            dump_object_pattern_props(properties, level + 1, out);
        }
    }
}

fn dump_object_pattern_props(properties: &[ObjectPatternProp], level: usize, out: &mut String) {
    for p in properties {
        match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                shorthand,
                default,
                ..
            } => {
                indent(level, out);
                if *shorthand {
                    out.push_str("prop shorthand:\n");
                } else {
                    out.push_str("prop:\n");
                }
                indent(level + 1, out);
                match key {
                    ObjectKey::Ident(id) => out.push_str(&format!("key: {}\n", id.name)),
                    ObjectKey::String(s) => {
                        out.push_str(&format!("key: {}\n", s.value.to_string_lossy()))
                    }
                    ObjectKey::Computed(expr) => {
                        out.push_str("key: Computed\n");
                        dump_expr(expr, level + 2, out);
                    }
                }
                indent(level + 1, out);
                out.push_str("binding:\n");
                dump_binding_pattern(binding, level + 2, out);
                if let Some(def) = default {
                    indent(level + 1, out);
                    out.push_str("default:\n");
                    dump_expr(def, level + 2, out);
                }
            }
            ObjectPatternProp::Rest(binding) => {
                indent(level, out);
                out.push_str("rest:\n");
                dump_binding_pattern(binding, level + 1, out);
            }
        }
    }
}

fn dump_object_key_inline(key: &ObjectKey, out: &mut String) {
    match key {
        ObjectKey::Ident(id) => out.push_str(&id.name),
        ObjectKey::String(s) => out.push_str(&s.value.to_string_lossy()),
        ObjectKey::Computed(_) => out.push_str("[…]"),
    }
}

fn dump_import_attributes(attributes: &[ImportAttribute], level: usize, out: &mut String) {
    for attr in attributes {
        indent(level, out);
        out.push_str("ImportAttribute\n");
        indent(level + 1, out);
        match &attr.key {
            ImportAttributeKey::Ident(id) => {
                out.push_str("key: ");
                out.push_str(&id.name);
                out.push('\n');
            }
            ImportAttributeKey::String(s) => {
                out.push_str(&format!("key: String {:?}\n", s.value.to_string_lossy()));
            }
        }
        indent(level + 1, out);
        out.push_str(&format!(
            "value: {:?}\n",
            attr.value.value.to_string_lossy()
        ));
    }
}

fn dump_stmt(stmt: &Stmt, level: usize, out: &mut String) {
    match stmt {
        Stmt::Expression { expr, .. } => {
            indent(level, out);
            out.push_str("ExpressionStatement\n");
            dump_expr(expr, level + 1, out);
        }
        Stmt::Let {
            kind,
            binding,
            type_ann,
            init,
            ..
        } => {
            indent(level, out);
            match kind {
                BindingKind::Let => out.push_str("Let\n"),
                BindingKind::Const => out.push_str("Const\n"),
                BindingKind::Var => out.push_str("Var\n"),
                BindingKind::Function => out.push_str("FunctionBinding\n"),
                BindingKind::Using => out.push_str("Using\n"),
                BindingKind::AwaitUsing => out.push_str("AwaitUsing\n"),
            }
            dump_binding_pattern(binding, level + 1, out);
            if let Some(ann) = type_ann {
                indent(level + 1, out);
                out.push_str("type:\n");
                dump_type_ann(ann, level + 2, out);
            }
            if let Some(init) = init {
                indent(level + 1, out);
                out.push_str("init:\n");
                dump_expr(init, level + 2, out);
            }
        }
        Stmt::Empty { .. } => {
            indent(level, out);
            out.push_str("EmptyStatement\n");
        }
        Stmt::Block { body, .. } => {
            indent(level, out);
            out.push_str("Block\n");
            for s in body {
                dump_stmt(s, level + 1, out);
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            indent(level, out);
            out.push_str("If\n");
            indent(level + 1, out);
            out.push_str("test:\n");
            dump_expr(test, level + 2, out);
            indent(level + 1, out);
            out.push_str("consequent:\n");
            dump_stmt(consequent, level + 2, out);
            if let Some(alt) = alternate {
                indent(level + 1, out);
                out.push_str("alternate:\n");
                dump_stmt(alt, level + 2, out);
            }
        }
        Stmt::While { test, body, .. } => {
            indent(level, out);
            out.push_str("While\n");
            indent(level + 1, out);
            out.push_str("test:\n");
            dump_expr(test, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::DoWhile { body, test, .. } => {
            indent(level, out);
            out.push_str("DoWhile\n");
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
            indent(level + 1, out);
            out.push_str("test:\n");
            dump_expr(test, level + 2, out);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            indent(level, out);
            out.push_str("For\n");
            if let Some(init) = init {
                indent(level + 1, out);
                out.push_str("init:\n");
                dump_stmt(init, level + 2, out);
            }
            if let Some(test) = test {
                indent(level + 1, out);
                out.push_str("test:\n");
                dump_expr(test, level + 2, out);
            }
            if let Some(update) = update {
                indent(level + 1, out);
                out.push_str("update:\n");
                dump_expr(update, level + 2, out);
            }
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::ForIn {
            left, right, body, ..
        } => {
            indent(level, out);
            out.push_str("ForIn\n");
            indent(level + 1, out);
            out.push_str("left:\n");
            dump_stmt(left, level + 2, out);
            indent(level + 1, out);
            out.push_str("right:\n");
            dump_expr(right, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
            ..
        } => {
            indent(level, out);
            if *is_await {
                out.push_str("ForOf await\n");
            } else {
                out.push_str("ForOf\n");
            }
            indent(level + 1, out);
            out.push_str("left:\n");
            dump_stmt(left, level + 2, out);
            indent(level + 1, out);
            out.push_str("right:\n");
            dump_expr(right, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::Break { label, .. } => {
            indent(level, out);
            if let Some(label) = label {
                out.push_str(&format!("Break {}\n", label.name));
            } else {
                out.push_str("Break\n");
            }
        }
        Stmt::Continue { label, .. } => {
            indent(level, out);
            if let Some(label) = label {
                out.push_str(&format!("Continue {}\n", label.name));
            } else {
                out.push_str("Continue\n");
            }
        }
        Stmt::Labeled { label, body, .. } => {
            indent(level, out);
            out.push_str(&format!("Labeled {}\n", label.name));
            dump_stmt(body, level + 1, out);
        }
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            indent(level, out);
            out.push_str("Switch\n");
            indent(level + 1, out);
            out.push_str("discriminant:\n");
            dump_expr(discriminant, level + 2, out);
            for case in cases {
                indent(level + 1, out);
                if let Some(test) = &case.test {
                    out.push_str("Case\n");
                    indent(level + 2, out);
                    out.push_str("test:\n");
                    dump_expr(test, level + 3, out);
                } else {
                    out.push_str("Default\n");
                }
                for s in &case.body {
                    dump_stmt(s, level + 2, out);
                }
            }
        }
        Stmt::FunctionDeclaration {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            is_generator,
            ..
        } => {
            indent(level, out);
            out.push_str("FunctionDeclaration\n");
            if *is_async {
                indent(level + 1, out);
                out.push_str("async: true\n");
            }
            if *is_generator {
                indent(level + 1, out);
                out.push_str("generator: true\n");
            }
            indent(level + 1, out);
            out.push_str(&format!("name: {}\n", name.name));
            if !type_params.is_empty() {
                indent(level + 1, out);
                out.push_str("typeParams:\n");
                for tp in type_params {
                    indent(level + 2, out);
                    out.push_str(&format!("{}\n", tp.name.name));
                }
            }
            dump_params(params, level + 1, out);
            if let Some(ret) = return_type {
                indent(level + 1, out);
                out.push_str("returnType:\n");
                dump_type_ann(ret, level + 2, out);
            }
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::TypeAlias {
            name,
            type_params,
            ty,
            ..
        } => {
            indent(level, out);
            out.push_str("TypeAlias\n");
            indent(level + 1, out);
            out.push_str(&format!("name: {}\n", name.name));
            if !type_params.is_empty() {
                indent(level + 1, out);
                out.push_str("typeParams:\n");
                for tp in type_params {
                    indent(level + 2, out);
                    out.push_str(&format!("{}\n", tp.name.name));
                }
            }
            indent(level + 1, out);
            out.push_str("type:\n");
            dump_type_ann(ty, level + 2, out);
        }
        Stmt::ExternFunctionDeclaration {
            abi,
            name,
            params,
            return_type,
            ..
        } => {
            indent(level, out);
            out.push_str("ExternFunctionDeclaration\n");
            indent(level + 1, out);
            out.push_str(&format!("abi: {:?}\n", abi.value.to_string_lossy()));
            indent(level + 1, out);
            out.push_str(&format!("name: {}\n", name.name));
            dump_params(params, level + 1, out);
            if let Some(ret) = return_type {
                indent(level + 1, out);
                out.push_str("returnType:\n");
                dump_type_ann(ret, level + 2, out);
            }
        }
        Stmt::ClassDeclaration {
            name,
            super_class,
            body,
            ..
        } => {
            indent(level, out);
            out.push_str("ClassDeclaration\n");
            indent(level + 1, out);
            out.push_str(&format!("name: {}\n", name.name));
            if let Some(sc) = super_class {
                indent(level + 1, out);
                out.push_str("extends:\n");
                dump_expr(sc, level + 2, out);
            }
            for el in body {
                match el {
                    ClassElement::Constructor { params, body, .. } => {
                        indent(level + 1, out);
                        out.push_str("Constructor\n");
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::Method {
                        key,
                        params,
                        body,
                        is_static,
                        is_async,
                        is_generator,
                        is_private,
                        ..
                    } => {
                        indent(level + 1, out);
                        match (*is_static, *is_private) {
                            (true, true) => out.push_str("StaticPrivateMethod\n"),
                            (true, false) => out.push_str("StaticMethod\n"),
                            (false, true) => out.push_str("PrivateMethod\n"),
                            (false, false) => out.push_str("Method\n"),
                        }
                        dump_class_element_key(key, *is_private, level + 2, out);
                        if *is_async {
                            indent(level + 2, out);
                            out.push_str("async: true\n");
                        }
                        if *is_generator {
                            indent(level + 2, out);
                            out.push_str("generator: true\n");
                        }
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::Accessor {
                        kind,
                        key,
                        params,
                        body,
                        is_static,
                        is_private,
                        ..
                    } => {
                        indent(level + 1, out);
                        let kind_s = match kind {
                            AccessorKind::Get => "get",
                            AccessorKind::Set => "set",
                        };
                        match (*is_static, *is_private) {
                            (true, true) => {
                                out.push_str(&format!("StaticPrivateAccessor {kind_s}\n"))
                            }
                            (true, false) => out.push_str(&format!("StaticAccessor {kind_s}\n")),
                            (false, true) => out.push_str(&format!("PrivateAccessor {kind_s}\n")),
                            (false, false) => out.push_str(&format!("Accessor {kind_s}\n")),
                        }
                        dump_class_element_key(key, *is_private, level + 2, out);
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        indent(level + 1, out);
                        out.push_str("StaticBlock\n");
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::Field {
                        key,
                        value,
                        is_static,
                        is_private,
                        ..
                    } => {
                        indent(level + 1, out);
                        match (*is_static, *is_private) {
                            (true, true) => out.push_str("StaticPrivateField\n"),
                            (true, false) => out.push_str("StaticField\n"),
                            (false, true) => out.push_str("PrivateField\n"),
                            (false, false) => out.push_str("Field\n"),
                        }
                        dump_class_element_key(key, *is_private, level + 2, out);
                        if let Some(v) = value {
                            indent(level + 2, out);
                            out.push_str("value:\n");
                            dump_expr(v, level + 3, out);
                        }
                    }
                }
            }
        }
        Stmt::Return { argument, .. } => {
            indent(level, out);
            out.push_str("Return\n");
            if let Some(arg) = argument {
                dump_expr(arg, level + 1, out);
            }
        }
        Stmt::Throw { argument, .. } => {
            indent(level, out);
            out.push_str("Throw\n");
            dump_expr(argument, level + 1, out);
        }
        Stmt::ImportDeclaration {
            specifiers,
            namespace,
            source,
            attributes,
            phase,
            ..
        } => {
            indent(level, out);
            out.push_str("ImportDeclaration\n");
            if *phase == ImportPhase::Defer {
                indent(level + 1, out);
                out.push_str("phase: defer\n");
            }
            for spec in specifiers {
                indent(level + 1, out);
                out.push_str("ImportSpecifier\n");
                indent(level + 2, out);
                out.push_str("imported: ");
                out.push_str(&spec.imported.name);
                out.push('\n');
                indent(level + 2, out);
                out.push_str("local: ");
                out.push_str(&spec.local.name);
                out.push('\n');
            }
            if let Some(ns) = namespace {
                indent(level + 1, out);
                out.push_str("namespace: ");
                out.push_str(&ns.name);
                out.push('\n');
            }
            indent(level + 1, out);
            out.push_str("source: ");
            out.push_str(&source.value.to_string_lossy());
            out.push('\n');
            dump_import_attributes(attributes, level + 1, out);
        }
        Stmt::ExportNamedDeclaration {
            declaration,
            specifiers,
            source,
            attributes,
            ..
        } => {
            indent(level, out);
            out.push_str("ExportNamedDeclaration\n");
            if let Some(decl) = declaration {
                indent(level + 1, out);
                out.push_str("declaration:\n");
                dump_stmt(decl, level + 2, out);
            }
            for spec in specifiers {
                indent(level + 1, out);
                out.push_str("ExportSpecifier\n");
                indent(level + 2, out);
                out.push_str("local: ");
                out.push_str(&spec.local.name);
                out.push('\n');
                indent(level + 2, out);
                out.push_str("exported: ");
                out.push_str(&spec.exported.name);
                out.push('\n');
            }
            if let Some(source) = source {
                indent(level + 1, out);
                out.push_str("source: ");
                out.push_str(&source.value.to_string_lossy());
                out.push('\n');
            }
            dump_import_attributes(attributes, level + 1, out);
        }
        Stmt::ExportDefaultDeclaration {
            declaration, local, ..
        } => {
            indent(level, out);
            out.push_str("ExportDefaultDeclaration\n");
            indent(level + 1, out);
            out.push_str("local: ");
            out.push_str(&local.name);
            out.push('\n');
            indent(level + 1, out);
            out.push_str("declaration:\n");
            dump_stmt(declaration, level + 2, out);
        }
        Stmt::ExportAllDeclaration {
            exported,
            source,
            attributes,
            ..
        } => {
            indent(level, out);
            out.push_str("ExportAllDeclaration\n");
            if let Some(exported) = exported {
                indent(level + 1, out);
                out.push_str("exported: ");
                out.push_str(&exported.name);
                out.push('\n');
            }
            indent(level + 1, out);
            out.push_str("source: ");
            out.push_str(&source.value.to_string_lossy());
            out.push('\n');
            dump_import_attributes(attributes, level + 1, out);
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            ..
        } => {
            indent(level, out);
            out.push_str("Try\n");
            indent(level + 1, out);
            out.push_str("block:\n");
            dump_stmt(block, level + 2, out);
            if let Some(handler) = handler {
                indent(level + 1, out);
                out.push_str("catch");
                if let Some(param) = handler_param {
                    out.push_str(" (");
                    dump_binding_pattern_inline(param, out);
                    out.push(')');
                }
                out.push_str(":\n");
                dump_stmt(handler, level + 2, out);
            }
            if let Some(finalizer) = finalizer {
                indent(level + 1, out);
                out.push_str("finally:\n");
                dump_stmt(finalizer, level + 2, out);
            }
        }
        Stmt::With { object, body, .. } => {
            indent(level, out);
            out.push_str("With\n");
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
    }
}

fn dump_expr(expr: &Expr, level: usize, out: &mut String) {
    match expr {
        Expr::Ident(id) => {
            indent(level, out);
            out.push_str(&format!("Ident {}\n", id.name));
        }
        Expr::Number(n) => {
            indent(level, out);
            out.push_str(&format!("Number {}\n", n.raw));
        }
        Expr::BigInt(n) => {
            indent(level, out);
            out.push_str(&format!("BigInt {}\n", n.raw));
        }
        Expr::String(s) => {
            indent(level, out);
            out.push_str(&format!("String {:?}\n", s.value.to_string_lossy()));
        }
        Expr::RegExp { pattern, flags, .. } => {
            indent(level, out);
            out.push_str(&format!("RegExp /{pattern}/{flags}\n"));
        }
        Expr::TemplateLiteral {
            quasis,
            expressions,
            ..
        } => {
            indent(level, out);
            out.push_str("TemplateLiteral\n");
            for (i, q) in quasis.iter().enumerate() {
                indent(level + 1, out);
                out.push_str(&format!("Quasi {:?}\n", q.cooked.to_string_lossy()));
                if i < expressions.len() {
                    dump_expr(&expressions[i], level + 1, out);
                }
            }
        }
        Expr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            ..
        } => {
            indent(level, out);
            out.push_str("TaggedTemplate\n");
            indent(level + 1, out);
            out.push_str("tag:\n");
            dump_expr(tag, level + 2, out);
            for (i, q) in quasis.iter().enumerate() {
                indent(level + 1, out);
                out.push_str(&format!("Quasi {:?}\n", q.cooked.to_string_lossy()));
                if i < expressions.len() {
                    dump_expr(&expressions[i], level + 1, out);
                }
            }
        }
        Expr::Boolean { value, .. } => {
            indent(level, out);
            out.push_str(&format!("Boolean {value}\n"));
        }
        Expr::Null { .. } => {
            indent(level, out);
            out.push_str("Null\n");
        }
        Expr::This { .. } => {
            indent(level, out);
            out.push_str("This\n");
        }
        Expr::Super { .. } => {
            indent(level, out);
            out.push_str("Super\n");
        }
        Expr::NewTarget { .. } => {
            indent(level, out);
            out.push_str("NewTarget\n");
        }
        Expr::ImportMeta { .. } => {
            indent(level, out);
            out.push_str("ImportMeta\n");
        }
        Expr::ImportCall {
            phase,
            source,
            options,
            ..
        } => {
            indent(level, out);
            match phase {
                ImportPhase::Evaluation => out.push_str("ImportCall\n"),
                ImportPhase::Defer => out.push_str("ImportCall defer\n"),
                ImportPhase::Source => out.push_str("ImportCall source\n"),
            }
            dump_expr(source, level + 1, out);
            if let Some(opts) = options {
                dump_expr(opts, level + 1, out);
            }
        }
        Expr::Unary { op, arg, .. } => {
            indent(level, out);
            out.push_str(&format!("Unary {op}\n"));
            dump_expr(arg, level + 1, out);
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            indent(level, out);
            out.push_str(&format!("Binary {op}\n"));
            dump_expr(left, level + 1, out);
            dump_expr(right, level + 1, out);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            indent(level, out);
            out.push_str("Conditional\n");
            dump_expr(test, level + 1, out);
            dump_expr(consequent, level + 1, out);
            dump_expr(alternate, level + 1, out);
        }
        Expr::Assign {
            target, op, value, ..
        } => {
            indent(level, out);
            out.push_str(&format!("Assign {op}\n"));
            dump_expr(target, level + 1, out);
            dump_expr(value, level + 1, out);
        }
        Expr::Update {
            op, arg, prefix, ..
        } => {
            indent(level, out);
            if *prefix {
                out.push_str(&format!("Update prefix {op}\n"));
            } else {
                out.push_str(&format!("Update postfix {op}\n"));
            }
            dump_expr(arg, level + 1, out);
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            indent(level, out);
            if *optional {
                out.push_str("Call optional\n");
            } else {
                out.push_str("Call\n");
            }
            indent(level + 1, out);
            out.push_str("callee:\n");
            dump_expr(callee, level + 2, out);
            for (i, arg) in args.iter().enumerate() {
                indent(level + 1, out);
                match arg {
                    Arg::Expr(expr) => {
                        out.push_str(&format!("arg[{i}]:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                    Arg::Spread(expr) => {
                        out.push_str(&format!("arg[{i}] spread:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                }
            }
        }
        Expr::New { callee, args, .. } => {
            indent(level, out);
            out.push_str("New\n");
            indent(level + 1, out);
            out.push_str("callee:\n");
            dump_expr(callee, level + 2, out);
            for (i, arg) in args.iter().enumerate() {
                indent(level + 1, out);
                match arg {
                    Arg::Expr(expr) => {
                        out.push_str(&format!("arg[{i}]:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                    Arg::Spread(expr) => {
                        out.push_str(&format!("arg[{i}] spread:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                }
            }
        }
        Expr::FunctionExpression {
            name,
            params,
            return_type,
            body,
            is_async,
            is_generator,
            is_method,
            ..
        } => {
            indent(level, out);
            out.push_str("FunctionExpression\n");
            if *is_async {
                indent(level + 1, out);
                out.push_str("async: true\n");
            }
            if *is_generator {
                indent(level + 1, out);
                out.push_str("generator: true\n");
            }
            if *is_method {
                indent(level + 1, out);
                out.push_str("method: true\n");
            }
            if let Some(name) = name {
                indent(level + 1, out);
                out.push_str(&format!("name: {}\n", name.name));
            }
            dump_params(params, level + 1, out);
            if let Some(ret) = return_type {
                indent(level + 1, out);
                out.push_str("returnType:\n");
                dump_type_ann(ret, level + 2, out);
            }
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Expr::ClassExpression {
            name,
            super_class,
            body,
            ..
        } => {
            indent(level, out);
            out.push_str("ClassExpression\n");
            if let Some(name) = name {
                indent(level + 1, out);
                out.push_str(&format!("name: {}\n", name.name));
            }
            if let Some(sc) = super_class {
                indent(level + 1, out);
                out.push_str("extends:\n");
                dump_expr(sc, level + 2, out);
            }
            for el in body {
                match el {
                    ClassElement::Constructor { params, body, .. } => {
                        indent(level + 1, out);
                        out.push_str("Constructor\n");
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::Method {
                        key,
                        params,
                        body,
                        is_static,
                        is_async,
                        is_generator,
                        is_private,
                        ..
                    } => {
                        indent(level + 1, out);
                        match (*is_static, *is_private) {
                            (true, true) => out.push_str("StaticPrivateMethod\n"),
                            (true, false) => out.push_str("StaticMethod\n"),
                            (false, true) => out.push_str("PrivateMethod\n"),
                            (false, false) => out.push_str("Method\n"),
                        }
                        dump_class_element_key(key, *is_private, level + 2, out);
                        if *is_async {
                            indent(level + 2, out);
                            out.push_str("async: true\n");
                        }
                        if *is_generator {
                            indent(level + 2, out);
                            out.push_str("generator: true\n");
                        }
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::Accessor {
                        kind,
                        key,
                        params,
                        body,
                        is_static,
                        is_private,
                        ..
                    } => {
                        indent(level + 1, out);
                        let kind_s = match kind {
                            AccessorKind::Get => "get",
                            AccessorKind::Set => "set",
                        };
                        match (*is_static, *is_private) {
                            (true, true) => {
                                out.push_str(&format!("StaticPrivateAccessor {kind_s}\n"))
                            }
                            (true, false) => out.push_str(&format!("StaticAccessor {kind_s}\n")),
                            (false, true) => out.push_str(&format!("PrivateAccessor {kind_s}\n")),
                            (false, false) => out.push_str(&format!("Accessor {kind_s}\n")),
                        }
                        dump_class_element_key(key, *is_private, level + 2, out);
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        indent(level + 1, out);
                        out.push_str("StaticBlock\n");
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::Field {
                        key,
                        value,
                        is_static,
                        is_private,
                        ..
                    } => {
                        indent(level + 1, out);
                        match (*is_static, *is_private) {
                            (true, true) => out.push_str("StaticPrivateField\n"),
                            (true, false) => out.push_str("StaticField\n"),
                            (false, true) => out.push_str("PrivateField\n"),
                            (false, false) => out.push_str("Field\n"),
                        }
                        dump_class_element_key(key, *is_private, level + 2, out);
                        if let Some(v) = value {
                            indent(level + 2, out);
                            out.push_str("value:\n");
                            dump_expr(v, level + 3, out);
                        }
                    }
                }
            }
        }
        Expr::ArrowFunction {
            params,
            return_type,
            body,
            is_async,
            ..
        } => {
            indent(level, out);
            out.push_str("ArrowFunction\n");
            if *is_async {
                indent(level + 1, out);
                out.push_str("async: true\n");
            }
            dump_params(params, level + 1, out);
            if let Some(ret) = return_type {
                indent(level + 1, out);
                out.push_str("returnType:\n");
                dump_type_ann(ret, level + 2, out);
            }
            indent(level + 1, out);
            out.push_str("body:\n");
            match body {
                ArrowBody::Expr(expr) => dump_expr(expr, level + 2, out),
                ArrowBody::Block(stmt) => dump_stmt(stmt, level + 2, out),
            }
        }
        Expr::ObjectExpression { properties, .. } => {
            indent(level, out);
            out.push_str("ObjectExpression\n");
            for prop in properties {
                match prop {
                    ObjectProp::Property {
                        key,
                        value,
                        shorthand,
                        ..
                    } => {
                        indent(level + 1, out);
                        if *shorthand {
                            out.push_str("prop shorthand:\n");
                        } else {
                            out.push_str("prop:\n");
                        }
                        indent(level + 2, out);
                        match key {
                            ObjectKey::Ident(id) => {
                                out.push_str(&format!("key: Ident {}\n", id.name))
                            }
                            ObjectKey::String(s) => out.push_str(&format!(
                                "key: String {:?}\n",
                                s.value.to_string_lossy()
                            )),
                            ObjectKey::Computed(expr) => {
                                out.push_str("key: Computed\n");
                                dump_expr(expr, level + 3, out);
                            }
                        }
                        indent(level + 2, out);
                        out.push_str("value:\n");
                        dump_expr(value, level + 3, out);
                    }
                    ObjectProp::Accessor {
                        kind,
                        key,
                        params,
                        body,
                        ..
                    } => {
                        indent(level + 1, out);
                        let kind_s = match kind {
                            AccessorKind::Get => "get",
                            AccessorKind::Set => "set",
                        };
                        out.push_str(&format!("accessor {kind_s}:\n"));
                        indent(level + 2, out);
                        match key {
                            ObjectKey::Ident(id) => {
                                out.push_str(&format!("key: Ident {}\n", id.name))
                            }
                            ObjectKey::String(s) => out.push_str(&format!(
                                "key: String {:?}\n",
                                s.value.to_string_lossy()
                            )),
                            ObjectKey::Computed(expr) => {
                                out.push_str("key: Computed\n");
                                dump_expr(expr, level + 3, out);
                            }
                        }
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ObjectProp::Spread { expr, .. } => {
                        indent(level + 1, out);
                        out.push_str("spread:\n");
                        dump_expr(expr, level + 2, out);
                    }
                }
            }
        }
        Expr::ArrayExpression { elements, .. } => {
            indent(level, out);
            out.push_str("ArrayExpression\n");
            for (i, el) in elements.iter().enumerate() {
                indent(level + 1, out);
                match el {
                    ArrayElement::Expr(expr) => {
                        out.push_str(&format!("element[{i}]:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                    ArrayElement::Spread(expr) => {
                        out.push_str(&format!("element[{i}] spread:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                    ArrayElement::Elision => {
                        out.push_str(&format!("element[{i}] elision\n"));
                    }
                }
            }
        }
        Expr::MemberExpression {
            object,
            property,
            computed,
            optional,
            private,
            ..
        } => {
            indent(level, out);
            if *private {
                match *optional {
                    true => out.push_str("PrivateMemberExpression optional\n"),
                    false => out.push_str("PrivateMemberExpression\n"),
                }
            } else {
                match (*optional, *computed) {
                    (true, true) => out.push_str("MemberExpression optional computed\n"),
                    (true, false) => out.push_str("MemberExpression optional\n"),
                    (false, true) => out.push_str("MemberExpression computed\n"),
                    (false, false) => out.push_str("MemberExpression\n"),
                }
            }
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("property:\n");
            dump_expr(property, level + 2, out);
        }
        Expr::PrivateIn { name, object, .. } => {
            indent(level, out);
            out.push_str("PrivateIn\n");
            indent(level + 1, out);
            out.push_str(&format!("name: #{}\n", name.name));
            dump_expr(object, level + 1, out);
        }
        Expr::Paren { expr, .. } => {
            indent(level, out);
            out.push_str("Paren\n");
            dump_expr(expr, level + 1, out);
        }
        Expr::As { expr, ty, .. } => {
            indent(level, out);
            out.push_str("As\n");
            dump_expr(expr, level + 1, out);
            indent(level + 1, out);
            out.push_str("type:\n");
            dump_type_ann(ty, level + 2, out);
        }
        Expr::ArrayPattern { elements, .. } => {
            indent(level, out);
            out.push_str("ArrayPattern\n");
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {
                        indent(level + 1, out);
                        out.push_str("elision\n");
                    }
                    ArrayPatternElement::Pattern { binding, default } => {
                        dump_binding_pattern(binding, level + 1, out);
                        if let Some(def) = default {
                            indent(level + 1, out);
                            out.push_str("default:\n");
                            dump_expr(def, level + 2, out);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        indent(level + 1, out);
                        out.push_str("rest:\n");
                        dump_binding_pattern(binding, level + 2, out);
                    }
                }
            }
        }
        Expr::ObjectPattern { properties, .. } => {
            indent(level, out);
            out.push_str("ObjectPattern\n");
            dump_object_pattern_props(properties, level + 1, out);
        }
    }
}

fn dump_class_element_key(key: &ObjectKey, is_private: bool, level: usize, out: &mut String) {
    indent(level, out);
    match key {
        ObjectKey::Ident(id) if is_private => {
            out.push_str(&format!("name: #{}\n", id.name));
        }
        ObjectKey::Ident(id) => {
            out.push_str(&format!("name: {}\n", id.name));
        }
        ObjectKey::String(s) => {
            out.push_str(&format!("key: String {:?}\n", s.value.to_string_lossy()));
        }
        ObjectKey::Computed(expr) => {
            out.push_str("key: Computed\n");
            dump_expr(expr, level + 1, out);
        }
    }
}

fn dump_params(params: &[Param], level: usize, out: &mut String) {
    if params.is_empty() {
        return;
    }
    indent(level, out);
    out.push_str("params:\n");
    for p in params {
        match (&p.binding, p.rest) {
            (BindingPattern::Ident(id), true) => {
                indent(level + 1, out);
                out.push_str(&format!("rest: {}\n", id.name));
            }
            (binding, false) => {
                dump_binding_pattern(binding, level + 1, out);
            }
            (binding, true) => {
                indent(level + 1, out);
                out.push_str("rest:\n");
                dump_binding_pattern(binding, level + 2, out);
            }
        }
        if let Some(ann) = &p.type_ann {
            indent(level + 2, out);
            out.push_str("type:\n");
            dump_type_ann(ann, level + 3, out);
        }
        if let Some(default) = &p.default {
            indent(level + 2, out);
            out.push_str("default:\n");
            dump_expr(default, level + 3, out);
        }
    }
}

fn dump_type_ann(ann: &TypeAnn, level: usize, out: &mut String) {
    match ann {
        TypeAnn::Named { name, .. } => {
            indent(level, out);
            out.push_str(&format!("NamedType {}\n", name));
        }
        TypeAnn::GenericApp { name, args, .. } => {
            indent(level, out);
            out.push_str(&format!("GenericApp {}\n", name));
            for a in args {
                dump_type_ann(a, level + 1, out);
            }
        }
        TypeAnn::Object { props, .. } => {
            indent(level, out);
            out.push_str("ObjectType\n");
            for p in props {
                indent(level + 1, out);
                out.push_str(&format!("prop: {}\n", p.name));
                dump_type_ann(&p.ty, level + 2, out);
            }
        }
        TypeAnn::Tuple { elements, .. } => {
            indent(level, out);
            out.push_str("TupleType\n");
            for el in elements {
                dump_type_ann(el, level + 1, out);
            }
        }
        TypeAnn::Pointer { inner, .. } => {
            indent(level, out);
            out.push_str("PointerType\n");
            dump_type_ann(inner, level + 1, out);
        }
        TypeAnn::Union { types, .. } => {
            indent(level, out);
            out.push_str("UnionType\n");
            for t in types {
                dump_type_ann(t, level + 1, out);
            }
        }
        TypeAnn::Intersection { types, .. } => {
            indent(level, out);
            out.push_str("IntersectionType\n");
            for t in types {
                dump_type_ann(t, level + 1, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_diagnostics::Span;

    #[test]
    fn dump_let_number() {
        let program = Program {
            body: vec![Stmt::Let {
                kind: BindingKind::Let,
                binding: BindingPattern::Ident(Ident {
                    name: "x".into(),
                    span: Span::dummy(),
                }),
                type_ann: None,
                init: Some(Expr::Number(NumberLit {
                    raw: "1".into(),
                    span: Span::dummy(),
                })),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let dump = dump_program(&program);
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Number 1
"
        );
    }
}
