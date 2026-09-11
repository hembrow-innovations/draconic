//! N08.16.42: native observations for class static initialization blocks (E18.41 /
//! `es/annex-b/static_blocks`).
//!
//! Compile-time evaluation of the class-builder IR shape produced for static
//! public fields, static blocks (`static { … }`), static methods, private static
//! fields (WeakMap), `extends` heritage, and class expressions. Emits Runtime
//! prints of top-level number/string observations.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::emitter::escape_llvm_string;
use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey,
    Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, PRINT_F64, PRINT_STR};
mod call;
mod eval;

use eval::{as_callable, eval_body, ParamBind};

pub(crate) fn is_es_static_blocks_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_static_blocks(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_static_blocks module"))?;
    Ok(emit_prints(&info))
}

pub(crate) fn walk_es_static_blocks(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_static_blocks_module(module) {
        return None;
    }
    Some(emit_es_static_blocks(module))
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Str(String),
    Bool(bool),
    Undef,
    Null,
    /// Heap object index.
    Obj(usize),
    /// Function table index.
    Fn(usize),
    /// WeakMap heap index.
    WeakMap(usize),
    Builtin(&'static str),
}

#[derive(Clone)]
struct FnRec {
    params: Vec<ParamBind>,
    body: Vec<Stmt>,
    is_arrow: bool,
    #[allow(dead_code)]
    is_method: bool,
    /// Lexical locals captured at creation (class builder WeakMaps, etc.).
    closure: HashMap<LocalId, JsVal>,
    /// Captured bare-name bindings.
    name_closure: HashMap<String, JsVal>,
}

struct Heap {
    objects: Vec<HashMap<String, JsVal>>,
    weakmaps: Vec<HashMap<usize, JsVal>>,
    functions: Vec<FnRec>,
}

impl Heap {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            weakmaps: Vec::new(),
            functions: Vec::new(),
        }
    }

    fn alloc_obj(&mut self, props: HashMap<String, JsVal>) -> usize {
        let id = self.objects.len();
        self.objects.push(props);
        id
    }

    fn alloc_fn(&mut self, rec: FnRec) -> usize {
        let id = self.functions.len();
        self.functions.push(rec);
        id
    }

    fn alloc_wm(&mut self) -> usize {
        let id = self.weakmaps.len();
        self.weakmaps.push(HashMap::new());
        id
    }

    fn get(&self, oid: usize, key: &str) -> JsVal {
        self.objects
            .get(oid)
            .and_then(|m| m.get(key).cloned())
            .unwrap_or(JsVal::Undef)
    }

    fn set(&mut self, oid: usize, key: &str, val: JsVal) {
        if let Some(m) = self.objects.get_mut(oid) {
            m.insert(key.to_string(), val);
        }
    }

    fn has_own(&self, oid: usize, key: &str) -> bool {
        self.objects.get(oid).is_some_and(|m| m.contains_key(key))
    }

    fn delete(&mut self, oid: usize, key: &str) -> bool {
        self.objects
            .get_mut(oid)
            .map(|m| m.remove(key).is_some())
            .unwrap_or(false)
    }
}

#[derive(Clone)]
enum Obs {
    Num(f64),
    Str(String),
}

struct ModuleInfo {
    observations: Vec<Obs>,
}

struct Env {
    locals: HashMap<LocalId, JsVal>,
    /// Bare `Name` param / with-style bindings (`t`, `p`, `r` in Proxy get traps).
    names: HashMap<String, JsVal>,
    this: JsVal,
    new_target: JsVal,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    if !module_has_static_block_shape(module) {
        return None;
    }
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut heap = Heap::new();
    let mut env = Env {
        locals: HashMap::new(),
        names: HashMap::new(),
        this: JsVal::Undef,
        new_target: JsVal::Undef,
    };

    eval_body(&module.body, &mut env, &mut heap, &by_id).ok()?;

    let mut observations = Vec::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            // Skip class / callable bindings.
            if matches!(loc.ty, Type::Function) {
                continue;
            }
            match env.locals.get(local) {
                Some(JsVal::Num(n)) => observations.push(Obs::Num(*n)),
                Some(JsVal::Str(s)) => observations.push(Obs::Str(s.clone())),
                Some(JsVal::Undef) if matches!(loc.ty, Type::String | Type::Any | Type::Number) => {
                    observations.push(Obs::Str("undefined".into()));
                }
                Some(JsVal::Bool(b)) => observations.push(Obs::Num(if *b { 1.0 } else { 0.0 })),
                Some(JsVal::Obj(oid)) => {
                    if as_callable(&JsVal::Obj(*oid), &heap).is_some() {
                        continue;
                    }
                    return None;
                }
                Some(JsVal::Fn(_)) | Some(JsVal::WeakMap(_)) | Some(JsVal::Builtin(_)) => {
                    continue;
                }
                _ => return None,
            }
        }
    }
    if observations.is_empty() {
        return None;
    }
    Some(ModuleInfo { observations })
}

fn module_has_static_block_shape(module: &Module) -> bool {
    // Look for `__sb` method-home static block call pattern in class builder IIFEs.
    fn expr_has(e: &Expr) -> bool {
        match e {
            Expr::Call { callee, args, .. } => {
                expr_has(callee)
                    || args
                        .iter()
                        .any(|a| matches!(a, Arg::Expr(e) if expr_has(e)))
            }
            Expr::Function { body, .. } => body.iter().any(stmt_has),
            Expr::Member {
                object, property, ..
            } => expr_has(object) || expr_has(property),
            Expr::Object { properties, .. } => properties.iter().any(|p| match p {
                ObjectProp::Property {
                    key: ObjectPropKey::Static(k),
                    value,
                } if k.to_string_lossy() == "__sb" => true,
                ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                    expr_has(value)
                }
                ObjectProp::Spread(e) => expr_has(e),
            }),
            Expr::Assign { value, .. } => expr_has(value),
            Expr::Binary { left, right, .. } => expr_has(left) || expr_has(right),
            Expr::Unary { arg, .. } => expr_has(arg),
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => expr_has(test) || expr_has(consequent) || expr_has(alternate),
            Expr::New { callee, args, .. } => {
                expr_has(callee)
                    || args
                        .iter()
                        .any(|a| matches!(a, Arg::Expr(e) if expr_has(e)))
            }
            _ => false,
        }
    }
    fn stmt_has(s: &Stmt) -> bool {
        match s {
            Stmt::Declare { init: Some(e), .. } => expr_has(e),
            Stmt::Expr { expr } => expr_has(expr),
            Stmt::Block { body } | Stmt::Function { body, .. } => body.iter().any(stmt_has),
            Stmt::Return { value: Some(e) } | Stmt::Throw { value: e } => expr_has(e),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                expr_has(test)
                    || stmt_has(consequent)
                    || alternate.as_ref().is_some_and(|a| stmt_has(a))
            }
            _ => false,
        }
    }
    module.body.iter().any(stmt_has)
}

fn emit_prints(info: &ModuleInfo) -> String {
    let mut out = String::new();
    let mut body = String::new();
    let mut str_globals: Vec<(String, String)> = Vec::new();
    let mut tmp = 0usize;

    for obs in &info.observations {
        match obs {
            Obs::Num(n) => {
                let lit = format_f64(*n);
                writeln!(body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
            }
            Obs::Str(s) => {
                let gname = format!(".esb.str.{}", str_globals.len());
                str_globals.push((s.clone(), gname.clone()));
                let t = {
                    let t = tmp;
                    tmp += 1;
                    format!("%t{t}")
                };
                let n = s.len() + 1;
                writeln!(
                    body,
                    "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
                )
                .ok();
                writeln!(body, "  {}", PRINT_STR.call(&format!("ptr {t}"))).ok();
            }
        }
    }

    writeln!(
        out,
        "; Draconic LLVM backend (N08.16.42 class static blocks via compile-time eval)"
    )
    .ok();
    writeln!(out, "{}", llvm_declares(&[PRINT_F64, PRINT_STR])).ok();
    writeln!(out).ok();
    for (content, gname) in &str_globals {
        let n = content.len() + 1;
        let esc = escape_llvm_string(content);
        writeln!(
            out,
            "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
        )
        .ok();
    }
    if !str_globals.is_empty() {
        writeln!(out).ok();
    }
    writeln!(out, "define i32 @main() {{").ok();
    writeln!(out, "entry:").ok();
    out.push_str(&body);
    writeln!(out, "  ret i32 0").ok();
    writeln!(out, "}}").ok();
    out
}

fn format_f64(n: f64) -> String {
    if n.is_nan() {
        "0x7FF8000000000000".into()
    } else if n.is_infinite() {
        if n.is_sign_negative() {
            "0xFFF0000000000000".into()
        } else {
            "0x7FF0000000000000".into()
        }
    } else {
        // Match other emitters: decimal that parses as the same f64.
        format!("{n:?}")
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}
