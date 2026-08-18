//! H09.01: native DNS lookup — `dnsLookup(hostname)` → string[] of IPv4 addresses.
//!
//! - `dnsLookup(host)` → GC string array (`.length` + index `[i]`)
//! - Resolution failure → stderr `EADDR` + exit 1
//! - Empty/invalid host → stderr `EINVAL` + exit 1

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, GC_INIT, HOST_DNS_LOOKUP,
    HOST_PROCESS_EXIT, HOST_STDERR_WRITE, PRINT_F64, PRINT_STR,
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
            match ty {
                SlotTy::String | SlotTy::Number => {
                    ctx.print_locals.push((*local, ty));
                }
                SlotTy::Array => {}
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
            if args.len() == 1 && is_named_callee(callee, "dnsLookup") =>
        {
            ctx.has_dns = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
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
            "; Draconic LLVM host_dns (H09.01 dnsLookup → IPv4 address strings)"
        )
        .ok();
        self.out.push_str(&llvm_declares(&[
            GC_INIT,
            PRINT_STR,
            PRINT_F64,
            HOST_DNS_LOOKUP,
            ARRAY_NEW,
            ARRAY_SET,
            ARRAY_GET,
            ARRAY_LEN,
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
                SlotTy::Number => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
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
                SlotTy::Array => {}
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
            _ => Err(diag("host_dns: unsupported string expr")),
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
}
