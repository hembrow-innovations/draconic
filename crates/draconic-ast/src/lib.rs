use draconic_diagnostics::Span;
pub use draconic_lexer::JsString;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// Lexical binding kind for `let` / `const` declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingKind {
    Let,
    Const,
    /// Function declaration binding (hoisted, not reassignable in the minimal surface).
    Function,
}

/// Binding target for `let` / `const`: simple name or array destructuring pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum BindingPattern {
    Ident(Ident),
    /// `[a, b, ...rest]` (no holes/defaults in this surface).
    Array {
        elements: Vec<ArrayPatternElement>,
        span: Span,
    },
}

/// One element of an array binding/assignment pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayPatternElement {
    /// Nested or simple binding (`a` or `[a, b]`).
    Pattern(BindingPattern),
    /// `...name` rest (must be last; simple ident only).
    Rest(Ident),
}

impl BindingPattern {
    pub fn span(&self) -> Span {
        match self {
            BindingPattern::Ident(id) => id.span,
            BindingPattern::Array { span, .. } => *span,
        }
    }

    /// Visit every identifier bound by this pattern (declaration names).
    pub fn for_each_ident(&self, f: &mut dyn FnMut(&Ident)) {
        match self {
            BindingPattern::Ident(id) => f(id),
            BindingPattern::Array { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Pattern(p) => p.for_each_ident(f),
                        ArrayPatternElement::Rest(id) => f(id),
                    }
                }
            }
        }
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
    /// `for (left of right) body` — `left` is `Let` or assignable `Expression`.
    ForOf {
        left: Box<Stmt>,
        right: Expr,
        body: Box<Stmt>,
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
    /// `async? function *? name (params): ret? { body }`
    FunctionDeclaration {
        name: Ident,
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
        /// Catch parameter name when present (`catch (e)`).
        handler_param: Option<Ident>,
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
    /// / `import * as ns from "mod"` / `import d, * as ns from "mod"`.
    /// Default import is a specifier with `imported.name == "default"`.
    /// Namespace import binds `namespace` to a module namespace object.
    ImportDeclaration {
        specifiers: Vec<ImportSpecifier>,
        /// `import * as name` binding, when present.
        namespace: Option<Ident>,
        source: StringLit,
        span: Span,
    },
    /// `export let/const/function …` or `export { a, b as c }`
    ExportNamedDeclaration {
        /// Present for `export let` / `export const` / `export function`.
        declaration: Option<Box<Stmt>>,
        /// Present for `export { … }` (and empty when declaration carries the names).
        specifiers: Vec<ExportSpecifier>,
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
}

/// One binding of `import { imported as local }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportSpecifier {
    /// Exported name in the source module.
    pub imported: Ident,
    /// Local binding name in this module.
    pub local: Ident,
}

/// One binding of `export { local as exported }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSpecifier {
    /// Local name in this module.
    pub local: Ident,
    /// Name under which it is exported.
    pub exported: Ident,
}

/// One element of a class body (`constructor` or method).
#[derive(Debug, Clone, PartialEq)]
pub enum ClassElement {
    /// `constructor(params) { body }`
    Constructor {
        params: Vec<Param>,
        body: Box<Stmt>,
        span: Span,
    },
    /// `static? *? name(params) { body }` instance or static method (optional generator)
    Method {
        name: Ident,
        params: Vec<Param>,
        body: Box<Stmt>,
        is_static: bool,
        is_generator: bool,
        span: Span,
    },
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
    /// `[elem, …]` array literal (spread elements allowed; no holes in this surface).
    ArrayExpression {
        elements: Vec<ArrayElement>,
        span: Span,
    },
    /// `obj.prop` or `obj[expr]` (property read).
    MemberExpression {
        object: Box<Expr>,
        /// Non-computed: `Expr::Ident`. Computed: any expression.
        property: Box<Expr>,
        computed: bool,
        span: Span,
    },
    /// Parenthesized expression — preserved for dump fidelity.
    Paren {
        expr: Box<Expr>,
        span: Span,
    },
    /// Array destructuring pattern used as assignment target: `[a, b, ...rest]`.
    ArrayPattern {
        elements: Vec<ArrayPatternElement>,
        span: Span,
    },
}

/// One element of an array literal: value or `...spread`.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayElement {
    Expr(Expr),
    Spread(Expr),
}

/// One argument of a call or `new`: value or `...spread`.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Expr(Expr),
    Spread(Expr),
}

/// One property in an object literal (`key: value`, shorthand, or method).
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectProp {
    pub key: ObjectKey,
    pub value: Expr,
    /// True for property shorthand `{ a }` (value is the same Ident as key).
    pub shorthand: bool,
    pub span: Span,
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

/// Formal parameter: `name`, `name: T`, `name = default`, `name: T = default`, or `...name` / `...name: T`.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: Ident,
    /// Optional type annotation after the parameter name.
    pub type_ann: Option<TypeAnn>,
    pub default: Option<Expr>,
    /// `true` for a rest parameter (`...name`). Must be last; no default.
    pub rest: bool,
}

/// Type annotation (`: TypeName`) — TS-inspired named types (T01 surface).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAnn {
    pub name: String,
    pub span: Span,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOp {
    Inc,
    Dec,
}

/// Simple `=` or compound assignment operator (`+=`, `-=`, …).
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

fn dump_binding_pattern(pat: &BindingPattern, level: usize, out: &mut String) {
    match pat {
        BindingPattern::Ident(name) => {
            indent(level, out);
            out.push_str(&format!("name: {}\n", name.name));
        }
        BindingPattern::Array { elements, .. } => {
            indent(level, out);
            out.push_str("ArrayPattern\n");
            for el in elements {
                match el {
                    ArrayPatternElement::Pattern(p) => {
                        dump_binding_pattern(p, level + 1, out);
                    }
                    ArrayPatternElement::Rest(id) => {
                        indent(level + 1, out);
                        out.push_str(&format!("rest: {}\n", id.name));
                    }
                }
            }
        }
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
                BindingKind::Function => out.push_str("FunctionBinding\n"),
            }
            dump_binding_pattern(binding, level + 1, out);
            if let Some(ann) = type_ann {
                indent(level + 1, out);
                out.push_str(&format!("type: {}\n", ann.name));
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
            left,
            right,
            body,
            ..
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
            ..
        } => {
            indent(level, out);
            out.push_str("ForOf\n");
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
            dump_params(params, level + 1, out);
            if let Some(ret) = return_type {
                indent(level + 1, out);
                out.push_str(&format!("returnType: {}\n", ret.name));
            }
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
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
                        name,
                        params,
                        body,
                        is_static,
                        is_generator,
                        ..
                    } => {
                        indent(level + 1, out);
                        if *is_static {
                            out.push_str("StaticMethod\n");
                        } else {
                            out.push_str("Method\n");
                        }
                        indent(level + 2, out);
                        out.push_str(&format!("name: {}\n", name.name));
                        if *is_generator {
                            indent(level + 2, out);
                            out.push_str("generator: true\n");
                        }
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
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
            ..
        } => {
            indent(level, out);
            out.push_str("ImportDeclaration\n");
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
        }
        Stmt::ExportNamedDeclaration {
            declaration,
            specifiers,
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
        }
        Stmt::ExportDefaultDeclaration {
            declaration,
            local,
            ..
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
                    out.push_str(&format!(" ({})", param.name));
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
            target,
            op,
            value,
            ..
        } => {
            indent(level, out);
            out.push_str(&format!("Assign {op}\n"));
            dump_expr(target, level + 1, out);
            dump_expr(value, level + 1, out);
        }
        Expr::Update {
            op,
            arg,
            prefix,
            ..
        } => {
            indent(level, out);
            if *prefix {
                out.push_str(&format!("Update prefix {op}\n"));
            } else {
                out.push_str(&format!("Update postfix {op}\n"));
            }
            dump_expr(arg, level + 1, out);
        }
        Expr::Call { callee, args, .. } => {
            indent(level, out);
            out.push_str("Call\n");
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
            if let Some(name) = name {
                indent(level + 1, out);
                out.push_str(&format!("name: {}\n", name.name));
            }
            dump_params(params, level + 1, out);
            if let Some(ret) = return_type {
                indent(level + 1, out);
                out.push_str(&format!("returnType: {}\n", ret.name));
            }
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
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
                out.push_str(&format!("returnType: {}\n", ret.name));
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
                indent(level + 1, out);
                if prop.shorthand {
                    out.push_str("prop shorthand:\n");
                } else {
                    out.push_str("prop:\n");
                }
                indent(level + 2, out);
                match &prop.key {
                    ObjectKey::Ident(id) => out.push_str(&format!("key: Ident {}\n", id.name)),
                    ObjectKey::String(s) => {
                        out.push_str(&format!("key: String {:?}\n", s.value.to_string_lossy()))
                    }
                    ObjectKey::Computed(expr) => {
                        out.push_str("key: Computed\n");
                        dump_expr(expr, level + 3, out);
                    }
                }
                indent(level + 2, out);
                out.push_str("value:\n");
                dump_expr(&prop.value, level + 3, out);
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
                }
            }
        }
        Expr::MemberExpression {
            object,
            property,
            computed,
            ..
        } => {
            indent(level, out);
            if *computed {
                out.push_str("MemberExpression computed\n");
            } else {
                out.push_str("MemberExpression\n");
            }
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("property:\n");
            dump_expr(property, level + 2, out);
        }
        Expr::Paren { expr, .. } => {
            indent(level, out);
            out.push_str("Paren\n");
            dump_expr(expr, level + 1, out);
        }
        Expr::ArrayPattern { elements, .. } => {
            indent(level, out);
            out.push_str("ArrayPattern\n");
            for el in elements {
                match el {
                    ArrayPatternElement::Pattern(p) => {
                        dump_binding_pattern(p, level + 1, out);
                    }
                    ArrayPatternElement::Rest(id) => {
                        indent(level + 1, out);
                        out.push_str(&format!("rest: {}\n", id.name));
                    }
                }
            }
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
        indent(level + 1, out);
        if p.rest {
            out.push_str(&format!("rest: {}\n", p.name.name));
        } else {
            out.push_str(&format!("name: {}\n", p.name.name));
        }
        if let Some(ann) = &p.type_ann {
            indent(level + 2, out);
            out.push_str(&format!("type: {}\n", ann.name));
        }
        if let Some(default) = &p.default {
            indent(level + 2, out);
            out.push_str("default:\n");
            dump_expr(default, level + 3, out);
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
