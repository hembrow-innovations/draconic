//! C02.01–C02.03: `makeChannel` + `channelSend` + `channelRecv`.
//!
//! Supported subset:
//! - `typeof makeChannel` / `typeof channelSend` / `typeof channelRecv` → `"function"`
//! - `makeChannel()` / `makeChannel(n)` → handle number (n>0 bounded)
//! - `channelSend(ch, number|string|bool|plain object)` → 0 success / -1 invalid / -2 full
//! - `channelRecv(ch)` → FIFO head (type from preceding sends)
//! - structured clone of plain objects; shared refs rejected at send
//! - number comparisons (`>` `!==` `===` `<`) and bool locals; object `===`/`!==`

use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, Local, LocalId, Module, ObjectProp, ObjectPropKey, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, GC_INIT, HOST_CHANNEL_MAKE, HOST_CHANNEL_RECV_BOOL,
    HOST_CHANNEL_RECV_F64, HOST_CHANNEL_RECV_OBJ, HOST_CHANNEL_RECV_STR, HOST_CHANNEL_SEND_BOOL,
    HOST_CHANNEL_SEND_F64, HOST_CHANNEL_SEND_OBJ, HOST_CHANNEL_SEND_STR, OBJECT_GET, OBJECT_SET,
    PRINT_BOOL, PRINT_F64, PRINT_STR,
};

pub(crate) fn is_host_channels_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_channels(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_channels module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, PartialEq, Eq)]
enum SlotTy {
    Number,
    Bool,
    String,
    Object(HashMap<String, SlotTy>),
}

fn is_scalar_print(ty: &SlotTy) -> bool {
    matches!(ty, SlotTy::Number | SlotTy::Bool | SlotTy::String)
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    queues: HashMap<LocalId, VecDeque<SlotTy>>,
    uses_make: bool,
    uses_send: bool,
    uses_recv: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        print_locals: Vec::new(),
        slot_of: HashMap::new(),
        queues: HashMap::new(),
        uses_make: false,
        uses_send: false,
        uses_recv: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !(ctx.uses_make || ctx.uses_send || ctx.uses_recv) || ctx.print_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        slots: ctx.slots,
        print_locals: ctx.print_locals,
    })
}

fn classify_stmt(stmt: &Stmt, ctx: &mut ClassifyCtx) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let init = init.as_ref()?;
            let ty = classify_expr(init, ctx)?;
            ctx.slots.push((*local, ty.clone()));
            ctx.slot_of.insert(*local, ty.clone());
            if is_make_channel_call(init) {
                ctx.queues.insert(*local, VecDeque::new());
            }
            if is_scalar_print(&ty) {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => {
            let _ = classify_expr(expr, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn is_make_channel_call(expr: &Expr) -> bool {
    matches!(expr, Expr::Call { callee, args, .. } if is_named_callee(callee, "makeChannel") && args.len() <= 1)
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. } if is_named_callee(callee, "makeChannel") => {
            if args.len() > 1 {
                return None;
            }
            if args.len() == 1 {
                let cap = arg_expr(&args[0])?;
                if classify_expr(cap, ctx)? != SlotTy::Number {
                    return None;
                }
            }
            ctx.uses_make = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "channelSend") => {
            if args.len() != 2 {
                return None;
            }
            let handle = arg_expr(&args[0])?;
            let value = arg_expr(&args[1])?;
            let ht = classify_expr(handle, ctx)?;
            if ht != SlotTy::Number {
                return None;
            }
            let vt = classify_expr(value, ctx)?;
            if let Expr::Local { id, .. } = handle {
                if let Some(q) = ctx.queues.get_mut(id) {
                    q.push_back(vt);
                }
            }
            ctx.uses_send = true;
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. } if is_named_callee(callee, "channelRecv") => {
            if args.len() != 1 {
                return None;
            }
            let handle = arg_expr(&args[0])?;
            let ht = classify_expr(handle, ctx)?;
            if ht != SlotTy::Number {
                return None;
            }
            ctx.uses_recv = true;
            if let Expr::Local { id, .. } = handle {
                if let Some(q) = ctx.queues.get_mut(id) {
                    return q.pop_front();
                }
            }
            None
        }
        Expr::Binary {
            op,
            left,
            right,
            ..
        } if matches!(
            op,
            BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
        ) =>
        {
            let lt = classify_expr(left, ctx)?;
            let rt = classify_expr(right, ctx)?;
            if lt == SlotTy::Number && rt == SlotTy::Number {
                Some(SlotTy::Bool)
            } else if matches!(
                op,
                BinaryOp::EqEqEq | BinaryOp::NotEqEq | BinaryOp::EqEq | BinaryOp::NotEq
            ) && matches!(lt, SlotTy::Object(_))
                && matches!(rt, SlotTy::Object(_))
            {
                Some(SlotTy::Bool)
            } else {
                None
            }
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            if is_named_ident(arg, "makeChannel") {
                ctx.uses_make = true;
                Some(SlotTy::String)
            } else if is_named_ident(arg, "channelSend") {
                ctx.uses_send = true;
                Some(SlotTy::String)
            } else if is_named_ident(arg, "channelRecv") {
                ctx.uses_recv = true;
                Some(SlotTy::String)
            } else {
                let _ = classify_expr(arg, ctx)?;
                Some(SlotTy::String)
            }
        }
        Expr::Local { id, .. } => ctx.slot_of.get(id).cloned(),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Boolean { .. } => Some(SlotTy::Bool),
        Expr::Object { properties, .. } => classify_object_lit(properties, ctx),
        Expr::Member {
            object,
            property,
            ..
        } => classify_member(object, property, ctx),
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object, property, ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let key = static_prop_key(property)?;
            let vt = classify_expr(value, ctx)?;
            match ot {
                SlotTy::Object(mut shape) => {
                    shape.insert(key, vt.clone());
                    if let Expr::Local { id, .. } = object.as_ref() {
                        ctx.slot_of.insert(*id, SlotTy::Object(shape));
                    }
                    Some(vt)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn static_prop_key(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn classify_object_lit(properties: &[ObjectProp], ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    let mut shape = HashMap::new();
    for p in properties {
        let ObjectProp::Property {
            key: ObjectPropKey::Static(k),
            value,
        } = p
        else {
            return None;
        };
        let ty = classify_expr(value, ctx)?;
        shape.insert(k.to_string_lossy(), ty);
    }
    Some(SlotTy::Object(shape))
}

fn classify_member(object: &Expr, property: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    let ot = classify_expr(object, ctx)?;
    let key = static_prop_key(property)?;
    match ot {
        SlotTy::Object(shape) => shape.get(&key).cloned(),
        _ => None,
    }
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn is_named_ident(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn arg_expr(arg: &Arg) -> Option<&Expr> {
    match arg {
        Arg::Expr(e) => Some(e),
        Arg::Spread(_) => None,
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) && c != b'"' => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    by_id: HashMap<LocalId, &'a Local>,
    slot_of: HashMap<LocalId, SlotTy>,
    body: String,
    out: String,
    next_tmp: u32,
    str_globals: HashMap<String, String>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
        let slot_of: HashMap<LocalId, SlotTy> = info.slots.iter().cloned().collect();
        Self {
            module,
            info,
            by_id,
            slot_of,
            body: String::new(),
            out: String::new(),
            next_tmp: 0,
            str_globals: HashMap::new(),
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let n = self.next_tmp;
        self.next_tmp += 1;
        format!("%t{n}")
    }

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .by_id
            .get(&id)
            .map(|l| l.name.as_str())
            .ok_or_else(|| diag("host_channels: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn intern_cstr(&mut self, s: &str) -> String {
        if let Some(g) = self.str_globals.get(s) {
            return g.clone();
        }
        let g = format!(".hc.str.{}", self.str_globals.len());
        self.str_globals.insert(s.to_string(), g.clone());
        g
    }

    fn emit_cstr_ptr(&mut self, s: &str) -> String {
        let g = self.intern_cstr(s);
        let n = s.len() + 1;
        let p = self.fresh();
        writeln!(
            self.body,
            "  {p} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0"
        )
        .ok();
        p
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(self.out, "; Draconic LLVM host_channels (C02.01–C02.03)").ok();
        let decls = vec![
            GC_INIT,
            ALLOC_OBJECT,
            OBJECT_GET,
            OBJECT_SET,
            PRINT_F64,
            PRINT_STR,
            PRINT_BOOL,
            HOST_CHANNEL_MAKE,
            HOST_CHANNEL_SEND_F64,
            HOST_CHANNEL_SEND_STR,
            HOST_CHANNEL_SEND_BOOL,
            HOST_CHANNEL_SEND_OBJ,
            HOST_CHANNEL_RECV_F64,
            HOST_CHANNEL_RECV_STR,
            HOST_CHANNEL_RECV_BOOL,
            HOST_CHANNEL_RECV_OBJ,
        ];
        self.out.push_str(&llvm_declares(&decls));
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            let ptr = self.slot_ptr(*id)?;
            let llvm_ty = match ty {
                SlotTy::Number => "double",
                SlotTy::Bool => "i8",
                SlotTy::String | SlotTy::Object(_) => "ptr",
            };
            writeln!(self.body, "  {ptr} = alloca {llvm_ty}, align 8").ok();
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, kind) in &self.info.print_locals {
            let ptr = self.slot_ptr(*id)?;
            match kind {
                SlotTy::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::Bool => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {v}"))).ok();
                }
                SlotTy::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotTy::Object(_) => {}
            }
        }

        let body = std::mem::take(&mut self.body);
        for (content, gname) in &self.str_globals {
            let n = content.len() + 1;
            let esc = escape_llvm_string(content);
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
        self.out.push_str(&body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                let kind = self
                    .slot_of
                    .get(local)
                    .cloned()
                    .ok_or_else(|| diag("host_channels: declare unknown slot"))?;
                let ptr = self.slot_ptr(*local)?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Bool => {
                        let v = self.emit_bool_expr(init)?;
                        writeln!(self.body, "  store i8 {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Object(_) => {
                        let v = self.emit_object_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => {
                if matches!(
                    expr,
                    Expr::Assign {
                        target: AssignTarget::Member { .. },
                        op: AssignOp::Eq,
                        ..
                    }
                ) {
                    self.emit_member_assign(expr)?;
                } else {
                    let _ = self.emit_number_expr(expr)?;
                }
                Ok(())
            }
            _ => Err(diag("host_channels: unsupported statement")),
        }
    }

    fn emit_handle_i32(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let h_f = self.emit_number_expr(expr)?;
        let h_i32 = self.fresh();
        writeln!(self.body, "  {h_i32} = fptosi double {h_f} to i32").ok();
        Ok(h_i32)
    }

    fn emit_make(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let cap_i32 = if args.is_empty() {
            let z = self.fresh();
            writeln!(self.body, "  {z} = add i32 0, 0").ok();
            z
        } else {
            let cap_expr = arg_expr(&args[0]).ok_or_else(|| diag("makeChannel capacity"))?;
            let cap_f = self.emit_number_expr(cap_expr)?;
            let cap_i32 = self.fresh();
            writeln!(self.body, "  {cap_i32} = fptosi double {cap_f} to i32").ok();
            cap_i32
        };
        let h_i32 = self.fresh();
        let h_f = self.fresh();
        writeln!(
            self.body,
            "  {h_i32} = call i32 @{}(i32 {cap_i32})",
            HOST_CHANNEL_MAKE.symbol
        )
        .ok();
        writeln!(self.body, "  {h_f} = sitofp i32 {h_i32} to double").ok();
        Ok(h_f)
    }

    fn emit_send(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("channelSend handle"))?;
        let value = arg_expr(&args[1]).ok_or_else(|| diag("channelSend value"))?;
        let h_i32 = self.emit_handle_i32(handle)?;
        let r_i32 = self.fresh();
        let r_f = self.fresh();
        match value {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let p = self.emit_cstr_ptr(&s);
                writeln!(
                    self.body,
                    "  {r_i32} = call i32 @{}(i32 {h_i32}, ptr {p})",
                    HOST_CHANNEL_SEND_STR.symbol
                )
                .ok();
            }
            Expr::Boolean { value, .. } => {
                let b = if *value { 1 } else { 0 };
                writeln!(
                    self.body,
                    "  {r_i32} = call i32 @{}(i32 {h_i32}, i32 {b})",
                    HOST_CHANNEL_SEND_BOOL.symbol
                )
                .ok();
            }
            Expr::Local { id, .. } if self.slot_of.get(id) == Some(&SlotTy::String) => {
                let p = self.emit_string_expr(value)?;
                writeln!(
                    self.body,
                    "  {r_i32} = call i32 @{}(i32 {h_i32}, ptr {p})",
                    HOST_CHANNEL_SEND_STR.symbol
                )
                .ok();
            }
            Expr::Local { id, .. } if self.slot_of.get(id) == Some(&SlotTy::Bool) => {
                let b = self.emit_bool_expr(value)?;
                let b_i32 = self.fresh();
                writeln!(self.body, "  {b_i32} = zext i8 {b} to i32").ok();
                writeln!(
                    self.body,
                    "  {r_i32} = call i32 @{}(i32 {h_i32}, i32 {b_i32})",
                    HOST_CHANNEL_SEND_BOOL.symbol
                )
                .ok();
            }
            _ if self.expr_is_object(value) => {
                let p = self.emit_object_expr(value)?;
                writeln!(
                    self.body,
                    "  {r_i32} = call i32 @{}(i32 {h_i32}, ptr {p})",
                    HOST_CHANNEL_SEND_OBJ.symbol
                )
                .ok();
            }
            _ => {
                let v = self.emit_number_expr(value)?;
                writeln!(
                    self.body,
                    "  {r_i32} = call i32 @{}(i32 {h_i32}, double {v})",
                    HOST_CHANNEL_SEND_F64.symbol
                )
                .ok();
            }
        }
        writeln!(self.body, "  {r_f} = sitofp i32 {r_i32} to double").ok();
        Ok(r_f)
    }

    fn emit_recv_number(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("channelRecv handle"))?;
        let h_i32 = self.emit_handle_i32(handle)?;
        let tmp = self.fresh();
        let st = self.fresh();
        let v = self.fresh();
        writeln!(self.body, "  {tmp} = alloca double, align 8").ok();
        writeln!(
            self.body,
            "  {st} = call i32 @{}(i32 {h_i32}, ptr {tmp})",
            HOST_CHANNEL_RECV_F64.symbol
        )
        .ok();
        let _ = st;
        writeln!(self.body, "  {v} = load double, ptr {tmp}").ok();
        Ok(v)
    }

    fn emit_recv_string(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("channelRecv handle"))?;
        let h_i32 = self.emit_handle_i32(handle)?;
        let tmp = self.fresh();
        let st = self.fresh();
        let v = self.fresh();
        writeln!(self.body, "  {tmp} = alloca ptr, align 8").ok();
        writeln!(
            self.body,
            "  {st} = call i32 @{}(i32 {h_i32}, ptr {tmp})",
            HOST_CHANNEL_RECV_STR.symbol
        )
        .ok();
        let _ = st;
        writeln!(self.body, "  {v} = load ptr, ptr {tmp}").ok();
        Ok(v)
    }

    fn emit_recv_bool(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("channelRecv handle"))?;
        let h_i32 = self.emit_handle_i32(handle)?;
        let tmp = self.fresh();
        let st = self.fresh();
        let i = self.fresh();
        let b = self.fresh();
        writeln!(self.body, "  {tmp} = alloca i32, align 4").ok();
        writeln!(
            self.body,
            "  {st} = call i32 @{}(i32 {h_i32}, ptr {tmp})",
            HOST_CHANNEL_RECV_BOOL.symbol
        )
        .ok();
        let _ = st;
        writeln!(self.body, "  {i} = load i32, ptr {tmp}").ok();
        writeln!(self.body, "  {b} = trunc i32 {i} to i8").ok();
        Ok(b)
    }

    fn emit_recv_obj(&mut self, args: &[Arg]) -> Result<String, Diagnostic> {
        let handle = arg_expr(&args[0]).ok_or_else(|| diag("channelRecv handle"))?;
        let h_i32 = self.emit_handle_i32(handle)?;
        let tmp = self.fresh();
        let st = self.fresh();
        let v = self.fresh();
        writeln!(self.body, "  {tmp} = alloca ptr, align 8").ok();
        writeln!(
            self.body,
            "  {st} = call i32 @{}(i32 {h_i32}, ptr {tmp})",
            HOST_CHANNEL_RECV_OBJ.symbol
        )
        .ok();
        let _ = st;
        writeln!(self.body, "  {v} = load ptr, ptr {tmp}").ok();
        Ok(v)
    }

    fn expr_is_object(&self, expr: &Expr) -> bool {
        matches!(self.expr_slot(expr), Some(SlotTy::Object(_)))
            || matches!(expr, Expr::Object { .. })
    }

    fn expr_slot(&self, expr: &Expr) -> Option<SlotTy> {
        match expr {
            Expr::Local { id, .. } => self.slot_of.get(id).cloned(),
            Expr::Member {
                object, property, ..
            } => self.member_slot(object, property),
            Expr::Number { .. } => Some(SlotTy::Number),
            Expr::String { .. } => Some(SlotTy::String),
            Expr::Boolean { .. } => Some(SlotTy::Bool),
            _ => None,
        }
    }

    fn member_slot(&self, object: &Expr, property: &Expr) -> Option<SlotTy> {
        let key = static_prop_key(property)?;
        match self.expr_slot(object)? {
            SlotTy::Object(shape) => shape.get(&key).cloned(),
            _ => None,
        }
    }

    fn emit_member_key(&mut self, property: &Expr) -> Result<String, Diagnostic> {
        let key = static_prop_key(property).ok_or_else(|| diag("host_channels: member key"))?;
        Ok(self.emit_cstr_ptr(&key))
    }

    fn emit_member_get(&mut self, object: &Expr, property: &Expr) -> Result<String, Diagnostic> {
        let obj = self.emit_object_expr(object)?;
        let key = self.emit_member_key(property)?;
        let raw = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&raw, &format!("ptr {obj}, ptr {key}"))
        )
        .ok();
        Ok(raw)
    }

    fn emit_member_assign(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        let Expr::Assign {
            target:
                AssignTarget::Member {
                    object, property, ..
                },
            value,
            ..
        } = expr
        else {
            return Err(diag("host_channels: expected member assign"));
        };
        let obj = self.emit_object_expr(object)?;
        let key = self.emit_member_key(property)?;
        let val_ptr = if self.expr_is_object(value) {
            self.emit_object_expr(value)?
        } else if matches!(self.expr_slot(value), Some(SlotTy::String))
            || matches!(value.as_ref(), Expr::String { .. })
        {
            self.emit_string_expr(value)?
        } else {
            let n = self.emit_number_expr(value)?;
            let i = self.fresh();
            writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
            let p = self.fresh();
            writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
            p
        };
        writeln!(
            self.body,
            "  {}",
            OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {val_ptr}"))
        )
        .ok();
        Ok(())
    }

    fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Object { properties, .. } => {
                let obj = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
                for p in properties {
                    let ObjectProp::Property {
                        key: ObjectPropKey::Static(k),
                        value,
                    } = p
                    else {
                        return Err(diag("host_channels: only static object props"));
                    };
                    let key = self.emit_cstr_ptr(&k.to_string_lossy());
                    let val_ptr = if self.expr_is_object(value)
                        || matches!(value, Expr::Object { .. })
                    {
                        self.emit_object_expr(value)?
                    } else if matches!(self.expr_slot(value), Some(SlotTy::String))
                        || matches!(value, Expr::String { .. })
                    {
                        self.emit_string_expr(value)?
                    } else {
                        let n = self.emit_number_expr(value)?;
                        let i = self.fresh();
                        writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                        let p = self.fresh();
                        writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                        p
                    };
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {val_ptr}"))
                    )
                    .ok();
                }
                Ok(obj)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "channelRecv") => {
                self.emit_recv_obj(args)
            }
            Expr::Member {
                object, property, ..
            } => self.emit_member_get(object, property),
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_channels: expected object expr")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let v = self.fresh();
                let n: f64 = raw.parse().unwrap_or(0.0);
                let lit = if n.fract() == 0.0 {
                    format!("{n:.1}")
                } else {
                    format!("{n}")
                };
                writeln!(self.body, "  {v} = fadd double {lit}, 0.0").ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "makeChannel") => {
                self.emit_make(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "channelSend") => {
                self.emit_send(args)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "channelRecv") => {
                self.emit_recv_number(args)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Member {
                object, property, ..
            } => {
                let raw = self.emit_member_get(object, property)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
                let v = self.fresh();
                writeln!(self.body, "  {v} = sitofp i64 {i} to double").ok();
                Ok(v)
            }
            _ => Err(diag("host_channels: expected number expr")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Boolean { value, .. } => {
                let v = self.fresh();
                let b = if *value { 1 } else { 0 };
                writeln!(self.body, "  {v} = add i8 {b}, 0").ok();
                Ok(v)
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "channelRecv") => {
                self.emit_recv_bool(args)
            }
            Expr::Binary {
                op,
                left,
                right,
                ..
            } if matches!(
                op,
                BinaryOp::Gt
                    | BinaryOp::GtEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
                    | BinaryOp::EqEq
                    | BinaryOp::NotEq
            ) =>
            {
                if self.expr_is_object(left) && self.expr_is_object(right) {
                    let l = self.emit_object_expr(left)?;
                    let r = self.emit_object_expr(right)?;
                    let pred = match op {
                        BinaryOp::EqEqEq | BinaryOp::EqEq => "eq",
                        _ => "ne",
                    };
                    let cmp = self.fresh();
                    writeln!(self.body, "  {cmp} = icmp {pred} ptr {l}, {r}").ok();
                    let b = self.fresh();
                    writeln!(self.body, "  {b} = zext i1 {cmp} to i8").ok();
                    return Ok(b);
                }
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let cmp = self.fresh();
                let pred = match op {
                    BinaryOp::Gt => "ogt",
                    BinaryOp::GtEq => "oge",
                    BinaryOp::Lt => "olt",
                    BinaryOp::LtEq => "ole",
                    BinaryOp::EqEqEq | BinaryOp::EqEq => "oeq",
                    BinaryOp::NotEqEq | BinaryOp::NotEq => "one",
                    _ => unreachable!(),
                };
                writeln!(self.body, "  {cmp} = fcmp {pred} double {l}, {r}").ok();
                let b = self.fresh();
                writeln!(self.body, "  {b} = zext i1 {cmp} to i8").ok();
                Ok(b)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load i8, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_channels: expected bool expr")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                Ok(self.emit_cstr_ptr(&s))
            }
            Expr::Call { callee, args, .. } if is_named_callee(callee, "channelRecv") => {
                self.emit_recv_string(args)
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } if is_named_ident(arg, "makeChannel")
                || is_named_ident(arg, "channelSend")
                || is_named_ident(arg, "channelRecv") =>
            {
                Ok(self.emit_cstr_ptr("function"))
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Member {
                object, property, ..
            } => self.emit_member_get(object, property),
            _ => Err(diag("host_channels: expected string expr")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn lower_src(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn classifies_typeof() {
        let m = lower_src(
            r#"
            let tMake = typeof makeChannel;
            let tSend = typeof channelSend;
            let tRecv = typeof channelRecv;
            "#,
        );
        assert!(is_host_channels_module(&m));
        let ir = emit_host_channels(&m).expect("emit");
        assert!(ir.contains("function"), "{ir}");
    }

    #[test]
    fn classifies_number_fifo() {
        let m = lower_src(
            r#"
            let ch = makeChannel();
            let sent1 = channelSend(ch, 1);
            let sent2 = channelSend(ch, 2);
            let a = channelRecv(ch);
            let b = channelRecv(ch);
            "#,
        );
        assert!(is_host_channels_module(&m));
        let ir = emit_host_channels(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_channel_make"), "{ir}");
        assert!(ir.contains("draconic_rt_host_channel_send_f64"), "{ir}");
        assert!(ir.contains("draconic_rt_host_channel_recv_f64"), "{ir}");
    }

    #[test]
    fn classifies_string_fifo() {
        let m = lower_src(
            r#"
            let ch = makeChannel();
            let sent1 = channelSend(ch, "hello");
            let sent2 = channelSend(ch, "world");
            let a = channelRecv(ch);
            let b = channelRecv(ch);
            "#,
        );
        assert!(is_host_channels_module(&m));
        let ir = emit_host_channels(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_channel_send_str"), "{ir}");
        assert!(ir.contains("draconic_rt_host_channel_recv_str"), "{ir}");
    }

    #[test]
    fn classifies_bool_fifo() {
        let m = lower_src(
            r#"
            let ch = makeChannel();
            let sent1 = channelSend(ch, true);
            let sent2 = channelSend(ch, false);
            let a = channelRecv(ch);
            let b = channelRecv(ch);
            "#,
        );
        assert!(is_host_channels_module(&m));
        let ir = emit_host_channels(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_channel_send_bool"), "{ir}");
        assert!(ir.contains("draconic_rt_host_channel_recv_bool"), "{ir}");
    }

    #[test]
    fn classifies_send_bad() {
        let m = lower_src(
            r#"
            let bad = channelSend(0, 1);
            let err = bad < 0;
            "#,
        );
        assert!(is_host_channels_module(&m));
        let ir = emit_host_channels(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_channel_send_f64"), "{ir}");
    }

    #[test]
    fn classifies_object_clone() {
        let m = lower_src(
            r#"
            let inner = { n: 2 };
            let obj = { a: 1, s: "hi", b: inner };
            let ch = makeChannel();
            let sent = channelSend(ch, obj);
            obj.a = 99;
            inner.n = 8;
            let rec = channelRecv(ch);
            let same = rec === obj;
            let a = rec.a;
            let s = rec.s;
            let n = rec.b.n;
            "#,
        );
        assert!(is_host_channels_module(&m));
        let ir = emit_host_channels(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_channel_send_obj"), "{ir}");
        assert!(ir.contains("draconic_rt_host_channel_recv_obj"), "{ir}");
        assert!(ir.contains("draconic_rt_alloc_object"), "{ir}");
        assert!(ir.contains("draconic_rt_object_get"), "{ir}");
    }

    #[test]
    fn classifies_object_shared_ref() {
        let m = lower_src(
            r#"
            let inner = { n: 1 };
            let o = { a: inner, b: inner };
            let ch = makeChannel();
            let bad = channelSend(ch, o);
            let err = bad < 0;
            "#,
        );
        assert!(is_host_channels_module(&m));
        let ir = emit_host_channels(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_channel_send_obj"), "{ir}");
    }

    #[test]
    fn classifies_bounded_fifo() {
        let m = lower_src(
            r#"
            let ch = makeChannel(1);
            let ok1 = channelSend(ch, 10);
            let full = channelSend(ch, 20);
            let v = channelRecv(ch);
            let ok2 = channelSend(ch, 20);
            let w = channelRecv(ch);
            "#,
        );
        assert!(is_host_channels_module(&m));
        let ir = emit_host_channels(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_channel_make"), "{ir}");
        assert!(ir.contains("fptosi double"), "{ir}");
        assert!(ir.contains("draconic_rt_host_channel_send_f64"), "{ir}");
        assert!(ir.contains("draconic_rt_host_channel_recv_f64"), "{ir}");
    }
}
