//! Link ESM modules into a single Program (ROADMAP E11.01–E11.04).
//!
//! Loads an entry file, follows static relative `import … from "…"`, mangles
//! per-module top-level bindings to avoid collisions, rewrites import locals to
//! the exporter's mangled names, and concatenates dependency bodies before the
//! entry. Supports named, default, and namespace (`import * as ns`) export/import,
//! including cyclic graphs (live bindings via shared cells).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use draconic_ast::{
    Arg, ArrayElement, ArrayPatternElement, ArrowBody, BindingKind, BindingPattern, ClassElement,
    Expr, Ident, ObjectKey, ObjectProp, Param, Program, Stmt,
};
use draconic_diagnostics::{Diagnostic, Span};

use crate::parse;

/// Parse `entry` and all static relative imports into one linked Program.
pub fn link_entry(entry: &Path) -> Result<Program, Diagnostic> {
    let entry = normalize_path(entry)?;
    let mut loader = Loader::new();
    loader.load_graph(&entry)?;
    loader.link(&entry)
}

struct Loader {
    /// Canonical path → module id (load order).
    ids: HashMap<PathBuf, usize>,
    modules: Vec<ModuleData>,
}

struct ModuleData {
    /// Body statements after peeling import/export wrappers.
    body: Vec<Stmt>,
    /// export_name → local name (pre-mangle) in this module.
    exports: HashMap<String, String>,
    /// import local → (resolved module path, exported name).
    imports: Vec<ImportBind>,
    /// `import * as local` → resolved module path.
    namespaces: Vec<NamespaceBind>,
}

struct ImportBind {
    local: String,
    from: PathBuf,
    imported: String,
}

#[derive(Clone)]
struct NamespaceBind {
    local: String,
    from: PathBuf,
}

impl Loader {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            modules: Vec::new(),
        }
    }

    fn load_graph(&mut self, entry: &Path) -> Result<(), Diagnostic> {
        let mut stack = Vec::new();
        self.load_module(entry, &mut stack)
    }

    fn load_module(&mut self, path: &Path, stack: &mut Vec<PathBuf>) -> Result<(), Diagnostic> {
        let path = normalize_path(path)?;
        if self.ids.contains_key(&path) {
            return Ok(());
        }
        // Back-edge into an in-progress module: allow cycles. The ancestor stays
        // on the stack, finishes after its deps, and registers once.
        if stack.iter().any(|p| p == &path) {
            return Ok(());
        }
        stack.push(path.clone());

        let source = fs::read_to_string(&path).map_err(|e| {
            Diagnostic::new(
                format!("failed to read module {}: {e}", path.display()),
                Span::dummy(),
            )
        })?;
        let program = parse(&source)?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));

        let mut body = Vec::new();
        let mut exports: HashMap<String, String> = HashMap::new();
        let mut imports: Vec<ImportBind> = Vec::new();
        let mut namespaces: Vec<NamespaceBind> = Vec::new();
        let mut dep_paths = Vec::new();

        for stmt in program.body {
            match stmt {
                Stmt::ImportDeclaration {
                    specifiers,
                    namespace,
                    source,
                    ..
                } => {
                    let spec = source.value.to_string_strict().ok_or_else(|| {
                        Diagnostic::new(
                            "module specifier must be a well-formed string".to_string(),
                            source.span,
                        )
                    })?;
                    let dep = resolve_specifier(parent, &spec, source.span)?;
                    dep_paths.push(dep.clone());
                    for s in specifiers {
                        imports.push(ImportBind {
                            local: s.local.name,
                            from: dep.clone(),
                            imported: s.imported.name,
                        });
                    }
                    if let Some(ns) = namespace {
                        namespaces.push(NamespaceBind {
                            local: ns.name,
                            from: dep.clone(),
                        });
                    }
                }
                Stmt::ExportNamedDeclaration {
                    declaration,
                    specifiers,
                    ..
                } => {
                    if let Some(decl) = declaration {
                        collect_decl_exports(&decl, &mut exports)?;
                        body.push(*decl);
                    }
                    for s in specifiers {
                        if exports
                            .insert(s.exported.name.clone(), s.local.name.clone())
                            .is_some()
                        {
                            return Err(Diagnostic::new(
                                format!("duplicate export `{}`", s.exported.name),
                                s.exported.span,
                            ));
                        }
                    }
                }
                Stmt::ExportDefaultDeclaration {
                    declaration,
                    local,
                    ..
                } => {
                    if exports
                        .insert("default".into(), local.name.clone())
                        .is_some()
                    {
                        return Err(Diagnostic::new(
                            "duplicate default export".to_string(),
                            local.span,
                        ));
                    }
                    body.push(*declaration);
                }
                other => body.push(other),
            }
        }

        for dep in &dep_paths {
            self.load_module(dep, stack)?;
        }

        let id = self.modules.len();
        self.ids.insert(path.clone(), id);
        self.modules.push(ModuleData {
            body,
            exports,
            imports,
            namespaces,
        });
        stack.pop();
        Ok(())
    }

    fn link(&mut self, entry: &Path) -> Result<Program, Diagnostic> {
        let entry = normalize_path(entry)?;
        let entry_id = *self.ids.get(&entry).expect("entry loaded");

        // Mangle non-entry modules fully. Entry keeps original local names so
        // host checks (js.check) and scripts see the source binding names.
        let mut mangled: Vec<HashMap<String, String>> = Vec::with_capacity(self.modules.len());
        for (id, module) in self.modules.iter().enumerate() {
            let mut map = HashMap::new();
            if id != entry_id {
                for name in top_level_names(&module.body) {
                    map.insert(name.clone(), format!("__m{id}_{name}"));
                }
                for ns in &module.namespaces {
                    map.insert(ns.local.clone(), format!("__m{id}_{}", ns.local));
                }
            }
            mangled.push(map);
        }

        let mut import_renames: Vec<HashMap<String, String>> =
            vec![HashMap::new(); self.modules.len()];
        for (id, module) in self.modules.iter().enumerate() {
            for bind in &module.imports {
                let from_id = *self.ids.get(&bind.from).ok_or_else(|| {
                    Diagnostic::new(
                        format!("module not loaded: {}", bind.from.display()),
                        Span::dummy(),
                    )
                })?;
                let local_in_exporter = self.modules[from_id]
                    .exports
                    .get(&bind.imported)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            format!(
                                "module {} has no export `{}`",
                                bind.from.display(),
                                bind.imported
                            ),
                            Span::dummy(),
                        )
                    })?;
                let remote = final_local_name(&mangled[from_id], local_in_exporter).ok_or_else(
                    || {
                        Diagnostic::new(
                            format!(
                                "export `{}` local `{}` missing in {}",
                                bind.imported,
                                local_in_exporter,
                                bind.from.display()
                            ),
                            Span::dummy(),
                        )
                    },
                )?;
                if let Some(prev) = import_renames[id].get(&bind.local) {
                    if prev != &remote {
                        return Err(Diagnostic::new(
                            format!("duplicate import binding `{}`", bind.local),
                            Span::dummy(),
                        ));
                    }
                }
                import_renames[id].insert(bind.local.clone(), remote);
            }
        }

        // Inject namespace object bindings before rename (object values use final remote names).
        // Unique synthetic spans: binder/IR key symbols and resolutions by Span.
        let mut span_gen = SyntheticSpans::new();
        for id in 0..self.modules.len() {
            let namespaces = self.modules[id].namespaces.clone();
            let mut ns_stmts = Vec::new();
            for bind in &namespaces {
                let from_id = *self.ids.get(&bind.from).ok_or_else(|| {
                    Diagnostic::new(
                        format!("module not loaded: {}", bind.from.display()),
                        Span::dummy(),
                    )
                })?;
                let props = namespace_object_props(
                    &self.modules[from_id].exports,
                    &mangled[from_id],
                    &mut span_gen,
                )?;
                let bind_span = span_gen.next();
                let obj_span = span_gen.next();
                ns_stmts.push(Stmt::Let {
                    kind: BindingKind::Let,
                    binding: BindingPattern::Ident(Ident {
                        name: bind.local.clone(),
                        span: bind_span,
                    }),
                    init: Some(Expr::ObjectExpression {
                        properties: props,
                        span: obj_span,
                    }),
                    span: bind_span,
                });
            }
            if !ns_stmts.is_empty() {
                let body = &mut self.modules[id].body;
                let mut combined = ns_stmts;
                combined.append(body);
                *body = combined;
            }
        }

        // modules are stored in post-order (deps before importers). Entry last.
        let mut order: Vec<usize> = (0..self.modules.len()).collect();
        order.retain(|&id| id != entry_id);
        order.push(entry_id);

        let mut linked_body = Vec::new();
        let mut start = 0u32;
        let mut end = 0u32;
        for id in order {
            let mut rename = mangled[id].clone();
            rename.extend(import_renames[id].clone());
            let mut body = std::mem::take(&mut self.modules[id].body);
            for stmt in &mut body {
                rename_stmt(stmt, &rename, &mut ScopeStack::new());
                // Per-file source offsets collide across modules; binder/IR key by Span.
                uniqueify_stmt_spans(stmt, &mut span_gen);
            }
            for stmt in &body {
                let sp = stmt_span_approx(stmt);
                if linked_body.is_empty() {
                    start = sp.start.0;
                }
                end = sp.end.0;
            }
            linked_body.extend(body);
        }

        Ok(Program {
            body: linked_body,
            span: Span::new(start, end),
        })
    }
}

/// Resolve a local name through the module's mangling map (identity if unmangled).
fn final_local_name(mangled: &HashMap<String, String>, local: &str) -> Option<String> {
    if let Some(m) = mangled.get(local) {
        return Some(m.clone());
    }
    // Entry module: locals keep source names (not present in mangled).
    Some(local.to_string())
}

/// Fresh spans outside typical source ranges so binder span→symbol maps stay unique.
struct SyntheticSpans {
    next: u32,
}

impl SyntheticSpans {
    fn new() -> Self {
        // High half of u32 avoids colliding with real UTF-8 source offsets in fixtures.
        Self {
            next: 0x8000_0000,
        }
    }

    fn next(&mut self) -> Span {
        let start = self.next;
        self.next = self.next.saturating_add(2);
        Span::new(start, start + 1)
    }
}

/// Assign fresh unique spans across a linked module body so multi-file programs
/// do not share binder/IR span keys (source offsets are per-file).
fn uniqueify_stmt_spans(stmt: &mut Stmt, spans: &mut SyntheticSpans) {
    match stmt {
        Stmt::Expression { expr, span } => {
            *span = spans.next();
            uniqueify_expr_spans(expr, spans);
        }
        Stmt::Let {
            binding,
            init,
            span,
            ..
        } => {
            *span = spans.next();
            uniqueify_binding_spans(binding, spans);
            if let Some(init) = init {
                uniqueify_expr_spans(init, spans);
            }
        }
        Stmt::Empty { span } => *span = spans.next(),
        Stmt::Block { body, span } => {
            *span = spans.next();
            for s in body {
                uniqueify_stmt_spans(s, spans);
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            span,
        } => {
            *span = spans.next();
            uniqueify_expr_spans(test, spans);
            uniqueify_stmt_spans(consequent, spans);
            if let Some(alt) = alternate {
                uniqueify_stmt_spans(alt, spans);
            }
        }
        Stmt::While { test, body, span } | Stmt::DoWhile { body, test, span } => {
            *span = spans.next();
            uniqueify_expr_spans(test, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            span,
        } => {
            *span = spans.next();
            if let Some(init) = init {
                uniqueify_stmt_spans(init, spans);
            }
            if let Some(test) = test {
                uniqueify_expr_spans(test, spans);
            }
            if let Some(update) = update {
                uniqueify_expr_spans(update, spans);
            }
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::ForIn {
            left,
            right,
            body,
            span,
        }
        | Stmt::ForOf {
            left,
            right,
            body,
            span,
        } => {
            *span = spans.next();
            uniqueify_stmt_spans(left, spans);
            uniqueify_expr_spans(right, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::Break { label, span } | Stmt::Continue { label, span } => {
            *span = spans.next();
            if let Some(label) = label {
                label.span = spans.next();
            }
        }
        Stmt::Labeled { label, body, span } => {
            *span = spans.next();
            label.span = spans.next();
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::Switch {
            discriminant,
            cases,
            span,
        } => {
            *span = spans.next();
            uniqueify_expr_spans(discriminant, spans);
            for case in cases {
                case.span = spans.next();
                if let Some(test) = &mut case.test {
                    uniqueify_expr_spans(test, spans);
                }
                for s in &mut case.body {
                    uniqueify_stmt_spans(s, spans);
                }
            }
        }
        Stmt::FunctionDeclaration {
            name,
            params,
            body,
            span,
            ..
        } => {
            *span = spans.next();
            name.span = spans.next();
            uniqueify_params_spans(params, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::ClassDeclaration {
            name,
            super_class,
            body,
            span,
        } => {
            *span = spans.next();
            name.span = spans.next();
            if let Some(sc) = super_class {
                uniqueify_expr_spans(sc, spans);
            }
            for el in body {
                match el {
                    ClassElement::Constructor { params, body, span } => {
                        *span = spans.next();
                        uniqueify_params_spans(params, spans);
                        uniqueify_stmt_spans(body, spans);
                    }
                    ClassElement::Method {
                        name,
                        params,
                        body,
                        span,
                        ..
                    } => {
                        *span = spans.next();
                        name.span = spans.next();
                        uniqueify_params_spans(params, spans);
                        uniqueify_stmt_spans(body, spans);
                    }
                }
            }
        }
        Stmt::Return { argument, span } => {
            *span = spans.next();
            if let Some(arg) = argument {
                uniqueify_expr_spans(arg, spans);
            }
        }
        Stmt::Throw { argument, span } => {
            *span = spans.next();
            uniqueify_expr_spans(argument, spans);
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            span,
        } => {
            *span = spans.next();
            uniqueify_stmt_spans(block, spans);
            if let Some(param) = handler_param {
                param.span = spans.next();
            }
            if let Some(handler) = handler {
                uniqueify_stmt_spans(handler, spans);
            }
            if let Some(finalizer) = finalizer {
                uniqueify_stmt_spans(finalizer, spans);
            }
        }
        Stmt::With {
            object,
            body,
            span,
        } => {
            *span = spans.next();
            uniqueify_expr_spans(object, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::ImportDeclaration { span, .. }
        | Stmt::ExportNamedDeclaration { span, .. }
        | Stmt::ExportDefaultDeclaration { span, .. } => {
            *span = spans.next();
        }
    }
}

fn uniqueify_binding_spans(pat: &mut BindingPattern, spans: &mut SyntheticSpans) {
    match pat {
        BindingPattern::Ident(id) => id.span = spans.next(),
        BindingPattern::Array { elements, span } => {
            *span = spans.next();
            for el in elements {
                match el {
                    ArrayPatternElement::Pattern(p) => uniqueify_binding_spans(p, spans),
                    ArrayPatternElement::Rest(id) => id.span = spans.next(),
                }
            }
        }
    }
}

fn uniqueify_params_spans(params: &mut [Param], spans: &mut SyntheticSpans) {
    for p in params {
        p.name.span = spans.next();
        if let Some(default) = &mut p.default {
            uniqueify_expr_spans(default, spans);
        }
    }
}

fn uniqueify_expr_spans(expr: &mut Expr, spans: &mut SyntheticSpans) {
    match expr {
        Expr::Ident(id) => id.span = spans.next(),
        Expr::Number(n) => n.span = spans.next(),
        Expr::BigInt(n) => n.span = spans.next(),
        Expr::String(s) => s.span = spans.next(),
        Expr::Boolean { span, .. }
        | Expr::Null { span }
        | Expr::This { span }
        | Expr::Super { span } => *span = spans.next(),
        Expr::TemplateLiteral {
            quasis,
            expressions,
            span,
        } => {
            *span = spans.next();
            for q in quasis {
                q.span = spans.next();
            }
            for e in expressions {
                uniqueify_expr_spans(e, spans);
            }
        }
        Expr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            span,
        } => {
            *span = spans.next();
            uniqueify_expr_spans(tag, spans);
            for q in quasis {
                q.span = spans.next();
            }
            for e in expressions {
                uniqueify_expr_spans(e, spans);
            }
        }
        Expr::Unary { arg, span, .. } | Expr::Update { arg, span, .. } | Expr::Paren { expr: arg, span } => {
            *span = spans.next();
            uniqueify_expr_spans(arg, spans);
        }
        Expr::Binary {
            left,
            right,
            span,
            ..
        } => {
            *span = spans.next();
            uniqueify_expr_spans(left, spans);
            uniqueify_expr_spans(right, spans);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            span,
        } => {
            *span = spans.next();
            uniqueify_expr_spans(test, spans);
            uniqueify_expr_spans(consequent, spans);
            uniqueify_expr_spans(alternate, spans);
        }
        Expr::Assign {
            target,
            value,
            span,
            ..
        } => {
            *span = spans.next();
            uniqueify_expr_spans(target, spans);
            uniqueify_expr_spans(value, spans);
        }
        Expr::Call {
            callee,
            args,
            span,
        }
        | Expr::New {
            callee,
            args,
            span,
        } => {
            *span = spans.next();
            uniqueify_expr_spans(callee, spans);
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => uniqueify_expr_spans(e, spans),
                }
            }
        }
        Expr::FunctionExpression {
            name,
            params,
            body,
            span,
            ..
        } => {
            *span = spans.next();
            if let Some(name) = name {
                name.span = spans.next();
            }
            uniqueify_params_spans(params, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Expr::ArrowFunction {
            params,
            body,
            span,
            ..
        } => {
            *span = spans.next();
            uniqueify_params_spans(params, spans);
            match body {
                ArrowBody::Expr(e) => uniqueify_expr_spans(e, spans),
                ArrowBody::Block(b) => uniqueify_stmt_spans(b, spans),
            }
        }
        Expr::ObjectExpression { properties, span } => {
            *span = spans.next();
            for p in properties {
                p.span = spans.next();
                match &mut p.key {
                    ObjectKey::Ident(id) => id.span = spans.next(),
                    ObjectKey::String(s) => s.span = spans.next(),
                    ObjectKey::Computed(e) => uniqueify_expr_spans(e, spans),
                }
                uniqueify_expr_spans(&mut p.value, spans);
            }
        }
        Expr::ArrayExpression { elements, span } => {
            *span = spans.next();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        uniqueify_expr_spans(e, spans)
                    }
                }
            }
        }
        Expr::ArrayPattern { elements, span } => {
            *span = spans.next();
            for el in elements {
                match el {
                    ArrayPatternElement::Pattern(p) => uniqueify_binding_spans(p, spans),
                    ArrayPatternElement::Rest(id) => id.span = spans.next(),
                }
            }
        }
        Expr::MemberExpression {
            object,
            property,
            span,
            ..
        } => {
            *span = spans.next();
            uniqueify_expr_spans(object, spans);
            uniqueify_expr_spans(property, spans);
        }
    }
}

fn namespace_object_props(
    exports: &HashMap<String, String>,
    mangled: &HashMap<String, String>,
    spans: &mut SyntheticSpans,
) -> Result<Vec<ObjectProp>, Diagnostic> {
    let mut names: Vec<_> = exports.keys().cloned().collect();
    names.sort();
    let mut props = Vec::with_capacity(names.len());
    for export_name in names {
        let local_in_exporter = exports.get(&export_name).expect("key from map");
        let remote = final_local_name(mangled, local_in_exporter).ok_or_else(|| {
            Diagnostic::new(
                format!("namespace export `{export_name}` local missing"),
                Span::dummy(),
            )
        })?;
        let key_span = spans.next();
        let val_span = spans.next();
        let prop_span = spans.next();
        let key = Ident {
            name: export_name,
            span: key_span,
        };
        let value = Expr::Ident(Ident {
            name: remote,
            span: val_span,
        });
        props.push(ObjectProp {
            key: ObjectKey::Ident(key),
            value,
            shorthand: false,
            span: prop_span,
        });
    }
    Ok(props)
}

fn collect_decl_exports(
    decl: &Stmt,
    exports: &mut HashMap<String, String>,
) -> Result<(), Diagnostic> {
    match decl {
        Stmt::Let { binding, .. } => {
            let mut err = None;
            binding.for_each_ident(&mut |id| {
                if err.is_some() {
                    return;
                }
                if exports.insert(id.name.clone(), id.name.clone()).is_some() {
                    err = Some(Diagnostic::new(
                        format!("duplicate export `{}`", id.name),
                        id.span,
                    ));
                }
            });
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
        Stmt::FunctionDeclaration { name, .. } | Stmt::ClassDeclaration { name, .. } => {
            if exports
                .insert(name.name.clone(), name.name.clone())
                .is_some()
            {
                return Err(Diagnostic::new(
                    format!("duplicate export `{}`", name.name),
                    name.span,
                ));
            }
            Ok(())
        }
        _ => Err(Diagnostic::new(
            "unsupported export declaration".to_string(),
            Span::dummy(),
        )),
    }
}

fn top_level_names(body: &[Stmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    for stmt in body {
        match stmt {
            Stmt::Let { binding, .. } => {
                binding.for_each_ident(&mut |id| {
                    names.insert(id.name.clone());
                });
            }
            Stmt::FunctionDeclaration { name, .. } | Stmt::ClassDeclaration { name, .. } => {
                names.insert(name.name.clone());
            }
            _ => {}
        }
    }
    names
}

/// Nested scopes for rename: frame 0 is module top-level; deeper frames shadow.
struct ScopeStack {
    frames: Vec<HashSet<String>>,
}

impl ScopeStack {
    fn new() -> Self {
        Self {
            frames: vec![HashSet::new()],
        }
    }

    fn push(&mut self) {
        self.frames.push(HashSet::new());
    }

    fn pop(&mut self) {
        self.frames.pop();
    }

    fn declare_nested(&mut self, name: &str) {
        self.frames
            .last_mut()
            .expect("scope")
            .insert(name.to_string());
    }

    fn depth(&self) -> usize {
        self.frames.len()
    }

    fn is_shadowed(&self, name: &str) -> bool {
        for frame in self.frames.iter().skip(1).rev() {
            if frame.contains(name) {
                return true;
            }
        }
        false
    }
}

fn rename_ident(id: &mut Ident, renames: &HashMap<String, String>, scopes: &ScopeStack) {
    if scopes.is_shadowed(&id.name) {
        return;
    }
    if let Some(new_name) = renames.get(&id.name) {
        id.name = new_name.clone();
    }
}

fn rename_binding_decl(
    pat: &mut BindingPattern,
    renames: &HashMap<String, String>,
    scopes: &mut ScopeStack,
) {
    match pat {
        BindingPattern::Ident(id) => {
            if scopes.depth() == 1 {
                rename_ident(id, renames, scopes);
            } else {
                scopes.declare_nested(&id.name);
            }
        }
        BindingPattern::Array { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Pattern(p) => rename_binding_decl(p, renames, scopes),
                    ArrayPatternElement::Rest(id) => {
                        if scopes.depth() == 1 {
                            rename_ident(id, renames, scopes);
                        } else {
                            scopes.declare_nested(&id.name);
                        }
                    }
                }
            }
        }
    }
}

fn rename_stmt(stmt: &mut Stmt, renames: &HashMap<String, String>, scopes: &mut ScopeStack) {
    match stmt {
        Stmt::Expression { expr, .. } => rename_expr(expr, renames, scopes),
        Stmt::Let { binding, init, .. } => {
            if let Some(init) = init {
                rename_expr(init, renames, scopes);
            }
            rename_binding_decl(binding, renames, scopes);
        }
        Stmt::Empty { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::ImportDeclaration { .. }
        | Stmt::ExportNamedDeclaration { .. }
        | Stmt::ExportDefaultDeclaration { .. } => {}
        Stmt::Block { body, .. } => {
            scopes.push();
            // declare nested first for TDZ-ish list, then rename bodies
            for s in body.iter_mut() {
                predeclare_nested(s, scopes);
            }
            for s in body.iter_mut() {
                rename_stmt(s, renames, scopes);
            }
            scopes.pop();
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            rename_expr(test, renames, scopes);
            rename_stmt(consequent, renames, scopes);
            if let Some(alt) = alternate {
                rename_stmt(alt, renames, scopes);
            }
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { body, test, .. } => {
            rename_expr(test, renames, scopes);
            rename_stmt(body, renames, scopes);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            scopes.push();
            if let Some(init) = init {
                rename_stmt(init, renames, scopes);
            }
            if let Some(test) = test {
                rename_expr(test, renames, scopes);
            }
            if let Some(update) = update {
                rename_expr(update, renames, scopes);
            }
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Stmt::ForIn {
            left, right, body, ..
        }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            scopes.push();
            rename_stmt(left, renames, scopes);
            rename_expr(right, renames, scopes);
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Stmt::Labeled { body, .. } => rename_stmt(body, renames, scopes),
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            rename_expr(discriminant, renames, scopes);
            scopes.push();
            for case in cases.iter_mut() {
                if let Some(test) = &mut case.test {
                    rename_expr(test, renames, scopes);
                }
                for s in case.body.iter_mut() {
                    predeclare_nested(s, scopes);
                }
            }
            for case in cases.iter_mut() {
                for s in case.body.iter_mut() {
                    rename_stmt(s, renames, scopes);
                }
            }
            scopes.pop();
        }
        Stmt::FunctionDeclaration {
            name, params, body, ..
        } => {
            if scopes.depth() == 1 {
                rename_ident(name, renames, scopes);
            } else {
                scopes.declare_nested(&name.name);
            }
            scopes.push();
            rename_params(params, renames, scopes);
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Stmt::ClassDeclaration {
            name,
            super_class,
            body,
            ..
        } => {
            if scopes.depth() == 1 {
                rename_ident(name, renames, scopes);
            } else {
                scopes.declare_nested(&name.name);
            }
            if let Some(sc) = super_class {
                rename_expr(sc, renames, scopes);
            }
            for el in body.iter_mut() {
                match el {
                    ClassElement::Constructor { params, body, .. }
                    | ClassElement::Method { params, body, .. } => {
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                }
            }
        }
        Stmt::Return { argument, .. } => {
            if let Some(arg) = argument {
                rename_expr(arg, renames, scopes);
            }
        }
        Stmt::Throw { argument, .. } => rename_expr(argument, renames, scopes),
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            ..
        } => {
            rename_stmt(block, renames, scopes);
            if let Some(handler) = handler {
                scopes.push();
                if let Some(param) = handler_param {
                    scopes.declare_nested(&param.name);
                }
                rename_stmt(handler, renames, scopes);
                scopes.pop();
            }
            if let Some(finalizer) = finalizer {
                rename_stmt(finalizer, renames, scopes);
            }
        }
        Stmt::With { object, body, .. } => {
            rename_expr(object, renames, scopes);
            rename_stmt(body, renames, scopes);
        }
    }
}

fn predeclare_nested(stmt: &Stmt, scopes: &mut ScopeStack) {
    if scopes.depth() == 1 {
        return;
    }
    match stmt {
        Stmt::Let { binding, .. } => {
            binding.for_each_ident(&mut |id| scopes.declare_nested(&id.name));
        }
        Stmt::FunctionDeclaration { name, .. } | Stmt::ClassDeclaration { name, .. } => {
            scopes.declare_nested(&name.name);
        }
        _ => {}
    }
}

fn rename_params(params: &mut [Param], renames: &HashMap<String, String>, scopes: &mut ScopeStack) {
    for p in params.iter_mut() {
        scopes.declare_nested(&p.name.name);
        if let Some(default) = &mut p.default {
            rename_expr(default, renames, scopes);
        }
    }
}

fn rename_expr(expr: &mut Expr, renames: &HashMap<String, String>, scopes: &mut ScopeStack) {
    match expr {
        Expr::Ident(id) => rename_ident(id, renames, scopes),
        Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::String(_)
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::Super { .. } => {}
        Expr::TemplateLiteral { expressions, .. } => {
            for e in expressions {
                rename_expr(e, renames, scopes);
            }
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            rename_expr(tag, renames, scopes);
            for e in expressions {
                rename_expr(e, renames, scopes);
            }
        }
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } | Expr::Paren { expr: arg, .. } => {
            rename_expr(arg, renames, scopes)
        }
        Expr::Binary { left, right, .. } => {
            rename_expr(left, renames, scopes);
            rename_expr(right, renames, scopes);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            rename_expr(test, renames, scopes);
            rename_expr(consequent, renames, scopes);
            rename_expr(alternate, renames, scopes);
        }
        Expr::Assign { target, value, .. } => {
            rename_expr(target, renames, scopes);
            rename_expr(value, renames, scopes);
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            rename_expr(callee, renames, scopes);
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => rename_expr(e, renames, scopes),
                }
            }
        }
        Expr::FunctionExpression {
            name, params, body, ..
        } => {
            scopes.push();
            if let Some(name) = name {
                scopes.declare_nested(&name.name);
            }
            rename_params(params, renames, scopes);
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Expr::ArrowFunction { params, body, .. } => {
            scopes.push();
            rename_params(params, renames, scopes);
            match body {
                ArrowBody::Expr(e) => rename_expr(e, renames, scopes),
                ArrowBody::Block(b) => rename_stmt(b, renames, scopes),
            }
            scopes.pop();
        }
        Expr::ObjectExpression { properties, .. } => {
            for p in properties {
                let shorthand = p.shorthand;
                match &mut p.key {
                    ObjectKey::Computed(e) => rename_expr(e, renames, scopes),
                    ObjectKey::Ident(id) if shorthand => rename_ident(id, renames, scopes),
                    ObjectKey::Ident(_) | ObjectKey::String(_) => {}
                }
                rename_expr(&mut p.value, renames, scopes);
            }
        }
        Expr::ArrayExpression { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        rename_expr(e, renames, scopes)
                    }
                }
            }
        }
        Expr::ArrayPattern { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Pattern(p) => {
                        rename_binding_pattern_use(p, renames, scopes);
                    }
                    ArrayPatternElement::Rest(id) => rename_ident(id, renames, scopes),
                }
            }
        }
        Expr::MemberExpression {
            object,
            property,
            computed,
            ..
        } => {
            rename_expr(object, renames, scopes);
            if *computed {
                rename_expr(property, renames, scopes);
            }
            // Non-computed property name is not a variable reference.
        }
    }
}

fn rename_binding_pattern_use(
    pat: &mut BindingPattern,
    renames: &HashMap<String, String>,
    scopes: &ScopeStack,
) {
    match pat {
        BindingPattern::Ident(id) => rename_ident(id, renames, scopes),
        BindingPattern::Array { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Pattern(p) => {
                        rename_binding_pattern_use(p, renames, scopes)
                    }
                    ArrayPatternElement::Rest(id) => rename_ident(id, renames, scopes),
                }
            }
        }
    }
}

fn stmt_span_approx(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Expression { span, .. }
        | Stmt::Let { span, .. }
        | Stmt::Empty { span }
        | Stmt::Block { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoWhile { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForIn { span, .. }
        | Stmt::ForOf { span, .. }
        | Stmt::Break { span, .. }
        | Stmt::Continue { span, .. }
        | Stmt::Labeled { span, .. }
        | Stmt::Switch { span, .. }
        | Stmt::FunctionDeclaration { span, .. }
        | Stmt::ClassDeclaration { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Throw { span, .. }
        | Stmt::Try { span, .. }
        | Stmt::With { span, .. }
        | Stmt::ImportDeclaration { span, .. }
        | Stmt::ExportNamedDeclaration { span, .. }
        | Stmt::ExportDefaultDeclaration { span, .. } => *span,
    }
}

fn normalize_path(path: &Path) -> Result<PathBuf, Diagnostic> {
    if path.exists() {
        fs::canonicalize(path).map_err(|e| {
            Diagnostic::new(
                format!("canonicalize {}: {e}", path.display()),
                Span::dummy(),
            )
        })
    } else {
        Ok(path.to_path_buf())
    }
}

fn resolve_specifier(parent: &Path, spec: &str, span: Span) -> Result<PathBuf, Diagnostic> {
    if !(spec.starts_with("./") || spec.starts_with("../")) {
        return Err(Diagnostic::new(
            format!("only relative module specifiers are supported (got `{spec}`)"),
            span,
        ));
    }
    let joined = parent.join(spec);
    if joined.exists() {
        return fs::canonicalize(&joined).map_err(|e| {
            Diagnostic::new(
                format!("canonicalize {}: {e}", joined.display()),
                span,
            )
        });
    }
    Err(Diagnostic::new(
        format!("cannot resolve module `{spec}` from {}", parent.display()),
        span,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_named_export_import() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let main = dir.join("main.drac");
        fs::write(&lib, "export let value = 41;\nexport function inc(x) { return x + 1; }\n")
            .unwrap();
        fs::write(
            &main,
            "import { value, inc } from \"./lib.drac\";\nlet a = value;\nlet b = inc(value);\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("inc") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_default_export_import() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-default-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let main = dir.join("main.drac");
        fs::write(
            &lib,
            "export default function answer() { return 42; }\nexport let tag = \"ok\";\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import answer from \"./lib.drac\";\nimport ans, { tag } from \"./lib.drac\";\nlet a = answer();\nlet b = ans();\nlet c = tag;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("answer") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_default_export_expr() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-default-expr-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let main = dir.join("main.drac");
        fs::write(&lib, "export default 41 + 1;\n").unwrap();
        fs::write(&main, "import n from \"./lib.drac\";\nlet a = n;\n").unwrap();
        let program = link_entry(&main).expect("link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("__default") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_namespace_import() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-ns-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let main = dir.join("main.drac");
        fs::write(
            &lib,
            "export let value = 41;\nexport default function answer() { return 42; }\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import * as ns from \"./lib.drac\";\nimport answer, * as ns2 from \"./lib.drac\";\nlet a = ns.value;\nlet b = ns.default();\nlet c = answer();\nlet d = ns2.value;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("ObjectExpression"), "{dump}");
        assert!(dump.contains("value") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_cyclic_named_functions() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-cycle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.drac");
        let b = dir.join("b.drac");
        let main = dir.join("main.drac");
        fs::write(
            &a,
            "import { fromB } from \"./b.drac\";\nexport function fromA(x) { return x + 1; }\nexport function callB(x) { return fromB(x); }\n",
        )
        .unwrap();
        fs::write(
            &b,
            "import { fromA } from \"./a.drac\";\nexport function fromB(x) { return fromA(x) + 1; }\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import { fromA, callB } from \"./a.drac\";\nimport { fromB } from \"./b.drac\";\nlet a = fromA(40);\nlet b = fromB(40);\nlet c = callB(40);\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("cyclic link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("fromA") || dump.contains("__m"), "{dump}");
        assert!(dump.contains("fromB") || dump.contains("__m"), "{dump}");
        assert!(dump.contains("a"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }
}
