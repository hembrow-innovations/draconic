//! N01–N03.03: lower pure native scalar/layout/pointer Programs to LLVM IR.
//! N08.17 / T06: JS `number` locals may mix with unboxed natives (dual-worlds
//! boundary); `number` lowers as IEEE-754 double with explicit int↔float casts.

use std::collections::{HashMap, HashSet};

use draconic_ast::{AssignOp, BinaryOp, UnaryOp, UpdateOp};
use draconic_diagnostics::{Diagnostic, SourceFile, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, NativeType,
    ObjectProp, ObjectPropKey, ObjectShape, Param, Pattern, Stmt, UpdateTarget,
};
use draconic_runtime::abi::{
    llvm_declares, NATIVE_INT_DECLARES, PRINT_BOOL, PRINT_F64, PRINT_I64, PRINT_U64,
};

use crate::debug_info::{dbg_marker, SourceDebug};

mod emit;
mod expr;
mod layout;
mod ops;

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
        // Dual-worlds (T06 / N08.17): JS `number` is unboxed IEEE-754 double on native.
        Type::Number => Some(Scalar(NativeType::F64)),
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

fn native_layout_of(module: &Module, ty: Type) -> Option<&ObjectShape> {
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

fn layout_size(shape: &ObjectShape) -> u32 {
    let mut off = 0u32;
    let st_align = layout_align(shape).max(1);
    for (_, t) in &shape.props {
        let Some(sc) = scalar_of_type(*t) else {
            continue;
        };
        let a = sc.align().max(1);
        off = off.div_ceil(a) * a;
        off += a;
    }
    off.div_ceil(st_align) * st_align
}

/// AAPCS64 / SysV integer-class aggregate: memory image in 1 or 2 GPRs (F03.02).
fn layout_abi_llvm(shape: &ObjectShape) -> String {
    if layout_size(shape) <= 8 {
        "i64".to_string()
    } else {
        "[2 x i64]".to_string()
    }
}

/// True when every **user-declared** local is a native scalar (`i*`/`u*`/`f*`/`bool`),
/// a **native layout** shape (all-native fields), a **native pointer** (`*T`),
/// JS `number` (dual-worlds unboxed double), a **function declaration** binding,
/// or an **`extern "C"`** binding (F06.03), and the module has at least one native
/// scalar/layout/pointer local **or** an extern ABI decl (N01–N03.03 + N08.17 + F06.03).
///
/// Arrow / function-expression bindings are excluded so T05 erase fixtures that
/// mix natives with callable values stay on the B08 hello stub. JS `boolean` or
/// pure-`number` locals alone also do not qualify (those stay on ES paths).
/// Globals (Object/Function builtins) ignored.
pub(crate) fn is_native_int_module(module: &Module) -> bool {
    let mut user = HashSet::new();
    collect_user_local_ids(&module.body, &mut user);
    if user.is_empty() {
        return false;
    }
    let fn_decl_locals = function_decl_local_ids(&module.body);
    let extern_locals = extern_decl_local_ids(&module.body);
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut has_native = false;
    let mut has_extern = false;
    for id in user {
        let Some(local) = by_id.get(&id) else {
            return false;
        };
        match local.ty {
            Type::Native(_) => has_native = true,
            Type::Ptr(_) => has_native = true,
            Type::Shape(_) if native_layout_of(module, local.ty).is_some() => has_native = true,
            // Dual-worlds: allow JS number alongside natives; alone does not claim this path.
            Type::Number => {}
            Type::Function if fn_decl_locals.contains(&id) => {}
            Type::Function if extern_locals.contains(&id) => has_extern = true,
            _ => return false,
        }
    }
    has_native || has_extern
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
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } | Stmt::Labeled { body, .. } => {
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

fn extern_decl_local_ids(body: &[Stmt]) -> HashSet<LocalId> {
    let mut out = HashSet::new();
    for stmt in body {
        if let Stmt::ExternFunction { local, .. } = stmt {
            out.insert(*local);
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
        Stmt::ExternFunction { local, .. } => {
            out.insert(*local);
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
        Stmt::For { init, body, .. } => {
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
        Pattern::Name(_) | Pattern::Member { .. } => {}
        Pattern::Array(els) => {
            for el in els {
                match el {
                    draconic_ir::ArrayPatternEl::Elision => {}
                    draconic_ir::ArrayPatternEl::Pattern { binding, .. } => {
                        collect_pattern_locals(binding, out);
                    }
                    draconic_ir::ArrayPatternEl::Rest(binding) => {
                        collect_pattern_locals(binding, out);
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
                    draconic_ir::ObjectPatternEl::Rest(binding) => {
                        collect_pattern_locals(binding, out);
                    }
                }
            }
        }
    }
}

pub(crate) fn emit_native_ints(
    module: &Module,
    debug: Option<&SourceDebug>,
) -> Result<String, Diagnostic> {
    let mut em = Emitter::new(module, debug);
    em.emit_module()?;
    Ok(em.finish())
}

/// C ABI surface for one `extern "C"` decl (F06.03).
#[derive(Debug, Clone)]
struct ExternAbi {
    /// Linkage symbol (source name).
    name: String,
    params: Vec<Type>,
    /// `None` = void return.
    ret: Option<Type>,
}

struct Emitter<'a> {
    module: &'a Module,
    debug: Option<&'a SourceDebug>,
    locals: HashMap<LocalId, &'a Local>,
    /// Alloca pointer SSA name per local: `%l{id}`
    allocas: HashMap<LocalId, String>,
    /// Function IR local → LLVM function name (user `define` or extern `declare` symbol)
    fn_names: HashMap<LocalId, String>,
    /// Extern function locals (C ABI; call uses source linkage name + typed params/ret)
    extern_fns: HashMap<LocalId, ExternAbi>,
    /// Function param locals (no alloca; SSA param name)
    params: HashMap<LocalId, (String, Scalar)>,
    out: String,
    body: String,
    tmp: u32,
    label: u32,
    /// Top-level native scalar locals to print at end of main (declare order).
    print_order: Vec<LocalId>,
    /// Index into `module.body` while emitting top-level statements (for DWARF).
    body_idx: usize,
}

fn scalar_operand_ty(
    left: &Expr,
    right: &Expr,
    expect: Option<Scalar>,
) -> Result<Scalar, Diagnostic> {
    if let Some(s) = scalar_of_type(left.ty()) {
        if !s.is_bool() {
            return Ok(s);
        }
    }
    if let Some(s) = scalar_of_type(right.ty()) {
        if !s.is_bool() {
            return Ok(s);
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
        Stmt::Return { value: Some(v), .. } => scalar_of_type(v.ty()),
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
