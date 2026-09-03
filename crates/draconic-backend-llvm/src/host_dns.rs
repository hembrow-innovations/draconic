//! H09 / H09.01–H09.02: native DNS surface — lookup + connect-by-name.
//!
//! - `dnsLookup(host)` → GC string array (`.length` + index `[i]`)
//! - `tcpListen` / `tcpLocalPort` / `tcpConnect(name, port)` / `closeTcp` (H09.02)
//! - Resolution failure → stderr `EADDR` + exit 1
//! - Empty/invalid host → stderr `EINVAL` + exit 1
//! - Connect refused → stderr `ECONN` + exit 1

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, GC_INIT, HOST_DNS_LOOKUP,
    HOST_HANDLE_CLOSE, HOST_PROCESS_EXIT, HOST_STDERR_WRITE, HOST_TCP_CONNECT, HOST_TCP_LISTEN,
    HOST_TCP_LOCAL_PORT, PRINT_BOOL, PRINT_F64, PRINT_STR,
};

pub(crate) fn is_host_dns_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_dns(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_dns module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    String,
    Number,
    Array,
    Handle,
    Bool,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    has_dns: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_dns: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_dns {
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
            if matches!(ty, SlotTy::Bool | SlotTy::String)
                || (ty == SlotTy::Number && is_array_length(init, ctx))
            {
                ctx.print_locals.push((*local, ty));
            }
            Some(())
        }
        Stmt::Expr { expr, .. } => classify_side_effect(expr, ctx),
        _ => None,
    }
}

fn is_array_length(expr: &Expr, ctx: &ClassifyCtx) -> bool {
    match expr {
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let name = match string_lit(property) {
                Some(n) => n,
                None => return false,
            };
            if name != "length" {
                return false;
            }
            match object.as_ref() {
                Expr::Local { id, .. } => ctx.slot_of.get(id) == Some(&SlotTy::Array),
                _ => false,
            }
        }
        _ => false,
    }
}

fn classify_side_effect(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "dnsLookup") =>
        {
            ctx.has_dns = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "closeTcp") =>
        {
            classify_expr(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "dnsLookup") =>
        {
            ctx.has_dns = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::Array)
        }
        Expr::Call { callee, args, .. }
            if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
        {
            classify_expr(arg_expr(&args[0])?, ctx)?;
            if args.len() == 2 {
                classify_expr(arg_expr(&args[1])?, ctx)?;
            }
            Some(SlotTy::Handle)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
        {
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            if ht != SlotTy::Handle {
                return None;
            }
            Some(SlotTy::Number)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
        {
            let ht = classify_expr(arg_expr(&args[0])?, ctx)?;
            let pt = classify_expr(arg_expr(&args[1])?, ctx)?;
            if ht != SlotTy::String || pt != SlotTy::Number {
                return None;
            }
            Some(SlotTy::Handle)
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
                (SlotTy::Array, "length") => Some(SlotTy::Number),
                _ => None,
            }
        }
        Expr::Member {
            object,
            property,
            computed: true,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let it = classify_expr(property, ctx)?;
            if ot != SlotTy::Array || it != SlotTy::Number {
                return None;
            }
            Some(SlotTy::String)
        }
        Expr::Binary {
            op:
                BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::EqEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEq
                | BinaryOp::NotEqEq,
            left,
            right,
            ..
        } => {
            let lt = classify_expr(left, ctx)?;
            let rt = classify_expr(right, ctx)?;
            if matches!(lt, SlotTy::Number | SlotTy::Handle)
                && matches!(rt, SlotTy::Number | SlotTy::Handle)
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
            let _ = classify_expr(arg, ctx)?;
            Some(SlotTy::String)
        }
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
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
            .ok_or_else(|| diag("host_dns: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn emit_cstr_ptr(&mut self, s: &str) -> String {
        let g = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".str.dns.{}", self.str_globals.len());
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
        writeln!(
            self.out,
            "; Draconic LLVM host_dns (H09 lookup + connect-by-name surface)"
        )
        .ok();
        self.out.push_str(&llvm_declares(&[
            GC_INIT,
            PRINT_STR,
            PRINT_F64,
            PRINT_BOOL,
            HOST_DNS_LOOKUP,
            ARRAY_NEW,
            ARRAY_SET,
            ARRAY_GET,
            ARRAY_LEN,
            HOST_TCP_LISTEN,
            HOST_TCP_LOCAL_PORT,
            HOST_TCP_CONNECT,
            HOST_HANDLE_CLOSE,
            HOST_STDERR_WRITE,
            HOST_PROCESS_EXIT,
        ]));
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            let ptr = self.slot_ptr(*id)?;
            match ty {
                SlotTy::String | SlotTy::Array => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                }
                SlotTy::Number | SlotTy::Handle => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                }
                SlotTy::Bool => {
                    writeln!(self.body, "  {ptr} = alloca i8, align 1").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, ty) in &self.info.print_locals {
            let ptr = self.slot_ptr(*id)?;
            match ty {
                SlotTy::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
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
                SlotTy::Array | SlotTy::Handle => {}
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
        let fail = format!("dns_err_{}", self.next_tmp);
        let cont = format!("dns_ok_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {ok} = icmp eq i32 {rc}, 0").ok();
        writeln!(self.body, "  br i1 {ok}, label %{cont}, label %{fail}").ok();
        writeln!(self.body, "{fail}:").ok();
        let is_inval = self.fresh();
        let inval_l = format!("dns_inval_{}", self.next_tmp);
        let addr_chk = format!("dns_addrchk_{}", self.next_tmp);
        self.next_tmp += 1;
        // HOST_E_INVAL = 1
        writeln!(self.body, "  {is_inval} = icmp eq i32 {rc}, 1").ok();
        writeln!(
            self.body,
            "  br i1 {is_inval}, label %{inval_l}, label %{addr_chk}"
        )
        .ok();
        writeln!(self.body, "{inval_l}:").ok();
        self.emit_host_err_exit("EINVAL")?;
        writeln!(self.body, "{addr_chk}:").ok();
        let is_conn = self.fresh();
        let conn_l = format!("dns_conn_{}", self.next_tmp);
        let addr_chk2 = format!("dns_addrchk2_{}", self.next_tmp);
        self.next_tmp += 1;
        // HOST_E_CONN = 10
        writeln!(self.body, "  {is_conn} = icmp eq i32 {rc}, 10").ok();
        writeln!(
            self.body,
            "  br i1 {is_conn}, label %{conn_l}, label %{addr_chk2}"
        )
        .ok();
        writeln!(self.body, "{conn_l}:").ok();
        self.emit_host_err_exit("ECONN")?;
        writeln!(self.body, "{addr_chk2}:").ok();
        let is_addr = self.fresh();
        let addr_l = format!("dns_addr_{}", self.next_tmp);
        let other_l = format!("dns_other_{}", self.next_tmp);
        self.next_tmp += 1;
        // HOST_E_ADDR = 11
        writeln!(self.body, "  {is_addr} = icmp eq i32 {rc}, 11").ok();
        writeln!(
            self.body,
            "  br i1 {is_addr}, label %{addr_l}, label %{other_l}"
        )
        .ok();
        writeln!(self.body, "{addr_l}:").ok();
        self.emit_host_err_exit("EADDR")?;
        writeln!(self.body, "{other_l}:").ok();
        self.emit_host_err_exit("EIO")?;
        writeln!(self.body, "{cont}:").ok();
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                let ty = self
                    .slot_of
                    .get(local)
                    .copied()
                    .ok_or_else(|| diag("host_dns: declare unknown slot"))?;
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
                    SlotTy::Handle => {
                        let v = self.emit_handle_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Bool => {
                        let v = self.emit_bool_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store i8 {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Array => {
                        let v = self.emit_array_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => self.emit_expr_stmt(expr),
            _ => Err(diag("host_dns: unsupported stmt")),
        }
    }

    fn emit_expr_stmt(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "dnsLookup") =>
            {
                let _ = self.emit_dns_lookup(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_dns: dnsLookup host"))?,
                )?;
                Ok(())
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "closeTcp") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_dns: closeTcp handle"))?,
                )?;
                let rc = self.fresh();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h})",
                    HOST_HANDLE_CLOSE.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            _ => Err(diag("host_dns: unsupported expr stmt")),
        }
    }

    fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "dnsLookup") =>
            {
                self.emit_dns_lookup(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_dns: dnsLookup host"))?,
                )
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_dns: expected dnsLookup array")),
        }
    }

    fn emit_dns_lookup(&mut self, host: &Expr) -> Result<String, Diagnostic> {
        let h = self.emit_string_expr(host)?;
        let out_addrs = self.fresh();
        let out_count = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out_addrs} = alloca ptr, align 8").ok();
        writeln!(self.body, "  {out_count} = alloca i64, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out_addrs}").ok();
        writeln!(self.body, "  store i64 0, ptr {out_count}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(ptr {h}, ptr {out_addrs}, ptr {out_count})",
            HOST_DNS_LOOKUP.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let names = self.fresh();
        let n = self.fresh();
        writeln!(self.body, "  {names} = load ptr, ptr {out_addrs}").ok();
        writeln!(self.body, "  {n} = load i64, ptr {out_count}").ok();
        let arr = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_NEW.call_to(&arr, &format!("i64 {n}"))
        )
        .ok();
        let i_slot = self.fresh();
        let loop_cond = format!("dns_loop_cond_{}", self.next_tmp);
        let loop_body = format!("dns_loop_body_{}", self.next_tmp);
        let loop_end = format!("dns_loop_end_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {i_slot} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {i_slot}").ok();
        writeln!(self.body, "  br label %{loop_cond}").ok();
        writeln!(self.body, "{loop_cond}:").ok();
        let i_load = self.fresh();
        let cmp = self.fresh();
        writeln!(self.body, "  {i_load} = load i64, ptr {i_slot}").ok();
        writeln!(self.body, "  {cmp} = icmp slt i64 {i_load}, {n}").ok();
        writeln!(
            self.body,
            "  br i1 {cmp}, label %{loop_body}, label %{loop_end}"
        )
        .ok();
        writeln!(self.body, "{loop_body}:").ok();
        let name_pp = self.fresh();
        let name_p = self.fresh();
        let i_next = self.fresh();
        writeln!(
            self.body,
            "  {name_pp} = getelementptr inbounds ptr, ptr {names}, i64 {i_load}"
        )
        .ok();
        writeln!(self.body, "  {name_p} = load ptr, ptr {name_pp}").ok();
        writeln!(
            self.body,
            "  call void @{}(ptr {arr}, i64 {i_load}, ptr {name_p})",
            ARRAY_SET.symbol
        )
        .ok();
        writeln!(self.body, "  {i_next} = add i64 {i_load}, 1").ok();
        writeln!(self.body, "  store i64 {i_next}, ptr {i_slot}").ok();
        writeln!(self.body, "  br label %{loop_cond}").ok();
        writeln!(self.body, "{loop_end}:").ok();
        Ok(arr)
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
                computed: true,
                ..
            } => {
                let arr = self.emit_array_expr(object)?;
                let idx_f = self.emit_number_expr(property)?;
                let idx = self.fresh();
                let el = self.fresh();
                writeln!(self.body, "  {idx} = fptosi double {idx_f} to i64").ok();
                writeln!(
                    self.body,
                    "  {el} = call ptr @{}(ptr {arr}, i64 {idx})",
                    ARRAY_GET.symbol
                )
                .ok();
                Ok(el)
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => self.emit_typeof(arg),
            _ => Err(diag("host_dns: unsupported string expr")),
        }
    }

    fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        match arg {
            Expr::Local { id, .. } => {
                let ty = self
                    .slot_of
                    .get(id)
                    .copied()
                    .ok_or_else(|| diag("host_dns: typeof unknown local"))?;
                let s = match ty {
                    SlotTy::Handle | SlotTy::Number => "number",
                    SlotTy::Bool => "boolean",
                    SlotTy::String | SlotTy::Array => "object",
                };
                Ok(self.emit_cstr_ptr(s))
            }
            _ => Err(diag("host_dns: typeof unsupported arg")),
        }
    }

    fn emit_handle_i64(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let f = self.emit_handle_expr(expr)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {f} to i64").ok();
        Ok(i)
    }

    fn emit_handle_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if (args.len() == 1 || args.len() == 2) && is_named_callee(callee, "tcpListen") =>
            {
                let port_f = self.emit_number_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_dns: tcpListen port"))?,
                )?;
                let port_i = self.fresh();
                writeln!(self.body, "  {port_i} = fptosi double {port_f} to i32").ok();
                let backlog_i = if args.len() == 2 {
                    let bf = self.emit_number_expr(
                        arg_expr(&args[1]).ok_or_else(|| diag("host_dns: tcpListen backlog"))?,
                    )?;
                    let bi = self.fresh();
                    writeln!(self.body, "  {bi} = fptosi double {bf} to i32").ok();
                    bi
                } else {
                    "0".to_string()
                };
                let out_h = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_h} = alloca i64, align 8").ok();
                writeln!(self.body, "  store i64 -1, ptr {out_h}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i32 {port_i}, i32 {backlog_i}, ptr {out_h})",
                    HOST_TCP_LISTEN.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let iv = self.fresh();
                let fv = self.fresh();
                writeln!(self.body, "  {iv} = load i64, ptr {out_h}").ok();
                writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                Ok(fv)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "tcpConnect") =>
            {
                let host = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_dns: tcpConnect host"))?,
                )?;
                let port_f = self.emit_number_expr(
                    arg_expr(&args[1]).ok_or_else(|| diag("host_dns: tcpConnect port"))?,
                )?;
                let port_i = self.fresh();
                writeln!(self.body, "  {port_i} = fptosi double {port_f} to i32").ok();
                let out_h = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_h} = alloca i64, align 8").ok();
                writeln!(self.body, "  store i64 -1, ptr {out_h}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {host}, i32 {port_i}, ptr {out_h})",
                    HOST_TCP_CONNECT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let iv = self.fresh();
                let fv = self.fresh();
                writeln!(self.body, "  {iv} = load i64, ptr {out_h}").ok();
                writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                Ok(fv)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("host_dns: expected handle expr")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Binary {
                op, left, right, ..
            } if matches!(
                op,
                BinaryOp::Gt
                    | BinaryOp::GtEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::EqEq
                    | BinaryOp::EqEqEq
                    | BinaryOp::NotEq
                    | BinaryOp::NotEqEq
            ) =>
            {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let cmp = self.fresh();
                let pred = match op {
                    BinaryOp::Gt => "ogt",
                    BinaryOp::GtEq => "oge",
                    BinaryOp::Lt => "olt",
                    BinaryOp::LtEq => "ole",
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "oeq",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "one",
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
            _ => Err(diag("host_dns: expected bool expr")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                if raw.contains('.') || raw.contains('e') || raw.contains('E') {
                    Ok(raw.clone())
                } else {
                    Ok(format!("{raw}.0"))
                }
            }
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "tcpLocalPort") =>
            {
                let h = self.emit_handle_i64(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_dns: tcpLocalPort handle"))?,
                )?;
                let out_p = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {out_p} = alloca i32, align 4").ok();
                writeln!(self.body, "  store i32 0, ptr {out_p}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(i64 {h}, ptr {out_p})",
                    HOST_TCP_LOCAL_PORT.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                let iv = self.fresh();
                let fv = self.fresh();
                writeln!(self.body, "  {iv} = load i32, ptr {out_p}").ok();
                writeln!(self.body, "  {fv} = sitofp i32 {iv} to double").ok();
                Ok(fv)
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
                let prop = string_lit(property).ok_or_else(|| diag("host_dns: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_dns: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::Array), "length") => {
                        let ap = self.slot_ptr(id)?;
                        let arr = self.fresh();
                        let iv = self.fresh();
                        let fv = self.fresh();
                        writeln!(self.body, "  {arr} = load ptr, ptr {ap}").ok();
                        writeln!(
                            self.body,
                            "  {iv} = call i64 @{}(ptr {arr})",
                            ARRAY_LEN.symbol
                        )
                        .ok();
                        writeln!(self.body, "  {fv} = sitofp i64 {iv} to double").ok();
                        Ok(fv)
                    }
                    _ => Err(diag("host_dns: unsupported number member")),
                }
            }
            _ => Err(diag("host_dns: unsupported number expr")),
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
    fn emit_dns_lookup_loopback() {
        let m = lower_src(
            r#"
            let addrs = dnsLookup("127.0.0.1");
            let n = addrs.length;
            let a0 = addrs[0];
            "#,
        );
        assert!(is_host_dns_module(&m));
        let ir = emit_host_dns(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_dns_lookup"), "{ir}");
        assert!(ir.contains("draconic_rt_array_new"), "{ir}");
    }

    #[test]
    fn emit_dns_lookup_fail_stmt() {
        let m = lower_src(
            r#"
            dnsLookup("this-host-definitely-does-not-exist.invalid");
            "#,
        );
        assert!(is_host_dns_module(&m));
        let ir = emit_host_dns(&m).expect("emit");
        assert!(ir.contains("EADDR"), "{ir}");
    }

    #[test]
    fn emit_dns_connect_by_name_surface() {
        let m = lower_src(
            r#"
            let addrs = dnsLookup("127.0.0.1");
            let n = addrs.length;
            let a0 = addrs[0];
            let s = tcpListen(0);
            let p = tcpLocalPort(s);
            let c = tcpConnect("localhost", p);
            let t = typeof c;
            let ok = c > 0;
            closeTcp(c);
            closeTcp(s);
            "#,
        );
        assert!(is_host_dns_module(&m));
        let ir = emit_host_dns(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_dns_lookup"), "{ir}");
        assert!(ir.contains("draconic_rt_host_tcp_connect"), "{ir}");
        assert!(ir.contains("draconic_rt_host_handle_close"), "{ir}");
    }
}
