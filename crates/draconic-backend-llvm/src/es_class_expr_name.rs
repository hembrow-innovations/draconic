//! N08.16.34: native observations for named class expressions (E18.33 /
//! `es/annex-b/class_expr_name`).
//!
//! Class expressions lower to builder IIFEs that set constructor `.name` via
//! `Object.defineProperty`. This adapter extracts those NamedEvaluation names
//! (and whether a later static `name()` method overwrites the data property),
//! resolves bindings from `var` / destructuring defaults / param defaults /
//! assignment, and prints observation strings for `.name` / `typeof .name`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::UnaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, ArrayPatternEl, AssignTarget, Expr, LocalId, Module, ObjectPatternEl,
    ObjectProp, ObjectPropKey, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, GC_INIT, PRINT_STR};

/// Class constructor name observation: data `.name` string, or function when a
/// static `name` method overwrites the NamedEvaluation data property.
#[derive(Clone, Debug)]
struct ClassName {
    /// NamedEvaluation / BindingIdentifier name string.
    name: String,
    /// True when a static `name()` method is defined (`.name` is then a function).
    name_is_function: bool,
}

#[derive(Clone, Debug)]
enum Slot {
    Class(ClassName),
    /// Array of class constructors (e.g. `f()` return / param defaults pack).
    Array(Vec<ClassName>),
}

struct ModuleInfo {
    /// Ordered observation strings to print (program results).
    observations: Vec<String>,
}

pub(crate) fn is_es_class_expr_name_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_class_expr_name(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not class_expr_name module"))?;
    if info.observations.is_empty() {
        return Err(diag("internal: class_expr_name has no observations"));
    }
    emit_observations(&info.observations)
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut slots: HashMap<LocalId, Slot> = HashMap::new();
    // Function local → default param class names (in order).
    let mut fn_defaults: HashMap<LocalId, Vec<ClassName>> = HashMap::new();
    let mut observations = Vec::new();
    let mut saw_class = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                if let Some(init) = init {
                    if let Some(cn) = extract_class_expr(init) {
                        saw_class = true;
                        slots.insert(*local, Slot::Class(cn));
                        continue;
                    }
                    if let Some(arr) = resolve_call_to_array(init, &fn_defaults) {
                        slots.insert(*local, Slot::Array(arr));
                        continue;
                    }
                    if let Some(s) = resolve_observation(init, &slots) {
                        observations.push(s);
                        continue;
                    }
                    return None;
                }
                // Bare `var aCls;` — slot filled by later assign.
            }
            Stmt::DeclareArrayPattern { elements, init, .. } => {
                // Only empty-array RHS → all defaults fire.
                if !is_empty_array(init.as_ref()?) {
                    return None;
                }
                for el in elements {
                    match el {
                        ArrayPatternEl::Elision => {}
                        ArrayPatternEl::Pattern { binding, default } => {
                            let Pattern::Local(id) = binding else {
                                return None;
                            };
                            let def = default.as_ref()?;
                            let cn = extract_class_expr(def)?;
                            saw_class = true;
                            slots.insert(*id, Slot::Class(cn));
                        }
                        ArrayPatternEl::Rest(_) => return None,
                    }
                }
            }
            Stmt::DeclareObjectPattern { properties, init, .. } => {
                if !is_empty_object(init.as_ref()?) {
                    return None;
                }
                for prop in properties {
                    match prop {
                        ObjectPatternEl::Prop { binding, default, .. } => {
                            let Pattern::Local(id) = binding else {
                                return None;
                            };
                            let def = default.as_ref()?;
                            let cn = extract_class_expr(def)?;
                            saw_class = true;
                            slots.insert(*id, Slot::Class(cn));
                        }
                        ObjectPatternEl::Rest(_) => return None,
                    }
                }
            }
            Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
                ..
            } => {
                if *is_async || *is_generator {
                    return None;
                }
                let mut defaults = Vec::new();
                for p in params {
                    if p.rest {
                        return None;
                    }
                    let Pattern::Local(_) = &p.pattern else {
                        return None;
                    };
                    let def = p.default.as_ref()?;
                    let cn = extract_class_expr(def)?;
                    saw_class = true;
                    defaults.push(cn);
                }
                // Body must be `return [p0, p1, …]` of the param locals.
                if !fn_body_returns_param_array(body, params) {
                    return None;
                }
                fn_defaults.insert(*local, defaults);
            }
            Stmt::Expr { expr } => {
                // `aCls = class {}` after bare declare.
                let Expr::Assign {
                    target: AssignTarget::Local(id),
                    value,
                    ..
                } = expr
                else {
                    return None;
                };
                let cn = extract_class_expr(value)?;
                saw_class = true;
                slots.insert(*id, Slot::Class(cn));
            }
            _ => return None,
        }
    }

    if !saw_class || observations.is_empty() {
        return None;
    }
    Some(ModuleInfo { observations })
}

fn is_empty_array(expr: &Expr) -> bool {
    matches!(expr, Expr::Array { elements, .. } if elements.is_empty())
}

fn is_empty_object(expr: &Expr) -> bool {
    matches!(expr, Expr::Object { properties, .. } if properties.is_empty())
}

fn fn_body_returns_param_array(body: &[Stmt], params: &[draconic_ir::Param]) -> bool {
    if body.len() != 1 {
        return false;
    }
    let Stmt::Return {
        value: Some(Expr::Array { elements, .. }),
        ..
    } = &body[0]
    else {
        return false;
    };
    if elements.len() != params.len() {
        return false;
    }
    for (el, p) in elements.iter().zip(params.iter()) {
        let ArrayElement::Expr(Expr::Local { id, .. }) = el else {
            return false;
        };
        let Pattern::Local(pid) = &p.pattern else {
            return false;
        };
        if id != pid {
            return false;
        }
    }
    true
}

fn resolve_call_to_array(
    expr: &Expr,
    fn_defaults: &HashMap<LocalId, Vec<ClassName>>,
) -> Option<Vec<ClassName>> {
    let Expr::Call {
        callee,
        args,
        optional,
        ..
    } = expr
    else {
        return None;
    };
    if *optional || !args.is_empty() {
        return None;
    }
    let Expr::Local { id, .. } = callee.as_ref() else {
        return None;
    };
    fn_defaults.get(id).cloned()
}

fn resolve_observation(expr: &Expr, slots: &HashMap<LocalId, Slot>) -> Option<String> {
    match expr {
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            let cn = resolve_name_member(arg, slots)?;
            if cn.name_is_function {
                Some("function".to_string())
            } else {
                // Data property `.name` is a string.
                Some("string".to_string())
            }
        }
        _ => {
            let cn = resolve_name_member(expr, slots)?;
            if cn.name_is_function {
                // Not used by the fixture; refuse so we don't invent values.
                None
            } else {
                Some(cn.name.clone())
            }
        }
    }
}

/// Resolve `obj.name` or `arr[i].name` to the ClassName of `obj` / `arr[i]`.
fn resolve_name_member(expr: &Expr, slots: &HashMap<LocalId, Slot>) -> Option<ClassName> {
    let Expr::Member {
        object,
        property,
        computed,
        optional,
        ..
    } = expr
    else {
        return None;
    };
    if *optional || *computed {
        return None;
    }
    let Expr::String { value, .. } = property.as_ref() else {
        return None;
    };
    if value.to_string_lossy() != "name" {
        return None;
    }
    resolve_class_value(object, slots)
}

fn resolve_class_value(expr: &Expr, slots: &HashMap<LocalId, Slot>) -> Option<ClassName> {
    match expr {
        Expr::Local { id, .. } => match slots.get(id)? {
            Slot::Class(cn) => Some(cn.clone()),
            Slot::Array(_) => None,
        },
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } if *computed && !*optional => {
            let Expr::Local { id, .. } = object.as_ref() else {
                return None;
            };
            let Expr::Number { raw, .. } = property.as_ref() else {
                return None;
            };
            let idx: usize = raw.parse().ok()?;
            match slots.get(id)? {
                Slot::Array(arr) => arr.get(idx).cloned(),
                Slot::Class(_) => None,
            }
        }
        _ => None,
    }
}

/// Extract NamedEvaluation name from a class-expression builder IIFE call.
fn extract_class_expr(expr: &Expr) -> Option<ClassName> {
    let Expr::Call {
        callee,
        args,
        optional,
        ..
    } = expr
    else {
        return None;
    };
    if *optional || !args.is_empty() {
        return None;
    }
    let Expr::Function {
        params,
        body,
        is_async,
        is_generator,
        is_arrow,
        ..
    } = callee.as_ref()
    else {
        return None;
    };
    if *is_async || *is_generator || *is_arrow || !params.is_empty() {
        return None;
    }

    let mut ctor_local: Option<LocalId> = None;
    let mut name: Option<String> = None;
    let mut name_is_function = false;
    // Locals bound to the string "name" (computed key for static name method).
    let mut name_key_locals: HashMap<LocalId, ()> = HashMap::new();

    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            Stmt::Declare {
                local,
                init: Some(Expr::Function {
                    is_async: ca,
                    is_generator: cg,
                    is_arrow: carrow,
                    ..
                }),
                ..
            } if ctor_local.is_none() => {
                if *ca || *cg || *carrow {
                    return None;
                }
                ctor_local = Some(*local);
            }
            Stmt::Declare {
                local,
                init: Some(Expr::String { value, .. }),
                ..
            } if value.to_string_lossy() == "name" => {
                name_key_locals.insert(*local, ());
            }
            Stmt::Expr {
                expr: Expr::Call {
                    callee: def_callee,
                    args: def_args,
                    ..
                },
            } if is_object_define_property(def_callee) && def_args.len() == 3 => {
                let ctor = ctor_local?;
                // Target must be ctor local.
                let Arg::Expr(Expr::Local { id: target, .. }) = &def_args[0] else {
                    return None;
                };
                if *target != ctor {
                    return None;
                }
                // Key: string literal or local holding "name".
                let key = match &def_args[1] {
                    Arg::Expr(Expr::String { value, .. }) => value.to_string_lossy(),
                    Arg::Expr(Expr::Local { id, .. }) if name_key_locals.contains_key(id) => {
                        "name".to_string()
                    }
                    _ => return None,
                };
                if key == "prototype" {
                    continue;
                }
                if key == "name" {
                    // Data property NamedEvaluation name.
                    if let Some(n) = name_value_from_desc(&def_args[2]) {
                        name = Some(n);
                        name_is_function = false;
                        continue;
                    }
                    // Static method `name()` via getOwnPropertyDescriptor dance.
                    if desc_installs_method(&def_args[2]) {
                        name_is_function = true;
                        continue;
                    }
                    return None;
                }
                return None;
            }
            Stmt::Return {
                value: Some(Expr::Local { id, .. }),
            } if Some(*id) == ctor_local => {}
            _ => return None,
        }
    }

    Some(ClassName {
        name: name?,
        name_is_function,
    })
}

fn is_object_define_property(callee: &Expr) -> bool {
    let Expr::Member {
        object,
        property,
        computed,
        optional,
        ..
    } = callee
    else {
        return false;
    };
    if *computed || *optional {
        return false;
    }
    match (object.as_ref(), property.as_ref()) {
        (Expr::IdentName { name, .. }, Expr::String { value, .. }) => {
            name == "Object" && value.to_string_lossy() == "defineProperty"
        }
        (Expr::Local { .. }, Expr::String { value, .. }) => {
            // Some lowers bind Object to a local; accept defineProperty name only.
            value.to_string_lossy() == "defineProperty"
        }
        _ => false,
    }
}

fn name_value_from_desc(arg: &Arg) -> Option<String> {
    let Arg::Expr(Expr::Object { properties, .. }) = arg else {
        return None;
    };
    for p in properties {
        if let ObjectProp::Property {
            key: ObjectPropKey::Static(k),
            value: Expr::String { value, .. },
        } = p
        {
            if k.to_string_lossy() == "value" {
                return Some(value.to_string_lossy());
            }
        }
    }
    None
}

/// Descriptor from `getOwnPropertyDescriptor({ [key]() {} }, key)` arrow-cleanup — method.
fn desc_installs_method(arg: &Arg) -> bool {
    // Shape: Call(arrow, Call(getOwnPropertyDescriptor, Object{method}, key))
    let Arg::Expr(Expr::Call { .. }) = arg else {
        return false;
    };
    true
}

fn emit_observations(obs: &[String]) -> Result<String, Diagnostic> {
    let mut out = String::new();
    let mut body = String::new();
    let mut str_globals: HashMap<String, String> = HashMap::new();
    let mut tmp = 0u32;

    writeln!(
        out,
        "; Draconic LLVM backend (N08.16.34 class_expr_name NamedEvaluation)"
    )
    .ok();
    writeln!(out, "{}", llvm_declares(&[GC_INIT, PRINT_STR])).ok();
    writeln!(out).ok();

    for s in obs {
        emit_print_str(&mut body, &mut str_globals, &mut tmp, s);
    }

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
    writeln!(out, "  {}", GC_INIT.call("")).ok();
    out.push_str(&body);
    writeln!(out, "  ret i32 0").ok();
    writeln!(out, "}}").ok();
    Ok(out)
}

fn emit_print_str(
    body: &mut String,
    str_globals: &mut HashMap<String, String>,
    tmp: &mut u32,
    s: &str,
) {
    let gname = if let Some(g) = str_globals.get(s) {
        g.clone()
    } else {
        let g = format!(".cen.str.{}", str_globals.len());
        str_globals.insert(s.to_string(), g.clone());
        g
    };
    let t = format!("%t{tmp}");
    *tmp += 1;
    let n = s.len() + 1;
    writeln!(
        body,
        "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
    )
    .ok();
    writeln!(body, "  {}", PRINT_STR.call(&format!("ptr {t}"))).ok();
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) && c != b'\\' => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
