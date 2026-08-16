//! N08.04.01–N08.04.06: native observations for ES object literals, property
//! access, simple property assignment, method call + `this`, `new`
//! constructors, prototypes, and object-literal sugar (E04.01–E04.06 /
//! `es/objects/*` incl. `object_lit_sugar`).
//!
//! Object values are Runtime GC heap ptrs; number props are stored as
//! `inttoptr` of integer bit-patterns (fixture uses small integers). Nested
//! objects store GC ptrs. Function-valued props and constructors store LLVM
//! method fn ptrs. Method/ctor calls use a uniform signature
//! `double (ptr this, double a0..a3)`. Function decls are ctor objects with a
//! `.prototype` own prop; `new C(args)` allocates an instance, sets
//! `[[Prototype]]` from `C.prototype`, calls the ctor, and yields the instance.
//! Runtime `object_get` walks the prototype chain so inherited methods resolve.
//! Property shorthand / method shorthand lower as static keys; computed keys
//! (`[expr]`) accept string locals or string literals. Top-level number/string
//! slots are module globals so methods can read free number locals.
//! Number locals from member reads / method returns are printed via `print_f64`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey,
    Param, Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, GC_INIT, OBJECT_GET, OBJECT_SET, OBJECT_SET_PROTO, PRINT_F64,
};

/// Max fixed args for method/ctor calling convention (fixtures use ≤2).
const MAX_METHOD_ARGS: usize = 4;

pub(crate) fn is_es_objects_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_objects(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_objects module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    Object,
    String,
}

#[derive(Clone)]
struct FnInfo {
    idx: usize,
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    number_locals: Vec<LocalId>,
    functions: Vec<FnInfo>,
    /// Function-declaration bindings → LLVM method index (`new` callees).
    fn_binding: HashMap<LocalId, usize>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut fn_binding = HashMap::new();
    collect_all_fns(&module.body, &by_id, &mut functions, &mut fn_binding)?;

    for f in &functions {
        if !method_body_ok(&f.body, &by_id, &functions, &fn_binding, &f.params) {
            return None;
        }
    }

    let mut slots = Vec::new();
    let mut number_locals = Vec::new();
    let mut has_object = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Function { local, .. } => {
                // N08.04.05: ctor binding is a heap object with `.prototype`.
                has_object = true;
                slots.push((*local, SlotTy::Object));
            }
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                let init = init.as_ref()?;
                if is_object_slot_ty(&loc.ty) || expr_is_object_init(init) {
                    if !object_expr_ok(init, &by_id, &functions, &fn_binding) {
                        return None;
                    }
                    has_object = true;
                    slots.push((*local, SlotTy::Object));
                } else if is_string_slot_ty(&loc.ty) || expr_is_string_init(init) {
                    if !string_expr_ok(init, &by_id) {
                        return None;
                    }
                    slots.push((*local, SlotTy::String));
                } else if is_number_slot_ty(&loc.ty) || expr_is_number_init(init) {
                    if !number_expr_ok(init, &by_id, &functions, &fn_binding) {
                        return None;
                    }
                    slots.push((*local, SlotTy::Number));
                    number_locals.push(*local);
                } else {
                    return None;
                }
            }
            Stmt::Expr { expr } => {
                if !member_assign_ok(expr, &by_id, &functions, &fn_binding)
                    && !number_expr_ok(expr, &by_id, &functions, &fn_binding)
                {
                    return None;
                }
            }
            _ => return None,
        }
    }

    if !has_object || number_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        slots,
        number_locals,
        functions,
        fn_binding,
    })
}

fn collect_all_fns(
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
                let param_ids = simple_param_ids(params, by_id)?;
                collect_all_fns(body, by_id, out, fn_binding)?;
                let idx = out.len();
                out.push(FnInfo {
                    idx,
                    params: param_ids,
                    body: body.clone(),
                });
                fn_binding.insert(*local, idx);
            }
            Stmt::Declare { init: Some(e), .. } => {
                collect_expr_fns(e, by_id, out, fn_binding)?
            }
            Stmt::Expr { expr } => collect_expr_fns(expr, by_id, out, fn_binding)?,
            Stmt::Block { body } => collect_all_fns(body, by_id, out, fn_binding)?,
            Stmt::Return { value: Some(e) } => collect_expr_fns(e, by_id, out, fn_binding)?,
            Stmt::Return { value: None } => {}
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                collect_expr_fns(test, by_id, out, fn_binding)?;
                collect_all_fns(std::slice::from_ref(consequent), by_id, out, fn_binding)?;
                if let Some(a) = alternate {
                    collect_all_fns(std::slice::from_ref(a), by_id, out, fn_binding)?;
                }
            }
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
            ..
        } => {
            if *is_async || *is_generator {
                return None;
            }
            let param_ids = simple_param_ids(params, by_id)?;
            collect_all_fns(body, by_id, out, fn_binding)?;
            let idx = out.len();
            out.push(FnInfo {
                idx,
                params: param_ids,
                body: body.clone(),
            });
            Some(())
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { value, .. } => {
                        collect_expr_fns(value, by_id, out, fn_binding)?
                    }
                    ObjectProp::Accessor { value, .. } => {
                        collect_expr_fns(value, by_id, out, fn_binding)?
                    }
                    ObjectProp::Spread(e) => collect_expr_fns(e, by_id, out, fn_binding)?,
                }
            }
            Some(())
        }
        Expr::Member { object, property, .. } => {
            collect_expr_fns(object, by_id, out, fn_binding)?;
            collect_expr_fns(property, by_id, out, fn_binding)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            collect_expr_fns(callee, by_id, out, fn_binding)?;
            for a in args {
                if let Arg::Expr(e) = a {
                    collect_expr_fns(e, by_id, out, fn_binding)?;
                }
            }
            Some(())
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_fns(left, by_id, out, fn_binding)?;
            collect_expr_fns(right, by_id, out, fn_binding)
        }
        Expr::Assign { value, target, .. } => {
            collect_expr_fns(value, by_id, out, fn_binding)?;
            if let AssignTarget::Member {
                object, property, ..
            } = target
            {
                collect_expr_fns(object, by_id, out, fn_binding)?;
                collect_expr_fns(property, by_id, out, fn_binding)?;
            }
            Some(())
        }
        _ => Some(()),
    }
}

fn simple_param_ids(params: &[Param], by_id: &HashMap<LocalId, &Local>) -> Option<Vec<LocalId>> {
    let mut ids = Vec::new();
    for p in params {
        if p.rest || p.default.is_some() {
            return None;
        }
        match &p.pattern {
            Pattern::Local(id) => {
                let _ = by_id.get(id)?;
                ids.push(*id);
            }
            _ => return None,
        }
    }
    if ids.len() > MAX_METHOD_ARGS {
        return None;
    }
    Some(ids)
}

fn find_fn_idx(params: &[Param], body: &[Stmt], functions: &[FnInfo]) -> Option<usize> {
    let ids: Vec<LocalId> = params
        .iter()
        .filter_map(|p| match &p.pattern {
            Pattern::Local(id) => Some(*id),
            _ => None,
        })
        .collect();
    if ids.len() != params.len() {
        return None;
    }
    // Match params + body so zero-arity methods (getX / m / g) stay distinct.
    functions
        .iter()
        .find(|f| f.params == ids && f.body == body)
        .map(|f| f.idx)
}

fn is_object_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Object | Type::Shape(_) | Type::Function)
}

fn is_number_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Number | Type::Any)
}

fn is_string_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::String)
}

fn expr_is_string_init(expr: &Expr) -> bool {
    matches!(expr, Expr::String { .. })
}

fn string_expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::String { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::String)
                || by_id.get(id).is_some_and(|l| matches!(l.ty, Type::String))
        }
        _ => false,
    }
}

fn prop_key_ok(key: &ObjectPropKey, by_id: &HashMap<LocalId, &Local>) -> bool {
    match key {
        ObjectPropKey::Static(_) => true,
        ObjectPropKey::Computed(e) => string_expr_ok(e, by_id),
    }
}

fn expr_is_object_init(expr: &Expr) -> bool {
    match expr {
        Expr::Object { .. } | Expr::New { .. } => true,
        Expr::Local { ty, .. } => is_object_slot_ty(ty),
        Expr::Member { ty, .. } => is_object_slot_ty(ty),
        _ => false,
    }
}

fn expr_is_number_init(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { ty, .. } => matches!(ty, Type::Number),
        Expr::Member {
            ty: Type::Number | Type::Any,
            ..
        } => true,
        Expr::Assign {
            op: AssignOp::Eq,
            ty: Type::Number | Type::Any,
            ..
        } => true,
        Expr::Call {
            ty: Type::Number | Type::Any,
            ..
        } => true,
        Expr::Binary {
            ty: Type::Number | Type::Any,
            ..
        } => true,
        _ => false,
    }
}

fn member_assign_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            object_expr_ok(object, by_id, functions, fn_binding)
                && member_key_ok(property)
                && (number_expr_ok(value, by_id, functions, fn_binding)
                    || function_expr_ok(value, by_id, functions, fn_binding)
                    || object_expr_ok(value, by_id, functions, fn_binding))
        }
        _ => false,
    }
}

fn object_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::This { .. } => true,
        Expr::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { key, value } => {
                        if !prop_key_ok(key, by_id) {
                            return false;
                        }
                        if object_expr_ok(value, by_id, functions, fn_binding) {
                            continue;
                        }
                        if number_expr_ok(value, by_id, functions, fn_binding) {
                            continue;
                        }
                        if function_expr_ok(value, by_id, functions, fn_binding) {
                            continue;
                        }
                        return false;
                    }
                    ObjectProp::Accessor { .. } | ObjectProp::Spread(_) => return false,
                }
            }
            true
        }
        Expr::Local { id, ty } => {
            fn_binding.contains_key(id)
                || is_object_slot_ty(ty)
                || by_id.get(id).is_some_and(|l| {
                    is_object_slot_ty(&l.ty)
                        || matches!(l.ty, Type::Any | Type::Function)
                        || fn_binding.contains_key(id)
                })
        }
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && object_expr_ok(object, by_id, functions, fn_binding)
                && member_key_ok(property)
        }
        Expr::New {
            callee,
            args,
            ..
        } => {
            let Expr::Local { id, .. } = callee.as_ref() else {
                return false;
            };
            if !fn_binding.contains_key(id) {
                return false;
            }
            if args.len() > MAX_METHOD_ARGS {
                return false;
            }
            args.iter().all(|a| match a {
                Arg::Expr(e) => number_expr_ok(e, by_id, functions, fn_binding),
                Arg::Spread(_) => false,
            })
        }
        _ => false,
    }
}

fn function_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    match expr {
        Expr::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if *is_async || *is_generator {
                return false;
            }
            let Some(param_ids) = simple_param_ids(params, by_id) else {
                return false;
            };
            if find_fn_idx(params, body, functions).is_none() {
                return false;
            }
            method_body_ok(body, by_id, functions, fn_binding, &param_ids)
        }
        _ => false,
    }
}

fn method_body_ok(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    for stmt in body {
        match stmt {
            Stmt::Return { value: Some(e) } => {
                if !number_expr_ok_in_method(e, by_id, functions, fn_binding, params) {
                    return false;
                }
            }
            Stmt::Return { value: None } => {}
            Stmt::Block { body } => {
                if !method_body_ok(body, by_id, functions, fn_binding, params) {
                    return false;
                }
            }
            Stmt::Expr { expr } => {
                if !ctor_or_method_expr_ok(expr, by_id, functions, fn_binding, params) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

/// Method/ctor statement expressions: number exprs, or `this.k =` number/function.
fn ctor_or_method_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    match expr {
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let obj_ok = object_expr_ok(object, by_id, functions, fn_binding)
                || matches!(object.as_ref(), Expr::This { .. });
            if !obj_ok || !member_key_ok(property) {
                return false;
            }
            number_expr_ok_in_method(value, by_id, functions, fn_binding, params)
                || function_expr_ok(value, by_id, functions, fn_binding)
        }
        _ => number_expr_ok_in_method(expr, by_id, functions, fn_binding, params),
    }
}

fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
) -> bool {
    number_expr_ok_in_method(expr, by_id, functions, fn_binding, &[])
}

fn number_expr_ok_in_method(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    params: &[LocalId],
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            if params.contains(id) {
                return true;
            }
            matches!(ty, Type::Number | Type::Any)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::This { .. } => false, // this alone is object, not number
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && member_key_ok(property)
                && (object_expr_ok(object, by_id, functions, fn_binding)
                    || matches!(object.as_ref(), Expr::This { .. }))
        }
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            (object_expr_ok(object, by_id, functions, fn_binding)
                || matches!(object.as_ref(), Expr::This { .. }))
                && member_key_ok(property)
                && number_expr_ok_in_method(value, by_id, functions, fn_binding, params)
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem,
            right,
            ..
        } => {
            number_expr_ok_in_method(left, by_id, functions, fn_binding, params)
                && number_expr_ok_in_method(right, by_id, functions, fn_binding, params)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return false;
            }
            if !args.iter().all(|a| match a {
                Arg::Expr(e) => number_expr_ok_in_method(e, by_id, functions, fn_binding, params),
                Arg::Spread(_) => false,
            }) {
                return false;
            }
            // Method call: obj.m(...) / obj["m"](...)
            match callee.as_ref() {
                Expr::Member {
                    object,
                    property,
                    optional: mop,
                    ..
                } => {
                    !*mop
                        && member_key_ok(property)
                        && object_expr_ok(object, by_id, functions, fn_binding)
                        && args.len() <= MAX_METHOD_ARGS
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn member_key_ok(property: &Expr) -> bool {
    matches!(property, Expr::String { .. })
}

fn number_global_name(id: LocalId) -> String {
    format!("es_obj_n_{}", id.0)
}

fn string_global_name(id: LocalId) -> String {
    format!("es_obj_s_{}", id.0)
}

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    slot_of: HashMap<LocalId, SlotTy>,
    allocas: HashMap<LocalId, String>,
    /// Method param local → alloca name (only while emitting a method).
    param_allocas: HashMap<LocalId, String>,
    /// Current method `this` SSA value (ptr), if any.
    this_ssa: Option<String>,
    str_globals: Vec<(String, String)>,
    out: String,
    body: String,
    tmp: usize,
    str_n: usize,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        Self {
            module,
            info,
            slot_of: HashMap::new(),
            allocas: HashMap::new(),
            param_allocas: HashMap::new(),
            this_ssa: None,
            str_globals: Vec::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
            str_n: 0,
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("%t{t}")
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for (id, ty) in &info.slots {
            self.slot_of.insert(*id, *ty);
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.04 ES objects + new/prototype via Runtime ABI)"
        )
        .ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[
                GC_INIT,
                ALLOC_OBJECT,
                OBJECT_SET,
                OBJECT_GET,
                OBJECT_SET_PROTO,
                PRINT_F64,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        // Number/string slots as module globals so method bodies can load free vars.
        for (id, kind) in &info.slots {
            match kind {
                SlotTy::Number => {
                    let g = number_global_name(*id);
                    writeln!(
                        self.out,
                        "@{g} = internal global double 0.00000000000000000e+00, align 8"
                    )
                    .ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::String => {
                    let g = string_global_name(*id);
                    writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::Object => {}
            }
        }
        if info
            .slots
            .iter()
            .any(|(_, k)| matches!(k, SlotTy::Number | SlotTy::String))
        {
            writeln!(self.out).ok();
        }

        // Emit method/ctor functions first (collect string globals into self.str_globals).
        for f in &info.functions.clone() {
            self.emit_method_fn(f)?;
        }

        // Main body into self.body — object slots stay stack allocas.
        for (id, kind) in &info.slots {
            if *kind != SlotTy::Object {
                continue;
            }
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
            writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for id in &info.number_locals {
            let ptr = self.number_slot_ptr(*id)?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
        }

        for (content, gname) in self.str_globals.clone() {
            let n = content.len() + 1;
            let esc = escape_llvm_string(&content);
            writeln!(
                self.out,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        if !self.str_globals.is_empty() {
            writeln!(self.out).ok();
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        writeln!(self.out, "  {}", GC_INIT.call("")).ok();
        self.out.push_str(&self.body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_method_fn(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let name = format!("m_fn_{}", f.idx);
        // Uniform signature: double (ptr %this, double %a0, ... %a{MAX-1})
        let mut params_s = String::from("ptr %this");
        for i in 0..MAX_METHOD_ARGS {
            write!(params_s, ", double %a{i}").ok();
        }
        writeln!(self.out, "define double @{name}({params_s}) {{").ok();
        writeln!(self.out, "entry:").ok();

        // Save outer body/this/params and emit into a fresh buffer that becomes the fn body.
        let saved_body = std::mem::take(&mut self.body);
        let saved_this = self.this_ssa.take();
        let saved_params = std::mem::take(&mut self.param_allocas);
        let saved_allocas = std::mem::take(&mut self.allocas);

        self.this_ssa = Some("%this".to_string());

        for (i, pid) in f.params.iter().enumerate() {
            let ptr = format!("%p{}", pid.0);
            writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
            writeln!(self.body, "  store double %a{i}, ptr {ptr}").ok();
            self.param_allocas.insert(*pid, ptr);
        }

        // Default return 0 if fall-off.
        let mut saw_return = false;
        for stmt in &f.body {
            if matches!(stmt, Stmt::Return { .. }) {
                saw_return = true;
            }
            self.emit_method_stmt(stmt)?;
        }
        if !saw_return {
            writeln!(
                self.body,
                "  ret double 0.00000000000000000e+00"
            )
            .ok();
        }

        self.out.push_str(&self.body);
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.this_ssa = saved_this;
        self.param_allocas = saved_params;
        self.allocas = saved_allocas;
        Ok(())
    }

    fn emit_method_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(e) } => {
                let v = self.emit_number_expr(e)?;
                writeln!(self.body, "  ret double {v}").ok();
                Ok(())
            }
            Stmt::Return { value: None } => {
                writeln!(
                    self.body,
                    "  ret double 0.00000000000000000e+00"
                )
                .ok();
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_method_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Expr { expr } => self.emit_side_effect_expr(expr),
            _ => Err(diag("es_objects: unsupported method stmt")),
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Function { local, .. } => {
                // Ctor object with empty `.prototype` (N08.04.05).
                let ctor = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&ctor, "")).ok();
                let proto = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&proto, "")).ok();
                let key = self.string_const("prototype")?;
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {ctor}, ptr {key}, ptr {proto}"))
                )
                .ok();
                let ptr = self
                    .allocas
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("es_objects: function binding missing alloca"))?;
                writeln!(self.body, "  store ptr {ctor}, ptr {ptr}").ok();
                Ok(())
            }
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                let kind = *self
                    .slot_of
                    .get(local)
                    .ok_or_else(|| diag("es_objects: declare unknown slot"))?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        let ptr = self.number_slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        let ptr = self.string_slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Object => {
                        let v = self.emit_object_expr(init)?;
                        let ptr = self.allocas.get(local).cloned().unwrap();
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr } => self.emit_side_effect_expr(expr),
            _ => Err(diag("es_objects: unsupported stmt")),
        }
    }

    /// Top-level / method statement expressions (discarded values).
    fn emit_side_effect_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object,
                        property,
                        ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                let val_ptr = if let Expr::Function { params, body, .. } = value.as_ref() {
                    let idx = find_fn_idx(params, body, &self.info.functions)
                        .ok_or_else(|| diag("es_objects: unknown method FunctionExpr"))?;
                    format!("@m_fn_{idx}")
                } else if object_value_is_object(value) || matches!(value.as_ref(), Expr::New { .. })
                {
                    self.emit_object_expr(value)?
                } else {
                    let n = self.emit_number_expr(value)?;
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                    let p = self.fresh();
                    writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                    p
                };
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {val_ptr}"))
                )
                .ok();
                Ok(())
            }
            _ => {
                let _ = self.emit_number_expr(expr)?;
                Ok(())
            }
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => format_number_const(raw),
            Expr::Local { id, .. } => {
                if let Some(ptr) = self.param_allocas.get(id).cloned() {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                    return Ok(t);
                }
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_objects: number local unknown"))?;
                if kind != SlotTy::Number {
                    return Err(diag("es_objects: expected number local"));
                }
                let ptr = self.number_slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_objects: optional member not supported"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                let raw = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_GET.call_to(&raw, &format!("ptr {obj}, ptr {key}"))
                )
                .ok();
                let i = self.fresh();
                writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
                let d = self.fresh();
                writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                Ok(d)
            }
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object,
                        property,
                        ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                let n = self.emit_number_expr(value)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                let p = self.fresh();
                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(n)
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let inst = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    BinaryOp::Rem => "frem",
                    _ => return Err(diag("es_objects: unsupported binary")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_objects: optional call not supported"));
                }
                self.emit_method_call(callee, args)
            }
            _ => Err(diag("es_objects: unsupported number expr")),
        }
    }

    fn emit_method_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Member {
            object,
            property,
            optional,
            ..
        } = callee
        else {
            return Err(diag("es_objects: method call requires member callee"));
        };
        if *optional {
            return Err(diag("es_objects: optional member call not supported"));
        }
        let recv = self.emit_object_expr(object)?;
        let key = self.member_key_cstr(property)?;
        let fn_ptr = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&fn_ptr, &format!("ptr {recv}, ptr {key}"))
        )
        .ok();

        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_objects: spread args not supported"));
                }
            }
        }
        while arg_vals.len() < MAX_METHOD_ARGS {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }

        let mut call_args = format!("ptr {recv}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        // Typed call through opaque ptr.
        let mut ty_params = String::from("ptr");
        for _ in 0..MAX_METHOD_ARGS {
            ty_params.push_str(", double");
        }
        let ret = self.fresh();
        writeln!(
            self.body,
            "  {ret} = call double ({ty_params}) {fn_ptr}({call_args})"
        )
        .ok();
        Ok(ret)
    }

    fn emit_new(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_objects: new callee must be local ctor"));
        };
        let idx = *self
            .info
            .fn_binding
            .get(id)
            .ok_or_else(|| diag("es_objects: unknown constructor"))?;
        // N08.04.05: instance.[[Prototype]] = C.prototype
        let ctor = {
            let ptr = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("es_objects: ctor binding missing alloca"))?;
            let t = self.fresh();
            writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
            t
        };
        let proto_key = self.string_const("prototype")?;
        let proto = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&proto, &format!("ptr {ctor}, ptr {proto_key}"))
        )
        .ok();
        let obj = self.fresh();
        writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
        writeln!(
            self.body,
            "  {}",
            OBJECT_SET_PROTO.call(&format!("ptr {obj}, ptr {proto}"))
        )
        .ok();

        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_objects: spread args not supported"));
                }
            }
        }
        while arg_vals.len() < MAX_METHOD_ARGS {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }

        let mut call_args = format!("ptr {obj}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        let mut ty_params = String::from("ptr");
        for _ in 0..MAX_METHOD_ARGS {
            ty_params.push_str(", double");
        }
        let ret = self.fresh();
        writeln!(
            self.body,
            "  {ret} = call double ({ty_params}) @m_fn_{idx}({call_args})"
        )
        .ok();
        // Ignore ctor return value; instance is the allocated object.
        let _ = ret;
        Ok(obj)
    }

    fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::This { .. } => self
                .this_ssa
                .clone()
                .ok_or_else(|| diag("es_objects: This outside method")),
            Expr::New { callee, args, .. } => self.emit_new(callee, args),
            Expr::Object { properties, .. } => {
                let obj = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
                for p in properties {
                    match p {
                        ObjectProp::Property { key, value } => {
                            let key_ptr = self.emit_prop_key(key)?;
                            let val_ptr = if let Expr::Function { params, body, .. } = value {
                                let idx = find_fn_idx(params, body, &self.info.functions)
                                    .ok_or_else(|| {
                                        diag("es_objects: unknown method FunctionExpr")
                                    })?;
                                // Function address as ptr (opaque pointers).
                                format!("@m_fn_{idx}")
                            } else if object_value_is_object(value) {
                                self.emit_object_expr(value)?
                            } else {
                                let n = self.emit_number_expr(value)?;
                                let i = self.fresh();
                                writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                                let p = self.fresh();
                                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                                p
                            };
                            writeln!(
                                self.body,
                                "  {}",
                                OBJECT_SET.call(&format!(
                                    "ptr {obj}, ptr {key_ptr}, ptr {val_ptr}"
                                ))
                            )
                            .ok();
                        }
                        _ => return Err(diag("es_objects: only plain properties")),
                    }
                }
                Ok(obj)
            }
            Expr::Local { id, .. } => {
                if let Some(kind) = self.slot_of.get(id).copied() {
                    if kind != SlotTy::Object {
                        return Err(diag("es_objects: expected object local"));
                    }
                    let ptr = self.allocas.get(id).cloned().unwrap();
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                    return Ok(t);
                }
                if self.info.fn_binding.contains_key(id) {
                    let ptr = self
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("es_objects: fn object local unknown"))?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                    return Ok(t);
                }
                Err(diag("es_objects: object local unknown"))
            }
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_objects: optional member not supported"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_GET.call_to(&t, &format!("ptr {obj}, ptr {key}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_objects: unsupported object expr")),
        }
    }

    fn member_key_cstr(&mut self, property: &Expr) -> Result<String, Diagnostic> {
        match property {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            _ => Err(diag("es_objects: member key must be string")),
        }
    }

    fn emit_prop_key(&mut self, key: &ObjectPropKey) -> Result<String, Diagnostic> {
        match key {
            ObjectPropKey::Static(s) => self.string_const(&s.to_string_lossy()),
            ObjectPropKey::Computed(e) => self.emit_string_expr(e),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_objects: string local unknown"))?;
                if kind != SlotTy::String {
                    return Err(diag("es_objects: expected string local"));
                }
                let ptr = self.string_slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            _ => Err(diag("es_objects: unsupported string expr")),
        }
    }

    fn number_slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        if let Some(ptr) = self.allocas.get(&id) {
            return Ok(ptr.clone());
        }
        // Methods emit before main fills object allocas; number slots are globals.
        if self.slot_of.get(&id) == Some(&SlotTy::Number) {
            return Ok(format!("@{}", number_global_name(id)));
        }
        Err(diag("es_objects: number slot missing"))
    }

    fn string_slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        if let Some(ptr) = self.allocas.get(&id) {
            return Ok(ptr.clone());
        }
        if self.slot_of.get(&id) == Some(&SlotTy::String) {
            return Ok(format!("@{}", string_global_name(id)));
        }
        Err(diag("es_objects: string slot missing"))
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_obj_str.{}", self.str_n);
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

fn object_value_is_object(expr: &Expr) -> bool {
    match expr {
        Expr::Object { .. } | Expr::New { .. } => true,
        Expr::Member { ty, .. } => is_object_slot_ty(ty),
        Expr::Local { ty, .. } => is_object_slot_ty(ty) || matches!(ty, Type::Function),
        _ => false,
    }
}

fn format_number_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(format!("invalid number literal {raw}")))?;
    Ok(format!("{f:.17e}"))
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
