//! Shared IR lowered from checked Programs (ROADMAP B06).

pub use draconic_ast::BindingKind;
pub use draconic_check::{NativeType, ObjectShape, SymbolId as LocalId, Type as IrType};

mod dump;
mod expr;
mod lower;
mod module;
mod stmt;

pub use expr::{
    Arg, ArrayElement, ArrayPatternEl, AssignTarget, Expr, ObjectPatternEl, ObjectProp,
    ObjectPropKey, Param, Pattern, UpdateTarget,
};
pub use module::{Local, Module};
pub use stmt::{Stmt, SwitchCase};

pub use lower::lower;

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_ast::{BinaryOp, UpdateOp};
    use draconic_check::{check, NativeType, Type};
    use draconic_parser::parse;

    use crate::dump::dump_module;
    use crate::lower::lower_class_element::{params_are_simple, with_use_strict};

    fn lower_src(src: &str) -> Module {
        let program = parse(src).unwrap();
        let checked = check(program).unwrap();
        lower(&checked)
    }

    fn local_by_name<'a>(module: &'a Module, name: &str) -> &'a Local {
        module
            .locals
            .iter()
            .find(|l| l.name == name)
            .unwrap_or_else(|| panic!("no local `{name}`"))
    }

    #[test]
    fn lower_let_number_declares_typed_local() {
        let module = lower_src("let x = 1;");
        let x = local_by_name(&module, "x");
        assert_eq!(x.ty, Type::Number);
        assert_eq!(module.body.len(), 1);
        match &module.body[0] {
            Stmt::Declare {
                local,
                init: Some(Expr::Number { raw, ty }),
                kind,
            } => {
                assert_eq!(*local, x.id);
                assert_eq!(raw, "1");
                assert_eq!(*ty, Type::Number);
                assert_eq!(*kind, BindingKind::Let);
            }
            other => panic!("unexpected body: {other:?}"),
        }
    }

    #[test]
    fn lower_resolves_ident_to_local() {
        let module = lower_src("let x = 1; x;");
        let x = local_by_name(&module, "x");
        assert_eq!(module.body.len(), 2);
        match &module.body[1] {
            Stmt::Expr {
                expr: Expr::Local { id, ty },
            } => {
                assert_eq!(*id, x.id);
                assert_eq!(*ty, Type::Number);
            }
            other => panic!("unexpected body: {other:?}"),
        }
    }

    #[test]
    fn lower_binary_preserves_result_type() {
        let module = lower_src("let a = 1 + 2;");
        match &module.body[0] {
            Stmt::Declare {
                init:
                    Some(Expr::Binary {
                        op,
                        ty,
                        left,
                        right,
                    }),
                ..
            } => {
                assert_eq!(*op, BinaryOp::Add);
                assert_eq!(*ty, Type::Number);
                assert_eq!(left.ty(), Type::Number);
                assert_eq!(right.ty(), Type::Number);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn lower_string_concat_type() {
        let module = lower_src(r#"let s = "a" + "b";"#);
        assert_eq!(local_by_name(&module, "s").ty, Type::String);
        match &module.body[0] {
            Stmt::Declare {
                init: Some(Expr::Binary { ty, .. }),
                ..
            } => assert_eq!(*ty, Type::String),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn lower_strips_parens() {
        let module = lower_src("let x = (1);");
        match &module.body[0] {
            Stmt::Declare {
                init: Some(Expr::Number { raw, .. }),
                ..
            } => assert_eq!(raw, "1"),
            other => panic!("parens should be stripped: {other:?}"),
        }
    }

    #[test]
    fn lower_drops_empty_statements() {
        let module = lower_src("let x = 1;;;");
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Stmt::Declare { .. }));
    }

    #[test]
    fn lower_uninitialized_let() {
        let module = lower_src("let x;");
        assert_eq!(local_by_name(&module, "x").ty, Type::Any);
        match &module.body[0] {
            Stmt::Declare { init: None, .. } => {}
            other => panic!("expected bare declare: {other:?}"),
        }
    }

    #[test]
    fn lower_unary_and_literals() {
        let module = lower_src(r#"let a = -1; let b = !false; let c = null; let d = true;"#);
        assert_eq!(local_by_name(&module, "a").ty, Type::Number);
        assert_eq!(local_by_name(&module, "b").ty, Type::Boolean);
        assert_eq!(local_by_name(&module, "c").ty, Type::Null);
        assert_eq!(local_by_name(&module, "d").ty, Type::Boolean);
    }

    #[test]
    fn lower_propagates_binding_through_use() {
        let module = lower_src("let x = 1; let y = x + 2;");
        let x = local_by_name(&module, "x");
        match &module.body[1] {
            Stmt::Declare {
                init:
                    Some(Expr::Binary {
                        left: box_left, ty, ..
                    }),
                ..
            } => {
                assert_eq!(*ty, Type::Number);
                match box_left.as_ref() {
                    Expr::Local { id, ty } => {
                        assert_eq!(*id, x.id);
                        assert_eq!(*ty, Type::Number);
                    }
                    other => panic!("expected local x: {other:?}"),
                }
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn dump_module_stable() {
        let module = lower_src("let x = 1; x;");
        let dump = dump_module(&module);
        // Host global list grows with E/H/L work; assert shape + user binding, not a full golden.
        assert!(dump.starts_with("Module\n  locals:\n"), "got:\n{dump}");
        assert!(dump.contains(" x: number\n"), "got:\n{dump}");
        assert!(dump.contains("  body:\n"), "got:\n{dump}");
        assert!(dump.contains("Number 1 : number\n"), "got:\n{dump}");
        let x = local_by_name(&module, "x");
        assert!(
            dump.contains(&format!("Declare let %{}\n", x.id.0)),
            "got:\n{dump}"
        );
        assert!(
            dump.contains(&format!("Local %{} : number\n", x.id.0)),
            "got:\n{dump}"
        );
        assert!(!module.has_extern_ffi);
    }

    #[test]
    fn lower_call_shape() {
        // `any` is callable on the minimal surface; use uninit binding.
        let module = lower_src("let f; f(1);");
        match &module.body[1] {
            Stmt::Expr {
                expr:
                    Expr::Call {
                        callee,
                        args,
                        optional,
                        ty,
                    },
            } => {
                assert_eq!(*ty, Type::Any);
                assert!(!*optional);
                assert!(matches!(callee.as_ref(), Expr::Local { .. }));
                assert_eq!(args.len(), 1);
                assert!(matches!(args[0], Arg::Expr(Expr::Number { .. })));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn lower_update_prefix_and_postfix() {
        let module = lower_src("let x = 1; ++x; x++;");
        let x = local_by_name(&module, "x");
        match &module.body[1] {
            Stmt::Expr {
                expr:
                    Expr::Update {
                        op,
                        target,
                        prefix,
                        ty,
                    },
            } => {
                assert_eq!(*op, UpdateOp::Inc);
                assert_eq!(*target, UpdateTarget::Local(x.id));
                assert!(*prefix);
                assert_eq!(*ty, Type::Number);
            }
            other => panic!("unexpected prefix: {other:?}"),
        }
        match &module.body[2] {
            Stmt::Expr {
                expr:
                    Expr::Update {
                        op,
                        target,
                        prefix,
                        ty,
                    },
            } => {
                assert_eq!(*op, UpdateOp::Inc);
                assert_eq!(*target, UpdateTarget::Local(x.id));
                assert!(!*prefix);
                assert_eq!(*ty, Type::Number);
            }
            other => panic!("unexpected postfix: {other:?}"),
        }
    }

    #[test]
    fn lower_update_on_member() {
        let module = lower_src("let o = { x: 1 }; o.x++; ++o[\"x\"];");
        match &module.body[1] {
            Stmt::Expr {
                expr:
                    Expr::Update {
                        op,
                        target: UpdateTarget::Member { computed, .. },
                        prefix,
                        ty,
                    },
            } => {
                assert_eq!(*op, UpdateOp::Inc);
                assert!(!*computed);
                assert!(!*prefix);
                assert_eq!(*ty, Type::Number);
            }
            other => panic!("unexpected member postfix: {other:?}"),
        }
        match &module.body[2] {
            Stmt::Expr {
                expr:
                    Expr::Update {
                        target: UpdateTarget::Member { computed, .. },
                        prefix,
                        ..
                    },
            } => {
                assert!(*computed);
                assert!(*prefix);
            }
            other => panic!("unexpected member prefix: {other:?}"),
        }
    }

    /// Repeated / nested lower must not share private-field bookkeeping (issues-15).
    #[test]
    fn lower_private_field_state_isolated_across_calls() {
        let src_a = r#"
            class A {
                #x = 1;
                getX() { return this.#x; }
            }
        "#;
        let src_b = r#"
            class B {
                #y = 2;
                getY() { return this.#y; }
            }
        "#;

        let a1 = lower_src(src_a);
        let b = lower_src(src_b);
        let a2 = lower_src(src_a);

        let wm_a1: Vec<_> = a1
            .locals
            .iter()
            .filter(|l| l.name.contains("__drac_pf_"))
            .map(|l| l.name.as_str())
            .collect();
        let wm_b: Vec<_> = b
            .locals
            .iter()
            .filter(|l| l.name.contains("__drac_pf_"))
            .map(|l| l.name.as_str())
            .collect();
        let wm_a2: Vec<_> = a2
            .locals
            .iter()
            .filter(|l| l.name.contains("__drac_pf_"))
            .map(|l| l.name.as_str())
            .collect();

        assert_eq!(
            wm_a1.len(),
            1,
            "A should allocate one private-field WeakMap"
        );
        assert_eq!(wm_b.len(), 1, "B should allocate one private-field WeakMap");
        assert_eq!(wm_a2.len(), 1, "second A lower should allocate one WeakMap");
        assert!(
            wm_a1[0].contains("x"),
            "A WeakMap name should mention x: {}",
            wm_a1[0]
        );
        assert!(
            wm_b[0].contains("y"),
            "B WeakMap name should mention y: {}",
            wm_b[0]
        );
        assert_ne!(wm_a1[0], wm_b[0]);
        assert_eq!(wm_a1[0], wm_a2[0]);
        assert_eq!(dump_module(&a1), dump_module(&a2));
    }

    #[test]
    fn lower_nested_classes_private_fields_do_not_clobber() {
        let module = lower_src(
            r#"
            class Outer {
                #o = 1;
                inner() {
                    class Inner {
                        #i = 2;
                        getI() { return this.#i; }
                    }
                    return new Inner();
                }
                getO() { return this.#o; }
            }
        "#,
        );
        let pfs: Vec<_> = module
            .locals
            .iter()
            .filter(|l| l.name.starts_with("__drac_pf_"))
            .map(|l| l.name.as_str())
            .collect();
        assert_eq!(pfs.len(), 2, "outer and inner each need a WeakMap: {pfs:?}");
        assert!(pfs.iter().any(|n| n.contains("o")), "{pfs:?}");
        assert!(pfs.iter().any(|n| n.contains("i")), "{pfs:?}");
    }

    /// E19.82.07: nested private field shadows outer private method of same name.
    #[test]
    fn lower_nested_private_field_shadows_outer_method() {
        let module = lower_src(
            r#"
            class C {
                #m() { return "outer"; }
                outer() { return this.#m(); }
                B = class {
                    #m = "inner";
                    read(o) { return o.#m; }
                };
            }
        "#,
        );
        let dump = dump_module(&module);
        // Inner field needs a WeakMap; outer method needs a brand WeakSet / fn local.
        assert!(
            dump.contains("__drac_pf_") && dump.contains("__drac_pm_"),
            "expected both private field WeakMap and private method fn: {dump}"
        );
        assert!(
            !dump.contains("unknown private"),
            "shadowed private must resolve"
        );
    }

    /// E19.36: nested class body may read outer private names (no IR panic).
    #[test]
    fn lower_nested_class_accesses_outer_private() {
        let module = lower_src(
            r#"
            class C {
                #outer = 1;
                m() {
                    class D {
                        n(o) { return o.#outer; }
                    }
                    return new D().n(this);
                }
            }
        "#,
        );
        let dump = dump_module(&module);
        assert!(
            dump.contains("__drac_pf_") || dump.contains("WeakMap"),
            "expected private field WeakMap in dump"
        );
        assert!(
            !dump.contains("unknown private"),
            "nested access must resolve outer private"
        );
    }

    /// E19.36: compound / logical assign on private fields lower without panic.
    #[test]
    fn lower_private_compound_and_logical_assign() {
        let _ = lower_src(
            r#"
            class C {
                #x = 1;
                #y = 0;
                #z;
                step() {
                    this.#x += 2;
                    this.#y ||= 5;
                    this.#z ??= 9;
                    return this.#x + this.#y + this.#z;
                }
            }
        "#,
        );
        let _ = lower_src(
            r#"
            class C {
                #n = 0;
                inc() { return ++this.#n; }
                post() { return this.#n++; }
            }
        "#,
        );
    }

    /// E19.82.08: direct eval string with private access lowers via WeakMap, not raw `#`.
    #[test]
    fn lower_direct_eval_private_field_rewrites() {
        let module = lower_src(
            r#"
            class C {
                #m = 44;
                getWithEval() { return eval("this.#m"); }
            }
        "#,
        );
        let dump = dump_module(&module);
        assert!(
            dump.contains("__drac_pf_") || dump.contains("WeakMap"),
            "expected private field desugar in dump: {dump}"
        );
        // The eval string must not remain as a runtime `#` private access.
        assert!(
            !dump.contains("this.#m") && !dump.contains("\"this.#m\""),
            "eval private access should be inlined/desugared, dump: {dump}"
        );
    }

    #[test]
    fn params_are_simple_classifies_param_lists() {
        let p = |pat: Pattern, default: Option<Expr>, rest: bool| Param {
            pattern: pat,
            default,
            rest,
        };
        let id = |n: u32| Pattern::Local(LocalId(n));
        let num = || Expr::Number {
            raw: "1".into(),
            ty: Type::Number,
        };
        assert!(params_are_simple(&[p(id(0), None, false)]));
        assert!(params_are_simple(&[
            p(id(0), None, false),
            p(id(1), None, false)
        ]));
        assert!(
            !params_are_simple(&[p(id(0), Some(num()), false)]),
            "default param"
        );
        assert!(!params_are_simple(&[p(id(0), None, true)]), "rest param");
        assert!(
            !params_are_simple(&[p(Pattern::Array(vec![]), None, false)]),
            "destructured param"
        );
        assert!(
            !params_are_simple(&[p(Pattern::Object(vec![]), None, false)]),
            "object pattern param"
        );
    }

    #[test]
    fn with_use_strict_skips_directive_for_non_simple_params() {
        let id = || LocalId(0);
        let body = vec![Stmt::Declare {
            local: id(),
            init: None,
            kind: BindingKind::Let,
        }];
        let num = || Expr::Number {
            raw: "1".into(),
            ty: Type::Number,
        };
        let simple = [Param {
            pattern: Pattern::Local(id()),
            default: None,
            rest: false,
        }];
        let non_simple = [Param {
            pattern: Pattern::Local(id()),
            default: Some(num()),
            rest: false,
        }];
        let simple_body = with_use_strict(&simple, body.clone());
        assert!(
            matches!(&simple_body[0], Stmt::Expr { expr: Expr::String { value, .. } } if value.to_string_lossy() == "use strict"),
            "simple params: directive must be injected first: {simple_body:?}"
        );
        let non_simple_body = with_use_strict(&non_simple, body.clone());
        assert!(
            !matches!(&non_simple_body[0], Stmt::Expr { expr: Expr::String { value, .. } } if value.to_string_lossy() == "use strict"),
            "non-simple params: directive must be skipped (E19.87): {non_simple_body:?}"
        );
    }

    #[test]
    fn class_method_default_param_emits_no_directive_in_method() {
        // E19.87: a method with a default (non-simple) parameter list must not
        // carry a "use strict" directive inside its own body (SyntaxError).
        // `class C { m(a = 1) { … } }` lowers to: strict class-builder IIFE + strict
        // constructor (both simple-param) and the method (non-simple, no directive).
        let module = lower_src("class C { m(a = 1) { return a; } }");
        let dump = dump_module(&module);
        let directives = dump.matches("String \"use strict\"").count();
        assert_eq!(
            directives, 2,
            "expected directives only in class-builder IIFE + ctor (simple params), got {directives}: {dump}"
        );
    }

    // --- F06.03: extern "C" → IR/ABI surface ---

    #[test]
    fn lower_extern_c_function_abi_surface() {
        let module = lower_src(
            r#"
            extern "C" function add(a: i32, b: i32): i32;
            extern "C" function puts(s: *u8): i32;
            extern "C" function free(p: *u8): void;
            extern "C" function quit();
            "#,
        );
        assert!(module.has_extern_ffi);
        let mut externs = module.body.iter().filter_map(|s| match s {
            Stmt::ExternFunction {
                local,
                abi,
                name,
                params,
                ret,
            } => Some((
                local,
                abi.as_str(),
                name.as_str(),
                params.as_slice(),
                ret.as_ref(),
            )),
            _ => None,
        });
        let (add_id, abi, name, params, ret) = externs.next().expect("add");
        assert_eq!(abi, "C");
        assert_eq!(name, "add");
        assert_eq!(local_by_name(&module, "add").id, *add_id);
        assert_eq!(
            params,
            &[Type::Native(NativeType::I32), Type::Native(NativeType::I32)]
        );
        assert_eq!(ret, Some(&Type::Native(NativeType::I32)));

        let (_, _, name, params, ret) = externs.next().expect("puts");
        assert_eq!(name, "puts");
        assert_eq!(params, &[Type::Ptr(NativeType::U8)]);
        assert_eq!(ret, Some(&Type::Native(NativeType::I32)));

        let (_, _, name, params, ret) = externs.next().expect("free");
        assert_eq!(name, "free");
        assert_eq!(params, &[Type::Ptr(NativeType::U8)]);
        assert_eq!(ret, None);

        let (_, _, name, params, ret) = externs.next().expect("quit");
        assert_eq!(name, "quit");
        assert!(params.is_empty());
        assert_eq!(ret, None);
        assert!(externs.next().is_none());

        let dump = dump_module(&module);
        assert!(dump.contains("ExternFunction"), "got:\n{dump}");
        assert!(dump.contains("name=add"), "got:\n{dump}");
        assert!(dump.contains("ret: void"), "got:\n{dump}");
    }

    #[test]
    fn lower_extern_with_call_keeps_abi_and_call() {
        let module = lower_src(
            r#"
            extern "C" function add(a: i32, b: i32): i32;
            let s: i32 = add(20, 22);
            "#,
        );
        assert!(module.has_extern_ffi);
        assert!(
            module
                .body
                .iter()
                .any(|s| matches!(s, Stmt::ExternFunction { name, .. } if name == "add")),
            "expected ExternFunction add: {:?}",
            module.body
        );
        assert!(
            module.body.iter().any(|s| matches!(
                s,
                Stmt::Declare {
                    init: Some(Expr::Call { .. }),
                    ..
                }
            )),
            "expected call in declare: {:?}",
            module.body
        );
    }

    #[test]
    fn lower_extern_function_pointer_param() {
        let module = lower_src(
            r#"
            function twice(x: i32): i32 {
              return x + x;
            }
            extern "C" function draconic_rt_fnptr_nonnull(cb: function): i32;
            let ok: i32 = draconic_rt_fnptr_nonnull(twice);
            "#,
        );
        assert!(module.has_extern_ffi);
        let ext = module.body.iter().find_map(|s| match s {
            Stmt::ExternFunction { name, params, .. } if name == "draconic_rt_fnptr_nonnull" => {
                Some(params.as_slice())
            }
            _ => None,
        });
        assert_eq!(ext, Some(&[Type::Function][..]));
        assert!(
            module.body.iter().any(|s| matches!(
                s,
                Stmt::Declare {
                    init: Some(Expr::Call { .. }),
                    ..
                }
            )),
            "expected call passing fn: {:?}",
            module.body
        );
    }

    #[test]
    fn lower_extern_native_layout_struct_abi() {
        let module = lower_src(
            r#"
            type Pair = { a: i32; b: i64 };
            extern "C" function take(p: Pair): i32;
            extern "C" function make(a: i32, b: i64): Pair;
            "#,
        );
        assert!(module.has_extern_ffi);
        let take = module.body.iter().find_map(|s| match s {
            Stmt::ExternFunction {
                name, params, ret, ..
            } if name == "take" => Some((params.as_slice(), ret.as_ref())),
            _ => None,
        });
        let (params, ret) = take.expect("take");
        assert!(
            matches!(params, [Type::Shape(_)]),
            "take param should be native layout shape, got {params:?}"
        );
        assert_eq!(ret, Some(&Type::Native(NativeType::I32)));
        let make = module.body.iter().find_map(|s| match s {
            Stmt::ExternFunction {
                name, params, ret, ..
            } if name == "make" => Some((params.as_slice(), ret.as_ref())),
            _ => None,
        });
        let (params, ret) = make.expect("make");
        assert_eq!(
            params,
            &[Type::Native(NativeType::I32), Type::Native(NativeType::I64)]
        );
        assert!(
            matches!(ret, Some(Type::Shape(_))),
            "make return should be native layout shape, got {ret:?}"
        );
    }
}
