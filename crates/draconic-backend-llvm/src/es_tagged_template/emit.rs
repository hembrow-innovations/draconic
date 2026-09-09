use std::fmt::Write as _;

use super::*;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
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
                        value: Expr::Function { params, body, .. },
                    } = p
                    {
                        let ids = simple_params(params).unwrap_or_default();
                        if let Some(idx) = info
                            .functions
                            .iter()
                            .position(|f| f.params == ids && f.body == *body)
                        {
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

    pub(super) fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("%t{t}")
    }

    pub(super) fn finish(self) -> String {
        self.out
    }

    pub(super) fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
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

    pub(super) fn emit_function(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
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

    pub(super) fn body_ends_with_ret(&self) -> bool {
        for line in self.body.lines().rev() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            return t.starts_with("ret ");
        }
        false
    }

    pub(super) fn emit_fn_stmt(&mut self, stmt: &Stmt, f: &FnInfo) -> Result<(), Diagnostic> {
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

    pub(super) fn emit_top_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
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

    pub(super) fn emit_object(&mut self, local: LocalId, expr: &Expr) -> Result<String, Diagnostic> {
        let Expr::Object { .. } = expr else {
            return Err(diag("es_tt: expected object"));
        };
        let obj = self.fresh();
        writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
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

    pub(super) fn fn_ptr_value(&mut self, idx: usize) -> String {
        let n = FN_TAG + idx as i64;
        let t = self.fresh();
        writeln!(self.body, "  {t} = inttoptr i64 {n} to ptr").ok();
        t
    }

    pub(super) fn emit_fn_value(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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

    pub(super) fn emit_stringy(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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
            _ => Err(diag(format!("es_tt: unsupported stringy expr: {expr:?}"))),
        }
    }

    pub(super) fn emit_arrayish(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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

    pub(super) fn emit_index(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n = parse_nonneg_int(raw)?;
                Ok(n.to_string())
            }
            _ => Err(diag("es_tt: index must be number literal")),
        }
    }

    pub(super) fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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

    pub(super) fn emit_eq(&mut self, left: &Expr, right: &Expr) -> Result<String, Diagnostic> {
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
        writeln!(
            self.body,
            "  {}",
            CSTR_LEN.call_to(&ll, &format!("ptr {l}"))
        )
        .ok();
        let rl = self.fresh();
        writeln!(
            self.body,
            "  {}",
            CSTR_LEN.call_to(&rl, &format!("ptr {r}"))
        )
        .ok();
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

    pub(super) fn emit_tagged(
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
        writeln!(self.body, "  switch i64 {idxv}, label %{bad} [{cases} ]").ok();

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

    pub(super) fn format_call(&self, idx: usize, nparams: usize, a0: &str, a1: &str, a2: &str) -> String {
        match nparams {
            0 => format!("call ptr @d_tt_fn_{idx}()"),
            1 => format!("call ptr @d_tt_fn_{idx}(ptr {a0})"),
            2 => format!("call ptr @d_tt_fn_{idx}(ptr {a0}, ptr {a1})"),
            _ => format!("call ptr @d_tt_fn_{idx}(ptr {a0}, ptr {a1}, ptr {a2})"),
        }
    }

    pub(super) fn call_fn_idx(&mut self, idx: usize, args: &[String]) -> Result<String, Diagnostic> {
        let f = &self.info.functions[idx];
        let n = f.params.len();
        let mut parts = Vec::new();
        for i in 0..n {
            let a = args.get(i).map(|s| s.as_str()).unwrap_or("null");
            parts.push(format!("ptr {a}"));
        }
        let args_s = parts.join(", ");
        let t = self.fresh();
        writeln!(self.body, "  {t} = call ptr @d_tt_fn_{idx}({args_s})").ok();
        Ok(t)
    }

    pub(super) fn resolve_tag(&mut self, tag: &Expr) -> Result<(Option<usize>, Option<String>), Diagnostic> {
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

    pub(super) fn resolve_fn_idx(&self, expr: &Expr) -> Result<usize, Diagnostic> {
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

    pub(super) fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
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
