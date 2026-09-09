use std::collections::HashMap;

use draconic_ast::{AssignOp, BindingPattern, Expr as AstExpr, Stmt as AstStmt};
use draconic_check::{CheckedProgram, NativeType, Type};
use draconic_diagnostics::Span;

mod lower_class;
pub(crate) mod lower_class_element;
mod lower_class_local;
mod lower_class_prop;
mod lower_class_static;
mod lower_eval;
mod lower_expr;
mod lower_expr_object;
mod lower_pattern;
mod lower_private;

use lower_class::lower_class;
use lower_class_element::{possible_constructor_return, undef_expr};
use lower_expr::{
    lower_array_pattern_els, lower_binding_pattern, lower_expr, lower_expr_hint,
    lower_object_pattern_props, lower_params,
};
use crate::{AssignTarget, BindingKind, Expr, Local, LocalId, Module, Stmt, SwitchCase};

/// Per-`lower` bookkeeping for private fields/methods/brands and synthetic locals.
/// Owned by `lower` for the duration of one lowering — no process-global state.
pub(crate) struct LowerCtx {
    /// Private field name → WeakMap local.
    pub(crate) private_fields: HashMap<String, LocalId>,
    /// Private method name → function local.
    pub(crate) private_methods: HashMap<String, LocalId>,
    /// Private accessor name → (get fn, set fn) locals.
    pub(crate) private_accessors: HashMap<String, (Option<LocalId>, Option<LocalId>)>,
    /// Private method/accessor brand → WeakSet local (E18.40; fields use WeakMap as brand).
    pub(crate) private_brands: HashMap<String, LocalId>,
    /// Inside object method/accessor: keep `super` for JS home-object emit (E19.23).
    pub(crate) object_super: bool,
    /// Derived class constructor: ES `this` binding temp (uninit until `super()`).
    /// Inherited by nested arrows (lexical this); cleared in nested non-arrow functions.
    pub(crate) derived_this: Option<LocalId>,
    /// Derived class constructor: heritage local for `Reflect.construct`.
    pub(crate) derived_super: Option<LocalId>,
    /// Side-effect exprs run once after `super()` binds this (fields/brands).
    pub(crate) derived_super_inits: Vec<Expr>,
    /// True only while lowering the constructor body itself (not nested arrows/fns).
    /// Gates [[Construct]] return completion wrapping (E19.82.03).
    pub(crate) derived_ctor_body: bool,
    /// Derived ctor: label + temps so return TypeError is thrown outside user try/catch (E19.82).
    /// `return v` → store v, break label; after label, run [[Construct]] completion.
    pub(crate) derived_ctor_label: Option<String>,
    pub(crate) derived_ret_mode: Option<LocalId>,
    pub(crate) derived_ret_val: Option<LocalId>,
    /// True while lowering a class field initializer expression (not nested fn bodies).
    /// Direct `eval` gets field-init early errors (E19.82.06 ContainsArguments).
    pub(crate) in_field_init: bool,
    /// Class declaration: outer mutable name → inner immutable const local (E19.57).
    /// Stack so nested class decls restore the outer remap.
    pub(crate) class_name_remap: Vec<(LocalId, LocalId)>,
    pub(crate) extra_locals: Vec<Local>,
    pub(crate) next_synth_id: u32,
}

impl LowerCtx {
    fn new(next_synth_id: u32) -> Self {
        Self {
            private_fields: HashMap::new(),
            private_methods: HashMap::new(),
            private_accessors: HashMap::new(),
            private_brands: HashMap::new(),
            object_super: false,
            derived_this: None,
            derived_super: None,
            derived_super_inits: Vec::new(),
            derived_ctor_body: false,
            derived_ctor_label: None,
            derived_ret_mode: None,
            derived_ret_val: None,
            in_field_init: false,
            class_name_remap: Vec::new(),
            extra_locals: Vec::new(),
            next_synth_id,
        }
    }

    pub(crate) fn alloc_synthetic_local(&mut self, name: String, ty: Type) -> LocalId {
        let id = LocalId(self.next_synth_id);
        self.next_synth_id += 1;
        self.extra_locals.push(Local {
            id,
            name,
            ty,
            kind: BindingKind::Let,
        });
        id
    }

    /// Map class declaration outer name to the inner immutable binding while lowering the body.
    pub(crate) fn map_class_name(&self, id: LocalId) -> LocalId {
        for &(outer, inner) in self.class_name_remap.iter().rev() {
            if outer == id {
                return inner;
            }
        }
        id
    }
}

/// Lower a checked Program to shared IR.
pub fn lower(checked: &CheckedProgram) -> Module {
    let mut locals: Vec<Local> = checked
        .bound
        .symbols()
        .iter()
        .map(|s| Local {
            id: s.id,
            name: s.name.clone(),
            ty: checked.type_of_symbol(s.id),
            kind: s.kind,
        })
        .collect();

    let max_id = locals.iter().map(|l| l.id.0).max().unwrap_or(0);
    let mut ctx = LowerCtx::new(max_id.saturating_add(1));

    let mut body = Vec::new();
    let mut body_spans = Vec::new();
    // F08.01: track erased extern decls so the JS backend can hard-error (N04 spirit).
    let mut has_extern_ffi = false;
    for stmt in &checked.bound.program.body {
        if matches!(stmt, AstStmt::ExternFunctionDeclaration { .. }) {
            has_extern_ffi = true;
        }
        let span = ast_stmt_span(stmt);
        let expanded = lower_stmt_expand(checked, &mut ctx, stmt, None);
        for s in expanded {
            body.push(s);
            body_spans.push(span);
        }
    }

    // Private compound/update temps are assigned without a prior Declare (sloppy-mode
    // globals historically). Hoist `var` so strict class methods can assign them (E19.72).
    // Only these prefixes: other synthetics are params (`__drac_o`) or already declared.
    let mut hoisted = Vec::new();
    let mut hoisted_spans = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    for local in &ctx.extra_locals {
        let n = local.name.as_str();
        if !(n.starts_with("__drac_pobj_")
            || n.starts_with("__drac_pval_")
            || n.starts_with("__drac_pnext_")
            || n.starts_with("__drac_pcur_")
            || n.starts_with("__drac_dstr_")
            || n.starts_with("__drac_sv_"))
        {
            continue;
        }
        if !seen_names.insert(local.name.clone()) {
            continue;
        }
        hoisted.push(Stmt::Declare {
            local: local.id,
            init: None,
            kind: BindingKind::Var,
        });
        hoisted_spans.push(Span::dummy());
    }
    if !hoisted.is_empty() {
        hoisted.append(&mut body);
        hoisted_spans.append(&mut body_spans);
        body = hoisted;
        body_spans = hoisted_spans;
    }

    locals.append(&mut ctx.extra_locals);

    debug_assert_eq!(body.len(), body_spans.len());
    Module {
        locals,
        body,
        body_spans,
        shapes: checked.shapes().to_vec(),
        has_extern_ffi,
    }
}

pub(crate) fn ast_stmt_span(stmt: &AstStmt) -> Span {
    match stmt {
        AstStmt::Expression { span, .. }
        | AstStmt::Let { span, .. }
        | AstStmt::Empty { span }
        | AstStmt::Block { span, .. }
        | AstStmt::If { span, .. }
        | AstStmt::While { span, .. }
        | AstStmt::DoWhile { span, .. }
        | AstStmt::For { span, .. }
        | AstStmt::ForIn { span, .. }
        | AstStmt::ForOf { span, .. }
        | AstStmt::Break { span, .. }
        | AstStmt::Continue { span, .. }
        | AstStmt::Labeled { span, .. }
        | AstStmt::Switch { span, .. }
        | AstStmt::FunctionDeclaration { span, .. }
        | AstStmt::ClassDeclaration { span, .. }
        | AstStmt::Return { span, .. }
        | AstStmt::Throw { span, .. }
        | AstStmt::Try { span, .. }
        | AstStmt::With { span, .. }
        | AstStmt::ImportDeclaration { span, .. }
        | AstStmt::ExportNamedDeclaration { span, .. }
        | AstStmt::ExportDefaultDeclaration { span, .. }
        | AstStmt::ExportAllDeclaration { span, .. }
        | AstStmt::TypeAlias { span, .. }
        | AstStmt::ExternFunctionDeclaration { span, .. } => *span,
    }
}

/// Lower one AST statement, expanding constructs that become multiple IR stmts (e.g. class).
pub(crate) fn lower_stmt_expand(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    stmt: &AstStmt,
    super_class: Option<&AstExpr>,
) -> Vec<Stmt> {
    match stmt {
        AstStmt::ClassDeclaration {
            name,
            super_class: sc,
            body,
            ..
        } => lower_class(checked, ctx, name, sc.as_deref(), body),
        other => lower_stmt(checked, ctx, other, super_class)
            .into_iter()
            .collect(),
    }
}

pub(crate) fn lower_stmt_body(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    body: &[AstStmt],
    super_class: Option<&AstExpr>,
) -> Vec<Stmt> {
    let mut out = Vec::new();
    for s in body {
        out.extend(lower_stmt_expand(checked, ctx, s, super_class));
    }
    out
}

pub(crate) fn lower_stmt(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    stmt: &AstStmt,
    super_class: Option<&AstExpr>,
) -> Option<Stmt> {
    match stmt {
        AstStmt::Empty { .. } => None,
        AstStmt::ClassDeclaration { .. } => {
            // Expanded via `lower_stmt_expand`.
            None
        }
        AstStmt::Expression { expr, .. } => match expr {
            AstExpr::ArrayPattern { elements, .. } => Some(Stmt::AssignLeft {
                target: AssignTarget::ArrayPattern {
                    elements: lower_array_pattern_els(checked, ctx, elements),
                },
            }),
            AstExpr::ObjectPattern { properties, .. } => Some(Stmt::AssignLeft {
                target: AssignTarget::ObjectPattern {
                    properties: lower_object_pattern_props(checked, ctx, properties),
                },
            }),
            _ => Some(Stmt::Expr {
                expr: lower_expr(checked, ctx, expr, super_class),
            }),
        },
        AstStmt::Let {
            kind,
            binding,
            init,
            ..
        } => match binding {
            BindingPattern::Ident(name) => {
                let local = checked
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == name.span)
                    .map(|s| s.id)
                    .expect("let binding must be declared");
                Some(Stmt::Declare {
                    local,
                    init: init.as_ref().map(|e| {
                        lower_expr_hint(checked, ctx, e, super_class, Some(name.name.as_str()))
                    }),
                    kind: *kind,
                })
            }
            BindingPattern::Array { elements, .. } => Some(Stmt::DeclareArrayPattern {
                kind: *kind,
                elements: lower_array_pattern_els(checked, ctx, elements),
                init: init
                    .as_ref()
                    .map(|e| lower_expr(checked, ctx, e, super_class)),
            }),
            BindingPattern::Object { properties, .. } => Some(Stmt::DeclareObjectPattern {
                kind: *kind,
                properties: lower_object_pattern_props(checked, ctx, properties),
                init: init
                    .as_ref()
                    .map(|e| lower_expr(checked, ctx, e, super_class)),
            }),
            BindingPattern::Member(_) => {
                panic!("member binding is assignment-only; rejected at check")
            }
        },
        AstStmt::Block { body, .. } => {
            let body = lower_stmt_body(checked, ctx, body, super_class);
            Some(Stmt::Block { body })
        }
        AstStmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            let consequent = Box::new(
                lower_stmt(checked, ctx, consequent, super_class)
                    .unwrap_or(Stmt::Block { body: vec![] }),
            );
            let alternate = alternate.as_ref().map(|alt| {
                Box::new(
                    lower_stmt(checked, ctx, alt, super_class)
                        .unwrap_or(Stmt::Block { body: vec![] }),
                )
            });
            Some(Stmt::If {
                test: lower_expr(checked, ctx, test, super_class),
                consequent,
                alternate,
            })
        }
        AstStmt::While { test, body, .. } => {
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class).unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::While {
                test: lower_expr(checked, ctx, test, super_class),
                body,
            })
        }
        AstStmt::DoWhile { body, test, .. } => {
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class).unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::DoWhile {
                body,
                test: lower_expr(checked, ctx, test, super_class),
            })
        }
        AstStmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            let init = init
                .as_ref()
                .and_then(|s| lower_stmt(checked, ctx, s, super_class).map(Box::new));
            let test = test
                .as_ref()
                .map(|e| lower_expr(checked, ctx, e, super_class));
            let update = update
                .as_ref()
                .map(|e| lower_expr(checked, ctx, e, super_class));
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class).unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::For {
                init,
                test,
                update,
                body,
            })
        }
        AstStmt::ForIn {
            left, right, body, ..
        } => {
            let left = Box::new(
                lower_stmt(checked, ctx, left, super_class).unwrap_or(Stmt::Block { body: vec![] }),
            );
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class).unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::ForIn {
                left,
                right: lower_expr(checked, ctx, right, super_class),
                body,
            })
        }
        AstStmt::ForOf {
            left,
            right,
            body,
            is_await,
            ..
        } => {
            let left = Box::new(
                lower_stmt(checked, ctx, left, super_class).unwrap_or(Stmt::Block { body: vec![] }),
            );
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class).unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::ForOf {
                left,
                right: lower_expr(checked, ctx, right, super_class),
                body,
                is_await: *is_await,
            })
        }
        AstStmt::Break { label, .. } => Some(Stmt::Break {
            label: label.as_ref().map(|l| l.name.clone()),
        }),
        AstStmt::Continue { label, .. } => Some(Stmt::Continue {
            label: label.as_ref().map(|l| l.name.clone()),
        }),
        AstStmt::Labeled { label, body, .. } => {
            let body = Box::new(
                lower_stmt(checked, ctx, body, super_class).unwrap_or(Stmt::Block { body: vec![] }),
            );
            Some(Stmt::Labeled {
                label: label.name.clone(),
                body,
            })
        }
        AstStmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            let cases = cases
                .iter()
                .map(|c| SwitchCase {
                    test: c
                        .test
                        .as_ref()
                        .map(|e| lower_expr(checked, ctx, e, super_class)),
                    body: lower_stmt_body(checked, ctx, &c.body, super_class),
                })
                .collect();
            Some(Stmt::Switch {
                discriminant: lower_expr(checked, ctx, discriminant, super_class),
                cases,
            })
        }
        AstStmt::FunctionDeclaration {
            name,
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            let local = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.span == name.span)
                .map(|s| s.id)
                .expect("function binding must be declared");
            let params = lower_params(checked, ctx, params, None);
            // Nested functions do not inherit `super` / derived ctor this (class parent or object home).
            let prev_object_super = ctx.object_super;
            let prev_derived_this = ctx.derived_this.take();
            let prev_derived_super = ctx.derived_super.take();
            let prev_inits = std::mem::take(&mut ctx.derived_super_inits);
            let prev_ctor_body = ctx.derived_ctor_body;
            let prev_ctor_label = ctx.derived_ctor_label.take();
            let prev_ret_mode = ctx.derived_ret_mode.take();
            let prev_ret_val = ctx.derived_ret_val.take();
            ctx.object_super = false;
            ctx.derived_ctor_body = false;
            let body = lower_fn_body(checked, ctx, body, None);
            ctx.object_super = prev_object_super;
            ctx.derived_this = prev_derived_this;
            ctx.derived_super = prev_derived_super;
            ctx.derived_super_inits = prev_inits;
            ctx.derived_ctor_body = prev_ctor_body;
            ctx.derived_ctor_label = prev_ctor_label;
            ctx.derived_ret_mode = prev_ret_mode;
            ctx.derived_ret_val = prev_ret_val;
            Some(Stmt::Function {
                local,
                params,
                body,
                is_async: *is_async,
                is_generator: *is_generator,
            })
        }
        AstStmt::Return { argument, .. } => {
            let value = argument
                .as_ref()
                .map(|e| lower_expr(checked, ctx, e, super_class));
            // Derived ctor: store completion + break so TypeError is outside try/catch (E19.82).
            if ctx.derived_ctor_body {
                if let (Some(mode_id), Some(val_id), Some(label)) = (
                    ctx.derived_ret_mode,
                    ctx.derived_ret_val,
                    ctx.derived_ctor_label.clone(),
                ) {
                    return Some(Stmt::Block {
                        body: vec![
                            Stmt::Expr {
                                expr: Expr::Assign {
                                    target: AssignTarget::Local(mode_id),
                                    op: AssignOp::Eq,
                                    value: Box::new(Expr::Number {
                                        raw: "1".into(),
                                        ty: Type::Number,
                                    }),
                                    ty: Type::Any,
                                },
                            },
                            Stmt::Expr {
                                expr: Expr::Assign {
                                    target: AssignTarget::Local(val_id),
                                    op: AssignOp::Eq,
                                    value: Box::new(value.unwrap_or_else(undef_expr)),
                                    ty: Type::Any,
                                },
                            },
                            Stmt::Break { label: Some(label) },
                        ],
                    });
                }
                if let Some(this_id) = ctx.derived_this {
                    // Fallback: inline [[Construct]] return completion (E19.82.03).
                    return Some(Stmt::Return {
                        value: Some(possible_constructor_return(this_id, value)),
                    });
                }
            }
            Some(Stmt::Return { value })
        }
        AstStmt::Throw { argument, .. } => Some(Stmt::Throw {
            value: lower_expr(checked, ctx, argument, super_class),
        }),
        AstStmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            ..
        } => {
            let block = lower_fn_body(checked, ctx, block, super_class);
            let handler_param = handler_param
                .as_ref()
                .map(|param| lower_binding_pattern(checked, ctx, param));
            let handler = handler
                .as_ref()
                .map(|h| lower_fn_body(checked, ctx, h, super_class));
            let finalizer = finalizer
                .as_ref()
                .map(|f| lower_fn_body(checked, ctx, f, super_class));
            Some(Stmt::Try {
                block,
                handler_param,
                handler,
                finalizer,
            })
        }
        AstStmt::With { object, body, .. } => Some(Stmt::With {
            object: lower_expr(checked, ctx, object, super_class),
            body: lower_fn_body(checked, ctx, body, super_class),
        }),
        AstStmt::ImportDeclaration { .. }
        | AstStmt::ExportNamedDeclaration { .. }
        | AstStmt::ExportDefaultDeclaration { .. }
        | AstStmt::ExportAllDeclaration { .. } => {
            panic!("import/export must be linked before lower")
        }
        // Type aliases are erased (T02); no runtime value.
        AstStmt::TypeAlias { .. } => None,
        // F06.03: lower `extern "C" function` to IR/ABI surface for LLVM.
        AstStmt::ExternFunctionDeclaration {
            abi,
            name,
            params,
            return_type,
            ..
        } => {
            let local = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.span == name.span)
                .map(|s| s.id)
                .expect("extern function binding must be declared");
            let param_tys = params
                .iter()
                .map(|p| {
                    let ann = p
                        .type_ann
                        .as_ref()
                        .expect("extern param type checked (F06.02)");
                    lower_extern_abi_type(checked, ann)
                })
                .collect();
            let ret = match return_type {
                None => None,
                Some(ann) if is_void_type_ann(ann) => None,
                Some(ann) => Some(lower_extern_abi_type(checked, ann)),
            };
            Some(Stmt::ExternFunction {
                local,
                abi: abi.value.to_string_lossy(),
                name: name.name.clone(),
                params: param_tys,
                ret,
            })
        }
    }
}

/// Resolve an extern ABI type annotation to IR `Type` (native scalar, pointer, function, or layout).
/// Checker (F06.02 / F03.02) already rejected non-ABI types.
pub(crate) fn lower_extern_abi_type(checked: &CheckedProgram, ann: &draconic_ast::TypeAnn) -> Type {
    match ann {
        draconic_ast::TypeAnn::Named { name, .. } => {
            if let Some(n) = NativeType::from_name(name) {
                Type::Native(n)
            } else if name == "function" {
                Type::Function
            } else if let Some(ty) = checked.type_alias(name) {
                ty
            } else {
                panic!("extern ABI type `{name}` must be native, function, or layout alias")
            }
        }
        draconic_ast::TypeAnn::Pointer { inner, .. } => {
            match lower_extern_abi_type(checked, inner) {
                Type::Native(n) => Type::Ptr(n),
                other => panic!("extern pointer pointee must be native scalar, got {other:?}"),
            }
        }
        draconic_ast::TypeAnn::Object { props, .. } => {
            let want: Vec<(String, Type)> = props
                .iter()
                .map(|p| (p.name.clone(), lower_extern_abi_type(checked, &p.ty)))
                .collect();
            for (i, shape) in checked.shapes().iter().enumerate() {
                if shape.props == want {
                    return Type::Shape(i as u32);
                }
            }
            panic!("extern ABI object type must match an interned native layout")
        }
        other => panic!("extern ABI type not native/pointer/layout: {other:?}"),
    }
}

pub(crate) fn is_void_type_ann(ann: &draconic_ast::TypeAnn) -> bool {
    matches!(ann, draconic_ast::TypeAnn::Named { name, .. } if name == "void")
}

pub(crate) fn lower_fn_body(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    body: &AstStmt,
    super_class: Option<&AstExpr>,
) -> Vec<Stmt> {
    match body {
        AstStmt::Block { body, .. } => lower_stmt_body(checked, ctx, body, super_class),
        other => lower_stmt_expand(checked, ctx, other, super_class),
    }
}
