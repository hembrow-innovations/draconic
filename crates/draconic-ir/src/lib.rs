//! Shared IR lowered from checked Programs (ROADMAP B06).

use draconic_ast::{
    Arg as AstArg, ArrayElement as AstArrayElement, ArrayPatternElement, AssignOp, BinaryOp,
    BindingPattern, ClassElement, Expr as AstExpr, Ident, Stmt as AstStmt, UnaryOp, UpdateOp,
};
use draconic_check::{CheckedProgram, Type};
use draconic_diagnostics::Span;

pub use draconic_ast::BindingKind;
pub use draconic_check::{SymbolId as LocalId, Type as IrType};

/// Top-level IR unit both backends consume.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub locals: Vec<Local>,
    pub body: Vec<Stmt>,
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
    /// `let` / `const` `[a, b, ...rest] = init;`
    DeclareArrayPattern {
        kind: BindingKind,
        elements: Vec<ArrayPatternEl>,
        init: Expr,
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
        ty: Type,
    },
    /// `new callee(args?)`.
    New {
        callee: Box<Expr>,
        args: Vec<Arg>,
        ty: Type,
    },
    /// `async? function *? name? (params) { body }` expression value.
    Function {
        /// Named function expression binding (local to the body), if any.
        name: Option<LocalId>,
        params: Vec<Param>,
        body: Vec<Stmt>,
        is_async: bool,
        is_generator: bool,
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
    /// `obj.prop` or `obj[expr]` property read.
    Member {
        object: Box<Expr>,
        /// Non-computed: string key name as `String` expr. Computed: any expr.
        property: Box<Expr>,
        computed: bool,
        ty: Type,
    },
}

/// One element of an array literal after lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayElement {
    Expr(Expr),
    Spread(Expr),
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
pub struct ObjectProp {
    pub key: ObjectPropKey,
    pub value: Expr,
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
    /// `[a, b, ...rest] = …`
    ArrayPattern {
        elements: Vec<ArrayPatternEl>,
    },
}

/// Target of `++` / `--` after lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateTarget {
    Local(LocalId),
    Name(String),
}

/// One element of an array destructuring pattern in IR.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayPatternEl {
    /// Simple or nested binding.
    Pattern(Pattern),
    Rest(LocalId),
}

/// Binding pattern after lowering (ident or nested array).
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Local(LocalId),
    Array(Vec<ArrayPatternEl>),
}

/// Formal parameter in IR, optionally with a default initializer or rest flag.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub local: LocalId,
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
            | Expr::Template { ty, .. }
            | Expr::TaggedTemplate { ty, .. }
            | Expr::Boolean { ty, .. }
            | Expr::Null { ty }
            | Expr::This { ty }
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
    let locals: Vec<Local> = checked
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

    let mut body = Vec::new();
    for stmt in &checked.bound.program.body {
        body.extend(lower_stmt_expand(checked, stmt, None));
    }

    Module { locals, body }
}

/// Lower one AST statement, expanding constructs that become multiple IR stmts (e.g. class).
fn lower_stmt_expand(
    checked: &CheckedProgram,
    stmt: &AstStmt,
    super_class: Option<&AstExpr>,
) -> Vec<Stmt> {
    match stmt {
        AstStmt::ClassDeclaration {
            name,
            super_class: sc,
            body,
            ..
        } => lower_class(checked, name, sc.as_deref(), body),
        other => lower_stmt(checked, other, super_class)
            .into_iter()
            .collect(),
    }
}

fn lower_stmt_body(
    checked: &CheckedProgram,
    body: &[AstStmt],
    super_class: Option<&AstExpr>,
) -> Vec<Stmt> {
    body.iter()
        .flat_map(|s| lower_stmt_expand(checked, s, super_class))
        .collect()
}

fn lower_stmt(
    checked: &CheckedProgram,
    stmt: &AstStmt,
    super_class: Option<&AstExpr>,
) -> Option<Stmt> {
    match stmt {
        AstStmt::Empty { .. } => None,
        AstStmt::ClassDeclaration { .. } => {
            // Expanded via `lower_stmt_expand`.
            None
        }
        AstStmt::Expression { expr, .. } => Some(Stmt::Expr {
            expr: lower_expr(checked, expr, super_class),
        }),
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
                        .map(|e| lower_expr(checked, e, super_class)),
                    kind: *kind,
                })
            }
            BindingPattern::Array { elements, .. } => {
                let init = init
                    .as_ref()
                    .map(|e| lower_expr(checked, e, super_class))
                    .expect("array pattern declaration requires initializer");
                Some(Stmt::DeclareArrayPattern {
                    kind: *kind,
                    elements: lower_array_pattern_els(checked, elements),
                    init,
                })
            }
        }
        AstStmt::Block { body, .. } => {
            let body = lower_stmt_body(checked, body, super_class);
            Some(Stmt::Block { body })
        }
        AstStmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            let consequent = Box::new(
                lower_stmt(checked, consequent, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            let alternate = alternate.as_ref().map(|alt| {
                Box::new(
                    lower_stmt(checked, alt, super_class)
                        .unwrap_or(Stmt::Block { body: vec![] }),
                )
            });
            Some(Stmt::If {
                test: lower_expr(checked, test, super_class),
                consequent,
                alternate,
            })
        }
        AstStmt::While { test, body, .. } => {
            let body = Box::new(
                lower_stmt(checked, body, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::While {
                test: lower_expr(checked, test, super_class),
                body,
            })
        }
        AstStmt::DoWhile { body, test, .. } => {
            let body = Box::new(
                lower_stmt(checked, body, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::DoWhile {
                body,
                test: lower_expr(checked, test, super_class),
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
                .and_then(|s| lower_stmt(checked, s, super_class).map(Box::new));
            let test = test
                .as_ref()
                .map(|e| lower_expr(checked, e, super_class));
            let update = update
                .as_ref()
                .map(|e| lower_expr(checked, e, super_class));
            let body = Box::new(
                lower_stmt(checked, body, super_class)
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
                lower_stmt(checked, left, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            let body = Box::new(
                lower_stmt(checked, body, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::ForIn {
                left,
                right: lower_expr(checked, right, super_class),
                body,
            })
        }
        AstStmt::ForOf {
            left, right, body, ..
        } => {
            let left = Box::new(
                lower_stmt(checked, left, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            let body = Box::new(
                lower_stmt(checked, body, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::ForOf {
                left,
                right: lower_expr(checked, right, super_class),
                body,
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
                lower_stmt(checked, body, super_class)
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
                        .map(|e| lower_expr(checked, e, super_class)),
                    body: lower_stmt_body(checked, &c.body, super_class),
                })
                .collect();
            Some(Stmt::Switch {
                discriminant: lower_expr(checked, discriminant, super_class),
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
            let params = lower_params(checked, params, None);
            // Nested functions do not inherit `super`.
            let body = lower_fn_body(checked, body, None);
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
                .map(|e| lower_expr(checked, e, super_class)),
        }),
        AstStmt::Throw { argument, .. } => Some(Stmt::Throw {
            value: lower_expr(checked, argument, super_class),
        }),
        AstStmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            ..
        } => {
            let block = lower_fn_body(checked, block, super_class);
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
                .map(|h| lower_fn_body(checked, h, super_class));
            let finalizer = finalizer
                .as_ref()
                .map(|f| lower_fn_body(checked, f, super_class));
            Some(Stmt::Try {
                block,
                handler_param,
                handler,
                finalizer,
            })
        }
        AstStmt::With { object, body, .. } => Some(Stmt::With {
            object: lower_expr(checked, object, super_class),
            body: lower_fn_body(checked, body, super_class),
        }),
        AstStmt::ImportDeclaration { .. }
        | AstStmt::ExportNamedDeclaration { .. }
        | AstStmt::ExportDefaultDeclaration { .. } => {
            panic!("import/export must be linked before lower")
        }
        // Type aliases are erased (T02); no runtime value.
        AstStmt::TypeAlias { .. } => None,
    }
}

fn lower_fn_body(
    checked: &CheckedProgram,
    body: &AstStmt,
    super_class: Option<&AstExpr>,
) -> Vec<Stmt> {
    match body {
        AstStmt::Block { body, .. } => lower_stmt_body(checked, body, super_class),
        other => lower_stmt_expand(checked, other, super_class),
    }
}

/// Desugar `class Name extends? Super { constructor… methods… }` to function + prototype assigns.
fn lower_class(
    checked: &CheckedProgram,
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

    let mut ctor_params = Vec::new();
    let mut ctor_body = Vec::new();
    let mut methods: Vec<(&Ident, &Vec<draconic_ast::Param>, &AstStmt, bool, bool)> = Vec::new();

    for el in elements {
        match el {
            ClassElement::Constructor { params, body, .. } => {
                ctor_params = lower_params(checked, params, super_class);
                ctor_body = lower_fn_body(checked, body, super_class);
            }
            ClassElement::Method {
                name: method_name,
                params,
                body,
                is_static,
                is_generator,
                ..
            } => {
                methods.push((
                    method_name,
                    params,
                    body.as_ref(),
                    *is_static,
                    *is_generator,
                ));
            }
        }
    }

    let mut out = vec![Stmt::Function {
        local,
        params: ctor_params,
        body: ctor_body,
        is_async: false,
        is_generator: false,
    }];

    for (method_name, params, body, is_static, is_generator) in methods {
        let method_fn = Expr::Function {
            name: None,
            params: lower_params(checked, params, super_class),
            body: lower_fn_body(checked, body, super_class),
            is_async: false,
            is_generator,
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

    if let Some(sc) = super_class {
        // Child.prototype.__proto__ = Parent.prototype
        let parent = lower_expr(checked, sc, None);
        let parent_proto = Expr::Member {
            object: Box::new(parent.clone()),
            property: Box::new(Expr::String {
                value: "prototype".into(),
                ty: Type::String,
            }),
            computed: false,
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

    out
}

fn lower_arg(checked: &CheckedProgram, arg: &AstArg, super_class: Option<&AstExpr>) -> Arg {
    match arg {
        AstArg::Expr(e) => Arg::Expr(lower_expr(checked, e, super_class)),
        AstArg::Spread(e) => Arg::Spread(lower_expr(checked, e, super_class)),
    }
}

fn lower_expr(
    checked: &CheckedProgram,
    expr: &AstExpr,
    super_class: Option<&AstExpr>,
) -> Expr {
    match expr {
        AstExpr::Paren { expr: inner, .. } => lower_expr(checked, inner, super_class),
        AstExpr::ArrayPattern { .. } => {
            panic!("array pattern must only appear as assignment target")
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
        AstExpr::TemplateLiteral {
            quasis,
            expressions,
            span,
        } => Expr::Template {
            quasis: quasis.iter().map(|q| q.cooked.clone()).collect(),
            expressions: expressions
                .iter()
                .map(|e| lower_expr(checked, e, super_class))
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            span,
        } => Expr::TaggedTemplate {
            tag: Box::new(lower_expr(checked, tag, super_class)),
            quasis: quasis.iter().map(|q| q.cooked.clone()).collect(),
            expressions: expressions
                .iter()
                .map(|e| lower_expr(checked, e, super_class))
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
        AstExpr::Super { .. } => {
            panic!("bare `super` must appear as super(...) or super.prop after check")
        }
        AstExpr::Unary { op, arg, span } => Expr::Unary {
            op: *op,
            arg: Box::new(lower_expr(checked, arg, super_class)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Binary {
            left,
            op,
            right,
            span,
        } => Expr::Binary {
            left: Box::new(lower_expr(checked, left, super_class)),
            op: *op,
            right: Box::new(lower_expr(checked, right, super_class)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Conditional {
            test,
            consequent,
            alternate,
            span,
        } => Expr::Conditional {
            test: Box::new(lower_expr(checked, test, super_class)),
            consequent: Box::new(lower_expr(checked, consequent, super_class)),
            alternate: Box::new(lower_expr(checked, alternate, super_class)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Assign {
            target,
            op,
            value,
            span,
        } => {
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
                    ..
                } => {
                    let property = if *computed {
                        lower_expr(checked, property, super_class)
                    } else {
                        match property.as_ref() {
                            AstExpr::Ident(id) => Expr::String {
                                value: id.name.clone().into(),
                                ty: Type::String,
                            },
                            other => lower_expr(checked, other, super_class),
                        }
                    };
                    AssignTarget::Member {
                        object: Box::new(lower_expr(checked, object, super_class)),
                        property: Box::new(property),
                        computed: *computed,
                    }
                }
                AstExpr::ArrayPattern { elements, .. } => AssignTarget::ArrayPattern {
                    elements: lower_array_pattern_els(checked, elements),
                },
                _ => panic!("assign target must be ident, member, or array pattern after check"),
            };
            Expr::Assign {
                target,
                op: *op,
                value: Box::new(lower_expr(checked, value, super_class)),
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::Update {
            op,
            arg,
            prefix,
            span,
        } => {
            let AstExpr::Ident(id) = arg.as_ref() else {
                panic!("update target must be ident after check");
            };
            let target = if let Some(local) = checked.bound.resolve(id.span) {
                UpdateTarget::Local(local)
            } else {
                UpdateTarget::Name(id.name.clone())
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
            span,
        } => {
            // `super(args)` → `Parent.call(this, ...args)`
            if matches!(callee.as_ref(), AstExpr::Super { .. }) {
                let parent_ast = super_class
                    .expect("`super(...)` requires `extends` on the enclosing class");
                let parent = lower_expr(checked, parent_ast, None);
                let call_member = Expr::Member {
                    object: Box::new(parent),
                    property: Box::new(Expr::String {
                        value: "call".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    ty: Type::Function,
                };
                let mut call_args = Vec::with_capacity(args.len() + 1);
                call_args.push(Arg::Expr(Expr::This { ty: Type::Any }));
                for a in args {
                    call_args.push(lower_arg(checked, a, super_class));
                }
                return Expr::Call {
                    callee: Box::new(call_member),
                    args: call_args,
                    ty: expr_ty(checked, *span),
                };
            }
            // `super.m(args)` → `Parent.prototype.m.call(this, ...args)`
            if let AstExpr::MemberExpression {
                object,
                property,
                computed,
                ..
            } = callee.as_ref()
            {
                if matches!(object.as_ref(), AstExpr::Super { .. }) {
                    let parent_ast = super_class
                        .expect("`super.prop` requires `extends` on the enclosing class");
                    let parent = lower_expr(checked, parent_ast, None);
                    let parent_proto = Expr::Member {
                        object: Box::new(parent),
                        property: Box::new(Expr::String {
                            value: "prototype".into(),
                            ty: Type::String,
                        }),
                        computed: false,
                        ty: Type::Any,
                    };
                    let prop = if *computed {
                        lower_expr(checked, property, super_class)
                    } else {
                        match property.as_ref() {
                            AstExpr::Ident(id) => Expr::String {
                                value: id.name.clone().into(),
                                ty: Type::String,
                            },
                            other => lower_expr(checked, other, super_class),
                        }
                    };
                    let method = Expr::Member {
                        object: Box::new(parent_proto),
                        property: Box::new(prop),
                        computed: *computed,
                        ty: Type::Function,
                    };
                    let call_member = Expr::Member {
                        object: Box::new(method),
                        property: Box::new(Expr::String {
                            value: "call".into(),
                            ty: Type::String,
                        }),
                        computed: false,
                        ty: Type::Function,
                    };
                    let mut call_args = Vec::with_capacity(args.len() + 1);
                    call_args.push(Arg::Expr(Expr::This { ty: Type::Any }));
                    for a in args {
                        call_args.push(lower_arg(checked, a, super_class));
                    }
                    return Expr::Call {
                        callee: Box::new(call_member),
                        args: call_args,
                        ty: expr_ty(checked, *span),
                    };
                }
            }
            Expr::Call {
                callee: Box::new(lower_expr(checked, callee, super_class)),
                args: args
                    .iter()
                    .map(|a| lower_arg(checked, a, super_class))
                    .collect(),
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::New {
            callee,
            args,
            span,
        } => Expr::New {
            callee: Box::new(lower_expr(checked, callee, super_class)),
            args: args
                .iter()
                .map(|a| lower_arg(checked, a, super_class))
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
            let params = lower_params(checked, params, None);
            let body = lower_fn_body(checked, body, None);
            Expr::Function {
                name,
                params,
                body,
                is_async: *is_async,
                is_generator: *is_generator,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::ArrowFunction {
            params,
            body,
            is_async,
            span,
            ..
        } => {
            let params = lower_params(checked, params, None);
            let body = match body {
                draconic_ast::ArrowBody::Block(stmt) => lower_fn_body(checked, stmt, None),
                draconic_ast::ArrowBody::Expr(expr) => {
                    vec![Stmt::Return {
                        value: Some(lower_expr(checked, expr, None)),
                    }]
                }
            };
            Expr::Function {
                name: None,
                params,
                body,
                is_async: *is_async,
                is_generator: false,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::ObjectExpression { properties, span } => Expr::Object {
            properties: properties
                .iter()
                .map(|p| ObjectProp {
                    key: match &p.key {
                        draconic_ast::ObjectKey::Ident(id) => {
                            ObjectPropKey::Static(id.name.clone().into())
                        }
                        draconic_ast::ObjectKey::String(s) => {
                            ObjectPropKey::Static(s.value.clone())
                        }
                        draconic_ast::ObjectKey::Computed(expr) => {
                            ObjectPropKey::Computed(lower_expr(checked, expr, super_class))
                        }
                    },
                    value: lower_expr(checked, &p.value, super_class),
                })
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::ArrayExpression { elements, span } => Expr::Array {
            elements: elements
                .iter()
                .map(|el| match el {
                    AstArrayElement::Expr(e) => {
                        ArrayElement::Expr(lower_expr(checked, e, super_class))
                    }
                    AstArrayElement::Spread(e) => {
                        ArrayElement::Spread(lower_expr(checked, e, super_class))
                    }
                })
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::MemberExpression {
            object,
            property,
            computed,
            span,
        } => {
            // `super.prop` → `Parent.prototype.prop`
            if matches!(object.as_ref(), AstExpr::Super { .. }) {
                let parent_ast = super_class
                    .expect("`super.prop` requires `extends` on the enclosing class");
                let parent = lower_expr(checked, parent_ast, None);
                let parent_proto = Expr::Member {
                    object: Box::new(parent),
                    property: Box::new(Expr::String {
                        value: "prototype".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    ty: Type::Any,
                };
                let property = if *computed {
                    lower_expr(checked, property, super_class)
                } else {
                    match property.as_ref() {
                        AstExpr::Ident(id) => Expr::String {
                            value: id.name.clone().into(),
                            ty: Type::String,
                        },
                        other => lower_expr(checked, other, super_class),
                    }
                };
                return Expr::Member {
                    object: Box::new(parent_proto),
                    property: Box::new(property),
                    computed: *computed,
                    ty: expr_ty(checked, *span),
                };
            }
            let property = if *computed {
                lower_expr(checked, property, super_class)
            } else {
                match property.as_ref() {
                    AstExpr::Ident(id) => Expr::String {
                        value: id.name.clone().into(),
                        ty: Type::String,
                    },
                    other => lower_expr(checked, other, super_class),
                }
            };
            Expr::Member {
                object: Box::new(lower_expr(checked, object, super_class)),
                property: Box::new(property),
                computed: *computed,
                ty: expr_ty(checked, *span),
            }
        }
    }
}

fn lower_params(
    checked: &CheckedProgram,
    params: &[draconic_ast::Param],
    super_class: Option<&AstExpr>,
) -> Vec<Param> {
    params
        .iter()
        .map(|p| {
            let local = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.span == p.name.span)
                .map(|s| s.id)
                .expect("param binding must be declared");
            Param {
                local,
                default: p
                    .default
                    .as_ref()
                    .map(|e| lower_expr(checked, e, super_class)),
                rest: p.rest,
            }
        })
        .collect()
}

fn expr_ty(checked: &CheckedProgram, span: Span) -> Type {
    checked
        .type_of_expr(span)
        .expect("checked expression must have a type")
}

fn lower_array_pattern_els(
    checked: &CheckedProgram,
    elements: &[ArrayPatternElement],
) -> Vec<ArrayPatternEl> {
    elements
        .iter()
        .map(|el| match el {
            ArrayPatternElement::Pattern(p) => {
                ArrayPatternEl::Pattern(lower_binding_pattern(checked, p))
            }
            ArrayPatternElement::Rest(id) => {
                let local = checked
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == id.span)
                    .map(|s| s.id)
                    .or_else(|| checked.bound.resolve(id.span))
                    .expect("rest binding must be declared or resolved");
                ArrayPatternEl::Rest(local)
            }
        })
        .collect()
}

fn lower_binding_pattern(checked: &CheckedProgram, pat: &BindingPattern) -> Pattern {
    match pat {
        BindingPattern::Ident(id) => {
            let local = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.span == id.span)
                .map(|s| s.id)
                .or_else(|| checked.bound.resolve(id.span))
                .expect("pattern binding must be declared or resolved");
            Pattern::Local(local)
        }
        BindingPattern::Array { elements, .. } => {
            Pattern::Array(lower_array_pattern_els(checked, elements))
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

fn dump_array_pattern_els(elements: &[ArrayPatternEl], level: usize, out: &mut String) {
    for el in elements {
        match el {
            ArrayPatternEl::Pattern(p) => dump_pattern(p, level, out),
            ArrayPatternEl::Rest(id) => {
                indent(level, out);
                out.push_str(&format!("rest %{}\n", id.0));
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
        Pattern::Array(els) => {
            indent(level, out);
            out.push_str("ArrayPattern\n");
            dump_array_pattern_els(els, level + 1, out);
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
                BindingKind::Function => "function",
            };
            out.push_str(&format!("DeclareArrayPattern {kw}\n"));
            dump_array_pattern_els(elements, level + 1, out);
            indent(level + 1, out);
            out.push_str("init:\n");
            dump_expr(init, level + 2, out);
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
        Stmt::ForOf { left, right, body } => {
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
                AssignTarget::ArrayPattern { elements } => {
                    out.push_str(&format!("Assign {op} ArrayPattern : {ty}\n"));
                    dump_array_pattern_els(elements, level + 1, out);
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
            }
        }
        Expr::Call { callee, args, ty } => {
            indent(level, out);
            out.push_str(&format!("Call : {ty}\n"));
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
                match &prop.key {
                    ObjectPropKey::Static(k) => {
                        out.push_str(&format!("prop {:?}:\n", k.to_string_lossy()));
                        dump_expr(&prop.value, level + 2, out);
                    }
                    ObjectPropKey::Computed(k) => {
                        out.push_str("prop computed:\n");
                        indent(level + 2, out);
                        out.push_str("key:\n");
                        dump_expr(k, level + 3, out);
                        indent(level + 2, out);
                        out.push_str("value:\n");
                        dump_expr(&prop.value, level + 3, out);
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
                }
            }
        }
        Expr::Member {
            object,
            property,
            computed,
            ty,
        } => {
            indent(level, out);
            if *computed {
                out.push_str(&format!("Member computed : {ty}\n"));
            } else {
                out.push_str(&format!("Member : {ty}\n"));
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
        indent(level + 1, out);
        if p.rest {
            out.push_str(&format!("rest %{}\n", p.local.0));
        } else {
            out.push_str(&format!("%{}\n", p.local.0));
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
                expr: Expr::Call { callee, args, ty },
            } => {
                assert_eq!(*ty, Type::Any);
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
}
