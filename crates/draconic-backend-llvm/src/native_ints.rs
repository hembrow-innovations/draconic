//! N01–N03.03: lower pure native scalar/layout/pointer Programs to LLVM IR.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp, UpdateOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, NativeType,
    ObjectProp, ObjectPropKey, ObjectShape, Param, Pattern, Stmt, UpdateTarget,
};

/// LLVM IR type spelling for a semantic native type (backend-owned mapping).
fn llvm_ty(n: NativeType) -> &'static str {
    match n {
        NativeType::I8 | NativeType::U8 => "i8",
        NativeType::I16 | NativeType::U16 => "i16",
        NativeType::I32 | NativeType::U32 => "i32",
        NativeType::I64 | NativeType::U64 => "i64",
        NativeType::F32 => "float",
        NativeType::F64 => "double",
        NativeType::Bool => "i1",
    }
}

/// Unboxed scalar lowered by this backend (native int/float/`bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Scalar(NativeType);

impl Scalar {
    fn llvm_ty(self) -> &'static str {
        llvm_ty(self.0)
    }

    fn align(self) -> u32 {
        if self.0.is_bool() {
            1
        } else {
            (self.0.bit_width() / 8).max(1)
        }
    }

    fn is_float(self) -> bool {
        self.0.is_float()
    }

    fn is_int(self) -> bool {
        self.0.is_int()
    }

    fn is_bool(self) -> bool {
        self.0.is_bool()
    }

    fn native(self) -> NativeType {
        self.0
    }

    fn zero_const(self) -> &'static str {
        if self.0.is_float() {
            "0.000000e+00"
        } else {
            "0"
        }
    }
}

fn scalar_of_type(ty: Type) -> Option<Scalar> {
    match ty {
        Type::Native(n) => Some(Scalar(n)),
        // Comparison / logical results are JS `boolean` in the checker; lower as i1
        // when already inside a native-scalar module.
        Type::Boolean => Some(Scalar(NativeType::Bool)),
        _ => None,
    }
}

/// True when `shape` is a native layout: every field is a native scalar.
fn shape_is_native_layout(shape: &ObjectShape) -> bool {
    !shape.props.is_empty()
        && shape
            .props
            .iter()
            .all(|(_, t)| matches!(t, Type::Native(_)))
}

fn native_layout_of<'a>(module: &'a Module, ty: Type) -> Option<&'a ObjectShape> {
    match ty {
        Type::Shape(id) => {
            let shape = module.shapes.get(id as usize)?;
            if shape_is_native_layout(shape) {
                Some(shape)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn llvm_layout_ty(shape: &ObjectShape) -> String {
    let mut s = String::from("{ ");
    for (i, (_, t)) in shape.props.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        let Type::Native(n) = *t else {
            unreachable!("native layout fields are Native");
        };
        s.push_str(llvm_ty(n));
    }
    s.push_str(" }");
    s
}

fn layout_align(shape: &ObjectShape) -> u32 {
    shape
        .props
        .iter()
        .filter_map(|(_, t)| scalar_of_type(*t).map(|s| s.align()))
        .max()
        .unwrap_or(1)
}

/// True when every **user-declared** local is a native scalar (`i*`/`u*`/`f*`/`bool`),
/// a **native layout** shape (all-native fields), a **native pointer** (`*T`),
/// or a **function declaration** binding, and the module has at least one native
/// scalar, layout, or pointer local (N01–N03.03 surface).
///
/// Arrow / function-expression bindings are excluded so T05 erase fixtures that
/// mix natives with callable values stay on the B08 hello stub. JS `boolean`
/// locals alone also do not qualify. Globals (Object/Function builtins) ignored.
pub(crate) fn is_native_int_module(module: &Module) -> bool {
    let mut user = HashSet::new();
    collect_user_local_ids(&module.body, &mut user);
    if user.is_empty() {
        return false;
    }
    let fn_decl_locals = function_decl_local_ids(&module.body);
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut has_native = false;
    for id in user {
        let Some(local) = by_id.get(&id) else {
            return false;
        };
        match local.ty {
            Type::Native(_) => has_native = true,
            Type::Ptr(_) => has_native = true,
            Type::Shape(_) if native_layout_of(module, local.ty).is_some() => has_native = true,
            Type::Function if fn_decl_locals.contains(&id) => {}
            _ => return false,
        }
    }
    has_native
}

fn function_decl_local_ids(body: &[Stmt]) -> HashSet<LocalId> {
    let mut out = HashSet::new();
    for stmt in body {
        match stmt {
            Stmt::Function { local, body, .. } => {
                out.insert(*local);
                out.extend(function_decl_local_ids(body));
            }
            Stmt::Block { body } => out.extend(function_decl_local_ids(body)),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                out.extend(function_decl_local_ids_stmt(consequent));
                if let Some(a) = alternate {
                    out.extend(function_decl_local_ids_stmt(a));
                }
            }
            Stmt::While { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::Labeled { body, .. } => {
                out.extend(function_decl_local_ids_stmt(body));
            }
            Stmt::For { init, body, .. } => {
                if let Some(i) = init {
                    out.extend(function_decl_local_ids_stmt(i));
                }
                out.extend(function_decl_local_ids_stmt(body));
            }
            _ => {}
        }
    }
    out
}

fn function_decl_local_ids_stmt(stmt: &Stmt) -> HashSet<LocalId> {
    function_decl_local_ids(std::slice::from_ref(stmt))
}

fn collect_user_local_ids(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for stmt in body {
        collect_user_local_ids_stmt(stmt, out);
    }
}

fn collect_user_local_ids_stmt(stmt: &Stmt, out: &mut HashSet<LocalId>) {
    match stmt {
        Stmt::Declare { local, .. } => {
            out.insert(*local);
        }
        Stmt::Function {
            local,
            params,
            body,
            ..
        } => {
            out.insert(*local);
            for p in params {
                collect_pattern_locals(&p.pattern, out);
            }
            collect_user_local_ids(body, out);
        }
        Stmt::Block { body } => collect_user_local_ids(body, out),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_user_local_ids_stmt(consequent, out);
            if let Some(a) = alternate {
                collect_user_local_ids_stmt(a, out);
            }
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } | Stmt::Labeled { body, .. } => {
            collect_user_local_ids_stmt(body, out);
        }
        Stmt::For {
            init,
            body,
            ..
        } => {
            if let Some(i) = init {
                collect_user_local_ids_stmt(i, out);
            }
            collect_user_local_ids_stmt(body, out);
        }
        _ => {}
    }
}

fn collect_pattern_locals(pat: &Pattern, out: &mut HashSet<LocalId>) {
    match pat {
        Pattern::Local(id) => {
            out.insert(*id);
        }
        Pattern::Array(els) => {
            for el in els {
                match el {
                    draconic_ir::ArrayPatternEl::Pattern { binding, .. } => {
                        collect_pattern_locals(binding, out);
                    }
                    draconic_ir::ArrayPatternEl::Rest(id) => {
                        out.insert(*id);
                    }
                }
            }
        }
        Pattern::Object(props) => {
            for p in props {
                match p {
                    draconic_ir::ObjectPatternEl::Prop { binding, .. } => {
                        collect_pattern_locals(binding, out);
                    }
                    draconic_ir::ObjectPatternEl::Rest(id) => {
                        out.insert(*id);
                    }
                }
            }
        }
    }
}

pub(crate) fn emit_native_ints(module: &Module) -> Result<String, Diagnostic> {
    let mut em = Emitter::new(module);
    em.emit_module()?;
    Ok(em.finish())
}

struct Emitter<'a> {
    module: &'a Module,
    locals: HashMap<LocalId, &'a Local>,
    /// Alloca pointer SSA name per local: `%l{id}`
    allocas: HashMap<LocalId, String>,
    /// Function IR local → LLVM function name
    fn_names: HashMap<LocalId, String>,
    /// Function param locals (no alloca; SSA param name)
    params: HashMap<LocalId, (String, Scalar)>,
    out: String,
    body: String,
    tmp: u32,
    label: u32,
    /// Top-level native scalar locals to print at end of main (declare order).
    print_order: Vec<LocalId>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        let locals: HashMap<LocalId, &'a Local> =
            module.locals.iter().map(|l| (l.id, l)).collect();
        let mut fn_names = HashMap::new();
        for stmt in &module.body {
            if let Stmt::Function { local, .. } = stmt {
                let name = locals
                    .get(local)
                    .map(|l| l.name.as_str())
                    .unwrap_or("fn");
                let safe: String = name
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                    .collect();
                fn_names.insert(*local, format!("d_{safe}_{}", local.0));
            }
        }
        Self {
            module,
            locals,
            allocas: HashMap::new(),
            fn_names,
            params: HashMap::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
            label: 0,
            print_order: Vec::new(),
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N01–N03.03 native scalars/layouts/pointers)"
        )
        .ok();
        writeln!(self.out, "declare void @draconic_rt_print_i64(i64)").ok();
        writeln!(self.out, "declare void @draconic_rt_print_u64(i64)").ok();
        writeln!(self.out, "declare void @draconic_rt_print_f64(double)").ok();
        writeln!(self.out, "declare void @draconic_rt_print_bool(i8)").ok();
        writeln!(self.out).ok();

        // Emit nested function definitions first.
        for stmt in &self.module.body {
            if let Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
            } = stmt
            {
                if *is_async || *is_generator {
                    return Err(diag("native scalars: async/generator functions not supported"));
                }
                self.emit_function(*local, params, body)?;
            }
        }

        // main
        self.body.clear();
        self.tmp = 0;
        self.label = 0;
        self.params.clear();
        self.allocas.clear();
        self.print_order.clear();

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();

        // Pre-declare allocas for all non-function, non-param top-level locals.
        for local in &self.module.locals {
            if matches!(local.ty, Type::Function) {
                continue;
            }
            if let Some(sc) = scalar_of_type(local.ty) {
                let ptr = format!("%l{}", local.id.0);
                self.allocas.insert(local.id, ptr.clone());
                writeln!(
                    self.out,
                    "  {ptr} = alloca {}, align {}",
                    sc.llvm_ty(),
                    sc.align()
                )
                .ok();
            } else if matches!(local.ty, Type::Ptr(_)) {
                let ptr = format!("%l{}", local.id.0);
                self.allocas.insert(local.id, ptr.clone());
                writeln!(self.out, "  {ptr} = alloca ptr, align 8").ok();
            } else if let Some(shape) = native_layout_of(self.module, local.ty) {
                let ptr = format!("%l{}", local.id.0);
                self.allocas.insert(local.id, ptr.clone());
                writeln!(
                    self.out,
                    "  {ptr} = alloca {}, align {}",
                    llvm_layout_ty(shape),
                    layout_align(shape)
                )
                .ok();
            }
        }

        for stmt in &self.module.body {
            if matches!(stmt, Stmt::Function { .. }) {
                continue;
            }
            self.emit_stmt(stmt)?;
        }

        // Print top-level native int declares in source order.
        for id in self.print_order.clone() {
            self.emit_print_local(id)?;
        }

        writeln!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_function(
        &mut self,
        local: LocalId,
        params: &[Param],
        body: &[Stmt],
    ) -> Result<(), Diagnostic> {
        let fn_name = self
            .fn_names
            .get(&local)
            .cloned()
            .ok_or_else(|| diag("internal: missing function name"))?;

        let mut param_tys = Vec::new();
        let mut param_ids = Vec::new();
        for p in params {
            if p.rest || p.default.is_some() {
                return Err(diag("native scalars: rest/default params not supported"));
            }
            let Pattern::Local(id) = &p.pattern else {
                return Err(diag("native scalars: only simple ident params supported"));
            };
            let ty = self.local_scalar(*id)?;
            param_tys.push(ty);
            param_ids.push(*id);
        }

        // Infer return type from first Return with a value, else i32.
        let ret_ty = infer_return_scalar(body).unwrap_or(Scalar(NativeType::I32));

        let mut sig = String::new();
        write!(sig, "define {} @{fn_name}(", ret_ty.llvm_ty()).ok();
        for (i, (id, ty)) in param_ids.iter().zip(param_tys.iter()).enumerate() {
            if i > 0 {
                sig.push_str(", ");
            }
            write!(sig, "{} %p{}", ty.llvm_ty(), id.0).ok();
        }
        sig.push(')');

        // Save main state.
        let saved_body = std::mem::take(&mut self.body);
        let saved_tmp = self.tmp;
        let saved_label = self.label;
        let saved_params = std::mem::take(&mut self.params);
        let saved_allocas = std::mem::take(&mut self.allocas);
        let saved_print = std::mem::take(&mut self.print_order);

        self.tmp = 0;
        self.label = 0;
        self.params.clear();
        self.allocas.clear();

        for (id, ty) in param_ids.iter().zip(param_tys.iter()) {
            self.params.insert(*id, (format!("%p{}", id.0), *ty));
        }

        // Allocas for locals declared inside the function (and copy params to allocas for mutability).
        let mut pre = String::new();
        for (id, ty) in param_ids.iter().zip(param_tys.iter()) {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            writeln!(
                pre,
                "  {ptr} = alloca {}, align {}",
                ty.llvm_ty(),
                ty.align()
            )
            .ok();
            writeln!(
                pre,
                "  store {} %p{}, ptr {ptr}",
                ty.llvm_ty(),
                id.0
            )
            .ok();
            // Params are also addressable via alloca now.
            self.params.remove(id);
        }

        // Collect locals used in body that aren't params.
        let mut body_locals = Vec::new();
        collect_declared_locals(body, &mut body_locals);
        for id in body_locals {
            if self.allocas.contains_key(&id) {
                continue;
            }
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(id, ptr.clone());
            if let Ok(ty) = self.local_scalar(id) {
                writeln!(
                    pre,
                    "  {ptr} = alloca {}, align {}",
                    ty.llvm_ty(),
                    ty.align()
                )
                .ok();
            } else if self
                .locals
                .get(&id)
                .is_some_and(|l| matches!(l.ty, Type::Ptr(_)))
            {
                writeln!(pre, "  {ptr} = alloca ptr, align 8").ok();
            } else if let Some(shape) = self.local_layout(id) {
                writeln!(
                    pre,
                    "  {ptr} = alloca {}, align {}",
                    llvm_layout_ty(shape),
                    layout_align(shape)
                )
                .ok();
            } else {
                return Err(diag("native scalars: unsupported local type in function"));
            }
        }

        for stmt in body {
            self.emit_stmt(stmt)?;
        }

        // Ensure terminator.
        if !self.body_ends_with_terminator() {
            writeln!(
                self.body,
                "  ret {} {}",
                ret_ty.llvm_ty(),
                ret_ty.zero_const()
            )
            .ok();
        }

        writeln!(self.out, "{sig} {{").ok();
        writeln!(self.out, "entry:").ok();
        write!(self.out, "{pre}").ok();
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.tmp = saved_tmp;
        self.label = saved_label;
        self.params = saved_params;
        self.allocas = saved_allocas;
        self.print_order = saved_print;
        Ok(())
    }

    fn body_ends_with_terminator(&self) -> bool {
        for line in self.body.lines().rev() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            return t.starts_with("ret ") || t.starts_with("br ");
        }
        false
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let ptr = self
                    .allocas
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("internal: missing alloca for local"))?;
                if let Ok(ty) = self.local_scalar(*local) {
                    if let Some(init) = init {
                        let v = self.emit_expr(init, Some(ty))?;
                        writeln!(self.body, "  store {} {v}, ptr {ptr}", ty.llvm_ty()).ok();
                    } else {
                        writeln!(
                            self.body,
                            "  store {} {}, ptr {ptr}",
                            ty.llvm_ty(),
                            ty.zero_const()
                        )
                        .ok();
                    }
                } else if self
                    .locals
                    .get(local)
                    .is_some_and(|l| matches!(l.ty, Type::Ptr(_)))
                {
                    if let Some(init) = init {
                        let v = self.emit_ptr_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    } else {
                        writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                    }
                    // Pointers are not printed at end of main.
                    return Ok(());
                } else if let Some(shape) = self.local_layout(*local).cloned() {
                    let layout_ty = llvm_layout_ty(&shape);
                    let fields: Vec<Scalar> = shape
                        .props
                        .iter()
                        .map(|(_, fty)| {
                            let Type::Native(n) = *fty else {
                                return Err(diag("native layout: non-native field"));
                            };
                            Ok(Scalar(n))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if let Some(init) = init {
                        self.emit_store_layout(&ptr, &shape, init)?;
                    } else {
                        // Zero-init each field.
                        for (i, sc) in fields.iter().enumerate() {
                            let gep = self.fresh_tmp();
                            writeln!(
                                self.body,
                                "  {gep} = getelementptr inbounds {layout_ty}, ptr {ptr}, i32 0, i32 {i}"
                            )
                            .ok();
                            writeln!(
                                self.body,
                                "  store {} {}, ptr {gep}",
                                sc.llvm_ty(),
                                sc.zero_const()
                            )
                            .ok();
                        }
                    }
                } else {
                    return Err(diag(
                        "native scalars: declare needs scalar, layout, or pointer local",
                    ));
                }
                // Main tracks declares for end-of-program print; function emit uses a
                // saved empty print_order that is discarded.
                if !self.print_order.contains(local) {
                    self.print_order.push(*local);
                }
                Ok(())
            }
            Stmt::Expr { expr } => {
                let _ = self.emit_expr(expr, None)?;
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Return { value } => {
                if let Some(v) = value {
                    let sty = scalar_of_type(v.ty()).ok_or_else(|| {
                        diag("native scalars: return value must be a native scalar")
                    })?;
                    let val = self.emit_expr(v, Some(sty))?;
                    writeln!(self.body, "  ret {} {val}", sty.llvm_ty()).ok();
                } else {
                    writeln!(self.body, "  ret i32 0").ok();
                }
                // Unreachable padding label so subsequent code is valid if any.
                let lab = self.fresh_label("after_ret");
                writeln!(self.body, "{lab}:").ok();
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let cond = self.emit_bool(test)?;
                let then_l = self.fresh_label("then");
                let else_l = self.fresh_label("else");
                let end_l = self.fresh_label("endif");
                if alternate.is_some() {
                    writeln!(
                        self.body,
                        "  br i1 {cond}, label %{then_l}, label %{else_l}"
                    )
                    .ok();
                } else {
                    writeln!(
                        self.body,
                        "  br i1 {cond}, label %{then_l}, label %{end_l}"
                    )
                    .ok();
                }
                writeln!(self.body, "{then_l}:").ok();
                self.emit_stmt(consequent)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{end_l}").ok();
                }
                if let Some(alt) = alternate {
                    writeln!(self.body, "{else_l}:").ok();
                    self.emit_stmt(alt)?;
                    if !self.body_ends_with_terminator() {
                        writeln!(self.body, "  br label %{end_l}").ok();
                    }
                }
                writeln!(self.body, "{end_l}:").ok();
                Ok(())
            }
            Stmt::While { test, body } => {
                let head = self.fresh_label("while_head");
                let bod = self.fresh_label("while_body");
                let end = self.fresh_label("while_end");
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{head}:").ok();
                let cond = self.emit_bool(test)?;
                writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                writeln!(self.body, "{bod}:").ok();
                self.emit_stmt(body)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{head}").ok();
                }
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::Function { .. } => Ok(()), // emitted separately
            other => Err(diag(&format!(
                "native scalars: unsupported statement {other:?}"
            ))),
        }
    }

    fn emit_print_local(&mut self, id: LocalId) -> Result<(), Diagnostic> {
        let ptr = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("internal: print missing alloca"))?;
        if let Ok(ty) = self.local_scalar(id) {
            let v = self.fresh_tmp();
            writeln!(self.body, "  {v} = load {}, ptr {ptr}", ty.llvm_ty()).ok();
            return self.emit_print_scalar_value(ty, &v);
        }
        if let Some(shape) = self.local_layout(id) {
            let layout_ty = llvm_layout_ty(shape);
            let fields: Vec<Scalar> = shape
                .props
                .iter()
                .map(|(_, fty)| {
                    let Type::Native(n) = *fty else {
                        return Err(diag("native layout: non-native field"));
                    };
                    Ok(Scalar(n))
                })
                .collect::<Result<Vec<_>, _>>()?;
            for (i, sc) in fields.iter().enumerate() {
                let gep = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {gep} = getelementptr inbounds {layout_ty}, ptr {ptr}, i32 0, i32 {i}"
                )
                .ok();
                let v = self.fresh_tmp();
                writeln!(self.body, "  {v} = load {}, ptr {gep}", sc.llvm_ty()).ok();
                self.emit_print_scalar_value(*sc, &v)?;
            }
            return Ok(());
        }
        Err(diag(
            "native scalars: print only supports native scalars/layouts",
        ))
    }

    fn emit_print_scalar_value(&mut self, ty: Scalar, v: &str) -> Result<(), Diagnostic> {
        let n = ty.native();
        if n.is_bool() {
            let ext = self.fresh_tmp();
            writeln!(self.body, "  {ext} = zext i1 {v} to i8").ok();
            writeln!(self.body, "  call void @draconic_rt_print_bool(i8 {ext})").ok();
        } else if n.is_float() {
            let d = if n == NativeType::F32 {
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = fpext float {v} to double").ok();
                t
            } else {
                v.to_string()
            };
            writeln!(self.body, "  call void @draconic_rt_print_f64(double {d})").ok();
        } else {
            let ext = self.fresh_tmp();
            if n.bit_width() < 64 {
                if n.is_signed() {
                    writeln!(
                        self.body,
                        "  {ext} = sext {} {v} to i64",
                        llvm_ty(n)
                    )
                    .ok();
                } else {
                    writeln!(
                        self.body,
                        "  {ext} = zext {} {v} to i64",
                        llvm_ty(n)
                    )
                    .ok();
                }
            } else {
                writeln!(self.body, "  {ext} = add i64 {v}, 0").ok();
            }
            if n.is_signed() {
                writeln!(self.body, "  call void @draconic_rt_print_i64(i64 {ext})").ok();
            } else {
                writeln!(self.body, "  call void @draconic_rt_print_u64(i64 {ext})").ok();
            }
        }
        Ok(())
    }

    fn emit_bool(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr.ty() {
            Type::Boolean | Type::Native(NativeType::Bool) => {
                self.emit_expr(expr, Some(Scalar(NativeType::Bool)))
            }
            Type::Native(n) if n.is_int() => {
                let v = self.emit_expr(expr, Some(Scalar(n)))?;
                let t = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {t} = icmp ne {} {v}, 0",
                    llvm_ty(n)
                )
                .ok();
                Ok(t)
            }
            Type::Native(n) if n.is_float() => {
                let v = self.emit_expr(expr, Some(Scalar(n)))?;
                let t = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {t} = fcmp one {} {v}, 0.000000e+00",
                    llvm_ty(n)
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag(
                "native scalars: condition must be bool or native numeric",
            )),
        }
    }

    fn emit_expr(
        &mut self,
        expr: &Expr,
        expect: Option<Scalar>,
    ) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, ty } => {
                if let Some((pname, _)) = self.params.get(id) {
                    return Ok(pname.clone());
                }
                if matches!(ty, Type::Ptr(_)) {
                    return self.emit_ptr_expr(expr);
                }
                if let Some(ptr) = self.allocas.get(id).cloned() {
                    let sty = scalar_of_type(*ty).unwrap_or(self.local_scalar(*id)?);
                    let t = self.fresh_tmp();
                    writeln!(self.body, "  {t} = load {}, ptr {ptr}", sty.llvm_ty()).ok();
                    return Ok(t);
                }
                // Function reference used as value — not supported except as callee.
                Err(diag("native scalars: bare function value not supported"))
            }
            Expr::Boolean { value, .. } => Ok(if *value { "1".into() } else { "0".into() }),
            Expr::Number { raw, ty } => {
                let sty = match (ty, expect) {
                    (Type::Native(n), _) if !n.is_bool() => Scalar(*n),
                    (_, Some(s)) if !s.is_bool() => s,
                    _ => {
                        return Err(diag(
                            "native scalars: number literal needs native numeric context",
                        ))
                    }
                };
                let nty = sty.native();
                if nty.is_float() {
                    Ok(format_float_const(raw, nty)?)
                } else {
                    Ok(format_int_const(raw, nty)?)
                }
            }
            Expr::Unary { op, arg, ty } => {
                if matches!(op, UnaryOp::Not) {
                    let a = self.emit_bool(arg)?;
                    let t = self.fresh_tmp();
                    writeln!(self.body, "  {t} = xor i1 {a}, true").ok();
                    return Ok(t);
                }
                // N03.03: `&x` → pointer value (address of local).
                if matches!(op, UnaryOp::Ref) {
                    return self.emit_ptr_expr(expr);
                }
                // N03.03: `*p` → load pointee scalar.
                if matches!(op, UnaryOp::Deref) {
                    let ptr_v = self.emit_ptr_expr(arg)?;
                    let sty = match ty {
                        Type::Native(n) => Scalar(*n),
                        Type::Boolean => Scalar(NativeType::Bool),
                        _ => {
                            return Err(diag(
                                "native pointers: dereference result must be native scalar",
                            ))
                        }
                    };
                    let t = self.fresh_tmp();
                    writeln!(self.body, "  {t} = load {}, ptr {ptr_v}", sty.llvm_ty()).ok();
                    return Ok(t);
                }
                let nty = match ty {
                    Type::Native(n) if !n.is_bool() => *n,
                    _ => {
                        return Err(diag("native scalars: unary result must be native numeric"))
                    }
                };
                let a = self.emit_expr(arg, Some(Scalar(nty)))?;
                let t = self.fresh_tmp();
                match op {
                    UnaryOp::Minus if nty.is_float() => {
                        writeln!(
                            self.body,
                            "  {t} = fneg {} {a}",
                            llvm_ty(nty)
                        )
                        .ok();
                    }
                    UnaryOp::Minus => {
                        writeln!(
                            self.body,
                            "  {t} = sub {} 0, {a}",
                            llvm_ty(nty)
                        )
                        .ok();
                    }
                    UnaryOp::BitNot if nty.is_int() => {
                        writeln!(
                            self.body,
                            "  {t} = xor {} {a}, -1",
                            llvm_ty(nty)
                        )
                        .ok();
                    }
                    UnaryOp::Plus if nty.is_float() => {
                        writeln!(
                            self.body,
                            "  {t} = fadd {} {a}, 0.000000e+00",
                            llvm_ty(nty)
                        )
                        .ok();
                    }
                    UnaryOp::Plus => {
                        writeln!(self.body, "  {t} = add {} {a}, 0", llvm_ty(nty)).ok();
                    }
                    _ => {
                        return Err(diag(&format!(
                            "native scalars: unsupported unary {op}"
                        )))
                    }
                }
                Ok(t)
            }
            Expr::Binary {
                left,
                op,
                right,
                ty,
            } => self.emit_binary(left, *op, right, ty, expect),
            Expr::Assign {
                target,
                op,
                value,
                ty,
            } => self.emit_assign(target, *op, value, ty),
            Expr::Update {
                op,
                target,
                prefix,
                ty,
            } => self.emit_update(*op, target, *prefix, ty),
            Expr::Call {
                callee,
                args,
                optional,
                ty,
            } => {
                if *optional {
                    return Err(diag("native scalars: optional call not supported"));
                }
                let Expr::Local { id, .. } = callee.as_ref() else {
                    return Err(diag("native scalars: only direct function calls supported"));
                };
                let fn_name = self
                    .fn_names
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("native scalars: call to unknown function"))?;
                // Checker currently types non-generic calls as `Any`; prefer the
                // expression type, then expected context, then inferred signature.
                let ret_ty = match scalar_of_type(*ty) {
                    Some(s) => s,
                    None => match expect {
                        Some(n) => n,
                        None => self.function_sig(*id)?.1,
                    },
                };
                let (param_tys, _) = self.function_sig(*id)?;
                if param_tys.len() != args.len() {
                    return Err(diag("native scalars: arity mismatch"));
                }
                let mut arg_parts = Vec::new();
                for (arg, pty) in args.iter().zip(param_tys.iter()) {
                    let Arg::Expr(e) = arg else {
                        return Err(diag("native scalars: spread args not supported"));
                    };
                    let v = self.emit_expr(e, Some(*pty))?;
                    arg_parts.push(format!("{} {v}", pty.llvm_ty()));
                }
                let t = self.fresh_tmp();
                write!(
                    self.body,
                    "  {t} = call {} @{fn_name}(",
                    ret_ty.llvm_ty()
                )
                .ok();
                for (i, p) in arg_parts.iter().enumerate() {
                    if i > 0 {
                        self.body.push_str(", ");
                    }
                    self.body.push_str(p);
                }
                writeln!(self.body, ")").ok();
                Ok(t)
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ty,
            } => {
                let sty = match scalar_of_type(*ty) {
                    Some(s) => s,
                    None => expect.ok_or_else(|| {
                        diag("native scalars: conditional needs native scalar type")
                    })?,
                };
                let cond = self.emit_bool(test)?;
                let then_l = self.fresh_label("sel_then");
                let else_l = self.fresh_label("sel_else");
                let end_l = self.fresh_label("sel_end");
                let slot = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {slot} = alloca {}, align {}",
                    sty.llvm_ty(),
                    sty.align()
                )
                .ok();
                writeln!(
                    self.body,
                    "  br i1 {cond}, label %{then_l}, label %{else_l}"
                )
                .ok();
                writeln!(self.body, "{then_l}:").ok();
                let c = self.emit_expr(consequent, Some(sty))?;
                writeln!(self.body, "  store {} {c}, ptr {slot}", sty.llvm_ty()).ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{else_l}:").ok();
                let a = self.emit_expr(alternate, Some(sty))?;
                writeln!(self.body, "  store {} {a}, ptr {slot}", sty.llvm_ty()).ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{end_l}:").ok();
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = load {}, ptr {slot}", sty.llvm_ty()).ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                computed,
                optional,
                ty,
            } => {
                if *optional {
                    return Err(diag("native layout: optional member not supported"));
                }
                let key = if *computed {
                    // Fixed-array index: `a[0]` with constant non-neg integer (N03.02).
                    const_index_key(property).ok_or_else(|| {
                        diag("native layout: computed member needs constant integer index")
                    })?
                } else {
                    match property.as_ref() {
                        Expr::String { value, .. } => value.to_string_lossy(),
                        _ => {
                            return Err(diag(
                                "native layout: member property must be a static string key",
                            ))
                        }
                    }
                };
                let (obj_ptr, layout_ty, idx, sc) = {
                    let (obj_ptr, shape) = self.emit_layout_base(object)?;
                    let idx = shape
                        .props
                        .iter()
                        .position(|(n, _)| n == &key)
                        .ok_or_else(|| diag(&format!("native layout: unknown field `{key}`")))?;
                    let field_ty = shape.props[idx].1;
                    let sc = scalar_of_type(field_ty)
                        .or_else(|| scalar_of_type(*ty))
                        .ok_or_else(|| diag("native layout: field must be native scalar"))?;
                    (obj_ptr, llvm_layout_ty(shape), idx, sc)
                };
                let gep = self.fresh_tmp();
                writeln!(
                    self.body,
                    "  {gep} = getelementptr inbounds {layout_ty}, ptr {obj_ptr}, i32 0, i32 {idx}"
                )
                .ok();
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = load {}, ptr {gep}", sc.llvm_ty()).ok();
                Ok(t)
            }
            Expr::Object { .. } => Err(diag(
                "native layout: object literal only supported as layout init",
            )),
            Expr::Array { .. } => Err(diag(
                "native layout: array literal only supported as layout init",
            )),
            _ => Err(diag(&format!(
                "native scalars: unsupported expression {expr:?}"
            ))),
        }
    }

    /// Pointer to a layout local + its shape (for field GEP).
    fn emit_layout_base<'b>(
        &'b self,
        expr: &'b Expr,
    ) -> Result<(String, &'b ObjectShape), Diagnostic> {
        match expr {
            Expr::Local { id, ty } => {
                let shape = native_layout_of(self.module, *ty)
                    .or_else(|| self.local_layout(*id))
                    .ok_or_else(|| diag("native layout: member base is not a layout local"))?;
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("internal: layout local missing alloca"))?;
                Ok((ptr, shape))
            }
            _ => Err(diag(
                "native layout: only direct local field access supported",
            )),
        }
    }

    fn emit_store_layout(
        &mut self,
        dest_ptr: &str,
        shape: &ObjectShape,
        init: &Expr,
    ) -> Result<(), Diagnostic> {
        let layout_ty = llvm_layout_ty(shape);
        let field_meta: Vec<(String, Scalar)> = shape
            .props
            .iter()
            .map(|(name, fty)| {
                let Type::Native(n) = *fty else {
                    return Err(diag("native layout: non-native field"));
                };
                Ok((name.clone(), Scalar(n)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        match init {
            Expr::Object { properties, .. } => {
                let mut by_name: HashMap<String, &Expr> = HashMap::new();
                for prop in properties {
                    match prop {
                        ObjectProp::Property {
                            key: ObjectPropKey::Static(k),
                            value,
                        } => {
                            by_name.insert(k.to_string_lossy(), value);
                        }
                        _ => {
                            return Err(diag(
                                "native layout: only static data properties in object init",
                            ))
                        }
                    }
                }
                for (i, (name, sc)) in field_meta.iter().enumerate() {
                    let val_expr = by_name.get(name).ok_or_else(|| {
                        diag(&format!("native layout: missing field `{name}` in init"))
                    })?;
                    let v = self.emit_expr(val_expr, Some(*sc))?;
                    let gep = self.fresh_tmp();
                    writeln!(
                        self.body,
                        "  {gep} = getelementptr inbounds {layout_ty}, ptr {dest_ptr}, i32 0, i32 {i}"
                    )
                    .ok();
                    writeln!(self.body, "  store {} {v}, ptr {gep}", sc.llvm_ty()).ok();
                }
                Ok(())
            }
            Expr::Array { elements, .. } => {
                if elements.len() != field_meta.len() {
                    return Err(diag(
                        "native layout: array init length must match tuple layout",
                    ));
                }
                for (i, (_, sc)) in field_meta.iter().enumerate() {
                    let ArrayElement::Expr(val_expr) = &elements[i] else {
                        return Err(diag("native layout: spread not supported in array init"));
                    };
                    let v = self.emit_expr(val_expr, Some(*sc))?;
                    let gep = self.fresh_tmp();
                    writeln!(
                        self.body,
                        "  {gep} = getelementptr inbounds {layout_ty}, ptr {dest_ptr}, i32 0, i32 {i}"
                    )
                    .ok();
                    writeln!(self.body, "  store {} {v}, ptr {gep}", sc.llvm_ty()).ok();
                }
                Ok(())
            }
            Expr::Local { id, .. } => {
                let src_ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("internal: layout copy missing src alloca"))?;
                for (i, (_, sc)) in field_meta.iter().enumerate() {
                    let src_gep = self.fresh_tmp();
                    writeln!(
                        self.body,
                        "  {src_gep} = getelementptr inbounds {layout_ty}, ptr {src_ptr}, i32 0, i32 {i}"
                    )
                    .ok();
                    let v = self.fresh_tmp();
                    writeln!(self.body, "  {v} = load {}, ptr {src_gep}", sc.llvm_ty()).ok();
                    let dst_gep = self.fresh_tmp();
                    writeln!(
                        self.body,
                        "  {dst_gep} = getelementptr inbounds {layout_ty}, ptr {dest_ptr}, i32 0, i32 {i}"
                    )
                    .ok();
                    writeln!(self.body, "  store {} {v}, ptr {dst_gep}", sc.llvm_ty()).ok();
                }
                Ok(())
            }
            _ => Err(diag(
                "native layout: init must be object/array literal or layout local",
            )),
        }
    }

    fn emit_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        ty: &Type,
        expect: Option<Scalar>,
    ) -> Result<String, Diagnostic> {
        // Comparisons → i1
        if matches!(
            op,
            BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
        ) {
            let sty = scalar_operand_ty(left, right, expect.filter(|s| s.is_int() || s.is_float()))?;
            if sty.is_bool() {
                return Err(diag("native scalars: compare needs numeric operands"));
            }
            let nty = sty.native();
            let l = self.emit_expr(left, Some(sty))?;
            let r = self.emit_expr(right, Some(sty))?;
            let t = self.fresh_tmp();
            if nty.is_float() {
                let pred = match op {
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "oeq",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "one",
                    BinaryOp::Lt => "olt",
                    BinaryOp::LtEq => "ole",
                    BinaryOp::Gt => "ogt",
                    BinaryOp::GtEq => "oge",
                    _ => unreachable!(),
                };
                writeln!(
                    self.body,
                    "  {t} = fcmp {pred} {} {l}, {r}",
                    llvm_ty(nty)
                )
                .ok();
            } else {
                let pred = match op {
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "eq",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "ne",
                    BinaryOp::Lt => {
                        if nty.is_signed() {
                            "slt"
                        } else {
                            "ult"
                        }
                    }
                    BinaryOp::LtEq => {
                        if nty.is_signed() {
                            "sle"
                        } else {
                            "ule"
                        }
                    }
                    BinaryOp::Gt => {
                        if nty.is_signed() {
                            "sgt"
                        } else {
                            "ugt"
                        }
                    }
                    BinaryOp::GtEq => {
                        if nty.is_signed() {
                            "sge"
                        } else {
                            "uge"
                        }
                    }
                    _ => unreachable!(),
                };
                writeln!(
                    self.body,
                    "  {t} = icmp {pred} {} {l}, {r}",
                    llvm_ty(nty)
                )
                .ok();
            }
            return Ok(t);
        }

        if matches!(op, BinaryOp::Comma) {
            let _ = self.emit_expr(left, expect)?;
            return self.emit_expr(right, expect);
        }

        let sty = match scalar_of_type(*ty) {
            Some(s) if !s.is_bool() => s,
            _ => scalar_operand_ty(left, right, expect)?,
        };
        if sty.is_bool() {
            return Err(diag("native scalars: arithmetic needs numeric type"));
        }
        let nty = sty.native();
        let l = self.emit_expr(left, Some(sty))?;
        let r = self.emit_expr(right, Some(sty))?;
        let t = self.fresh_tmp();
        let ll = llvm_ty(nty);
        if nty.is_float() {
            match op {
                BinaryOp::Add => writeln!(self.body, "  {t} = fadd {ll} {l}, {r}").ok(),
                BinaryOp::Sub => writeln!(self.body, "  {t} = fsub {ll} {l}, {r}").ok(),
                BinaryOp::Mul => writeln!(self.body, "  {t} = fmul {ll} {l}, {r}").ok(),
                BinaryOp::Div => writeln!(self.body, "  {t} = fdiv {ll} {l}, {r}").ok(),
                BinaryOp::Rem => writeln!(self.body, "  {t} = frem {ll} {l}, {r}").ok(),
                _ => {
                    return Err(diag(&format!(
                        "native scalars: unsupported float binary operator {op}"
                    )))
                }
            };
        } else {
            match op {
                BinaryOp::Add => writeln!(self.body, "  {t} = add {ll} {l}, {r}").ok(),
                BinaryOp::Sub => writeln!(self.body, "  {t} = sub {ll} {l}, {r}").ok(),
                BinaryOp::Mul => writeln!(self.body, "  {t} = mul {ll} {l}, {r}").ok(),
                BinaryOp::Div => {
                    if nty.is_signed() {
                        writeln!(self.body, "  {t} = sdiv {ll} {l}, {r}").ok()
                    } else {
                        writeln!(self.body, "  {t} = udiv {ll} {l}, {r}").ok()
                    }
                }
                BinaryOp::Rem => {
                    if nty.is_signed() {
                        writeln!(self.body, "  {t} = srem {ll} {l}, {r}").ok()
                    } else {
                        writeln!(self.body, "  {t} = urem {ll} {l}, {r}").ok()
                    }
                }
                BinaryOp::BitAnd => writeln!(self.body, "  {t} = and {ll} {l}, {r}").ok(),
                BinaryOp::BitOr => writeln!(self.body, "  {t} = or {ll} {l}, {r}").ok(),
                BinaryOp::BitXor => writeln!(self.body, "  {t} = xor {ll} {l}, {r}").ok(),
                BinaryOp::Shl => writeln!(self.body, "  {t} = shl {ll} {l}, {r}").ok(),
                BinaryOp::Shr => {
                    if nty.is_signed() {
                        writeln!(self.body, "  {t} = ashr {ll} {l}, {r}").ok()
                    } else {
                        writeln!(self.body, "  {t} = lshr {ll} {l}, {r}").ok()
                    }
                }
                _ => {
                    return Err(diag(&format!(
                        "native scalars: unsupported binary operator {op}"
                    )))
                }
            };
        }
        Ok(t)
    }

    fn emit_assign(
        &mut self,
        target: &AssignTarget,
        op: AssignOp,
        value: &Expr,
        ty: &Type,
    ) -> Result<String, Diagnostic> {
        // N03.03: `*p = v` store through pointer (simple `=` only).
        if let AssignTarget::Deref(ptr_expr) = target {
            if !matches!(op, AssignOp::Eq) {
                return Err(diag(
                    "native pointers: only simple `=` store through pointer supported",
                ));
            }
            let sty = match scalar_of_type(*ty) {
                Some(s) => s,
                None => match ptr_expr.ty() {
                    Type::Ptr(n) => Scalar(n),
                    _ => {
                        return Err(diag(
                            "native pointers: store value must be a native scalar",
                        ))
                    }
                },
            };
            let dest = self.emit_ptr_expr(ptr_expr)?;
            let rhs = self.emit_expr(value, Some(sty))?;
            writeln!(self.body, "  store {} {rhs}, ptr {dest}", sty.llvm_ty()).ok();
            return Ok(rhs);
        }

        let AssignTarget::Local(id) = target else {
            return Err(diag("native scalars: only local assignment supported"));
        };
        // Pointer local assign: `p = &x` / `p = q`.
        if matches!(self.locals.get(id).map(|l| l.ty), Some(Type::Ptr(_))) {
            if !matches!(op, AssignOp::Eq) {
                return Err(diag("native pointers: only simple `=` to pointer local"));
            }
            let slot = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("internal: pointer assign missing alloca"))?;
            let rhs = self.emit_ptr_expr(value)?;
            writeln!(self.body, "  store ptr {rhs}, ptr {slot}").ok();
            return Ok(rhs);
        }
        let sty = match scalar_of_type(*ty) {
            Some(s) => s,
            None => self.local_scalar(*id)?,
        };
        let ptr = self
            .allocas
            .get(id)
            .cloned()
            .ok_or_else(|| diag("internal: assign missing alloca"))?;

        let rhs = if matches!(op, AssignOp::Eq) {
            self.emit_expr(value, Some(sty))?
        } else {
            if sty.is_bool() {
                return Err(diag("native scalars: compound assign needs numeric local"));
            }
            let nty = sty.native();
            let cur = self.fresh_tmp();
            writeln!(self.body, "  {cur} = load {}, ptr {ptr}", llvm_ty(nty)).ok();
            let rhs_v = self.emit_expr(value, Some(sty))?;
            let t = self.fresh_tmp();
            let ll = llvm_ty(nty);
            if nty.is_float() {
                match op {
                    AssignOp::AddEq => {
                        writeln!(self.body, "  {t} = fadd {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::SubEq => {
                        writeln!(self.body, "  {t} = fsub {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::MulEq => {
                        writeln!(self.body, "  {t} = fmul {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::DivEq => {
                        writeln!(self.body, "  {t} = fdiv {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::RemEq => {
                        writeln!(self.body, "  {t} = frem {ll} {cur}, {rhs_v}").ok()
                    }
                    _ => {
                        return Err(diag(&format!(
                            "native scalars: unsupported float compound assign {op:?}"
                        )))
                    }
                };
            } else {
                match op {
                    AssignOp::AddEq => {
                        writeln!(self.body, "  {t} = add {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::SubEq => {
                        writeln!(self.body, "  {t} = sub {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::MulEq => {
                        writeln!(self.body, "  {t} = mul {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::DivEq => {
                        if nty.is_signed() {
                            writeln!(self.body, "  {t} = sdiv {ll} {cur}, {rhs_v}").ok()
                        } else {
                            writeln!(self.body, "  {t} = udiv {ll} {cur}, {rhs_v}").ok()
                        }
                    }
                    AssignOp::RemEq => {
                        if nty.is_signed() {
                            writeln!(self.body, "  {t} = srem {ll} {cur}, {rhs_v}").ok()
                        } else {
                            writeln!(self.body, "  {t} = urem {ll} {cur}, {rhs_v}").ok()
                        }
                    }
                    AssignOp::BitAndEq => {
                        writeln!(self.body, "  {t} = and {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::BitOrEq => {
                        writeln!(self.body, "  {t} = or {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::BitXorEq => {
                        writeln!(self.body, "  {t} = xor {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::ShlEq => {
                        writeln!(self.body, "  {t} = shl {ll} {cur}, {rhs_v}").ok()
                    }
                    AssignOp::ShrEq => {
                        if nty.is_signed() {
                            writeln!(self.body, "  {t} = ashr {ll} {cur}, {rhs_v}").ok()
                        } else {
                            writeln!(self.body, "  {t} = lshr {ll} {cur}, {rhs_v}").ok()
                        }
                    }
                    _ => {
                        return Err(diag(&format!(
                            "native scalars: unsupported compound assign {op:?}"
                        )))
                    }
                };
            }
            t
        };
        writeln!(self.body, "  store {} {rhs}, ptr {ptr}", sty.llvm_ty()).ok();
        Ok(rhs)
    }

    fn emit_update(
        &mut self,
        op: UpdateOp,
        target: &UpdateTarget,
        prefix: bool,
        ty: &Type,
    ) -> Result<String, Diagnostic> {
        let UpdateTarget::Local(id) = target else {
            return Err(diag("native scalars: only local ++/-- supported"));
        };
        let sty = match scalar_of_type(*ty) {
            Some(s) => s,
            None => self.local_scalar(*id)?,
        };
        if sty.is_bool() || sty.is_float() {
            return Err(diag("native scalars: ++/-- needs integer local"));
        }
        let nty = sty.native();
        let ptr = self
            .allocas
            .get(id)
            .cloned()
            .ok_or_else(|| diag("internal: update missing alloca"))?;
        let cur = self.fresh_tmp();
        writeln!(self.body, "  {cur} = load {}, ptr {ptr}", llvm_ty(nty)).ok();
        let next = self.fresh_tmp();
        match op {
            UpdateOp::Inc => {
                writeln!(self.body, "  {next} = add {} {cur}, 1", llvm_ty(nty)).ok()
            }
            UpdateOp::Dec => {
                writeln!(self.body, "  {next} = sub {} {cur}, 1", llvm_ty(nty)).ok()
            }
        };
        writeln!(self.body, "  store {} {next}, ptr {ptr}", llvm_ty(nty)).ok();
        if prefix {
            Ok(next)
        } else {
            Ok(cur)
        }
    }

    fn function_sig(&self, id: LocalId) -> Result<(Vec<Scalar>, Scalar), Diagnostic> {
        for stmt in &self.module.body {
            if let Stmt::Function {
                local,
                params,
                body,
                ..
            } = stmt
            {
                if *local == id {
                    let mut ptys = Vec::new();
                    for p in params {
                        let Pattern::Local(pid) = &p.pattern else {
                            return Err(diag("native scalars: only simple params"));
                        };
                        ptys.push(self.local_scalar(*pid)?);
                    }
                    let ret = infer_return_scalar(body).unwrap_or(Scalar(NativeType::I32));
                    return Ok((ptys, ret));
                }
            }
        }
        Err(diag("native scalars: function not found for signature"))
    }

    fn local_scalar(&self, id: LocalId) -> Result<Scalar, Diagnostic> {
        let local = self
            .locals
            .get(&id)
            .ok_or_else(|| diag("internal: unknown local"))?;
        match local.ty {
            Type::Native(n) => Ok(Scalar(n)),
            Type::Boolean => Ok(Scalar(NativeType::Bool)),
            _ => Err(diag(&format!(
                "native scalars: local `{}` is not a native scalar",
                local.name
            ))),
        }
    }

    fn local_layout(&self, id: LocalId) -> Option<&ObjectShape> {
        let local = self.locals.get(&id)?;
        native_layout_of(self.module, local.ty)
    }

    /// Emit a pointer-typed expression (`*T` value): local load, `&local`, or copy.
    fn emit_ptr_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => {
                if let Some((pname, _)) = self.params.get(id) {
                    return Ok(pname.clone());
                }
                let slot = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("native pointers: local missing alloca"))?;
                let t = self.fresh_tmp();
                writeln!(self.body, "  {t} = load ptr, ptr {slot}").ok();
                Ok(t)
            }
            Expr::Unary {
                op: UnaryOp::Ref,
                arg,
                ..
            } => match arg.as_ref() {
                Expr::Local { id, .. } => self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("native pointers: address-of needs stack local")),
                _ => Err(diag(
                    "native pointers: address-of only supports direct locals",
                )),
            },
            _ => Err(diag(&format!(
                "native pointers: unsupported pointer expression {expr:?}"
            ))),
        }
    }

    fn fresh_tmp(&mut self) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("%t{n}")
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label;
        self.label += 1;
        format!("{prefix}{n}")
    }
}

fn scalar_operand_ty(
    left: &Expr,
    right: &Expr,
    expect: Option<Scalar>,
) -> Result<Scalar, Diagnostic> {
    if let Type::Native(n) = left.ty() {
        if !n.is_bool() {
            return Ok(Scalar(n));
        }
    }
    if let Type::Native(n) = right.ty() {
        if !n.is_bool() {
            return Ok(Scalar(n));
        }
    }
    if let Some(s) = expect {
        if !s.is_bool() {
            return Ok(s);
        }
    }
    Err(diag(
        "native scalars: cannot determine numeric type for operands",
    ))
}

fn format_float_const(raw: &str, ty: NativeType) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(&format!("invalid float literal {raw}")))?;
    // LLVM accepts decimal floating constants; keep enough digits for round-trip.
    let s = format!("{f:.17e}");
    match ty {
        NativeType::F32 | NativeType::F64 => Ok(s),
        _ => Err(diag("internal: format_float_const on non-float")),
    }
}

fn format_int_const(raw: &str, ty: NativeType) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let (neg, digits) = if let Some(rest) = cleaned.strip_prefix('-') {
        (true, rest)
    } else {
        (false, cleaned.as_str())
    };

    let bits = parse_int_bits(digits)?;
    let width = ty.bit_width();
    let mask = if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    let mut v = bits & mask;
    if neg {
        // two's complement negate within width
        v = (!v).wrapping_add(1) & mask;
    }

    if ty.is_signed() {
        // sign-extend interpretation for printing as LLVM signed const
        let sign_bit = 1u64 << (width - 1);
        let signed = if v & sign_bit != 0 && width < 64 {
            (v | !mask) as i64
        } else if v & sign_bit != 0 && width == 64 {
            v as i64
        } else {
            v as i64
        };
        Ok(format!("{signed}"))
    } else {
        Ok(format!("{v}"))
    }
}

fn parse_int_bits(digits: &str) -> Result<u64, Diagnostic> {
    if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16)
            .map_err(|_| diag(&format!("invalid hex literal {digits}")));
    }
    if let Some(bin) = digits
        .strip_prefix("0b")
        .or_else(|| digits.strip_prefix("0B"))
    {
        return u64::from_str_radix(bin, 2)
            .map_err(|_| diag(&format!("invalid binary literal {digits}")));
    }
    if let Some(oct) = digits
        .strip_prefix("0o")
        .or_else(|| digits.strip_prefix("0O"))
    {
        return u64::from_str_radix(oct, 8)
            .map_err(|_| diag(&format!("invalid octal literal {digits}")));
    }
    // decimal — allow float-looking only if integral
    if digits.contains('.') || digits.contains('e') || digits.contains('E') {
        let f: f64 = digits
            .parse()
            .map_err(|_| diag(&format!("invalid numeric literal {digits}")))?;
        if f.fract() != 0.0 || f < 0.0 || f > u64::MAX as f64 {
            return Err(diag(&format!(
                "native scalars: non-integral literal {digits}"
            )));
        }
        return Ok(f as u64);
    }
    digits
        .parse::<u64>()
        .map_err(|_| diag(&format!("invalid integer literal {digits}")))
}

/// Constant non-negative integer index key from IR number literal (`0` → `"0"`).
fn const_index_key(expr: &Expr) -> Option<String> {
    let raw = match expr {
        Expr::Number { raw, .. } => raw.as_str(),
        _ => return None,
    };
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u64 = raw.parse().ok()?;
    Some(n.to_string())
}

fn infer_return_scalar(body: &[Stmt]) -> Option<Scalar> {
    for stmt in body {
        if let Some(n) = infer_return_scalar_stmt(stmt) {
            return Some(n);
        }
    }
    None
}

fn infer_return_scalar_stmt(stmt: &Stmt) -> Option<Scalar> {
    match stmt {
        Stmt::Return {
            value: Some(v), ..
        } => scalar_of_type(v.ty()),
        Stmt::Block { body } => infer_return_scalar(body),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => infer_return_scalar_stmt(consequent)
            .or_else(|| alternate.as_ref().and_then(|a| infer_return_scalar_stmt(a))),
        Stmt::While { body, .. } => infer_return_scalar_stmt(body),
        _ => None,
    }
}

fn collect_declared_locals(body: &[Stmt], out: &mut Vec<LocalId>) {
    for stmt in body {
        match stmt {
            Stmt::Declare { local, .. } => out.push(*local),
            Stmt::Block { body } => collect_declared_locals(body, out),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                collect_declared_locals_stmt(consequent, out);
                if let Some(a) = alternate {
                    collect_declared_locals_stmt(a, out);
                }
            }
            Stmt::While { body, .. } => collect_declared_locals_stmt(body, out),
            _ => {}
        }
    }
}

fn collect_declared_locals_stmt(stmt: &Stmt, out: &mut Vec<LocalId>) {
    match stmt {
        Stmt::Declare { local, .. } => out.push(*local),
        Stmt::Block { body } => collect_declared_locals(body, out),
        other => collect_declared_locals(std::slice::from_ref(other), out),
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

