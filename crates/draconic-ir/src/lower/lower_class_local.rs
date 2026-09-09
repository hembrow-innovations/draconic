use std::collections::{HashMap, HashSet};

use draconic_ast::{
    AccessorKind, AssignOp, BinaryOp, ClassElement, Expr as AstExpr, Stmt as AstStmt,
};
use draconic_check::{CheckedProgram, Type};

use crate::lower::LowerCtx;
use crate::{Arg, AssignTarget, BindingKind, Expr, LocalId, ObjectProp, ObjectPropKey, Stmt};

use crate::lower::lower_fn_body;
use crate::lower::lower_class_element::{
    assert_derived_this, class_ctor_new_target_check, define_class_element_with_home, local_expr,
    lower_object_key_name_expr, lower_object_key_prop, object_set_prototype_of,
    possible_constructor_return, undef_expr, with_use_strict,
};
use crate::lower::lower_class_prop::{
    create_data_property_or_throw, data_prop_desc, heritage_validation_stmts, member_prop,
    object_key_private_name, object_method_call, parent_instance_super_base,
    set_function_name_stmt,
};
use crate::lower::lower_class_static::{
    emit_static_inits, field_name_hint, lower_field_init_expr, StaticInit,
};
use crate::lower::lower_expr::{lower_expr, lower_params};
use crate::lower::lower_private::{
    call_method_with_home, ensure_private_brand, private_brand_add, private_field_add,
};

pub(crate) fn lower_class_local(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    local: LocalId,
    super_class: Option<&AstExpr>,
    elements: &[ClassElement],
    // NamedEvaluation name for anonymous class expressions (E19.31).
    name_hint: Option<&str>,
) -> Vec<Stmt> {
    let mut ctor_params = Vec::new();
    let mut ctor_body_ast: Option<&AstStmt> = None;
    let mut methods: Vec<(
        &draconic_ast::ObjectKey,
        &Vec<draconic_ast::Param>,
        &AstStmt,
        bool,
        bool,
        bool,
        bool,
    )> = Vec::new();
    let mut accessors: Vec<(
        AccessorKind,
        &draconic_ast::ObjectKey,
        &Vec<draconic_ast::Param>,
        &AstStmt,
        bool,
        bool,
    )> = Vec::new();
    // Instance fields: (key, value, is_private, precomputed_key_local for public computed).
    let mut instance_fields: Vec<(
        &draconic_ast::ObjectKey,
        Option<&AstExpr>,
        bool,
        Option<LocalId>,
    )> = Vec::new();
    // Static fields and static blocks in source order (E18.41).
    let mut static_inits: Vec<StaticInit<'_>> = Vec::new();
    // Computed public field keys (instance + static) in source order — evaluated at
    // class definition before any field initializers (E19.82.04 intercalated keys).
    let mut computed_field_key_locals: Vec<(LocalId, Expr)> = Vec::new();

    for el in elements {
        match el {
            ClassElement::Constructor { params, body, .. } => {
                ctor_params = lower_params(checked, ctx, params, super_class);
                ctor_body_ast = Some(body.as_ref());
            }
            ClassElement::Method {
                key: method_key,
                params,
                body,
                is_static,
                is_async,
                is_generator,
                is_private,
                ..
            } => {
                methods.push((
                    method_key,
                    params,
                    body.as_ref(),
                    *is_static,
                    *is_async,
                    *is_generator,
                    *is_private,
                ));
            }
            ClassElement::Accessor {
                kind,
                key: acc_key,
                params,
                body,
                is_static,
                is_private,
                ..
            } => {
                accessors.push((
                    *kind,
                    acc_key,
                    params,
                    body.as_ref(),
                    *is_static,
                    *is_private,
                ));
            }
            ClassElement::Field {
                key: field_key,
                value,
                is_static,
                is_private,
                ..
            } => {
                let v = value.as_ref();
                // Public computed keys: ToPropertyKey at class eval, source order (E19.82.04).
                let computed_key = if !*is_private
                    && matches!(field_key, draconic_ast::ObjectKey::Computed(_))
                {
                    let key_id = ctx.alloc_synthetic_local(
                        format!("__drac_cfk_{}_{}", local.0, computed_field_key_locals.len()),
                        Type::Any,
                    );
                    let key_expr = lower_object_key_name_expr(checked, ctx, field_key, super_class);
                    // Reflect.ownKeys({[key]:1})[0] forces ToPropertyKey.
                    let to_key = Expr::Member {
                        object: Box::new(Expr::Call {
                            callee: Box::new(Expr::Member {
                                object: Box::new(Expr::IdentName {
                                    name: "Reflect".into(),
                                    ty: Type::Object,
                                }),
                                property: Box::new(Expr::String {
                                    value: "ownKeys".into(),
                                    ty: Type::String,
                                }),
                                computed: false,
                                optional: false,
                                ty: Type::Function,
                            }),
                            args: vec![Arg::Expr(Expr::Object {
                                properties: vec![ObjectProp::Property {
                                    key: ObjectPropKey::Computed(key_expr),
                                    value: Expr::Number {
                                        raw: "1".into(),
                                        ty: Type::Number,
                                    },
                                }],
                                ty: Type::Object,
                            })],
                            optional: false,
                            ty: Type::Any,
                        }),
                        property: Box::new(Expr::Number {
                            raw: "0".into(),
                            ty: Type::Number,
                        }),
                        computed: true,
                        optional: false,
                        ty: Type::Any,
                    };
                    computed_field_key_locals.push((key_id, to_key));
                    Some(key_id)
                } else {
                    None
                };
                if *is_static {
                    static_inits.push(StaticInit::Field {
                        key: field_key,
                        value: v,
                        is_private: *is_private,
                        computed_key,
                    });
                } else {
                    instance_fields.push((field_key, v, *is_private, computed_key));
                }
            }
            ClassElement::StaticBlock { body, .. } => {
                static_inits.push(StaticInit::Block(body.as_ref()));
            }
        }
    }

    // WeakMap per private field (E18.35 instance; E18.36 static — class as key).
    let mut private_map: HashMap<String, LocalId> = HashMap::new();
    let mut private_wm_decls: Vec<Stmt> = Vec::new();
    let mut add_private_wm = |fname: &str| {
        if private_map.contains_key(fname) {
            return;
        }
        let wm_name = format!("__drac_pf_{}_{}", local.0, fname);
        let wm_id = ctx.alloc_synthetic_local(wm_name, Type::Any);
        private_map.insert(fname.to_string(), wm_id);
        private_wm_decls.push(Stmt::Declare {
            local: wm_id,
            init: Some(Expr::New {
                callee: Box::new(Expr::IdentName {
                    name: "WeakMap".into(),
                    ty: Type::Function,
                }),
                args: Vec::new(),
                ty: Type::Any,
            }),
            kind: BindingKind::Let,
        });
    };
    for (fkey, _, is_private, _) in &instance_fields {
        if *is_private {
            if let Some(n) = object_key_private_name(fkey) {
                add_private_wm(n);
            }
        }
    }
    for init in &static_inits {
        if let StaticInit::Field {
            key: fkey,
            is_private,
            ..
        } = init
        {
            if *is_private {
                if let Some(n) = object_key_private_name(fkey) {
                    add_private_wm(n);
                }
            }
        }
    }

    // Private methods: synthetic function locals (E18.37 instance; E18.38 static). Bodies lowered after maps are live.
    let mut private_method_map: HashMap<String, LocalId> = HashMap::new();
    let mut private_method_meta: Vec<(
        LocalId,
        String,
        &Vec<draconic_ast::Param>,
        &AstStmt,
        bool,
        bool,
    )> = Vec::new();
    let mut private_brand_map: HashMap<String, LocalId> = HashMap::new();
    let mut private_brand_decls: Vec<Stmt> = Vec::new();
    let mut instance_brands: Vec<LocalId> = Vec::new();
    let mut static_brands: Vec<LocalId> = Vec::new();
    for (method_key, params, body, is_static, is_async, is_generator, is_private) in &methods {
        if !*is_private {
            continue;
        }
        let Some(method_name) = object_key_private_name(method_key) else {
            continue;
        };
        if private_method_map.contains_key(method_name) {
            continue;
        }
        let fn_name = format!("__drac_pm_{}_{}", local.0, method_name);
        let fn_id = ctx.alloc_synthetic_local(fn_name, Type::Function);
        private_method_map.insert(method_name.to_string(), fn_id);
        private_method_meta.push((
            fn_id,
            method_name.to_string(),
            params,
            body,
            *is_async,
            *is_generator,
        ));
        ensure_private_brand(
            ctx,
            local,
            &mut private_brand_map,
            &mut private_brand_decls,
            &mut instance_brands,
            &mut static_brands,
            method_name,
            *is_static,
        );
    }

    // Private accessors: synthetic get/set function locals (E18.39).
    let mut private_accessor_map: HashMap<String, (Option<LocalId>, Option<LocalId>)> =
        HashMap::new();
    let mut private_accessor_meta: Vec<(LocalId, String, &Vec<draconic_ast::Param>, &AstStmt)> =
        Vec::new();
    for (kind, acc_key, params, body, is_static, is_private) in &accessors {
        if !*is_private {
            continue;
        }
        let Some(acc_name) = object_key_private_name(acc_key) else {
            continue;
        };
        let entry = private_accessor_map
            .entry(acc_name.to_string())
            .or_insert((None, None));
        let tag = match kind {
            AccessorKind::Get => "g",
            AccessorKind::Set => "s",
        };
        let fn_name = format!("__drac_pa{}_{}_{}", tag, local.0, acc_name);
        let fn_id = ctx.alloc_synthetic_local(fn_name, Type::Function);
        match kind {
            AccessorKind::Get => entry.0 = Some(fn_id),
            AccessorKind::Set => entry.1 = Some(fn_id),
        }
        let display = match kind {
            AccessorKind::Get => format!("get #{acc_name}"),
            AccessorKind::Set => format!("set #{acc_name}"),
        };
        private_accessor_meta.push((fn_id, display, params, body));
        ensure_private_brand(
            ctx,
            local,
            &mut private_brand_map,
            &mut private_brand_decls,
            &mut instance_brands,
            &mut static_brands,
            acc_name,
            *is_static,
        );
    }

    // Nested classes inherit outer private names; inner same-name bindings fully
    // shadow any outer kind (field/method/accessor/brand) — E19.36 / E19.82.07.
    let prev_privates = ctx.private_fields.clone();
    let prev_private_methods = ctx.private_methods.clone();
    let prev_private_accessors = ctx.private_accessors.clone();
    let prev_private_brands = ctx.private_brands.clone();
    let mut shadowed: HashSet<String> = HashSet::new();
    shadowed.extend(private_map.keys().cloned());
    shadowed.extend(private_method_map.keys().cloned());
    shadowed.extend(private_accessor_map.keys().cloned());
    for name in &shadowed {
        ctx.private_fields.remove(name);
        ctx.private_methods.remove(name);
        ctx.private_accessors.remove(name);
        ctx.private_brands.remove(name);
    }
    for (k, v) in private_map {
        ctx.private_fields.insert(k, v);
    }
    for (k, v) in private_method_map {
        ctx.private_methods.insert(k, v);
    }
    for (k, v) in private_accessor_map {
        ctx.private_accessors.insert(k, v);
    }
    for (k, v) in private_brand_map {
        ctx.private_brands.insert(k, v);
    }

    let mut private_method_fns: Vec<Stmt> = Vec::new();
    for (fn_id, method_name, params, body, is_async, is_generator) in private_method_meta {
        private_method_fns.push(Stmt::Function {
            local: fn_id,
            params: lower_params(checked, ctx, params, super_class),
            body: lower_fn_body(checked, ctx, body, super_class),
            is_async,
            is_generator,
        });
        // SetFunctionName(closure, PrivateName) → "#description" (E19.82).
        private_method_fns.push(set_function_name_stmt(fn_id, &format!("#{method_name}")));
    }
    for (fn_id, display_name, params, body) in private_accessor_meta {
        private_method_fns.push(Stmt::Function {
            local: fn_id,
            params: lower_params(checked, ctx, params, super_class),
            body: lower_fn_body(checked, ctx, body, super_class),
            is_async: false,
            is_generator: false,
        });
        private_method_fns.push(set_function_name_stmt(fn_id, &display_name));
    }

    // Derived constructors use a TDZ `this` temp + Reflect.construct (E19.82.03).
    // Super expression is evaluated once at class def (not inside ctor) so TLA
    // `extends fn(await x)` keeps `await` at module top-level. Always bind the
    // heritage value so IsConstructor / prototype checks run once (E19.82.02).
    let is_derived = super_class.is_some();
    let default_derived_ctor = ctor_body_ast.is_none() && is_derived;
    let derived_this_id = if is_derived {
        Some(ctx.alloc_synthetic_local(format!("__drac_this_{}", local.0), Type::Any))
    } else {
        None
    };
    let super_local_id = if is_derived {
        Some(ctx.alloc_synthetic_local(format!("__drac_super_{}", local.0), Type::Any))
    } else {
        None
    };
    // Receiver for field/brand inits: derived `_this` temp or bare `this` (base).
    let ctor_this = || {
        if let Some(id) = derived_this_id {
            local_expr(id)
        } else {
            Expr::This { ty: Type::Any }
        }
    };

    // Instance field inits reference computed key temps allocated in source order above.
    let mut instance_init_exprs: Vec<Expr> = Vec::new();
    // Brands before fields (InitializeInstanceElements).
    for brand in &instance_brands {
        instance_init_exprs.push(private_brand_add(ctx, *brand, ctor_this()));
    }
    // Instance SuperProperty home base: Parent.prototype or Object.prototype (E19.82.05).
    let instance_super_home = match super_local_id {
        Some(sid) => parent_instance_super_base(local_expr(sid)),
        None => match super_class {
            Some(sc) => parent_instance_super_base(lower_expr(checked, ctx, sc, None)),
            None => member_prop(
                Expr::IdentName {
                    name: "Object".into(),
                    ty: Type::Function,
                },
                "prototype",
                Type::Any,
            ),
        },
    };
    for (fkey, value, is_private, computed_key) in &instance_fields {
        let name_hint = field_name_hint(fkey, *is_private);
        // Always method HomeObject so field-init direct eval gets SuperProperty /
        // new.target (E19.82.05 / E19.82.06). Clear derived temps so Super stays bare.
        let prev_derived_this = ctx.derived_this.take();
        let prev_derived_super = ctx.derived_super.take();
        let prev_field_init = ctx.in_field_init;
        ctx.in_field_init = true;
        let init = match value {
            Some(v) => lower_field_init_expr(checked, ctx, v, None, name_hint.as_deref()),
            None => undef_expr(),
        };
        ctx.in_field_init = prev_field_init;
        // Bare `this` inside method; .call(receiver) supplies the instance.
        let receiver = Expr::This { ty: Type::Any };
        let assign_expr = if *is_private {
            let pname = object_key_private_name(fkey).expect("private field name");
            let wm = *ctx
                .private_fields
                .get(pname)
                .expect("private field WeakMap");
            // PrivateFieldAdd: non-extensible / already-present → TypeError (E19.82.09).
            private_field_add(ctx, wm, receiver, init)
        } else if let Some(key_id) = computed_key {
            // CreateDataPropertyOrThrow — not [[Set]] (superclass setters must not run) (E19.82).
            create_data_property_or_throw(receiver, local_expr(*key_id), init)
        } else {
            let (prop, _computed) = lower_object_key_prop(checked, ctx, fkey, None);
            create_data_property_or_throw(receiver, prop, init)
        };
        let expr = call_method_with_home(
            instance_super_home.clone(),
            vec![Stmt::Expr { expr: assign_expr }],
            ctor_this(),
        );
        instance_init_exprs.push(expr);
        ctx.derived_this = prev_derived_this;
        ctx.derived_super = prev_derived_super;
    }
    // Clear derived ctx after field RHS; re-set for user ctor body below.
    ctx.derived_this = None;
    ctx.derived_super = None;
    ctx.derived_super_inits.clear();

    let ctor_body = if default_derived_ctor {
        let this_id = derived_this_id.expect("derived this temp");
        let super_id = super_local_id.expect("super temp");
        // `_this = Reflect.construct(__drac_super, arguments, new.target)` then inits.
        let reflect_construct = Expr::Call {
            callee: Box::new(Expr::Member {
                object: Box::new(Expr::IdentName {
                    name: "Reflect".into(),
                    ty: Type::Object,
                }),
                property: Box::new(Expr::String {
                    value: "construct".into(),
                    ty: Type::String,
                }),
                computed: false,
                optional: false,
                ty: Type::Function,
            }),
            args: vec![
                Arg::Expr(local_expr(super_id)),
                Arg::Expr(Expr::IdentName {
                    name: "arguments".into(),
                    ty: Type::Any,
                }),
                Arg::Expr(Expr::NewTarget { ty: Type::Any }),
            ],
            optional: false,
            ty: Type::Any,
        };
        let mut body = with_use_strict(&ctor_params, vec![class_ctor_new_target_check()]);
        body.push(Stmt::Declare {
            local: this_id,
            init: Some(reflect_construct),
            kind: BindingKind::Let,
        });
        for init in &instance_init_exprs {
            body.push(Stmt::Expr { expr: init.clone() });
        }
        body.push(Stmt::Return {
            value: Some(local_expr(this_id)),
        });
        body
    } else if is_derived {
        let this_id = derived_this_id.expect("derived this temp");
        let super_id = super_local_id.expect("super temp");
        // User-defined derived ctor: TDZ this, super() binds via Reflect.construct.
        // Return completion is deferred past user try/catch via labeled break (E19.82).
        let ret_mode_id = ctx.alloc_synthetic_local(format!("__drac_rm_{}", local.0), Type::Number);
        let ret_val_id = ctx.alloc_synthetic_local(format!("__drac_rv_{}", local.0), Type::Any);
        let ctor_label = format!("__drac_ctor_{}", local.0);
        ctx.derived_this = Some(this_id);
        ctx.derived_super = Some(super_id);
        ctx.derived_super_inits = instance_init_exprs.clone();
        ctx.derived_ctor_body = true;
        ctx.derived_ctor_label = Some(ctor_label.clone());
        ctx.derived_ret_mode = Some(ret_mode_id);
        ctx.derived_ret_val = Some(ret_val_id);
        let mut inner = Vec::new();
        if let Some(ast_body) = ctor_body_ast {
            inner.extend(lower_fn_body(checked, ctx, ast_body, super_class));
        }
        ctx.derived_this = None;
        ctx.derived_super = None;
        ctx.derived_super_inits.clear();
        ctx.derived_ctor_body = false;
        ctx.derived_ctor_label = None;
        ctx.derived_ret_mode = None;
        ctx.derived_ret_val = None;
        // Class constructors must be called with `new` (E19.82). Directive first (strict).
        let mut body = with_use_strict(&ctor_params, vec![class_ctor_new_target_check()]);
        body.push(Stmt::Declare {
            local: this_id,
            init: None,
            kind: BindingKind::Let,
        });
        body.push(Stmt::Declare {
            local: ret_mode_id,
            init: Some(Expr::Number {
                raw: "0".into(),
                ty: Type::Number,
            }),
            kind: BindingKind::Let,
        });
        body.push(Stmt::Declare {
            local: ret_val_id,
            init: Some(undef_expr()),
            kind: BindingKind::Let,
        });
        body.push(Stmt::Labeled {
            label: ctor_label,
            body: Box::new(Stmt::Block { body: inner }),
        });
        // After label: explicit return → [[Construct]] completion; else assert this.
        body.push(Stmt::Return {
            value: Some(Expr::Conditional {
                test: Box::new(Expr::Binary {
                    left: Box::new(local_expr(ret_mode_id)),
                    op: BinaryOp::EqEqEq,
                    right: Box::new(Expr::Number {
                        raw: "1".into(),
                        ty: Type::Number,
                    }),
                    ty: Type::Boolean,
                }),
                consequent: Box::new(possible_constructor_return(
                    this_id,
                    Some(local_expr(ret_val_id)),
                )),
                alternate: Box::new(assert_derived_this(this_id)),
                ty: Type::Any,
            }),
        });
        body
    } else {
        // Base class: bare `this`; field inits at start of ctor (after super N/A).
        let mut body = match ctor_body_ast {
            Some(ast_body) => lower_fn_body(checked, ctx, ast_body, super_class),
            None => Vec::new(),
        };
        if !instance_init_exprs.is_empty() {
            let mut new_body = Vec::with_capacity(body.len() + instance_init_exprs.len());
            for init in &instance_init_exprs {
                new_body.push(Stmt::Expr { expr: init.clone() });
            }
            new_body.extend(body);
            body = new_body;
        }
        // Class constructors: strict + must be called with `new` (E19.82).
        let mut with_check = with_use_strict(&ctor_params, vec![class_ctor_new_target_check()]);
        with_check.extend(body);
        with_check
    };

    let mut out = private_wm_decls;
    out.extend(private_brand_decls);
    out.extend(private_method_fns);
    // Evaluate extends once (TLA-safe) and validate IsConstructor + prototype (E19.82.02).
    // Cache Get(superclass,"prototype") once for linking (E19.82 prototype-getter).
    let mut cached_super_proto: Option<LocalId> = None;
    if let (Some(super_id), Some(sc)) = (super_local_id, super_class) {
        let parent = lower_expr(checked, ctx, sc, None);
        out.push(Stmt::Declare {
            local: super_id,
            init: Some(parent),
            kind: BindingKind::Let,
        });
        let proto_id = ctx.alloc_synthetic_local(format!("__drac_sproto_{}", local.0), Type::Any);
        out.push(Stmt::Declare {
            local: proto_id,
            init: None,
            kind: BindingKind::Let,
        });
        out.extend(heritage_validation_stmts(
            Expr::Local {
                id: super_id,
                ty: Type::Any,
            },
            Some(proto_id),
        ));
        cached_super_proto = Some(proto_id);
    }
    // Declare computed field key temps before constructor (referenced from ctor body).
    for (key_id, _) in &computed_field_key_locals {
        out.push(Stmt::Declare {
            local: *key_id,
            init: None,
            kind: BindingKind::Let,
        });
    }
    // E19.57: class name binding is immutable (const-like). Emit `const C = function…`
    // (anonymous FE so body refs resolve to outer const) so `C = …` is a runtime TypeError.
    out.push(Stmt::Declare {
        local,
        init: Some(Expr::Function {
            name: None,
            params: ctor_params,
            body: ctor_body,
            is_async: false,
            is_generator: false,
            is_arrow: false,
            is_method: false,
            ty: Type::Function,
        }),
        kind: BindingKind::Const,
    });
    // NamedEvaluation / class BindingIdentifier → constructor `.name` (E19.31 / E19.57).
    if let Some(hint) = name_hint {
        out.push(set_function_name_stmt(local, hint));
    } else if let Some(sym) = checked.bound.symbols().iter().find(|s| s.id == local) {
        if sym.name != "__class" {
            out.push(set_function_name_stmt(local, sym.name.as_str()));
        }
    }
    // Class constructors: `.prototype` is non-writable/non-enumerable/non-configurable (E19.82.05).
    {
        let proto = member_prop(
            Expr::Local {
                id: local,
                ty: Type::Function,
            },
            "prototype",
            Type::Any,
        );
        out.push(Stmt::Expr {
            expr: object_method_call(
                "defineProperty",
                vec![
                    Arg::Expr(Expr::Local {
                        id: local,
                        ty: Type::Function,
                    }),
                    Arg::Expr(Expr::String {
                        value: "prototype".into(),
                        ty: Type::String,
                    }),
                    Arg::Expr(data_prop_desc(proto, false, false, false)),
                ],
            ),
        });
    }
    // Evaluate computed instance field names at class definition time (E19.53).
    for (key_id, to_key) in computed_field_key_locals {
        out.push(Stmt::Expr {
            expr: Expr::Assign {
                target: AssignTarget::Local(key_id),
                op: AssignOp::Eq,
                value: Box::new(to_key),
                ty: Type::Any,
            },
        });
    }

    // Parent expression for heritage / method home-object super base (E19.72).
    let parent_expr = super_class.map(|sc| {
        if let Some(super_id) = super_local_id {
            Expr::Local {
                id: super_id,
                ty: Type::Any,
            }
        } else {
            lower_expr(checked, ctx, sc, None)
        }
    });

    for (method_key, params, body, is_static, is_async, is_generator, is_private) in methods {
        if is_private {
            // Already emitted as standalone function; not installed on prototype.
            continue;
        }
        // Keep Super + method form so [[HomeObject]] / eval('super…') work (E19.72).
        // SuperCall stays desugared only in constructors (super_class still passed there).
        let prev_object_super = ctx.object_super;
        ctx.object_super = true;
        let method_params = lower_params(checked, ctx, params, None);
        let method_body = with_use_strict(&method_params, lower_fn_body(checked, ctx, body, None));
        ctx.object_super = prev_object_super;
        let method_fn = Expr::Function {
            name: None,
            params: method_params,
            body: method_body,
            is_async,
            is_generator,
            is_arrow: false,
            is_method: true,
            ty: Type::Function,
        };
        let class_ref = Expr::Local {
            id: local,
            ty: Type::Function,
        };
        let target_object = if is_static {
            class_ref
        } else {
            member_prop(class_ref, "prototype", Type::Any)
        };
        // Key lowered once; define_class_element_with_home binds to temp (E19.78).
        let prop_name = lower_object_key_name_expr(checked, ctx, method_key, None);
        let home_proto = match (&parent_expr, is_static) {
            (Some(p), true) => Some(p.clone()),
            (Some(p), false) => Some(parent_instance_super_base(p.clone())),
            (None, true) => Some(member_prop(
                Expr::IdentName {
                    name: "Function".into(),
                    ty: Type::Function,
                },
                "prototype",
                Type::Any,
            )),
            (None, false) => None,
        };
        out.extend(define_class_element_with_home(
            ctx,
            target_object,
            prop_name,
            ObjectProp::Property {
                // Placeholder key — rewritten to the once-bound temp inside helper.
                key: ObjectPropKey::Static("".into()),
                value: method_fn,
            },
            home_proto,
        ));
    }

    for (kind, acc_key, params, body, is_static, is_private) in accessors {
        if is_private {
            // Already emitted as standalone function; not installed on prototype.
            continue;
        }
        let prev_object_super = ctx.object_super;
        ctx.object_super = true;
        let acc_params = lower_params(checked, ctx, params, None);
        let acc_body = with_use_strict(&acc_params, lower_fn_body(checked, ctx, body, None));
        ctx.object_super = prev_object_super;
        let accessor_fn = Expr::Function {
            name: None,
            params: acc_params,
            body: acc_body,
            is_async: false,
            is_generator: false,
            is_arrow: false,
            is_method: true,
            ty: Type::Function,
        };
        let class_ref = Expr::Local {
            id: local,
            ty: Type::Function,
        };
        let target_object = if is_static {
            class_ref
        } else {
            member_prop(class_ref, "prototype", Type::Any)
        };
        let prop_name = lower_object_key_name_expr(checked, ctx, acc_key, None);
        let home_proto = match (&parent_expr, is_static) {
            (Some(p), true) => Some(p.clone()),
            (Some(p), false) => Some(parent_instance_super_base(p.clone())),
            (None, true) => Some(member_prop(
                Expr::IdentName {
                    name: "Function".into(),
                    ty: Type::Function,
                },
                "prototype",
                Type::Any,
            )),
            (None, false) => None,
        };
        out.extend(define_class_element_with_home(
            ctx,
            target_object,
            prop_name,
            ObjectProp::Accessor {
                kind,
                key: ObjectPropKey::Static("".into()),
                value: accessor_fn,
            },
            home_proto,
        ));
    }

    if let Some(parent) = parent_expr.as_ref() {
        // extends null → instance [[Prototype]] is null; constructor [[Prototype]] is %FunctionPrototype%.
        // Reuse cached Get(superclass,"prototype") from heritage validation when present (E19.82).
        let parent_proto = if let Some(pid) = cached_super_proto {
            local_expr(pid)
        } else {
            parent_instance_super_base(parent.clone())
        };
        let child_proto = member_prop(
            Expr::Local {
                id: local,
                ty: Type::Function,
            },
            "prototype",
            Type::Any,
        );
        out.push(Stmt::Expr {
            expr: object_set_prototype_of(child_proto, parent_proto),
        });
        // F.[[Prototype]] = null heritage → %FunctionPrototype%; else superclass (E19.82).
        let ctor_parent = Expr::Conditional {
            test: Box::new(Expr::Binary {
                left: Box::new(parent.clone()),
                op: BinaryOp::EqEqEq,
                right: Box::new(Expr::Null { ty: Type::Any }),
                ty: Type::Boolean,
            }),
            consequent: Box::new(member_prop(
                Expr::IdentName {
                    name: "Function".into(),
                    ty: Type::Function,
                },
                "prototype",
                Type::Any,
            )),
            alternate: Box::new(parent.clone()),
            ty: Type::Any,
        };
        out.push(Stmt::Expr {
            expr: object_set_prototype_of(
                Expr::Local {
                    id: local,
                    ty: Type::Function,
                },
                ctor_parent,
            ),
        });
    }

    // Brand the constructor for static private methods/accessors (E18.40)
    // before static field/block evaluation so blocks can use private statics.
    for brand in static_brands {
        out.push(Stmt::Expr {
            expr: private_brand_add(
                ctx,
                brand,
                Expr::Local {
                    id: local,
                    ty: Type::Function,
                },
            ),
        });
    }

    // Static fields and static blocks run after the class is fully linked, in order (E18.41).
    emit_static_inits(checked, ctx, local, static_inits, parent_expr, &mut out);

    ctx.private_fields = prev_privates;
    ctx.private_methods = prev_private_methods;
    ctx.private_accessors = prev_private_accessors;
    ctx.private_brands = prev_private_brands;
    out
}
