//! H12.02: native WebSocket frames (RFC 6455 §5).
//!
//! - `wsEncodeText` / `wsEncodeBinary` / `wsEncodeClose` / `wsEncodePing` / `wsEncodePong` → DynBytes
//! - `wsDecodeFrame(bytes)` → `{ fin, opcode, payload, closeCode }`
//! - Malformed → stderr `EINVAL` + exit 1

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_PROCESS_EXIT, HOST_STDERR_WRITE, HOST_STDOUT_WRITE,
    HOST_WS_DECODE_FRAME, HOST_WS_ENCODE_BINARY, HOST_WS_ENCODE_CLOSE, HOST_WS_ENCODE_PING,
    HOST_WS_ENCODE_PONG, HOST_WS_ENCODE_TEXT, PRINT_I64, PRINT_STR,
};

pub(crate) fn is_host_ws_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_ws(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_ws module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    String,
    Number,
    DynBytes,
    WsFrame,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    has_ws: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_ws: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_ws {
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
            ctx.slots.push((*local, ty));
            ctx.slot_of.insert(*local, ty);
            match ty {
                SlotTy::Number | SlotTy::String => {
                    ctx.print_locals.push((*local, ty));
                }
                SlotTy::DynBytes | SlotTy::WsFrame => {}
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        _ => None,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
        {
            classify_bytes_or_string(arg_expr(&args[0])?, ctx)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsDecodeFrame") =>
        {
            ctx.has_ws = true;
            classify_bytes_or_string(arg_expr(&args[0])?, ctx)
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodeText") =>
        {
            ctx.has_ws = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodeBinary") =>
        {
            ctx.has_ws = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "wsEncodeClose") =>
        {
            ctx.has_ws = true;
            classify_number_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodePing") =>
        {
            ctx.has_ws = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsEncodePong") =>
        {
            ctx.has_ws = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::DynBytes)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "wsDecodeFrame") =>
        {
            ctx.has_ws = true;
            classify_bytes_or_string(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::WsFrame)
        }
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            match (ot, name.as_str()) {
                (SlotTy::DynBytes, "length") => Some(SlotTy::Number),
                (SlotTy::WsFrame, "fin" | "opcode" | "closeCode") => Some(SlotTy::Number),
                (SlotTy::WsFrame, "payload") => Some(SlotTy::String),
                _ => None,
            }
        }
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::Local { id, .. } => ctx.slot_of.get(id).copied(),
        _ => None,
    }
}

fn classify_string_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::String => Some(()),
            _ => None,
        },
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            matches!((ot, name.as_str()), (SlotTy::WsFrame, "payload")).then_some(())
        }
        _ => None,
    }
}

fn classify_number_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Number { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::Number => Some(()),
            _ => None,
        },
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            match (ot, name.as_str()) {
                (SlotTy::DynBytes, "length") => Some(()),
                (SlotTy::WsFrame, "fin" | "opcode" | "closeCode") => Some(()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_bytes_or_string(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::String { .. } => Some(()),
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::String | SlotTy::DynBytes => Some(()),
            _ => None,
        },
        Expr::Call { .. } => {
            let ty = classify_expr(expr, ctx)?;
            matches!(ty, SlotTy::DynBytes | SlotTy::String).then_some(())
        }
        _ => None,
    }
}

fn is_named_callee(expr: &Expr, want: &str) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == want)
}

fn arg_expr(arg: &Arg) -> Option<&Expr> {
    match arg {
        Arg::Expr(e) => Some(e),
        _ => None,
    }
}

fn string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy().to_string()),
        _ => None,
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
    out: String,
    body: String,
    next_tmp: usize,
    str_globals: Vec<(String, String)>,
    local_name: HashMap<LocalId, String>,
    slot_of: HashMap<LocalId, SlotTy>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        let mut local_name = HashMap::new();
        for Local { id, name, .. } in &module.locals {
            local_name.insert(*id, name.clone());
        }
        let mut slot_of = HashMap::new();
        for (id, ty) in &info.slots {
            slot_of.insert(*id, *ty);
        }
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            next_tmp: 0,
            str_globals: Vec::new(),
            local_name,
            slot_of,
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
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_ws: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn slot_len_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_ws: unknown local"))?;
        Ok(format!("%slot_{name}_len"))
    }

    fn slot_frame_field(&self, id: LocalId, field: &str) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_ws: unknown frame local"))?;
        Ok(format!("%slot_{name}_{field}"))
    }

    fn emit_cstr_ptr(&mut self, s: &str) -> String {
        let g = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".str.ws.{}", self.str_globals.len());
            self.str_globals.push((s.to_string(), g.clone()));
            g
        };
        let p = self.fresh();
        let n = s.len() + 1;
        writeln!(
            self.body,
            "  {p} = getelementptr inbounds [{n} x i8], ptr @{g}, i64 0, i64 0"
        )
        .ok();
        p
    }

    fn emit_module(&mut self) -> Result<(), Diagnostic> {
        writeln!(self.out, "; Draconic LLVM host_ws (H12.02 frames)").ok();
        self.out.push_str(&llvm_declares(&[
            GC_INIT,
            PRINT_STR,
            PRINT_I64,
            HOST_STDOUT_WRITE,
            HOST_WS_ENCODE_TEXT,
            HOST_WS_ENCODE_BINARY,
            HOST_WS_ENCODE_CLOSE,
            HOST_WS_ENCODE_PING,
            HOST_WS_ENCODE_PONG,
            HOST_WS_DECODE_FRAME,
            HOST_STDERR_WRITE,
            HOST_PROCESS_EXIT,
        ]));
        writeln!(self.out, "declare i64 @strlen(ptr)").ok();
        writeln!(self.out, "declare ptr @malloc(i64)").ok();
        writeln!(
            self.out,
            "declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg)"
        )
        .ok();
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            match ty {
                SlotTy::String => {
                    let ptr = self.slot_ptr(*id)?;
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                }
                SlotTy::Number => {
                    let ptr = self.slot_ptr(*id)?;
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                }
                SlotTy::DynBytes => {
                    let ptr = self.slot_ptr(*id)?;
                    let lp = self.slot_len_ptr(*id)?;
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {lp} = alloca i64, align 8").ok();
                }
                SlotTy::WsFrame => {
                    for f in ["fin", "opcode", "close_code"] {
                        let p = self.slot_frame_field(*id, f)?;
                        writeln!(self.body, "  {p} = alloca i32, align 4").ok();
                    }
                    let pp = self.slot_frame_field(*id, "payload")?;
                    let pl = self.slot_frame_field(*id, "payload_len")?;
                    writeln!(self.body, "  {pp} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  {pl} = alloca i64, align 8").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, ty) in &self.info.print_locals {
            match ty {
                SlotTy::String => {
                    let ptr = self.slot_ptr(*id)?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotTy::Number => {
                    let ptr = self.slot_ptr(*id)?;
                    let v = self.fresh();
                    let i = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {i} = fptosi double {v} to i64").ok();
                    writeln!(self.body, "  {}", PRINT_I64.call(&format!("i64 {i}"))).ok();
                }
                SlotTy::DynBytes | SlotTy::WsFrame => {}
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

    fn emit_frame_payload_cstr(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        let pp = self.slot_frame_field(id, "payload")?;
        let pl = self.slot_frame_field(id, "payload_len")?;
        let d = self.fresh();
        let n = self.fresh();
        writeln!(self.body, "  {d} = load ptr, ptr {pp}").ok();
        writeln!(self.body, "  {n} = load i64, ptr {pl}").ok();
        let buf = self.fresh();
        let np1 = self.fresh();
        writeln!(self.body, "  {np1} = add i64 {n}, 1").ok();
        writeln!(self.body, "  {buf} = call ptr @malloc(i64 {np1})").ok();
        let copy_ok = format!("ws_pay_copy_{}", self.next_tmp);
        let copy_skip = format!("ws_pay_skip_{}", self.next_tmp);
        let copy_done = format!("ws_pay_done_{}", self.next_tmp);
        self.next_tmp += 1;
        let is_null = self.fresh();
        writeln!(self.body, "  {is_null} = icmp eq ptr {d}, null").ok();
        writeln!(
            self.body,
            "  br i1 {is_null}, label %{copy_skip}, label %{copy_ok}"
        )
        .ok();
        writeln!(self.body, "{copy_ok}:").ok();
        writeln!(
            self.body,
            "  call void @llvm.memcpy.p0.p0.i64(ptr {buf}, ptr {d}, i64 {n}, i1 false)"
        )
        .ok();
        writeln!(self.body, "  br label %{copy_done}").ok();
        writeln!(self.body, "{copy_skip}:").ok();
        writeln!(self.body, "  br label %{copy_done}").ok();
        writeln!(self.body, "{copy_done}:").ok();
        let endp = self.fresh();
        writeln!(
            self.body,
            "  {endp} = getelementptr inbounds i8, ptr {buf}, i64 {n}"
        )
        .ok();
        writeln!(self.body, "  store i8 0, ptr {endp}").ok();
        Ok(buf)
    }

    fn emit_host_err_exit(&mut self, code: &str) -> Result<(), Diagnostic> {
        let msg = format!("{code}\n");
        let p = self.emit_cstr_ptr(&msg);
        let n = msg.len();
        writeln!(
            self.body,
            "  {}",
            HOST_STDERR_WRITE.call(&format!("ptr {p}, i64 {n}"))
        )
        .ok();
        writeln!(self.body, "  {}", HOST_PROCESS_EXIT.call("i32 1")).ok();
        writeln!(self.body, "  unreachable").ok();
        Ok(())
    }

    fn emit_check_rc(&mut self, rc: &str) -> Result<(), Diagnostic> {
        let ok = self.fresh();
        let fail = format!("ws_err_{}", self.next_tmp);
        let cont = format!("ws_ok_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {ok} = icmp eq i32 {rc}, 0").ok();
        writeln!(self.body, "  br i1 {ok}, label %{cont}, label %{fail}").ok();
        writeln!(self.body, "{fail}:").ok();
        let is_inval = self.fresh();
        let inval_l = format!("ws_inval_{}", self.next_tmp);
        let other_l = format!("ws_other_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {is_inval} = icmp eq i32 {rc}, 1").ok();
        writeln!(
            self.body,
            "  br i1 {is_inval}, label %{inval_l}, label %{other_l}"
        )
        .ok();
        writeln!(self.body, "{inval_l}:").ok();
        self.emit_host_err_exit("EINVAL")?;
        writeln!(self.body, "{other_l}:").ok();
        self.emit_host_err_exit("EIO")?;
        writeln!(self.body, "{cont}:").ok();
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let init = init
                    .as_ref()
                    .ok_or_else(|| diag("host_ws: declare needs init"))?;
                let ty = self
                    .slot_of
                    .get(local)
                    .copied()
                    .ok_or_else(|| diag("host_ws: unknown slot"))?;
                match ty {
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::DynBytes => self.emit_dynbytes_into(*local, init)?,
                    SlotTy::WsFrame => self.emit_frame_into(*local, init)?,
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => self.emit_expr_stmt(expr),
            _ => Err(diag("host_ws: unsupported stmt")),
        }
    }

    fn emit_expr_stmt(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "stdoutWrite") =>
            {
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws: stdoutWrite arg"))?,
                )?;
                writeln!(
                    self.body,
                    "  {}",
                    HOST_STDOUT_WRITE.call(&format!("ptr {d}, i64 {n}"))
                )
                .ok();
                Ok(())
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsDecodeFrame") =>
            {
                // Discard result; still check rc (EINVAL on bad frame).
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws: decode data"))?,
                )?;
                let out_fin = self.fresh();
                let out_op = self.fresh();
                let out_pay = self.fresh();
                let out_plen = self.fresh();
                let out_cc = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_fin} = alloca i32, align 4").ok();
                writeln!(self.body, "  {out_op} = alloca i32, align 4").ok();
                writeln!(self.body, "  {out_pay} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_plen} = alloca i64, align 8").ok();
                writeln!(self.body, "  {out_cc} = alloca i32, align 4").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {d}, i64 {n}, ptr {out_fin}, ptr {out_op}, ptr {out_pay}, ptr {out_plen}, ptr {out_cc})",
                    HOST_WS_DECODE_FRAME.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            _ => Err(diag("host_ws: unsupported expr stmt")),
        }
    }

    fn emit_dynbytes_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        let out_data = self.fresh();
        let out_len = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out_data} = alloca ptr, align 8").ok();
        writeln!(self.body, "  {out_len} = alloca i64, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out_data}").ok();
        writeln!(self.body, "  store i64 0, ptr {out_len}").ok();

        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsEncodeText") =>
            {
                let p = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws: text payload"))?,
                )?;
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {p}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_TEXT.symbol
                )
                .ok();
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsEncodeBinary") =>
            {
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws: bin payload"))?,
                )?;
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {d}, i64 {n}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_BINARY.symbol
                )
                .ok();
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "wsEncodeClose") =>
            {
                let code_f = self.emit_number_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws: close code"))?,
                )?;
                let code_i = self.fresh();
                writeln!(self.body, "  {code_i} = fptosi double {code_f} to i32").ok();
                let reason = self.emit_string_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_ws: close reason"))?,
                )?;
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i32 {code_i}, ptr {reason}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_CLOSE.symbol
                )
                .ok();
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsEncodePing") =>
            {
                let p = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws: ping payload"))?,
                )?;
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {p}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_PING.symbol
                )
                .ok();
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsEncodePong") =>
            {
                let p = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws: pong payload"))?,
                )?;
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {p}, ptr {out_data}, ptr {out_len})",
                    HOST_WS_ENCODE_PONG.symbol
                )
                .ok();
            }
            _ => return Err(diag("host_ws: expected encode for DynBytes")),
        }

        self.emit_check_rc(&rc)?;
        let d = self.fresh();
        let n = self.fresh();
        writeln!(self.body, "  {d} = load ptr, ptr {out_data}").ok();
        writeln!(self.body, "  {n} = load i64, ptr {out_len}").ok();
        let dp = self.slot_ptr(local)?;
        let lp = self.slot_len_ptr(local)?;
        writeln!(self.body, "  store ptr {d}, ptr {dp}").ok();
        writeln!(self.body, "  store i64 {n}, ptr {lp}").ok();
        Ok(())
    }

    fn emit_frame_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "wsDecodeFrame") =>
            {
                let (d, n) = self.emit_bytes_ptr_len(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_ws: decode data"))?,
                )?;
                let out_fin = self.fresh();
                let out_op = self.fresh();
                let out_pay = self.fresh();
                let out_plen = self.fresh();
                let out_cc = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_fin} = alloca i32, align 4").ok();
                writeln!(self.body, "  {out_op} = alloca i32, align 4").ok();
                writeln!(self.body, "  {out_pay} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {out_plen} = alloca i64, align 8").ok();
                writeln!(self.body, "  {out_cc} = alloca i32, align 4").ok();
                writeln!(self.body, "  store i32 0, ptr {out_fin}").ok();
                writeln!(self.body, "  store i32 0, ptr {out_op}").ok();
                writeln!(self.body, "  store ptr null, ptr {out_pay}").ok();
                writeln!(self.body, "  store i64 0, ptr {out_plen}").ok();
                writeln!(self.body, "  store i32 -1, ptr {out_cc}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {d}, i64 {n}, ptr {out_fin}, ptr {out_op}, ptr {out_pay}, ptr {out_plen}, ptr {out_cc})",
                    HOST_WS_DECODE_FRAME.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                for (field, tmp) in [
                    ("fin", &out_fin),
                    ("opcode", &out_op),
                    ("close_code", &out_cc),
                ] {
                    let v = self.fresh();
                    let dest = self.slot_frame_field(local, field)?;
                    writeln!(self.body, "  {v} = load i32, ptr {tmp}").ok();
                    writeln!(self.body, "  store i32 {v}, ptr {dest}").ok();
                }
                let pd = self.fresh();
                let pn = self.fresh();
                writeln!(self.body, "  {pd} = load ptr, ptr {out_pay}").ok();
                writeln!(self.body, "  {pn} = load i64, ptr {out_plen}").ok();
                let pp = self.slot_frame_field(local, "payload")?;
                let pl = self.slot_frame_field(local, "payload_len")?;
                writeln!(self.body, "  store ptr {pd}, ptr {pp}").ok();
                writeln!(self.body, "  store i64 {pn}, ptr {pl}").ok();
                Ok(())
            }
            _ => Err(diag("host_ws: expected wsDecodeFrame for WsFrame")),
        }
    }

    fn emit_bytes_ptr_len(&mut self, expr: &Expr) -> Result<(String, String), Diagnostic> {
        match expr {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy();
                let p = self.emit_cstr_ptr(&s);
                Ok((p, s.len().to_string()))
            }
            Expr::Local { id, .. } => match self.slot_of.get(id) {
                Some(SlotTy::DynBytes) => {
                    let dp = self.slot_ptr(*id)?;
                    let lp = self.slot_len_ptr(*id)?;
                    let d = self.fresh();
                    let n = self.fresh();
                    writeln!(self.body, "  {d} = load ptr, ptr {dp}").ok();
                    writeln!(self.body, "  {n} = load i64, ptr {lp}").ok();
                    Ok((d, n))
                }
                Some(SlotTy::String) => {
                    let sp = self.slot_ptr(*id)?;
                    let s = self.fresh();
                    writeln!(self.body, "  {s} = load ptr, ptr {sp}").ok();
                    let n = self.fresh();
                    writeln!(self.body, "  {n} = call i64 @strlen(ptr {s})").ok();
                    Ok((s, n))
                }
                _ => Err(diag("host_ws: bytes arg unsupported")),
            },
            _ => Err(diag("host_ws: bytes arg unsupported")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let t = self.fresh();
                let lit = if raw.contains('.') || raw.contains('e') || raw.contains('E') {
                    raw.clone()
                } else {
                    format!("{raw}.0")
                };
                writeln!(self.body, "  {t} = fadd double {lit}, 0.0").ok();
                Ok(t)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_ws: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_ws: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::DynBytes), "length") => {
                        let lp = self.slot_len_ptr(id)?;
                        let i = self.fresh();
                        let d = self.fresh();
                        writeln!(self.body, "  {i} = load i64, ptr {lp}").ok();
                        writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                        Ok(d)
                    }
                    (Some(SlotTy::WsFrame), "fin" | "opcode" | "closeCode") => {
                        let field = if prop == "closeCode" {
                            "close_code"
                        } else {
                            prop.as_str()
                        };
                        let fp = self.slot_frame_field(id, field)?;
                        let i = self.fresh();
                        let d = self.fresh();
                        writeln!(self.body, "  {i} = load i32, ptr {fp}").ok();
                        writeln!(self.body, "  {d} = sitofp i32 {i} to double").ok();
                        Ok(d)
                    }
                    _ => Err(diag("host_ws: unsupported number member")),
                }
            }
            _ => Err(diag("host_ws: unsupported number expr")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => {
                let s = value.to_string_lossy().to_string();
                Ok(self.emit_cstr_ptr(&s))
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_ws: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_ws: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::WsFrame), "payload") => self.emit_frame_payload_cstr(id),
                    _ => Err(diag("host_ws: unsupported string member")),
                }
            }
            _ => Err(diag("host_ws: unsupported string expr")),
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
    fn emit_ws_encode_text_decode() {
        let m = lower_src(
            r#"
            let f = wsEncodeText("Hello");
            let n = f.length;
            let d = wsDecodeFrame(f);
            let op = d.opcode;
            let fin = d.fin;
            let p = d.payload;
            "#,
        );
        assert!(is_host_ws_module(&m));
        let ir = emit_host_ws(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_ws_encode_text"), "{ir}");
        assert!(ir.contains("draconic_rt_host_ws_decode_frame"), "{ir}");
    }

    #[test]
    fn emit_ws_encode_close_ping_pong() {
        let m = lower_src(
            r#"
            let c = wsEncodeClose(1000, "bye");
            let pi = wsEncodePing("x");
            let po = wsEncodePong("x");
            let d1 = wsDecodeFrame(c);
            let d2 = wsDecodeFrame(pi);
            let d3 = wsDecodeFrame(po);
            let a = d1.closeCode;
            let b = d2.opcode;
            let e = d3.opcode;
            "#,
        );
        assert!(is_host_ws_module(&m));
        let ir = emit_host_ws(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_ws_encode_close"), "{ir}");
        assert!(ir.contains("draconic_rt_host_ws_encode_ping"), "{ir}");
        assert!(ir.contains("draconic_rt_host_ws_encode_pong"), "{ir}");
    }
}
