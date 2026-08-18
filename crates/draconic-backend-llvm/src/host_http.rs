//! H10.01: native HTTP/1.1 request parse.
//!
//! - `httpParseRequest(raw)` → HttpReq; `.method` / `.path` / `.version` / `.body`
//! - `httpRequestHeader(req, name)` → string (empty if missing; case-insensitive)
//! - Malformed request → stderr `EINVAL` + exit 1

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_HTTP_PARSE_REQUEST, HOST_HTTP_REQUEST_HEADER, HOST_PROCESS_EXIT,
    HOST_STDERR_WRITE, PRINT_STR,
};

pub(crate) fn is_host_http_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_host_http(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not a host_http module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module()?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    String,
    /// Opaque parse result: method/path/version/body + raw/raw_len for headers.
    HttpReq,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    print_locals: Vec<(LocalId, SlotTy)>,
}

struct ClassifyCtx {
    slots: Vec<(LocalId, SlotTy)>,
    slot_of: HashMap<LocalId, SlotTy>,
    print_locals: Vec<(LocalId, SlotTy)>,
    has_http: bool,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let mut ctx = ClassifyCtx {
        slots: Vec::new(),
        slot_of: HashMap::new(),
        print_locals: Vec::new(),
        has_http: false,
    };
    for stmt in &module.body {
        classify_stmt(stmt, &mut ctx)?;
    }
    if !ctx.has_http {
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
            if ty == SlotTy::String {
                ctx.print_locals.push((*local, ty));
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
            if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpRequestHeader") =>
        {
            ctx.has_http = true;
            classify_req_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(())
        }
        _ => None,
    }
}

fn classify_expr(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<SlotTy> {
    match expr {
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(SlotTy::HttpReq)
        }
        Expr::Call { callee, args, .. }
            if args.len() == 2 && is_named_callee(callee, "httpRequestHeader") =>
        {
            ctx.has_http = true;
            classify_req_arg(arg_expr(&args[0])?, ctx)?;
            classify_string_arg(arg_expr(&args[1])?, ctx)?;
            Some(SlotTy::String)
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
                (SlotTy::HttpReq, "method" | "path" | "version" | "body") => Some(SlotTy::String),
                _ => None,
            }
        }
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
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } => {
            let ot = classify_expr(object, ctx)?;
            let name = string_lit(property)?;
            match (ot, name.as_str()) {
                (SlotTy::HttpReq, "method" | "path" | "version" | "body") => Some(()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn classify_req_arg(expr: &Expr, ctx: &mut ClassifyCtx) -> Option<()> {
    match expr {
        Expr::Local { id, .. } => match ctx.slot_of.get(id)? {
            SlotTy::HttpReq => Some(()),
            _ => None,
        },
        Expr::Call { callee, args, .. }
            if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
        {
            ctx.has_http = true;
            classify_string_arg(arg_expr(&args[0])?, ctx)?;
            Some(())
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
            .ok_or_else(|| diag("host_http: unknown local"))?;
        Ok(format!("%slot_{name}"))
    }

    fn slot_req_field(&self, id: LocalId, field: &str) -> Result<String, Diagnostic> {
        let name = self
            .local_name
            .get(&id)
            .ok_or_else(|| diag("host_http: unknown req local"))?;
        Ok(format!("%slot_{name}_{field}"))
    }

    fn emit_cstr_ptr(&mut self, s: &str) -> String {
        let g = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".str.http.{}", self.str_globals.len());
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
            "; Draconic LLVM host_http (H10.01 httpParseRequest + httpRequestHeader)"
        )
        .ok();
        self.out.push_str(&llvm_declares(&[
            GC_INIT,
            PRINT_STR,
            HOST_HTTP_PARSE_REQUEST,
            HOST_HTTP_REQUEST_HEADER,
            HOST_STDERR_WRITE,
            HOST_PROCESS_EXIT,
        ]));
        writeln!(self.out, "declare i64 @strlen(ptr)").ok();
        writeln!(self.out).ok();

        for (id, ty) in &self.info.slots {
            match ty {
                SlotTy::String => {
                    let ptr = self.slot_ptr(*id)?;
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                }
                SlotTy::HttpReq => {
                    for f in ["method", "path", "version", "body", "raw"] {
                        let p = self.slot_req_field(*id, f)?;
                        writeln!(self.body, "  {p} = alloca ptr, align 8").ok();
                    }
                    let plen = self.slot_req_field(*id, "raw_len")?;
                    writeln!(self.body, "  {plen} = alloca i64, align 8").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, ty) in &self.info.print_locals {
            if *ty != SlotTy::String {
                continue;
            }
            let ptr = self.slot_ptr(*id)?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
            writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
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
        let fail = format!("http_err_{}", self.next_tmp);
        let cont = format!("http_ok_{}", self.next_tmp);
        self.next_tmp += 1;
        writeln!(self.body, "  {ok} = icmp eq i32 {rc}, 0").ok();
        writeln!(self.body, "  br i1 {ok}, label %{cont}, label %{fail}").ok();
        writeln!(self.body, "{fail}:").ok();
        let is_inval = self.fresh();
        let inval_l = format!("http_inval_{}", self.next_tmp);
        let other_l = format!("http_other_{}", self.next_tmp);
        self.next_tmp += 1;
        // HOST_E_INVAL = 1
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
                let Some(init) = init else {
                    return Ok(());
                };
                let ty = self
                    .slot_of
                    .get(local)
                    .copied()
                    .ok_or_else(|| diag("host_http: declare unknown slot"))?;
                match ty {
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        let ptr = self.slot_ptr(*local)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::HttpReq => {
                        self.emit_http_req_into(*local, init)?;
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => self.emit_expr_stmt(expr),
            _ => Err(diag("host_http: unsupported stmt")),
        }
    }

    fn emit_expr_stmt(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
            {
                // discard result
                let tmp_id = LocalId(u32::MAX); // unused path — emit parse into scratch
                let _ = tmp_id;
                let raw = self.emit_string_expr(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http: parse raw"))?,
                )?;
                let raw_len = self.emit_cstr_len(&raw)?;
                let om = self.fresh();
                let op = self.fresh();
                let ov = self.fresh();
                let ob = self.fresh();
                let rc = self.fresh();
                writeln!(self.body, "  {om} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {op} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {ov} = alloca ptr, align 8").ok();
                writeln!(self.body, "  {ob} = alloca ptr, align 8").ok();
                writeln!(self.body, "  store ptr null, ptr {om}").ok();
                writeln!(self.body, "  store ptr null, ptr {op}").ok();
                writeln!(self.body, "  store ptr null, ptr {ov}").ok();
                writeln!(self.body, "  store ptr null, ptr {ob}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {raw}, i64 {raw_len}, ptr {om}, ptr {op}, ptr {ov}, ptr {ob})",
                    HOST_HTTP_PARSE_REQUEST.symbol
                )
                .ok();
                self.emit_check_rc(&rc)
            }
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "httpRequestHeader") =>
            {
                let _ = self.emit_header_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http: header req"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http: header name"))?,
                )?;
                Ok(())
            }
            _ => Err(diag("host_http: unsupported expr stmt")),
        }
    }

    fn emit_http_req_into(&mut self, local: LocalId, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Call { callee, args, .. }
                if args.len() == 1 && is_named_callee(callee, "httpParseRequest") =>
            {
                let raw_e = arg_expr(&args[0]).ok_or_else(|| diag("host_http: parse raw"))?;
                let raw = self.emit_string_expr(raw_e)?;
                let raw_len = self.emit_cstr_len(&raw)?;
                let om = self.slot_req_field(local, "method")?;
                let op = self.slot_req_field(local, "path")?;
                let ov = self.slot_req_field(local, "version")?;
                let ob = self.slot_req_field(local, "body")?;
                let oraw = self.slot_req_field(local, "raw")?;
                let orlen = self.slot_req_field(local, "raw_len")?;
                let rc = self.fresh();
                writeln!(self.body, "  store ptr null, ptr {om}").ok();
                writeln!(self.body, "  store ptr null, ptr {op}").ok();
                writeln!(self.body, "  store ptr null, ptr {ov}").ok();
                writeln!(self.body, "  store ptr null, ptr {ob}").ok();
                writeln!(
                    self.body,
                    "  {rc} = call i32 @{}(ptr {raw}, i64 {raw_len}, ptr {om}, ptr {op}, ptr {ov}, ptr {ob})",
                    HOST_HTTP_PARSE_REQUEST.symbol
                )
                .ok();
                self.emit_check_rc(&rc)?;
                writeln!(self.body, "  store ptr {raw}, ptr {oraw}").ok();
                writeln!(self.body, "  store i64 {raw_len}, ptr {orlen}").ok();
                Ok(())
            }
            _ => Err(diag("host_http: expected httpParseRequest")),
        }
    }

    fn emit_cstr_len(&mut self, ptr: &str) -> Result<String, Diagnostic> {
        let n = self.fresh();
        writeln!(self.body, "  {n} = call i64 @strlen(ptr {ptr})").ok();
        Ok(n)
    }

    fn emit_header_call(&mut self, req: &Expr, name: &Expr) -> Result<String, Diagnostic> {
        let (raw, raw_len) = self.emit_req_raw(req)?;
        let nm = self.emit_string_expr(name)?;
        let out = self.fresh();
        let rc = self.fresh();
        writeln!(self.body, "  {out} = alloca ptr, align 8").ok();
        writeln!(self.body, "  store ptr null, ptr {out}").ok();
        writeln!(
            self.body,
            "  {rc} = call i32 @{}(ptr {raw}, i64 {raw_len}, ptr {nm}, ptr {out})",
            HOST_HTTP_REQUEST_HEADER.symbol
        )
        .ok();
        self.emit_check_rc(&rc)?;
        let v = self.fresh();
        writeln!(self.body, "  {v} = load ptr, ptr {out}").ok();
        Ok(v)
    }

    fn emit_req_raw(&mut self, expr: &Expr) -> Result<(String, String), Diagnostic> {
        match expr {
            Expr::Local { id, .. } => {
                let rp = self.slot_req_field(*id, "raw")?;
                let lp = self.slot_req_field(*id, "raw_len")?;
                let raw = self.fresh();
                let len = self.fresh();
                writeln!(self.body, "  {raw} = load ptr, ptr {rp}").ok();
                writeln!(self.body, "  {len} = load i64, ptr {lp}").ok();
                Ok((raw, len))
            }
            _ => Err(diag("host_http: req must be local")),
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
            Expr::Call { callee, args, .. }
                if args.len() == 2 && is_named_callee(callee, "httpRequestHeader") =>
            {
                self.emit_header_call(
                    arg_expr(&args[0]).ok_or_else(|| diag("host_http: header req"))?,
                    arg_expr(&args[1]).ok_or_else(|| diag("host_http: header name"))?,
                )
            }
            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => {
                let prop = string_lit(property).ok_or_else(|| diag("host_http: member prop"))?;
                let id = match object.as_ref() {
                    Expr::Local { id, .. } => *id,
                    _ => return Err(diag("host_http: member object must be local")),
                };
                match (self.slot_of.get(&id), prop.as_str()) {
                    (Some(SlotTy::HttpReq), "method" | "path" | "version" | "body") => {
                        let fp = self.slot_req_field(id, prop.as_str())?;
                        let v = self.fresh();
                        writeln!(self.body, "  {v} = load ptr, ptr {fp}").ok();
                        Ok(v)
                    }
                    _ => Err(diag("host_http: unsupported string member")),
                }
            }
            _ => Err(diag("host_http: unsupported string expr")),
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
    fn emit_http_parse_request_fields() {
        let m = lower_src(
            r#"
            let raw = "GET / HTTP/1.1\r\nHost: x\r\n\r\n";
            let req = httpParseRequest(raw);
            let m = req.method;
            let p = req.path;
            let v = req.version;
            let b = req.body;
            let h = httpRequestHeader(req, "Host");
            "#,
        );
        assert!(is_host_http_module(&m));
        let ir = emit_host_http(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_http_parse_request"), "{ir}");
        assert!(ir.contains("draconic_rt_host_http_request_header"), "{ir}");
    }

    #[test]
    fn emit_http_parse_malformed() {
        let m = lower_src(
            r#"
            httpParseRequest("nope");
            "#,
        );
        assert!(is_host_http_module(&m));
        let ir = emit_host_http(&m).expect("emit");
        assert!(ir.contains("EINVAL"), "{ir}");
    }
}
