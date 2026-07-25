//! Shared IR lowered from checked Programs (ROADMAP B06).

use draconic_ast::{
    AssignOp, BinaryOp, Expr as AstExpr, Stmt as AstStmt, UnaryOp, UpdateOp,
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
    /// `function name(params) { body }`
    Function {
        local: LocalId,
        params: Vec<LocalId>,
        body: Vec<Stmt>,
    },
    /// `return;` or `return value;`
    Return {
        value: Option<Expr>,
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
    Number {
        raw: String,
        ty: Type,
    },
    String {
        value: String,
        ty: Type,
    },
    Boolean {
        value: bool,
        ty: Type,
    },
    Null {
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
    /// `target = value` or compound `op=` — target is a local binding.
    Assign {
        target: LocalId,
        op: AssignOp,
        value: Box<Expr>,
        ty: Type,
    },
    /// Prefix or postfix `++` / `--` on a local binding.
    Update {
        op: UpdateOp,
        target: LocalId,
        prefix: bool,
        ty: Type,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        ty: Type,
    },
}

impl Expr {
    pub fn ty(&self) -> Type {
        match self {
            Expr::Local { ty, .. }
            | Expr::Number { ty, .. }
            | Expr::String { ty, .. }
            | Expr::Boolean { ty, .. }
            | Expr::Null { ty }
            | Expr::Unary { ty, .. }
            | Expr::Binary { ty, .. }
            | Expr::Conditional { ty, .. }
            | Expr::Assign { ty, .. }
            | Expr::Update { ty, .. }
            | Expr::Call { ty, .. } => *ty,
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
        if let Some(ir_stmt) = lower_stmt(checked, stmt) {
            body.push(ir_stmt);
        }
    }

    Module { locals, body }
}

fn lower_stmt(checked: &CheckedProgram, stmt: &AstStmt) -> Option<Stmt> {
    match stmt {
        AstStmt::Empty { .. } => None,
        AstStmt::Expression { expr, .. } => Some(Stmt::Expr {
            expr: lower_expr(checked, expr),
        }),
        AstStmt::Let {
            kind, name, init, ..
        } => {
            let local = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.span == name.span)
                .map(|s| s.id)
                .expect("let binding must be declared");
            Some(Stmt::Declare {
                local,
                init: init.as_ref().map(|e| lower_expr(checked, e)),
                kind: *kind,
            })
        }
        AstStmt::Block { body, .. } => {
            let body: Vec<Stmt> = body
                .iter()
                .filter_map(|s| lower_stmt(checked, s))
                .collect();
            Some(Stmt::Block { body })
        }
        AstStmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            let consequent = Box::new(
                lower_stmt(checked, consequent)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            let alternate = alternate.as_ref().map(|alt| {
                Box::new(
                    lower_stmt(checked, alt).unwrap_or(Stmt::Block { body: vec![] }),
                )
            });
            Some(Stmt::If {
                test: lower_expr(checked, test),
                consequent,
                alternate,
            })
        }
        AstStmt::While { test, body, .. } => {
            let body = Box::new(
                lower_stmt(checked, body).unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::While {
                test: lower_expr(checked, test),
                body,
            })
        }
        AstStmt::DoWhile { body, test, .. } => {
            let body = Box::new(
                lower_stmt(checked, body).unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::DoWhile {
                body,
                test: lower_expr(checked, test),
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
                .and_then(|s| lower_stmt(checked, s).map(Box::new));
            let test = test.as_ref().map(|e| lower_expr(checked, e));
            let update = update.as_ref().map(|e| lower_expr(checked, e));
            let body = Box::new(
                lower_stmt(checked, body).unwrap_or(Stmt::Block { body: vec![] }),
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
                lower_stmt(checked, left).unwrap_or(Stmt::Block { body: vec![] }),
            );
            let body = Box::new(
                lower_stmt(checked, body).unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::ForIn {
                left,
                right: lower_expr(checked, right),
                body,
            })
        }
        AstStmt::ForOf {
            left, right, body, ..
        } => {
            let left = Box::new(
                lower_stmt(checked, left).unwrap_or(Stmt::Block { body: vec![] }),
            );
            let body = Box::new(
                lower_stmt(checked, body).unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::ForOf {
                left,
                right: lower_expr(checked, right),
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
                lower_stmt(checked, body).unwrap_or(Stmt::Block { body: vec![] }),
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
                    test: c.test.as_ref().map(|e| lower_expr(checked, e)),
                    body: c
                        .body
                        .iter()
                        .filter_map(|s| lower_stmt(checked, s))
                        .collect(),
                })
                .collect();
            Some(Stmt::Switch {
                discriminant: lower_expr(checked, discriminant),
                cases,
            })
        }
        AstStmt::FunctionDeclaration {
            name, params, body, ..
        } => {
            let local = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.span == name.span)
                .map(|s| s.id)
                .expect("function binding must be declared");
            let params: Vec<LocalId> = params
                .iter()
                .map(|p| {
                    checked
                        .bound
                        .symbols()
                        .iter()
                        .find(|s| s.span == p.span)
                        .map(|s| s.id)
                        .expect("param binding must be declared")
                })
                .collect();
            let body = match body.as_ref() {
                AstStmt::Block { body, .. } => body
                    .iter()
                    .filter_map(|s| lower_stmt(checked, s))
                    .collect(),
                other => lower_stmt(checked, other).into_iter().collect(),
            };
            Some(Stmt::Function {
                local,
                params,
                body,
            })
        }
        AstStmt::Return { argument, .. } => Some(Stmt::Return {
            value: argument.as_ref().map(|e| lower_expr(checked, e)),
        }),
    }
}

fn lower_expr(checked: &CheckedProgram, expr: &AstExpr) -> Expr {
    match expr {
        AstExpr::Paren { expr: inner, .. } => lower_expr(checked, inner),
        AstExpr::Ident(id) => {
            let sym = checked
                .bound
                .resolve(id.span)
                .expect("ident must be resolved after check");
            let ty = expr_ty(checked, id.span);
            Expr::Local { id: sym, ty }
        }
        AstExpr::Number(n) => Expr::Number {
            raw: n.raw.clone(),
            ty: expr_ty(checked, n.span),
        },
        AstExpr::String(s) => Expr::String {
            value: s.value.clone(),
            ty: expr_ty(checked, s.span),
        },
        AstExpr::Boolean { value, span } => Expr::Boolean {
            value: *value,
            ty: expr_ty(checked, *span),
        },
        AstExpr::Null { span } => Expr::Null {
            ty: expr_ty(checked, *span),
        },
        AstExpr::Unary { op, arg, span } => Expr::Unary {
            op: *op,
            arg: Box::new(lower_expr(checked, arg)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Binary {
            left,
            op,
            right,
            span,
        } => Expr::Binary {
            left: Box::new(lower_expr(checked, left)),
            op: *op,
            right: Box::new(lower_expr(checked, right)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Conditional {
            test,
            consequent,
            alternate,
            span,
        } => Expr::Conditional {
            test: Box::new(lower_expr(checked, test)),
            consequent: Box::new(lower_expr(checked, consequent)),
            alternate: Box::new(lower_expr(checked, alternate)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Assign {
            target,
            op,
            value,
            span,
        } => {
            let AstExpr::Ident(id) = target.as_ref() else {
                panic!("assign target must be ident after check");
            };
            let local = checked
                .bound
                .resolve(id.span)
                .expect("assign target must be resolved after check");
            Expr::Assign {
                target: local,
                op: *op,
                value: Box::new(lower_expr(checked, value)),
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
            let local = checked
                .bound
                .resolve(id.span)
                .expect("update target must be resolved after check");
            Expr::Update {
                op: *op,
                target: local,
                prefix: *prefix,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::Call {
            callee,
            args,
            span,
        } => Expr::Call {
            callee: Box::new(lower_expr(checked, callee)),
            args: args.iter().map(|a| lower_expr(checked, a)).collect(),
            ty: expr_ty(checked, *span),
        },
    }
}

fn expr_ty(checked: &CheckedProgram, span: Span) -> Type {
    checked
        .type_of_expr(span)
        .expect("checked expression must have a type")
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
        } => {
            indent(level, out);
            out.push_str(&format!("Function %{}\n", local.0));
            if !params.is_empty() {
                indent(level + 1, out);
                out.push_str("params:\n");
                for p in params {
                    indent(level + 2, out);
                    out.push_str(&format!("%{}\n", p.0));
                }
            }
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
    }
}

fn dump_expr(expr: &Expr, level: usize, out: &mut String) {
    match expr {
        Expr::Local { id, ty } => {
            indent(level, out);
            out.push_str(&format!("Local %{} : {ty}\n", id.0));
        }
        Expr::Number { raw, ty } => {
            indent(level, out);
            out.push_str(&format!("Number {raw} : {ty}\n"));
        }
        Expr::String { value, ty } => {
            indent(level, out);
            out.push_str(&format!("String {value:?} : {ty}\n"));
        }
        Expr::Boolean { value, ty } => {
            indent(level, out);
            out.push_str(&format!("Boolean {value} : {ty}\n"));
        }
        Expr::Null { ty } => {
            indent(level, out);
            out.push_str(&format!("Null : {ty}\n"));
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
            out.push_str(&format!("Assign {op} %{} : {ty}\n", target.0));
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
            out.push_str(&format!("Update {kind} {op} %{} : {ty}\n", target.0));
        }
        Expr::Call { callee, args, ty } => {
            indent(level, out);
            out.push_str(&format!("Call : {ty}\n"));
            indent(level + 1, out);
            out.push_str("callee:\n");
            dump_expr(callee, level + 2, out);
            for (i, arg) in args.iter().enumerate() {
                indent(level + 1, out);
                out.push_str(&format!("arg[{i}]:\n"));
                dump_expr(arg, level + 2, out);
            }
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
        assert_eq!(module.locals.len(), 1);
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
    %0 x: number
  body:
    Declare let %0
      init:
        Number 1 : number
    Expr
      Local %0 : number
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
                assert!(matches!(args[0], Expr::Number { .. }));
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
                assert_eq!(*target, x.id);
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
                assert_eq!(*target, x.id);
                assert!(!*prefix);
                assert_eq!(*ty, Type::Number);
            }
            other => panic!("unexpected postfix: {other:?}"),
        }
    }
}
