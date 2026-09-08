use draconic_ast::{Expr as AstExpr, Stmt as AstStmt};
use draconic_check::{CheckedProgram, Type};

use crate::lower::LowerCtx;
use crate::{Arg, Expr, LocalId, ObjectProp, ObjectPropKey, Stmt};

use crate::lower::lower_fn_body;
use crate::lower_class_element::{local_expr, lower_object_key_name_expr, with_use_strict};
use crate::lower_class_prop::{
    create_data_property_or_throw, member_prop, object_key_private_name, set_function_name_on_expr,
};
use crate::lower_expr::lower_expr_hint;
use crate::lower_private::{call_method_with_home, private_field_add};

pub(crate) enum StaticInit<'a> {
    Field {
        key: &'a draconic_ast::ObjectKey,
        value: Option<&'a AstExpr>,
        is_private: bool,
        /// Public computed key temp evaluated in source order (E19.82.04).
        computed_key: Option<LocalId>,
    },
    Block(&'a AstStmt),
}

/// Static fields and static blocks run after the class is fully linked, in order (E18.41).
/// Initializers run as `function(){ return <init>; }.call(Class)` so `this` / direct
/// eval see the constructor (E19.82.04). Arrows inside capture that this.
pub(crate) fn emit_static_inits(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    local: LocalId,
    static_inits: Vec<StaticInit<'_>>,
    parent_expr: Option<Expr>,
    out: &mut Vec<Stmt>,
) {
    for init in static_inits {
        match init {
            StaticInit::Field {
                key: fkey,
                value,
                is_private,
                computed_key,
            } => {
                let name_hint = field_name_hint(fkey, is_private);
                // Always method HomeObject: this = constructor; SuperProperty + field-init
                // direct eval (E19.82.04 / E19.82.05 / E19.82.06).
                let prev_field_init = ctx.in_field_init;
                ctx.in_field_init = true;
                let init_body = match value {
                    Some(v) => lower_field_init_expr(checked, ctx, v, None, name_hint.as_deref()),
                    None => Expr::IdentName {
                        name: "undefined".into(),
                        ty: Type::Any,
                    },
                };
                ctx.in_field_init = prev_field_init;
                let class_ref = Expr::Local {
                    id: local,
                    ty: Type::Function,
                };
                let home_proto = match parent_expr.as_ref() {
                    Some(p) => p.clone(),
                    None => member_prop(
                        Expr::IdentName {
                            name: "Function".into(),
                            ty: Type::Function,
                        },
                        "prototype",
                        Type::Any,
                    ),
                };
                let init_expr = call_method_with_home(
                    home_proto,
                    vec![Stmt::Return {
                        value: Some(init_body),
                    }],
                    class_ref.clone(),
                );
                if is_private {
                    let pname = object_key_private_name(fkey).expect("static private field name");
                    let wm = *ctx
                        .private_fields
                        .get(pname)
                        .expect("static private field WeakMap");
                    // PrivateFieldAdd on constructor (E19.82.09).
                    out.push(Stmt::Expr {
                        expr: private_field_add(
                            ctx,
                            wm,
                            Expr::Local {
                                id: local,
                                ty: Type::Function,
                            },
                            init_expr,
                        ),
                    });
                } else {
                    // CreateDataPropertyOrThrow — TypeError on non-writable prototype (E19.82.04).
                    let key_expr = if let Some(key_id) = computed_key {
                        local_expr(key_id)
                    } else {
                        lower_object_key_name_expr(checked, ctx, fkey, None)
                    };
                    out.push(Stmt::Expr {
                        expr: create_data_property_or_throw(
                            Expr::Local {
                                id: local,
                                ty: Type::Function,
                            },
                            key_expr,
                            init_expr,
                        ),
                    });
                }
            }
            StaticInit::Block(body) => {
                // Method-form on home with correct super base so `super.x` works (E19.72).
                // `({ __proto__: Parent, __sb() { … } }).__sb.call(Class)`
                let prev_object_super = ctx.object_super;
                ctx.object_super = true;
                let block_body = with_use_strict(&[], lower_fn_body(checked, ctx, body, None));
                ctx.object_super = prev_object_super;
                let method_fn = Expr::Function {
                    name: None,
                    params: Vec::new(),
                    body: block_body,
                    is_async: false,
                    is_generator: false,
                    is_arrow: false,
                    is_method: true,
                    ty: Type::Function,
                };
                let home_proto = match parent_expr.as_ref() {
                    Some(p) => p.clone(),
                    None => member_prop(
                        Expr::IdentName {
                            name: "Function".into(),
                            ty: Type::Function,
                        },
                        "prototype",
                        Type::Any,
                    ),
                };
                let home = Expr::Object {
                    properties: vec![
                        ObjectProp::Property {
                            key: ObjectPropKey::Static("__proto__".into()),
                            value: home_proto,
                        },
                        ObjectProp::Property {
                            key: ObjectPropKey::Static("__sb".into()),
                            value: method_fn,
                        },
                    ],
                    ty: Type::Object,
                };
                out.push(Stmt::Expr {
                    expr: Expr::Call {
                        callee: Box::new(member_prop(
                            member_prop(home, "__sb", Type::Function),
                            "call",
                            Type::Function,
                        )),
                        args: vec![Arg::Expr(Expr::Local {
                            id: local,
                            ty: Type::Function,
                        })],
                        optional: false,
                        ty: Type::Any,
                    },
                });
            }
        }
    }
}

/// NamedEvaluation name for a class field (`#x` for private) (E19.82.04).
pub(crate) fn field_name_hint(key: &draconic_ast::ObjectKey, is_private: bool) -> Option<String> {
    match key {
        draconic_ast::ObjectKey::Ident(id) => {
            if is_private {
                Some(format!("#{}", id.name))
            } else {
                Some(id.name.clone())
            }
        }
        draconic_ast::ObjectKey::String(s) => Some(s.value.to_string_lossy()),
        draconic_ast::ObjectKey::Computed(_) => None,
    }
}

/// ECMA-262 IsAnonymousFunctionDefinition (function/arrow only; classes use name_hint).
pub(crate) fn is_anonymous_function_def(expr: &AstExpr) -> bool {
    let mut e = expr;
    loop {
        match e {
            AstExpr::Paren { expr: inner, .. } | AstExpr::As { expr: inner, .. } => e = inner,
            AstExpr::FunctionExpression {
                name: None,
                is_method: false,
                ..
            } => return true,
            AstExpr::ArrowFunction { .. } => return true,
            _ => return false,
        }
    }
}

/// Lower a class field initializer with NamedEvaluation SetFunctionName (E19.82.04).
pub(crate) fn lower_field_init_expr(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    value: &AstExpr,
    super_class: Option<&AstExpr>,
    name_hint: Option<&str>,
) -> Expr {
    // Class expressions still use name_hint inside lower_expr_hint.
    let init = lower_expr_hint(checked, ctx, value, super_class, name_hint);
    if let Some(hint) = name_hint {
        if is_anonymous_function_def(value) {
            return set_function_name_on_expr(ctx, init, hint);
        }
    }
    init
}
