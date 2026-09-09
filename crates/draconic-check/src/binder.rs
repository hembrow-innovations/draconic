use super::{collect_lexically_declared_names, is_simple_parameter_list, Symbol, SymbolId};
use draconic_ast::{BindingKind, BindingPattern, Param, Stmt};
use draconic_diagnostics::{Diagnostic, Span};
use std::collections::HashMap;

pub(crate) struct Binder {
    /// Scope stack (innermost last): name → symbol id.
    scopes: Vec<HashMap<String, SymbolId>>,
    /// Indices into `scopes` that are var environments (program + function).
    var_env_indices: Vec<usize>,
    /// Host globals (e.g. `Math`): resolve after lexical scopes so `let Math` can shadow.
    builtins: HashMap<String, SymbolId>,
    pub(crate) symbols: Vec<Symbol>,
    pub(crate) resolutions: HashMap<Span, SymbolId>,
    /// Nesting depth of enclosing `with` statements.
    pub(crate) with_depth: u32,
    /// Current strict-mode code (directive prologue / nested function body).
    pub(crate) strict: bool,
    /// SuperProperty allowed (class/object method or constructor). Arrows inherit; plain
    /// functions clear it. SuperCall is never allowed in arrows (E19.58).
    pub(crate) super_allowed: bool,
}

impl Binder {
    pub(crate) fn new() -> Self {
        let mut binder = Self {
            scopes: vec![HashMap::new()],
            // Program/script body is a var environment.
            var_env_indices: vec![0],
            builtins: HashMap::new(),
            symbols: Vec::new(),
            resolutions: HashMap::new(),
            with_depth: 0,
            strict: false,
            super_allowed: false,
        };
        binder.install_builtin("Math", BindingKind::Const);
        binder.install_builtin("Number", BindingKind::Const);
        binder.install_builtin("NaN", BindingKind::Const);
        binder.install_builtin("Infinity", BindingKind::Const);
        binder.install_builtin("Symbol", BindingKind::Const);
        binder.install_builtin("Promise", BindingKind::Const);
        binder.install_builtin("Proxy", BindingKind::Const);
        binder.install_builtin("Reflect", BindingKind::Const);
        // E19.73: ShadowRealm constructor (host Node --harmony-shadow-realm)
        binder.install_builtin("ShadowRealm", BindingKind::Const);
        // E15.01: global object basics
        binder.install_builtin("undefined", BindingKind::Const);
        binder.install_builtin("globalThis", BindingKind::Const);
        binder.install_builtin("Object", BindingKind::Const);
        binder.install_builtin("Function", BindingKind::Const);
        binder.install_builtin("Array", BindingKind::Const);
        binder.install_builtin("String", BindingKind::Const);
        binder.install_builtin("Boolean", BindingKind::Const);
        // E15.02: Error constructors
        binder.install_builtin("Error", BindingKind::Const);
        binder.install_builtin("TypeError", BindingKind::Const);
        binder.install_builtin("RangeError", BindingKind::Const);
        binder.install_builtin("ReferenceError", BindingKind::Const);
        binder.install_builtin("SyntaxError", BindingKind::Const);
        binder.install_builtin("URIError", BindingKind::Const);
        binder.install_builtin("EvalError", BindingKind::Const);
        binder.install_builtin("AggregateError", BindingKind::Const);
        // E15.03: global number-parsing / predicate functions
        binder.install_builtin("parseInt", BindingKind::Const);
        binder.install_builtin("parseFloat", BindingKind::Const);
        binder.install_builtin("isNaN", BindingKind::Const);
        binder.install_builtin("isFinite", BindingKind::Const);
        // E15.04: URI encode/decode
        binder.install_builtin("encodeURI", BindingKind::Const);
        binder.install_builtin("decodeURI", BindingKind::Const);
        binder.install_builtin("encodeURIComponent", BindingKind::Const);
        binder.install_builtin("decodeURIComponent", BindingKind::Const);
        // E15.05: JSON
        binder.install_builtin("JSON", BindingKind::Const);
        // E15.06: Date
        binder.install_builtin("Date", BindingKind::Const);
        // E15.07: RegExp
        binder.install_builtin("RegExp", BindingKind::Const);
        // E15.08: Map / Set
        binder.install_builtin("Map", BindingKind::Const);
        binder.install_builtin("Set", BindingKind::Const);
        // E15.09: WeakMap / WeakSet
        binder.install_builtin("WeakMap", BindingKind::Const);
        binder.install_builtin("WeakSet", BindingKind::Const);
        // E15.10: ArrayBuffer / DataView / TypedArrays
        binder.install_builtin("ArrayBuffer", BindingKind::Const);
        binder.install_builtin("DataView", BindingKind::Const);
        binder.install_builtin("Int8Array", BindingKind::Const);
        binder.install_builtin("Uint8Array", BindingKind::Const);
        binder.install_builtin("Uint8ClampedArray", BindingKind::Const);
        binder.install_builtin("Int16Array", BindingKind::Const);
        binder.install_builtin("Uint16Array", BindingKind::Const);
        binder.install_builtin("Int32Array", BindingKind::Const);
        binder.install_builtin("Uint32Array", BindingKind::Const);
        binder.install_builtin("Float32Array", BindingKind::Const);
        binder.install_builtin("Float64Array", BindingKind::Const);
        binder.install_builtin("BigInt64Array", BindingKind::Const);
        binder.install_builtin("BigUint64Array", BindingKind::Const);
        // L01.01: UTF-8 TextEncoder / TextDecoder (WHATWG Encoding; portable)
        binder.install_builtin("TextEncoder", BindingKind::Const);
        binder.install_builtin("TextDecoder", BindingKind::Const);
        // E16.01: direct eval
        binder.install_builtin("eval", BindingKind::Const);
        // E18.01: Annex B escape / unescape
        binder.install_builtin("escape", BindingKind::Const);
        binder.install_builtin("unescape", BindingKind::Const);
        // L07.01 / L07.02: stdlib flags parse + help text
        binder.install_builtin("parseFlags", BindingKind::Const);
        binder.install_builtin("flagHelp", BindingKind::Const);
        // L08.01: stdlib URL parse
        binder.install_builtin("parseUrl", BindingKind::Const);
        // L08.02: query parse/serialize
        binder.install_builtin("parseQuery", BindingKind::Const);
        binder.install_builtin("serializeQuery", BindingKind::Const);
        // L09: MIME multipart parse/serialize
        binder.install_builtin("parseMultipart", BindingKind::Const);
        binder.install_builtin("serializeMultipart", BindingKind::Const);
        // L03.01: SHA-256 digest over bytes
        binder.install_builtin("sha256", BindingKind::Const);
        // L03.02: OS CSPRNG bytes
        binder.install_builtin("randomBytes", BindingKind::Const);
        // L10.01: HMAC-SHA256 over bytes with a key
        binder.install_builtin("hmacSha256", BindingKind::Const);
        // L10.02: AES-256-GCM AEAD encrypt/decrypt
        binder.install_builtin("aeadEncrypt", BindingKind::Const);
        binder.install_builtin("aeadDecrypt", BindingKind::Const);
        // L04: gzip / zlib-deflate byte buffers
        binder.install_builtin("gzip", BindingKind::Const);
        binder.install_builtin("gunzip", BindingKind::Const);
        binder.install_builtin("deflate", BindingKind::Const);
        binder.install_builtin("inflate", BindingKind::Const);
        // L06.01: leveled logger factory
        binder.install_builtin("createLogger", BindingKind::Const);
        // L02.01 / L02.02: designed collections helpers (not Object.groupBy / Map.groupBy)
        binder.install_builtin("groupBy", BindingKind::Const);
        binder.install_builtin("chunk", BindingKind::Const);
        binder.install_builtin("Deque", BindingKind::Const);
        binder
    }

    /// Register a host global. Not placed in lexical scopes so programs may shadow it.
    fn install_builtin(&mut self, name: &str, kind: BindingKind) {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.to_string(),
            span: Span::dummy(),
            kind,
            with_depth: 0,
        });
        self.builtins.insert(name.to_string(), id);
    }

    /// E19.39: LexicallyDeclaredNames of FunctionBody must not intersect BoundNames of formals.
    pub(crate) fn check_params_body_lexical_conflict(
        &self,
        params: &[Param],
        body: &Stmt,
    ) -> Result<(), Diagnostic> {
        let mut param_names = std::collections::HashSet::new();
        for p in params {
            p.binding.for_each_ident(&mut |id| {
                param_names.insert(id.name.clone());
            });
        }
        let stmts: &[Stmt] = match body {
            Stmt::Block { body, .. } => body.as_slice(),
            _ => return Ok(()),
        };
        // FunctionBody uses TopLevelLexicallyDeclaredNames (not hoistable functions).
        let lexical = collect_lexically_declared_names(stmts.iter(), true);
        for (name, span, _) in lexical {
            if param_names.contains(&name) {
                return Err(Diagnostic::new(
                    format!("duplicate declaration of `{name}`"),
                    span,
                ));
            }
        }
        Ok(())
    }

    /// Hoistable declarations for one statement-list item (Annex B.3.2 peels labels).
    pub(crate) fn declare_list_item(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Let {
                kind: BindingKind::Var,
                binding,
                ..
            } => self.declare_var_binding(binding),
            Stmt::Let { kind, binding, .. } => self.declare_binding(binding, *kind),
            Stmt::ClassDeclaration { name, .. } => {
                self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                Ok(())
            }
            Stmt::FunctionDeclaration {
                name,
                is_async,
                is_generator,
                ..
            } => {
                // Annex B.3.2: outer var-like binding already hosts this name — do not
                // shadow with a block-local binding (IR + uses share the outer symbol).
                if let Some(existing) = self.resolve_name(&name.name) {
                    let in_current = self
                        .scopes
                        .last()
                        .is_some_and(|s| s.contains_key(&name.name));
                    if !in_current
                        && matches!(
                            self.symbols[existing.0 as usize].kind,
                            BindingKind::Function | BindingKind::Var
                        )
                    {
                        self.declare_annex_b_function_span(name);
                        return Ok(());
                    }
                }
                // `var f` then `function f` in the same list: reuse var binding.
                // Annex B / E19.24: sloppy duplicate plain FunctionDeclarations share binding.
                let scope = self.scopes.last().expect("scope stack non-empty");
                if let Some(&existing) = scope.get(&name.name) {
                    let existing_kind = self.symbols[existing.0 as usize].kind;
                    if existing_kind == BindingKind::Var
                        || (!self.strict
                            && !*is_async
                            && !*is_generator
                            && existing_kind == BindingKind::Function)
                    {
                        self.declare_annex_b_function_span(name);
                        return Ok(());
                    }
                }
                self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                Ok(())
            }
            // F06.02: `extern "C" function` binds like a function declaration (no body).
            Stmt::ExternFunctionDeclaration { name, .. } => {
                self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                Ok(())
            }
            // Annex B.3.2: `label: function f() {}` hoists `f` in this list.
            Stmt::Labeled { body, .. } => self.declare_list_item(body),
            // Annex B.3.2: block-level `function` → enclosing var-like binding.
            // E18.14: also hoist nested `var` into the current var environment.
            Stmt::Block { body, .. } => {
                for s in body {
                    self.declare_annex_b_block_functions(s)?;
                    self.hoist_vars_from_stmt(s)?;
                }
                Ok(())
            }
            // Annex B.3.4: bare `function` as if/else Statement clause.
            // Annex B.3.2: also hoist from block bodies (`if (c) { function f(){} }`).
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                self.declare_if_function_clause(consequent)?;
                if let Some(alt) = alternate {
                    self.declare_if_function_clause(alt)?;
                }
                self.declare_annex_b_block_functions(consequent)?;
                if let Some(alt) = alternate {
                    self.declare_annex_b_block_functions(alt)?;
                }
                self.hoist_vars_from_stmt(consequent)?;
                if let Some(alt) = alternate {
                    self.hoist_vars_from_stmt(alt)?;
                }
                Ok(())
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                self.declare_annex_b_block_functions(body)?;
                self.hoist_vars_from_stmt(body)
            }
            Stmt::For { init, body, .. } => {
                if let Some(init) = init {
                    self.declare_annex_b_block_functions(init)?;
                    self.hoist_vars_from_stmt(init)?;
                }
                self.declare_annex_b_block_functions(body)?;
                self.hoist_vars_from_stmt(body)
            }
            Stmt::ForIn { body, left, .. } | Stmt::ForOf { body, left, .. } => {
                self.declare_annex_b_block_functions(left)?;
                self.hoist_vars_from_stmt(left)?;
                self.declare_annex_b_block_functions(body)?;
                self.hoist_vars_from_stmt(body)
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                self.declare_annex_b_block_functions(block)?;
                self.hoist_vars_from_stmt(block)?;
                if let Some(handler) = handler {
                    self.declare_annex_b_block_functions(handler)?;
                    self.hoist_vars_from_stmt(handler)?;
                }
                if let Some(finalizer) = finalizer {
                    self.declare_annex_b_block_functions(finalizer)?;
                    self.hoist_vars_from_stmt(finalizer)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Hoist `var` declarations from a nested statement into the current var environment.
    fn hoist_vars_from_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        let mut s = stmt;
        while let Stmt::Labeled { body, .. } = s {
            s = body;
        }
        match s {
            Stmt::Let {
                kind: BindingKind::Var,
                binding,
                ..
            } => self.declare_var_binding(binding),
            Stmt::Block { body, .. } => {
                for child in body {
                    self.hoist_vars_from_stmt(child)?;
                }
                Ok(())
            }
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                self.hoist_vars_from_stmt(consequent)?;
                if let Some(alt) = alternate {
                    self.hoist_vars_from_stmt(alt)?;
                }
                Ok(())
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                self.hoist_vars_from_stmt(body)
            }
            Stmt::For { init, body, .. } => {
                if let Some(init) = init {
                    self.hoist_vars_from_stmt(init)?;
                }
                self.hoist_vars_from_stmt(body)
            }
            Stmt::ForIn { body, left, .. } | Stmt::ForOf { body, left, .. } => {
                self.hoist_vars_from_stmt(left)?;
                self.hoist_vars_from_stmt(body)
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                self.hoist_vars_from_stmt(block)?;
                if let Some(handler) = handler {
                    self.hoist_vars_from_stmt(handler)?;
                }
                if let Some(finalizer) = finalizer {
                    self.hoist_vars_from_stmt(finalizer)?;
                }
                Ok(())
            }
            // Nested function bodies have their own var environment — do not hoist out.
            Stmt::FunctionDeclaration { .. } | Stmt::ClassDeclaration { .. } => Ok(()),
            _ => Ok(()),
        }
    }

    fn declare_var_binding(&mut self, binding: &BindingPattern) -> Result<(), Diagnostic> {
        let mut err = None;
        binding.for_each_ident(&mut |id| {
            if err.is_some() {
                return;
            }
            if let Err(e) = self.declare_var(id.name.clone(), id.span) {
                err = Some(e);
            }
        });
        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Declare a function-scoped `var` in the nearest var environment.
    /// Redeclaration with `var`/`function` is allowed; creates a span alias for IR.
    fn declare_var(&mut self, name: String, span: Span) -> Result<SymbolId, Diagnostic> {
        // E19.49: strict BindingIdentifier cannot be `eval`/`arguments`.
        if self.strict && (name == "eval" || name == "arguments") {
            return Err(Diagnostic::new(
                format!("binding `{name}` is invalid in strict mode"),
                span,
            ));
        }
        let env_idx = *self
            .var_env_indices
            .last()
            .expect("var environment stack non-empty");
        if let Some(&existing) = self.scopes[env_idx].get(&name) {
            let existing_kind = self.symbols[existing.0 as usize].kind;
            match existing_kind {
                BindingKind::Var | BindingKind::Function => {
                    self.declare_var_span(&name, span);
                    return Ok(existing);
                }
                BindingKind::Let
                | BindingKind::Const
                | BindingKind::Using
                | BindingKind::AwaitUsing => {
                    return Err(Diagnostic::new(
                        format!("duplicate declaration of `{name}`"),
                        span,
                    ));
                }
            }
        }
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.clone(),
            span,
            kind: BindingKind::Var,
            with_depth: self.with_depth,
        });
        self.scopes[env_idx].insert(name, id);
        Ok(id)
    }

    /// Extra symbol keyed by declaration span for IR; uses keep the scoped binding.
    fn declare_var_span(&mut self, name: &str, span: Span) {
        if self
            .symbols
            .iter()
            .any(|s| s.span == span && s.name == name)
        {
            return;
        }
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.to_string(),
            span,
            kind: BindingKind::Var,
            with_depth: self.with_depth,
        });
    }

    /// Annex B.3.2: walk a statement and hoist nested block-level function names
    /// into the current (enclosing) scope as var-like bindings.
    fn declare_annex_b_block_functions(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        let mut s = stmt;
        while let Stmt::Labeled { body, .. } = s {
            s = body;
        }
        match s {
            Stmt::Block { body, .. } => {
                for child in body {
                    self.declare_annex_b_block_functions(child)?;
                }
                Ok(())
            }
            Stmt::FunctionDeclaration { name, .. } => self.declare_annex_b_function_name(name),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                self.declare_if_function_clause(consequent)?;
                if let Some(alt) = alternate {
                    self.declare_if_function_clause(alt)?;
                }
                self.declare_annex_b_block_functions(consequent)?;
                if let Some(alt) = alternate {
                    self.declare_annex_b_block_functions(alt)?;
                }
                Ok(())
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                self.declare_annex_b_block_functions(body)
            }
            Stmt::For { init, body, .. } => {
                if let Some(init) = init {
                    self.declare_annex_b_block_functions(init)?;
                }
                self.declare_annex_b_block_functions(body)
            }
            Stmt::ForIn { body, .. } | Stmt::ForOf { body, .. } => {
                self.declare_annex_b_block_functions(body)
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                self.declare_annex_b_block_functions(block)?;
                if let Some(handler) = handler {
                    self.declare_annex_b_block_functions(handler)?;
                }
                if let Some(finalizer) = finalizer {
                    self.declare_annex_b_block_functions(finalizer)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Annex B.3.2 / B.3.4: declare a function name in the current scope (var-like).
    fn declare_annex_b_function_name(
        &mut self,
        name: &draconic_ast::Ident,
    ) -> Result<(), Diagnostic> {
        let scope = self.scopes.last().expect("scope stack non-empty");
        if let Some(&existing) = scope.get(&name.name) {
            let existing_kind = self.symbols[existing.0 as usize].kind;
            if !matches!(existing_kind, BindingKind::Function | BindingKind::Var) {
                return Err(Diagnostic::new(
                    format!("duplicate declaration of `{}`", name.name),
                    name.span,
                ));
            }
            self.declare_annex_b_function_span(name);
            return Ok(());
        }
        self.declare(name.name.clone(), name.span, BindingKind::Function)?;
        Ok(())
    }

    /// Extra symbol keyed by declaration span for IR; uses keep the scoped binding.
    fn declare_annex_b_function_span(&mut self, name: &draconic_ast::Ident) {
        if self
            .symbols
            .iter()
            .any(|s| s.span == name.span && s.name == name.name)
        {
            return;
        }
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.name.clone(),
            span: name.span,
            kind: BindingKind::Function,
            with_depth: self.with_depth,
        });
    }

    /// Annex B.3.4: hoist `function f` / `label: function f` when it is the if/else clause.
    /// Does not walk into blocks (`if (c) { function f(){} }` is B.3.2 / B.3.3).
    fn declare_if_function_clause(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        let mut s = stmt;
        while let Stmt::Labeled { body, .. } = s {
            s = body;
        }
        let Stmt::FunctionDeclaration { name, .. } = s else {
            return Ok(());
        };
        self.declare_annex_b_function_name(name)
    }

    pub(crate) fn declare_binding(
        &mut self,
        binding: &BindingPattern,
        kind: BindingKind,
    ) -> Result<(), Diagnostic> {
        let mut err = None;
        binding.for_each_ident(&mut |id| {
            if err.is_some() {
                return;
            }
            if let Err(e) = self.declare(id.name.clone(), id.span, kind) {
                err = Some(e);
            }
        });
        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    pub(crate) fn push_scope(&mut self) {
        self.push_scope_kind(false);
    }

    /// Push a scope. `is_var_env` is true for function scopes (params + body share it).
    pub(crate) fn push_scope_kind(&mut self, is_var_env: bool) {
        self.scopes.push(HashMap::new());
        if is_var_env {
            self.var_env_indices.push(self.scopes.len() - 1);
        }
    }

    pub(crate) fn pop_scope(&mut self) {
        let idx = self.scopes.len() - 1;
        if self.var_env_indices.last().copied() == Some(idx) {
            self.var_env_indices.pop();
        }
        self.scopes.pop();
    }

    pub(crate) fn declare(
        &mut self,
        name: String,
        span: Span,
        kind: BindingKind,
    ) -> Result<SymbolId, Diagnostic> {
        // E19.49: strict BindingIdentifier cannot be `eval`/`arguments`.
        if self.strict && (name == "eval" || name == "arguments") {
            return Err(Diagnostic::new(
                format!("binding `{name}` is invalid in strict mode"),
                span,
            ));
        }
        let scope = self.scopes.last_mut().expect("scope stack non-empty");
        if let Some(&existing) = scope.get(&name) {
            // `var` then `let`/`const` in the same var environment is a conflict.
            let existing_kind = self.symbols[existing.0 as usize].kind;
            if existing_kind == BindingKind::Var
                && matches!(
                    kind,
                    BindingKind::Let
                        | BindingKind::Const
                        | BindingKind::Using
                        | BindingKind::AwaitUsing
                )
            {
                return Err(Diagnostic::new(
                    format!("duplicate declaration of `{name}`"),
                    span,
                ));
            }
            return Err(Diagnostic::new(
                format!("duplicate declaration of `{name}`"),
                span,
            ));
        }
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.clone(),
            span,
            kind,
            with_depth: self.with_depth,
        });
        scope.insert(name, id);
        Ok(id)
    }

    fn resolve_name(&self, name: &str) -> Option<SymbolId> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.get(name) {
                return Some(*id);
            }
        }
        self.builtins.get(name).copied()
    }

    /// Resolve an identifier use. Inside `with`, only bindings declared in the
    /// innermost with body (or nested deeper) become static resolutions; outer
    /// names stay unresolved so the JS backend can emit bare idents for the
    /// Object Environment chain.
    ///
    /// Free identifiers outside `with` also stay unresolved (E19.05): ECMA-262
    /// global object / unresolvable references are runtime GetValue/PutValue
    /// (ReferenceError on read; non-strict assign creates a global property;
    /// `typeof` unresolvable → `"undefined"`). IR emits `IdentName` /
    /// `AssignTarget::Name` for the JS backend.
    pub(crate) fn bind_ident_use(&mut self, id: &draconic_ast::Ident) -> Result<(), Diagnostic> {
        if let Some(sym) = self.resolve_name(&id.name) {
            let decl_depth = self.symbols[sym.0 as usize].with_depth;
            if self.with_depth == 0 || decl_depth >= self.with_depth {
                self.resolutions.insert(id.span, sym);
            }
            return Ok(());
        }
        Ok(())
    }

    pub(crate) fn bind_params(
        &mut self,
        params: &[Param],
        allow_sloppy_dups: bool,
    ) -> Result<(), Diagnostic> {
        // E19.24: strict FormalParameters / ArrowParameters cannot bind `eval` or `arguments`.
        if self.strict {
            for p in params {
                let mut err = None;
                p.binding.for_each_ident(&mut |id| {
                    if err.is_some() {
                        return;
                    }
                    if id.name == "eval" || id.name == "arguments" {
                        err = Some(Diagnostic::new(
                            format!("binding `{}` is invalid in strict mode", id.name),
                            id.span,
                        ));
                    }
                });
                if let Some(e) = err {
                    return Err(e);
                }
            }
        }
        // E17.02.04: non-strict simple FormalParameters on plain `function` may
        // repeat BoundNames (last wins). Strict, non-simple, arrows, methods,
        // async, and generators require unique names.
        let simple = is_simple_parameter_list(params);
        let allow_dups = allow_sloppy_dups && !self.strict && simple;
        if !allow_dups {
            let mut seen: HashMap<String, Span> = HashMap::new();
            for p in params {
                let mut err = None;
                p.binding.for_each_ident(&mut |id| {
                    if err.is_some() {
                        return;
                    }
                    if seen.contains_key(&id.name) {
                        err = Some(Diagnostic::new(
                            format!("duplicate declaration of `{}`", id.name),
                            id.span,
                        ));
                    } else {
                        seen.insert(id.name.clone(), id.span);
                    }
                });
                if let Some(e) = err {
                    return Err(e);
                }
            }
        }
        for p in params {
            if allow_dups {
                // Simple list: each param is a single Ident.
                if let BindingPattern::Ident(id) = &p.binding {
                    self.declare_param_allow_dup(id.name.clone(), id.span)?;
                } else {
                    self.declare_binding(&p.binding, BindingKind::Let)?;
                }
            } else {
                self.declare_binding(&p.binding, BindingKind::Let)?;
            }
        }
        Ok(())
    }

    /// Declare a formal binding, allowing a later same-name formal to replace
    /// the scope entry (E17.02.04). Each declaration span keeps its own symbol
    /// so IR can lower every parameter pattern.
    fn declare_param_allow_dup(
        &mut self,
        name: String,
        span: Span,
    ) -> Result<SymbolId, Diagnostic> {
        let scope = self.scopes.last_mut().expect("scope stack non-empty");
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.clone(),
            span,
            kind: BindingKind::Let,
            with_depth: self.with_depth,
        });
        scope.insert(name, id);
        Ok(id)
    }

    /// Implicit `arguments` binding for non-arrow functions (E18.24).
    /// Skipped when a param already shadows the name. Arrows inherit lexically.
    pub(crate) fn install_arguments_object(&mut self) -> Result<(), Diagnostic> {
        let scope = self.scopes.last_mut().expect("scope stack non-empty");
        if scope.contains_key("arguments") {
            return Ok(());
        }
        // Implicit Arguments object — not a user BindingIdentifier (ok in strict).
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: "arguments".into(),
            span: Span::dummy(),
            kind: BindingKind::Var,
            with_depth: self.with_depth,
        });
        scope.insert("arguments".into(), id);
        Ok(())
    }
}
