use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::Diagnostic;
use draconic_ir::{Arg, Expr, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, GC_INIT, OBJECT_GET, OBJECT_SET, OBJECT_SET_PROTO, PRINT_F64,
    PRINT_STR,
};

use super::{
    diag, format_number_const, number_global_name, string_global_name, FieldVal, FnInfo, LocalSlot,
    MethodRet, ModuleInfo, MAX_METHOD_ARGS, UNDEF_BITS,
};
use crate::emitter::escape_llvm_string;

impl<'a> super::Emitter<'a> {
    pub(super) fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        let mut class_binding = HashMap::new();
        for (id, idx) in &info.class_of {
            class_binding.insert(*idx, *id);
        }
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            allocas: HashMap::new(),
            slot_of: HashMap::new(),
            param_allocas: HashMap::new(),
            this_ssa: None,
            active_parent_ctor: None,
            active_super_class: None,
            active_method_ret: MethodRet::Number,
            class_binding,
            str_globals: Vec::new(),
            tmp: 0,
            str_n: 0,
        }
    }

    pub(super) fn finish(self) -> String {
        self.out
    }

    pub(super) fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("%t{t}")
    }

    pub(super) fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for (id, ty) in &info.slots {
            self.slot_of.insert(*id, *ty);
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.05/N08.16.26/N08.16.36 ES classes + public/private fields via Runtime ABI)"
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
                PRINT_STR,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for (id, kind) in &info.slots {
            match kind {
                LocalSlot::Number => {
                    let g = number_global_name(*id);
                    writeln!(
                        self.out,
                        "@{g} = internal global double 0.00000000000000000e+00, align 8"
                    )
                    .ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                LocalSlot::String => {
                    let g = string_global_name(*id);
                    writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                LocalSlot::Object | LocalSlot::Undefined => {}
            }
        }
        if info
            .slots
            .iter()
            .any(|(_, k)| matches!(k, LocalSlot::Number | LocalSlot::String))
        {
            writeln!(self.out).ok();
        }

        for f in &info.functions.clone() {
            self.emit_method_fn(f)?;
        }

        for (id, kind) in &info.slots {
            if *kind != LocalSlot::Object {
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

        for id in &info.observe_locals {
            match self.slot_of.get(id).copied() {
                Some(LocalSlot::Number) => {
                    let ptr = self.number_slot_ptr(*id)?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    let bits = self.fresh();
                    writeln!(self.body, "  {bits} = bitcast double {v} to i64").ok();
                    let is_u = self.fresh();
                    writeln!(self.body, "  {is_u} = icmp eq i64 {bits}, {UNDEF_BITS}").ok();
                    let und_l = format!("print_und_{}", id.0);
                    let num_l = format!("print_num_{}", id.0);
                    let end_l = format!("print_end_{}", id.0);
                    writeln!(self.body, "  br i1 {is_u}, label %{und_l}, label %{num_l}").ok();
                    writeln!(self.body, "{und_l}:").ok();
                    self.emit_print_str_lit("undefined")?;
                    writeln!(self.body, "  br label %{end_l}").ok();
                    writeln!(self.body, "{num_l}:").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                    writeln!(self.body, "  br label %{end_l}").ok();
                    writeln!(self.body, "{end_l}:").ok();
                }
                Some(LocalSlot::String) => {
                    let ptr = self
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("es_classes: string slot missing"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                Some(LocalSlot::Undefined) => {
                    self.emit_print_str_lit("undefined")?;
                }
                _ => return Err(diag("es_classes: bad observe slot")),
            }
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

    pub(super) fn emit_method_fn(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let name = format!("m_fn_{}", f.idx);
        let mut params_s = String::from("ptr %this");
        for i in 0..MAX_METHOD_ARGS {
            write!(params_s, ", double %a{i}").ok();
        }
        let ret_ty = match f.ret {
            MethodRet::Number => "double",
            MethodRet::String => "ptr",
        };
        writeln!(self.out, "define {ret_ty} @{name}({params_s}) {{").ok();
        writeln!(self.out, "entry:").ok();

        let saved_body = std::mem::take(&mut self.body);
        let saved_this = self.this_ssa.take();
        let saved_params = std::mem::take(&mut self.param_allocas);
        let saved_allocas = std::mem::take(&mut self.allocas);
        let saved_parent = self.active_parent_ctor.take();
        let saved_super = self.active_super_class.take();
        let saved_ret = self.active_method_ret;
        self.active_method_ret = f.ret;

        self.this_ssa = Some("%this".to_string());
        self.active_parent_ctor = f.parent_ctor_fn_idx;
        self.active_super_class = f.super_class_idx;
        for (i, pid) in f.params.iter().enumerate() {
            let ptr = format!("%p{}", pid.0);
            writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
            writeln!(self.body, "  store double %a{i}, ptr {ptr}").ok();
            self.param_allocas.insert(*pid, ptr);
        }

        let mut saw_return = false;
        for stmt in &f.body {
            if matches!(stmt, Stmt::Return { .. }) {
                saw_return = true;
            }
            self.emit_method_stmt(stmt)?;
        }
        if !saw_return {
            match f.ret {
                MethodRet::Number => {
                    writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
                }
                MethodRet::String => {
                    let s = self.string_const("undefined")?;
                    writeln!(self.body, "  ret ptr {s}").ok();
                }
            }
        }

        self.out.push_str(&self.body);
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.this_ssa = saved_this;
        self.param_allocas = saved_params;
        self.allocas = saved_allocas;
        self.active_parent_ctor = saved_parent;
        self.active_super_class = saved_super;
        self.active_method_ret = saved_ret;
        Ok(())
    }

    pub(super) fn emit_method_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(e) } => {
                match self.active_method_ret {
                    MethodRet::Number => {
                        let v = self.emit_number_expr(e)?;
                        writeln!(self.body, "  ret double {v}").ok();
                    }
                    MethodRet::String => {
                        let v = self.emit_string_expr(e)?;
                        writeln!(self.body, "  ret ptr {v}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Return { value: None } => {
                match self.active_method_ret {
                    MethodRet::Number => {
                        writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
                    }
                    MethodRet::String => {
                        let s = self.string_const("undefined")?;
                        writeln!(self.body, "  ret ptr {s}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_method_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Expr {
                expr:
                    Expr::Call {
                        callee,
                        args,
                        optional,
                        ..
                    },
            } if matches!(callee.as_ref(), Expr::Super { .. }) => {
                if *optional {
                    return Err(diag("es_classes: optional super call"));
                }
                self.emit_super_call(args)?;
                Ok(())
            }
            Stmt::Expr { expr } => self.emit_side_effect_expr(expr),
            _ => Err(diag("es_classes: unsupported method stmt")),
        }
    }

    pub(super) fn emit_super_call(&mut self, args: &[Arg]) -> Result<(), Diagnostic> {
        let parent = self
            .active_parent_ctor
            .ok_or_else(|| diag("es_classes: super() outside derived ctor"))?;
        let this = self
            .this_ssa
            .clone()
            .ok_or_else(|| diag("es_classes: super() without this"))?;
        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_classes: spread super args not supported"));
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
            "  {ret} = call double ({ty_params}) @m_fn_{parent}({call_args})"
        )
        .ok();
        let _ = ret;
        Ok(())
    }

    pub(super) fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                let Some(kind) = self.slot_of.get(local).copied() else {
                    return Ok(());
                };
                match kind {
                    LocalSlot::Number => {
                        let v = if let Some(raw) = self.info.const_number.get(local) {
                            format_number_const(raw)?
                        } else {
                            self.emit_number_expr(init)?
                        };
                        let ptr = self.number_slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    LocalSlot::String => {
                        let v = self.emit_string_expr(init)?;
                        let ptr = self
                            .allocas
                            .get(local)
                            .cloned()
                            .ok_or_else(|| diag("es_classes: string alloca missing"))?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    LocalSlot::Undefined => {}
                    LocalSlot::Object => {
                        let v = if let Some(ci) = self.info.class_of.get(local) {
                            self.emit_class_ctor(*ci)?
                        } else {
                            self.emit_object_expr(init)?
                        };
                        let ptr = self.allocas.get(local).cloned().unwrap();
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr } => self.emit_side_effect_expr(expr),
            _ => Err(diag("es_classes: unsupported stmt")),
        }
    }

    pub(super) fn emit_class_ctor(&mut self, class_idx: usize) -> Result<String, Diagnostic> {
        let cls = self.info.classes[class_idx].clone();
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
        if let Some(parent_idx) = cls.parent {
            let parent_binding = *self
                .class_binding
                .get(&parent_idx)
                .ok_or_else(|| diag("es_classes: parent class binding missing"))?;
            let parent_ptr = self
                .allocas
                .get(&parent_binding)
                .cloned()
                .ok_or_else(|| diag("es_classes: parent class alloca missing"))?;
            let parent_ctor = self.fresh();
            writeln!(self.body, "  {parent_ctor} = load ptr, ptr {parent_ptr}").ok();
            let parent_proto = self.fresh();
            writeln!(
                self.body,
                "  {}",
                OBJECT_GET.call_to(&parent_proto, &format!("ptr {parent_ctor}, ptr {key}"))
            )
            .ok();
            writeln!(
                self.body,
                "  {}",
                OBJECT_SET_PROTO.call(&format!("ptr {proto}, ptr {parent_proto}"))
            )
            .ok();
        }
        for (name, fn_idx) in &cls.methods {
            let mkey = self.string_const(name)?;
            let fptr = format!("@m_fn_{fn_idx}");
            writeln!(
                self.body,
                "  {}",
                OBJECT_SET.call(&format!("ptr {proto}, ptr {mkey}, ptr {fptr}"))
            )
            .ok();
        }
        for (name, fn_idx) in &cls.static_methods {
            let mkey = self.string_const(name)?;
            let fptr = format!("@m_fn_{fn_idx}");
            writeln!(
                self.body,
                "  {}",
                OBJECT_SET.call(&format!("ptr {ctor}, ptr {mkey}, ptr {fptr}"))
            )
            .ok();
        }
        for (name, fv) in &cls.static_fields {
            self.emit_field_on_object(&ctor, name, fv)?;
        }
        Ok(ctor)
    }

    pub(super) fn emit_field_on_object(
        &mut self,
        obj: &str,
        name: &str,
        fv: &FieldVal,
    ) -> Result<(), Diagnostic> {
        match fv {
            FieldVal::Undef => Ok(()),
            FieldVal::Number(e) => {
                let key = self.string_const(name)?;
                let n = self.emit_number_expr(e)?;
                let p = self.box_number(&n)?;
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(())
            }
            FieldVal::String(s) => {
                let key = self.string_const(name)?;
                let p = self.string_const(s)?;
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(())
            }
        }
    }

    pub(super) fn emit_print_str_lit(&mut self, s: &str) -> Result<(), Diagnostic> {
        let p = self.string_const(s)?;
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {p}"))).ok();
        Ok(())
    }

    /// Pack f64 bits into a non-null ptr so `0` is distinct from missing/undefined.
    pub(super) fn box_number(&mut self, n: &str) -> Result<String, Diagnostic> {
        let bits = self.fresh();
        writeln!(self.body, "  {bits} = bitcast double {n} to i64").ok();
        let tagged = self.fresh();
        writeln!(self.body, "  {tagged} = or i64 {bits}, 1").ok();
        let p = self.fresh();
        writeln!(self.body, "  {p} = inttoptr i64 {tagged} to ptr").ok();
        Ok(p)
    }

    pub(super) fn unbox_number(&mut self, raw: &str) -> Result<String, Diagnostic> {
        let i = self.fresh();
        writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
        let bits = self.fresh();
        writeln!(self.body, "  {bits} = and i64 {i}, -2").ok();
        let d = self.fresh();
        writeln!(self.body, "  {d} = bitcast i64 {bits} to double").ok();
        Ok(d)
    }

    pub(super) fn member_key_cstr(&mut self, property: &Expr) -> Result<String, Diagnostic> {
        match property {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            _ => Err(diag("es_classes: member key must be string")),
        }
    }

    pub(super) fn number_slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        if let Some(ptr) = self.allocas.get(&id) {
            return Ok(ptr.clone());
        }
        if self.slot_of.get(&id) == Some(&LocalSlot::Number) {
            return Ok(format!("@{}", number_global_name(id)));
        }
        Err(diag("es_classes: number slot missing"))
    }

    pub(super) fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_cls_str.{}", self.str_n);
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
