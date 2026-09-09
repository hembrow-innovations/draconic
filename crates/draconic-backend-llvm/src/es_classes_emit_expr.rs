use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::Diagnostic;
use draconic_ir::{Arg, AssignTarget, Expr};
use draconic_runtime::abi::{ALLOC_OBJECT, OBJECT_GET, OBJECT_SET, OBJECT_SET_PROTO};

use super::ok::is_undefined_expr;
use super::{
    diag, format_number_const, member_field_val, undef_double_const, FieldVal, SlotTy,
    MAX_METHOD_ARGS,
};

impl<'a> super::Emitter<'a> {
    pub(super) fn emit_side_effect_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object, property, ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                if is_undefined_expr(value) {
                    // Leave property missing → undefined on get.
                    let _ = (obj, key);
                    return Ok(());
                }
                if let Expr::String { value: s, .. } = value.as_ref() {
                    let p = self.string_const(&s.to_string_lossy())?;
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                    )
                    .ok();
                    return Ok(());
                }
                let n = self.emit_number_expr(value)?;
                let p = self.box_number(&n)?;
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(())
            }
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional call not supported"));
                }
                let _ = self.emit_method_call(callee, args)?;
                Ok(())
            }
            _ => {
                let _ = self.emit_number_expr(expr)?;
                Ok(())
            }
        }
    }

    pub(super) fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => self.emit_typeof(arg),
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional string member"));
                }
                // Known string static/instance fields (classify-time FieldVal::String).
                if let Some(FieldVal::String(s)) = member_field_val(
                    expr,
                    &self.info.class_of,
                    &self.info.instance_of,
                    &self.info.classes,
                ) {
                    return self.string_const(s);
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
                Ok(raw)
            }
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional call"));
                }
                self.emit_method_call_string(callee, args)
            }
            Expr::Local { id, .. } => {
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_classes: string local missing"))?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            _ => Err(diag("es_classes: unsupported string expr")),
        }
    }

    pub(super) fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        match arg {
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional typeof member"));
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
                let cmp = self.fresh();
                writeln!(self.body, "  {cmp} = icmp eq ptr {raw}, null").ok();
                let s_undef = self.string_const("undefined")?;
                let s_num = self.string_const("number")?;
                let sel = self.fresh();
                writeln!(
                    self.body,
                    "  {sel} = select i1 {cmp}, ptr {s_undef}, ptr {s_num}"
                )
                .ok();
                Ok(sel)
            }
            _ => self.string_const("undefined"),
        }
    }

    pub(super) fn emit_method_call_string(
        &mut self,
        callee: &Expr,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let Expr::Member {
            object,
            property,
            optional,
            ..
        } = callee
        else {
            return Err(diag("es_classes: string method callee"));
        };
        if *optional {
            return Err(diag("es_classes: optional string method"));
        }
        let obj = self.emit_object_expr(object)?;
        let key = self.member_key_cstr(property)?;
        let fptr_slot = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&fptr_slot, &format!("ptr {obj}, ptr {key}"))
        )
        .ok();
        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => return Err(diag("es_classes: spread args")),
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
            "  {ret} = call ptr ({ty_params}) {fptr_slot}({call_args})"
        )
        .ok();
        Ok(ret)
    }

    pub(super) fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
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
                    .ok_or_else(|| diag("es_classes: number local unknown"))?;
                if kind != SlotTy::Number {
                    return Err(diag("es_classes: expected number local"));
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
                    return Err(diag("es_classes: optional member not supported"));
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
                // null get → undefined sentinel; else unbox tagged number
                let is_null = self.fresh();
                writeln!(self.body, "  {is_null} = icmp eq ptr {raw}, null").ok();
                let und_l = format!("mget_und_{}", self.tmp);
                let num_l = format!("mget_num_{}", self.tmp + 1);
                let end_l = format!("mget_end_{}", self.tmp + 2);
                self.tmp += 3;
                let d = self.fresh();
                let slot = self.fresh();
                writeln!(self.body, "  {slot} = alloca double, align 8").ok();
                writeln!(
                    self.body,
                    "  br i1 {is_null}, label %{und_l}, label %{num_l}"
                )
                .ok();
                writeln!(self.body, "{und_l}:").ok();
                writeln!(
                    self.body,
                    "  store double {}, ptr {slot}",
                    undef_double_const()
                )
                .ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{num_l}:").ok();
                let dn = self.unbox_number(&raw)?;
                writeln!(self.body, "  store double {dn}, ptr {slot}").ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{end_l}:").ok();
                writeln!(self.body, "  {d} = load double, ptr {slot}").ok();
                Ok(d)
            }
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object, property, ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                if is_undefined_expr(value) {
                    // Leave missing.
                    let _ = (obj, key);
                    return Ok(undef_double_const());
                }
                let n = self.emit_number_expr(value)?;
                let p = self.box_number(&n)?;
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
                    _ => return Err(diag("es_classes: unsupported binary")),
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
                    return Err(diag("es_classes: optional call not supported"));
                }
                self.emit_method_call(callee, args)
            }
            _ => Err(diag("es_classes: unsupported number expr")),
        }
    }

    pub(super) fn emit_method_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Member {
            object,
            property,
            optional,
            ..
        } = callee
        else {
            return Err(diag("es_classes: method call requires member callee"));
        };
        if *optional {
            return Err(diag("es_classes: optional member call not supported"));
        }
        if matches!(object.as_ref(), Expr::Super { .. }) {
            return self.emit_super_method_call(property, args);
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
                    return Err(diag("es_classes: spread args not supported"));
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

    /// `super.m(args)` — call parent prototype method with current `this`.
    pub(super) fn emit_super_method_call(
        &mut self,
        property: &Expr,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let name = match property {
            Expr::String { value, .. } => value.to_string_lossy(),
            _ => return Err(diag("es_classes: super method name must be string key")),
        };
        let start = self
            .active_super_class
            .ok_or_else(|| diag("es_classes: super.m outside derived method"))?;
        let fn_idx = self
            .resolve_super_method(start, &name)
            .ok_or_else(|| diag(format!("es_classes: super.{name} not found on parent")))?;
        let this = self
            .this_ssa
            .clone()
            .ok_or_else(|| diag("es_classes: super.m without this"))?;

        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_classes: spread super method args not supported"));
                }
            }
        }
        while arg_vals.len() < MAX_METHOD_ARGS {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }

        let mut call_args = format!("ptr {this}");
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
            "  {ret} = call double ({ty_params}) @m_fn_{fn_idx}({call_args})"
        )
        .ok();
        Ok(ret)
    }

    pub(super) fn resolve_super_method(&self, mut class_idx: usize, name: &str) -> Option<usize> {
        loop {
            let cls = self.info.classes.get(class_idx)?;
            if let Some((_, fn_idx)) = cls.methods.iter().find(|(n, _)| n == name) {
                return Some(*fn_idx);
            }
            class_idx = cls.parent?;
        }
    }

    pub(super) fn emit_new(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_classes: new callee must be local class"));
        };
        let ci = *self
            .info
            .class_of
            .get(id)
            .ok_or_else(|| diag("es_classes: unknown class constructor"))?;
        let ctor_idx = self.info.classes[ci].ctor_fn_idx;

        let ctor = {
            let ptr = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("es_classes: class binding missing alloca"))?;
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
                    return Err(diag("es_classes: spread args not supported"));
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
            "  {ret} = call double ({ty_params}) @m_fn_{ctor_idx}({call_args})"
        )
        .ok();
        let _ = ret;
        Ok(obj)
    }

    pub(super) fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::This { .. } => self
                .this_ssa
                .clone()
                .ok_or_else(|| diag("es_classes: This outside method")),
            Expr::New { callee, args, .. } => self.emit_new(callee, args),
            Expr::Local { id, .. } => {
                if self.slot_of.get(id) == Some(&SlotTy::Object)
                    || self.info.class_of.contains_key(id)
                {
                    let ptr = self
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("es_classes: object local unknown"))?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                    return Ok(t);
                }
                Err(diag("es_classes: object local unknown"))
            }
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional member not supported"));
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
            _ => Err(diag("es_classes: unsupported object expr")),
        }
    }

}
