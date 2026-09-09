use draconic_ast::{Arg, BinaryOp, BindingPattern, Expr, Param, TypeAnn, UnaryOp};
use draconic_diagnostics::{codes, Diagnostic, Span};

use super::checker::{Checker, FnSig};
use super::{
    extern_unsupported_on_js_diagnostic, format_type_full, is_void_type_ann, CompileTarget,
    NativeType, Type,
};

impl Checker {
    /// F06.02: check an `extern "C" function` declaration.
    ///
    /// - Binds the name as a callable `function`.
    /// - Every parameter must be annotated with a native scalar, pointer, `function`, or native layout.
    /// - Return type is optional / `void`, or native scalar / pointer / `function` / native layout.
    /// - JS-only types (`string`, `number`, `any`, non-layout shapes, …) are rejected.
    /// - Records a full `FnSig` so later call sites get arity/arg checking.
    /// - F08.01: when the compile target is js, hard-error (native-only FFI).
    pub(crate) fn check_extern_function_declaration(
        &mut self,
        name: &draconic_ast::Ident,
        params: &[Param],
        return_type: &Option<TypeAnn>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        // F08.01: `extern "C"` is native-only FFI — reject before signature detail on js.
        if self.host_target == Some(CompileTarget::Js) {
            return Err(extern_unsupported_on_js_diagnostic(&name.name, span));
        }

        let Some(id) = self
            .symbols()
            .iter()
            .find(|s| s.span == name.span)
            .map(|s| s.id)
        else {
            return Err(Diagnostic::new(
                format!("extern function binding `{}` must be declared", name.name),
                span,
            ));
        };
        self.symbol_types[id.0 as usize] = Type::Function;

        let mut param_types = Vec::with_capacity(params.len());
        for p in params {
            if p.rest {
                return Err(Diagnostic::new(
                    "extern parameter cannot be a rest parameter".to_string(),
                    p.binding.span(),
                )
                .with_code(codes::INVALID_EXTERN_TYPE)
                .with_help("C ABI parameters are fixed arity; remove `...`"));
            }
            if p.default.is_some() {
                return Err(Diagnostic::new(
                    "extern parameter cannot have a default value".to_string(),
                    p.binding.span(),
                )
                .with_code(codes::INVALID_EXTERN_TYPE)
                .with_help("C ABI parameters have no defaults; remove the initializer"));
            }
            if !matches!(p.binding, BindingPattern::Ident(_)) {
                return Err(Diagnostic::new(
                    "extern parameter must be a simple identifier".to_string(),
                    p.binding.span(),
                )
                .with_code(codes::INVALID_EXTERN_TYPE)
                .with_help("destructuring is not valid in an extern \"C\" signature"));
            }
            let Some(ann) = &p.type_ann else {
                return Err(Diagnostic::new(
                    "extern parameter must have a type annotation".to_string(),
                    p.binding.span(),
                )
                .with_code(codes::INVALID_EXTERN_TYPE)
                .with_help(
                    "annotate with a native scalar, pointer, function, or native layout struct",
                ));
            };
            if is_void_type_ann(ann) {
                return Err(Diagnostic::new(
                    "extern parameter type cannot be `void`".to_string(),
                    ann.span(),
                )
                .with_code(codes::INVALID_EXTERN_TYPE)
                .with_help("`void` is only valid as an extern return type"));
            }
            let ty = self.resolve_extern_abi_type(ann, "parameter")?;
            param_types.push(Some(ty));
        }

        if let Some(ann) = return_type {
            if !is_void_type_ann(ann) {
                let _ = self.resolve_extern_abi_type(ann, "return")?;
            }
        }

        // Always record a signature (including zero-param) so call sites check arity.
        self.fn_sigs[id.0 as usize] = Some(FnSig {
            param_types,
            required: params.len(),
            has_rest: false,
        });
        Ok(())
    }

    /// Resolve a type annotation for an extern ABI position: native scalar, `*T`, `function`, or native layout.
    pub(crate) fn resolve_extern_abi_type(
        &mut self,
        ann: &TypeAnn,
        role: &str,
    ) -> Result<Type, Diagnostic> {
        let ty = self.resolve_type_ann(ann)?;
        if matches!(ty, Type::Native(_) | Type::Ptr(_) | Type::Function) {
            return Ok(ty);
        }
        if self.is_native_layout(ty) {
            return Ok(ty);
        }
        let pretty = format_type_full(ty, &self.shapes, &self.unions, &self.intersections);
        Err(Diagnostic::new(
            format!("extern {role} type must be a native scalar, pointer, function, or native layout, got `{pretty}`"),
            ann.span(),
        )
        .with_code(codes::INVALID_EXTERN_TYPE)
        .with_help(
            "use a native type such as `i32`, `i64`, `f64`, `bool`, a pointer like `*u8`, `function`, or a native-field struct",
        ))
    }

    /// Build a resolved call signature from a parameter list (T07.01).
    /// Returns `None` when the function has no annotated parameters, so untyped
    /// (E19-era) functions keep permissive call-site behavior.
    pub(crate) fn fn_sig_from_params(&mut self, params: &[Param]) -> Option<FnSig> {
        let mut param_types = Vec::with_capacity(params.len());
        let mut required = 0usize;
        let mut has_rest = false;
        let mut any_annotated = false;
        for p in params {
            let ann = match &p.type_ann {
                Some(ann) => {
                    any_annotated = true;
                    Some(self.resolve_type_ann(ann).ok()?)
                }
                None => None,
            };
            if p.rest {
                has_rest = true;
            } else if p.default.is_none() && ann.is_some() {
                required += 1;
            }
            param_types.push(ann);
        }
        if !any_annotated {
            return None;
        }
        Some(FnSig {
            param_types,
            required,
            has_rest,
        })
    }

    /// Check a call against a recorded annotated signature (T07.01): reject wrong
    /// arity and non-assignable arguments. Unannotated params are skipped.
    pub(crate) fn check_call_sig(
        &self,
        sig: &FnSig,
        args: &[Arg],
        arg_tys: &[Type],
        span: Span,
    ) -> Result<(), Diagnostic> {
        // Spreads make arity unknowable statically; still type-check the leading args.
        let has_spread = args.iter().any(|a| matches!(a, Arg::Spread(_)));
        if !has_spread {
            if arg_tys.len() < sig.required {
                return Err(Diagnostic::new(
                    format!(
                        "expected at least {} argument(s), got {}",
                        sig.required,
                        arg_tys.len()
                    ),
                    span,
                )
                .with_code(codes::WRONG_ARITY)
                .with_help("pass the required number of arguments for this function"));
            }
            if !sig.has_rest && arg_tys.len() > sig.param_types.len() {
                return Err(Diagnostic::new(
                    format!(
                        "expected at most {} argument(s), got {}",
                        sig.param_types.len(),
                        arg_tys.len()
                    ),
                    span,
                )
                .with_code(codes::WRONG_ARITY)
                .with_help("pass the required number of arguments for this function"));
            }
        }
        for (i, want) in sig.param_types.iter().enumerate() {
            let Some(want) = want else { continue };
            let Some(Arg::Expr(expr)) = args.get(i) else {
                continue;
            };
            let got = arg_tys.get(i).copied().unwrap_or(Type::Any);
            self.require_assignable_expr(got, *want, expr)?;
        }
        Ok(())
    }

    /// T07.04: whether a JS value type has a call/construct surface. Only called for
    /// annotated bindings; `Any`, un-annotated `Object`, and inferred (non-strict)
    /// object-literal/tuple shapes stay permissive (runtime TypeError, E19.13/E19.59).
    pub(crate) fn type_is_callable(&self, ty: Type) -> bool {
        match ty {
            Type::Function | Type::GenericFn(_) | Type::Any => true,
            // Strict (annotated) shapes have no call signatures; inferred shapes stay permissive.
            Type::Shape(id) => !self.shapes.get(id as usize).is_some_and(|s| s.strict),
            _ => false,
        }
    }

    /// True when `ty` is a non-empty shape of native scalar fields (N03 / F03.01).
    pub(crate) fn is_native_layout(&self, ty: Type) -> bool {
        let Type::Shape(id) = ty else {
            return false;
        };
        let Some(shape) = self.shapes.get(id as usize) else {
            return false;
        };
        !shape.props.is_empty()
            && shape
                .props
                .iter()
                .all(|(_, t)| matches!(t, Type::Native(_)))
    }

    pub(crate) fn check_unary(
        &self,
        op: UnaryOp,
        arg: Type,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        if !self.typecheck {
            return Ok(Type::Any);
        }
        match op {
            // Unary `+` is ToNumber (ECMA-262); BigInt throws at runtime — reject statically.
            UnaryOp::Plus => {
                if arg == Type::BigInt {
                    Err(Diagnostic::new(
                        format!("unary `{op}` cannot be applied to type `{arg}`"),
                        span,
                    ))
                } else {
                    Ok(Type::Number)
                }
            }
            UnaryOp::Minus | UnaryOp::BitNot => {
                if arg == Type::BigInt {
                    Ok(Type::BigInt)
                } else if let Type::Native(n) = arg {
                    if n.is_float() && matches!(op, UnaryOp::BitNot) {
                        Err(Diagnostic::new(
                            format!("unary `{op}` cannot be applied to type `{arg}`"),
                            span,
                        ))
                    } else if n.is_float() && matches!(op, UnaryOp::Minus) {
                        Ok(Type::Native(n))
                    } else if n.is_int() {
                        Ok(Type::Native(n))
                    } else {
                        Err(Diagnostic::new(
                            format!("unary `{op}` cannot be applied to type `{arg}`"),
                            span,
                        ))
                    }
                } else if self.is_js_to_number_operand(arg) {
                    // E19.04: ToNumber / ToInt32 on JS values (string, boolean, object, …).
                    Ok(Type::Number)
                } else {
                    Err(Diagnostic::new(
                        format!("unary `{op}` cannot be applied to type `{arg}`"),
                        span,
                    ))
                }
            }
            UnaryOp::Not => Ok(Type::Boolean),
            UnaryOp::TypeOf => Ok(Type::String),
            UnaryOp::Void => Ok(Type::Null),
            UnaryOp::Delete => Ok(Type::Boolean),
            // Await yields the fulfillment value; keep coarse `any` for now.
            UnaryOp::Await => Ok(Type::Any),
            // Yield expression value is the next `.next(arg)` resume value; coarse `any`.
            // `yield*` completion is the inner iterator's final value; coarse `any`.
            UnaryOp::Yield | UnaryOp::YieldStar => Ok(Type::Any),
            // N03.03: `&x` → `*T` when x is native scalar T.
            // F03.01: `&layout` → `*u8` (byte pointer to C ABI struct).
            UnaryOp::Ref => match arg {
                Type::Native(n) => Ok(Type::Ptr(n)),
                Type::Shape(_) if self.is_native_layout(arg) => Ok(Type::Ptr(NativeType::U8)),
                other => Err(Diagnostic::new(
                    format!("cannot take address of type `{other}` (native scalar required)"),
                    span,
                )),
            },
            // N03.03: `*p` → T when p is `*T`.
            UnaryOp::Deref => match arg {
                Type::Ptr(n) => Ok(Type::Native(n)),
                other => Err(Diagnostic::new(
                    format!("cannot dereference type `{other}` (pointer required)"),
                    span,
                )),
            },
        }
    }

    pub(crate) fn check_binary(
        &self,
        op: BinaryOp,
        left: Type,
        right: Type,
        span: Span,
        left_expr: &Expr,
        right_expr: &Expr,
    ) -> Result<Type, Diagnostic> {
        if !self.typecheck {
            return Ok(Type::Any);
        }
        match op {
            // Binary `+`: string preference (ToString) else numeric (ToNumber), per ECMA-262.
            // Object/Function sides use runtime ToPrimitive (valueOf/toString); static type is Any.
            BinaryOp::Add => {
                if left == Type::BigInt && right == Type::BigInt {
                    Ok(Type::BigInt)
                } else if left == Type::String || right == Type::String {
                    // Including BigInt + string → ToString concat (ECMA-262).
                    if self.is_js_add_side(left) && self.is_js_add_side(right) {
                        Ok(Type::String)
                    } else {
                        Err(Diagnostic::new(
                            format!(
                                "operator `+` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    }
                } else if left == Type::BigInt || right == Type::BigInt {
                    // E19.07: mixed bigint×number/object/any — TypeError (or ToPrimitive) at runtime.
                    if self.is_js_bigint_mixed_operand(left)
                        && self.is_js_bigint_mixed_operand(right)
                    {
                        Ok(Type::Any)
                    } else {
                        Err(Diagnostic::new(
                            format!(
                                "operator `+` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    }
                } else if let Some(n) = self.native_arith_result(left, right, left_expr, right_expr)
                {
                    Ok(Type::Native(n))
                } else if matches!(
                    left,
                    Type::Object | Type::Shape(_) | Type::Function | Type::Any
                ) || matches!(
                    right,
                    Type::Object | Type::Shape(_) | Type::Function | Type::Any
                ) {
                    if self.is_add_operand(left) && self.is_add_operand(right) {
                        Ok(Type::Any)
                    } else {
                        Err(Diagnostic::new(
                            format!(
                                "operator `+` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    }
                } else if self.is_primitive_numeric_coercible(left)
                    && self.is_primitive_numeric_coercible(right)
                {
                    Ok(Type::Number)
                } else {
                    Err(Diagnostic::new(
                        format!("operator `+` cannot be applied to types `{left}` and `{right}`"),
                        span,
                    ))
                }
            }
            BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::Pow
            | BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr => {
                if left == Type::BigInt || right == Type::BigInt {
                    // Same-type BigInt (except `>>>`, which always TypeErrors on BigInt).
                    // E19.07: mixed bigint×number/object/any — TypeError is runtime, not compile.
                    if left == Type::BigInt
                        && right == Type::BigInt
                        && !matches!(op, BinaryOp::UShr)
                    {
                        Ok(Type::BigInt)
                    } else if self.is_js_bigint_mixed_operand(left)
                        && self.is_js_bigint_mixed_operand(right)
                    {
                        Ok(Type::Any)
                    } else {
                        Err(Diagnostic::new(
                            format!(
                                "operator `{op}` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    }
                } else if let Some(n) = self.native_arith_result(left, right, left_expr, right_expr)
                {
                    // `>>>` is JS ToUint32; reject on native types.
                    if matches!(op, BinaryOp::UShr) {
                        Err(Diagnostic::new(
                            format!(
                                "operator `{op}` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    } else {
                        Ok(Type::Native(n))
                    }
                } else if self.is_js_to_number_operand(left) && self.is_js_to_number_operand(right)
                {
                    // E19.04: ToNumber both sides (string/boolean/null/object/…).
                    Ok(Type::Number)
                } else {
                    Err(Diagnostic::new(
                        format!(
                            "operator `{op}` cannot be applied to types `{left}` and `{right}`"
                        ),
                        span,
                    ))
                }
            }
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                if self
                    .native_arith_result(left, right, left_expr, right_expr)
                    .is_some()
                {
                    Ok(Type::Boolean)
                } else if self.is_js_relational_operand(left)
                    && self.is_js_relational_operand(right)
                {
                    // E19.04: ToPrimitive; mixed primitives/objects/BigInt+Number ok.
                    Ok(Type::Boolean)
                } else {
                    Err(Diagnostic::new(
                        format!(
                            "operator `{op}` cannot be applied to types `{left}` and `{right}`"
                        ),
                        span,
                    ))
                }
            }
            BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq => {
                Ok(Type::Boolean)
            }
            // `in`: property-key left, object-like right; result is boolean (ECMA-262).
            BinaryOp::In => Ok(Type::Boolean),
            // `instanceof`: object left, constructor right; result is boolean (ECMA-262).
            BinaryOp::InstanceOf => Ok(Type::Boolean),
            BinaryOp::And | BinaryOp::Or | BinaryOp::Nullish => {
                if left == right {
                    Ok(left)
                } else if left == Type::Any || right == Type::Any {
                    Ok(Type::Any)
                } else {
                    // TS-style union collapsed to any for the minimal surface.
                    Ok(Type::Any)
                }
            }
            // Comma yields the RHS type (both sides still typechecked for effects).
            BinaryOp::Comma => Ok(right),
        }
    }

    /// JS values that numeric operators coerce via ToNumber (not BigInt, not native/ptr).
    pub(crate) fn is_js_to_number_operand(&self, ty: Type) -> bool {
        matches!(
            ty,
            Type::Number
                | Type::String
                | Type::Boolean
                | Type::Null
                | Type::Object
                | Type::Shape(_)
                | Type::Function
                | Type::GenericFn(_)
                | Type::Union(_)
                | Type::Intersection(_)
                | Type::TypeParam(_)
                | Type::Any
        )
    }

    /// E19.04 / E19.13: `++`/`--` apply ToNumber (objects via valueOf/toString); BigInt stays BigInt.
    pub(crate) fn check_update_operand(
        &self,
        left_ty: Type,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let ok = left_ty == Type::BigInt
            || matches!(left_ty, Type::Native(n) if n.is_int())
            || self.is_js_to_number_operand(left_ty);
        if !ok {
            return Err(Diagnostic::new(
                format!("update operator cannot be applied to type `{left_ty}`"),
                span,
            ));
        }
        Ok(if matches!(left_ty, Type::Native(_)) {
            left_ty
        } else if left_ty == Type::BigInt {
            Type::BigInt
        } else {
            Type::Number
        })
    }

    /// E19.07: BigInt or JS value that may mix with BigInt at runtime (TypeError / ToPrimitive).
    pub(crate) fn is_js_bigint_mixed_operand(&self, ty: Type) -> bool {
        ty == Type::BigInt || self.is_js_to_number_operand(ty)
    }

    /// Sides legal for binary `+` string/numeric paths (JS values + BigInt; not native/ptr).
    pub(crate) fn is_js_add_side(&self, ty: Type) -> bool {
        self.is_js_bigint_mixed_operand(ty)
    }

    /// Relational comparison operands after ToPrimitive (includes BigInt same-type path separately).
    pub(crate) fn is_js_relational_operand(&self, ty: Type) -> bool {
        self.is_js_to_number_operand(ty) || ty == Type::BigInt
    }

    /// Same native numeric type on both sides, or native + number-literal (contextual).
    pub(crate) fn native_arith_result(
        &self,
        left: Type,
        right: Type,
        left_expr: &Expr,
        right_expr: &Expr,
    ) -> Option<NativeType> {
        match (left, right) {
            (Type::Native(a), Type::Native(b)) if a == b && !a.is_bool() => Some(a),
            (Type::Native(a), Type::Number)
                if !a.is_bool() && Self::is_number_literal_expr(right_expr) =>
            {
                Some(a)
            }
            (Type::Number, Type::Native(b))
                if !b.is_bool() && Self::is_number_literal_expr(left_expr) =>
            {
                Some(b)
            }
            _ => None,
        }
    }

    /// Primitives ToNumber accepts for binary `+` when neither side is string/BigInt/object.
    pub(crate) fn is_primitive_numeric_coercible(&self, ty: Type) -> bool {
        matches!(ty, Type::Number | Type::Boolean | Type::Null)
    }

    pub(crate) fn is_add_operand(&self, ty: Type) -> bool {
        !matches!(ty, Type::Native(_) | Type::Ptr(_))
    }
}
