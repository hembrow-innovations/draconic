use draconic_ast::BindingKind;
use draconic_check::{SymbolId as LocalId, Type};

use crate::{ArrayPatternEl, AssignTarget, Expr, ObjectPatternEl, Param, Pattern};

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let` / `const` name = init; or `let name;`
    Declare {
        local: LocalId,
        init: Option<Expr>,
        kind: BindingKind,
    },
    /// `let` / `const` `[a, b, ...rest] = init;` — `init` is `None` for for-in/of heads.
    DeclareArrayPattern {
        kind: BindingKind,
        elements: Vec<ArrayPatternEl>,
        init: Option<Expr>,
    },
    /// `let` / `const` `{ a, b: c, ...rest } = init;` — `init` is `None` for for-in/of heads.
    DeclareObjectPattern {
        kind: BindingKind,
        properties: Vec<ObjectPatternEl>,
        init: Option<Expr>,
    },
    /// Assignment-pattern / member LHS of `for (… in/of …)` without a declaration keyword.
    AssignLeft {
        target: AssignTarget,
    },
    Expr {
        expr: Expr,
    },
    Block {
        body: Vec<Stmt>,
    },
    If {
        test: Expr,
        consequent: Box<Stmt>,
        alternate: Option<Box<Stmt>>,
    },
    While {
        test: Expr,
        body: Box<Stmt>,
    },
    DoWhile {
        body: Box<Stmt>,
        test: Expr,
    },
    For {
        init: Option<Box<Stmt>>,
        test: Option<Expr>,
        update: Option<Expr>,
        body: Box<Stmt>,
    },
    ForIn {
        left: Box<Stmt>,
        right: Expr,
        body: Box<Stmt>,
    },
    ForOf {
        left: Box<Stmt>,
        right: Expr,
        body: Box<Stmt>,
        is_await: bool,
    },
    Break {
        label: Option<String>,
    },
    Continue {
        label: Option<String>,
    },
    Labeled {
        label: String,
        body: Box<Stmt>,
    },
    Switch {
        discriminant: Expr,
        cases: Vec<SwitchCase>,
    },
    /// `async? function *? name(params) { body }`
    Function {
        local: LocalId,
        params: Vec<Param>,
        body: Vec<Stmt>,
        is_async: bool,
        is_generator: bool,
    },
    /// `extern "C" function name(params): ret?;` — C ABI surface for LLVM (F06.03).
    /// No body; linkage name is the source binding name. Param/return types are
    /// native scalars or pointers (`void` return → `ret: None`).
    ExternFunction {
        local: LocalId,
        /// ABI string (v1: `"C"`).
        abi: String,
        /// Linkage / symbol name (source function name).
        name: String,
        /// Ordered ABI parameter types (native scalar or `*T`).
        params: Vec<Type>,
        /// Return type; `None` means C `void`.
        ret: Option<Type>,
    },
    /// `return;` or `return value;`
    Return {
        value: Option<Expr>,
    },
    /// `throw value;`
    Throw {
        value: Expr,
    },
    /// `try { … } catch (param?) { … }? finally { … }?`
    Try {
        block: Vec<Stmt>,
        /// Catch parameter pattern when present (`e` / `[a]` / `{x}`).
        handler_param: Option<Pattern>,
        /// Catch body when a `catch` clause is present.
        handler: Option<Vec<Stmt>>,
        /// `finally` body when present.
        finalizer: Option<Vec<Stmt>>,
    },
    /// `with (object) body` — non-strict Object Environment.
    With {
        object: Expr,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    /// `None` means `default`.
    pub test: Option<Expr>,
    pub body: Vec<Stmt>,
}
