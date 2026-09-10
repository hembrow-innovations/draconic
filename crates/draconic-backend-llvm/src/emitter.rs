use std::fmt::Write as _;

use draconic_ir::Module;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotTy {
    Number,
    BigInt,
    Boolean,
    String,
    Undefined,
}

impl SlotTy {
    pub(crate) fn llvm_ty(self) -> Option<(&'static str, u32)> {
        match self {
            SlotTy::Number => Some(("double", 8)),
            SlotTy::BigInt => Some(("i64", 8)),
            SlotTy::Boolean => Some(("i1", 1)),
            SlotTy::String => Some(("ptr", 8)),
            SlotTy::Undefined => None,
        }
    }

    pub(crate) fn write_alloca(self, body: &mut String, ptr: &str) {
        if let Some((ty, align)) = self.llvm_ty() {
            let _ = writeln!(body, "  {ptr} = alloca {ty}, align {align}");
        }
    }
}

#[derive(Clone, Copy)]
enum LabelAlloc {
    WithTmp,
    Dedicated,
}

pub(crate) struct Emitter<'a, S> {
    pub(crate) module: &'a Module,
    pub(crate) state: S,
    pub(crate) out: String,
    pub(crate) body: String,
    pub(crate) tmp: u32,
    pub(crate) label: u32,
    label_alloc: LabelAlloc,
}

impl<'a, S> Emitter<'a, S> {
    pub(crate) fn new(module: &'a Module, state: S) -> Self {
        Self {
            module,
            state,
            out: String::new(),
            body: String::new(),
            tmp: 0,
            label: 0,
            label_alloc: LabelAlloc::WithTmp,
        }
    }

    pub(crate) fn new_dedicated_labels(module: &'a Module, state: S) -> Self {
        Self {
            label_alloc: LabelAlloc::Dedicated,
            ..Self::new(module, state)
        }
    }

    pub(crate) fn fresh(&mut self) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("%t{n}")
    }

    pub(crate) fn fresh_label(&mut self, prefix: &str) -> String {
        match self.label_alloc {
            LabelAlloc::WithTmp => {
                let n = self.tmp;
                self.tmp += 1;
                format!("{prefix}{n}")
            }
            LabelAlloc::Dedicated => {
                let n = self.label;
                self.label += 1;
                format!("{prefix}{n}")
            }
        }
    }

    pub(crate) fn body_ends_with_terminator(&self) -> bool {
        self.body
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .is_some_and(|l| {
                let t = l.trim_start();
                t.starts_with("br ")
                    || t.starts_with("ret ")
                    || t.starts_with("unreachable")
                    || t.starts_with("switch ")
                    || t.starts_with("indirectbr ")
            })
    }

    pub(crate) fn finish(self) -> String {
        self.out
    }
}

pub(crate) fn escape_llvm_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for b in bytes {
        match *b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) && c != b'\\' => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

pub(crate) fn escape_llvm_string(s: &str) -> String {
    escape_llvm_bytes(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use draconic_ir::Module;

    use super::*;

    fn empty_module() -> Module {
        Module {
            locals: Vec::new(),
            body: Vec::new(),
            body_spans: Vec::new(),
            shapes: Vec::new(),
            has_extern_ffi: false,
        }
    }

    #[test]
    fn slot_ty_llvm_layout_matches_es_expr() {
        assert_eq!(SlotTy::Number.llvm_ty(), Some(("double", 8)));
        assert_eq!(SlotTy::BigInt.llvm_ty(), Some(("i64", 8)));
        assert_eq!(SlotTy::Boolean.llvm_ty(), Some(("i1", 1)));
        assert_eq!(SlotTy::String.llvm_ty(), Some(("ptr", 8)));
        assert_eq!(SlotTy::Undefined.llvm_ty(), None);
    }

    #[test]
    fn write_alloca_emits_layout_or_nothing() {
        let mut body = String::new();
        SlotTy::Number.write_alloca(&mut body, "%l0");
        SlotTy::Undefined.write_alloca(&mut body, "%l1");
        assert_eq!(body, "  %l0 = alloca double, align 8\n");
    }

    #[test]
    fn fresh_temps_are_sequential() {
        let module = empty_module();
        let mut em = Emitter::new(&module, ());
        assert_eq!(em.fresh(), "%t0");
        assert_eq!(em.fresh(), "%t1");
        assert_eq!(em.fresh_label("br"), "br2");
    }

    #[test]
    fn dedicated_labels_do_not_consume_temps() {
        let module = empty_module();
        let mut em = Emitter::new_dedicated_labels(&module, ());
        assert_eq!(em.fresh(), "%t0");
        assert_eq!(em.fresh_label("br"), "br0");
        assert_eq!(em.fresh(), "%t1");
    }

    #[test]
    fn terminator_detects_ret_and_br() {
        let module = empty_module();
        let mut em = Emitter::new(&module, ());
        em.body = "  br label %x\n".into();
        assert!(em.body_ends_with_terminator());
        em.body = "  %t0 = add i32 1, 2\n".into();
        assert!(!em.body_ends_with_terminator());
        em.body = "  ret i32 0\n".into();
        assert!(em.body_ends_with_terminator());
    }

    #[test]
    fn finish_returns_out_buffer() {
        let module = empty_module();
        let mut em = Emitter::new(&module, ());
        em.out = "hello".into();
        assert_eq!(em.finish(), "hello");
    }

    #[test]
    fn escape_llvm_bytes_quotes_and_nuls() {
        assert_eq!(escape_llvm_bytes(b"a\"b"), "a\\22b");
        assert_eq!(escape_llvm_bytes(&[0]), "\\00");
    }

    #[test]
    fn escape_llvm_string_matches_bytes() {
        assert_eq!(escape_llvm_string("a\"b"), escape_llvm_bytes(b"a\"b"));
        assert_eq!(escape_llvm_string("\0"), "\\00");
        assert_eq!(escape_llvm_string("\\"), "\\\\");
        assert_eq!(escape_llvm_string("~"), "~");
    }
}
