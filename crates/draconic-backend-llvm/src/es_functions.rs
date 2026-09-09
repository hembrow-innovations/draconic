//! N08.03.01–N08.03.07 + N08.16.11–N08.16.14 + N08.16.24: native observations for ES
//! function declarations, expressions, and arrows (simple ident params + defaults +
//! rest) — E03.01–E03.07 / `es/functions/*`, Annex B labelled function declarations —
//! E18.11 / `es/annex-b/labelled_function`, Annex B FunctionDeclarations in
//! `if` — E18.12 / `es/annex-b/if_function`, block-level function declarations —
//! E18.13 / `es/annex-b/block_function`, `var` declarations (hoist, redeclare,
//! uninit → undefined) — E18.14 / `es/annex-b/var_decl`, and the `arguments` object
//! (`arguments.length` / `arguments[i]`) — E18.24 / `es/annex-b/arguments_object`.
//!
//! Nested/non-escaping decls use extra by-value capture params. Function
//! expressions and arrows are first-class as fn-id doubles; returned closures
//! stash captures in a small return buffer for immediate call (`make(10)(7)`).
//! Missing/undefined args use a NaN payload sentinel; callee applies defaults.
//! Rest params pack trailing args into a stack buffer of doubles; `for-of` over
//! the rest local iterates that buffer (no full JS array heap).
//! Labelled function declarations (`L: function f(){…}`) unwrap to ordinary decls.
//! If-clause function decls (Annex B.3.4) bind only when the branch runs; same-name
//! then/else share one slot; `typeof` on an unbound name is `"undefined"`.
//! Block-level function decls (Annex B.3.2) activate when the block runs; same-name
//! redecls share one outer slot (last activation wins).
//! `var` is function/script-scoped: slots hoist to entry as undefined; same-name
//! redecls share one primary; `typeof` of uninit is `"undefined"`.
//! Non-arrow functions that read `arguments` receive a packed args buffer + argc;
//! object-literal methods with static keys are callables via `obj.m(...)`.

use std::collections::{HashMap, HashSet};

use draconic_ast::AssignOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, BindingKind, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, PRINT_F64, PRINT_STR};

use crate::emitter::{escape_llvm_bytes, Emitter as IrEmitter, SlotTy};

#[path = "es_functions_classify.rs"]
mod classify;
#[path = "es_functions_collect.rs"]
mod collect;
#[path = "es_functions_ok.rs"]
mod ok;
#[path = "es_functions_emit.rs"]
mod emit;
#[path = "es_functions_emit_expr.rs"]
mod emit_expr;

use classify::classify;

const MAX_CAPS: usize = 8;
/// Max trailing rest arguments packed into the stack buffer (fixture uses ≤3).
const MAX_REST: usize = 8;
/// Max call arity / `arguments` buffer length (fixture uses ≤3).
const MAX_ARGS: usize = 8;
/// qNaN payload marking JS `undefined` for default-parameter application.
const UNDEF_BITS: u64 = 0x7FF8_0000_0000_0001;

pub(crate) fn walk_es_functions(module: &Module) -> Option<Result<String, Diagnostic>> {
    let info = classify(module)?;
    Some(emit_classified(module, info))
}

fn emit_classified(module: &Module, info: ModuleInfo) -> Result<String, Diagnostic> {
    let mut em = Emitter::new_dedicated_labels(module, FnState::new(&info));
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone)]
struct FnInfo {
    /// Stable index → LLVM name `d_fn_{idx}`.
    idx: usize,
    /// Fixed (non-rest) params only.
    params: Vec<LocalId>,
    /// Parallel to `params`; `Some` → apply when arg missing/undefined.
    defaults: Vec<Option<Expr>>,
    /// Last param `...rest` local, if any.
    rest: Option<LocalId>,
    /// Implicit `arguments` local when body reads `arguments.length` / `arguments[i]`.
    arguments: Option<LocalId>,
    captures: Vec<LocalId>,
    body: Vec<Stmt>,
    /// Named function expression recursive binding.
    name_local: Option<LocalId>,
}

struct ModuleInfo {
    functions: Vec<FnInfo>,
    /// Locals statically bound to a function index (decl / expr assign / name).
    fn_binding: HashMap<LocalId, usize>,
    /// Top-level user locals to print (declare order): numbers and typeof-strings.
    user_locals: Vec<LocalId>,
    /// Subset of `user_locals` holding typeof string observations.
    string_locals: HashSet<LocalId>,
    /// Annex B if-clause primary binding locals (runtime i32 fn-idx slot; -1 = unbound).
    if_fn_slots: HashSet<LocalId>,
    /// If-clause Function local → primary slot local (then/else same name share primary).
    if_fn_primary: HashMap<LocalId, LocalId>,
    /// Primary slot → possible fn idxs that may be stored there (for dynamic dispatch).
    if_fn_candidates: HashMap<LocalId, Vec<usize>>,
    /// `var` redeclare/use local → primary storage (same name, script/function scope).
    var_primary: HashMap<LocalId, LocalId>,
    /// Top-level (script) hoisted `var` primary slots.
    top_var_slots: HashSet<LocalId>,
    /// Per-function idx → hoisted `var` primary slots in that body.
    fn_var_slots: HashMap<usize, HashSet<LocalId>>,
    /// Object local → static method name → fn idx (`{ m: function… }`).
    obj_methods: HashMap<LocalId, HashMap<String, usize>>,
}

struct FnState<'a> {
    info: &'a ModuleInfo,
    fn_names: HashMap<usize, String>,
    allocas: HashMap<LocalId, String>,
    rest_slots: HashMap<LocalId, (String, String)>,
    arguments_slots: HashMap<LocalId, (String, String)>,
    if_fn_slot_ptrs: HashMap<LocalId, String>,
    typeof_code_ptrs: HashMap<LocalId, String>,
    str_globals: HashMap<String, String>,
}

impl<'a> FnState<'a> {
    fn new(info: &'a ModuleInfo) -> Self {
        let mut fn_names = HashMap::new();
        for f in &info.functions {
            fn_names.insert(f.idx, format!("d_fn_{}", f.idx));
        }
        Self {
            info,
            fn_names,
            allocas: HashMap::new(),
            rest_slots: HashMap::new(),
            arguments_slots: HashMap::new(),
            if_fn_slot_ptrs: HashMap::new(),
            typeof_code_ptrs: HashMap::new(),
            str_globals: HashMap::new(),
        }
    }
}

type Emitter<'a> = IrEmitter<'a, FnState<'a>>;

/// First `arguments` local referenced via Member in `body`, if any.
fn find_arguments_local(body: &[Stmt], by_id: &HashMap<LocalId, &Local>) -> Option<LocalId> {
    let mut found = None;
    fn walk_expr(expr: &Expr, by_id: &HashMap<LocalId, &Local>, found: &mut Option<LocalId>) {
        if found.is_some() {
            return;
        }
        match expr {
            Expr::Member {
                object, property, ..
            } => {
                if let Expr::Local { id, .. } = object.as_ref() {
                    if by_id.get(id).map(|l| l.name.as_str()) == Some("arguments") {
                        *found = Some(*id);
                        return;
                    }
                }
                walk_expr(object, by_id, found);
                walk_expr(property, by_id, found);
            }
            Expr::Unary { arg, .. } => walk_expr(arg, by_id, found),
            Expr::Binary { left, right, .. } => {
                walk_expr(left, by_id, found);
                walk_expr(right, by_id, found);
            }
            Expr::Call { callee, args, .. } => {
                walk_expr(callee, by_id, found);
                for a in args {
                    if let Arg::Expr(e) = a {
                        walk_expr(e, by_id, found);
                    }
                }
            }
            Expr::Assign { value, .. } => walk_expr(value, by_id, found),
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                walk_expr(test, by_id, found);
                walk_expr(consequent, by_id, found);
                walk_expr(alternate, by_id, found);
            }
            Expr::Function { body, .. } => {
                // Nested function has its own arguments — do not claim outer.
                let _ = body;
            }
            _ => {}
        }
    }
    fn walk_stmt(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>, found: &mut Option<LocalId>) {
        if found.is_some() {
            return;
        }
        match stmt {
            Stmt::Return { value: Some(v) } => walk_expr(v, by_id, found),
            Stmt::Declare { init: Some(e), .. } => walk_expr(e, by_id, found),
            Stmt::Expr { expr } => walk_expr(expr, by_id, found),
            Stmt::Block { body } => {
                for s in body {
                    walk_stmt(s, by_id, found);
                }
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                walk_expr(test, by_id, found);
                walk_stmt(consequent, by_id, found);
                if let Some(a) = alternate {
                    walk_stmt(a, by_id, found);
                }
            }
            Stmt::Labeled { body, .. } => walk_stmt(body, by_id, found),
            Stmt::Function { body, .. } => {
                let _ = body;
            }
            _ => {}
        }
    }
    for s in body {
        walk_stmt(s, by_id, &mut found);
    }
    found
}

fn static_prop_name(property: &Expr) -> Option<String> {
    match property {
        Expr::String { value, .. } => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn parse_nonneg_index(raw: &str) -> Option<usize> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f: f64 = cleaned.parse().ok()?;
    if f.is_finite() && f >= 0.0 && f.fract() == 0.0 && f < (MAX_ARGS as f64) {
        Some(f as usize)
    } else {
        None
    }
}
/// Unwrap `L: …: function f` / bare `function f` as if/else clause.
fn unwrap_if_fn_local(stmt: &Stmt) -> Option<LocalId> {
    let mut s = stmt;
    while let Stmt::Labeled { body, .. } = s {
        s = body;
    }
    match s {
        Stmt::Function { local, .. } => Some(*local),
        _ => None,
    }
}
/// Match a FunctionExpr to its `FnInfo` by param local ids (unique per lower).
fn find_fn_idx_by_param_patterns(params: &[Param], out: &[FnInfo]) -> Option<usize> {
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
    out.iter()
        .find(|f| {
            let mut all = f.params.clone();
            if let Some(r) = f.rest {
                all.push(r);
            }
            all == ids
        })
        .map(|f| f.idx)
}
fn simple_params(
    params: &[Param],
    by_id: &HashMap<LocalId, &Local>,
) -> Option<(Vec<LocalId>, Vec<Option<Expr>>, Option<LocalId>)> {
    let mut ids = Vec::with_capacity(params.len());
    let mut defaults = Vec::with_capacity(params.len());
    let mut rest = None;
    for (i, p) in params.iter().enumerate() {
        let Pattern::Local(id) = &p.pattern else {
            return None;
        };
        let loc = by_id.get(id)?;
        if !matches!(loc.ty, Type::Number | Type::Any) {
            return None;
        }
        if p.rest {
            if i != params.len() - 1 || p.default.is_some() || rest.is_some() {
                return None;
            }
            rest = Some(*id);
        } else {
            ids.push(*id);
            defaults.push(p.default.clone());
        }
    }
    Some((ids, defaults, rest))
}

fn call_arity_ok(f: &FnInfo, args_len: usize) -> bool {
    if f.arguments.is_some() {
        return args_len <= MAX_ARGS;
    }
    if f.rest.is_some() {
        if args_len >= f.params.len() {
            args_len - f.params.len() <= MAX_REST
        } else {
            f.defaults[args_len..].iter().all(|d| d.is_some())
        }
    } else if args_len > f.params.len() {
        false
    } else {
        f.defaults[args_len..].iter().all(|d| d.is_some())
    }
}

fn call_arity_ok_params(
    defaults: &[Option<Expr>],
    has_rest: bool,
    has_arguments: bool,
    args_len: usize,
) -> bool {
    if has_arguments {
        return args_len <= MAX_ARGS;
    }
    if has_rest {
        if args_len >= defaults.len() {
            args_len - defaults.len() <= MAX_REST
        } else {
            defaults[args_len..].iter().all(|d| d.is_some())
        }
    } else if args_len > defaults.len() {
        false
    } else {
        defaults[args_len..].iter().all(|d| d.is_some())
    }
}

fn undef_double_const() -> String {
    format!("bitcast (i64 {UNDEF_BITS} to double)")
}

fn fn_id_for_name(
    name: &str,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
) -> Option<LocalId> {
    fn_binding
        .keys()
        .copied()
        .filter(|id| by_id.get(id).is_some_and(|l| l.name == name))
        .max_by_key(|id| id.0)
}

fn callee_fn_id(
    callee: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_binding: &HashMap<LocalId, usize>,
) -> Option<LocalId> {
    match callee {
        Expr::Local { id, .. } => Some(*id),
        Expr::IdentName { name, .. } => fn_id_for_name(name, by_id, fn_binding),
        _ => None,
    }
}

fn number_id_named(name: &str, by_id: &HashMap<LocalId, &Local>) -> Option<LocalId> {
    by_id
        .iter()
        .filter(|(_, loc)| loc.name == name && matches!(loc.ty, Type::Number | Type::Any))
        .map(|(id, _)| *id)
        .min_by_key(|id| id.0)
}
fn format_number_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(format!("invalid number literal {raw}")))?;
    Ok(format!("{f:.17e}"))
}

/// Whether `body` contains an if-clause Function for primary/local `id`.
fn stmt_list_mentions_if_fn(body: &[Stmt], id: LocalId) -> bool {
    for stmt in body {
        if stmt_mentions_if_fn(stmt, id) {
            return true;
        }
    }
    false
}

fn stmt_mentions_if_fn(stmt: &Stmt, id: LocalId) -> bool {
    match stmt {
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            if unwrap_if_fn_local(consequent) == Some(id)
                || alternate.as_ref().and_then(|a| unwrap_if_fn_local(a)) == Some(id)
            {
                return true;
            }
            // Primary may be consequent while else has different local aliased to primary.
            if let Some(cl) = unwrap_if_fn_local(consequent) {
                if cl == id {
                    return true;
                }
            }
            if let Some(alt) = alternate {
                if let Some(al) = unwrap_if_fn_local(alt) {
                    // Caller checks primary set membership separately; match either local.
                    if al == id {
                        return true;
                    }
                }
                if stmt_mentions_if_fn(alt, id) {
                    return true;
                }
            }
            stmt_mentions_if_fn(consequent, id)
        }
        Stmt::Block { body } => stmt_list_mentions_if_fn(body, id),
        Stmt::Labeled { body, .. } => stmt_mentions_if_fn(body, id),
        Stmt::Function { body, local, .. } => *local == id || stmt_list_mentions_if_fn(body, id),
        _ => false,
    }
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
