//! N08.11 / N08.16.32: native observations for linked ESM fixtures (E11) and
//! `export class` (E18.32).
//!
//! After the linker flattens static imports, module programs are ordinary IR with
//! mangled `__mN_*` locals. Compile-time evaluation covers named/default/namespace/
//! cyclic fixtures (number/string values, simple param calls, live `let` assign,
//! `import * as ns` via `__draconic_make_ns` pairs) plus exported class IIFEs
//! (`new`, instance props, methods, namespace class access). Emits Runtime prints
//! of entry top-level number/string locals (not mangled deps).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BYTES, PRINT_F64};

#[path = "es_modules_eval.rs"]
mod eval;

use eval::eval_body;

pub(crate) fn is_es_modules_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_modules(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_modules module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_modules(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_modules_module(module) {
        return None;
    }
    Some(emit_es_modules(module))
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Str(String),
    Undef,
    Fn(LocalId),
    /// Function expression value (namespace getters, etc.).
    FnExpr(FnRec),
    /// Class constructor + prototype methods (from export class IIFE).
    Class(ClassRec),
    /// Instance object id into `EvalCtx::heap`.
    Obj(u32),
    /// Module namespace object: export name → getter (Fn / FnExpr).
    Ns(HashMap<String, JsVal>),
    /// Temporary array for `__draconic_make_ns` pair construction only.
    Arr(Vec<JsVal>),
}

#[derive(Clone, Debug)]
struct FnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
struct ClassRec {
    ctor: FnRec,
    methods: HashMap<String, FnRec>,
}

#[derive(Clone, Debug)]
struct InstRec {
    props: HashMap<String, JsVal>,
    methods: HashMap<String, FnRec>,
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Return(JsVal),
}

struct EvalCtx<'a> {
    env: HashMap<LocalId, JsVal>,
    functions: &'a HashMap<LocalId, FnRec>,
    make_ns: Option<LocalId>,
    heap: HashMap<u32, InstRec>,
    next_obj: u32,
    this_obj: Option<u32>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !is_linked_module_ir(module) {
        return None;
    }
    if !body_ok(&module.body, &by_id) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    let mut functions: HashMap<LocalId, FnRec> = HashMap::new();
    let mut make_ns: Option<LocalId> = None;

    // Hoist function decls (JS / linked module bodies).
    for stmt in &module.body {
        if let Stmt::Function {
            local,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } = stmt
        {
            let param_ids = simple_param_ids(params)?;
            functions.insert(
                *local,
                FnRec {
                    params: param_ids,
                    body: body.clone(),
                },
            );
            env.insert(*local, JsVal::Fn(*local));
            if by_id
                .get(local)
                .is_some_and(|l| l.name == "__draconic_make_ns")
            {
                make_ns = Some(*local);
            }
        }
    }

    let mut ctx = EvalCtx {
        env,
        functions: &functions,
        make_ns,
        heap: HashMap::new(),
        next_obj: 1,
        this_obj: None,
    };

    match eval_body(&module.body, &mut ctx) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }

    let mut user_locals = Vec::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            if is_mangled_or_internal(&loc.name) {
                continue;
            }
            if matches!(
                ctx.env.get(local),
                Some(JsVal::Fn(_) | JsVal::Class(_) | JsVal::Obj(_) | JsVal::Ns(_))
            ) {
                continue;
            }
            if matches!(loc.ty, Type::Number | Type::Any | Type::String) {
                user_locals.push(*local);
            }
        }
    }

    if user_locals.is_empty() {
        return None;
    }

    let mut values = HashMap::new();
    for id in &user_locals {
        let v = ctx.env.get(id)?.clone();
        match &v {
            JsVal::Num(_) | JsVal::Str(_) => {
                values.insert(*id, v);
            }
            _ => return None,
        }
    }

    Some(ModuleInfo {
        user_locals,
        values,
    })
}

fn is_linked_module_ir(module: &Module) -> bool {
    module.locals.iter().any(|l| {
        l.name.starts_with("__m")
            || l.name.starts_with("__ns")
            || l.name.starts_with("__draconic_make_ns")
    })
}

fn is_mangled_or_internal(name: &str) -> bool {
    name.starts_with("__m")
        || name.starts_with("__ns")
        || name.starts_with("__draconic")
        || name == "arguments"
}

fn simple_param_ids(params: &[draconic_ir::Param]) -> Option<Vec<LocalId>> {
    let mut ids = Vec::with_capacity(params.len());
    for p in params {
        if p.default.is_some() || p.rest {
            return None;
        }
        match &p.pattern {
            Pattern::Local(id) => ids.push(*id),
            _ => return None,
        }
    }
    Some(ids)
}

fn body_ok(body: &[Stmt], by_id: &HashMap<LocalId, &Local>) -> bool {
    body.iter().all(|s| stmt_ok(s, by_id))
}

fn stmt_ok(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let Some(loc) = by_id.get(local) else {
                return false;
            };
            if !matches!(
                loc.ty,
                Type::Number | Type::Any | Type::String | Type::Function | Type::Object
            ) {
                return false;
            }
            match init {
                None => true,
                Some(e) => expr_ok(e, by_id),
            }
        }
        Stmt::Function {
            local,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => {
            // Namespace polyfill body is not CT-eval'd; pairs are interpreted at call.
            if by_id
                .get(local)
                .is_some_and(|l| l.name == "__draconic_make_ns")
            {
                return simple_param_ids(params).is_some();
            }
            simple_param_ids(params).is_some() && body_ok(body, by_id)
        }
        Stmt::Return { value } => match value {
            None => true,
            Some(e) => expr_ok(e, by_id),
        },
        Stmt::Block { body } => body_ok(body, by_id),
        Stmt::Expr { expr } => expr_ok(expr, by_id),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_ok(test, by_id)
                && stmt_ok(consequent, by_id)
                && alternate.as_ref().is_none_or(|a| stmt_ok(a, by_id))
        }
        _ => false,
    }
}

fn expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::NewTarget { .. }
        | Expr::IdentName { .. } => true,
        Expr::Local { id, .. } => by_id.contains_key(id),
        Expr::Unary { arg, .. } => expr_ok(arg, by_id),
        Expr::Binary {
            left, right, op, ..
        } => {
            matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
                    | BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
                    | BinaryOp::And
                    | BinaryOp::Or
                    | BinaryOp::Comma
                    | BinaryOp::In
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq
            ) && expr_ok(left, by_id)
                && expr_ok(right, by_id)
        }
        Expr::Assign {
            target: AssignTarget::Local(_),
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(value, by_id),
        Expr::Assign {
            target: AssignTarget::Member {
                object, property, ..
            },
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(object, by_id) && expr_ok(property, by_id) && expr_ok(value, by_id),
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            expr_ok(callee, by_id)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e, by_id),
                    _ => false,
                })
        }
        Expr::New { callee, args, .. } => {
            expr_ok(callee, by_id)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e, by_id),
                    _ => false,
                })
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => expr_ok(object, by_id) && expr_ok(property, by_id),
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => expr_ok(e, by_id),
            _ => false,
        }),
        Expr::Object { properties, .. } => properties.iter().all(|p| match p {
            ObjectProp::Property { value, .. } => expr_ok(value, by_id),
            _ => false,
        }),
        Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => {
            if simple_param_ids(params).is_none() {
                return false;
            }
            // Class builder IIFEs are extracted at eval time; skip deep body walk.
            if looks_like_class_iife(body) {
                return true;
            }
            body_ok(body, by_id)
        }
        _ => false,
    }
}

fn looks_like_class_iife(body: &[Stmt]) -> bool {
    let mut saw_strict = false;
    let mut saw_ctor_fn = false;
    let mut saw_return = false;
    for s in body {
        match s {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => saw_strict = true,
            Stmt::Declare {
                init: Some(Expr::Function { .. }),
                ..
            } => saw_ctor_fn = true,
            Stmt::Return {
                value: Some(Expr::Local { .. }),
            } => saw_return = true,
            _ => {}
        }
    }
    saw_strict && saw_ctor_fn && saw_return
}

struct Emitter {
    out: String,
    body: String,
    str_globals: Vec<(Vec<u8>, String)>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
            str_globals: Vec::new(),
        }
    }

    fn string_const(&mut self, s: &str) -> (String, usize) {
        let bytes = s.as_bytes().to_vec();
        let len = bytes.len();
        let name = format!("@.s{}", self.str_globals.len());
        self.str_globals.push((bytes, name.clone()));
        let data = format!(
            "getelementptr inbounds ([{n} x i8], ptr {name}, i64 0, i64 0)",
            n = len + 1
        );
        (data, len)
    }

    fn emit_num(&mut self, n: f64) {
        let lit = if n.is_nan() {
            "0x7FF8000000000000".to_string()
        } else if n.is_infinite() {
            if n.is_sign_negative() {
                "0xFFF0000000000000".into()
            } else {
                "0x7FF0000000000000".into()
            }
        } else {
            format!("{n:?}")
        };
        writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
    }

    fn emit_str(&mut self, s: &str) {
        let (data, len) = self.string_const(s);
        writeln!(
            self.body,
            "  {}",
            PRINT_BYTES.call(&format!("ptr {data}, i64 {len}"))
        )
        .ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_modules: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => self.emit_str(s),
                _ => return Err(diag("es_modules: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.11 linked ESM modules, incl. namespace)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        let mut globals: Vec<(Vec<u8>, String)> = self.str_globals.clone();
        globals.sort_by(|a, b| a.1.cmp(&b.1));
        for (bytes, name) in globals {
            let n = bytes.len() + 1;
            let mut esc = String::new();
            for &b in &bytes {
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
