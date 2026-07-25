use draconic_diagnostics::Span;
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

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expression {
        expr: Expr,
        span: Span,
    },
    /// `let name = init;`, `let name;`, or `const name = init;`
    Let {
        kind: BindingKind,
        name: Ident,
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
    /// `function name (params) { body }`
    FunctionDeclaration {
        name: Ident,
        params: Vec<Ident>,
        body: Box<Stmt>,
        span: Span,
    },
    /// `return;` or `return expr;`
    Return {
        argument: Option<Expr>,
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
    String(StringLit),
    Boolean {
        value: bool,
        span: Span,
    },
    Null {
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
        args: Vec<Expr>,
        span: Span,
    },
    /// Parenthesized expression — preserved for dump fidelity.
    Paren {
        expr: Box<Expr>,
        span: Span,
    },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLit {
    pub value: String,
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

fn dump_stmt(stmt: &Stmt, level: usize, out: &mut String) {
    match stmt {
        Stmt::Expression { expr, .. } => {
            indent(level, out);
            out.push_str("ExpressionStatement\n");
            dump_expr(expr, level + 1, out);
        }
        Stmt::Let {
            kind,
            name,
            init,
            ..
        } => {
            indent(level, out);
            match kind {
                BindingKind::Let => out.push_str("Let\n"),
                BindingKind::Const => out.push_str("Const\n"),
                BindingKind::Function => out.push_str("FunctionBinding\n"),
            }
            indent(level + 1, out);
            out.push_str(&format!("name: {}\n", name.name));
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
            body,
            ..
        } => {
            indent(level, out);
            out.push_str("FunctionDeclaration\n");
            indent(level + 1, out);
            out.push_str(&format!("name: {}\n", name.name));
            if !params.is_empty() {
                indent(level + 1, out);
                out.push_str("params:\n");
                for p in params {
                    indent(level + 2, out);
                    out.push_str(&format!("name: {}\n", p.name));
                }
            }
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::Return { argument, .. } => {
            indent(level, out);
            out.push_str("Return\n");
            if let Some(arg) = argument {
                dump_expr(arg, level + 1, out);
            }
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
        Expr::String(s) => {
            indent(level, out);
            out.push_str(&format!("String {:?}\n", s.value));
        }
        Expr::Boolean { value, .. } => {
            indent(level, out);
            out.push_str(&format!("Boolean {value}\n"));
        }
        Expr::Null { .. } => {
            indent(level, out);
            out.push_str("Null\n");
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
                out.push_str(&format!("arg[{i}]:\n"));
                dump_expr(arg, level + 2, out);
            }
        }
        Expr::Paren { expr, .. } => {
            indent(level, out);
            out.push_str("Paren\n");
            dump_expr(expr, level + 1, out);
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
                name: Ident {
                    name: "x".into(),
                    span: Span::dummy(),
                },
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
