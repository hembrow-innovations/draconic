//! Shared IR lowered from checked Programs (ROADMAP B06).

use std::collections::HashMap;

use draconic_ast::{
    AccessorKind, Arg as AstArg, ArrayElement as AstArrayElement, ArrayPatternElement, AssignOp,
    BinaryOp, BindingPattern, ClassElement, Expr as AstExpr, Ident, ObjectPatternProp,
    ObjectProp as AstObjectProp, Stmt as AstStmt, UnaryOp, UpdateOp,
};
use draconic_check::{CheckedProgram, Type};
use draconic_diagnostics::Span;

pub use draconic_ast::BindingKind;
pub use draconic_check::{NativeType, ObjectShape, SymbolId as LocalId, Type as IrType};

/// Per-`lower` bookkeeping for private fields/methods/brands and synthetic locals.
/// Owned by `lower` for the duration of one lowering — no process-global state.
struct LowerCtx {
    /// Private field name → WeakMap local.
    private_fields: HashMap<String, LocalId>,
    /// Private method name → function local.
    private_methods: HashMap<String, LocalId>,
    /// Private accessor name → (get fn, set fn) locals.
    private_accessors: HashMap<String, (Option<LocalId>, Option<LocalId>)>,
    /// Private method/accessor brand → WeakSet local (E18.40; fields use WeakMap as brand).
    private_brands: HashMap<String, LocalId>,
    extra_locals: Vec<Local>,
    next_synth_id: u32,
}

impl LowerCtx {
    fn new(next_synth_id: u32) -> Self {
        Self {
            private_fields: HashMap::new(),
            private_methods: HashMap::new(),
            private_accessors: HashMap::new(),
            private_brands: HashMap::new(),
            extra_locals: Vec::new(),
            next_synth_id,
        }
    }

    fn alloc_synthetic_local(&mut self, name: String, ty: Type) -> LocalId {
        let id = LocalId(self.next_synth_id);
        self.next_synth_id += 1;
        self.extra_locals.push(Local {
            id,
            name,
            ty,
            kind: BindingKind::Let,
        });
        id
    }
}

/// Top-level IR unit both backends consume.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub locals: Vec<Local>,
    pub body: Vec<Stmt>,
    /// Original source span for each top-level `body` entry (same length as `body`).
    /// Expanded lowerings (e.g. class → several stmts) share the originating AST span.
    pub body_spans: Vec<Span>,
    /// Structural object shapes referenced by `Type::Shape` (N03 native layouts).
    pub shapes: Vec<ObjectShape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub id: LocalId,
    pub name: String,
    pub ty: Type,
    pub kind: BindingKind,
}

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
        /// Catch parameter local when present.
        handler_param: Option<LocalId>,
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
    /// `key` / `key: pattern` / defaults (static ident key only).
    Prop {
        key: String,
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

/// Lower a checked Program to shared IR.
pub fn lower(checked: &CheckedProgram) -> Module {
    let mut locals: Vec<Local> = checked
        .bound
        .symbols()
        .iter()
        .map(|s| Local {
            id: s.id,
            name: s.name.clone(),
            ty: checked.type_of_symbol(s.id),
            kind: s.kind,
        })
        .collect();

    let max_id = locals.iter().map(|l| l.id.0).max().unwrap_or(0);
    let mut ctx = LowerCtx::new(max_id.saturating_add(1));

    let mut body = Vec::new();
    let mut body_spans = Vec::new();
    for stmt in &checked.bound.program.body {
        let span = ast_stmt_span(stmt);
        let expanded = lower_stmt_expand(checked, &mut ctx, stmt, None);
        for s in expanded {
            body.push(s);
            body_spans.push(span);
        }
    }

    locals.extend(ctx.extra_locals.drain(..));

    debug_assert_eq!(body.len(), body_spans.len());
    Module {
        locals,
        body,
        body_spans,
        shapes: checked.shapes().to_vec(),
    }
}

fn ast_stmt_span(stmt: &AstStmt) -> Span {
    match stmt {
        AstStmt::Expression { span, .. }
        | AstStmt::Let { span, .. }
        | AstStmt::Empty { span }
        | AstStmt::Block { span, .. }
        | AstStmt::If { span, .. }
        | AstStmt::While { span, .. }
        | AstStmt::DoWhile { span, .. }
        | AstStmt::For { span, .. }
        | AstStmt::ForIn { span, .. }
        | AstStmt::ForOf { span, .. }
        | AstStmt::Break { span, .. }
        | AstStmt::Continue { span, .. }
        | AstStmt::Labeled { span, .. }
        | AstStmt::Switch { span, .. }
        | AstStmt::FunctionDeclaration { span, .. }
        | AstStmt::ClassDeclaration { span, .. }
        | AstStmt::Return { span, .. }
        | AstStmt::Throw { span, .. }
        | AstStmt::Try { span, .. }
        | AstStmt::With { span, .. }
        | AstStmt::ImportDeclaration { span, .. }
        | AstStmt::ExportNamedDeclaration { span, .. }
        | AstStmt::ExportDefaultDeclaration { span, .. }
        | AstStmt::ExportAllDeclaration { span, .. }
        | AstStmt::TypeAlias { span, .. } => *span,
    }
}

/// Lower one AST statement, expanding constructs that become multiple IR stmts (e.g. class).
fn lower_stmt_expand(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    stmt: &AstStmt,
    super_class: Option<&AstExpr>,
) -> Vec<Stmt> {
    match stmt {
        AstStmt::ClassDeclaration {
            name,
            super_class: sc,
            body,
            ..
        } => lower_class(checked, ctx, name, sc.as_deref(), body),
        other => lower_stmt(checked, ctx, other, super_class)
            .into_iter()
            .collect(),
    }
}

fn lower_stmt_body(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    body: &[AstStmt],
    super_class: Option<&AstExpr>,
) -> Vec<Stmt> {
    let mut out = Vec::new();
    for s in body {
        out.extend(lower_stmt_expand(checked, ctx, s, super_class));
    }
    out
}

fn lower_stmt(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    stmt: &AstStmt,
    super_class: Option<&AstExpr>,
) -> Option<Stmt> {
    match stmt {
        AstStmt::Empty { .. } => None,
        AstStmt::ClassDeclaration { .. } => {
            // Expanded via `lower_stmt_expand`.
            None
        }
        AstStmt::Expression { expr, .. } => match expr {
            AstExpr::ArrayPattern { elements, .. } => Some(Stmt::AssignLeft {
                target: AssignTarget::ArrayPattern {
                    elements: lower_array_pattern_els(checked, ctx, elements),
                },
            }),
            AstExpr::ObjectPattern { properties, .. } => Some(Stmt::AssignLeft {
                target: AssignTarget::ObjectPattern {
                    properties: lower_object_pattern_props(checked, ctx, properties),
                },
            }),
            _ => Some(Stmt::Expr {
                expr: lower_expr(checked, ctx, expr, super_class),
            }),
        },
        AstStmt::Let {
            kind,
            binding,
            init,
            ..
        } => match binding {
            BindingPattern::Ident(name) => {
                let local = checked
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == name.span)
                    .map(|s| s.id)
                    .expect("let binding must be declared");
                Some(Stmt::Declare {
                    local,
                    init: init
                        .as_ref()
                        .map(|e| lower_expr(checked, ctx, e, super_class)),
                    kind: *kind,
                })
            }
            BindingPattern::Array { elements, .. } => Some(Stmt::DeclareArrayPattern {
                kind: *kind,
                elements: lower_array_pattern_els(checked, ctx, elements),
                init: init
                    .as_ref()
                    .map(|e| lower_expr(checked, ctx, e, super_class)),
            }),
            BindingPattern::Object { properties, .. } => Some(Stmt::DeclareObjectPattern {
                kind: *kind,
                properties: lower_object_pattern_props(checked, ctx, properties),
                init: init
                    .as_ref()
                    .map(|e| lower_expr(checked, ctx, e, super_class)),
            }),
            BindingPattern::Member(_) => {
                panic!("member binding is assignment-only; rejected at check")
            }
        }
        AstStmt::Block { body, .. } => {
            let body = lower_stmt_body(checked, ctx, body, super_class);
            Some(Stmt::Block { body })
        }
        AstStmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            let consequent = Box::new(
                lower_stmt(checked, ctx, consequent, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            let alternate = alternate.as_ref().map(|alt| {
                Box::new(
                    lower_stmt(checked, ctx, alt, super_class)
                        .unwrap_or(Stmt::Block { body: vec![] }),
                )
            });
            Some(Stmt::If {
                test: lower_expr(checked, ctx, test, super_class),
                consequent,
                alternate,
            })
        }
        AstStmt::While { test, body, .. } => {
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::While {
                test: lower_expr(checked, ctx, test, super_class),
                body,
            })
        }
        AstStmt::DoWhile { body, test, .. } => {
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::DoWhile {
                body,
                test: lower_expr(checked, ctx, test, super_class),
            })
        }
        AstStmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            let init = init
                .as_ref()
                .and_then(|s| lower_stmt(checked, ctx, s, super_class).map(Box::new));
            let test = test
                .as_ref()
                .map(|e| lower_expr(checked, ctx, e, super_class));
            let update = update
                .as_ref()
                .map(|e| lower_expr(checked, ctx, e, super_class));
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::For {
                init,
                test,
                update,
                body,
            })
        }
        AstStmt::ForIn {
            left, right, body, ..
        } => {
            let left = Box::new(
                lower_stmt(checked, ctx, left, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::ForIn {
                left,
                right: lower_expr(checked, ctx, right, super_class),
                body,
            })
        }
        AstStmt::ForOf {
            left,
            right,
            body,
            is_await,
            ..
        } => {
            let left = Box::new(
                lower_stmt(checked, ctx, left, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::ForOf {
                left,
                right: lower_expr(checked, ctx, right, super_class),
                body,
                is_await: *is_await,
            })
        }
        AstStmt::Break { label, .. } => Some(Stmt::Break {
            label: label.as_ref().map(|l| l.name.clone()),
        }),
        AstStmt::Continue { label, .. } => Some(Stmt::Continue {
            label: label.as_ref().map(|l| l.name.clone()),
        }),
        AstStmt::Labeled { label, body, .. } => {
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::Labeled {
                label: label.name.clone(),
                body,
            })
        }
        AstStmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            let cases = cases
                .iter()
                .map(|c| SwitchCase {
                    test: c
                        .test
                        .as_ref()
                        .map(|e| lower_expr(checked, ctx, e, super_class)),
                    body: lower_stmt_body(checked, ctx, &c.body, super_class),
                })
                .collect();
            Some(Stmt::Switch {
                discriminant: lower_expr(checked, ctx, discriminant, super_class),
                cases,
            })
        }
        AstStmt::FunctionDeclaration {
            name,
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            let local = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.span == name.span)
                .map(|s| s.id)
                .expect("function binding must be declared");
            let params = lower_params(checked, ctx, params, None);
            // Nested functions do not inherit `super`.
            let body = lower_fn_body(checked, ctx, body, None);
            Some(Stmt::Function {
                local,
                params,
                body,
                is_async: *is_async,
                is_generator: *is_generator,
            })
        }
        AstStmt::Return { argument, .. } => Some(Stmt::Return {
            value: argument
                .as_ref()
                .map(|e| lower_expr(checked, ctx, e, super_class)),
        }),
        AstStmt::Throw { argument, .. } => Some(Stmt::Throw {
            value: lower_expr(checked, ctx, argument, super_class),
        }),
        AstStmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            ..
        } => {
            let block = lower_fn_body(checked, ctx, block, super_class);
            let handler_param = handler_param.as_ref().map(|param| {
                checked
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == param.span)
                    .map(|s| s.id)
                    .expect("catch binding must be declared")
            });
            let handler = handler
                .as_ref()
                .map(|h| lower_fn_body(checked, ctx, h, super_class));
            let finalizer = finalizer
                .as_ref()
                .map(|f| lower_fn_body(checked, ctx, f, super_class));
            Some(Stmt::Try {
                block,
                handler_param,
                handler,
                finalizer,
            })
        }
        AstStmt::With { object, body, .. } => Some(Stmt::With {
            object: lower_expr(checked, ctx, object, super_class),
            body: lower_fn_body(checked, ctx, body, super_class),
        }),
        AstStmt::ImportDeclaration { .. }
        | AstStmt::ExportNamedDeclaration { .. }
        | AstStmt::ExportDefaultDeclaration { .. }
        | AstStmt::ExportAllDeclaration { .. } => {
            panic!("import/export must be linked before lower")
        }
        // Type aliases are erased (T02); no runtime value.
        AstStmt::TypeAlias { .. } => None,
    }
}

fn lower_fn_body(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    body: &AstStmt,
    super_class: Option<&AstExpr>,
) -> Vec<Stmt> {
    match body {
        AstStmt::Block { body, .. } => lower_stmt_body(checked, ctx, body, super_class),
        other => lower_stmt_expand(checked, ctx, other, super_class),
    }
}

/// Desugar `class Name extends? Super { constructor… methods… fields… }` to function + assigns.
fn lower_class(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    name: &Ident,
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
) -> Vec<Stmt> {
    let local = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.span == name.span)
        .map(|s| s.id)
        .expect("class binding must be declared");
    lower_class_local(checked, ctx, local, super_class, elements)
}

/// Class expression → IIFE that builds the constructor and returns it (E18.33).
fn lower_class_expression(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    name: Option<&Ident>,
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
    span: Span,
) -> Expr {
    let class_span = name.map(|n| n.span).unwrap_or(span);
    let local = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.span == class_span)
        .map(|s| s.id)
        .expect("class expression binding must be declared");
    let mut body = lower_class_local(checked, ctx, local, super_class, elements);
    body.push(Stmt::Return {
        value: Some(Expr::Local {
            id: local,
            ty: Type::Function,
        }),
    });
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: Vec::new(),
            body,
            is_async: false,
            is_generator: false,
            is_arrow: false,
            ty: Type::Function,
        }),
        args: Vec::new(),
        optional: false,
        ty: Type::Function,
    }
}

fn lower_class_local(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    local: LocalId,
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
) -> Vec<Stmt> {
    let mut ctor_params = Vec::new();
    let mut ctor_body_ast: Option<&AstStmt> = None;
    let mut methods: Vec<(
        &Ident,
        &Vec<draconic_ast::Param>,
        &AstStmt,
        bool,
        bool,
        bool,
        bool,
    )> = Vec::new();
    let mut accessors: Vec<(
        AccessorKind,
        &Ident,
        &Vec<draconic_ast::Param>,
        &AstStmt,
        bool,
        bool,
    )> = Vec::new();
    let mut instance_fields: Vec<(&Ident, Option<&AstExpr>, bool)> = Vec::new();
    // Static fields and static blocks in source order (E18.41).
    enum StaticInit<'a> {
        Field(&'a Ident, Option<&'a AstExpr>, bool),
        Block(&'a AstStmt),
    }
    let mut static_inits: Vec<StaticInit<'_>> = Vec::new();

    for el in elements {
        match el {
            ClassElement::Constructor { params, body, .. } => {
                ctor_params = lower_params(checked, ctx, params, super_class);
                ctor_body_ast = Some(body.as_ref());
            }
            ClassElement::Method {
                name: method_name,
                params,
                body,
                is_static,
                is_async,
                is_generator,
                is_private,
                ..
            } => {
                methods.push((
                    method_name,
                    params,
                    body.as_ref(),
                    *is_static,
                    *is_async,
                    *is_generator,
                    *is_private,
                ));
            }
            ClassElement::Accessor {
                kind,
                name: acc_name,
                params,
                body,
                is_static,
                is_private,
                ..
            } => {
                accessors.push((
                    *kind,
                    acc_name,
                    params,
                    body.as_ref(),
                    *is_static,
                    *is_private,
                ));
            }
            ClassElement::Field {
                name: field_name,
                value,
                is_static,
                is_private,
                ..
            } => {
                let v = value.as_ref();
                if *is_static {
                    static_inits.push(StaticInit::Field(field_name, v, *is_private));
                } else {
                    instance_fields.push((field_name, v, *is_private));
                }
            }
            ClassElement::StaticBlock { body, .. } => {
                static_inits.push(StaticInit::Block(body.as_ref()));
            }
        }
    }

    // WeakMap per private field (E18.35 instance; E18.36 static — class as key).
    let mut private_map: HashMap<String, LocalId> = HashMap::new();
    let mut private_wm_decls: Vec<Stmt> = Vec::new();
    let mut add_private_wm = |fname: &Ident| {
        if private_map.contains_key(&fname.name) {
            return;
        }
        let wm_name = format!("__drac_pf_{}_{}", local.0, fname.name);
        let wm_id = ctx.alloc_synthetic_local(wm_name, Type::Any);
        private_map.insert(fname.name.clone(), wm_id);
        private_wm_decls.push(Stmt::Declare {
            local: wm_id,
            init: Some(Expr::New {
                callee: Box::new(Expr::IdentName {
                    name: "WeakMap".into(),
                    ty: Type::Function,
                }),
                args: Vec::new(),
                ty: Type::Any,
            }),
            kind: BindingKind::Let,
        });
    };
    for (fname, _, is_private) in &instance_fields {
        if *is_private {
            add_private_wm(fname);
        }
    }
    for init in &static_inits {
        if let StaticInit::Field(fname, _, is_private) = init {
            if *is_private {
                add_private_wm(fname);
            }
        }
    }

    // Private methods: synthetic function locals (E18.37 instance; E18.38 static). Bodies lowered after maps are live.
    let mut private_method_map: HashMap<String, LocalId> = HashMap::new();
    let mut private_method_meta: Vec<(
        LocalId,
        &Vec<draconic_ast::Param>,
        &AstStmt,
        bool,
        bool,
    )> = Vec::new();
    let mut private_brand_map: HashMap<String, LocalId> = HashMap::new();
    let mut private_brand_decls: Vec<Stmt> = Vec::new();
    let mut instance_brands: Vec<LocalId> = Vec::new();
    let mut static_brands: Vec<LocalId> = Vec::new();
    for (method_name, params, body, is_static, is_async, is_generator, is_private) in &methods {
        if !*is_private {
            continue;
        }
        if private_method_map.contains_key(&method_name.name) {
            continue;
        }
        let fn_name = format!("__drac_pm_{}_{}", local.0, method_name.name);
        let fn_id = ctx.alloc_synthetic_local(fn_name, Type::Function);
        private_method_map.insert(method_name.name.clone(), fn_id);
        private_method_meta.push((fn_id, params, body, *is_async, *is_generator));
        ensure_private_brand(
            ctx,
            local,
            &mut private_brand_map,
            &mut private_brand_decls,
            &mut instance_brands,
            &mut static_brands,
            &method_name.name,
            *is_static,
        );
    }

    // Private accessors: synthetic get/set function locals (E18.39).
    let mut private_accessor_map: HashMap<String, (Option<LocalId>, Option<LocalId>)> =
        HashMap::new();
    let mut private_accessor_meta: Vec<(
        LocalId,
        &Vec<draconic_ast::Param>,
        &AstStmt,
    )> = Vec::new();
    for (kind, acc_name, params, body, is_static, is_private) in &accessors {
        if !*is_private {
            continue;
        }
        let entry = private_accessor_map
            .entry(acc_name.name.clone())
            .or_insert((None, None));
        let tag = match kind {
            AccessorKind::Get => "g",
            AccessorKind::Set => "s",
        };
        let fn_name = format!("__drac_pa{}_{}_{}", tag, local.0, acc_name.name);
        let fn_id = ctx.alloc_synthetic_local(fn_name, Type::Function);
        match kind {
            AccessorKind::Get => entry.0 = Some(fn_id),
            AccessorKind::Set => entry.1 = Some(fn_id),
        }
        private_accessor_meta.push((fn_id, params, body));
        ensure_private_brand(
            ctx,
            local,
            &mut private_brand_map,
            &mut private_brand_decls,
            &mut instance_brands,
            &mut static_brands,
            &acc_name.name,
            *is_static,
        );
    }

    let prev_privates = std::mem::replace(&mut ctx.private_fields, private_map);
    let prev_private_methods = std::mem::replace(&mut ctx.private_methods, private_method_map);
    let prev_private_accessors =
        std::mem::replace(&mut ctx.private_accessors, private_accessor_map);
    let prev_private_brands = std::mem::replace(&mut ctx.private_brands, private_brand_map);

    let mut private_method_fns: Vec<Stmt> = Vec::new();
    for (fn_id, params, body, is_async, is_generator) in private_method_meta {
        private_method_fns.push(Stmt::Function {
            local: fn_id,
            params: lower_params(checked, ctx, params, super_class),
            body: lower_fn_body(checked, ctx, body, super_class),
            is_async,
            is_generator,
        });
    }
    for (fn_id, params, body) in private_accessor_meta {
        private_method_fns.push(Stmt::Function {
            local: fn_id,
            params: lower_params(checked, ctx, params, super_class),
            body: lower_fn_body(checked, ctx, body, super_class),
            is_async: false,
            is_generator: false,
        });
    }

    let mut ctor_body = match ctor_body_ast {
        Some(body) => lower_fn_body(checked, ctx, body, super_class),
        None => Vec::new(),
    };

    if !instance_fields.is_empty() {
        let field_inits: Vec<Stmt> = instance_fields
            .iter()
            .map(|(fname, value, is_private)| {
                let init = match value {
                    Some(v) => lower_expr(checked, ctx, v, super_class),
                    None => Expr::IdentName {
                        name: "undefined".into(),
                        ty: Type::Any,
                    },
                };
                if *is_private {
                    let wm = *ctx
                        .private_fields
                        .get(&fname.name)
                        .expect("private field WeakMap");
                    // wm.set(this, init)
                    Stmt::Expr {
                        expr: Expr::Call {
                            callee: Box::new(Expr::Member {
                                object: Box::new(Expr::Local {
                                    id: wm,
                                    ty: Type::Any,
                                }),
                                property: Box::new(Expr::String {
                                    value: "set".into(),
                                    ty: Type::String,
                                }),
                                computed: false,
                                optional: false,
                                ty: Type::Function,
                            }),
                            args: vec![
                                Arg::Expr(Expr::This { ty: Type::Any }),
                                Arg::Expr(init),
                            ],
                            optional: false,
                            ty: Type::Any,
                        },
                    }
                } else {
                    Stmt::Expr {
                        expr: Expr::Assign {
                            target: AssignTarget::Member {
                                object: Box::new(Expr::This { ty: Type::Any }),
                                property: Box::new(Expr::String {
                                    value: fname.name.clone().into(),
                                    ty: Type::String,
                                }),
                                computed: false,
                            },
                            op: AssignOp::Eq,
                            value: Box::new(init),
                            ty: Type::Any,
                        },
                    }
                }
            })
            .collect();
        // After `super(...)` when present (first body stmt), else at start of ctor.
        let insert_at = if super_class.is_some() && !ctor_body.is_empty() {
            1
        } else {
            0
        };
        let mut new_body = Vec::with_capacity(ctor_body.len() + field_inits.len());
        new_body.extend(ctor_body.drain(..insert_at.min(ctor_body.len())));
        new_body.extend(field_inits);
        new_body.extend(ctor_body);
        ctor_body = new_body;
    }

    // Brand instances for private methods/accessors (E18.40).
    if !instance_brands.is_empty() {
        let brand_inits: Vec<Stmt> = instance_brands
            .iter()
            .map(|brand| private_brand_add(*brand, Expr::This { ty: Type::Any }))
            .collect();
        let insert_at = if super_class.is_some() && !ctor_body.is_empty() {
            // After super; field inits already after super when present.
            1
        } else {
            0
        };
        // Prefer after field inits: find end of leading brand/field region is complex;
        // append brand adds right after super (or start), then fields already shifted.
        // Install brands at the same insert point as fields would use when fields empty,
        // or immediately after whatever was inserted for fields.
        let mut new_body = Vec::with_capacity(ctor_body.len() + brand_inits.len());
        // If we already inserted fields after super, brands should also run after super.
        // Use insert_at but if fields were inserted, brands should be after fields.
        // Simpler: always insert brands just after super (index 1) or at 0, before fields
        // is wrong for fields-only branding via WeakMap — methods need brand on this.
        // Order: super, fields (wm.set), brands (ws.add), rest — fields already in place.
        // Find first non-field-init is hard; append brands after all field inits by
        // inserting at insert_at + field count. Track field count from instance_fields.
        let after_fields = if !instance_fields.is_empty() {
            let n_fields = instance_fields.len();
            insert_at + n_fields
        } else {
            insert_at
        };
        new_body.extend(ctor_body.drain(..after_fields.min(ctor_body.len())));
        new_body.extend(brand_inits);
        new_body.extend(ctor_body);
        ctor_body = new_body;
    }

    let mut out = private_wm_decls;
    out.extend(private_brand_decls);
    out.extend(private_method_fns);
    out.push(Stmt::Function {
        local,
        params: ctor_params,
        body: ctor_body,
        is_async: false,
        is_generator: false,
    });

    for (method_name, params, body, is_static, is_async, is_generator, is_private) in methods {
        if is_private {
            // Already emitted as standalone function; not installed on prototype.
            continue;
        }
        let method_fn = Expr::Function {
            name: None,
            params: lower_params(checked, ctx, params, super_class),
            body: lower_fn_body(checked, ctx, body, super_class),
            is_async,
            is_generator,
            is_arrow: false,
            ty: Type::Function,
        };
        let class_ref = Expr::Local {
            id: local,
            ty: Type::Function,
        };
        let target_object = if is_static {
            class_ref
        } else {
            Expr::Member {
                object: Box::new(class_ref),
                property: Box::new(Expr::String {
                    value: "prototype".into(),
                    ty: Type::String,
                }),
                computed: false,
                optional: false,
                ty: Type::Any,
            }
        };
        out.push(Stmt::Expr {
            expr: Expr::Assign {
                target: AssignTarget::Member {
                    object: Box::new(target_object),
                    property: Box::new(Expr::String {
                        value: method_name.name.clone().into(),
                        ty: Type::String,
                    }),
                    computed: false,
                },
                op: AssignOp::Eq,
                value: Box::new(method_fn),
                ty: Type::Function,
            },
        });
    }

    for (kind, acc_name, params, body, is_static, is_private) in accessors {
        if is_private {
            // Already emitted as standalone function; not installed on prototype.
            continue;
        }
        let accessor_fn = Expr::Function {
            name: None,
            params: lower_params(checked, ctx, params, super_class),
            body: lower_fn_body(checked, ctx, body, super_class),
            is_async: false,
            is_generator: false,
            is_arrow: false,
            ty: Type::Function,
        };
        let class_ref = Expr::Local {
            id: local,
            ty: Type::Function,
        };
        let target_object = if is_static {
            class_ref
        } else {
            Expr::Member {
                object: Box::new(class_ref),
                property: Box::new(Expr::String {
                    value: "prototype".into(),
                    ty: Type::String,
                }),
                computed: false,
                optional: false,
                ty: Type::Any,
            }
        };
        let kind_key = match kind {
            AccessorKind::Get => "get",
            AccessorKind::Set => "set",
        };
        // Object.defineProperty(target, name, { get|set: fn, configurable: true, enumerable: false })
        let desc = Expr::Object {
            properties: vec![
                ObjectProp::Property {
                    key: ObjectPropKey::Static(kind_key.into()),
                    value: accessor_fn,
                },
                ObjectProp::Property {
                    key: ObjectPropKey::Static("configurable".into()),
                    value: Expr::Boolean {
                        value: true,
                        ty: Type::Boolean,
                    },
                },
                ObjectProp::Property {
                    key: ObjectPropKey::Static("enumerable".into()),
                    value: Expr::Boolean {
                        value: false,
                        ty: Type::Boolean,
                    },
                },
            ],
            ty: Type::Object,
        };
        out.push(Stmt::Expr {
            expr: Expr::Call {
                callee: Box::new(Expr::Member {
                    object: Box::new(Expr::IdentName {
                        name: "Object".into(),
                        ty: Type::Object,
                    }),
                    property: Box::new(Expr::String {
                        value: "defineProperty".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Function,
                }),
                args: vec![
                    Arg::Expr(target_object),
                    Arg::Expr(Expr::String {
                        value: acc_name.name.clone().into(),
                        ty: Type::String,
                    }),
                    Arg::Expr(desc),
                ],
                optional: false,
                ty: Type::Any,
            },
        });
    }

    if let Some(sc) = super_class {
        // Child.prototype.__proto__ = Parent.prototype
        let parent = lower_expr(checked, ctx, sc, None);
        let parent_proto = Expr::Member {
            object: Box::new(parent.clone()),
            property: Box::new(Expr::String {
                value: "prototype".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Any,
        };
        let child_proto = Expr::Member {
            object: Box::new(Expr::Local {
                id: local,
                ty: Type::Function,
            }),
            property: Box::new(Expr::String {
                value: "prototype".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Any,
        };
        out.push(Stmt::Expr {
            expr: Expr::Assign {
                target: AssignTarget::Member {
                    object: Box::new(child_proto),
                    property: Box::new(Expr::String {
                        value: "__proto__".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                },
                op: AssignOp::Eq,
                value: Box::new(parent_proto),
                ty: Type::Any,
            },
        });
        // Child.__proto__ = Parent (static inheritance / instanceof chain helpers)
        out.push(Stmt::Expr {
            expr: Expr::Assign {
                target: AssignTarget::Member {
                    object: Box::new(Expr::Local {
                        id: local,
                        ty: Type::Function,
                    }),
                    property: Box::new(Expr::String {
                        value: "__proto__".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                },
                op: AssignOp::Eq,
                value: Box::new(parent),
                ty: Type::Any,
            },
        });
    }

    // Brand the constructor for static private methods/accessors (E18.40)
    // before static field/block evaluation so blocks can use private statics.
    for brand in static_brands {
        out.push(private_brand_add(
            brand,
            Expr::Local {
                id: local,
                ty: Type::Function,
            },
        ));
    }

    // Static fields and static blocks run after the class is fully linked, in order (E18.41).
    for init in static_inits {
        match init {
            StaticInit::Field(fname, value, is_private) => {
                let init_expr = match value {
                    Some(v) => lower_expr(checked, ctx, v, None),
                    None => Expr::IdentName {
                        name: "undefined".into(),
                        ty: Type::Any,
                    },
                };
                if is_private {
                    let wm = *ctx
                        .private_fields
                        .get(&fname.name)
                        .expect("static private field WeakMap");
                    // wm.set(Class, init)
                    out.push(Stmt::Expr {
                        expr: Expr::Call {
                            callee: Box::new(Expr::Member {
                                object: Box::new(Expr::Local {
                                    id: wm,
                                    ty: Type::Any,
                                }),
                                property: Box::new(Expr::String {
                                    value: "set".into(),
                                    ty: Type::String,
                                }),
                                computed: false,
                                optional: false,
                                ty: Type::Function,
                            }),
                            args: vec![
                                Arg::Expr(Expr::Local {
                                    id: local,
                                    ty: Type::Function,
                                }),
                                Arg::Expr(init_expr),
                            ],
                            optional: false,
                            ty: Type::Any,
                        },
                    });
                } else {
                    out.push(Stmt::Expr {
                        expr: Expr::Assign {
                            target: AssignTarget::Member {
                                object: Box::new(Expr::Local {
                                    id: local,
                                    ty: Type::Function,
                                }),
                                property: Box::new(Expr::String {
                                    value: fname.name.clone().into(),
                                    ty: Type::String,
                                }),
                                computed: false,
                            },
                            op: AssignOp::Eq,
                            value: Box::new(init_expr),
                            ty: Type::Any,
                        },
                    });
                }
            }
            StaticInit::Block(body) => {
                // (function() { … }).call(Class) so `this` is the constructor.
                let block_body = lower_fn_body(checked, ctx, body, None);
                out.push(Stmt::Expr {
                    expr: Expr::Call {
                        callee: Box::new(Expr::Member {
                            object: Box::new(Expr::Function {
                                name: None,
                                params: Vec::new(),
                                body: block_body,
                                is_async: false,
                                is_generator: false,
                                is_arrow: false,
                                ty: Type::Function,
                            }),
                            property: Box::new(Expr::String {
                                value: "call".into(),
                                ty: Type::String,
                            }),
                            computed: false,
                            optional: false,
                            ty: Type::Function,
                        }),
                        args: vec![Arg::Expr(Expr::Local {
                            id: local,
                            ty: Type::Function,
                        })],
                        optional: false,
                        ty: Type::Any,
                    },
                });
            }
        }
    }

    ctx.private_fields = prev_privates;
    ctx.private_methods = prev_private_methods;
    ctx.private_accessors = prev_private_accessors;
    ctx.private_brands = prev_private_brands;
    out
}

/// `fn.call(object, ...args)` for private method/accessor invocation.
fn private_fn_call(fn_id: LocalId, object: Expr, args: Vec<Arg>) -> Expr {
    let call_member = Expr::Member {
        object: Box::new(Expr::Local {
            id: fn_id,
            ty: Type::Function,
        }),
        property: Box::new(Expr::String {
            value: "call".into(),
            ty: Type::String,
        }),
        computed: false,
        optional: false,
        ty: Type::Function,
    };
    let mut call_args = Vec::with_capacity(args.len() + 1);
    call_args.push(Arg::Expr(object));
    call_args.extend(args);
    Expr::Call {
        callee: Box::new(call_member),
        args: call_args,
        optional: false,
        ty: Type::Any,
    }
}

/// `brand.add(object)` statement for private method/accessor branding (E18.40).
fn private_brand_add(brand: LocalId, object: Expr) -> Stmt {
    Stmt::Expr {
        expr: Expr::Call {
            callee: Box::new(Expr::Member {
                object: Box::new(Expr::Local {
                    id: brand,
                    ty: Type::Any,
                }),
                property: Box::new(Expr::String {
                    value: "add".into(),
                    ty: Type::String,
                }),
                computed: false,
                optional: false,
                ty: Type::Function,
            }),
            args: vec![Arg::Expr(object)],
            optional: false,
            ty: Type::Any,
        },
    }
}

/// `#name in object` → object is object-like and brand/WeakMap has it (E18.40).
fn private_in_check(brand: LocalId, object: Expr) -> Expr {
    // `obj != null && (typeof obj === "object" || typeof obj === "function") && brand.has(obj)`
    let not_nullish = Expr::Binary {
        left: Box::new(object.clone()),
        op: BinaryOp::NotEq,
        right: Box::new(Expr::Null { ty: Type::Null }),
        ty: Type::Boolean,
    };
    let typeof_obj = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(object.clone()),
        ty: Type::String,
    };
    let is_object = Expr::Binary {
        left: Box::new(typeof_obj),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::String {
            value: "object".into(),
            ty: Type::String,
        }),
        ty: Type::Boolean,
    };
    let typeof_fn = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(object.clone()),
        ty: Type::String,
    };
    let is_function = Expr::Binary {
        left: Box::new(typeof_fn),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::String {
            value: "function".into(),
            ty: Type::String,
        }),
        ty: Type::Boolean,
    };
    let is_obj_like = Expr::Binary {
        left: Box::new(is_object),
        op: BinaryOp::Or,
        right: Box::new(is_function),
        ty: Type::Boolean,
    };
    let guard = Expr::Binary {
        left: Box::new(not_nullish),
        op: BinaryOp::And,
        right: Box::new(is_obj_like),
        ty: Type::Boolean,
    };
    let has_call = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(Expr::Local {
                id: brand,
                ty: Type::Any,
            }),
            property: Box::new(Expr::String {
                value: "has".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(object)],
        optional: false,
        ty: Type::Boolean,
    };
    Expr::Binary {
        left: Box::new(guard),
        op: BinaryOp::And,
        right: Box::new(has_call),
        ty: Type::Boolean,
    }
}


fn ensure_private_brand(
    ctx: &mut LowerCtx,
    class_local: LocalId,
    private_brand_map: &mut HashMap<String, LocalId>,
    private_brand_decls: &mut Vec<Stmt>,
    instance_brands: &mut Vec<LocalId>,
    static_brands: &mut Vec<LocalId>,
    name: &str,
    is_static: bool,
) {
    if let Some(existing) = private_brand_map.get(name) {
        if is_static {
            if !static_brands.contains(existing) {
                static_brands.push(*existing);
            }
        } else if !instance_brands.contains(existing) {
            instance_brands.push(*existing);
        }
        return;
    }
    let brand_name = format!("__drac_pb_{}_{}", class_local.0, name);
    let brand_id = ctx.alloc_synthetic_local(brand_name, Type::Any);
    private_brand_map.insert(name.to_string(), brand_id);
    private_brand_decls.push(Stmt::Declare {
        local: brand_id,
        init: Some(Expr::New {
            callee: Box::new(Expr::IdentName {
                name: "WeakSet".into(),
                ty: Type::Function,
            }),
            args: Vec::new(),
            ty: Type::Any,
        }),
        kind: BindingKind::Let,
    });
    if is_static {
        static_brands.push(brand_id);
    } else {
        instance_brands.push(brand_id);
    }
}

fn resolve_private_brand(ctx: &LowerCtx, name: &str) -> LocalId {
    if let Some(wm) = ctx.private_fields.get(name).copied() {
        return wm;
    }
    if let Some(brand) = ctx.private_brands.get(name).copied() {
        return brand;
    }
    panic!("unknown private brand #{name}");
}

/// `wm.get(object)` for private field read.
fn private_field_get(wm: LocalId, object: Expr) -> Expr {
    Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(Expr::Local {
                id: wm,
                ty: Type::Any,
            }),
            property: Box::new(Expr::String {
                value: "get".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(object)],
        optional: false,
        ty: Type::Any,
    }
}

/// `(wm.set(object, value), value)` so assignment yields the RHS.
fn private_field_set(wm: LocalId, object: Expr, value: Expr) -> Expr {
    let set_call = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(Expr::Local {
                id: wm,
                ty: Type::Any,
            }),
            property: Box::new(Expr::String {
                value: "set".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(object), Arg::Expr(value.clone())],
        optional: false,
        ty: Type::Any,
    };
    Expr::Binary {
        left: Box::new(set_call),
        op: BinaryOp::Comma,
        right: Box::new(value),
        ty: Type::Any,
    }
}

fn lower_arg(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    arg: &AstArg,
    super_class: Option<&AstExpr>,
) -> Arg {
    match arg {
        AstArg::Expr(e) => Arg::Expr(lower_expr(checked, ctx, e, super_class)),
        AstArg::Spread(e) => Arg::Spread(lower_expr(checked, ctx, e, super_class)),
    }
}

fn lower_expr(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    expr: &AstExpr,
    super_class: Option<&AstExpr>,
) -> Expr {
    match expr {
        AstExpr::Paren { expr: inner, .. } => lower_expr(checked, ctx, inner, super_class),
        // Dual-worlds `as` is a type-level boundary only (T06); erase at IR.
        AstExpr::As { expr: inner, .. } => lower_expr(checked, ctx, inner, super_class),
        AstExpr::ArrayPattern { .. } => {
            panic!("array pattern must only appear as assignment target")
        }
        AstExpr::ObjectPattern { .. } => {
            panic!("object pattern must only appear as assignment target")
        }
        AstExpr::Ident(id) => {
            let ty = expr_ty(checked, id.span);
            if let Some(sym) = checked.bound.resolve(id.span) {
                Expr::Local { id: sym, ty }
            } else {
                Expr::IdentName {
                    name: id.name.clone(),
                    ty,
                }
            }
        }
        AstExpr::Number(n) => Expr::Number {
            raw: n.raw.clone(),
            ty: expr_ty(checked, n.span),
        },
        AstExpr::BigInt(n) => Expr::BigInt {
            raw: n.raw.clone(),
            ty: expr_ty(checked, n.span),
        },
        AstExpr::String(s) => Expr::String {
            value: s.value.clone(),
            ty: expr_ty(checked, s.span),
        },
        AstExpr::RegExp {
            pattern,
            flags,
            span,
        } => Expr::RegExp {
            pattern: pattern.clone(),
            flags: flags.clone(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::TemplateLiteral {
            quasis,
            expressions,
            span,
        } => Expr::Template {
            quasis: quasis.iter().map(|q| q.cooked.clone()).collect(),
            expressions: expressions
                .iter()
                .map(|e| lower_expr(checked, ctx, e, super_class))
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            span,
        } => Expr::TaggedTemplate {
            tag: Box::new(lower_expr(checked, ctx, tag, super_class)),
            quasis: quasis.iter().map(|q| q.cooked.clone()).collect(),
            expressions: expressions
                .iter()
                .map(|e| lower_expr(checked, ctx, e, super_class))
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Boolean { value, span } => Expr::Boolean {
            value: *value,
            ty: expr_ty(checked, *span),
        },
        AstExpr::Null { span } => Expr::Null {
            ty: expr_ty(checked, *span),
        },
        AstExpr::This { span } => Expr::This {
            ty: expr_ty(checked, *span),
        },
        AstExpr::NewTarget { span } => Expr::NewTarget {
            ty: expr_ty(checked, *span),
        },
        AstExpr::Super { .. } => {
            panic!("bare `super` must appear as super(...) or super.prop after check")
        }
        AstExpr::Unary { op, arg, span } => Expr::Unary {
            op: *op,
            arg: Box::new(lower_expr(checked, ctx, arg, super_class)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Binary {
            left,
            op,
            right,
            span,
        } => Expr::Binary {
            left: Box::new(lower_expr(checked, ctx, left, super_class)),
            op: *op,
            right: Box::new(lower_expr(checked, ctx, right, super_class)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::PrivateIn { name, object, span } => {
            let brand = resolve_private_brand(ctx, &name.name);
            let obj = lower_expr(checked, ctx, object, super_class);
            let _ = span;
            private_in_check(brand, obj)
        }
        AstExpr::Conditional {
            test,
            consequent,
            alternate,
            span,
        } => Expr::Conditional {
            test: Box::new(lower_expr(checked, ctx, test, super_class)),
            consequent: Box::new(lower_expr(checked, ctx, consequent, super_class)),
            alternate: Box::new(lower_expr(checked, ctx, alternate, super_class)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Assign {
            target,
            op,
            value,
            span,
        } => {
            // Private field/accessor assign: `obj.#x = v` (simple `=` only).
            if let AstExpr::MemberExpression {
                object,
                property,
                private: true,
                ..
            } = target.as_ref()
            {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.clone(),
                    _ => panic!("private member property must be ident"),
                };
                assert!(
                    matches!(op, AssignOp::Eq),
                    "only simple `=` supported on private fields/accessors"
                );
                let obj = lower_expr(checked, ctx, object, super_class);
                let rhs = lower_expr(checked, ctx, value, super_class);
                if let Some(set_id) = ctx
                    .private_accessors
                    .get(&fname)
                    .and_then(|(_, set)| *set)
                {
                    // `(setter.call(obj, v), v)`
                    let set_call =
                        private_fn_call(set_id, obj, vec![Arg::Expr(rhs.clone())]);
                    return Expr::Binary {
                        left: Box::new(set_call),
                        op: BinaryOp::Comma,
                        right: Box::new(rhs),
                        ty: Type::Any,
                    };
                }
                let wm = ctx
                    .private_fields
                    .get(&fname)
                    .copied()
                    .unwrap_or_else(|| panic!("unknown private field #{fname}"));
                return private_field_set(wm, obj, rhs);
            }
            let target = match target.as_ref() {
                AstExpr::Ident(id) => {
                    if let Some(local) = checked.bound.resolve(id.span) {
                        AssignTarget::Local(local)
                    } else {
                        AssignTarget::Name(id.name.clone())
                    }
                }
                AstExpr::MemberExpression {
                    object,
                    property,
                    computed,
                    private: false,
                    ..
                } => {
                    let property = if *computed {
                        lower_expr(checked, ctx, property, super_class)
                    } else {
                        match property.as_ref() {
                            AstExpr::Ident(id) => Expr::String {
                                value: id.name.clone().into(),
                                ty: Type::String,
                            },
                            other => lower_expr(checked, ctx, other, super_class),
                        }
                    };
                    AssignTarget::Member {
                        object: Box::new(lower_expr(checked, ctx, object, super_class)),
                        property: Box::new(property),
                        computed: *computed,
                    }
                }
                AstExpr::Unary {
                    op: UnaryOp::Deref,
                    arg,
                    ..
                } => AssignTarget::Deref(Box::new(lower_expr(checked, ctx, arg, super_class))),
                AstExpr::ArrayPattern { elements, .. } => AssignTarget::ArrayPattern {
                    elements: lower_array_pattern_els(checked, ctx, elements),
                },
                AstExpr::ObjectPattern { properties, .. } => AssignTarget::ObjectPattern {
                    properties: lower_object_pattern_props(checked, ctx, properties),
                },
                _ => panic!(
                    "assign target must be ident, member, deref, array pattern, or object pattern after check"
                ),
            };
            Expr::Assign {
                target,
                op: *op,
                value: Box::new(lower_expr(checked, ctx, value, super_class)),
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::Update {
            op,
            arg,
            prefix,
            span,
        } => {
            let target = match arg.as_ref() {
                AstExpr::Ident(id) => {
                    if let Some(local) = checked.bound.resolve(id.span) {
                        UpdateTarget::Local(local)
                    } else {
                        UpdateTarget::Name(id.name.clone())
                    }
                }
                AstExpr::MemberExpression {
                    object,
                    property,
                    computed,
                    private: false,
                    ..
                } => {
                    let property = if *computed {
                        lower_expr(checked, ctx, property, super_class)
                    } else {
                        match property.as_ref() {
                            AstExpr::Ident(id) => Expr::String {
                                value: id.name.clone().into(),
                                ty: Type::String,
                            },
                            other => lower_expr(checked, ctx, other, super_class),
                        }
                    };
                    UpdateTarget::Member {
                        object: Box::new(lower_expr(checked, ctx, object, super_class)),
                        property: Box::new(property),
                        computed: *computed,
                    }
                }
                _ => panic!("update target must be ident or member after check"),
            };
            Expr::Update {
                op: *op,
                target,
                prefix: *prefix,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::Call {
            callee,
            args,
            optional,
            span,
        } => {
            // `super(args)` → `Parent.call(this, ...args)`
            if matches!(callee.as_ref(), AstExpr::Super { .. }) {
                let parent_ast = super_class
                    .expect("`super(...)` requires `extends` on the enclosing class");
                let parent = lower_expr(checked, ctx, parent_ast, None);
                let call_member = Expr::Member {
                    object: Box::new(parent),
                    property: Box::new(Expr::String {
                        value: "call".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Function,
                };
                let mut call_args = Vec::with_capacity(args.len() + 1);
                call_args.push(Arg::Expr(Expr::This { ty: Type::Any }));
                for a in args {
                    call_args.push(lower_arg(checked, ctx, a, super_class));
                }
                return Expr::Call {
                    callee: Box::new(call_member),
                    args: call_args,
                    optional: false,
                    ty: expr_ty(checked, *span),
                };
            }
            // `super.m(args)` → `Parent.prototype.m.call(this, ...args)`
            if let AstExpr::MemberExpression {
                object,
                property,
                computed,
                private,
                ..
            } = callee.as_ref()
            {
                if matches!(object.as_ref(), AstExpr::Super { .. }) {
                    let parent_ast = super_class
                        .expect("`super.prop` requires `extends` on the enclosing class");
                    let parent = lower_expr(checked, ctx, parent_ast, None);
                    let parent_proto = Expr::Member {
                        object: Box::new(parent),
                        property: Box::new(Expr::String {
                            value: "prototype".into(),
                            ty: Type::String,
                        }),
                        computed: false,
                        optional: false,
                        ty: Type::Any,
                    };
                    let prop = if *computed {
                        lower_expr(checked, ctx, property, super_class)
                    } else {
                        match property.as_ref() {
                            AstExpr::Ident(id) => Expr::String {
                                value: id.name.clone().into(),
                                ty: Type::String,
                            },
                            other => lower_expr(checked, ctx, other, super_class),
                        }
                    };
                    let method = Expr::Member {
                        object: Box::new(parent_proto),
                        property: Box::new(prop),
                        computed: *computed,
                        optional: false,
                        ty: Type::Function,
                    };
                    let call_member = Expr::Member {
                        object: Box::new(method),
                        property: Box::new(Expr::String {
                            value: "call".into(),
                            ty: Type::String,
                        }),
                        computed: false,
                        optional: false,
                        ty: Type::Function,
                    };
                    let mut call_args = Vec::with_capacity(args.len() + 1);
                    call_args.push(Arg::Expr(Expr::This { ty: Type::Any }));
                    for a in args {
                        call_args.push(lower_arg(checked, ctx, a, super_class));
                    }
                    return Expr::Call {
                        callee: Box::new(call_member),
                        args: call_args,
                        optional: false,
                        ty: expr_ty(checked, *span),
                    };
                }
                // `obj.#m(args)` → `__drac_pm_m.call(obj, ...args)` (E18.37)
                if *private {
                    let fname = match property.as_ref() {
                        AstExpr::Ident(id) => id.name.clone(),
                        _ => panic!("private member property must be ident"),
                    };
                    if let Some(fn_id) = ctx.private_methods.get(&fname).copied() {
                        let call_member = Expr::Member {
                            object: Box::new(Expr::Local {
                                id: fn_id,
                                ty: Type::Function,
                            }),
                            property: Box::new(Expr::String {
                                value: "call".into(),
                                ty: Type::String,
                            }),
                            computed: false,
                            optional: false,
                            ty: Type::Function,
                        };
                        let mut call_args = Vec::with_capacity(args.len() + 1);
                        call_args.push(Arg::Expr(lower_expr(checked, ctx, object, super_class)));
                        for a in args {
                            call_args.push(lower_arg(checked, ctx, a, super_class));
                        }
                        return Expr::Call {
                            callee: Box::new(call_member),
                            args: call_args,
                            optional: false,
                            ty: expr_ty(checked, *span),
                        };
                    }
                }
            }
            Expr::Call {
                callee: Box::new(lower_expr(checked, ctx, callee, super_class)),
                args: args
                    .iter()
                    .map(|a| lower_arg(checked, ctx, a, super_class))
                    .collect(),
                optional: *optional,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::New {
            callee,
            args,
            span,
        } => Expr::New {
            callee: Box::new(lower_expr(checked, ctx, callee, super_class)),
            args: args
                .iter()
                .map(|a| lower_arg(checked, ctx, a, super_class))
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::FunctionExpression {
            name,
            params,
            body,
            is_async,
            is_generator,
            span,
            ..
        } => {
            let name = name.as_ref().map(|n| {
                checked
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == n.span)
                    .map(|s| s.id)
                    .expect("function expression name must be declared")
            });
            let params = lower_params(checked, ctx, params, None);
            let body = lower_fn_body(checked, ctx, body, None);
            Expr::Function {
                name,
                params,
                body,
                is_async: *is_async,
                is_generator: *is_generator,
                is_arrow: false,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::ClassExpression {
            name,
            super_class: sc,
            body,
            span,
        } => lower_class_expression(checked, ctx, name.as_ref(), sc.as_deref(), body, *span),
        AstExpr::ArrowFunction {
            params,
            body,
            is_async,
            span,
            ..
        } => {
            let params = lower_params(checked, ctx, params, None);
            let body = match body {
                draconic_ast::ArrowBody::Block(stmt) => lower_fn_body(checked, ctx, stmt, None),
                draconic_ast::ArrowBody::Expr(expr) => {
                    vec![Stmt::Return {
                        value: Some(lower_expr(checked, ctx, expr, None)),
                    }]
                }
            };
            Expr::Function {
                name: None,
                params,
                body,
                is_async: *is_async,
                is_generator: false,
                is_arrow: true,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::ObjectExpression { properties, span } => Expr::Object {
            properties: properties
                .iter()
                .map(|p| match p {
                    AstObjectProp::Property { key, value, .. } => ObjectProp::Property {
                        key: match key {
                            draconic_ast::ObjectKey::Ident(id) => {
                                ObjectPropKey::Static(id.name.clone().into())
                            }
                            draconic_ast::ObjectKey::String(s) => {
                                ObjectPropKey::Static(s.value.clone())
                            }
                            draconic_ast::ObjectKey::Computed(expr) => {
                                ObjectPropKey::Computed(lower_expr(checked, ctx, expr, super_class))
                            }
                        },
                        value: lower_expr(checked, ctx, value, super_class),
                    },
                    AstObjectProp::Accessor {
                        kind,
                        key,
                        params,
                        body,
                        ..
                    } => {
                        let key = match key {
                            draconic_ast::ObjectKey::Ident(id) => {
                                ObjectPropKey::Static(id.name.clone().into())
                            }
                            draconic_ast::ObjectKey::String(s) => {
                                ObjectPropKey::Static(s.value.clone())
                            }
                            draconic_ast::ObjectKey::Computed(expr) => {
                                ObjectPropKey::Computed(lower_expr(checked, ctx, expr, super_class))
                            }
                        };
                        ObjectProp::Accessor {
                            kind: *kind,
                            key,
                            value: Expr::Function {
                                name: None,
                                params: lower_params(checked, ctx, params, super_class),
                                body: lower_fn_body(checked, ctx, body, super_class),
                                is_async: false,
                                is_generator: false,
                                is_arrow: false,
                                ty: Type::Function,
                            },
                        }
                    }
                    AstObjectProp::Spread { expr, .. } => {
                        ObjectProp::Spread(lower_expr(checked, ctx, expr, super_class))
                    }
                })
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::ArrayExpression { elements, span } => Expr::Array {
            elements: elements
                .iter()
                .map(|el| match el {
                    AstArrayElement::Expr(e) => {
                        ArrayElement::Expr(lower_expr(checked, ctx, e, super_class))
                    }
                    AstArrayElement::Spread(e) => {
                        ArrayElement::Spread(lower_expr(checked, ctx, e, super_class))
                    }
                    AstArrayElement::Elision => ArrayElement::Elision,
                })
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::MemberExpression {
            object,
            property,
            computed,
            optional,
            private,
            span,
        } => {
            if *private {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.clone(),
                    _ => panic!("private member property must be ident"),
                };
                if let Some(fn_id) = ctx.private_methods.get(&fname).copied() {
                    // Private method as value (unbound function).
                    let _ = object;
                    return Expr::Local {
                        id: fn_id,
                        ty: Type::Function,
                    };
                }
                let obj = lower_expr(checked, ctx, object, super_class);
                if let Some(get_id) = ctx
                    .private_accessors
                    .get(&fname)
                    .and_then(|(get, _)| *get)
                {
                    // Private getter: `getter.call(obj)`
                    return private_fn_call(get_id, obj, Vec::new());
                }
                let wm = ctx
                    .private_fields
                    .get(&fname)
                    .copied()
                    .unwrap_or_else(|| panic!("unknown private field #{fname}"));
                return private_field_get(wm, obj);
            }
            // `super.prop` → `Parent.prototype.prop`
            if matches!(object.as_ref(), AstExpr::Super { .. }) {
                let parent_ast = super_class
                    .expect("`super.prop` requires `extends` on the enclosing class");
                let parent = lower_expr(checked, ctx, parent_ast, None);
                let parent_proto = Expr::Member {
                    object: Box::new(parent),
                    property: Box::new(Expr::String {
                        value: "prototype".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Any,
                };
                let property = if *computed {
                    lower_expr(checked, ctx, property, super_class)
                } else {
                    match property.as_ref() {
                        AstExpr::Ident(id) => Expr::String {
                            value: id.name.clone().into(),
                            ty: Type::String,
                        },
                        other => lower_expr(checked, ctx, other, super_class),
                    }
                };
                return Expr::Member {
                    object: Box::new(parent_proto),
                    property: Box::new(property),
                    computed: *computed,
                    optional: false,
                    ty: expr_ty(checked, *span),
                };
            }
            let property = if *computed {
                lower_expr(checked, ctx, property, super_class)
            } else {
                match property.as_ref() {
                    AstExpr::Ident(id) => Expr::String {
                        value: id.name.clone().into(),
                        ty: Type::String,
                    },
                    other => lower_expr(checked, ctx, other, super_class),
                }
            };
            Expr::Member {
                object: Box::new(lower_expr(checked, ctx, object, super_class)),
                property: Box::new(property),
                computed: *computed,
                optional: *optional,
                ty: expr_ty(checked, *span),
            }
        }
    }
}

fn lower_params(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    params: &[draconic_ast::Param],
    super_class: Option<&AstExpr>,
) -> Vec<Param> {
    let mut out = Vec::with_capacity(params.len());
    for p in params {
        out.push(Param {
            pattern: lower_binding_pattern(checked, ctx, &p.binding),
            default: p
                .default
                .as_ref()
                .map(|e| lower_expr(checked, ctx, e, super_class)),
            rest: p.rest,
        });
    }
    out
}

fn expr_ty(checked: &CheckedProgram, span: Span) -> Type {
    checked
        .type_of_expr(span)
        .expect("checked expression must have a type")
}

fn lower_array_pattern_els(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    elements: &[ArrayPatternElement],
) -> Vec<ArrayPatternEl> {
    let mut out = Vec::with_capacity(elements.len());
    for el in elements {
        out.push(match el {
            ArrayPatternElement::Elision => ArrayPatternEl::Elision,
            ArrayPatternElement::Pattern { binding, default } => ArrayPatternEl::Pattern {
                binding: lower_binding_pattern(checked, ctx, binding),
                default: default
                    .as_ref()
                    .map(|d| lower_expr(checked, ctx, d, None)),
            },
            ArrayPatternElement::Rest(binding) => {
                ArrayPatternEl::Rest(lower_binding_pattern(checked, ctx, binding))
            }
        });
    }
    out
}

fn lower_object_pattern_props(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    properties: &[ObjectPatternProp],
) -> Vec<ObjectPatternEl> {
    let mut out = Vec::with_capacity(properties.len());
    for p in properties {
        out.push(match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                shorthand,
                default,
                ..
            } => ObjectPatternEl::Prop {
                key: key.name.clone(),
                binding: lower_binding_pattern(checked, ctx, binding),
                shorthand: *shorthand,
                default: default
                    .as_ref()
                    .map(|d| lower_expr(checked, ctx, d, None)),
            },
            ObjectPatternProp::Rest(binding) => {
                ObjectPatternEl::Rest(lower_binding_pattern(checked, ctx, binding))
            }
        });
    }
    out
}

fn lower_binding_pattern(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    pat: &BindingPattern,
) -> Pattern {
    match pat {
        BindingPattern::Ident(id) => {
            if let Some(local) = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.span == id.span)
                .map(|s| s.id)
                .or_else(|| checked.bound.resolve(id.span))
            {
                Pattern::Local(local)
            } else {
                Pattern::Name(id.name.clone())
            }
        }
        BindingPattern::Member(expr) => match expr.as_ref() {
            AstExpr::MemberExpression {
                object,
                property,
                computed,
                private: false,
                ..
            } => {
                let property = if *computed {
                    lower_expr(checked, ctx, property, None)
                } else {
                    match property.as_ref() {
                        AstExpr::Ident(id) => Expr::String {
                            value: id.name.clone().into(),
                            ty: Type::String,
                        },
                        other => lower_expr(checked, ctx, other, None),
                    }
                };
                Pattern::Member {
                    object: Box::new(lower_expr(checked, ctx, object, None)),
                    property: Box::new(property),
                    computed: *computed,
                }
            }
            _ => panic!("BindingPattern::Member must wrap MemberExpression"),
        },
        BindingPattern::Array { elements, .. } => {
            Pattern::Array(lower_array_pattern_els(checked, ctx, elements))
        }
        BindingPattern::Object { properties, .. } => {
            Pattern::Object(lower_object_pattern_props(checked, ctx, properties))
        }
    }
}

/// Stable, indentation-based IR dump for snapshots and debugging.
pub fn dump_module(module: &Module) -> String {
    let mut out = String::new();
    out.push_str("Module\n");
    if !module.locals.is_empty() {
        out.push_str("  locals:\n");
        for local in &module.locals {
            out.push_str(&format!(
                "    %{} {}: {}\n",
                local.id.0, local.name, local.ty
            ));
        }
    }
    out.push_str("  body:\n");
    for stmt in &module.body {
        dump_stmt(stmt, 2, &mut out);
    }
    out
}

fn indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn dump_assign_target(target: &AssignTarget, level: usize, out: &mut String) {
    match target {
        AssignTarget::Local(id) => {
            indent(level, out);
            out.push_str(&format!("Local %{}\n", id.0));
        }
        AssignTarget::Name(name) => {
            indent(level, out);
            out.push_str(&format!("Name {name}\n"));
        }
        AssignTarget::Member {
            object,
            property,
            computed,
        } => {
            indent(level, out);
            if *computed {
                out.push_str("Member computed\n");
            } else {
                out.push_str("Member\n");
            }
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("property:\n");
            dump_expr(property, level + 2, out);
        }
        AssignTarget::Deref(ptr) => {
            indent(level, out);
            out.push_str("Deref\n");
            dump_expr(ptr, level + 1, out);
        }
        AssignTarget::ArrayPattern { elements } => {
            indent(level, out);
            out.push_str("ArrayPattern\n");
            dump_array_pattern_els(elements, level + 1, out);
        }
        AssignTarget::ObjectPattern { properties } => {
            indent(level, out);
            out.push_str("ObjectPattern\n");
            dump_object_pattern_els(properties, level + 1, out);
        }
    }
}

fn dump_array_pattern_els(elements: &[ArrayPatternEl], level: usize, out: &mut String) {
    for el in elements {
        match el {
            ArrayPatternEl::Elision => {
                indent(level, out);
                out.push_str("elision\n");
            }
            ArrayPatternEl::Pattern { binding, default } => {
                dump_pattern(binding, level, out);
                if let Some(def) = default {
                    indent(level, out);
                    out.push_str("default:\n");
                    dump_expr(def, level + 1, out);
                }
            }
            ArrayPatternEl::Rest(pat) => {
                indent(level, out);
                out.push_str("rest:\n");
                dump_pattern(pat, level + 1, out);
            }
        }
    }
}

fn dump_object_pattern_els(properties: &[ObjectPatternEl], level: usize, out: &mut String) {
    for p in properties {
        match p {
            ObjectPatternEl::Prop {
                key,
                binding,
                shorthand,
                default,
            } => {
                indent(level, out);
                if *shorthand {
                    out.push_str(&format!("prop shorthand {key}:\n"));
                } else {
                    out.push_str(&format!("prop {key}:\n"));
                }
                dump_pattern(binding, level + 1, out);
                if let Some(def) = default {
                    indent(level + 1, out);
                    out.push_str("default:\n");
                    dump_expr(def, level + 2, out);
                }
            }
            ObjectPatternEl::Rest(pat) => {
                indent(level, out);
                out.push_str("rest:\n");
                dump_pattern(pat, level + 1, out);
            }
        }
    }
}

fn dump_pattern(pat: &Pattern, level: usize, out: &mut String) {
    match pat {
        Pattern::Local(id) => {
            indent(level, out);
            out.push_str(&format!("local %{}\n", id.0));
        }
        Pattern::Name(name) => {
            indent(level, out);
            out.push_str(&format!("name {name}\n"));
        }
        Pattern::Member {
            object,
            property,
            computed,
        } => {
            indent(level, out);
            out.push_str(&format!("Member computed={computed}\n"));
            dump_expr(object, level + 1, out);
            dump_expr(property, level + 1, out);
        }
        Pattern::Array(els) => {
            indent(level, out);
            out.push_str("ArrayPattern\n");
            dump_array_pattern_els(els, level + 1, out);
        }
        Pattern::Object(props) => {
            indent(level, out);
            out.push_str("ObjectPattern\n");
            dump_object_pattern_els(props, level + 1, out);
        }
    }
}

fn dump_stmt(stmt: &Stmt, level: usize, out: &mut String) {
    match stmt {
        Stmt::Declare { local, init, kind } => {
            indent(level, out);
            let kw = match kind {
                BindingKind::Let => "let",
                BindingKind::Const => "const",
                BindingKind::Var => "var",
                BindingKind::Function => "function",
            };
            out.push_str(&format!("Declare {kw} %{}\n", local.0));
            if let Some(init) = init {
                indent(level + 1, out);
                out.push_str("init:\n");
                dump_expr(init, level + 2, out);
            }
        }
        Stmt::DeclareArrayPattern {
            kind,
            elements,
            init,
        } => {
            indent(level, out);
            let kw = match kind {
                BindingKind::Let => "let",
                BindingKind::Const => "const",
                BindingKind::Var => "var",
                BindingKind::Function => "function",
            };
            out.push_str(&format!("DeclareArrayPattern {kw}\n"));
            dump_array_pattern_els(elements, level + 1, out);
            if let Some(init) = init {
                indent(level + 1, out);
                out.push_str("init:\n");
                dump_expr(init, level + 2, out);
            }
        }
        Stmt::DeclareObjectPattern {
            kind,
            properties,
            init,
        } => {
            indent(level, out);
            let kw = match kind {
                BindingKind::Let => "let",
                BindingKind::Const => "const",
                BindingKind::Var => "var",
                BindingKind::Function => "function",
            };
            out.push_str(&format!("DeclareObjectPattern {kw}\n"));
            dump_object_pattern_els(properties, level + 1, out);
            if let Some(init) = init {
                indent(level + 1, out);
                out.push_str("init:\n");
                dump_expr(init, level + 2, out);
            }
        }
        Stmt::AssignLeft { target } => {
            indent(level, out);
            out.push_str("AssignLeft\n");
            dump_assign_target(target, level + 1, out);
        }
        Stmt::Expr { expr } => {
            indent(level, out);
            out.push_str("Expr\n");
            dump_expr(expr, level + 1, out);
        }
        Stmt::Block { body } => {
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
        Stmt::While { test, body } => {
            indent(level, out);
            out.push_str("While\n");
            indent(level + 1, out);
            out.push_str("test:\n");
            dump_expr(test, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::DoWhile { body, test } => {
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
        Stmt::ForIn { left, right, body } => {
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
        Stmt::Break { label } => {
            indent(level, out);
            if let Some(label) = label {
                out.push_str(&format!("Break {label}\n"));
            } else {
                out.push_str("Break\n");
            }
        }
        Stmt::Continue { label } => {
            indent(level, out);
            if let Some(label) = label {
                out.push_str(&format!("Continue {label}\n"));
            } else {
                out.push_str("Continue\n");
            }
        }
        Stmt::Labeled { label, body } => {
            indent(level, out);
            out.push_str(&format!("Labeled {label}\n"));
            dump_stmt(body, level + 1, out);
        }
        Stmt::Switch {
            discriminant,
            cases,
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
        Stmt::Function {
            local,
            params,
            body,
            is_async,
            is_generator,
        } => {
            indent(level, out);
            out.push_str(&format!("Function %{}\n", local.0));
            if *is_async {
                indent(level + 1, out);
                out.push_str("async: true\n");
            }
            if *is_generator {
                indent(level + 1, out);
                out.push_str("generator: true\n");
            }
            dump_params(params, level + 1, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            for s in body {
                dump_stmt(s, level + 2, out);
            }
        }
        Stmt::Return { value } => {
            indent(level, out);
            out.push_str("Return\n");
            if let Some(value) = value {
                dump_expr(value, level + 1, out);
            }
        }
        Stmt::Throw { value } => {
            indent(level, out);
            out.push_str("Throw\n");
            dump_expr(value, level + 1, out);
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            indent(level, out);
            out.push_str("Try\n");
            indent(level + 1, out);
            out.push_str("block:\n");
            for s in block {
                dump_stmt(s, level + 2, out);
            }
            if let Some(handler) = handler {
                indent(level + 1, out);
                out.push_str("catch");
                if let Some(param) = handler_param {
                    out.push_str(&format!(" %{}", param.0));
                }
                out.push_str(":\n");
                for s in handler {
                    dump_stmt(s, level + 2, out);
                }
            }
            if let Some(finalizer) = finalizer {
                indent(level + 1, out);
                out.push_str("finally:\n");
                for s in finalizer {
                    dump_stmt(s, level + 2, out);
                }
            }
        }
        Stmt::With { object, body } => {
            indent(level, out);
            out.push_str("With\n");
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            for s in body {
                dump_stmt(s, level + 2, out);
            }
        }
    }
}

fn dump_expr(expr: &Expr, level: usize, out: &mut String) {
    match expr {
        Expr::Local { id, ty } => {
            indent(level, out);
            out.push_str(&format!("Local %{} : {ty}\n", id.0));
        }
        Expr::IdentName { name, ty } => {
            indent(level, out);
            out.push_str(&format!("IdentName {name} : {ty}\n"));
        }
        Expr::Number { raw, ty } => {
            indent(level, out);
            out.push_str(&format!("Number {raw} : {ty}\n"));
        }
        Expr::BigInt { raw, ty } => {
            indent(level, out);
            out.push_str(&format!("BigInt {raw} : {ty}\n"));
        }
        Expr::String { value, ty } => {
            indent(level, out);
            out.push_str(&format!("String {:?} : {ty}\n", value.to_string_lossy()));
        }
        Expr::RegExp {
            pattern,
            flags,
            ty,
        } => {
            indent(level, out);
            out.push_str(&format!("RegExp /{pattern}/{flags} : {ty}\n"));
        }
        Expr::Template {
            quasis,
            expressions,
            ty,
        } => {
            indent(level, out);
            out.push_str(&format!("Template : {ty}\n"));
            for (i, q) in quasis.iter().enumerate() {
                indent(level + 1, out);
                out.push_str(&format!("Quasi {:?}\n", q.to_string_lossy()));
                if i < expressions.len() {
                    dump_expr(&expressions[i], level + 1, out);
                }
            }
        }
        Expr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            ty,
        } => {
            indent(level, out);
            out.push_str(&format!("TaggedTemplate : {ty}\n"));
            indent(level + 1, out);
            out.push_str("tag:\n");
            dump_expr(tag, level + 2, out);
            for (i, q) in quasis.iter().enumerate() {
                indent(level + 1, out);
                out.push_str(&format!("Quasi {:?}\n", q.to_string_lossy()));
                if i < expressions.len() {
                    dump_expr(&expressions[i], level + 1, out);
                }
            }
        }
        Expr::Boolean { value, ty } => {
            indent(level, out);
            out.push_str(&format!("Boolean {value} : {ty}\n"));
        }
        Expr::Null { ty } => {
            indent(level, out);
            out.push_str(&format!("Null : {ty}\n"));
        }
        Expr::This { ty } => {
            indent(level, out);
            out.push_str(&format!("This : {ty}\n"));
        }
        Expr::NewTarget { ty } => {
            indent(level, out);
            out.push_str(&format!("NewTarget : {ty}\n"));
        }
        Expr::Unary { op, arg, ty } => {
            indent(level, out);
            out.push_str(&format!("Unary {op} : {ty}\n"));
            dump_expr(arg, level + 1, out);
        }
        Expr::Binary {
            left, op, right, ty,
        } => {
            indent(level, out);
            out.push_str(&format!("Binary {op} : {ty}\n"));
            dump_expr(left, level + 1, out);
            dump_expr(right, level + 1, out);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ty,
        } => {
            indent(level, out);
            out.push_str(&format!("Conditional : {ty}\n"));
            dump_expr(test, level + 1, out);
            dump_expr(consequent, level + 1, out);
            dump_expr(alternate, level + 1, out);
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            indent(level, out);
            match target {
                AssignTarget::Local(id) => {
                    out.push_str(&format!("Assign {op} %{} : {ty}\n", id.0));
                }
                AssignTarget::Name(name) => {
                    out.push_str(&format!("Assign {op} {name} : {ty}\n"));
                }
                AssignTarget::Member {
                    object,
                    property,
                    computed,
                } => {
                    if *computed {
                        out.push_str(&format!("Assign {op} member computed : {ty}\n"));
                    } else {
                        out.push_str(&format!("Assign {op} member : {ty}\n"));
                    }
                    indent(level + 1, out);
                    out.push_str("object:\n");
                    dump_expr(object, level + 2, out);
                    indent(level + 1, out);
                    out.push_str("property:\n");
                    dump_expr(property, level + 2, out);
                }
                AssignTarget::Deref(ptr) => {
                    out.push_str(&format!("Assign {op} deref : {ty}\n"));
                    dump_expr(ptr, level + 1, out);
                }
                AssignTarget::ArrayPattern { elements } => {
                    out.push_str(&format!("Assign {op} ArrayPattern : {ty}\n"));
                    dump_array_pattern_els(elements, level + 1, out);
                }
                AssignTarget::ObjectPattern { properties } => {
                    out.push_str(&format!("Assign {op} ObjectPattern : {ty}\n"));
                    dump_object_pattern_els(properties, level + 1, out);
                }
            }
            dump_expr(value, level + 1, out);
        }
        Expr::Update {
            op,
            target,
            prefix,
            ty,
        } => {
            indent(level, out);
            let kind = if *prefix { "prefix" } else { "postfix" };
            match target {
                UpdateTarget::Local(id) => {
                    out.push_str(&format!("Update {kind} {op} %{} : {ty}\n", id.0));
                }
                UpdateTarget::Name(name) => {
                    out.push_str(&format!("Update {kind} {op} {name} : {ty}\n"));
                }
                UpdateTarget::Member {
                    object,
                    property,
                    computed,
                } => {
                    out.push_str(&format!("Update {kind} {op} Member : {ty}\n"));
                    dump_expr(object, level + 1, out);
                    if *computed {
                        dump_expr(property, level + 1, out);
                    } else {
                        dump_expr(property, level + 1, out);
                    }
                }
            }
        }
        Expr::Call {
            callee,
            args,
            optional,
            ty,
        } => {
            indent(level, out);
            if *optional {
                out.push_str(&format!("Call optional : {ty}\n"));
            } else {
                out.push_str(&format!("Call : {ty}\n"));
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
        Expr::New { callee, args, ty } => {
            indent(level, out);
            out.push_str(&format!("New : {ty}\n"));
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
        Expr::Function {
            name,
            params,
            body,
            is_async,
            is_generator,
            is_arrow,
            ty,
        } => {
            indent(level, out);
            out.push_str(&format!("FunctionExpr : {ty}\n"));
            if *is_async {
                indent(level + 1, out);
                out.push_str("async: true\n");
            }
            if *is_generator {
                indent(level + 1, out);
                out.push_str("generator: true\n");
            }
            if *is_arrow {
                indent(level + 1, out);
                out.push_str("arrow: true\n");
            }
            if let Some(local) = name {
                indent(level + 1, out);
                out.push_str(&format!("name: %{}\n", local.0));
            }
            dump_params(params, level + 1, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            for s in body {
                dump_stmt(s, level + 2, out);
            }
        }
        Expr::Object { properties, ty } => {
            indent(level, out);
            out.push_str(&format!("Object : {ty}\n"));
            for prop in properties {
                indent(level + 1, out);
                match prop {
                    ObjectProp::Property { key, value } => match key {
                        ObjectPropKey::Static(k) => {
                            out.push_str(&format!("prop {:?}:\n", k.to_string_lossy()));
                            dump_expr(value, level + 2, out);
                        }
                        ObjectPropKey::Computed(k) => {
                            out.push_str("prop computed:\n");
                            indent(level + 2, out);
                            out.push_str("key:\n");
                            dump_expr(k, level + 3, out);
                            indent(level + 2, out);
                            out.push_str("value:\n");
                            dump_expr(value, level + 3, out);
                        }
                    },
                    ObjectProp::Accessor { kind, key, value } => {
                        let kind_s = match kind {
                            AccessorKind::Get => "get",
                            AccessorKind::Set => "set",
                        };
                        match key {
                            ObjectPropKey::Static(k) => {
                                out.push_str(&format!(
                                    "accessor {kind_s} {:?}:\n",
                                    k.to_string_lossy()
                                ));
                                dump_expr(value, level + 2, out);
                            }
                            ObjectPropKey::Computed(k) => {
                                out.push_str(&format!("accessor {kind_s} computed:\n"));
                                indent(level + 2, out);
                                out.push_str("key:\n");
                                dump_expr(k, level + 3, out);
                                indent(level + 2, out);
                                out.push_str("value:\n");
                                dump_expr(value, level + 3, out);
                            }
                        }
                    }
                    ObjectProp::Spread(expr) => {
                        out.push_str("spread:\n");
                        dump_expr(expr, level + 2, out);
                    }
                }
            }
        }
        Expr::Array { elements, ty } => {
            indent(level, out);
            out.push_str(&format!("Array : {ty}\n"));
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
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ty,
        } => {
            indent(level, out);
            match (*optional, *computed) {
                (true, true) => out.push_str(&format!("Member optional computed : {ty}\n")),
                (true, false) => out.push_str(&format!("Member optional : {ty}\n")),
                (false, true) => out.push_str(&format!("Member computed : {ty}\n")),
                (false, false) => out.push_str(&format!("Member : {ty}\n")),
            }
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("property:\n");
            dump_expr(property, level + 2, out);
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
        match (&p.pattern, p.rest) {
            (Pattern::Local(id), true) => {
                indent(level + 1, out);
                out.push_str(&format!("rest %{}\n", id.0));
            }
            (Pattern::Local(id), false) => {
                indent(level + 1, out);
                out.push_str(&format!("%{}\n", id.0));
            }
            (pat, true) => {
                indent(level + 1, out);
                out.push_str("rest:\n");
                dump_pattern(pat, level + 2, out);
            }
            (pat, false) => {
                dump_pattern(pat, level + 1, out);
            }
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
    use draconic_check::check;
    use draconic_parser::parse;

    fn lower_src(src: &str) -> Module {
        let program = parse(src).unwrap();
        let checked = check(program).unwrap();
        lower(&checked)
    }

    fn local_by_name<'a>(module: &'a Module, name: &str) -> &'a Local {
        module
            .locals
            .iter()
            .find(|l| l.name == name)
            .unwrap_or_else(|| panic!("no local `{name}`"))
    }

    #[test]
    fn lower_let_number_declares_typed_local() {
        let module = lower_src("let x = 1;");
        let x = local_by_name(&module, "x");
        assert_eq!(x.ty, Type::Number);
        assert_eq!(module.body.len(), 1);
        match &module.body[0] {
            Stmt::Declare {
                local,
                init: Some(Expr::Number { raw, ty }),
                kind,
            } => {
                assert_eq!(*local, x.id);
                assert_eq!(raw, "1");
                assert_eq!(*ty, Type::Number);
                assert_eq!(*kind, BindingKind::Let);
            }
            other => panic!("unexpected body: {other:?}"),
        }
    }

    #[test]
    fn lower_resolves_ident_to_local() {
        let module = lower_src("let x = 1; x;");
        let x = local_by_name(&module, "x");
        assert_eq!(module.body.len(), 2);
        match &module.body[1] {
            Stmt::Expr {
                expr: Expr::Local { id, ty },
            } => {
                assert_eq!(*id, x.id);
                assert_eq!(*ty, Type::Number);
            }
            other => panic!("unexpected body: {other:?}"),
        }
    }

    #[test]
    fn lower_binary_preserves_result_type() {
        let module = lower_src("let a = 1 + 2;");
        match &module.body[0] {
            Stmt::Declare {
                init: Some(Expr::Binary { op, ty, left, right }),
                ..
            } => {
                assert_eq!(*op, BinaryOp::Add);
                assert_eq!(*ty, Type::Number);
                assert_eq!(left.ty(), Type::Number);
                assert_eq!(right.ty(), Type::Number);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn lower_string_concat_type() {
        let module = lower_src(r#"let s = "a" + "b";"#);
        assert_eq!(local_by_name(&module, "s").ty, Type::String);
        match &module.body[0] {
            Stmt::Declare {
                init: Some(Expr::Binary { ty, .. }),
                ..
            } => assert_eq!(*ty, Type::String),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn lower_strips_parens() {
        let module = lower_src("let x = (1);");
        match &module.body[0] {
            Stmt::Declare {
                init: Some(Expr::Number { raw, .. }),
                ..
            } => assert_eq!(raw, "1"),
            other => panic!("parens should be stripped: {other:?}"),
        }
    }

    #[test]
    fn lower_drops_empty_statements() {
        let module = lower_src("let x = 1;;;");
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Stmt::Declare { .. }));
    }

    #[test]
    fn lower_uninitialized_let() {
        let module = lower_src("let x;");
        assert_eq!(local_by_name(&module, "x").ty, Type::Any);
        match &module.body[0] {
            Stmt::Declare { init: None, .. } => {}
            other => panic!("expected bare declare: {other:?}"),
        }
    }

    #[test]
    fn lower_unary_and_literals() {
        let module = lower_src(r#"let a = -1; let b = !false; let c = null; let d = true;"#);
        assert_eq!(local_by_name(&module, "a").ty, Type::Number);
        assert_eq!(local_by_name(&module, "b").ty, Type::Boolean);
        assert_eq!(local_by_name(&module, "c").ty, Type::Null);
        assert_eq!(local_by_name(&module, "d").ty, Type::Boolean);
    }

    #[test]
    fn lower_propagates_binding_through_use() {
        let module = lower_src("let x = 1; let y = x + 2;");
        let x = local_by_name(&module, "x");
        match &module.body[1] {
            Stmt::Declare {
                init:
                    Some(Expr::Binary {
                        left: box_left,
                        ty,
                        ..
                    }),
                ..
            } => {
                assert_eq!(*ty, Type::Number);
                match box_left.as_ref() {
                    Expr::Local { id, ty } => {
                        assert_eq!(*id, x.id);
                        assert_eq!(*ty, Type::Number);
                    }
                    other => panic!("expected local x: {other:?}"),
                }
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn dump_module_stable() {
        let module = lower_src("let x = 1; x;");
        let dump = dump_module(&module);
        assert_eq!(
            dump,
            "\
Module
  locals:
    %0 Math: object
    %1 Number: function
    %2 NaN: number
    %3 Infinity: number
    %4 Symbol: function
    %5 Promise: function
    %6 Proxy: function
    %7 Reflect: object
    %8 undefined: any
    %9 globalThis: object
    %10 Object: function
    %11 Function: function
    %12 Array: function
    %13 String: function
    %14 Boolean: function
    %15 Error: function
    %16 TypeError: function
    %17 RangeError: function
    %18 ReferenceError: function
    %19 SyntaxError: function
    %20 URIError: function
    %21 EvalError: function
    %22 AggregateError: function
    %23 parseInt: function
    %24 parseFloat: function
    %25 isNaN: function
    %26 isFinite: function
    %27 encodeURI: function
    %28 decodeURI: function
    %29 encodeURIComponent: function
    %30 decodeURIComponent: function
    %31 JSON: object
    %32 Date: function
    %33 RegExp: function
    %34 Map: function
    %35 Set: function
    %36 WeakMap: function
    %37 WeakSet: function
    %38 ArrayBuffer: function
    %39 DataView: function
    %40 Int8Array: function
    %41 Uint8Array: function
    %42 Uint8ClampedArray: function
    %43 Int16Array: function
    %44 Uint16Array: function
    %45 Int32Array: function
    %46 Uint32Array: function
    %47 Float32Array: function
    %48 Float64Array: function
    %49 BigInt64Array: function
    %50 BigUint64Array: function
    %51 eval: function
    %52 escape: function
    %53 unescape: function
    %54 x: number
  body:
    Declare let %54
      init:
        Number 1 : number
    Expr
      Local %54 : number
"
        );
    }

    #[test]
    fn lower_call_shape() {
        // `any` is callable on the minimal surface; use uninit binding.
        let module = lower_src("let f; f(1);");
        match &module.body[1] {
            Stmt::Expr {
                expr: Expr::Call {
                    callee,
                    args,
                    optional,
                    ty,
                },
            } => {
                assert_eq!(*ty, Type::Any);
                assert!(!*optional);
                assert!(matches!(callee.as_ref(), Expr::Local { .. }));
                assert_eq!(args.len(), 1);
                assert!(matches!(args[0], Arg::Expr(Expr::Number { .. })));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn lower_update_prefix_and_postfix() {
        let module = lower_src("let x = 1; ++x; x++;");
        let x = local_by_name(&module, "x");
        match &module.body[1] {
            Stmt::Expr {
                expr:
                    Expr::Update {
                        op,
                        target,
                        prefix,
                        ty,
                    },
            } => {
                assert_eq!(*op, UpdateOp::Inc);
                assert_eq!(*target, UpdateTarget::Local(x.id));
                assert!(*prefix);
                assert_eq!(*ty, Type::Number);
            }
            other => panic!("unexpected prefix: {other:?}"),
        }
        match &module.body[2] {
            Stmt::Expr {
                expr:
                    Expr::Update {
                        op,
                        target,
                        prefix,
                        ty,
                    },
            } => {
                assert_eq!(*op, UpdateOp::Inc);
                assert_eq!(*target, UpdateTarget::Local(x.id));
                assert!(!*prefix);
                assert_eq!(*ty, Type::Number);
            }
            other => panic!("unexpected postfix: {other:?}"),
        }
    }

    #[test]
    fn lower_update_on_member() {
        let module = lower_src("let o = { x: 1 }; o.x++; ++o[\"x\"];");
        match &module.body[1] {
            Stmt::Expr {
                expr:
                    Expr::Update {
                        op,
                        target: UpdateTarget::Member { computed, .. },
                        prefix,
                        ty,
                    },
            } => {
                assert_eq!(*op, UpdateOp::Inc);
                assert!(!*computed);
                assert!(!*prefix);
                assert_eq!(*ty, Type::Number);
            }
            other => panic!("unexpected member postfix: {other:?}"),
        }
        match &module.body[2] {
            Stmt::Expr {
                expr:
                    Expr::Update {
                        target: UpdateTarget::Member { computed, .. },
                        prefix,
                        ..
                    },
            } => {
                assert!(*computed);
                assert!(*prefix);
            }
            other => panic!("unexpected member prefix: {other:?}"),
        }
    }


    /// Repeated / nested lower must not share private-field bookkeeping (issues-15).
    #[test]
    fn lower_private_field_state_isolated_across_calls() {
        let src_a = r#"
            class A {
                #x = 1;
                getX() { return this.#x; }
            }
        "#;
        let src_b = r#"
            class B {
                #y = 2;
                getY() { return this.#y; }
            }
        "#;

        let a1 = lower_src(src_a);
        let b = lower_src(src_b);
        let a2 = lower_src(src_a);

        let wm_a1: Vec<_> = a1
            .locals
            .iter()
            .filter(|l| l.name.contains("__drac_pf_"))
            .map(|l| l.name.as_str())
            .collect();
        let wm_b: Vec<_> = b
            .locals
            .iter()
            .filter(|l| l.name.contains("__drac_pf_"))
            .map(|l| l.name.as_str())
            .collect();
        let wm_a2: Vec<_> = a2
            .locals
            .iter()
            .filter(|l| l.name.contains("__drac_pf_"))
            .map(|l| l.name.as_str())
            .collect();

        assert_eq!(wm_a1.len(), 1, "A should allocate one private-field WeakMap");
        assert_eq!(wm_b.len(), 1, "B should allocate one private-field WeakMap");
        assert_eq!(wm_a2.len(), 1, "second A lower should allocate one WeakMap");
        assert!(wm_a1[0].contains("x"), "A WeakMap name should mention x: {}", wm_a1[0]);
        assert!(wm_b[0].contains("y"), "B WeakMap name should mention y: {}", wm_b[0]);
        assert_ne!(wm_a1[0], wm_b[0]);
        assert_eq!(wm_a1[0], wm_a2[0]);
        assert_eq!(dump_module(&a1), dump_module(&a2));
    }

    #[test]
    fn lower_nested_classes_private_fields_do_not_clobber() {
        let module = lower_src(
            r#"
            class Outer {
                #o = 1;
                inner() {
                    class Inner {
                        #i = 2;
                        getI() { return this.#i; }
                    }
                    return new Inner();
                }
                getO() { return this.#o; }
            }
        "#,
        );
        let pfs: Vec<_> = module
            .locals
            .iter()
            .filter(|l| l.name.starts_with("__drac_pf_"))
            .map(|l| l.name.as_str())
            .collect();
        assert_eq!(pfs.len(), 2, "outer and inner each need a WeakMap: {pfs:?}");
        assert!(pfs.iter().any(|n| n.contains("o")), "{pfs:?}");
        assert!(pfs.iter().any(|n| n.contains("i")), "{pfs:?}");
    }
}
