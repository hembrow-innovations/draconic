//! L02.01 / L02.02: native observations for designed `groupBy` / `chunk` / `Deque`.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::rc::Rc;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

mod eval;

use eval::eval_body;

pub(crate) fn is_es_collections_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_collections(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_collections module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_collections(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_collections_module(module) {
        return None;
    }
    Some(emit_es_collections(module))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum BuiltinId {
    GlobalThis,
    GroupBy,
    Chunk,
    Deque,
    Array,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DequeOp {
    PushFront,
    PushBack,
    PopFront,
    PopBack,
}

#[derive(Clone, Debug, PartialEq)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Builtin(BuiltinId),
    ErrorInst {
        name: String,
        message: String,
    },
    Array(Vec<JsVal>),
    Object(Vec<(String, JsVal)>),
    UserFn {
        params: Vec<LocalId>,
        body: Vec<Stmt>,
    },
    Deque(Rc<RefCell<VecDeque<JsVal>>>),
    DequeMethod {
        kind: DequeOp,
        items: Rc<RefCell<VecDeque<JsVal>>>,
    },
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Throw(JsVal),
    Return(JsVal),
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_collections_surface(module, &by_id) {
        return None;
    }
    if !body_ok(&module.body) {
        return None;
    }
    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    for loc in &module.locals {
        if loc.name == "undefined" {
            env.insert(loc.id, JsVal::Undef);
        } else if let Some(b) = builtin_for_name(&loc.name) {
            env.insert(loc.id, JsVal::Builtin(b));
        }
    }
    match eval_body(&module.body, &mut env) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }
    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match env.get(local) {
                Some(v @ (JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_))) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String
                    ) {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(_) => {}
                None => return None,
            }
        }
    }
    if user_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        user_locals,
        values,
    })
}

fn builtin_for_name(name: &str) -> Option<BuiltinId> {
    match name {
        "globalThis" => Some(BuiltinId::GlobalThis),
        "groupBy" => Some(BuiltinId::GroupBy),
        "chunk" => Some(BuiltinId::Chunk),
        "Deque" => Some(BuiltinId::Deque),
        "Array" => Some(BuiltinId::Array),
        _ => None,
    }
}

fn module_has_collections_surface(module: &Module, by_id: &HashMap<LocalId, &Local>) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_has_collections_surface(s, by_id))
}

fn stmt_has_collections_surface(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Throw { value: e } => {
            expr_has_collections_surface(e, by_id)
        }
        Stmt::Return { value: Some(e) } => expr_has_collections_surface(e, by_id),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(|s| stmt_has_collections_surface(s, by_id))
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(|s| stmt_has_collections_surface(s, by_id)))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(|s| stmt_has_collections_surface(s, by_id)))
        }
        Stmt::Block { body } => body.iter().any(|s| stmt_has_collections_surface(s, by_id)),
        _ => false,
    }
}

fn expr_has_collections_surface(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, .. } => by_id
            .get(id)
            .is_some_and(|l| l.name == "groupBy" || l.name == "chunk" || l.name == "Deque"),
        Expr::IdentName { name, .. } => name == "groupBy" || name == "chunk" || name == "Deque",
        Expr::Unary { arg, .. } => expr_has_collections_surface(arg, by_id),
        Expr::Binary { left, right, .. } => {
            expr_has_collections_surface(left, by_id) || expr_has_collections_surface(right, by_id)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_collections_surface(test, by_id)
                || expr_has_collections_surface(consequent, by_id)
                || expr_has_collections_surface(alternate, by_id)
        }
        Expr::Member {
            object, property, ..
        } => {
            expr_has_collections_surface(object, by_id)
                || expr_has_collections_surface(property, by_id)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_has_collections_surface(callee, by_id)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_collections_surface(e, by_id),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_has_collections_surface(value, by_id),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) => expr_has_collections_surface(e, by_id),
            _ => false,
        }),
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } => expr_has_collections_surface(value, by_id),
            _ => false,
        }),
        Expr::Function { body, .. } => body.iter().any(|s| stmt_has_collections_surface(s, by_id)),
        _ => false,
    }
}

fn body_ok(body: &[Stmt]) -> bool {
    body.iter().all(stmt_ok)
}

fn stmt_ok(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init, .. } => match init {
            None => true,
            Some(e) => expr_ok(e),
        },
        Stmt::Expr { expr } => expr_ok(expr),
        Stmt::Throw { value } => expr_ok(value),
        Stmt::Return { value } => match value {
            None => true,
            Some(e) => expr_ok(e),
        },
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            match (handler.is_some(), handler_param) {
                (true, None) | (true, Some(Pattern::Local(_))) | (false, None) => {}
                _ => return false,
            }
            body_ok(block)
                && handler.as_ref().is_none_or(|h| body_ok(h))
                && finalizer.as_ref().is_none_or(|f| body_ok(f))
        }
        Stmt::Block { body } => body_ok(body),
        _ => false,
    }
}

fn simple_fn_params_ok(params: &[Param]) -> bool {
    params
        .iter()
        .all(|p| !p.rest && p.default.is_none() && matches!(p.pattern, Pattern::Local(_)))
}

fn expr_ok(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Null { .. }
        | Expr::Local { .. }
        | Expr::IdentName { .. } => true,
        Expr::Function {
            name: None,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => simple_fn_params_ok(params) && body_ok(body),
        Expr::Unary {
            op: UnaryOp::TypeOf | UnaryOp::Minus | UnaryOp::Plus,
            arg,
            ..
        } => expr_ok(arg),
        Expr::Binary {
            op:
                BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::And
                | BinaryOp::Or,
            left,
            right,
            ..
        } => expr_ok(left) && expr_ok(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => expr_ok(test) && expr_ok(consequent) && expr_ok(alternate),
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => expr_ok(object) && expr_ok(property),
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        }
        | Expr::New { callee, args, .. } => {
            expr_ok(callee)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_ok(value),
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => expr_ok(e),
            _ => false,
        }),
        Expr::Object { properties, .. } => properties.iter().all(|p| match p {
            ObjectProp::Property {
                key: ObjectPropKey::Static(_),
                value,
            } => expr_ok(value),
            _ => false,
        }),
        _ => false,
    }
}

struct Emitter {
    out: String,
    body: String,
    str_consts: Vec<(String, String)>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
            str_consts: Vec::new(),
        }
    }

    fn string_const(&mut self, s: &str) -> String {
        if let Some((_, name)) = self.str_consts.iter().find(|(v, _)| v == s) {
            return name.clone();
        }
        let name = format!("@.gstr.{}", self.str_consts.len());
        self.str_consts.push((s.to_string(), name.clone()));
        name
    }

    fn emit_num(&mut self, n: f64) {
        let lit = format!("{n:?}");
        writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_collections: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => {
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                JsVal::Bool(b) => {
                    let s = if *b { "true" } else { "false" };
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                _ => return Err(diag("es_collections: non-printable value")),
            }
        }
        writeln!(self.out, "; Draconic LLVM backend (L02 collections)").ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        for (s, name) in &self.str_consts {
            let n = s.len() + 1;
            let mut esc = String::new();
            for b in s.bytes() {
                match b {
                    b'\\' => esc.push_str("\\5C"),
                    b'"' => esc.push_str("\\22"),
                    c if (0x20..0x7f).contains(&c) => esc.push(c as char),
                    c => esc.push_str(&format!("\\{c:02X}")),
                }
            }
            writeln!(
                self.out,
                "{name} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        writeln!(self.out, "\ndefine i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        self.out.push_str(&self.body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn finish(self) -> String {
        self.out
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn compile_src(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn classifies_groupby_identity() {
        let m = compile_src(
            r#"
            let g = groupBy(["a", "b", "a"]);
            let n = g.a.length;
            "#,
        );
        assert!(is_es_collections_module(&m));
        let ir = emit_es_collections(&m).expect("emit");
        assert!(ir.contains("@main"), "{ir}");
    }

    #[test]
    fn classifies_chunk() {
        let m = compile_src(
            r#"
            let c = chunk([1, 2, 3, 4, 5], 2);
            let n = c.length;
            "#,
        );
        assert!(is_es_collections_module(&m));
        let ir = emit_es_collections(&m).expect("emit");
        assert!(ir.contains("@main"), "{ir}");
    }

    #[test]
    fn classifies_deque() {
        let m = compile_src(
            r#"
            let d = new Deque();
            d.pushBack(1);
            d.pushFront(0);
            let n = d.popFront();
            "#,
        );
        assert!(is_es_collections_module(&m));
        let ir = emit_es_collections(&m).expect("emit");
        assert!(ir.contains("@main"), "{ir}");
    }
}
