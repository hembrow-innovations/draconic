use draconic_diagnostics::Span;

use crate::{Expr, Ident, ImportPhase, ObjectKey, Param, StringLit, TypeAnn};

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
