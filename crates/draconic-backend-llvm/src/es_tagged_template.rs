//! N08.07.04: native observations for tagged templates (`es/strings/tagged_template`).
//!
//! `` tag`a${x}b` `` → call `tag(stringsArray, …interps)` where `stringsArray` is a
//! Runtime array of cooked quasi cstrings. Tag may be a function decl, a call that
//! returns a function, or `obj.method`. Tag bodies use array index/`.length`, string
//! concat (incl. number ToString), and `===` / `&&` for the empty-template bool case.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::BinaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, CSTR_CONCAT,
    CSTR_EQ_N, CSTR_FROM_U64, CSTR_LEN, GC_INIT, OBJECT_GET, OBJECT_SET, PRINT_BOOL, PRINT_STR,
};

/// Function indices encoded as `inttoptr i64 (idx + FN_TAG)`.
const FN_TAG: i64 = 1000;

pub(crate) fn is_es_tagged_template_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_tagged_template(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_tagged_template module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SlotTy {
    String,
    Bool,
    Object,
    Function,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RetKind {
    String,
    Bool,
    Function,
}

#[derive(Clone)]
struct FnInfo {
    idx: usize,
    params: Vec<LocalId>,
    body: Vec<Stmt>,
    ret: RetKind,
}

struct ModuleInfo {
    functions: Vec<FnInfo>,
    /// Local → function index (decls + method expr bindings).
    fn_binding: HashMap<LocalId, usize>,
    /// Top-level slots (declare order for non-fn).
    slots: Vec<(LocalId, SlotTy)>,
    /// Observation prints in declare order.
    print_locals: Vec<(LocalId, SlotTy)>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut fn_binding = HashMap::new();

    collect_functions(&module.body, &by_id, &mut functions, &mut fn_binding)?;

    let mut has_tt = false;
    for f in &functions {
        if !fn_body_ok(&f.body, &by_id, &fn_binding, &f.params) {
            return None;
        }
        if body_has_tagged(&f.body) {
            has_tt = true;
        }
    }

    let ret_of: HashMap<usize, RetKind> = functions.iter().map(|f| (f.idx, f.ret)).collect();

    let mut slots = Vec::new();
    let mut print_locals = Vec::new();
    let mut slot_of: HashMap<LocalId, SlotTy> = HashMap::new();

    for stmt in &module.body {
        match stmt {
            Stmt::Function { .. } => {}
            Stmt::Declare { local, init, .. } => {
                let init = init.as_ref()?;
                if fn_binding.contains_key(local) {
                    continue;
                }
                if let Expr::Object { properties, .. } = init {
                    if !object_ok(properties, &by_id, &fn_binding) {
                        return None;
                    }
                    slots.push((*local, SlotTy::Object));
                    slot_of.insert(*local, SlotTy::Object);
                    continue;
                }
                if matches!(init, Expr::Function { .. }) {
                    continue;
                }
                let kind =
                    slot_kind_of(init, &by_id, &fn_binding, &slot_of, &ret_of)?;
                if !expr_ok(init, &by_id, &fn_binding, &slot_of) {
                    return None;
                }
                if matches!(init, Expr::TaggedTemplate { .. }) {
                    has_tt = true;
                }
                slots.push((*local, kind));
                slot_of.insert(*local, kind);
                if matches!(kind, SlotTy::String | SlotTy::Bool) {
                    print_locals.push((*local, kind));
                }
            }
            _ => return None,
        }
    }

    if !has_tt || print_locals.is_empty() || functions.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        functions,
        fn_binding,
        slots,
        print_locals,
    })
}

fn collect_functions(
    stmts: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    out: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
) -> Option<()> {
    for stmt in stmts {
        match stmt {
            Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
            } => {
                if *is_async || *is_generator {
                    return None;
                }
                let ids = simple_params(params)?;
                let ret = infer_ret(body, by_id, fn_binding)?;
                let idx = out.len();
                fn_binding.insert(*local, idx);
                out.push(FnInfo {
                    idx,
                    params: ids,
                    body: body.clone(),
                    ret,
                });
                collect_functions(body, by_id, out, fn_binding)?;
            }
            Stmt::Declare {
                local,
                init: Some(Expr::Function {
                    params,
                    body,
                    is_async,
                    is_generator,
                    is_arrow,
                    ..
                }),
                ..
            } => {
                if *is_async || *is_generator || *is_arrow {
                    return None;
                }
                let ids = simple_params(params)?;
                let ret = infer_ret(body, by_id, fn_binding)?;
                let idx = out.len();
                fn_binding.insert(*local, idx);
                out.push(FnInfo {
                    idx,
                    params: ids,
                    body: body.clone(),
                    ret,
                });
                collect_functions(body, by_id, out, fn_binding)?;
            }
            Stmt::Declare {
                init: Some(Expr::Object { properties, .. }),
                ..
            } => {
                for p in properties {
                    if let ObjectProp::Property {
                        value:
                            Expr::Function {
                                params,
                                body,
                                is_async,
                                is_generator,
                                ..
                            },
                        ..
                    } = p
                    {
                        if *is_async || *is_generator {
                            return None;
                        }
                        let ids = simple_params(params)?;
                        let ret = infer_ret(body, by_id, fn_binding)?;
                        let idx = out.len();
                        out.push(FnInfo {
                            idx,
                            params: ids,
                            body: body.clone(),
                            ret,
                        });
                        collect_functions(body, by_id, out, fn_binding)?;
                    }
                }
            }
            Stmt::Declare {
                init: Some(e), ..
            } => collect_expr_fns(e, by_id, out, fn_binding)?,
            _ => {}
        }
    }
    Some(())
}

fn collect_expr_fns(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    out: &mut Vec<FnInfo>,
    fn_binding: &mut HashMap<LocalId, usize>,
) -> Option<()> {
    match expr {
        Expr::Function {
            params,
            body,
            is_async,
            is_generator,
            is_arrow,
            ..
        } => {
            if *is_async || *is_generator || *is_arrow {
                return None;
            }
            let ids = simple_params(params)?;
            let ret = infer_ret(body, by_id, fn_binding)?;
            let idx = out.len();
            out.push(FnInfo {
                idx,
                params: ids,
                body: body.clone(),
                ret,
            });
            collect_functions(body, by_id, out, fn_binding)?;
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            collect_expr_fns(tag, by_id, out, fn_binding)?;
            for e in expressions {
                collect_expr_fns(e, by_id, out, fn_binding)?;
            }
        }
        Expr::Call { callee, args, .. } => {
            collect_expr_fns(callee, by_id, out, fn_binding)?;
            for a in args {
                if let draconic_ir::Arg::Expr(e) = a {
                    collect_expr_fns(e, by_id, out, fn_binding)?;
                }
            }
        }
        Expr::Member { object, .. } => collect_expr_fns(object, by_id, out, fn_binding)?,
        Expr::Object { properties, .. } => {
            for p in properties {
                if let ObjectProp::Property { value, .. } = p {
                    collect_expr_fns(value, by_id, out, fn_binding)?;
                }
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_fns(left, by_id, out, fn_binding)?;
            collect_expr_fns(right, by_id, out, fn_binding)?;
        }
        _ => {}
    }
    Some(())
}

fn simple_params(params: &[Param]) -> Option<Vec<LocalId>> {
    let mut ids = Vec::new();
    for p in params {
        if p.rest || p.default.is_some() {
            return None;
        }
        match &p.pattern {
            Pattern::Local(id) => ids.push(*id),
            _ => return None,
        }
    }
    Some(ids)
}

fn infer_ret(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
) -> Option<RetKind> {
    let ret = find_return(body)?;
    match ret {
        Expr::Local { id, .. } if fn_binding.contains_key(id) => Some(RetKind::Function),
        Expr::Local { id, ty } => match ty {
            Type::Function => Some(RetKind::Function),
            Type::Boolean => Some(RetKind::Bool),
            Type::String => Some(RetKind::String),
            Type::Any => {
                if by_id.get(id).is_some_and(|l| l.ty == Type::Function) {
                    Some(RetKind::Function)
                } else {
                    Some(RetKind::String)
                }
            }
            _ => Some(RetKind::String),
        },
        Expr::Binary {
            op: BinaryOp::EqEqEq | BinaryOp::EqEq | BinaryOp::And | BinaryOp::Or,
            ..
        }
        | Expr::Boolean { .. } => Some(RetKind::Bool),
        Expr::Binary {
            op: BinaryOp::Add, ..
        }
        | Expr::String { .. }
        | Expr::Member { .. } => Some(RetKind::String),
        _ => Some(RetKind::String),
    }
}

fn find_return(body: &[Stmt]) -> Option<&Expr> {
    for s in body {
        match s {
            Stmt::Return { value: Some(v) } => return Some(v),
            Stmt::Block { body } => {
                if let Some(v) = find_return(body) {
                    return Some(v);
                }
            }
            _ => {}
        }
    }
    None
}

fn body_has_tagged(body: &[Stmt]) -> bool {
    body.iter().any(|s| stmt_has_tagged(s))
}

fn stmt_has_tagged(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { value: Some(e) } | Stmt::Expr { expr: e } => expr_has_tagged(e),
        Stmt::Declare {
            init: Some(e), ..
        } => expr_has_tagged(e),
        Stmt::Block { body } => body.iter().any(stmt_has_tagged),
        _ => false,
    }
}

fn expr_has_tagged(expr: &Expr) -> bool {
    match expr {
        Expr::TaggedTemplate { .. } => true,
        Expr::Binary { left, right, .. } => expr_has_tagged(left) || expr_has_tagged(right),
        Expr::Call { callee, args, .. } => {
            expr_has_tagged(callee)
                || args.iter().any(|a| {
                    matches!(a, draconic_ir::Arg::Expr(e) if expr_has_tagged(e))
                })
        }
        Expr::Member { object, .. } => expr_has_tagged(object),
        _ => false,
    }
}

fn object_ok(
    properties: &[ObjectProp],
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    properties.iter().all(|p| match p {
        ObjectProp::Property {
            key: ObjectPropKey::Static(_),
            value:
                Expr::Function {
                    params,
                    body,
                    is_async: false,
                    is_generator: false,
                    ..
                },
        } => {
            simple_params(params).is_some()
                && fn_body_ok(body, by_id, fn_binding, &simple_params(params).unwrap())
        }
        _ => false,
    })
}

fn slot_kind_of(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    slot_of: &HashMap<LocalId, SlotTy>,
    ret_of: &HashMap<usize, RetKind>,
) -> Option<SlotTy> {
    match expr {
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Boolean { .. } => Some(SlotTy::Bool),
        Expr::TaggedTemplate { tag, .. } => match tag.as_ref() {
            Expr::Local { id, .. } => {
                let idx = *fn_binding.get(id)?;
                match ret_of.get(&idx).copied().unwrap_or(RetKind::String) {
                    RetKind::Bool => Some(SlotTy::Bool),
                    RetKind::Function => Some(SlotTy::Function),
                    RetKind::String => Some(SlotTy::String),
                }
            }
            Expr::Call { .. } | Expr::Member { .. } => Some(SlotTy::String),
            _ => None,
        }
        Expr::Local { id, ty } => {
            if let Some(k) = slot_of.get(id) {
                return Some(*k);
            }
            match ty {
                Type::String => Some(SlotTy::String),
                Type::Boolean => Some(SlotTy::Bool),
                Type::Object => Some(SlotTy::Object),
                Type::Function => Some(SlotTy::Function),
                Type::Any => by_id.get(id).and_then(|l| match l.ty {
                    Type::String => Some(SlotTy::String),
                    Type::Boolean => Some(SlotTy::Bool),
                    Type::Object => Some(SlotTy::Object),
                    Type::Function => Some(SlotTy::Function),
                    _ => Some(SlotTy::String),
                }),
                _ => None,
            }
        }
        Expr::Binary {
            op: BinaryOp::EqEqEq | BinaryOp::EqEq | BinaryOp::And | BinaryOp::Or,
            ..
        } => Some(SlotTy::Bool),
        Expr::Binary {
            op: BinaryOp::Add, ..
        } => Some(SlotTy::String),
        Expr::Object { .. } => Some(SlotTy::Object),
        _ => None,
    }
}

fn fn_body_ok(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    body.iter()
        .all(|s| stmt_ok(s, by_id, fn_binding, params))
}

fn stmt_ok(
    stmt: &Stmt,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    match stmt {
        Stmt::Return { value: Some(e) } => expr_ok(e, by_id, fn_binding, &HashMap::new()) || {
            // allow param locals
            expr_ok_with_params(e, by_id, fn_binding, params)
        },
        Stmt::Return { value: None } => true,
        Stmt::Block { body } => body
            .iter()
            .all(|s| stmt_ok(s, by_id, fn_binding, params)),
        _ => false,
    }
}

fn expr_ok_with_params(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    match expr {
        Expr::Local { id, .. } => {
            params.contains(id) || fn_binding.contains_key(id) || by_id.contains_key(id)
        }
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } => true,
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            expr_ok_with_params(object, by_id, fn_binding, params)
                && (matches!(property.as_ref(), Expr::String { value, .. } if value.to_string_lossy() == "length")
                    || matches!(property.as_ref(), Expr::Number { .. })
                    || expr_ok_with_params(property, by_id, fn_binding, params))
        }
        Expr::Binary { left, right, op, .. } => {
            matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::EqEqEq
                    | BinaryOp::EqEq
                    | BinaryOp::And
                    | BinaryOp::Or
            ) && expr_ok_with_params(left, by_id, fn_binding, params)
                && expr_ok_with_params(right, by_id, fn_binding, params)
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            expr_ok_with_params(callee, by_id, fn_binding, params)
                && args.iter().all(|a| match a {
                    draconic_ir::Arg::Expr(e) => {
                        expr_ok_with_params(e, by_id, fn_binding, params)
                    }
                    _ => false,
                })
        }
        Expr::TaggedTemplate {
            tag,
            expressions,
            ..
        } => {
            expr_ok_with_params(tag, by_id, fn_binding, params)
                && expressions
                    .iter()
                    .all(|e| expr_ok_with_params(e, by_id, fn_binding, params))
        }
        _ => false,
    }
}

fn expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } => true,
        Expr::Local { id, .. } => {
            fn_binding.contains_key(id) || slot_of.contains_key(id) || by_id.contains_key(id)
        }
        Expr::TaggedTemplate {
            tag,
            expressions,
            ..
        } => {
            tag_ok(tag, by_id, fn_binding, slot_of)
                && expressions
                    .iter()
                    .all(|e| expr_ok(e, by_id, fn_binding, slot_of))
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            expr_ok(callee, by_id, fn_binding, slot_of)
                && args.iter().all(|a| match a {
                    draconic_ir::Arg::Expr(e) => expr_ok(e, by_id, fn_binding, slot_of),
                    _ => false,
                })
        }
        Expr::Member {
            object,
            property,
            optional: false,
            computed: false,
            ..
        } => {
            expr_ok(object, by_id, fn_binding, slot_of)
                && matches!(property.as_ref(), Expr::String { .. })
        }
        Expr::Object { properties, .. } => object_ok(properties, by_id, fn_binding),
        Expr::Binary { left, right, op, .. } => {
            matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::EqEqEq
                    | BinaryOp::EqEq
                    | BinaryOp::And
                    | BinaryOp::Or
            ) && expr_ok(left, by_id, fn_binding, slot_of)
                && expr_ok(right, by_id, fn_binding, slot_of)
        }
        _ => false,
    }
}

fn tag_ok(
    tag: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match tag {
        Expr::Local { id, .. } => fn_binding.contains_key(id),
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            matches!(callee.as_ref(), Expr::Local { id, .. } if fn_binding.contains_key(id))
                && args.is_empty()
        }
        Expr::Member {
            object,
            property,
            computed: false,
            optional: false,
            ..
        } => {
            matches!(object.as_ref(), Expr::Local { id, .. } if slot_of.get(id) == Some(&SlotTy::Object) || matches!(by_id.get(id).map(|l| l.ty), Some(Type::Object | Type::Any)))
                && matches!(property.as_ref(), Expr::String { .. })
        }
        _ => false,
    }
}

// --- Emitter ---

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    out: String,
    body: String,
    tmp: usize,
    str_n: usize,
    str_globals: Vec<(String, String)>,
    /// local → global ptr name
    slots: HashMap<LocalId, String>,
    /// param local → alloca while in function
    param_alloca: HashMap<LocalId, String>,
    /// method key "m" on object local → fn idx (filled during object emit)
    method_keys: HashMap<(LocalId, String), usize>,
    /// object local → list of (key, fn_idx) for property install
    object_methods: HashMap<LocalId, Vec<(String, usize)>>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        // Map object methods: walk module for Object props → function match by params.
        let mut object_methods: HashMap<LocalId, Vec<(String, usize)>> = HashMap::new();
        let mut method_keys = HashMap::new();
        for stmt in &module.body {
            if let Stmt::Declare {
                local,
                init: Some(Expr::Object { properties, .. }),
                ..
            } = stmt
            {
                for p in properties {
                    if let ObjectProp::Property {
                        key: ObjectPropKey::Static(k),
                        value:
                            Expr::Function {
                                params, body, ..
                            },
                    } = p
                    {
                        let ids = simple_params(params).unwrap_or_default();
                        if let Some(idx) = info.functions.iter().position(|f| {
                            f.params == ids && f.body == *body
                        }) {
                            let name = k.to_string_lossy();
                            object_methods
                                .entry(*local)
                                .or_default()
                                .push((name.clone(), idx));
                            method_keys.insert((*local, name), idx);
                        }
                    }
                }
            }
        }
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            tmp: 0,
            str_n: 0,
            str_globals: Vec::new(),
            slots: HashMap::new(),
            param_alloca: HashMap::new(),
            method_keys,
            object_methods,
        }
    }

    fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("%t{t}")
    }

    fn finish(self) -> String {
        self.out
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        writeln!(self.out, "; es_tagged_template N08.07.04").ok();
        writeln!(self.out, "target datalayout = \"e\"").ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[
                GC_INIT,
                ARRAY_NEW,
                ARRAY_GET,
                ARRAY_SET,
                ARRAY_LEN,
                CSTR_CONCAT,
                CSTR_FROM_U64,
                CSTR_LEN,
                CSTR_EQ_N,
                ALLOC_OBJECT,
                OBJECT_GET,
                OBJECT_SET,
                PRINT_STR,
                PRINT_BOOL,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for (id, kind) in &info.slots {
            let g = format!("es_tt_{}_{}", slot_tag(*kind), id.0);
            writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
            self.slots.insert(*id, format!("@{g}"));
        }
        if !info.slots.is_empty() {
            writeln!(self.out).ok();
        }

        // Emit each function.
        let fns = info.functions.clone();
        for f in &fns {
            self.emit_function(f)?;
        }

        // main
        self.body.clear();
        self.tmp = 0;
        self.param_alloca.clear();

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        writeln!(self.body, "  {}", GC_INIT.call("")).ok();

        for stmt in &self.module.body {
            self.emit_top_stmt(stmt)?;
        }

        for (id, kind) in &info.print_locals {
            let g = self
                .slots
                .get(id)
                .cloned()
                .ok_or_else(|| diag("es_tt: print slot missing"))?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load ptr, ptr {g}").ok();
            match kind {
                SlotTy::String => {
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotTy::Bool => {
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = ptrtoint ptr {v} to i64").ok();
                    let b = self.fresh();
                    writeln!(self.body, "  {b} = trunc i64 {i} to i8").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {b}"))).ok();
                }
                _ => {}
            }
        }

        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();

        for (content, gname) in self.str_globals.clone() {
            let n = content.len() + 1;
            let esc = escape_llvm_string(&content);
            writeln!(
                self.out,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        Ok(())
    }

    fn emit_function(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_params = std::mem::take(&mut self.param_alloca);

        self.tmp = 0;
        self.body.clear();
        self.param_alloca.clear();

        let mut sig = Vec::new();
        for (i, _) in f.params.iter().enumerate() {
            sig.push(format!("ptr %p{i}"));
        }
        let sig = sig.join(", ");

        writeln!(self.out, "define ptr @d_tt_fn_{}({sig}) {{", f.idx).ok();
        writeln!(self.out, "entry:").ok();

        let mut entry = String::new();
        for (i, pid) in f.params.iter().enumerate() {
            let ptr = format!("%pl{}", pid.0);
            self.param_alloca.insert(*pid, ptr.clone());
            writeln!(entry, "  {ptr} = alloca ptr, align 8").ok();
            writeln!(entry, "  store ptr %p{i}, ptr {ptr}").ok();
        }
        write!(self.out, "{entry}").ok();

        for stmt in &f.body {
            self.emit_fn_stmt(stmt, f)?;
        }
        if !self.body_ends_with_ret() {
            writeln!(self.body, "  ret ptr null").ok();
        }
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.param_alloca = saved_params;
        Ok(())
    }

    fn body_ends_with_ret(&self) -> bool {
        for line in self.body.lines().rev() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            return t.starts_with("ret ");
        }
        false
    }

    fn emit_fn_stmt(&mut self, stmt: &Stmt, f: &FnInfo) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(v) } => {
                let p = match f.ret {
                    RetKind::Bool => {
                        let b = self.emit_bool_expr(v)?;
                        let zext = self.fresh();
                        writeln!(self.body, "  {zext} = zext i1 {b} to i64").ok();
                        let p = self.fresh();
                        writeln!(self.body, "  {p} = inttoptr i64 {zext} to ptr").ok();
                        p
                    }
                    RetKind::Function => self.emit_fn_value(v)?,
                    RetKind::String => self.emit_stringy(v)?,
                };
                writeln!(self.body, "  ret ptr {p}").ok();
                Ok(())
            }
            Stmt::Return { value: None } => {
                writeln!(self.body, "  ret ptr null").ok();
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    if self.body_ends_with_ret() {
                        break;
                    }
                    self.emit_fn_stmt(s, f)?;
                }
                Ok(())
            }
            _ => Err(diag("es_tt: unsupported fn stmt")),
        }
    }

    fn emit_top_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Function { .. } => Ok(()),
            Stmt::Declare { local, init, .. } => {
                if self.info.fn_binding.contains_key(local) {
                    return Ok(());
                }
                let init = init
                    .as_ref()
                    .ok_or_else(|| diag("es_tt: declare needs init"))?;
                let g = self
                    .slots
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("es_tt: missing slot"))?;
                if let Expr::Object { .. } = init {
                    let obj = self.emit_object(*local, init)?;
                    writeln!(self.body, "  store ptr {obj}, ptr {g}").ok();
                    return Ok(());
                }
                let kind = self
                    .info
                    .slots
                    .iter()
                    .find(|(id, _)| id == local)
                    .map(|(_, k)| *k)
                    .ok_or_else(|| diag("es_tt: slot kind"))?;
                let v = match kind {
                    SlotTy::Bool => {
                        let b = self.emit_bool_expr(init)?;
                        let z = self.fresh();
                        writeln!(self.body, "  {z} = zext i1 {b} to i64").ok();
                        let p = self.fresh();
                        writeln!(self.body, "  {p} = inttoptr i64 {z} to ptr").ok();
                        p
                    }
                    SlotTy::String => self.emit_stringy(init)?,
                    SlotTy::Object => self.emit_object(*local, init)?,
                    SlotTy::Function => self.emit_fn_value(init)?,
                };
                writeln!(self.body, "  store ptr {v}, ptr {g}").ok();
                Ok(())
            }
            _ => Err(diag("es_tt: unsupported top stmt")),
        }
    }

    fn emit_object(&mut self, local: LocalId, expr: &Expr) -> Result<String, Diagnostic> {
        let Expr::Object { .. } = expr else {
            return Err(diag("es_tt: expected object"));
        };
        let obj = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ALLOC_OBJECT.call_to(&obj, "")
        )
        .ok();
        if let Some(methods) = self.object_methods.get(&local).cloned() {
            for (key, idx) in methods {
                let k = self.string_const(&key)?;
                let fv = self.fn_ptr_value(idx);
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {k}, ptr {fv}"))
                )
                .ok();
            }
        }
        Ok(obj)
    }

    fn fn_ptr_value(&mut self, idx: usize) -> String {
        let n = FN_TAG + idx as i64;
        let t = self.fresh();
        writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
        t
    }

    fn emit_fn_value(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => {
                let idx = *self
                    .info
                    .fn_binding
                    .get(id)
                    .ok_or_else(|| diag("es_tt: unknown fn local"))?;
                Ok(self.fn_ptr_value(idx))
            }
            _ => Err(diag("es_tt: unsupported fn value")),
        }
    }

    fn emit_stringy(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Number { raw, .. } => {
                let n = parse_nonneg_int(raw)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    CSTR_FROM_U64.call_to(&t, &format!("i64 {n}"))
                )
                .ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                if let Some(a) = self.param_alloca.get(id).cloned() {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {a}").ok();
                    return Ok(t);
                }
                if let Some(g) = self.slots.get(id).cloned() {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {g}").ok();
                    return Ok(t);
                }
                if self.info.fn_binding.contains_key(id) {
                    return self.emit_fn_value(expr);
                }
                Err(diag("es_tt: unknown string local"))
            }
            Expr::Member {
                object,
                property,
                computed,
                optional: false,
                ..
            } => {
                if !*computed {
                    if let Expr::String { value, .. } = property.as_ref() {
                        if value.to_string_lossy() == "length" {
                            // array.length → number → ToString
                            let arr = self.emit_arrayish(object)?;
                            let len = self.fresh();
                            writeln!(
                                self.body,
                                "  {}",
                                ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
                            )
                            .ok();
                            let t = self.fresh();
                            writeln!(
                                self.body,
                                "  {}",
                                CSTR_FROM_U64.call_to(&t, &format!("i64 {len}"))
                            )
                            .ok();
                            return Ok(t);
                        }
                    }
                }
                // strings[i]
                let arr = self.emit_arrayish(object)?;
                let idx = self.emit_index(property)?;
                let el = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_GET.call_to(&el, &format!("ptr {arr}, i64 {idx}"))
                )
                .ok();
                Ok(el)
            }
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } => {
                let l = self.emit_stringy(left)?;
                let r = self.emit_stringy(right)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    CSTR_CONCAT.call_to(&t, &format!("ptr {l}, ptr {r}"))
                )
                .ok();
                Ok(t)
            }
            Expr::TaggedTemplate {
                tag,
                quasis,
                expressions,
                ..
            } => self.emit_tagged(tag, quasis, expressions),
            Expr::Call {
                callee,
                args,
                optional: false,
                ..
            } if args.is_empty() => {
                // makeTag() → function ptr (used only as tag, but typed as any)
                let idx = self.resolve_fn_idx(callee)?;
                Ok(self.fn_ptr_value(idx))
            }
            _ => Err(diag(format!(
                "es_tt: unsupported stringy expr: {expr:?}"
            ))),
        }
    }

    fn emit_arrayish(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        // First param is always the strings array.
        match expr {
            Expr::Local { id, .. } => {
                if let Some(a) = self.param_alloca.get(id).cloned() {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {a}").ok();
                    return Ok(t);
                }
                Err(diag("es_tt: array local not a param"))
            }
            _ => Err(diag("es_tt: expected array local")),
        }
    }

    fn emit_index(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n = parse_nonneg_int(raw)?;
                Ok(n.to_string())
            }
            _ => Err(diag("es_tt: index must be number literal")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Boolean { value, .. } => {
                let t = self.fresh();
                if *value {
                    writeln!(self.body, "  {t} = icmp eq i32 0, 0").ok();
                } else {
                    writeln!(self.body, "  {t} = icmp eq i32 0, 1").ok();
                }
                Ok(t)
            }
            Expr::Binary {
                left,
                op: BinaryOp::And,
                right,
                ..
            } => {
                let l = self.emit_bool_expr(left)?;
                let r = self.emit_bool_expr(right)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = and i1 {l}, {r}").ok();
                Ok(t)
            }
            Expr::Binary {
                left,
                op: BinaryOp::EqEqEq | BinaryOp::EqEq,
                right,
                ..
            } => self.emit_eq(left, right),
            Expr::TaggedTemplate {
                tag,
                quasis,
                expressions,
                ..
            } => {
                // bool-returning tag
                let p = self.emit_tagged(tag, quasis, expressions)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = ptrtoint ptr {p} to i64").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = icmp ne i64 {i}, 0").ok();
                Ok(b)
            }
            _ => Err(diag("es_tt: unsupported bool expr")),
        }
    }

    fn emit_eq(&mut self, left: &Expr, right: &Expr) -> Result<String, Diagnostic> {
        // length === number, or string === string
        if is_length_member(left) {
            let arr = match left {
                Expr::Member { object, .. } => self.emit_arrayish(object)?,
                _ => unreachable!(),
            };
            let len = self.fresh();
            writeln!(
                self.body,
                "  {}",
                ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
            )
            .ok();
            let n = match right {
                Expr::Number { raw, .. } => parse_nonneg_int(raw)?,
                _ => return Err(diag("es_tt: length eq rhs number")),
            };
            let t = self.fresh();
            writeln!(self.body, "  {t} = icmp eq i64 {len}, {n}").ok();
            return Ok(t);
        }
        if is_length_member(right) {
            return self.emit_eq(right, left);
        }
        // string === string
        let l = self.emit_stringy(left)?;
        let r = self.emit_stringy(right)?;
        let ll = self.fresh();
        writeln!(self.body, "  {}", CSTR_LEN.call_to(&ll, &format!("ptr {l}"))).ok();
        let rl = self.fresh();
        writeln!(self.body, "  {}", CSTR_LEN.call_to(&rl, &format!("ptr {r}"))).ok();
        let eq = self.fresh();
        writeln!(
            self.body,
            "  {}",
            CSTR_EQ_N.call_to(&eq, &format!("ptr {l}, i64 {ll}, ptr {r}, i64 {rl}"))
        )
        .ok();
        let t = self.fresh();
        writeln!(self.body, "  {t} = icmp eq i32 {eq}, 1").ok();
        Ok(t)
    }

    fn emit_tagged(
        &mut self,
        tag: &Expr,
        quasis: &[draconic_ast::JsString],
        expressions: &[Expr],
    ) -> Result<String, Diagnostic> {
        // Build strings array of cooked quasis.
        let arr = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_NEW.call_to(&arr, &format!("i64 {}", quasis.len()))
        )
        .ok();
        for (i, q) in quasis.iter().enumerate() {
            let s = self.string_const(&q.to_string_lossy())?;
            writeln!(
                self.body,
                "  {}",
                ARRAY_SET.call(&format!("ptr {arr}, i64 {i}, ptr {s}"))
            )
            .ok();
        }

        // Resolve tag → fn index (static or runtime).
        let (static_idx, dyn_ptr) = self.resolve_tag(tag)?;

        // Evaluate interpolations as ptr (numbers → ToString cstr).
        let mut args: Vec<String> = vec![arr.clone()];
        for e in expressions {
            args.push(self.emit_stringy(e)?);
        }

        if let Some(idx) = static_idx {
            return self.call_fn_idx(idx, &args);
        }
        // Dynamic: switch on ptrtoint(dyn) - FN_TAG
        let dyn_ptr = dyn_ptr.ok_or_else(|| diag("es_tt: dynamic tag missing"))?;
        let raw = self.fresh();
        writeln!(self.body, "  {raw} = ptrtoint ptr {dyn_ptr} to i64").ok();
        let idxv = self.fresh();
        writeln!(self.body, "  {idxv} = sub i64 {raw}, {FN_TAG}").ok();

        // Call via switch among known arities — build a call helper per arity.
        // Max args in fixture: 1 array + 2 interps = 3.
        while args.len() < 3 {
            args.push("null".into());
        }
        let a0 = &args[0];
        let a1 = &args[1];
        let a2 = &args[2];

        let ret_slot = format!("%tt_ret{}", self.tmp);
        self.tmp += 1;
        writeln!(self.body, "  {ret_slot} = alloca ptr, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {ret_slot}").ok();

        let join = format!("tt_join{}", self.tmp);
        self.tmp += 1;
        // switch i64 idxv, label %tt_badN, [ cases ]
        let bad = format!("tt_bad{}", self.tmp);
        self.tmp += 1;
        let mut cases = String::new();
        let mut case_labels = Vec::new();
        for f in &self.info.functions {
            let lab = format!("tt_case{}_{}", f.idx, self.tmp);
            case_labels.push((f.idx, lab.clone()));
            cases.push_str(&format!(" i64 {}, label %{}", f.idx, lab));
        }
        self.tmp += 1;
        writeln!(
            self.body,
            "  switch i64 {idxv}, label %{bad} [{cases} ]"
        )
        .ok();

        for (idx, lab) in &case_labels {
            writeln!(self.body, "{lab}:").ok();
            let f = &self.info.functions[*idx];
            let call = self.format_call(*idx, f.params.len(), a0, a1, a2);
            let r = self.fresh();
            writeln!(self.body, "  {r} = {call}").ok();
            writeln!(self.body, "  store ptr {r}, ptr {ret_slot}").ok();
            writeln!(self.body, "  br label %{join}").ok();
        }
        writeln!(self.body, "{bad}:").ok();
        writeln!(self.body, "  store ptr null, ptr {ret_slot}").ok();
        writeln!(self.body, "  br label %{join}").ok();
        writeln!(self.body, "{join}:").ok();
        let out = self.fresh();
        writeln!(self.body, "  {out} = load ptr, ptr {ret_slot}").ok();
        Ok(out)
    }

    fn format_call(&self, idx: usize, nparams: usize, a0: &str, a1: &str, a2: &str) -> String {
        match nparams {
            0 => format!("call ptr @d_tt_fn_{idx}()"),
            1 => format!("call ptr @d_tt_fn_{idx}(ptr {a0})"),
            2 => format!("call ptr @d_tt_fn_{idx}(ptr {a0}, ptr {a1})"),
            _ => format!("call ptr @d_tt_fn_{idx}(ptr {a0}, ptr {a1}, ptr {a2})"),
        }
    }

    fn call_fn_idx(&mut self, idx: usize, args: &[String]) -> Result<String, Diagnostic> {
        let f = &self.info.functions[idx];
        let n = f.params.len();
        let mut parts = Vec::new();
        for i in 0..n {
            let a = args.get(i).map(|s| s.as_str()).unwrap_or("null");
            parts.push(format!("ptr {a}"));
        }
        let args_s = parts.join(", ");
        let t = self.fresh();
        writeln!(
            self.body,
            "  {t} = call ptr @d_tt_fn_{idx}({args_s})"
        )
        .ok();
        Ok(t)
    }

    fn resolve_tag(
        &mut self,
        tag: &Expr,
    ) -> Result<(Option<usize>, Option<String>), Diagnostic> {
        match tag {
            Expr::Local { id, .. } => {
                let idx = *self
                    .info
                    .fn_binding
                    .get(id)
                    .ok_or_else(|| diag("es_tt: tag not a fn"))?;
                Ok((Some(idx), None))
            }
            Expr::Call {
                callee,
                args,
                optional: false,
                ..
            } if args.is_empty() => {
                // makeTag() — call and get fn ptr
                let idx = self.resolve_fn_idx(callee)?;
                // call returns fn value
                let p = self.call_fn_idx(idx, &[])?;
                Ok((None, Some(p)))
            }
            Expr::Member {
                object,
                property,
                computed: false,
                optional: false,
                ..
            } => {
                let Expr::Local { id, .. } = object.as_ref() else {
                    return Err(diag("es_tt: method object must be local"));
                };
                let Expr::String { value, .. } = property.as_ref() else {
                    return Err(diag("es_tt: method key must be string"));
                };
                let key = value.to_string_lossy();
                if let Some(idx) = self.method_keys.get(&(*id, key.clone())).copied() {
                    return Ok((Some(idx), None));
                }
                // runtime get
                let obj = {
                    let g = self
                        .slots
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("es_tt: obj slot"))?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {g}").ok();
                    t
                };
                let k = self.string_const(&key)?;
                let p = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_GET.call_to(&p, &format!("ptr {obj}, ptr {k}"))
                )
                .ok();
                Ok((None, Some(p)))
            }
            _ => Err(diag("es_tt: unsupported tag")),
        }
    }

    fn resolve_fn_idx(&self, expr: &Expr) -> Result<usize, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => self
                .info
                .fn_binding
                .get(id)
                .copied()
                .ok_or_else(|| diag("es_tt: not a fn binding")),
            _ => Err(diag("es_tt: resolve_fn_idx")),
        }
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_tt_str.{}", self.str_n);
            self.str_n += 1;
            self.str_globals.push((s.to_string(), g.clone()));
            g
        };
        let t = self.fresh();
        let n = s.len() + 1;
        writeln!(
            self.body,
            "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
        )
        .ok();
        Ok(t)
    }
}

fn is_length_member(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Member {
            property,
            computed: false,
            optional: false,
            ..
        } if matches!(property.as_ref(), Expr::String { value, .. } if value.to_string_lossy() == "length")
    )
}

fn slot_tag(k: SlotTy) -> &'static str {
    match k {
        SlotTy::String => "s",
        SlotTy::Bool => "b",
        SlotTy::Object => "o",
        SlotTy::Function => "f",
    }
}

fn parse_nonneg_int(raw: &str) -> Result<u64, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    if let Ok(n) = cleaned.parse::<u64>() {
        return Ok(n);
    }
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(format!("es_tt: bad number {raw}")))?;
    if f >= 0.0 && f.fract() == 0.0 && f < (u64::MAX as f64) {
        Ok(f as u64)
    } else {
        Err(diag(format!("es_tt: non-int number {raw}")))
    }
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
