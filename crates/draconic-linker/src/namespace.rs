use std::collections::HashMap;

use draconic_ast::{Expr, Ident, Stmt};
use draconic_diagnostics::{codes, Diagnostic, Span};
use draconic_parser::parse;

use crate::eval::module_body_has_tla;
use crate::load::Loader;

/// Sentinel local name: export BindingName is ~namespace~ (module namespace object).
pub(crate) const BINDING_NAMESPACE: &str = "\0namespace";
/// Sentinel local name: export BindingName is a deferred module namespace
/// (E19.84.02) — distinct shared object from the eager namespace.
pub(crate) const BINDING_DEFERRED_NAMESPACE: &str = "\0deferred-namespace";

pub(crate) fn shared_namespace_binding_name(module_id: usize) -> String {
    format!("__ns{module_id}")
}

pub(crate) fn deferred_namespace_binding_name(module_id: usize) -> String {
    format!("__ns_defer{module_id}")
}

pub(crate) fn final_binding_name(
    mangled: &[HashMap<String, String>],
    def_id: usize,
    local_in_exporter: &str,
) -> Result<String, Diagnostic> {
    if local_in_exporter == BINDING_NAMESPACE {
        return Ok(shared_namespace_binding_name(def_id));
    }
    if local_in_exporter == BINDING_DEFERRED_NAMESPACE {
        return Ok(deferred_namespace_binding_name(def_id));
    }
    final_local_name(&mangled[def_id], local_in_exporter).ok_or_else(|| {
        Diagnostic::new(
            format!("export local `{local_in_exporter}` missing in defining module {def_id}"),
            Span::dummy(),
        )
        .with_code(codes::LINKER_INTERNAL)
    })
}

/// Resolve a local name through the module's mangling map (identity if unmangled).
pub(crate) fn final_local_name(mangled: &HashMap<String, String>, local: &str) -> Option<String> {
    if let Some(m) = mangled.get(local) {
        return Some(m.clone());
    }
    // Entry module: locals keep source names (not present in mangled).
    Some(local.to_string())
}

pub(crate) fn deferred_eval_fn_name(mod_id: usize) -> String {
    format!("__draconic_eval_m{mod_id}")
}

pub(crate) fn deferred_module_status_helper_stmts(
    loader: &Loader,
    n_modules: usize,
) -> Result<Vec<Stmt>, Diagnostic> {
    let mut status_inits = String::from("[");
    let mut tla_inits = String::from("[");
    let mut deps_inits = String::from("[");
    for id in 0..n_modules {
        if id > 0 {
            status_inits.push_str(", ");
            tla_inits.push_str(", ");
            deps_inits.push_str(", ");
        }
        status_inits.push('0'); // linked
        let has_tla = module_body_has_tla(&loader.modules[id].body);
        tla_inits.push_str(if has_tla { "true" } else { "false" });
        deps_inits.push('[');
        let mut first = true;
        for dep in &loader.modules[id].requested {
            if let Some(&dep_id) = loader.ids.get(dep) {
                if !first {
                    deps_inits.push_str(", ");
                }
                first = false;
                deps_inits.push_str(&dep_id.to_string());
            }
        }
        deps_inits.push(']');
    }
    status_inits.push(']');
    tla_inits.push(']');
    deps_inits.push(']');
    // E19.84.08: parallel [[EvaluationError]] slots (undefined until a throw).
    let mut error_inits = String::from("[");
    for id in 0..n_modules {
        if id > 0 {
            error_inits.push_str(", ");
        }
        error_inits.push_str("undefined");
    }
    error_inits.push(']');
    let src = format!(
        r#"
let __draconic_mstatus = {status_inits};
let __draconic_merror = {error_inits};
let __draconic_mtla = {tla_inits};
let __draconic_mdeps = {deps_inits};
function __draconic_ready(id, seen) {{
  if (seen === undefined) seen = [];
  if (seen.indexOf(id) >= 0) return true;
  seen = seen.concat([id]);
  let st = __draconic_mstatus[id];
  if (st === 3) return true;
  if (st === 1 || st === 2) return false;
  if (__draconic_mtla[id]) return false;
  let deps = __draconic_mdeps[id];
  for (let i = 0; i < deps.length; i++) {{
    if (!__draconic_ready(deps[i], seen)) return false;
  }}
  return true;
}}
"#
    );
    Ok(parse(&src)?.body)
}

pub(crate) fn deferred_namespace_helper_stmts() -> Result<Vec<Stmt>, Diagnostic> {
    // Parsed once per link that needs deferred namespaces. Node hosts lack native
    // `import defer`; this Proxy matches Test262 evaluation-trigger + MOP surface.
    let src = r#"
function __draconic_deferred_ns(evaluate, exportNames, modId) {
  let evaluated = false;
  let evalError = undefined;
  let exportsObj = null;
  let names = exportNames.slice().sort();
  // Target is a static stand-in matching a module namespace exotic object's
  // non-configurable keys, so Proxy invariants hold (E19.84.01).
  let target = Object.create(null);
  Object.defineProperty(target, Symbol.toStringTag, {
    value: "Deferred Module",
    writable: false,
    enumerable: false,
    configurable: false,
  });
  for (let i = 0; i < names.length; i++) {
    Object.defineProperty(target, names[i], {
      value: undefined,
      writable: true,
      enumerable: true,
      configurable: false,
    });
  }
  Object.preventExtensions(target);
  function ensure() {
    // E19.84.08: cached [[EvaluationError]] rethrows the same reason.
    if (evaluated) {
      if (evalError !== undefined) throw evalError;
      return exportsObj;
    }
    // E19.84.05: EnsureDeferredNamespaceEvaluation — if not ~evaluated~ and
    // ReadyForSyncExecution is false, throw TypeError (do not start evaluation).
    let st = __draconic_mstatus[modId];
    if (st !== 3 && !__draconic_ready(modId)) {
      throw new TypeError("Deferred module is not ready for synchronous evaluation");
    }
    // Already evaluated elsewhere (eager body or prior dynamic import) with error.
    if (st === 3 && __draconic_merror[modId] !== undefined) {
      evalError = __draconic_merror[modId];
      evaluated = true;
      throw evalError;
    }
    try {
      exportsObj = evaluate();
      evaluated = true;
      for (let i = 0; i < names.length; i++) {
        target[names[i]] = exportsObj[names[i]];
      }
    } catch (e) {
      evalError = __draconic_merror[modId] !== undefined ? __draconic_merror[modId] : e;
      evaluated = true;
      throw evalError;
    }
    return exportsObj;
  }
  function isSymbolLike(p) {
    return typeof p === "symbol" || p === "then";
  }
  // Traps encode deferred module-namespace trigger rules (E19.55) and the
  // deferred namespace object MOP (E19.84.01).
  return new Proxy(target, {
    get(_t, p) {
      if (p === Symbol.toStringTag) return "Deferred Module";
      if (isSymbolLike(p)) return undefined;
      let ex = ensure();
      if (Object.prototype.hasOwnProperty.call(ex, p)) return ex[p];
      return undefined;
    },
    has(_t, p) {
      if (isSymbolLike(p)) {
        return Object.prototype.hasOwnProperty.call(target, p);
      }
      let ex = ensure();
      return Object.prototype.hasOwnProperty.call(ex, p);
    },
    getOwnPropertyDescriptor(_t, p) {
      if (p === Symbol.toStringTag) {
        return { value: "Deferred Module", writable: false, enumerable: false, configurable: false };
      }
      if (isSymbolLike(p)) {
        if (!Object.prototype.hasOwnProperty.call(target, p)) return undefined;
        return { value: target[p], writable: true, enumerable: true, configurable: false };
      }
      let ex = ensure();
      if (!Object.prototype.hasOwnProperty.call(ex, p)) return undefined;
      target[p] = ex[p];
      return { value: ex[p], writable: true, enumerable: true, configurable: false };
    },
    ownKeys() {
      ensure();
      let keys = names.slice();
      keys.push(Symbol.toStringTag);
      return keys;
    },
    defineProperty(_t, p, desc) {
      if (isSymbolLike(p)) return false;
      ensure();
      return false;
    },
    deleteProperty(_t, p) {
      if (isSymbolLike(p)) return true;
      ensure();
      return false;
    },
    set() {
      return false;
    },
    getPrototypeOf() {
      return null;
    },
    setPrototypeOf(_t, p) {
      return p === null;
    },
    isExtensible() {
      return Object.isExtensible(target);
    },
    preventExtensions() {
      Object.preventExtensions(target);
      return true;
    },
  });
}
"#;
    Ok(parse(src)?.body)
}

pub(crate) fn shared_namespace_helper_stmts() -> Result<Vec<Stmt>, Diagnostic> {
    // E19.86: module namespace exotic object polyfill for eager namespaces. The
    // linked program flattens ESM into plain scripts, so `import * as ns` must
    // bind to an object that reproduces the spec [[ModuleNamespace]] MOP: null
    // prototype, non-extensible, sorted ownKeys (Symbol.toStringTag last),
    // non-configurable data descriptors, `[[Get]]` = GetBindingValue (throws
    // ReferenceError on uninitialized bindings), `[[Set]]` = false, etc. Values
    // are fetched through getter closures over the (mangled) export bindings so
    // they stay live and honor binding TDZ. Parsed once per link that needs it.
    let src = r#"
function __draconic_make_ns(pairs, exportNames, toStringTag) {
  let getters = Object.create(null);
  for (let i = 0; i < pairs.length; i++) {
    getters[pairs[i][0]] = pairs[i][1];
  }
  let names = exportNames.slice().sort();
  let target = Object.create(null);
  Object.defineProperty(target, Symbol.toStringTag, {
    value: toStringTag,
    writable: false,
    enumerable: false,
    configurable: false,
  });
  for (let i = 0; i < names.length; i++) {
    Object.defineProperty(target, names[i], {
      value: undefined,
      writable: true,
      enumerable: true,
      configurable: false,
    });
  }
  Object.preventExtensions(target);
  function hasName(p) {
    return typeof p === "string" && Object.prototype.hasOwnProperty.call(getters, p);
  }
  return new Proxy(target, {
    get(_t, p) {
      if (p === Symbol.toStringTag) return toStringTag;
      if (typeof p === "symbol") return undefined;
      if (!hasName(p)) return undefined;
      return getters[p]();
    },
    has(_t, p) {
      if (p === Symbol.toStringTag) return true;
      if (typeof p === "symbol") return false;
      return hasName(p);
    },
    getOwnPropertyDescriptor(_t, p) {
      if (p === Symbol.toStringTag) {
        return { value: toStringTag, writable: false, enumerable: false, configurable: false };
      }
      if (typeof p === "symbol") return undefined;
      if (!hasName(p)) return undefined;
      return { value: getters[p](), writable: true, enumerable: true, configurable: false };
    },
    ownKeys() {
      return names.concat([Symbol.toStringTag]);
    },
    defineProperty(_t, p, desc) {
      if (typeof p === "symbol") {
        if (p !== Symbol.toStringTag) return false;
        if (desc.configurable === true) return false;
        if (desc.writable === true) return false;
        if (desc.enumerable === true) return false;
        if ("value" in desc && desc.value !== toStringTag) return false;
        return true;
      }
      if (!hasName(p)) return false;
      let current = getters[p]();
      if (desc.configurable === true) return false;
      if (desc.enumerable === false) return false;
      if (desc.get !== undefined || desc.set !== undefined) return false;
      if (desc.writable === false) return false;
      if ("value" in desc && desc.value !== current) return false;
      return true;
    },
    deleteProperty(_t, p) {
      if (typeof p === "symbol") return p !== Symbol.toStringTag;
      return !hasName(p);
    },
    set() {
      return false;
    },
    getPrototypeOf() {
      return null;
    },
    setPrototypeOf(_t, p) {
      return p === null;
    },
    isExtensible() {
      return false;
    },
    preventExtensions() {
      return true;
    },
  });
}
"#;
    Ok(parse(src)?.body)
}

pub(crate) fn make_call_stmt(fn_name: &str, span: Span) -> Stmt {
    Stmt::Expression {
        expr: Expr::Call {
            callee: Box::new(Expr::Ident(Ident {
                name: fn_name.to_string(),
                span,
            })),
            args: vec![],
            optional: false,
            span,
        },
        span,
    }
}

pub(crate) fn make_deferred_namespace_binding(
    local: &str,
    eval_fn: Option<&str>,
    export_pairs: &[(String, String)],
    mod_id: usize,
    span: Span,
) -> Result<Stmt, Diagnostic> {
    let mut props = String::new();
    let mut names = String::new();
    for (export_name, remote) in export_pairs {
        let key = js_object_key(export_name);
        props.push_str(&format!("{key}: {remote}, "));
        let lit = export_name.replace('\\', "\\\\").replace('"', "\\\"");
        names.push_str(&format!("\"{lit}\", "));
    }
    // `eval_fn` is the once-eval thunk for a still-lazy deferred module; eager
    // modules already ran, so the closure just reads the initialized bindings.
    let eval_src = match eval_fn {
        Some(f) => format!("{f}();"),
        None => String::new(),
    };
    let src = format!(
        "let {local} = __draconic_deferred_ns(function () {{ {eval_src} return {{ {props} }}; }}, [{names}], {mod_id});"
    );
    let mut body = parse(&src)?.body;
    let stmt = body.pop().ok_or_else(|| {
        Diagnostic::new("deferred namespace binding parse produced no stmt", span)
            .with_code(codes::LINKER_INTERNAL)
    })?;
    Ok(stmt)
}

/// E19.86: build `let __ns{id} = __draconic_make_ns(pairs, names, "Module");`.
/// Each pair is `[exportName, function () { return <final binding name>; }]`, so
/// export values are read lazily (live bindings + TDZ ReferenceError) instead of
/// being snapshotted at namespace creation.
pub(crate) fn make_shared_namespace_binding(
    local: &str,
    resolved: &HashMap<String, (usize, String)>,
    mangled: &[HashMap<String, String>],
    span: Span,
) -> Result<Stmt, Diagnostic> {
    let mut names: Vec<String> = resolved.keys().cloned().collect();
    names.sort();
    let mut pairs: Vec<String> = Vec::with_capacity(names.len());
    let mut names_lit: Vec<String> = Vec::with_capacity(names.len());
    for export_name in &names {
        let (def_id, local_in_exporter) = resolved.get(export_name).expect("key from map");
        let remote = final_binding_name(mangled, *def_id, local_in_exporter)?;
        let lit = export_name.replace('\\', "\\\\").replace('"', "\\\"");
        pairs.push(format!("[\"{lit}\", function () {{ return {remote}; }}]"));
        names_lit.push(format!("\"{lit}\""));
    }
    let src = format!(
        "let {local} = __draconic_make_ns([{}], [{}], \"Module\");",
        pairs.join(", "),
        names_lit.join(", ")
    );
    let mut body = parse(&src)?.body;
    let stmt = body.pop().ok_or_else(|| {
        Diagnostic::new("shared namespace binding parse produced no stmt", span)
            .with_code(codes::LINKER_INTERNAL)
    })?;
    Ok(stmt)
}

pub(crate) fn js_object_key(name: &str) -> String {
    if is_js_ident(name) {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

pub(crate) fn is_js_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c == '_' || c == '$' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c == '$' || c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use crate::{link_entry, temp_link_dir};
    use std::fs;

    #[test]
    fn link_import_defer_namespace_lazy() {
        // E19.55: deferred namespace must not eagerly run the dependency body.
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-import-defer-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let dep = dir.join("dep.drac");
        let main = dir.join("main.drac");
        fs::write(
            &dep,
            "globalThis.side = (globalThis.side || 0) + 1;\nexport let exported = 3;\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import defer * as ns from \"./dep.drac\";\nlet x = ns;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("import defer link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("__draconic_deferred_ns") || dump.contains("draconic_deferred"),
            "expected deferred ns helper, got:\n{dump}"
        );
        assert!(
            dump.contains("__draconic_eval_m") || dump.contains("FunctionDeclaration"),
            "expected deferred eval thunk, got:\n{dump}"
        );
        // E19.84.05: ReadyForSyncExecution status machinery.
        assert!(
            dump.contains("__draconic_mstatus") && dump.contains("__draconic_ready"),
            "expected module status / ready helpers, got:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_import_defer_self_while_evaluating_status() {
        // E19.84.05: self deferred namespace during evaluation wraps body with status.
        let dir = temp_link_dir("import-defer-self-eval");
        let main = dir.join("main.drac");
        fs::write(
            &main,
            "import defer * as self from \"./main.drac\";\nexport let foo = 1;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("self defer link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("__draconic_mstatus") && dump.contains("__draconic_ready"),
            "expected status helpers:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
