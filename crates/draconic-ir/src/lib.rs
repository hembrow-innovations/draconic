//! Shared IR lowered from checked Programs (ROADMAP B06).

use std::collections::{HashMap, HashSet};

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
    /// Inside object method/accessor: keep `super` for JS home-object emit (E19.23).
    object_super: bool,
    /// Derived class constructor: ES `this` binding temp (uninit until `super()`).
    /// Inherited by nested arrows (lexical this); cleared in nested non-arrow functions.
    derived_this: Option<LocalId>,
    /// Derived class constructor: heritage local for `Reflect.construct`.
    derived_super: Option<LocalId>,
    /// Side-effect exprs run once after `super()` binds this (fields/brands).
    derived_super_inits: Vec<Expr>,
    /// True only while lowering the constructor body itself (not nested arrows/fns).
    /// Gates [[Construct]] return completion wrapping (E19.82.03).
    derived_ctor_body: bool,
    /// True while lowering a class field initializer expression (not nested fn bodies).
    /// Direct `eval` gets field-init early errors (E19.82.06 ContainsArguments).
    in_field_init: bool,
    /// Class declaration: outer mutable name → inner immutable const local (E19.57).
    /// Stack so nested class decls restore the outer remap.
    class_name_remap: Vec<(LocalId, LocalId)>,
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
            object_super: false,
            derived_this: None,
            derived_super: None,
            derived_super_inits: Vec::new(),
            derived_ctor_body: false,
            in_field_init: false,
            class_name_remap: Vec::new(),
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

    /// Map class declaration outer name to the inner immutable binding while lowering the body.
    fn map_class_name(&self, id: LocalId) -> LocalId {
        for &(outer, inner) in self.class_name_remap.iter().rev() {
            if outer == id {
                return inner;
            }
        }
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

    // Private compound/update temps are assigned without a prior Declare (sloppy-mode
    // globals historically). Hoist `var` so strict class methods can assign them (E19.72).
    // Only these prefixes: other synthetics are params (`__drac_o`) or already declared.
    let mut hoisted = Vec::new();
    let mut hoisted_spans = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    for local in &ctx.extra_locals {
        let n = local.name.as_str();
        if !(n.starts_with("__drac_pobj_")
            || n.starts_with("__drac_pval_")
            || n.starts_with("__drac_pnext_")
            || n.starts_with("__drac_pcur_")
            || n.starts_with("__drac_dstr_"))
        {
            continue;
        }
        if !seen_names.insert(local.name.clone()) {
            continue;
        }
        hoisted.push(Stmt::Declare {
            local: local.id,
            init: None,
            kind: BindingKind::Var,
        });
        hoisted_spans.push(Span::dummy());
    }
    if !hoisted.is_empty() {
        hoisted.append(&mut body);
        hoisted_spans.append(&mut body_spans);
        body = hoisted;
        body_spans = hoisted_spans;
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
                    init: init.as_ref().map(|e| {
                        lower_expr_hint(checked, ctx, e, super_class, Some(name.name.as_str()))
                    }),
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
            // Nested functions do not inherit `super` / derived ctor this (class parent or object home).
            let prev_object_super = ctx.object_super;
            let prev_derived_this = ctx.derived_this.take();
            let prev_derived_super = ctx.derived_super.take();
            let prev_inits = std::mem::take(&mut ctx.derived_super_inits);
            let prev_ctor_body = ctx.derived_ctor_body;
            ctx.object_super = false;
            ctx.derived_ctor_body = false;
            let body = lower_fn_body(checked, ctx, body, None);
            ctx.object_super = prev_object_super;
            ctx.derived_this = prev_derived_this;
            ctx.derived_super = prev_derived_super;
            ctx.derived_super_inits = prev_inits;
            ctx.derived_ctor_body = prev_ctor_body;
            Some(Stmt::Function {
                local,
                params,
                body,
                is_async: *is_async,
                is_generator: *is_generator,
            })
        }
        AstStmt::Return { argument, .. } => {
            let value = argument
                .as_ref()
                .map(|e| lower_expr(checked, ctx, e, super_class));
            if ctx.derived_ctor_body {
                if let Some(this_id) = ctx.derived_this {
                    // Derived [[Construct]] return completion (E19.82.03).
                    return Some(Stmt::Return {
                        value: Some(possible_constructor_return(this_id, value)),
                    });
                }
            }
            Some(Stmt::Return { value })
        }
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
            let handler_param = handler_param
                .as_ref()
                .map(|param| lower_binding_pattern(checked, ctx, param));
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
///
/// E19.57: outer binding is mutable (`let C = …`); methods close over an inner `const` name so
/// reassignment inside the class is a runtime TypeError while `C = null` outside still works.
fn lower_class(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    name: &Ident,
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
) -> Vec<Stmt> {
    let outer = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.span == name.span)
        .map(|s| s.id)
        .expect("class binding must be declared");
    let inner = ctx.alloc_synthetic_local(format!("__cls_{}", name.name), Type::Function);
    ctx.class_name_remap.push((outer, inner));
    // Pass BindingIdentifier so constructor `.name === "C"` (not `__cls_C` from const).
    // Anonymous `export default class {…}` uses synthetic `__class` → SetFunctionName "default"
    // (E19.82.04 / ClassDefinitionEvaluation className for Default export).
    let name_hint = if name.name == "__class" {
        "default"
    } else {
        name.name.as_str()
    };
    let mut body = lower_class_local(
        checked,
        ctx,
        inner,
        super_class,
        elements,
        Some(name_hint),
    );
    ctx.class_name_remap.pop();
    body.push(Stmt::Return {
        value: Some(Expr::Local {
            id: inner,
            ty: Type::Function,
        }),
    });
    let (needs_yield, needs_await) = class_eval_yield_await(super_class, elements);
    let iife = wrap_class_builder_iife(body, needs_yield, needs_await);
    vec![Stmt::Declare {
        local: outer,
        init: Some(iife),
        kind: BindingKind::Let,
    }]
}

/// Class expression → IIFE that builds the constructor and returns it (E18.33).
///
/// `name_hint` is the NamedEvaluation binding id for anonymous classes
/// (`var cls = class {}` → constructor `.name === "cls"`) (E19.31).
fn lower_class_expression(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    name: Option<&Ident>,
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
    span: Span,
    name_hint: Option<&str>,
) -> Expr {
    let class_span = name.map(|n| n.span).unwrap_or(span);
    let local = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.span == class_span)
        .map(|s| s.id)
        .expect("class expression binding must be declared");
    // Named classes keep their BindingIdentifier. Anonymous classes always get
    // SetFunctionName: binding hint when present, else "" (ECMA-262 default).
    let named_eval = if name.is_none() {
        Some(name_hint.unwrap_or(""))
    } else {
        None
    };
    let mut body = lower_class_local(checked, ctx, local, super_class, elements, named_eval);
    body.push(Stmt::Return {
        value: Some(Expr::Local {
            id: local,
            ty: Type::Function,
        }),
    });
    let (needs_yield, needs_await) = class_eval_yield_await(super_class, elements);
    wrap_class_builder_iife(body, needs_yield, needs_await)
}

/// Whether ClassDefinitionEvaluation evaluates `yield` / `await` (E19.78).
/// Computed keys, extends, static field inits, and static blocks run at class eval time.
fn class_eval_yield_await(
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
) -> (bool, bool) {
    let mut needs_yield = super_class.is_some_and(ast_has_yield);
    let mut needs_await = super_class.is_some_and(ast_has_await);
    for el in elements {
        match el {
            ClassElement::Method { key, .. } | ClassElement::Accessor { key, .. } => {
                needs_yield |= object_key_has_yield(key);
                needs_await |= object_key_has_await(key);
            }
            ClassElement::Field {
                key,
                value,
                is_static,
                ..
            } => {
                needs_yield |= object_key_has_yield(key);
                needs_await |= object_key_has_await(key);
                if *is_static {
                    if let Some(v) = value {
                        needs_yield |= ast_has_yield(v);
                        needs_await |= ast_has_await(v);
                    }
                }
            }
            ClassElement::StaticBlock { body, .. } => {
                needs_await |= stmt_has_await(body);
            }
            ClassElement::Constructor { .. } => {}
        }
        if needs_yield && needs_await {
            break;
        }
    }
    (needs_yield, needs_await)
}

/// Build class IIFE, preserving outer `yield`/`await` via `yield*` / `await` (E19.78).
fn wrap_class_builder_iife(body: Vec<Stmt>, needs_yield: bool, needs_await: bool) -> Expr {
    let call = Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: Vec::new(),
            body,
            is_async: needs_await && !needs_yield,
            is_generator: needs_yield,
            is_arrow: false,
            is_method: false,
            ty: Type::Function,
        }),
        args: Vec::new(),
        optional: false,
        ty: Type::Function,
    };
    if needs_yield {
        Expr::Unary {
            op: UnaryOp::YieldStar,
            arg: Box::new(call),
            ty: Type::Function,
        }
    } else if needs_await {
        Expr::Unary {
            op: UnaryOp::Await,
            arg: Box::new(call),
            ty: Type::Function,
        }
    } else {
        call
    }
}

fn object_key_has_yield(key: &draconic_ast::ObjectKey) -> bool {
    matches!(key, draconic_ast::ObjectKey::Computed(e) if ast_has_yield(e))
}

fn object_key_has_await(key: &draconic_ast::ObjectKey) -> bool {
    matches!(key, draconic_ast::ObjectKey::Computed(e) if ast_has_await(e))
}

/// True if `expr` evaluates a `yield`/`yield*` in the current function (not nested fn/class).
fn ast_has_yield(expr: &AstExpr) -> bool {
    match expr {
        AstExpr::Unary {
            op: UnaryOp::Yield | UnaryOp::YieldStar,
            ..
        } => true,
        AstExpr::FunctionExpression { .. } | AstExpr::ClassExpression { .. } => false,
        AstExpr::Unary { arg, .. }
        | AstExpr::Paren { expr: arg, .. }
        | AstExpr::As { expr: arg, .. }
        | AstExpr::Update { arg, .. } => ast_has_yield(arg),
        AstExpr::Binary { left, right, .. } | AstExpr::Assign { target: left, value: right, .. } => {
            ast_has_yield(left) || ast_has_yield(right)
        }
        AstExpr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => ast_has_yield(test) || ast_has_yield(consequent) || ast_has_yield(alternate),
        AstExpr::Call { callee, args, .. } | AstExpr::New { callee, args, .. } => {
            ast_has_yield(callee)
                || args.iter().any(|a| match a {
                    AstArg::Expr(e) | AstArg::Spread(e) => ast_has_yield(e),
                })
        }
        AstExpr::MemberExpression {
            object, property, ..
        } => ast_has_yield(object) || ast_has_yield(property),
        AstExpr::PrivateIn { object, .. } => ast_has_yield(object),
        AstExpr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            AstArrayElement::Expr(e) | AstArrayElement::Spread(e) => ast_has_yield(e),
            AstArrayElement::Elision => false,
        }),
        AstExpr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            AstObjectProp::Property { key, value, .. } => {
                object_key_has_yield(key) || ast_has_yield(value)
            }
            AstObjectProp::Accessor { key, body, .. } => {
                object_key_has_yield(key) || stmt_has_yield(body)
            }
            AstObjectProp::Spread { expr, .. } => ast_has_yield(expr),
        }),
        AstExpr::ArrowFunction { body, params, .. } => {
            // Arrow may contain yield only in defaults (illegal in generator params separately).
            params
                .iter()
                .any(|p| p.default.as_ref().is_some_and(ast_has_yield))
                || match body {
                    draconic_ast::ArrowBody::Expr(e) => ast_has_yield(e),
                    draconic_ast::ArrowBody::Block(s) => stmt_has_yield(s),
                }
        }
        AstExpr::TemplateLiteral { expressions, .. } => expressions.iter().any(ast_has_yield),
        AstExpr::TaggedTemplate {
            tag, expressions, ..
        } => ast_has_yield(tag) || expressions.iter().any(ast_has_yield),
        AstExpr::ImportCall {
            source, options, ..
        } => ast_has_yield(source) || options.as_ref().is_some_and(|o| ast_has_yield(o)),
        AstExpr::ArrayPattern { .. } | AstExpr::ObjectPattern { .. } => false,
        _ => false,
    }
}

fn stmt_has_yield(stmt: &AstStmt) -> bool {
    match stmt {
        AstStmt::Block { body, .. } => body.iter().any(stmt_has_yield),
        AstStmt::Expression { expr, .. } => ast_has_yield(expr),
        AstStmt::Return {
            argument: Some(e), ..
        }
        | AstStmt::Throw { argument: e, .. } => ast_has_yield(e),
        AstStmt::Let { init: Some(e), .. } => ast_has_yield(e),
        AstStmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            ast_has_yield(test)
                || stmt_has_yield(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_has_yield(a))
        }
        AstStmt::While { test, body, .. } => ast_has_yield(test) || stmt_has_yield(body),
        AstStmt::DoWhile { body, test, .. } => stmt_has_yield(body) || ast_has_yield(test),
        AstStmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            init.as_ref().is_some_and(|s| stmt_has_yield(s))
                || test.as_ref().is_some_and(ast_has_yield)
                || update.as_ref().is_some_and(ast_has_yield)
                || stmt_has_yield(body)
        }
        AstStmt::ForIn { left, right, body, .. } | AstStmt::ForOf { left, right, body, .. } => {
            stmt_has_yield(left) || ast_has_yield(right) || stmt_has_yield(body)
        }
        AstStmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            stmt_has_yield(block)
                || handler.as_ref().is_some_and(|h| stmt_has_yield(h))
                || finalizer.as_ref().is_some_and(|f| stmt_has_yield(f))
        }
        AstStmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            ast_has_yield(discriminant)
                || cases.iter().any(|c| {
                    c.test.as_ref().is_some_and(ast_has_yield) || c.body.iter().any(stmt_has_yield)
                })
        }
        AstStmt::Labeled { body, .. } => stmt_has_yield(body),
        AstStmt::With { object, body, .. } => ast_has_yield(object) || stmt_has_yield(body),
        _ => false,
    }
}

/// True if `expr` evaluates `await` in the current async/module context (not nested async fn).
fn ast_has_await(expr: &AstExpr) -> bool {
    match expr {
        AstExpr::Unary {
            op: UnaryOp::Await, ..
        } => true,
        AstExpr::FunctionExpression { .. } | AstExpr::ClassExpression { .. } => false,
        AstExpr::ArrowFunction {
            is_async: true, ..
        } => false,
        AstExpr::Unary { arg, .. }
        | AstExpr::Paren { expr: arg, .. }
        | AstExpr::As { expr: arg, .. }
        | AstExpr::Update { arg, .. } => ast_has_await(arg),
        AstExpr::Binary { left, right, .. } | AstExpr::Assign { target: left, value: right, .. } => {
            ast_has_await(left) || ast_has_await(right)
        }
        AstExpr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => ast_has_await(test) || ast_has_await(consequent) || ast_has_await(alternate),
        AstExpr::Call { callee, args, .. } | AstExpr::New { callee, args, .. } => {
            ast_has_await(callee)
                || args.iter().any(|a| match a {
                    AstArg::Expr(e) | AstArg::Spread(e) => ast_has_await(e),
                })
        }
        AstExpr::MemberExpression {
            object, property, ..
        } => ast_has_await(object) || ast_has_await(property),
        AstExpr::PrivateIn { object, .. } => ast_has_await(object),
        AstExpr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            AstArrayElement::Expr(e) | AstArrayElement::Spread(e) => ast_has_await(e),
            AstArrayElement::Elision => false,
        }),
        AstExpr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            AstObjectProp::Property { key, value, .. } => {
                object_key_has_await(key) || ast_has_await(value)
            }
            AstObjectProp::Accessor { key, body, .. } => {
                object_key_has_await(key) || stmt_has_await(body)
            }
            AstObjectProp::Spread { expr, .. } => ast_has_await(expr),
        }),
        AstExpr::ArrowFunction { body, params, .. } => {
            params
                .iter()
                .any(|p| p.default.as_ref().is_some_and(ast_has_await))
                || match body {
                    draconic_ast::ArrowBody::Expr(e) => ast_has_await(e),
                    draconic_ast::ArrowBody::Block(s) => stmt_has_await(s),
                }
        }
        AstExpr::TemplateLiteral { expressions, .. } => expressions.iter().any(ast_has_await),
        AstExpr::TaggedTemplate {
            tag, expressions, ..
        } => ast_has_await(tag) || expressions.iter().any(ast_has_await),
        AstExpr::ImportCall {
            source, options, ..
        } => ast_has_await(source) || options.as_ref().is_some_and(|o| ast_has_await(o)),
        _ => false,
    }
}

fn stmt_has_await(stmt: &AstStmt) -> bool {
    match stmt {
        AstStmt::Block { body, .. } => body.iter().any(stmt_has_await),
        AstStmt::Expression { expr, .. } => ast_has_await(expr),
        AstStmt::Return {
            argument: Some(e), ..
        }
        | AstStmt::Throw { argument: e, .. } => ast_has_await(e),
        AstStmt::Let { init: Some(e), .. } => ast_has_await(e),
        AstStmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            ast_has_await(test)
                || stmt_has_await(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_has_await(a))
        }
        AstStmt::While { test, body, .. } => ast_has_await(test) || stmt_has_await(body),
        AstStmt::DoWhile { body, test, .. } => stmt_has_await(body) || ast_has_await(test),
        AstStmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            init.as_ref().is_some_and(|s| stmt_has_await(s))
                || test.as_ref().is_some_and(ast_has_await)
                || update.as_ref().is_some_and(ast_has_await)
                || stmt_has_await(body)
        }
        AstStmt::ForIn { left, right, body, .. } | AstStmt::ForOf { left, right, body, .. } => {
            stmt_has_await(left) || ast_has_await(right) || stmt_has_await(body)
        }
        AstStmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            stmt_has_await(block)
                || handler.as_ref().is_some_and(|h| stmt_has_await(h))
                || finalizer.as_ref().is_some_and(|f| stmt_has_await(f))
        }
        AstStmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            ast_has_await(discriminant)
                || cases.iter().any(|c| {
                    c.test.as_ref().is_some_and(ast_has_await) || c.body.iter().any(stmt_has_await)
                })
        }
        AstStmt::Labeled { body, .. } => stmt_has_await(body),
        AstStmt::With { object, body, .. } => ast_has_await(object) || stmt_has_await(body),
        _ => false,
    }
}

/// Descriptor for SetFunctionName / CreateDataProperty helpers.
fn data_prop_desc(value: Expr, writable: bool, enumerable: bool, configurable: bool) -> Expr {
    Expr::Object {
        properties: vec![
            ObjectProp::Property {
                key: ObjectPropKey::Static("value".into()),
                value,
            },
            ObjectProp::Property {
                key: ObjectPropKey::Static("writable".into()),
                value: Expr::Boolean {
                    value: writable,
                    ty: Type::Boolean,
                },
            },
            ObjectProp::Property {
                key: ObjectPropKey::Static("enumerable".into()),
                value: Expr::Boolean {
                    value: enumerable,
                    ty: Type::Boolean,
                },
            },
            ObjectProp::Property {
                key: ObjectPropKey::Static("configurable".into()),
                value: Expr::Boolean {
                    value: configurable,
                    ty: Type::Boolean,
                },
            },
        ],
        ty: Type::Object,
    }
}

/// `Object.defineProperty(fn, "name", { value, writable: false, enumerable: false, configurable: true })`
/// — ECMA-262 SetFunctionName (used for class NamedEvaluation, E19.31).
fn set_function_name_stmt(local: LocalId, name: &str) -> Stmt {
    Stmt::Expr {
        expr: object_method_call(
            "defineProperty",
            vec![
                Arg::Expr(Expr::Local {
                    id: local,
                    ty: Type::Function,
                }),
                Arg::Expr(Expr::String {
                    value: "name".into(),
                    ty: Type::String,
                }),
                Arg::Expr(data_prop_desc(
                    Expr::String {
                        value: name.into(),
                        ty: Type::String,
                    },
                    false,
                    false,
                    true,
                )),
            ],
        ),
    }
}

/// NamedEvaluation: `((f) => (Object.defineProperty(f,"name",…), f))(fe)`.
/// Used for anonymous function/arrow field initializers (E19.82.04).
fn set_function_name_on_expr(ctx: &mut LowerCtx, fe: Expr, name: &str) -> Expr {
    let tmp = ctx.alloc_synthetic_local(
        format!("__drac_fnname_{}", ctx.next_synth_id),
        Type::Function,
    );
    let set_name = object_method_call(
        "defineProperty",
        vec![
            Arg::Expr(local_expr(tmp)),
            Arg::Expr(Expr::String {
                value: "name".into(),
                ty: Type::String,
            }),
            Arg::Expr(data_prop_desc(
                Expr::String {
                    value: name.into(),
                    ty: Type::String,
                },
                false,
                false,
                true,
            )),
        ],
    );
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(tmp),
                default: None,
                rest: false,
            }],
            body: vec![Stmt::Return {
                value: Some(Expr::Binary {
                    left: Box::new(set_name),
                    op: BinaryOp::Comma,
                    right: Box::new(local_expr(tmp)),
                    ty: Type::Function,
                }),
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(fe)],
        optional: false,
        ty: Type::Function,
    }
}

/// CreateDataPropertyOrThrow via defineProperty (throws on non-writable `prototype`, E19.82.04).
fn create_data_property_or_throw(object: Expr, key: Expr, value: Expr) -> Expr {
    object_method_call(
        "defineProperty",
        vec![
            Arg::Expr(object),
            Arg::Expr(key),
            Arg::Expr(data_prop_desc(value, true, true, true)),
        ],
    )
}

fn object_key_private_name(key: &draconic_ast::ObjectKey) -> Option<&str> {
    match key {
        draconic_ast::ObjectKey::Ident(id) => Some(id.name.as_str()),
        _ => None,
    }
}

/// `obj.prop` member read helper.
fn member_prop(object: Expr, prop: &str, ty: Type) -> Expr {
    Expr::Member {
        object: Box::new(object),
        property: Box::new(Expr::String {
            value: prop.into(),
            ty: Type::String,
        }),
        computed: false,
        optional: false,
        ty,
    }
}

/// `Object.method(...)` call helper.
fn object_method_call(method: &str, args: Vec<Arg>) -> Expr {
    Expr::Call {
        callee: Box::new(member_prop(
            Expr::IdentName {
                name: "Object".into(),
                ty: Type::Object,
            },
            method,
            Type::Function,
        )),
        args,
        optional: false,
        ty: Type::Any,
    }
}

/// ClassDefinitionEvaluation heritage checks (E19.82.02):
/// - `null` is allowed (protoParent = null)
/// - else IsConstructor(superclass) must be true → TypeError
/// - else Get(superclass, "prototype") must be Object or Null → TypeError
///
/// Emits roughly:
/// ```js
/// if (parent !== null) {
///   try { Reflect.construct(function () {}, [], parent); }
///   catch { throw new TypeError("…not a constructor or null"); }
///   let __p = parent.prototype;
///   if (__p !== null && typeof __p !== "object" && typeof __p !== "function")
///     throw new TypeError("…valid prototype property");
/// }
/// ```
fn heritage_validation_stmts(parent: Expr) -> Vec<Stmt> {
    let is_null = Expr::Binary {
        left: Box::new(parent.clone()),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::Null { ty: Type::Any }),
        ty: Type::Boolean,
    };
    let not_null = Expr::Unary {
        op: UnaryOp::Not,
        arg: Box::new(is_null),
        ty: Type::Boolean,
    };

    // Reflect.construct(function () {}, [], parent) — throws if !IsConstructor(parent)
    let empty_ctor = Expr::Function {
        name: None,
        params: Vec::new(),
        body: Vec::new(),
        is_async: false,
        is_generator: false,
        is_arrow: false,
        is_method: false,
        ty: Type::Function,
    };
    let is_ctor_probe = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(Expr::IdentName {
                name: "Reflect".into(),
                ty: Type::Object,
            }),
            property: Box::new(Expr::String {
                value: "construct".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![
            Arg::Expr(empty_ctor),
            Arg::Expr(Expr::Array {
                elements: Vec::new(),
                ty: Type::Any,
            }),
            Arg::Expr(parent.clone()),
        ],
        optional: false,
        ty: Type::Any,
    };

    let throw_not_ctor = Stmt::Throw {
        value: Expr::New {
            callee: Box::new(Expr::IdentName {
                name: "TypeError".into(),
                ty: Type::Function,
            }),
            args: vec![Arg::Expr(Expr::String {
                value: "Class extends value is not a constructor or null".into(),
                ty: Type::String,
            })],
            ty: Type::Any,
        },
    };

    let try_is_ctor = Stmt::Try {
        block: vec![Stmt::Expr {
            expr: is_ctor_probe,
        }],
        handler_param: None,
        handler: Some(vec![throw_not_ctor]),
        finalizer: None,
    };

    // prototype must be Object or Null (functions count as Object)
    let proto = member_prop(parent, "prototype", Type::Any);
    let proto_is_null = Expr::Binary {
        left: Box::new(proto.clone()),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::Null { ty: Type::Any }),
        ty: Type::Boolean,
    };
    let typeof_proto = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(proto.clone()),
        ty: Type::String,
    };
    let proto_is_object = Expr::Binary {
        left: Box::new(typeof_proto.clone()),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::String {
            value: "object".into(),
            ty: Type::String,
        }),
        ty: Type::Boolean,
    };
    let typeof_proto_fn = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(proto),
        ty: Type::String,
    };
    let proto_is_function = Expr::Binary {
        left: Box::new(typeof_proto_fn),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::String {
            value: "function".into(),
            ty: Type::String,
        }),
        ty: Type::Boolean,
    };
    let proto_ok = Expr::Binary {
        left: Box::new(proto_is_null),
        op: BinaryOp::Or,
        right: Box::new(Expr::Binary {
            left: Box::new(proto_is_object),
            op: BinaryOp::Or,
            right: Box::new(proto_is_function),
            ty: Type::Boolean,
        }),
        ty: Type::Boolean,
    };
    let proto_bad = Expr::Unary {
        op: UnaryOp::Not,
        arg: Box::new(proto_ok),
        ty: Type::Boolean,
    };
    let throw_bad_proto = Stmt::Throw {
        value: Expr::New {
            callee: Box::new(Expr::IdentName {
                name: "TypeError".into(),
                ty: Type::Function,
            }),
            args: vec![Arg::Expr(Expr::String {
                value: "Class extends value does not have valid prototype property"
                    .into(),
                ty: Type::String,
            })],
            ty: Type::Any,
        },
    };
    let check_proto = Stmt::If {
        test: proto_bad,
        consequent: Box::new(throw_bad_proto),
        alternate: None,
    };

    vec![Stmt::If {
        test: not_null,
        consequent: Box::new(Stmt::Block {
            body: vec![try_is_ctor, check_proto],
        }),
        alternate: None,
    }]
}

/// `(parent === null) ? null : parent.prototype` — `extends null` super base (E19.72).
fn parent_instance_super_base(parent: Expr) -> Expr {
    Expr::Conditional {
        test: Box::new(Expr::Binary {
            left: Box::new(parent.clone()),
            op: BinaryOp::EqEqEq,
            right: Box::new(Expr::Null { ty: Type::Any }),
            ty: Type::Boolean,
        }),
        consequent: Box::new(Expr::Null { ty: Type::Any }),
        alternate: Box::new(member_prop(parent, "prototype", Type::Any)),
        ty: Type::Any,
    }
}

/// Install a class method/accessor so the function keeps a real [[HomeObject]] and
/// correct super base — required for `super.x =`, compound assign, null-proto, and
/// `eval('super…')` in derived methods (E19.72).
///
/// Emits roughly:
/// ```js
/// let __k = key; // once — preserves yield/await side effects (E19.78)
/// Object.defineProperty(target, __k, ((d) => {
///   d.enumerable = false;
///   if (d.get === undefined) delete d.get;
///   if (d.set === undefined) delete d.set;
///   return d;
/// })(Object.getOwnPropertyDescriptor({ __proto__: homeProto, … }, __k)));
/// ```
/// Deleting absent get/set keeps separate get-then-set installs from wiping each other.
///
/// `home_prop` must use `ObjectPropKey::Computed(Local(key_temp))` or Static matching
/// `key_expr` — callers pass a key already bound via the returned temp binding.
fn define_class_element_with_home(
    ctx: &mut LowerCtx,
    target: Expr,
    key_expr: Expr,
    home_prop: ObjectProp,
    home_proto: Option<Expr>,
) -> Vec<Stmt> {
    // Evaluate key once (yield/await/ToPropertyKey side effects) (E19.78).
    // Unique name: JS emit uses local names, not ids.
    let key_id = ctx.alloc_synthetic_local(
        format!("__drac_ck_{}", ctx.next_synth_id),
        Type::Any,
    );
    let key_local = Expr::Local {
        id: key_id,
        ty: Type::Any,
    };
    let mut out = vec![Stmt::Declare {
        local: key_id,
        init: Some(key_expr),
        kind: BindingKind::Let,
    }];
    // Rewrite home prop key to the temp so the object literal does not re-eval.
    let home_prop = match home_prop {
        ObjectProp::Property { value, .. } => ObjectProp::Property {
            key: ObjectPropKey::Computed(key_local.clone()),
            value,
        },
        ObjectProp::Accessor { kind, value, .. } => ObjectProp::Accessor {
            kind,
            key: ObjectPropKey::Computed(key_local.clone()),
            value,
        },
        other => other,
    };
    let mut home_props = Vec::new();
    if let Some(proto) = home_proto {
        home_props.push(ObjectProp::Property {
            key: ObjectPropKey::Static("__proto__".into()),
            value: proto,
        });
    }
    home_props.push(home_prop);
    let home = Expr::Object {
        properties: home_props,
        ty: Type::Object,
    };
    let gopd = object_method_call(
        "getOwnPropertyDescriptor",
        vec![Arg::Expr(home), Arg::Expr(key_local.clone())],
    );
    // ((d) => (d.enumerable = false, d.get === void 0 && delete d.get, d.set === void 0 && delete d.set, d))(gopd)
    let d_id = ctx.alloc_synthetic_local("__drac_desc".into(), Type::Any);
    let d_local = Expr::Local {
        id: d_id,
        ty: Type::Any,
    };
    let set_enumerable = Expr::Assign {
        target: AssignTarget::Member {
            object: Box::new(d_local.clone()),
            property: Box::new(Expr::String {
                value: "enumerable".into(),
                ty: Type::String,
            }),
            computed: false,
        },
        op: AssignOp::Eq,
        value: Box::new(Expr::Boolean {
            value: false,
            ty: Type::Boolean,
        }),
        ty: Type::Any,
    };
    let undef = Expr::Unary {
        op: UnaryOp::Void,
        arg: Box::new(Expr::Number {
            raw: "0".into(),
            ty: Type::Number,
        }),
        ty: Type::Any,
    };
    let delete_if_undef = |prop: &str| {
        let get_prop = member_prop(d_local.clone(), prop, Type::Any);
        let is_undef = Expr::Binary {
            left: Box::new(get_prop),
            op: BinaryOp::EqEqEq,
            right: Box::new(undef.clone()),
            ty: Type::Boolean,
        };
        let del = Expr::Unary {
            op: UnaryOp::Delete,
            arg: Box::new(member_prop(d_local.clone(), prop, Type::Any)),
            ty: Type::Boolean,
        };
        Expr::Binary {
            left: Box::new(is_undef),
            op: BinaryOp::And,
            right: Box::new(del),
            ty: Type::Any,
        }
    };
    let clean = Expr::Binary {
        left: Box::new(set_enumerable),
        op: BinaryOp::Comma,
        right: Box::new(Expr::Binary {
            left: Box::new(delete_if_undef("get")),
            op: BinaryOp::Comma,
            right: Box::new(Expr::Binary {
                left: Box::new(delete_if_undef("set")),
                op: BinaryOp::Comma,
                right: Box::new(d_local.clone()),
                ty: Type::Any,
            }),
            ty: Type::Any,
        }),
        ty: Type::Any,
    };
    let desc = Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(d_id),
                default: None,
                rest: false,
            }],
            body: vec![Stmt::Return {
                value: Some(clean),
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(gopd)],
        optional: false,
        ty: Type::Any,
    };
    out.push(Stmt::Expr {
        expr: object_method_call(
            "defineProperty",
            vec![Arg::Expr(target), Arg::Expr(key_local), Arg::Expr(desc)],
        ),
    });
    out
}

fn lower_object_prop_key(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    key: &draconic_ast::ObjectKey,
    super_class: Option<&AstExpr>,
) -> ObjectPropKey {
    match key {
        draconic_ast::ObjectKey::Ident(id) => ObjectPropKey::Static(id.name.clone().into()),
        draconic_ast::ObjectKey::String(s) => ObjectPropKey::Static(s.value.clone()),
        draconic_ast::ObjectKey::Computed(expr) => {
            ObjectPropKey::Computed(lower_expr(checked, ctx, expr, super_class))
        }
    }
}

/// Class bodies are strict; method-form install is sloppy unless we inject a directive (E19.72).
fn with_use_strict(mut body: Vec<Stmt>) -> Vec<Stmt> {
    body.insert(
        0,
        Stmt::Expr {
            expr: Expr::String {
                value: "use strict".into(),
                ty: Type::String,
            },
        },
    );
    body
}

fn undef_expr() -> Expr {
    Expr::IdentName {
        name: "undefined".into(),
        ty: Type::Any,
    }
}

fn local_expr(id: LocalId) -> Expr {
    Expr::Local {
        id,
        ty: Type::Any,
    }
}

/// `(() => { throw new ReferenceError(msg); })()`
fn throw_reference_error_expr(msg: &str) -> Expr {
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: Vec::new(),
            body: vec![Stmt::Throw {
                value: Expr::New {
                    callee: Box::new(Expr::IdentName {
                        name: "ReferenceError".into(),
                        ty: Type::Function,
                    }),
                    args: vec![Arg::Expr(Expr::String {
                        value: msg.into(),
                        ty: Type::String,
                    })],
                    ty: Type::Any,
                },
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: Vec::new(),
        optional: false,
        ty: Type::Any,
    }
}

/// ES GetThisBinding for derived ctor: uninitialized → ReferenceError.
fn assert_derived_this(this_id: LocalId) -> Expr {
    Expr::Conditional {
        test: Box::new(Expr::Binary {
            left: Box::new(local_expr(this_id)),
            op: BinaryOp::EqEqEq,
            right: Box::new(undef_expr()),
            ty: Type::Boolean,
        }),
        consequent: Box::new(throw_reference_error_expr(
            "Must call super constructor in derived class before accessing 'this' or returning from it",
        )),
        alternate: Box::new(local_expr(this_id)),
        ty: Type::Any,
    }
}

/// [[Construct]] completion for derived constructors (E19.82.03):
/// object return → that object; undefined → assert this; else TypeError.
fn possible_constructor_return(this_id: LocalId, value: Option<Expr>) -> Expr {
    let v = value.unwrap_or_else(undef_expr);
    // (v === undefined) ? assertThis(_this)
    //   : (v !== null && (typeof v === "object" || typeof v === "function")) ? v
    //   : throw TypeError
    let is_undef = Expr::Binary {
        left: Box::new(v.clone()),
        op: BinaryOp::EqEqEq,
        right: Box::new(undef_expr()),
        ty: Type::Boolean,
    };
    let is_null = Expr::Binary {
        left: Box::new(v.clone()),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::Null { ty: Type::Any }),
        ty: Type::Boolean,
    };
    let typeof_v = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(v.clone()),
        ty: Type::String,
    };
    let is_object_type = Expr::Binary {
        left: Box::new(Expr::Binary {
            left: Box::new(typeof_v.clone()),
            op: BinaryOp::EqEqEq,
            right: Box::new(Expr::String {
                value: "object".into(),
                ty: Type::String,
            }),
            ty: Type::Boolean,
        }),
        op: BinaryOp::Or,
        right: Box::new(Expr::Binary {
            left: Box::new(typeof_v),
            op: BinaryOp::EqEqEq,
            right: Box::new(Expr::String {
                value: "function".into(),
                ty: Type::String,
            }),
            ty: Type::Boolean,
        }),
        ty: Type::Boolean,
    };
    let is_object = Expr::Binary {
        left: Box::new(Expr::Unary {
            op: UnaryOp::Not,
            arg: Box::new(is_null),
            ty: Type::Boolean,
        }),
        op: BinaryOp::And,
        right: Box::new(is_object_type),
        ty: Type::Boolean,
    };
    Expr::Conditional {
        test: Box::new(is_undef),
        consequent: Box::new(assert_derived_this(this_id)),
        alternate: Box::new(Expr::Conditional {
            test: Box::new(is_object),
            consequent: Box::new(v),
            alternate: Box::new(throw_type_error_expr(
                "Derived constructors may only return object or undefined",
            )),
            ty: Type::Any,
        }),
        ty: Type::Any,
    }
}

/// `super(...args)` in derived ctor → Reflect.construct + field inits + bind this.
/// Spec order: Construct first, then BindThisValue (double-super throws after parent runs).
fn derived_super_call_expr(ctx: &mut LowerCtx, args: Vec<Arg>) -> Expr {
    let this_id = ctx.derived_this.expect("derived_this");
    let super_id = ctx.derived_super.expect("derived_super");
    let inits = ctx.derived_super_inits.clone();
    let args_id = ctx.alloc_synthetic_local("__drac_sargs".into(), Type::Any);
    let result_id = ctx.alloc_synthetic_local("__drac_sres".into(), Type::Any);
    let mut body = Vec::new();
    // let result = Reflect.construct(Super, args, new.target) — always (E19.82.05 double-super).
    let reflect = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(Expr::IdentName {
                name: "Reflect".into(),
                ty: Type::Object,
            }),
            property: Box::new(Expr::String {
                value: "construct".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![
            Arg::Expr(local_expr(super_id)),
            Arg::Expr(local_expr(args_id)),
            Arg::Expr(Expr::NewTarget { ty: Type::Any }),
        ],
        optional: false,
        ty: Type::Any,
    };
    body.push(Stmt::Declare {
        local: result_id,
        init: Some(reflect),
        kind: BindingKind::Let,
    });
    // if (_this !== undefined) throw ReferenceError (already initialized)
    body.push(Stmt::If {
        test: Expr::Binary {
            left: Box::new(local_expr(this_id)),
            op: BinaryOp::NotEqEq,
            right: Box::new(undef_expr()),
            ty: Type::Boolean,
        },
        consequent: Box::new(Stmt::Throw {
            value: Expr::New {
                callee: Box::new(Expr::IdentName {
                    name: "ReferenceError".into(),
                    ty: Type::Function,
                }),
                args: vec![Arg::Expr(Expr::String {
                    value: "Super constructor may only be called once".into(),
                    ty: Type::String,
                })],
                ty: Type::Any,
            },
        }),
        alternate: None,
    });
    // _this = result; field inits once
    body.push(Stmt::Expr {
        expr: Expr::Assign {
            target: AssignTarget::Local(this_id),
            op: AssignOp::Eq,
            value: Box::new(local_expr(result_id)),
            ty: Type::Any,
        },
    });
    for init in inits {
        body.push(Stmt::Expr { expr: init });
    }
    body.push(Stmt::Return {
        value: Some(local_expr(this_id)),
    });
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(args_id),
                default: None,
                rest: true,
            }],
            body,
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args,
        optional: false,
        ty: Type::Any,
    }
}

/// Property key expression for `Object.defineProperty` / member name (always a value expr).
fn lower_object_key_name_expr(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    key: &draconic_ast::ObjectKey,
    super_class: Option<&AstExpr>,
) -> Expr {
    match key {
        draconic_ast::ObjectKey::Ident(id) => Expr::String {
            value: id.name.clone().into(),
            ty: Type::String,
        },
        draconic_ast::ObjectKey::String(s) => Expr::String {
            value: s.value.clone(),
            ty: Type::String,
        },
        draconic_ast::ObjectKey::Computed(expr) => lower_expr(checked, ctx, expr, super_class),
    }
}

/// Member property + computed flag for assignment targets.
fn lower_object_key_prop(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    key: &draconic_ast::ObjectKey,
    super_class: Option<&AstExpr>,
) -> (Expr, bool) {
    match key {
        draconic_ast::ObjectKey::Ident(id) => (
            Expr::String {
                value: id.name.clone().into(),
                ty: Type::String,
            },
            false,
        ),
        draconic_ast::ObjectKey::String(s) => (
            Expr::String {
                value: s.value.clone(),
                ty: Type::String,
            },
            false,
        ),
        draconic_ast::ObjectKey::Computed(expr) => {
            (lower_expr(checked, ctx, expr, super_class), true)
        }
    }
}

fn lower_class_local(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    local: LocalId,
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
    // NamedEvaluation name for anonymous class expressions (E19.31).
    name_hint: Option<&str>,
) -> Vec<Stmt> {
    let mut ctor_params = Vec::new();
    let mut ctor_body_ast: Option<&AstStmt> = None;
    let mut methods: Vec<(
        &draconic_ast::ObjectKey,
        &Vec<draconic_ast::Param>,
        &AstStmt,
        bool,
        bool,
        bool,
        bool,
    )> = Vec::new();
    let mut accessors: Vec<(
        AccessorKind,
        &draconic_ast::ObjectKey,
        &Vec<draconic_ast::Param>,
        &AstStmt,
        bool,
        bool,
    )> = Vec::new();
    // Instance fields: (key, value, is_private, precomputed_key_local for public computed).
    let mut instance_fields: Vec<(
        &draconic_ast::ObjectKey,
        Option<&AstExpr>,
        bool,
        Option<LocalId>,
    )> = Vec::new();
    // Static fields and static blocks in source order (E18.41).
    enum StaticInit<'a> {
        Field {
            key: &'a draconic_ast::ObjectKey,
            value: Option<&'a AstExpr>,
            is_private: bool,
            /// Public computed key temp evaluated in source order (E19.82.04).
            computed_key: Option<LocalId>,
        },
        Block(&'a AstStmt),
    }
    let mut static_inits: Vec<StaticInit<'_>> = Vec::new();
    // Computed public field keys (instance + static) in source order — evaluated at
    // class definition before any field initializers (E19.82.04 intercalated keys).
    let mut computed_field_key_locals: Vec<(LocalId, Expr)> = Vec::new();

    for el in elements {
        match el {
            ClassElement::Constructor { params, body, .. } => {
                ctor_params = lower_params(checked, ctx, params, super_class);
                ctor_body_ast = Some(body.as_ref());
            }
            ClassElement::Method {
                key: method_key,
                params,
                body,
                is_static,
                is_async,
                is_generator,
                is_private,
                ..
            } => {
                methods.push((
                    method_key,
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
                key: acc_key,
                params,
                body,
                is_static,
                is_private,
                ..
            } => {
                accessors.push((
                    *kind,
                    acc_key,
                    params,
                    body.as_ref(),
                    *is_static,
                    *is_private,
                ));
            }
            ClassElement::Field {
                key: field_key,
                value,
                is_static,
                is_private,
                ..
            } => {
                let v = value.as_ref();
                // Public computed keys: ToPropertyKey at class eval, source order (E19.82.04).
                let computed_key = if !*is_private
                    && matches!(field_key, draconic_ast::ObjectKey::Computed(_))
                {
                    let key_id = ctx.alloc_synthetic_local(
                        format!(
                            "__drac_cfk_{}_{}",
                            local.0,
                            computed_field_key_locals.len()
                        ),
                        Type::Any,
                    );
                    let key_expr =
                        lower_object_key_name_expr(checked, ctx, field_key, super_class);
                    // Reflect.ownKeys({[key]:1})[0] forces ToPropertyKey.
                    let to_key = Expr::Member {
                        object: Box::new(Expr::Call {
                            callee: Box::new(Expr::Member {
                                object: Box::new(Expr::IdentName {
                                    name: "Reflect".into(),
                                    ty: Type::Object,
                                }),
                                property: Box::new(Expr::String {
                                    value: "ownKeys".into(),
                                    ty: Type::String,
                                }),
                                computed: false,
                                optional: false,
                                ty: Type::Function,
                            }),
                            args: vec![Arg::Expr(Expr::Object {
                                properties: vec![ObjectProp::Property {
                                    key: ObjectPropKey::Computed(key_expr),
                                    value: Expr::Number {
                                        raw: "1".into(),
                                        ty: Type::Number,
                                    },
                                }],
                                ty: Type::Object,
                            })],
                            optional: false,
                            ty: Type::Any,
                        }),
                        property: Box::new(Expr::Number {
                            raw: "0".into(),
                            ty: Type::Number,
                        }),
                        computed: true,
                        optional: false,
                        ty: Type::Any,
                    };
                    computed_field_key_locals.push((key_id, to_key));
                    Some(key_id)
                } else {
                    None
                };
                if *is_static {
                    static_inits.push(StaticInit::Field {
                        key: field_key,
                        value: v,
                        is_private: *is_private,
                        computed_key,
                    });
                } else {
                    instance_fields.push((field_key, v, *is_private, computed_key));
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
    let mut add_private_wm = |fname: &str| {
        if private_map.contains_key(fname) {
            return;
        }
        let wm_name = format!("__drac_pf_{}_{}", local.0, fname);
        let wm_id = ctx.alloc_synthetic_local(wm_name, Type::Any);
        private_map.insert(fname.to_string(), wm_id);
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
    for (fkey, _, is_private, _) in &instance_fields {
        if *is_private {
            if let Some(n) = object_key_private_name(fkey) {
                add_private_wm(n);
            }
        }
    }
    for init in &static_inits {
        if let StaticInit::Field {
            key: fkey,
            is_private,
            ..
        } = init
        {
            if *is_private {
                if let Some(n) = object_key_private_name(fkey) {
                    add_private_wm(n);
                }
            }
        }
    }

    // Private methods: synthetic function locals (E18.37 instance; E18.38 static). Bodies lowered after maps are live.
    let mut private_method_map: HashMap<String, LocalId> = HashMap::new();
    let mut private_method_meta: Vec<(
        LocalId,
        String,
        &Vec<draconic_ast::Param>,
        &AstStmt,
        bool,
        bool,
    )> = Vec::new();
    let mut private_brand_map: HashMap<String, LocalId> = HashMap::new();
    let mut private_brand_decls: Vec<Stmt> = Vec::new();
    let mut instance_brands: Vec<LocalId> = Vec::new();
    let mut static_brands: Vec<LocalId> = Vec::new();
    for (method_key, params, body, is_static, is_async, is_generator, is_private) in &methods {
        if !*is_private {
            continue;
        }
        let Some(method_name) = object_key_private_name(method_key) else {
            continue;
        };
        if private_method_map.contains_key(method_name) {
            continue;
        }
        let fn_name = format!("__drac_pm_{}_{}", local.0, method_name);
        let fn_id = ctx.alloc_synthetic_local(fn_name, Type::Function);
        private_method_map.insert(method_name.to_string(), fn_id);
        private_method_meta.push((
            fn_id,
            method_name.to_string(),
            params,
            body,
            *is_async,
            *is_generator,
        ));
        ensure_private_brand(
            ctx,
            local,
            &mut private_brand_map,
            &mut private_brand_decls,
            &mut instance_brands,
            &mut static_brands,
            method_name,
            *is_static,
        );
    }

    // Private accessors: synthetic get/set function locals (E18.39).
    let mut private_accessor_map: HashMap<String, (Option<LocalId>, Option<LocalId>)> =
        HashMap::new();
    let mut private_accessor_meta: Vec<(
        LocalId,
        String,
        &Vec<draconic_ast::Param>,
        &AstStmt,
    )> = Vec::new();
    for (kind, acc_key, params, body, is_static, is_private) in &accessors {
        if !*is_private {
            continue;
        }
        let Some(acc_name) = object_key_private_name(acc_key) else {
            continue;
        };
        let entry = private_accessor_map
            .entry(acc_name.to_string())
            .or_insert((None, None));
        let tag = match kind {
            AccessorKind::Get => "g",
            AccessorKind::Set => "s",
        };
        let fn_name = format!("__drac_pa{}_{}_{}", tag, local.0, acc_name);
        let fn_id = ctx.alloc_synthetic_local(fn_name, Type::Function);
        match kind {
            AccessorKind::Get => entry.0 = Some(fn_id),
            AccessorKind::Set => entry.1 = Some(fn_id),
        }
        let display = match kind {
            AccessorKind::Get => format!("get #{acc_name}"),
            AccessorKind::Set => format!("set #{acc_name}"),
        };
        private_accessor_meta.push((fn_id, display, params, body));
        ensure_private_brand(
            ctx,
            local,
            &mut private_brand_map,
            &mut private_brand_decls,
            &mut instance_brands,
            &mut static_brands,
            acc_name,
            *is_static,
        );
    }

    // Nested classes inherit outer private names; inner same-name bindings fully
    // shadow any outer kind (field/method/accessor/brand) — E19.36 / E19.82.07.
    let prev_privates = ctx.private_fields.clone();
    let prev_private_methods = ctx.private_methods.clone();
    let prev_private_accessors = ctx.private_accessors.clone();
    let prev_private_brands = ctx.private_brands.clone();
    let mut shadowed: HashSet<String> = HashSet::new();
    shadowed.extend(private_map.keys().cloned());
    shadowed.extend(private_method_map.keys().cloned());
    shadowed.extend(private_accessor_map.keys().cloned());
    for name in &shadowed {
        ctx.private_fields.remove(name);
        ctx.private_methods.remove(name);
        ctx.private_accessors.remove(name);
        ctx.private_brands.remove(name);
    }
    for (k, v) in private_map {
        ctx.private_fields.insert(k, v);
    }
    for (k, v) in private_method_map {
        ctx.private_methods.insert(k, v);
    }
    for (k, v) in private_accessor_map {
        ctx.private_accessors.insert(k, v);
    }
    for (k, v) in private_brand_map {
        ctx.private_brands.insert(k, v);
    }

    let mut private_method_fns: Vec<Stmt> = Vec::new();
    for (fn_id, method_name, params, body, is_async, is_generator) in private_method_meta {
        private_method_fns.push(Stmt::Function {
            local: fn_id,
            params: lower_params(checked, ctx, params, super_class),
            body: lower_fn_body(checked, ctx, body, super_class),
            is_async,
            is_generator,
        });
        // SetFunctionName(closure, PrivateName) → "#description" (E19.82).
        private_method_fns.push(set_function_name_stmt(
            fn_id,
            &format!("#{method_name}"),
        ));
    }
    for (fn_id, display_name, params, body) in private_accessor_meta {
        private_method_fns.push(Stmt::Function {
            local: fn_id,
            params: lower_params(checked, ctx, params, super_class),
            body: lower_fn_body(checked, ctx, body, super_class),
            is_async: false,
            is_generator: false,
        });
        private_method_fns.push(set_function_name_stmt(fn_id, &display_name));
    }

    // Derived constructors use a TDZ `this` temp + Reflect.construct (E19.82.03).
    // Super expression is evaluated once at class def (not inside ctor) so TLA
    // `extends fn(await x)` keeps `await` at module top-level. Always bind the
    // heritage value so IsConstructor / prototype checks run once (E19.82.02).
    let is_derived = super_class.is_some();
    let default_derived_ctor = ctor_body_ast.is_none() && is_derived;
    let derived_this_id = if is_derived {
        Some(ctx.alloc_synthetic_local(
            format!("__drac_this_{}", local.0),
            Type::Any,
        ))
    } else {
        None
    };
    let super_local_id = if is_derived {
        Some(ctx.alloc_synthetic_local(
            format!("__drac_super_{}", local.0),
            Type::Any,
        ))
    } else {
        None
    };
    // Receiver for field/brand inits: derived `_this` temp or bare `this` (base).
    let ctor_this = || {
        if let Some(id) = derived_this_id {
            local_expr(id)
        } else {
            Expr::This { ty: Type::Any }
        }
    };

    // Instance field inits reference computed key temps allocated in source order above.
    let mut instance_init_exprs: Vec<Expr> = Vec::new();
    // Brands before fields (InitializeInstanceElements).
    for brand in &instance_brands {
        instance_init_exprs.push(private_brand_add(ctx, *brand, ctor_this()));
    }
    // Instance SuperProperty home base: Parent.prototype or Object.prototype (E19.82.05).
    let instance_super_home = match super_local_id {
        Some(sid) => parent_instance_super_base(local_expr(sid)),
        None => match super_class {
            Some(sc) => parent_instance_super_base(lower_expr(checked, ctx, sc, None)),
            None => member_prop(
                Expr::IdentName {
                    name: "Object".into(),
                    ty: Type::Function,
                },
                "prototype",
                Type::Any,
            ),
        },
    };
    for (fkey, value, is_private, computed_key) in &instance_fields {
        let name_hint = field_name_hint(fkey, *is_private);
        // Always method HomeObject so field-init direct eval gets SuperProperty /
        // new.target (E19.82.05 / E19.82.06). Clear derived temps so Super stays bare.
        let prev_derived_this = ctx.derived_this.take();
        let prev_derived_super = ctx.derived_super.take();
        let prev_field_init = ctx.in_field_init;
        ctx.in_field_init = true;
        let init = match value {
            Some(v) => lower_field_init_expr(checked, ctx, v, None, name_hint.as_deref()),
            None => undef_expr(),
        };
        ctx.in_field_init = prev_field_init;
        // Bare `this` inside method; .call(receiver) supplies the instance.
        let receiver = Expr::This { ty: Type::Any };
        let assign_expr = if *is_private {
            let pname = object_key_private_name(fkey).expect("private field name");
            let wm = *ctx
                .private_fields
                .get(pname)
                .expect("private field WeakMap");
            // PrivateFieldAdd: non-extensible / already-present → TypeError (E19.82.09).
            private_field_add(ctx, wm, receiver, init)
        } else if let Some(key_id) = computed_key {
            Expr::Assign {
                target: AssignTarget::Member {
                    object: Box::new(receiver),
                    property: Box::new(local_expr(*key_id)),
                    computed: true,
                },
                op: AssignOp::Eq,
                value: Box::new(init),
                ty: Type::Any,
            }
        } else {
            let (prop, computed) = lower_object_key_prop(checked, ctx, fkey, None);
            Expr::Assign {
                target: AssignTarget::Member {
                    object: Box::new(receiver),
                    property: Box::new(prop),
                    computed,
                },
                op: AssignOp::Eq,
                value: Box::new(init),
                ty: Type::Any,
            }
        };
        let expr = call_method_with_home(
            instance_super_home.clone(),
            vec![Stmt::Expr {
                expr: assign_expr,
            }],
            ctor_this(),
        );
        instance_init_exprs.push(expr);
        ctx.derived_this = prev_derived_this;
        ctx.derived_super = prev_derived_super;
    }
    // Clear derived ctx after field RHS; re-set for user ctor body below.
    ctx.derived_this = None;
    ctx.derived_super = None;
    ctx.derived_super_inits.clear();

    let ctor_body = if default_derived_ctor {
        let this_id = derived_this_id.expect("derived this temp");
        let super_id = super_local_id.expect("super temp");
        // `_this = Reflect.construct(__drac_super, arguments, new.target)` then inits.
        let reflect_construct = Expr::Call {
            callee: Box::new(Expr::Member {
                object: Box::new(Expr::IdentName {
                    name: "Reflect".into(),
                    ty: Type::Object,
                }),
                property: Box::new(Expr::String {
                    value: "construct".into(),
                    ty: Type::String,
                }),
                computed: false,
                optional: false,
                ty: Type::Function,
            }),
            args: vec![
                Arg::Expr(local_expr(super_id)),
                Arg::Expr(Expr::IdentName {
                    name: "arguments".into(),
                    ty: Type::Any,
                }),
                Arg::Expr(Expr::NewTarget { ty: Type::Any }),
            ],
            optional: false,
            ty: Type::Any,
        };
        let mut body = vec![Stmt::Declare {
            local: this_id,
            init: Some(reflect_construct),
            kind: BindingKind::Let,
        }];
        for init in &instance_init_exprs {
            body.push(Stmt::Expr {
                expr: init.clone(),
            });
        }
        body.push(Stmt::Return {
            value: Some(local_expr(this_id)),
        });
        body
    } else if is_derived {
        let this_id = derived_this_id.expect("derived this temp");
        let super_id = super_local_id.expect("super temp");
        // User-defined derived ctor: TDZ this, super() binds via Reflect.construct.
        ctx.derived_this = Some(this_id);
        ctx.derived_super = Some(super_id);
        ctx.derived_super_inits = instance_init_exprs.clone();
        ctx.derived_ctor_body = true;
        let mut body = vec![Stmt::Declare {
            local: this_id,
            init: None,
            kind: BindingKind::Let,
        }];
        if let Some(ast_body) = ctor_body_ast {
            body.extend(lower_fn_body(checked, ctx, ast_body, super_class));
        }
        ctx.derived_this = None;
        ctx.derived_super = None;
        ctx.derived_super_inits.clear();
        ctx.derived_ctor_body = false;
        // Fall-through: uninitialized this → ReferenceError; else return this.
        body.push(Stmt::Return {
            value: Some(assert_derived_this(this_id)),
        });
        body
    } else {
        // Base class: bare `this`; field inits at start of ctor (after super N/A).
        let mut body = match ctor_body_ast {
            Some(ast_body) => lower_fn_body(checked, ctx, ast_body, super_class),
            None => Vec::new(),
        };
        if !instance_init_exprs.is_empty() {
            let mut new_body = Vec::with_capacity(body.len() + instance_init_exprs.len());
            for init in &instance_init_exprs {
                new_body.push(Stmt::Expr {
                    expr: init.clone(),
                });
            }
            new_body.extend(body);
            body = new_body;
        }
        body
    };

    let mut out = private_wm_decls;
    out.extend(private_brand_decls);
    out.extend(private_method_fns);
    // Evaluate extends once (TLA-safe) and validate IsConstructor + prototype (E19.82.02).
    if let (Some(super_id), Some(sc)) = (super_local_id, super_class) {
        let parent = lower_expr(checked, ctx, sc, None);
        out.push(Stmt::Declare {
            local: super_id,
            init: Some(parent),
            kind: BindingKind::Let,
        });
        out.extend(heritage_validation_stmts(Expr::Local {
            id: super_id,
            ty: Type::Any,
        }));
    }
    // Declare computed field key temps before constructor (referenced from ctor body).
    for (key_id, _) in &computed_field_key_locals {
        out.push(Stmt::Declare {
            local: *key_id,
            init: None,
            kind: BindingKind::Let,
        });
    }
    // E19.57: class name binding is immutable (const-like). Emit `const C = function…`
    // (anonymous FE so body refs resolve to outer const) so `C = …` is a runtime TypeError.
    out.push(Stmt::Declare {
        local,
        init: Some(Expr::Function {
            name: None,
            params: ctor_params,
            body: ctor_body,
            is_async: false,
            is_generator: false,
            is_arrow: false,
            is_method: false,
            ty: Type::Function,
        }),
        kind: BindingKind::Const,
    });
    // NamedEvaluation / class BindingIdentifier → constructor `.name` (E19.31 / E19.57).
    if let Some(hint) = name_hint {
        out.push(set_function_name_stmt(local, hint));
    } else if let Some(sym) = checked.bound.symbols().iter().find(|s| s.id == local) {
        if sym.name != "__class" {
            out.push(set_function_name_stmt(local, sym.name.as_str()));
        }
    }
    // Class constructors: `.prototype` is non-writable/non-enumerable/non-configurable (E19.82.05).
    {
        let proto = member_prop(
            Expr::Local {
                id: local,
                ty: Type::Function,
            },
            "prototype",
            Type::Any,
        );
        out.push(Stmt::Expr {
            expr: object_method_call(
                "defineProperty",
                vec![
                    Arg::Expr(Expr::Local {
                        id: local,
                        ty: Type::Function,
                    }),
                    Arg::Expr(Expr::String {
                        value: "prototype".into(),
                        ty: Type::String,
                    }),
                    Arg::Expr(data_prop_desc(proto, false, false, false)),
                ],
            ),
        });
    }
    // Evaluate computed instance field names at class definition time (E19.53).
    for (key_id, to_key) in computed_field_key_locals {
        out.push(Stmt::Expr {
            expr: Expr::Assign {
                target: AssignTarget::Local(key_id),
                op: AssignOp::Eq,
                value: Box::new(to_key),
                ty: Type::Any,
            },
        });
    }

    // Parent expression for heritage / method home-object super base (E19.72).
    let parent_expr = super_class.map(|sc| {
        if let Some(super_id) = super_local_id {
            Expr::Local {
                id: super_id,
                ty: Type::Any,
            }
        } else {
            lower_expr(checked, ctx, sc, None)
        }
    });

    for (method_key, params, body, is_static, is_async, is_generator, is_private) in methods {
        if is_private {
            // Already emitted as standalone function; not installed on prototype.
            continue;
        }
        // Keep Super + method form so [[HomeObject]] / eval('super…') work (E19.72).
        // SuperCall stays desugared only in constructors (super_class still passed there).
        let method_fn = Expr::Function {
            name: None,
            params: lower_params(checked, ctx, params, None),
            body: with_use_strict(lower_fn_body(checked, ctx, body, None)),
            is_async,
            is_generator,
            is_arrow: false,
            is_method: true,
            ty: Type::Function,
        };
        let class_ref = Expr::Local {
            id: local,
            ty: Type::Function,
        };
        let target_object = if is_static {
            class_ref
        } else {
            member_prop(class_ref, "prototype", Type::Any)
        };
        // Key lowered once; define_class_element_with_home binds to temp (E19.78).
        let prop_name = lower_object_key_name_expr(checked, ctx, method_key, None);
        let home_proto = match (&parent_expr, is_static) {
            (Some(p), true) => Some(p.clone()),
            (Some(p), false) => Some(parent_instance_super_base(p.clone())),
            (None, true) => Some(member_prop(
                Expr::IdentName {
                    name: "Function".into(),
                    ty: Type::Function,
                },
                "prototype",
                Type::Any,
            )),
            (None, false) => None,
        };
        out.extend(define_class_element_with_home(
            ctx,
            target_object,
            prop_name,
            ObjectProp::Property {
                // Placeholder key — rewritten to the once-bound temp inside helper.
                key: ObjectPropKey::Static("".into()),
                value: method_fn,
            },
            home_proto,
        ));
    }

    for (kind, acc_key, params, body, is_static, is_private) in accessors {
        if is_private {
            // Already emitted as standalone function; not installed on prototype.
            continue;
        }
        let accessor_fn = Expr::Function {
            name: None,
            params: lower_params(checked, ctx, params, None),
            body: with_use_strict(lower_fn_body(checked, ctx, body, None)),
            is_async: false,
            is_generator: false,
            is_arrow: false,
            is_method: true,
            ty: Type::Function,
        };
        let class_ref = Expr::Local {
            id: local,
            ty: Type::Function,
        };
        let target_object = if is_static {
            class_ref
        } else {
            member_prop(class_ref, "prototype", Type::Any)
        };
        let prop_name = lower_object_key_name_expr(checked, ctx, acc_key, None);
        let home_proto = match (&parent_expr, is_static) {
            (Some(p), true) => Some(p.clone()),
            (Some(p), false) => Some(parent_instance_super_base(p.clone())),
            (None, true) => Some(member_prop(
                Expr::IdentName {
                    name: "Function".into(),
                    ty: Type::Function,
                },
                "prototype",
                Type::Any,
            )),
            (None, false) => None,
        };
        out.extend(define_class_element_with_home(
            ctx,
            target_object,
            prop_name,
            ObjectProp::Accessor {
                kind,
                key: ObjectPropKey::Static("".into()),
                value: accessor_fn,
            },
            home_proto,
        ));
    }

    if let Some(parent) = parent_expr.as_ref() {
        // extends null → instance [[Prototype]] is null, not null.prototype (E19.72).
        let parent_proto = parent_instance_super_base(parent.clone());
        let child_proto = member_prop(
            Expr::Local {
                id: local,
                ty: Type::Function,
            },
            "prototype",
            Type::Any,
        );
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
                value: Box::new(parent.clone()),
                ty: Type::Any,
            },
        });
    }

    // Brand the constructor for static private methods/accessors (E18.40)
    // before static field/block evaluation so blocks can use private statics.
    for brand in static_brands {
        out.push(Stmt::Expr {
            expr: private_brand_add(
                ctx,
                brand,
                Expr::Local {
                    id: local,
                    ty: Type::Function,
                },
            ),
        });
    }

    // Static fields and static blocks run after the class is fully linked, in order (E18.41).
    // Initializers run as `function(){ return <init>; }.call(Class)` so `this` / direct
    // eval see the constructor (E19.82.04). Arrows inside capture that this.
    for init in static_inits {
        match init {
            StaticInit::Field {
                key: fkey,
                value,
                is_private,
                computed_key,
            } => {
                let name_hint = field_name_hint(fkey, is_private);
                // Always method HomeObject: this = constructor; SuperProperty + field-init
                // direct eval (E19.82.04 / E19.82.05 / E19.82.06).
                let prev_field_init = ctx.in_field_init;
                ctx.in_field_init = true;
                let init_body = match value {
                    Some(v) => lower_field_init_expr(checked, ctx, v, None, name_hint.as_deref()),
                    None => Expr::IdentName {
                        name: "undefined".into(),
                        ty: Type::Any,
                    },
                };
                ctx.in_field_init = prev_field_init;
                let class_ref = Expr::Local {
                    id: local,
                    ty: Type::Function,
                };
                let home_proto = match parent_expr.as_ref() {
                    Some(p) => p.clone(),
                    None => member_prop(
                        Expr::IdentName {
                            name: "Function".into(),
                            ty: Type::Function,
                        },
                        "prototype",
                        Type::Any,
                    ),
                };
                let init_expr = call_method_with_home(
                    home_proto,
                    vec![Stmt::Return {
                        value: Some(init_body),
                    }],
                    class_ref.clone(),
                );
                if is_private {
                    let pname = object_key_private_name(fkey).expect("static private field name");
                    let wm = *ctx
                        .private_fields
                        .get(pname)
                        .expect("static private field WeakMap");
                    // PrivateFieldAdd on constructor (E19.82.09).
                    out.push(Stmt::Expr {
                        expr: private_field_add(
                            ctx,
                            wm,
                            Expr::Local {
                                id: local,
                                ty: Type::Function,
                            },
                            init_expr,
                        ),
                    });
                } else {
                    // CreateDataPropertyOrThrow — TypeError on non-writable prototype (E19.82.04).
                    let key_expr = if let Some(key_id) = computed_key {
                        local_expr(key_id)
                    } else {
                        lower_object_key_name_expr(checked, ctx, fkey, None)
                    };
                    out.push(Stmt::Expr {
                        expr: create_data_property_or_throw(
                            Expr::Local {
                                id: local,
                                ty: Type::Function,
                            },
                            key_expr,
                            init_expr,
                        ),
                    });
                }
            }
            StaticInit::Block(body) => {
                // Method-form on home with correct super base so `super.x` works (E19.72).
                // `({ __proto__: Parent, __sb() { … } }).__sb.call(Class)`
                let block_body = with_use_strict(lower_fn_body(checked, ctx, body, None));
                let method_fn = Expr::Function {
                    name: None,
                    params: Vec::new(),
                    body: block_body,
                    is_async: false,
                    is_generator: false,
                    is_arrow: false,
                    is_method: true,
                    ty: Type::Function,
                };
                let home_proto = match parent_expr.as_ref() {
                    Some(p) => p.clone(),
                    None => member_prop(
                        Expr::IdentName {
                            name: "Function".into(),
                            ty: Type::Function,
                        },
                        "prototype",
                        Type::Any,
                    ),
                };
                let home = Expr::Object {
                    properties: vec![
                        ObjectProp::Property {
                            key: ObjectPropKey::Static("__proto__".into()),
                            value: home_proto,
                        },
                        ObjectProp::Property {
                            key: ObjectPropKey::Static("__sb".into()),
                            value: method_fn,
                        },
                    ],
                    ty: Type::Object,
                };
                out.push(Stmt::Expr {
                    expr: Expr::Call {
                        callee: Box::new(member_prop(
                            member_prop(home, "__sb", Type::Function),
                            "call",
                            Type::Function,
                        )),
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

/// NamedEvaluation name for a class field (`#x` for private) (E19.82.04).
fn field_name_hint(key: &draconic_ast::ObjectKey, is_private: bool) -> Option<String> {
    match key {
        draconic_ast::ObjectKey::Ident(id) => {
            if is_private {
                Some(format!("#{}", id.name))
            } else {
                Some(id.name.clone())
            }
        }
        draconic_ast::ObjectKey::String(s) => Some(s.value.to_string_lossy()),
        draconic_ast::ObjectKey::Computed(_) => None,
    }
}

/// ECMA-262 IsAnonymousFunctionDefinition (function/arrow only; classes use name_hint).
fn is_anonymous_function_def(expr: &AstExpr) -> bool {
    let mut e = expr;
    loop {
        match e {
            AstExpr::Paren { expr: inner, .. } | AstExpr::As { expr: inner, .. } => e = inner,
            AstExpr::FunctionExpression { name: None, is_method: false, .. } => return true,
            AstExpr::ArrowFunction { .. } => return true,
            _ => return false,
        }
    }
}

/// Lower a class field initializer with NamedEvaluation SetFunctionName (E19.82.04).
fn lower_field_init_expr(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    value: &AstExpr,
    super_class: Option<&AstExpr>,
    name_hint: Option<&str>,
) -> Expr {
    // Class expressions still use name_hint inside lower_expr_hint.
    let init = lower_expr_hint(checked, ctx, value, super_class, name_hint);
    if let Some(hint) = name_hint {
        if is_anonymous_function_def(value) {
            return set_function_name_on_expr(ctx, init, hint);
        }
    }
    init
}

/// True if `expr` is the identifier `eval` (parens peeled).
fn ast_expr_is_eval_ident(expr: &AstExpr) -> bool {
    match expr {
        AstExpr::Ident(id) => id.name == "eval",
        AstExpr::Paren { expr: inner, .. } => ast_expr_is_eval_ident(inner),
        _ => false,
    }
}

/// String value of a string/template-no-sub literal (parens peeled), if any.
fn ast_string_literal_value(expr: &AstExpr) -> Option<String> {
    match expr {
        AstExpr::Paren { expr: inner, .. } => ast_string_literal_value(inner),
        AstExpr::String(s) => Some(s.value.to_string_lossy()),
        AstExpr::TemplateLiteral {
            quasis,
            expressions,
            ..
        } if expressions.is_empty() && quasis.len() == 1 => {
            Some(quasis[0].cooked.to_string_lossy())
        }
        _ => None,
    }
}

/// `ContainsArguments` over eval source text (skip strings/comments/templates roughly).
fn source_contains_arguments_ident(src: &str) -> bool {
    let b = src.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i];
        // line comment
        if c == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            i += 2;
            while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
                i += 1;
            }
            continue;
        }
        // block comment
        if c == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = i.saturating_add(2);
            continue;
        }
        // string ' or "
        if c == b'\'' || c == b'"' {
            let q = c;
            i += 1;
            while i < b.len() {
                if b[i] == b'\\' {
                    i = i.saturating_add(2);
                    continue;
                }
                if b[i] == q {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        // template literal (skip; nested ${} not fully parsed — false negatives ok for tests)
        if c == b'`' {
            i += 1;
            while i < b.len() {
                if b[i] == b'\\' {
                    i = i.saturating_add(2);
                    continue;
                }
                if b[i] == b'`' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        // identifier start
        if c == b'_'
            || c == b'$'
            || c.is_ascii_alphabetic()
            || (c >= 0x80)
        {
            let start = i;
            i += 1;
            while i < b.len() {
                let d = b[i];
                if d == b'_'
                    || d == b'$'
                    || d.is_ascii_alphanumeric()
                    || d >= 0x80
                {
                    i += 1;
                } else {
                    break;
                }
            }
            if &src[start..i] == "arguments" {
                return true;
            }
            continue;
        }
        i += 1;
    }
    false
}

/// True when the running private environment is non-empty (fields/methods/accessors).
fn ctx_has_private_env(ctx: &LowerCtx) -> bool {
    !ctx.private_fields.is_empty()
        || !ctx.private_methods.is_empty()
        || !ctx.private_accessors.is_empty()
}

/// Collect private identifier names currently in scope for eval fragment wrapping.
fn ctx_private_names(ctx: &LowerCtx) -> Vec<String> {
    let mut names: HashSet<String> = HashSet::new();
    names.extend(ctx.private_fields.keys().cloned());
    names.extend(ctx.private_methods.keys().cloned());
    names.extend(ctx.private_accessors.keys().cloned());
    let mut v: Vec<String> = names.into_iter().collect();
    v.sort();
    v
}

/// Parse `src` as an expression under a synthetic class that declares `private_names`,
/// so AllPrivateNamesValid accepts `#m` refs (E19.82.08).
fn parse_eval_expr_with_privates(src: &str, private_names: &[String]) -> Option<AstExpr> {
    let mut decls = String::new();
    for n in private_names {
        decls.push_str("#");
        decls.push_str(n);
        decls.push(';');
    }
    // Parenthesize so assignment / comma / etc. parse as a single Expression.
    let wrapped = format!("class __DracEvalPriv {{{decls}__run(){{return({src});}}}}");
    let program = draconic_parser::parse(&wrapped).ok()?;
    extract_synthetic_eval_return_expr(&program)
}

fn extract_synthetic_eval_return_expr(program: &draconic_ast::Program) -> Option<AstExpr> {
    let stmt = program.body.first()?;
    let AstStmt::ClassDeclaration { body, .. } = stmt else {
        return None;
    };
    for el in body {
        if let ClassElement::Method {
            key,
            body: method_body,
            is_static: false,
            is_private: false,
            ..
        } = el
        {
            let is_run = match key {
                draconic_ast::ObjectKey::Ident(id) => id.name == "__run",
                draconic_ast::ObjectKey::String(s) => s.value.to_string_lossy() == "__run",
                _ => false,
            };
            if !is_run {
                continue;
            }
            let AstStmt::Block { body, .. } = method_body.as_ref() else {
                continue;
            };
            if let Some(AstStmt::Return {
                argument: Some(expr),
                ..
            }) = body.first()
            {
                return Some(expr.clone());
            }
        }
    }
    None
}

/// Direct `eval("…#m…")` with a private environment: lower the string as an expression
/// so WeakMap/brand desugaring applies (native `#` would not see our desugared fields).
fn try_lower_direct_eval_private(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    src: &str,
    super_class: Option<&AstExpr>,
) -> Option<Expr> {
    if !src.contains('#') || !ctx_has_private_env(ctx) {
        return None;
    }
    let names = ctx_private_names(ctx);
    let expr = parse_eval_expr_with_privates(src, &names)?;
    Some(lower_expr(checked, ctx, &expr, super_class))
}

/// `(() => { throw new SyntaxError("…arguments…"); })()` for field-init eval (E19.82.06).
fn field_init_eval_arguments_error() -> Expr {
    let msg = Expr::String {
        value: "'arguments' is not allowed in class field initializer or static initialization block"
            .into(),
        ty: Type::String,
    };
    let err = Expr::New {
        callee: Box::new(Expr::IdentName {
            name: "SyntaxError".into(),
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(msg)],
        ty: Type::Any,
    };
    let throw_fn = Expr::Function {
        name: None,
        params: Vec::new(),
        body: vec![Stmt::Throw { value: err }],
        is_async: false,
        is_generator: false,
        is_arrow: true,
        is_method: false,
        ty: Type::Function,
    };
    Expr::Call {
        callee: Box::new(throw_fn),
        args: Vec::new(),
        optional: false,
        ty: Type::Any,
    }
}

/// Evaluate `body` as a method with [[HomeObject]] prototype `home_proto`, called with `receiver`.
/// Used so SuperProperty in field initializers resolves correctly (E19.82.05).
fn call_method_with_home(home_proto: Expr, body: Vec<Stmt>, receiver: Expr) -> Expr {
    let method_fn = Expr::Function {
        name: None,
        params: Vec::new(),
        body: with_use_strict(body),
        is_async: false,
        is_generator: false,
        is_arrow: false,
        is_method: true,
        ty: Type::Function,
    };
    let home = Expr::Object {
        properties: vec![
            ObjectProp::Property {
                key: ObjectPropKey::Static("__proto__".into()),
                value: home_proto,
            },
            ObjectProp::Property {
                key: ObjectPropKey::Static("__fi".into()),
                value: method_fn,
            },
        ],
        ty: Type::Object,
    };
    Expr::Call {
        callee: Box::new(member_prop(
            member_prop(home, "__fi", Type::Function),
            "call",
            Type::Function,
        )),
        args: vec![Arg::Expr(receiver)],
        optional: false,
        ty: Type::Any,
    }
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

/// `Object.isExtensible(object)`.
fn object_is_extensible(object: Expr) -> Expr {
    Expr::Call {
        callee: Box::new(member_prop(
            Expr::IdentName {
                name: "Object".into(),
                ty: Type::Function,
            },
            "isExtensible",
            Type::Function,
        )),
        args: vec![Arg::Expr(object)],
        optional: false,
        ty: Type::Boolean,
    }
}

/// PrivateMethodOrAccessorAdd: non-extensible or already branded → TypeError (E18.40 / E19.82.09).
fn private_brand_add(ctx: &mut LowerCtx, brand: LocalId, object: Expr) -> Expr {
    let oid = ctx.alloc_synthetic_local("__drac_o".into(), Type::Any);
    let o = local_expr(oid);
    let not_ext = Expr::Unary {
        op: UnaryOp::Not,
        arg: Box::new(object_is_extensible(o.clone())),
        ty: Type::Boolean,
    };
    let already = private_brand_has(brand, o.clone());
    let add_call = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(local_expr(brand)),
            property: Box::new(Expr::String {
                value: "add".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(o)],
        optional: false,
        ty: Type::Any,
    };
    let body = Expr::Conditional {
        test: Box::new(not_ext),
        consequent: Box::new(throw_type_error_expr(
            "Cannot define private method on non-extensible object",
        )),
        alternate: Box::new(Expr::Conditional {
            test: Box::new(already),
            consequent: Box::new(throw_type_error_expr(
                "Cannot add private method that already exists",
            )),
            alternate: Box::new(add_call),
            ty: Type::Any,
        }),
        ty: Type::Any,
    };
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(oid),
                default: None,
                rest: false,
            }],
            body: vec![Stmt::Return {
                value: Some(body),
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(object)],
        optional: false,
        ty: Type::Any,
    }
}

/// PrivateFieldAdd: non-extensible or already present → TypeError (E18.35 / E19.82.09).
fn private_field_add(ctx: &mut LowerCtx, wm: LocalId, object: Expr, value: Expr) -> Expr {
    let oid = ctx.alloc_synthetic_local("__drac_o".into(), Type::Any);
    let vid = ctx.alloc_synthetic_local("__drac_v".into(), Type::Any);
    let o = local_expr(oid);
    let v = local_expr(vid);
    let not_ext = Expr::Unary {
        op: UnaryOp::Not,
        arg: Box::new(object_is_extensible(o.clone())),
        ty: Type::Boolean,
    };
    let already = private_brand_has(wm, o.clone());
    let set_call = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(local_expr(wm)),
            property: Box::new(Expr::String {
                value: "set".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(o), Arg::Expr(v.clone())],
        optional: false,
        ty: Type::Any,
    };
    let set_and_yield = Expr::Binary {
        left: Box::new(set_call),
        op: BinaryOp::Comma,
        right: Box::new(v),
        ty: Type::Any,
    };
    let body = Expr::Conditional {
        test: Box::new(not_ext),
        consequent: Box::new(throw_type_error_expr(
            "Cannot define private field on non-extensible object",
        )),
        alternate: Box::new(Expr::Conditional {
            test: Box::new(already),
            consequent: Box::new(throw_type_error_expr(
                "Cannot add private field that already exists",
            )),
            alternate: Box::new(set_and_yield),
            ty: Type::Any,
        }),
        ty: Type::Any,
    };
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![
                Param {
                    pattern: Pattern::Local(oid),
                    default: None,
                    rest: false,
                },
                Param {
                    pattern: Pattern::Local(vid),
                    default: None,
                    rest: false,
                },
            ],
            body: vec![Stmt::Return {
                value: Some(body),
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(object), Arg::Expr(value)],
        optional: false,
        ty: Type::Any,
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

/// `brand.has(object)` (WeakMap/WeakSet).
fn private_brand_has(brand: LocalId, object: Expr) -> Expr {
    Expr::Call {
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
    }
}

/// Object-like check: `o != null && (typeof o === "object" || typeof o === "function")`.
fn is_object_like_expr(object: Expr) -> Expr {
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
        arg: Box::new(object),
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
    Expr::Binary {
        left: Box::new(not_nullish),
        op: BinaryOp::And,
        right: Box::new(is_obj_like),
        ty: Type::Boolean,
    }
}

/// `((o) => body)(arg)` with `o` bound once (E19.53 brand / optional private).
fn iife_bind_arg(ctx: &mut LowerCtx, arg: Expr, body: impl FnOnce(Expr) -> Expr) -> Expr {
    let pid = ctx.alloc_synthetic_local("__drac_o".into(), Type::Any);
    let body_expr = body(Expr::Local {
        id: pid,
        ty: Type::Any,
    });
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(pid),
                default: None,
                rest: false,
            }],
            body: vec![Stmt::Return {
                value: Some(body_expr),
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(arg)],
        optional: false,
        ty: Type::Any,
    }
}

/// `base?.#priv…` → `((o) => o == null ? undefined : then(o))(base)`.
fn optional_private_chain(
    ctx: &mut LowerCtx,
    base: Expr,
    then: impl FnOnce(&mut LowerCtx, Expr) -> Expr,
) -> Expr {
    let pid = ctx.alloc_synthetic_local("__drac_o".into(), Type::Any);
    let o = Expr::Local {
        id: pid,
        ty: Type::Any,
    };
    let nullish = Expr::Binary {
        left: Box::new(o.clone()),
        op: BinaryOp::EqEq,
        right: Box::new(Expr::Null { ty: Type::Null }),
        ty: Type::Boolean,
    };
    let then_expr = then(ctx, o);
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(pid),
                default: None,
                rest: false,
            }],
            body: vec![Stmt::Return {
                value: Some(Expr::Conditional {
                    test: Box::new(nullish),
                    consequent: Box::new(Expr::IdentName {
                        name: "undefined".into(),
                        ty: Type::Any,
                    }),
                    alternate: Box::new(then_expr),
                    ty: Type::Any,
                }),
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(base)],
        optional: false,
        ty: Type::Any,
    }
}

/// Brand-check `object` then yield `then` (PrivateBrandCheck / PrivateFieldFind).
fn private_access_checked(
    ctx: &mut LowerCtx,
    brand: LocalId,
    object: Expr,
    then: impl FnOnce(Expr) -> Expr,
    err_msg: &str,
) -> Expr {
    iife_bind_arg(ctx, object, |o| {
        let ok = Expr::Binary {
            left: Box::new(is_object_like_expr(o.clone())),
            op: BinaryOp::And,
            right: Box::new(private_brand_has(brand, o.clone())),
            ty: Type::Boolean,
        };
        Expr::Conditional {
            test: Box::new(ok),
            consequent: Box::new(then(o)),
            alternate: Box::new(throw_type_error_expr(err_msg)),
            ty: Type::Any,
        }
    })
}

/// `wm.get(object)` with brand check (missing → TypeError).
fn private_field_get(ctx: &mut LowerCtx, wm: LocalId, object: Expr) -> Expr {
    private_access_checked(
        ctx,
        wm,
        object,
        |o| {
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
                args: vec![Arg::Expr(o)],
                optional: false,
                ty: Type::Any,
            }
        },
        "Cannot read private member from an object whose class did not declare it",
    )
}

/// `(wm.set(object, value), value)` with brand check so assignment yields the RHS.
fn private_field_set(ctx: &mut LowerCtx, wm: LocalId, object: Expr, value: Expr) -> Expr {
    private_access_checked(
        ctx,
        wm,
        object,
        |o| {
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
                args: vec![Arg::Expr(o), Arg::Expr(value.clone())],
                optional: false,
                ty: Type::Any,
            };
            Expr::Binary {
                left: Box::new(set_call),
                op: BinaryOp::Comma,
                right: Box::new(value),
                ty: Type::Any,
            }
        },
        "Cannot write private member to an object whose class did not declare it",
    )
}

/// Read private field / accessor / method value for `object.#name` (with brand check).
fn private_member_get(ctx: &mut LowerCtx, fname: &str, object: Expr) -> Expr {
    if let Some(fn_id) = ctx.private_methods.get(fname).copied() {
        let brand = resolve_private_brand(ctx, fname);
        return private_access_checked(
            ctx,
            brand,
            object,
            |_| Expr::Local {
                id: fn_id,
                ty: Type::Function,
            },
            &format!("Cannot read private method #{fname}"),
        );
    }
    if let Some((get, set)) = ctx.private_accessors.get(fname).copied() {
        let _ = set;
        let brand = resolve_private_brand(ctx, fname);
        if let Some(get_id) = get {
            return private_access_checked(
                ctx,
                brand,
                object,
                |o| private_fn_call(get_id, o, Vec::new()),
                &format!("Cannot read private accessor #{fname}"),
            );
        }
        return throw_type_error_expr(&format!(
            "Private accessor #{fname} has no getter"
        ));
    }
    if let Some(wm) = ctx.private_fields.get(fname).copied() {
        return private_field_get(ctx, wm, object);
    }
    throw_type_error_expr(&format!("unknown private field #{fname}"))
}

/// `(() => { throw new TypeError(msg); })()` — expression-position TypeError (E19.36).
fn throw_type_error_expr(message: &str) -> Expr {
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: Vec::new(),
            body: vec![Stmt::Throw {
                value: Expr::New {
                    callee: Box::new(Expr::IdentName {
                        name: "TypeError".into(),
                        ty: Type::Function,
                    }),
                    args: vec![Arg::Expr(Expr::String {
                        value: message.into(),
                        ty: Type::String,
                    })],
                    ty: Type::Any,
                },
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: Vec::new(),
        optional: false,
        ty: Type::Any,
    }
}

/// Write private field / accessor: yields `value` (with brand check).
fn private_member_set(ctx: &mut LowerCtx, fname: &str, object: Expr, value: Expr) -> Expr {
    // Private methods are not writable (TypeError, not IR panic).
    if ctx.private_methods.contains_key(fname) {
        return throw_type_error_expr(&format!(
            "Private method #{fname} is not writable"
        ));
    }
    if let Some((get, set)) = ctx.private_accessors.get(fname).copied() {
        let _ = get;
        let brand = resolve_private_brand(ctx, fname);
        if let Some(set_id) = set {
            return private_access_checked(
                ctx,
                brand,
                object,
                |o| {
                    let set_call =
                        private_fn_call(set_id, o, vec![Arg::Expr(value.clone())]);
                    Expr::Binary {
                        left: Box::new(set_call),
                        op: BinaryOp::Comma,
                        right: Box::new(value),
                        ty: Type::Any,
                    }
                },
                &format!("Cannot write private accessor #{fname}"),
            );
        }
        return throw_type_error_expr(&format!(
            "Private accessor #{fname} has no setter"
        ));
    }
    if let Some(wm) = ctx.private_fields.get(fname).copied() {
        return private_field_set(ctx, wm, object, value);
    }
    throw_type_error_expr(&format!("unknown private field #{fname}"))
}

/// `obj.#f = v` / compound / logical assign; object evaluated once (E19.36).
fn lower_private_assign(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    fname: &str,
    object: &AstExpr,
    op: AssignOp,
    value: &AstExpr,
    super_class: Option<&AstExpr>,
) -> Expr {
    let obj_expr = lower_expr(checked, ctx, object, super_class);
    let rhs = lower_expr(checked, ctx, value, super_class);
    let tmp = ctx.alloc_synthetic_local(format!("__drac_pobj_{fname}"), Type::Any);
    let bind_obj = Expr::Assign {
        target: AssignTarget::Local(tmp),
        op: AssignOp::Eq,
        value: Box::new(obj_expr),
        ty: Type::Any,
    };
    let obj_local = || Expr::Local {
        id: tmp,
        ty: Type::Any,
    };
    // Bind computed RHS to a temp so `private_field_set`'s `(set, value)` does not
    // re-evaluate get+binop (would double-increment).
    let val_id = ctx.alloc_synthetic_local(format!("__drac_pval_{fname}"), Type::Any);
    let bind_val = |v: Expr| Expr::Assign {
        target: AssignTarget::Local(val_id),
        op: AssignOp::Eq,
        value: Box::new(v),
        ty: Type::Any,
    };
    let val_local = || Expr::Local {
        id: val_id,
        ty: Type::Any,
    };
    let assigned = match op {
        AssignOp::Eq => {
            let set = private_member_set(ctx, fname, obj_local(), val_local());
            Expr::Binary {
                left: Box::new(bind_val(rhs)),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            }
        }
        AssignOp::AndAndEq => {
            let cur = private_member_get(ctx, fname, obj_local());
            let set = private_member_set(ctx, fname, obj_local(), val_local());
            let then_set = Expr::Binary {
                left: Box::new(bind_val(rhs)),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            };
            Expr::Binary {
                left: Box::new(cur),
                op: BinaryOp::And,
                right: Box::new(then_set),
                ty: Type::Any,
            }
        }
        AssignOp::OrOrEq => {
            let cur = private_member_get(ctx, fname, obj_local());
            let set = private_member_set(ctx, fname, obj_local(), val_local());
            let then_set = Expr::Binary {
                left: Box::new(bind_val(rhs)),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            };
            Expr::Binary {
                left: Box::new(cur),
                op: BinaryOp::Or,
                right: Box::new(then_set),
                ty: Type::Any,
            }
        }
        AssignOp::NullishEq => {
            let cur = private_member_get(ctx, fname, obj_local());
            let set = private_member_set(ctx, fname, obj_local(), val_local());
            let then_set = Expr::Binary {
                left: Box::new(bind_val(rhs)),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            };
            Expr::Binary {
                left: Box::new(cur),
                op: BinaryOp::Nullish,
                right: Box::new(then_set),
                ty: Type::Any,
            }
        }
        other => {
            let binop = other
                .binary_op()
                .expect("compound assign op has binary_op");
            let cur = private_member_get(ctx, fname, obj_local());
            let combined = Expr::Binary {
                left: Box::new(cur),
                op: binop,
                right: Box::new(rhs),
                ty: Type::Any,
            };
            let set = private_member_set(ctx, fname, obj_local(), val_local());
            Expr::Binary {
                left: Box::new(bind_val(combined)),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            }
        }
    };
    Expr::Binary {
        left: Box::new(bind_obj),
        op: BinaryOp::Comma,
        right: Box::new(assigned),
        ty: Type::Any,
    }
}

fn binding_pattern_has_private(pat: &BindingPattern) -> bool {
    match pat {
        BindingPattern::Ident(_) => false,
        BindingPattern::Member(expr) => matches!(
            expr.as_ref(),
            AstExpr::MemberExpression {
                private: true,
                ..
            }
        ),
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Elision => false,
            ArrayPatternElement::Pattern { binding, .. }
            | ArrayPatternElement::Rest(binding) => binding_pattern_has_private(binding),
        }),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop { binding, .. } | ObjectPatternProp::Rest(binding) => {
                binding_pattern_has_private(binding)
            }
        }),
    }
}

fn array_pattern_has_private(elements: &[ArrayPatternElement]) -> bool {
    elements.iter().any(|el| match el {
        ArrayPatternElement::Elision => false,
        ArrayPatternElement::Pattern { binding, .. } | ArrayPatternElement::Rest(binding) => {
            binding_pattern_has_private(binding)
        }
    })
}

fn object_pattern_has_private(properties: &[ObjectPatternProp]) -> bool {
    properties.iter().any(|p| match p {
        ObjectPatternProp::Prop { binding, .. } | ObjectPatternProp::Rest(binding) => {
            binding_pattern_has_private(binding)
        }
    })
}

fn comma_seq(exprs: Vec<Expr>) -> Expr {
    let mut it = exprs.into_iter();
    let first = it.next().expect("comma_seq non-empty");
    it.fold(first, |left, right| Expr::Binary {
        left: Box::new(left),
        op: BinaryOp::Comma,
        right: Box::new(right),
        ty: Type::Any,
    })
}

fn bind_local(id: LocalId, value: Expr) -> Expr {
    Expr::Assign {
        target: AssignTarget::Local(id),
        op: AssignOp::Eq,
        value: Box::new(value),
        ty: Type::Any,
    }
}

fn undefined_expr() -> Expr {
    Expr::Unary {
        op: UnaryOp::Void,
        arg: Box::new(Expr::Number {
            raw: "0".into(),
            ty: Type::Number,
        }),
        ty: Type::Any,
    }
}

/// PutValue into a binding (private leaves use PrivateFieldSet).
fn lower_put_binding(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    binding: &BindingPattern,
    value: Expr,
    super_class: Option<&AstExpr>,
) -> Expr {
    match binding {
        BindingPattern::Ident(id) => {
            let target = if let Some(local) = checked.bound.resolve(id.span) {
                AssignTarget::Local(ctx.map_class_name(local))
            } else {
                AssignTarget::Name(id.name.clone())
            };
            Expr::Assign {
                target,
                op: AssignOp::Eq,
                value: Box::new(value),
                ty: Type::Any,
            }
        }
        BindingPattern::Member(expr) => match expr.as_ref() {
            AstExpr::MemberExpression {
                object,
                property,
                private: true,
                ..
            } => {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.as_str(),
                    _ => panic!("private member property must be ident"),
                };
                let obj = lower_expr(checked, ctx, object, super_class);
                private_member_set(ctx, fname, obj, value)
            }
            AstExpr::MemberExpression {
                object,
                property,
                computed,
                private: false,
                ..
            } => {
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
                Expr::Assign {
                    target: AssignTarget::Member {
                        object: Box::new(lower_expr(checked, ctx, object, super_class)),
                        property: Box::new(prop),
                        computed: *computed,
                    },
                    op: AssignOp::Eq,
                    value: Box::new(value),
                    ty: Type::Any,
                }
            }
            _ => panic!("BindingPattern::Member must wrap MemberExpression"),
        },
        BindingPattern::Array { elements, .. } => {
            lower_array_pattern_assign(checked, ctx, elements, value, super_class)
        }
        BindingPattern::Object { properties, .. } => {
            lower_object_pattern_assign(checked, ctx, properties, value, super_class)
        }
    }
}

/// Keyed/Iterator destructuring: evaluate private lref base before GetV/IteratorStep (E19.82.10).
fn lower_dstr_element_assign<F>(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    binding: &BindingPattern,
    value_after_lref: F,
    super_class: Option<&AstExpr>,
) -> Expr
where
    F: FnOnce(&mut LowerCtx) -> Expr,
{
    match binding {
        BindingPattern::Member(expr) => match expr.as_ref() {
            AstExpr::MemberExpression {
                object,
                property,
                private: true,
                ..
            } => {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.as_str(),
                    _ => panic!("private member property must be ident"),
                };
                let obj_id = ctx.alloc_synthetic_local(format!("__drac_dstr_lref_{fname}"), Type::Any);
                let val_id = ctx.alloc_synthetic_local(format!("__drac_dstr_v_{fname}"), Type::Any);
                let bind_obj = bind_local(obj_id, lower_expr(checked, ctx, object, super_class));
                let bind_val = bind_local(val_id, value_after_lref(ctx));
                let set = private_member_set(ctx, fname, local_expr(obj_id), local_expr(val_id));
                comma_seq(vec![bind_obj, bind_val, set])
            }
            AstExpr::MemberExpression {
                object,
                property,
                computed,
                private: false,
                ..
            } => {
                let obj_id = ctx.alloc_synthetic_local("__drac_dstr_lref_m".into(), Type::Any);
                let mut steps = vec![bind_local(
                    obj_id,
                    lower_expr(checked, ctx, object, super_class),
                )];
                let prop_expr = if *computed {
                    let prop_id = ctx.alloc_synthetic_local("__drac_dstr_pkey".into(), Type::Any);
                    steps.push(bind_local(
                        prop_id,
                        lower_expr(checked, ctx, property, super_class),
                    ));
                    local_expr(prop_id)
                } else {
                    match property.as_ref() {
                        AstExpr::Ident(id) => Expr::String {
                            value: id.name.clone().into(),
                            ty: Type::String,
                        },
                        other => lower_expr(checked, ctx, other, super_class),
                    }
                };
                let val_id = ctx.alloc_synthetic_local("__drac_dstr_vm".into(), Type::Any);
                steps.push(bind_local(val_id, value_after_lref(ctx)));
                steps.push(Expr::Assign {
                    target: AssignTarget::Member {
                        object: Box::new(local_expr(obj_id)),
                        property: Box::new(prop_expr),
                        computed: *computed,
                    },
                    op: AssignOp::Eq,
                    value: Box::new(local_expr(val_id)),
                    ty: Type::Any,
                });
                comma_seq(steps)
            }
            _ => panic!("BindingPattern::Member must wrap MemberExpression"),
        },
        other => {
            let val_id = ctx.alloc_synthetic_local("__drac_dstr_vo".into(), Type::Any);
            comma_seq(vec![
                bind_local(val_id, value_after_lref(ctx)),
                lower_put_binding(checked, ctx, other, local_expr(val_id), super_class),
            ])
        }
    }
}

fn iterator_next_value(ctx: &mut LowerCtx, iter_id: LocalId) -> Expr {
    let n_id = ctx.alloc_synthetic_local("__drac_dstr_n".into(), Type::Any);
    let v_id = ctx.alloc_synthetic_local("__drac_dstr_nv".into(), Type::Any);
    comma_seq(vec![
        bind_local(
            n_id,
            Expr::Call {
                callee: Box::new(Expr::Member {
                    object: Box::new(local_expr(iter_id)),
                    property: Box::new(Expr::String {
                        value: "next".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Function,
                }),
                args: Vec::new(),
                optional: false,
                ty: Type::Any,
            },
        ),
        bind_local(
            v_id,
            Expr::Conditional {
                test: Box::new(Expr::Member {
                    object: Box::new(local_expr(n_id)),
                    property: Box::new(Expr::String {
                        value: "done".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Any,
                }),
                consequent: Box::new(undefined_expr()),
                alternate: Box::new(Expr::Member {
                    object: Box::new(local_expr(n_id)),
                    property: Box::new(Expr::String {
                        value: "value".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Any,
                }),
                ty: Type::Any,
            },
        ),
        local_expr(v_id),
    ])
}

fn with_default(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    raw: Expr,
    default: Option<&AstExpr>,
    super_class: Option<&AstExpr>,
) -> Expr {
    match default {
        None => raw,
        Some(def) => {
            let raw_id = ctx.alloc_synthetic_local("__drac_dstr_raw".into(), Type::Any);
            comma_seq(vec![
                bind_local(raw_id, raw),
                Expr::Conditional {
                    test: Box::new(Expr::Binary {
                        left: Box::new(local_expr(raw_id)),
                        op: BinaryOp::EqEqEq,
                        right: Box::new(undefined_expr()),
                        ty: Type::Boolean,
                    }),
                    consequent: Box::new(lower_expr(checked, ctx, def, super_class)),
                    alternate: Box::new(local_expr(raw_id)),
                    ty: Type::Any,
                },
            ])
        }
    }
}

/// Desugar array destructuring assignment (used when private targets present).
fn lower_array_pattern_assign(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    elements: &[ArrayPatternElement],
    rhs: Expr,
    super_class: Option<&AstExpr>,
) -> Expr {
    let rhs_id = ctx.alloc_synthetic_local("__drac_dstr_rhs".into(), Type::Any);
    let iter_id = ctx.alloc_synthetic_local("__drac_dstr_it".into(), Type::Any);
    let mut steps = vec![
        bind_local(rhs_id, rhs),
        bind_local(
            iter_id,
            Expr::Call {
                callee: Box::new(Expr::Member {
                    object: Box::new(local_expr(rhs_id)),
                    property: Box::new(Expr::Member {
                        object: Box::new(Expr::IdentName {
                            name: "Symbol".into(),
                            ty: Type::Any,
                        }),
                        property: Box::new(Expr::String {
                            value: "iterator".into(),
                            ty: Type::String,
                        }),
                        computed: false,
                        optional: false,
                        ty: Type::Any,
                    }),
                    computed: true,
                    optional: false,
                    ty: Type::Function,
                }),
                args: Vec::new(),
                optional: false,
                ty: Type::Any,
            },
        ),
    ];

    for el in elements {
        match el {
            ArrayPatternElement::Elision => {
                steps.push(iterator_next_value(ctx, iter_id));
            }
            ArrayPatternElement::Pattern { binding, default } => {
                let def = default.as_ref();
                let step = lower_dstr_element_assign(
                    checked,
                    ctx,
                    binding,
                    |ctx| {
                        let raw = iterator_next_value(ctx, iter_id);
                        with_default(checked, ctx, raw, def, super_class)
                    },
                    super_class,
                );
                steps.push(step);
            }
            ArrayPatternElement::Rest(binding) => {
                let rest_id = ctx.alloc_synthetic_local("__drac_dstr_rest".into(), Type::Any);
                steps.push(bind_local(
                    rest_id,
                    Expr::Array {
                        elements: Vec::new(),
                        ty: Type::Any,
                    },
                ));
                let n_id = ctx.alloc_synthetic_local("__drac_dstr_rn".into(), Type::Any);
                let drain = Expr::Call {
                    callee: Box::new(Expr::Function {
                        name: None,
                        params: Vec::new(),
                        body: vec![Stmt::While {
                            test: Expr::Boolean {
                                value: true,
                                ty: Type::Boolean,
                            },
                            body: Box::new(Stmt::Block {
                                body: vec![
                                    Stmt::Expr {
                                        expr: bind_local(
                                            n_id,
                                            Expr::Call {
                                                callee: Box::new(Expr::Member {
                                                    object: Box::new(local_expr(iter_id)),
                                                    property: Box::new(Expr::String {
                                                        value: "next".into(),
                                                        ty: Type::String,
                                                    }),
                                                    computed: false,
                                                    optional: false,
                                                    ty: Type::Function,
                                                }),
                                                args: Vec::new(),
                                                optional: false,
                                                ty: Type::Any,
                                            },
                                        ),
                                    },
                                    Stmt::If {
                                        test: Expr::Member {
                                            object: Box::new(local_expr(n_id)),
                                            property: Box::new(Expr::String {
                                                value: "done".into(),
                                                ty: Type::String,
                                            }),
                                            computed: false,
                                            optional: false,
                                            ty: Type::Any,
                                        },
                                        consequent: Box::new(Stmt::Break { label: None }),
                                        alternate: None,
                                    },
                                    Stmt::Expr {
                                        expr: Expr::Call {
                                            callee: Box::new(Expr::Member {
                                                object: Box::new(local_expr(rest_id)),
                                                property: Box::new(Expr::String {
                                                    value: "push".into(),
                                                    ty: Type::String,
                                                }),
                                                computed: false,
                                                optional: false,
                                                ty: Type::Function,
                                            }),
                                            args: vec![Arg::Expr(Expr::Member {
                                                object: Box::new(local_expr(n_id)),
                                                property: Box::new(Expr::String {
                                                    value: "value".into(),
                                                    ty: Type::String,
                                                }),
                                                computed: false,
                                                optional: false,
                                                ty: Type::Any,
                                            })],
                                            optional: false,
                                            ty: Type::Any,
                                        },
                                    },
                                ],
                            }),
                        }],
                        is_async: false,
                        is_generator: false,
                        is_arrow: true,
                        is_method: false,
                        ty: Type::Function,
                    }),
                    args: Vec::new(),
                    optional: false,
                    ty: Type::Any,
                };
                // lref before collecting rest values for private targets.
                let step = lower_dstr_element_assign(
                    checked,
                    ctx,
                    binding,
                    move |_ctx| comma_seq(vec![drain, local_expr(rest_id)]),
                    super_class,
                );
                steps.push(step);
            }
        }
    }
    steps.push(local_expr(rhs_id));
    comma_seq(steps)
}

fn object_key_to_get_prop(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    key: &draconic_ast::ObjectKey,
    super_class: Option<&AstExpr>,
) -> Expr {
    match key {
        draconic_ast::ObjectKey::Ident(id) => Expr::String {
            value: id.name.clone().into(),
            ty: Type::String,
        },
        draconic_ast::ObjectKey::String(s) => Expr::String {
            value: s.value.clone(),
            ty: Type::String,
        },
        draconic_ast::ObjectKey::Computed(expr) => lower_expr(checked, ctx, expr, super_class),
    }
}

/// Desugar object destructuring assignment (used when private targets present).
fn lower_object_pattern_assign(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    properties: &[ObjectPatternProp],
    rhs: Expr,
    super_class: Option<&AstExpr>,
) -> Expr {
    let rhs_id = ctx.alloc_synthetic_local("__drac_dstr_rhs".into(), Type::Any);
    let mut steps = vec![bind_local(rhs_id, rhs)];
    let mut excluded_keys: Vec<LocalId> = Vec::new();

    for p in properties {
        match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                let key_id = ctx.alloc_synthetic_local("__drac_dstr_key".into(), Type::Any);
                steps.push(bind_local(
                    key_id,
                    object_key_to_get_prop(checked, ctx, key, super_class),
                ));
                excluded_keys.push(key_id);
                let def = default.as_ref();
                let step = lower_dstr_element_assign(
                    checked,
                    ctx,
                    binding,
                    |ctx| {
                        let get = Expr::Member {
                            object: Box::new(local_expr(rhs_id)),
                            property: Box::new(local_expr(key_id)),
                            computed: true,
                            optional: false,
                            ty: Type::Any,
                        };
                        with_default(checked, ctx, get, def, super_class)
                    },
                    super_class,
                );
                steps.push(step);
            }
            ObjectPatternProp::Rest(binding) => {
                let rest_id = ctx.alloc_synthetic_local("__drac_dstr_orest".into(), Type::Any);
                // Build rest via Object.assign after lref for private targets.
                let step = lower_dstr_element_assign(
                    checked,
                    ctx,
                    binding,
                    |ctx| {
                        let mut rest_steps = vec![bind_local(
                            rest_id,
                            Expr::Object {
                                properties: Vec::new(),
                                ty: Type::Any,
                            },
                        )];
                        rest_steps.push(Expr::Call {
                            callee: Box::new(Expr::Member {
                                object: Box::new(Expr::IdentName {
                                    name: "Object".into(),
                                    ty: Type::Any,
                                }),
                                property: Box::new(Expr::String {
                                    value: "assign".into(),
                                    ty: Type::String,
                                }),
                                computed: false,
                                optional: false,
                                ty: Type::Function,
                            }),
                            args: vec![
                                Arg::Expr(local_expr(rest_id)),
                                Arg::Expr(local_expr(rhs_id)),
                            ],
                            optional: false,
                            ty: Type::Any,
                        });
                        for kid in &excluded_keys {
                            rest_steps.push(Expr::Unary {
                                op: UnaryOp::Delete,
                                arg: Box::new(Expr::Member {
                                    object: Box::new(local_expr(rest_id)),
                                    property: Box::new(local_expr(*kid)),
                                    computed: true,
                                    optional: false,
                                    ty: Type::Any,
                                }),
                                ty: Type::Boolean,
                            });
                        }
                        rest_steps.push(local_expr(rest_id));
                        comma_seq(rest_steps)
                    },
                    super_class,
                );
                steps.push(step);
            }
        }
    }
    steps.push(local_expr(rhs_id));
    comma_seq(steps)
}

/// `++obj.#f` / `obj.#f++` (and `--`); object evaluated once (E19.36).
fn lower_private_update(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    fname: &str,
    object: &AstExpr,
    op: UpdateOp,
    prefix: bool,
    super_class: Option<&AstExpr>,
) -> Expr {
    let obj_expr = lower_expr(checked, ctx, object, super_class);
    let tmp = ctx.alloc_synthetic_local(format!("__drac_pobj_{fname}"), Type::Any);
    let bind_obj = Expr::Assign {
        target: AssignTarget::Local(tmp),
        op: AssignOp::Eq,
        value: Box::new(obj_expr),
        ty: Type::Any,
    };
    let obj_local = || Expr::Local {
        id: tmp,
        ty: Type::Any,
    };
    let one = Expr::Number {
        raw: "1".into(),
        ty: Type::Number,
    };
    let binop = match op {
        UpdateOp::Inc => BinaryOp::Add,
        UpdateOp::Dec => BinaryOp::Sub,
    };
    let next_id = ctx.alloc_synthetic_local(format!("__drac_pnext_{fname}"), Type::Any);
    let next_local = || Expr::Local {
        id: next_id,
        ty: Type::Any,
    };
    if prefix {
        let cur = private_member_get(ctx, fname, obj_local());
        let bind_next = Expr::Assign {
            target: AssignTarget::Local(next_id),
            op: AssignOp::Eq,
            value: Box::new(Expr::Binary {
                left: Box::new(cur),
                op: binop,
                right: Box::new(one),
                ty: Type::Any,
            }),
            ty: Type::Any,
        };
        let set = private_member_set(ctx, fname, obj_local(), next_local());
        return Expr::Binary {
            left: Box::new(bind_obj),
            op: BinaryOp::Comma,
            right: Box::new(Expr::Binary {
                left: Box::new(bind_next),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            }),
            ty: Type::Any,
        };
    }
    let cur_id = ctx.alloc_synthetic_local(format!("__drac_pcur_{fname}"), Type::Any);
    let bind_cur = Expr::Assign {
        target: AssignTarget::Local(cur_id),
        op: AssignOp::Eq,
        value: Box::new(private_member_get(ctx, fname, obj_local())),
        ty: Type::Any,
    };
    let bind_next = Expr::Assign {
        target: AssignTarget::Local(next_id),
        op: AssignOp::Eq,
        value: Box::new(Expr::Binary {
            left: Box::new(Expr::Local {
                id: cur_id,
                ty: Type::Any,
            }),
            op: binop,
            right: Box::new(one),
            ty: Type::Any,
        }),
        ty: Type::Any,
    };
    let set = private_member_set(ctx, fname, obj_local(), next_local());
    let set_then_old = Expr::Binary {
        left: Box::new(set),
        op: BinaryOp::Comma,
        right: Box::new(Expr::Local {
            id: cur_id,
            ty: Type::Any,
        }),
        ty: Type::Any,
    };
    Expr::Binary {
        left: Box::new(bind_obj),
        op: BinaryOp::Comma,
        right: Box::new(Expr::Binary {
            left: Box::new(bind_cur),
            op: BinaryOp::Comma,
            right: Box::new(Expr::Binary {
                left: Box::new(bind_next),
                op: BinaryOp::Comma,
                right: Box::new(set_then_old),
                ty: Type::Any,
            }),
            ty: Type::Any,
        }),
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
    lower_expr_hint(checked, ctx, expr, super_class, None)
}

/// Like `lower_expr`, but `name_hint` drives NamedEvaluation for anonymous
/// class expressions (`let cls = class {}` → `.name === "cls"`) (E19.31).
fn lower_expr_hint(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    expr: &AstExpr,
    super_class: Option<&AstExpr>,
    name_hint: Option<&str>,
) -> Expr {
    match expr {
        AstExpr::Paren { expr: inner, .. } => {
            lower_expr_hint(checked, ctx, inner, super_class, name_hint)
        }
        // Dual-worlds `as` is a type-level boundary only (T06); erase at IR.
        AstExpr::As { expr: inner, .. } => {
            lower_expr_hint(checked, ctx, inner, super_class, name_hint)
        }
        AstExpr::ArrayPattern { .. } => {
            panic!("array pattern must only appear as assignment target")
        }
        AstExpr::ObjectPattern { .. } => {
            panic!("object pattern must only appear as assignment target")
        }
        AstExpr::Ident(id) => {
            let ty = expr_ty(checked, id.span);
            if let Some(sym) = checked.bound.resolve(id.span) {
                Expr::Local {
                    id: ctx.map_class_name(sym),
                    ty,
                }
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
        AstExpr::This { span } => {
            if let Some(this_id) = ctx.derived_this {
                // Derived ctor: ES this TDZ until super() (E19.82.03).
                let _ = span;
                assert_derived_this(this_id)
            } else {
                Expr::This {
                    ty: expr_ty(checked, *span),
                }
            }
        }
        AstExpr::NewTarget { span } => Expr::NewTarget {
            ty: expr_ty(checked, *span),
        },
        AstExpr::ImportCall {
            phase,
            source,
            options,
            span,
        } => Expr::ImportCall {
            phase: *phase,
            source: Box::new(lower_expr(checked, ctx, source, super_class)),
            options: options
                .as_ref()
                .map(|o| Box::new(lower_expr(checked, ctx, o, super_class))),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Super { span } => {
            // Keep bare `super` for JS home-object emit; never panic (E19.34).
            // Invalid SuperCall/SuperProperty sites are early errors in parser/check.
            Expr::Super {
                ty: expr_ty(checked, *span),
            }
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
            // E19.60: peel cover parentheses so `(id) = v` lowers as a simple target.
            let mut core = target.as_ref();
            while let AstExpr::Paren { expr, .. } = core {
                core = expr.as_ref();
            }
            // Private field/accessor assign: `obj.#x = v` / compound / logical (E19.36).
            if let AstExpr::MemberExpression {
                object,
                property,
                private: true,
                ..
            } = core
            {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.as_str(),
                    _ => panic!("private member property must be ident"),
                };
                return lower_private_assign(
                    checked,
                    ctx,
                    fname,
                    object,
                    *op,
                    value,
                    super_class,
                );
            }
            // E19.82.10: destructuring assign into private fields — desugar so
            // lref-before-GetV order and PrivateFieldSet apply (not native `#` emit).
            if matches!(op, AssignOp::Eq) {
                if let AstExpr::ArrayPattern { elements, .. } = core {
                    if array_pattern_has_private(elements) {
                        let rhs = lower_expr(checked, ctx, value, super_class);
                        return lower_array_pattern_assign(
                            checked,
                            ctx,
                            elements,
                            rhs,
                            super_class,
                        );
                    }
                }
                if let AstExpr::ObjectPattern { properties, .. } = core {
                    if object_pattern_has_private(properties) {
                        let rhs = lower_expr(checked, ctx, value, super_class);
                        return lower_object_pattern_assign(
                            checked,
                            ctx,
                            properties,
                            rhs,
                            super_class,
                        );
                    }
                }
            }
            let assign_name_hint = match core {
                AstExpr::Ident(id) if matches!(op, AssignOp::Eq) => Some(id.name.as_str()),
                _ => None,
            };
            let target = match core {
                AstExpr::Ident(id) => {
                    if let Some(local) = checked.bound.resolve(id.span) {
                        AssignTarget::Local(ctx.map_class_name(local))
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
                value: Box::new(lower_expr_hint(
                    checked,
                    ctx,
                    value,
                    super_class,
                    assign_name_hint,
                )),
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::Update {
            op,
            arg,
            prefix,
            span,
        } => {
            // E19.60: peel cover parentheses so `(id)++` lowers as a simple target.
            let mut core = arg.as_ref();
            while let AstExpr::Paren { expr, .. } = core {
                core = expr.as_ref();
            }
            if let AstExpr::MemberExpression {
                object,
                property,
                private: true,
                ..
            } = core
            {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.as_str(),
                    _ => panic!("private member property must be ident"),
                };
                return lower_private_update(
                    checked,
                    ctx,
                    fname,
                    object,
                    *op,
                    *prefix,
                    super_class,
                );
            }
            let target = match core {
                AstExpr::Ident(id) => {
                    if let Some(local) = checked.bound.resolve(id.span) {
                        UpdateTarget::Local(ctx.map_class_name(local))
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
            // `super(args)` → derived ctor: Reflect.construct; object method: keep `super(...)`
            // (early SyntaxError for SuperCall in object methods is deferred to check/parser).
            if matches!(callee.as_ref(), AstExpr::Super { .. }) {
                // Derived constructor: bind this via Reflect.construct + field inits (E19.82.03).
                if ctx.derived_this.is_some() {
                    let call_args: Vec<Arg> = args
                        .iter()
                        .map(|a| lower_arg(checked, ctx, a, super_class))
                        .collect();
                    return derived_super_call_expr(ctx, call_args);
                }
                // Object methods and missing-extends: keep `super(...)` for JS emit (E19.34).
                if super_class.is_none() {
                    return Expr::Call {
                        callee: Box::new(Expr::Super {
                            ty: expr_ty(checked, *span),
                        }),
                        args: args
                            .iter()
                            .map(|a| lower_arg(checked, ctx, a, super_class))
                            .collect(),
                        optional: false,
                        ty: expr_ty(checked, *span),
                    };
                }
                let parent_ast = super_class.expect("super_class present");
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
            // Direct eval string literal handling (E19.82.06 / E19.82.08).
            // SuperProperty/new.target work via method HomeObject; SuperCall is SyntaxError
            // natively inside methods. Nested fn/arrow bodies clear `in_field_init`.
            if ast_expr_is_eval_ident(callee) {
                if let Some(first) = args.first() {
                    if let AstArg::Expr(arg_expr) = first {
                        if let Some(src) = ast_string_literal_value(arg_expr) {
                            // Field-init: ContainsArguments early error (E19.82.06).
                            if ctx.in_field_init && source_contains_arguments_ident(&src) {
                                return field_init_eval_arguments_error();
                            }
                            // Private names desugared to WeakMap/brand — rewrite eval
                            // source so `#m` resolves in the current private env (E19.82.08).
                            if let Some(lowered) =
                                try_lower_direct_eval_private(checked, ctx, &src, super_class)
                            {
                                return lowered;
                            }
                        }
                    }
                }
            }
            // `super.m(args)` → `Parent.prototype.m.call(this, ...args)`
            if let AstExpr::MemberExpression {
                object,
                property,
                computed,
                private,
                optional: member_optional,
                ..
            } = callee.as_ref()
            {
                if matches!(object.as_ref(), AstExpr::Super { .. }) {
                    // Object method / base class: keep `super.m(...)` for JS home-object emit (E19.34).
                    if super_class.is_none() {
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
                            object: Box::new(Expr::Super { ty: Type::Any }),
                            property: Box::new(prop),
                            computed: *computed,
                            optional: false,
                            ty: Type::Function,
                        };
                        return Expr::Call {
                            callee: Box::new(method),
                            args: args
                                .iter()
                                .map(|a| lower_arg(checked, ctx, a, super_class))
                                .collect(),
                            optional: false,
                            ty: expr_ty(checked, *span),
                        };
                    }
                    let parent = if let Some(sid) = ctx.derived_super {
                        local_expr(sid)
                    } else {
                        let parent_ast = super_class.expect("super_class present");
                        lower_expr(checked, ctx, parent_ast, None)
                    };
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
                    let this_arg = if let Some(tid) = ctx.derived_this {
                        assert_derived_this(tid)
                    } else {
                        Expr::This { ty: Type::Any }
                    };
                    let mut call_args = Vec::with_capacity(args.len() + 1);
                    call_args.push(Arg::Expr(this_arg));
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
                // `obj.#m(args)` / `obj?.#m(args)` → brand-check then `__drac_pm_m.call(obj, …)` (E18.37 / E19.53)
                if *private {
                    let fname = match property.as_ref() {
                        AstExpr::Ident(id) => id.name.clone(),
                        _ => panic!("private member property must be ident"),
                    };
                    if let Some(fn_id) = ctx.private_methods.get(&fname).copied() {
                        let brand = resolve_private_brand(ctx, &fname);
                        let obj_expr = lower_expr(checked, ctx, object, super_class);
                        let mut lowered_args = Vec::with_capacity(args.len());
                        for a in args {
                            lowered_args.push(lower_arg(checked, ctx, a, super_class));
                        }
                        let err = format!("Cannot read private method #{fname}");
                        let result_ty = expr_ty(checked, *span);
                        let build = |ctx: &mut LowerCtx, base: Expr| {
                            private_access_checked(
                                ctx,
                                brand,
                                base,
                                |o| {
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
                                    let mut call_args =
                                        Vec::with_capacity(lowered_args.len() + 1);
                                    call_args.push(Arg::Expr(o));
                                    call_args.extend(lowered_args.iter().cloned());
                                    Expr::Call {
                                        callee: Box::new(call_member),
                                        args: call_args,
                                        optional: false,
                                        ty: result_ty.clone(),
                                    }
                                },
                                &err,
                            )
                        };
                        if *member_optional || *optional {
                            return optional_private_chain(ctx, obj_expr, |ctx, o| {
                                build(ctx, o)
                            });
                        }
                        return build(ctx, obj_expr);
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
            is_method,
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
            // Methods get object-home `super`; plain function expressions do not inherit `super`
            // or derived ctor this TDZ (E19.82.03).
            let prev_object_super = ctx.object_super;
            let prev_derived_this = ctx.derived_this.take();
            let prev_derived_super = ctx.derived_super.take();
            let prev_inits = std::mem::take(&mut ctx.derived_super_inits);
            let prev_ctor_body = ctx.derived_ctor_body;
            let prev_field_init = ctx.in_field_init;
            if *is_method {
                ctx.object_super = true;
            } else {
                ctx.object_super = false;
            }
            ctx.derived_ctor_body = false;
            // Nested functions are not field-init PerformEval sites (E19.82.06).
            ctx.in_field_init = false;
            let params = lower_params(checked, ctx, params, None);
            let body = lower_fn_body(checked, ctx, body, None);
            ctx.object_super = prev_object_super;
            ctx.derived_this = prev_derived_this;
            ctx.derived_super = prev_derived_super;
            ctx.derived_super_inits = prev_inits;
            ctx.derived_ctor_body = prev_ctor_body;
            ctx.in_field_init = prev_field_init;
            Expr::Function {
                name,
                params,
                body,
                is_async: *is_async,
                is_generator: *is_generator,
                is_arrow: false,
                is_method: *is_method,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::ClassExpression {
            name,
            super_class: sc,
            body,
            span,
        } => lower_class_expression(
            checked,
            ctx,
            name.as_ref(),
            sc.as_deref(),
            body,
            *span,
            name_hint,
        ),
        AstExpr::ArrowFunction {
            params,
            body,
            is_async,
            span,
            ..
        } => {
            // Arrows inherit lexical `super` / derived this; not construct-return wrapping.
            // Also inherit field-init PerformEval early errors (E19.82.06 nested arrows).
            let prev_ctor_body = ctx.derived_ctor_body;
            ctx.derived_ctor_body = false;
            let params = lower_params(checked, ctx, params, super_class);
            let body = match body {
                draconic_ast::ArrowBody::Block(stmt) => {
                    lower_fn_body(checked, ctx, stmt, super_class)
                }
                draconic_ast::ArrowBody::Expr(expr) => {
                    vec![Stmt::Return {
                        value: Some(lower_expr(checked, ctx, expr, super_class)),
                    }]
                }
            };
            ctx.derived_ctor_body = prev_ctor_body;
            Expr::Function {
                name: None,
                params,
                body,
                is_async: *is_async,
                is_generator: false,
                is_arrow: true,
                is_method: false,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::ObjectExpression { properties, span } => Expr::Object {
            properties: properties
                .iter()
                .map(|p| match p {
                    AstObjectProp::Property { key, value, .. } => {
                        // NamedEvaluation: `{ id: class {} }` → constructor `.name === "id"` (E19.31).
                        let prop_name_hint: Option<String> = match key {
                            draconic_ast::ObjectKey::Ident(id) => Some(id.name.clone()),
                            draconic_ast::ObjectKey::String(s) => Some(s.value.to_string_lossy()),
                            draconic_ast::ObjectKey::Computed(_) => None,
                        };
                        ObjectProp::Property {
                            key: match key {
                                draconic_ast::ObjectKey::Ident(id) => {
                                    ObjectPropKey::Static(id.name.clone().into())
                                }
                                draconic_ast::ObjectKey::String(s) => {
                                    ObjectPropKey::Static(s.value.clone())
                                }
                                draconic_ast::ObjectKey::Computed(expr) => ObjectPropKey::Computed(
                                    lower_expr(checked, ctx, expr, super_class),
                                ),
                            },
                            value: lower_expr_hint(
                                checked,
                                ctx,
                                value,
                                super_class,
                                prop_name_hint.as_deref(),
                            ),
                        }
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
                            value: {
                                let prev_object_super = ctx.object_super;
                                ctx.object_super = true;
                                let params = lower_params(checked, ctx, params, None);
                                let body = lower_fn_body(checked, ctx, body, None);
                                ctx.object_super = prev_object_super;
                                Expr::Function {
                                    name: None,
                                    params,
                                    body,
                                    is_async: false,
                                    is_generator: false,
                                    is_arrow: false,
                                    is_method: true,
                                    ty: Type::Function,
                                }
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
        AstExpr::ArrayExpression { elements, span, .. } => Expr::Array {
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
                    AstExpr::Ident(id) => id.name.as_str(),
                    _ => panic!("private member property must be ident"),
                };
                // `obj?.#f` — optional on the private member itself.
                if *optional {
                    let obj = lower_expr(checked, ctx, object, super_class);
                    let fname = fname.to_string();
                    return optional_private_chain(ctx, obj, |ctx, o| {
                        private_member_get(ctx, &fname, o)
                    });
                }
                // `o?.c.#f` — private continues an optional chain; short-circuit on nullish
                // base before brand-check (not `(o?.c).#f` which would throw) (E19.53).
                if let AstExpr::MemberExpression {
                    object: inner,
                    property: mid_prop,
                    computed: mid_computed,
                    optional: true,
                    private: false,
                    ..
                } = object.as_ref()
                {
                    let base = lower_expr(checked, ctx, inner, super_class);
                    let mid_prop = if *mid_computed {
                        lower_expr(checked, ctx, mid_prop, super_class)
                    } else {
                        match mid_prop.as_ref() {
                            AstExpr::Ident(id) => Expr::String {
                                value: id.name.clone().into(),
                                ty: Type::String,
                            },
                            other => lower_expr(checked, ctx, other, super_class),
                        }
                    };
                    let fname = fname.to_string();
                    let mid_computed = *mid_computed;
                    return optional_private_chain(ctx, base, |ctx, o| {
                        let mid = Expr::Member {
                            object: Box::new(o),
                            property: Box::new(mid_prop.clone()),
                            computed: mid_computed,
                            optional: false,
                            ty: Type::Any,
                        };
                        private_member_get(ctx, &fname, mid)
                    });
                }
                let obj = lower_expr(checked, ctx, object, super_class);
                return private_member_get(ctx, fname, obj);
            }
            // `super.prop` → class with extends: `Parent.prototype.prop`; else keep `super.prop` (E19.34)
            if matches!(object.as_ref(), AstExpr::Super { .. }) {
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
                if super_class.is_none() && ctx.derived_super.is_none() {
                    return Expr::Member {
                        object: Box::new(Expr::Super { ty: Type::Any }),
                        property: Box::new(property),
                        computed: *computed,
                        optional: false,
                        ty: expr_ty(checked, *span),
                    };
                }
                let parent = if let Some(sid) = ctx.derived_super {
                    local_expr(sid)
                } else {
                    let parent_ast = super_class.expect("super_class present");
                    lower_expr(checked, ctx, parent_ast, None)
                };
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
                let member = Expr::Member {
                    object: Box::new(parent_proto),
                    property: Box::new(property),
                    computed: *computed,
                    optional: false,
                    ty: expr_ty(checked, *span),
                };
                // SuperProperty uses GetThisBinding — TDZ before super() (E19.82.03).
                if let Some(tid) = ctx.derived_this {
                    return Expr::Binary {
                        left: Box::new(assert_derived_this(tid)),
                        op: BinaryOp::Comma,
                        right: Box::new(member),
                        ty: expr_ty(checked, *span),
                    };
                }
                return member;
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
        let hint = single_name_binding_hint(&p.binding);
        out.push(Param {
            pattern: lower_binding_pattern(checked, ctx, &p.binding),
            default: p
                .default
                .as_ref()
                .map(|e| lower_expr_hint(checked, ctx, e, super_class, hint)),
            rest: p.rest,
        });
    }
    out
}

/// BindingIdentifier name for NamedEvaluation (SingleNameBinding only).
fn single_name_binding_hint(pat: &BindingPattern) -> Option<&str> {
    match pat {
        BindingPattern::Ident(id) => Some(id.name.as_str()),
        _ => None,
    }
}

fn expr_ty(checked: &CheckedProgram, span: Span) -> Type {
    // Missing types → Any: normal Programs are fully typed; direct-eval fragments
    // inlined for private access (E19.82.08) are parsed outside the CheckedProgram.
    checked.type_of_expr(span).unwrap_or(Type::Any)
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
            ArrayPatternElement::Pattern { binding, default } => {
                let hint = single_name_binding_hint(binding);
                ArrayPatternEl::Pattern {
                    binding: lower_binding_pattern(checked, ctx, binding),
                    default: default
                        .as_ref()
                        .map(|d| lower_expr_hint(checked, ctx, d, None, hint)),
                }
            }
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
            } => {
                let hint = single_name_binding_hint(binding);
                ObjectPatternEl::Prop {
                    key: match key {
                        draconic_ast::ObjectKey::Ident(id) => {
                            ObjectPropKey::Static(id.name.clone().into())
                        }
                        draconic_ast::ObjectKey::String(s) => {
                            ObjectPropKey::Static(s.value.clone())
                        }
                        draconic_ast::ObjectKey::Computed(expr) => {
                            ObjectPropKey::Computed(lower_expr(checked, ctx, expr, None))
                        }
                    },
                    binding: lower_binding_pattern(checked, ctx, binding),
                    shorthand: *shorthand,
                    default: default
                        .as_ref()
                        .map(|d| lower_expr_hint(checked, ctx, d, None, hint)),
                }
            }
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
                match key {
                    ObjectPropKey::Static(k) => {
                        let name = k.to_string_lossy();
                        if *shorthand {
                            out.push_str(&format!("prop shorthand {name}:\n"));
                        } else {
                            out.push_str(&format!("prop {name}:\n"));
                        }
                    }
                    ObjectPropKey::Computed(e) => {
                        if *shorthand {
                            out.push_str("prop shorthand Computed:\n");
                        } else {
                            out.push_str("prop Computed:\n");
                        }
                        dump_expr(e, level + 1, out);
                    }
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

fn dump_pattern_inline(pat: &Pattern, out: &mut String) {
    match pat {
        Pattern::Local(id) => out.push_str(&format!("%{}", id.0)),
        Pattern::Name(name) => out.push_str(name),
        Pattern::Member { .. } => out.push_str("<member>"),
        Pattern::Array(els) => {
            out.push('[');
            for (i, el) in els.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match el {
                    ArrayPatternEl::Elision => {}
                    ArrayPatternEl::Pattern { binding, default } => {
                        dump_pattern_inline(binding, out);
                        if default.is_some() {
                            out.push_str(" = …");
                        }
                    }
                    ArrayPatternEl::Rest(p) => {
                        out.push_str("...");
                        dump_pattern_inline(p, out);
                    }
                }
            }
            out.push(']');
        }
        Pattern::Object(props) => {
            out.push('{');
            for (i, p) in props.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match p {
                    ObjectPatternEl::Prop {
                        key,
                        binding,
                        shorthand,
                        default,
                    } => {
                        match key {
                            ObjectPropKey::Static(k) => out.push_str(&k.to_string_lossy()),
                            ObjectPropKey::Computed(_) => out.push_str("[…]"),
                        }
                        if !*shorthand {
                            out.push_str(": ");
                            dump_pattern_inline(binding, out);
                        }
                        if default.is_some() {
                            out.push_str(" = …");
                        }
                    }
                    ObjectPatternEl::Rest(p) => {
                        out.push_str("...");
                        dump_pattern_inline(p, out);
                    }
                }
            }
            out.push('}');
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
                BindingKind::Using => "using",
                BindingKind::AwaitUsing => "await using",
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
                BindingKind::Using => "using",
                BindingKind::AwaitUsing => "await using",
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
                BindingKind::Using => "using",
                BindingKind::AwaitUsing => "await using",
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
                    out.push(' ');
                    dump_pattern_inline(param, out);
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
        Expr::ImportCall {
            phase,
            source,
            options,
            ty,
        } => {
            indent(level, out);
            match phase {
                draconic_ast::ImportPhase::Evaluation => {
                    out.push_str(&format!("ImportCall : {ty}\n"))
                }
                draconic_ast::ImportPhase::Defer => {
                    out.push_str(&format!("ImportCall defer : {ty}\n"))
                }
                draconic_ast::ImportPhase::Source => {
                    out.push_str(&format!("ImportCall source : {ty}\n"))
                }
            }
            dump_expr(source, level + 1, out);
            if let Some(opts) = options {
                dump_expr(opts, level + 1, out);
            }
        }
        Expr::Super { ty } => {
            indent(level, out);
            out.push_str(&format!("Super : {ty}\n"));
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
            is_method,
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
            if *is_method {
                indent(level + 1, out);
                out.push_str("method: true\n");
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
    %8 ShadowRealm: function
    %9 undefined: any
    %10 globalThis: object
    %11 Object: function
    %12 Function: function
    %13 Array: function
    %14 String: function
    %15 Boolean: function
    %16 Error: function
    %17 TypeError: function
    %18 RangeError: function
    %19 ReferenceError: function
    %20 SyntaxError: function
    %21 URIError: function
    %22 EvalError: function
    %23 AggregateError: function
    %24 parseInt: function
    %25 parseFloat: function
    %26 isNaN: function
    %27 isFinite: function
    %28 encodeURI: function
    %29 decodeURI: function
    %30 encodeURIComponent: function
    %31 decodeURIComponent: function
    %32 JSON: object
    %33 Date: function
    %34 RegExp: function
    %35 Map: function
    %36 Set: function
    %37 WeakMap: function
    %38 WeakSet: function
    %39 ArrayBuffer: function
    %40 DataView: function
    %41 Int8Array: function
    %42 Uint8Array: function
    %43 Uint8ClampedArray: function
    %44 Int16Array: function
    %45 Uint16Array: function
    %46 Int32Array: function
    %47 Uint32Array: function
    %48 Float32Array: function
    %49 Float64Array: function
    %50 BigInt64Array: function
    %51 BigUint64Array: function
    %52 eval: function
    %53 escape: function
    %54 unescape: function
    %55 x: number
  body:
    Declare let %55
      init:
        Number 1 : number
    Expr
      Local %55 : number
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

    /// E19.82.07: nested private field shadows outer private method of same name.
    #[test]
    fn lower_nested_private_field_shadows_outer_method() {
        let module = lower_src(
            r#"
            class C {
                #m() { return "outer"; }
                outer() { return this.#m(); }
                B = class {
                    #m = "inner";
                    read(o) { return o.#m; }
                };
            }
        "#,
        );
        let dump = dump_module(&module);
        // Inner field needs a WeakMap; outer method needs a brand WeakSet / fn local.
        assert!(
            dump.contains("__drac_pf_") && dump.contains("__drac_pm_"),
            "expected both private field WeakMap and private method fn: {dump}"
        );
        assert!(
            !dump.contains("unknown private"),
            "shadowed private must resolve"
        );
    }

    /// E19.36: nested class body may read outer private names (no IR panic).
    #[test]
    fn lower_nested_class_accesses_outer_private() {
        let module = lower_src(
            r#"
            class C {
                #outer = 1;
                m() {
                    class D {
                        n(o) { return o.#outer; }
                    }
                    return new D().n(this);
                }
            }
        "#,
        );
        let dump = dump_module(&module);
        assert!(
            dump.contains("__drac_pf_") || dump.contains("WeakMap"),
            "expected private field WeakMap in dump"
        );
        assert!(
            !dump.contains("unknown private"),
            "nested access must resolve outer private"
        );
    }

    /// E19.36: compound / logical assign on private fields lower without panic.
    #[test]
    fn lower_private_compound_and_logical_assign() {
        let _ = lower_src(
            r#"
            class C {
                #x = 1;
                #y = 0;
                #z;
                step() {
                    this.#x += 2;
                    this.#y ||= 5;
                    this.#z ??= 9;
                    return this.#x + this.#y + this.#z;
                }
            }
        "#,
        );
        let _ = lower_src(
            r#"
            class C {
                #n = 0;
                inc() { return ++this.#n; }
                post() { return this.#n++; }
            }
        "#,
        );
    }

    /// E19.82.08: direct eval string with private access lowers via WeakMap, not raw `#`.
    #[test]
    fn lower_direct_eval_private_field_rewrites() {
        let module = lower_src(
            r#"
            class C {
                #m = 44;
                getWithEval() { return eval("this.#m"); }
            }
        "#,
        );
        let dump = dump_module(&module);
        assert!(
            dump.contains("__drac_pf_") || dump.contains("WeakMap"),
            "expected private field desugar in dump: {dump}"
        );
        // The eval string must not remain as a runtime `#` private access.
        assert!(
            !dump.contains("this.#m") && !dump.contains("\"this.#m\""),
            "eval private access should be inlined/desugared, dump: {dump}"
        );
    }
}
