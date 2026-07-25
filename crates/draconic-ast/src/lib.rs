use draconic_diagnostics::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expression {
        expr: Expr,
        span: Span,
    },
    /// `let name = init;` or `let name;`
    Let {
        name: Ident,
        init: Option<Expr>,
        span: Span,
    },
    Empty {
        span: Span,
    },
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
    TypeOf,
    Void,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    EqEq,
    NotEq,
    EqEqEq,
    NotEqEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UnaryOp::Plus => "+",
            UnaryOp::Minus => "-",
            UnaryOp::Not => "!",
            UnaryOp::TypeOf => "typeof",
            UnaryOp::Void => "void",
            UnaryOp::Delete => "delete",
        };
        write!(f, "{s}")
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
            BinaryOp::EqEq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::EqEqEq => "===",
            BinaryOp::NotEqEq => "!==",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
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
        Stmt::Let { name, init, .. } => {
            indent(level, out);
            out.push_str("Let\n");
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
