use draconic_diagnostics::Diagnostic;
use draconic_ir::Module;

use super::walk::Seen;
use super::{classify_body, diag, emit_es_expr_with};

pub(super) fn emit_es(module: &Module, seen: &Seen) -> Result<String, Diagnostic> {
    match es_kind(module, seen) {
        EsKind::Promise => crate::es_promise::emit_es_promise(module),
        EsKind::Eval => crate::es_eval::emit_es_eval(module),
        EsKind::PrivateIn => crate::es_private_in::emit_es_private_in(module),
        EsKind::Proxies => crate::es_proxies::emit_es_proxies(module),
        EsKind::Testing => crate::es_testing::emit_es_testing(module),
        EsKind::Logging => crate::es_logging::emit_es_logging(module),
        EsKind::Mime => crate::es_mime::emit_es_mime(module),
        EsKind::Collections => crate::es_collections::emit_es_collections(module),
        EsKind::Encoding => crate::es_encoding::emit_es_encoding(module),
        EsKind::NewTarget => crate::es_new_target::emit_es_new_target(module),
        EsKind::PrivateAccessors => crate::es_private_accessors::emit_es_private_accessors(module),
        EsKind::Instanceof => crate::es_instanceof::emit_es_instanceof(module),
        EsKind::Generators => crate::es_generators::emit_es_generators(module),
        EsKind::Modules => crate::es_modules::emit_es_modules(module),
        EsKind::Exceptions => crate::es_exceptions::emit_es_exceptions(module),
        EsKind::Legacy => crate::es_legacy::emit_es_legacy(module),
        EsKind::OptionalChain => crate::es_optional_chain::emit_es_optional_chain(module),
        EsKind::StaticBlocks => crate::es_static_blocks::emit_es_static_blocks(module),
        EsKind::Nullish => crate::es_nullish::emit_es_nullish(module),
        EsKind::ToPrimitive => crate::es_to_primitive::emit_es_to_primitive(module),
        EsKind::Coercion => crate::es_coercion::emit_es_coercion(module),
        EsKind::Values => crate::es_values::emit_es_values(module),
        EsKind::CallSpread => crate::es_call_spread::emit_es_call_spread(module),
        EsKind::TaggedTemplate => crate::es_tagged_template::emit_es_tagged_template(module),
        EsKind::ParamDstr => crate::es_param_dstr::emit_es_param_dstr(module),
        EsKind::VarFor => crate::es_var_for::emit_es_var_for(module),
        EsKind::ClassExprName => crate::es_class_expr_name::emit_es_class_expr_name(module),
        EsKind::StaticPrivateMethods => {
            crate::es_static_private_methods::emit_es_static_private_methods(module)
        }
        EsKind::ObjectDestructure => {
            crate::es_object_destructure::emit_es_object_destructure(module)
        }
        EsKind::DestructureDefaults => {
            crate::es_destructure_defaults::emit_es_destructure_defaults(module)
        }
        EsKind::Builtins => crate::es_builtins::emit_es_builtins(module),
        EsKind::Objects => crate::es_objects::emit_es_objects(module),
        EsKind::Arrays => crate::es_arrays::emit_es_arrays(module),
        EsKind::Classes => claimed(crate::es_classes::walk_es_classes(module)),
        EsKind::Functions => claimed(crate::es_functions::walk_es_functions(module)),
        EsKind::Expr => {
            let info = classify_body(module).ok_or_else(|| diag("unsupported IR node"))?;
            emit_es_expr_with(module, &info)
        }
    }
}

fn claimed(opt: Option<Result<String, Diagnostic>>) -> Result<String, Diagnostic> {
    opt.ok_or_else(|| diag("unsupported IR node"))?
}

enum EsKind {
    Promise,
    Eval,
    PrivateIn,
    Proxies,
    Testing,
    Logging,
    Mime,
    Collections,
    Encoding,
    NewTarget,
    PrivateAccessors,
    Instanceof,
    Generators,
    Modules,
    Exceptions,
    Legacy,
    OptionalChain,
    StaticBlocks,
    Nullish,
    ToPrimitive,
    Coercion,
    Values,
    CallSpread,
    TaggedTemplate,
    ParamDstr,
    VarFor,
    ClassExprName,
    StaticPrivateMethods,
    ObjectDestructure,
    DestructureDefaults,
    Builtins,
    Objects,
    Arrays,
    Classes,
    Functions,
    Expr,
}

fn es_kind(module: &Module, seen: &Seen) -> EsKind {
    if (seen.ident("Promise") || seen.has_async_fn)
        && crate::es_promise::is_es_promise_module(module)
    {
        return EsKind::Promise;
    }
    if (seen.ident("eval") || seen.ident("Function")) && crate::es_eval::is_es_eval_module(module) {
        return EsKind::Eval;
    }
    if crate::es_private_in::is_es_private_in_module(module) {
        return EsKind::PrivateIn;
    }
    if (seen.ident("Proxy") || seen.ident("Reflect"))
        && crate::es_proxies::is_es_proxies_module(module)
    {
        return EsKind::Proxies;
    }
    if (seen.ident("describe") || seen.ident("it") || seen.ident("expect"))
        && crate::es_testing::is_es_testing_module(module)
    {
        return EsKind::Testing;
    }
    if seen.ident("createLogger") && crate::es_logging::is_es_logging_module(module) {
        return EsKind::Logging;
    }
    if (seen.ident("parseMultipart") || seen.ident("serializeMultipart"))
        && crate::es_mime::is_es_mime_module(module)
    {
        return EsKind::Mime;
    }
    if (seen.ident("groupBy") || seen.ident("chunk") || seen.ident("Deque"))
        && crate::es_collections::is_es_collections_module(module)
    {
        return EsKind::Collections;
    }
    if (seen.ident("TextEncoder")
        || seen.ident("TextDecoder")
        || seen.ident("toBase64")
        || seen.ident("fromBase64")
        || seen.ident("toHex")
        || seen.ident("fromHex")
        || seen.ident("sha256")
        || seen.ident("hmacSha256")
        || seen.ident("aeadEncrypt")
        || seen.ident("aeadDecrypt")
        || seen.ident("randomBytes"))
        && crate::es_encoding::is_es_encoding_module(module)
    {
        return EsKind::Encoding;
    }
    if seen.has_new_target && crate::es_new_target::is_es_new_target_module(module) {
        return EsKind::NewTarget;
    }
    if crate::es_private_accessors::is_es_private_accessors_module(module) {
        return EsKind::PrivateAccessors;
    }
    if seen.has_instanceof && crate::es_instanceof::is_es_instanceof_module(module) {
        return EsKind::Instanceof;
    }
    if (seen.has_generator || seen.has_for_await || seen.has_async_fn)
        && crate::es_generators::is_es_generators_module(module)
    {
        return EsKind::Generators;
    }
    if seen.idents.iter().any(|n| n.starts_with("__m"))
        && crate::es_modules::is_es_modules_module(module)
    {
        return EsKind::Modules;
    }
    if seen.has_try && crate::es_exceptions::is_es_exceptions_module(module) {
        return EsKind::Exceptions;
    }
    if seen.has_with && crate::es_legacy::is_es_legacy_module(module) {
        return EsKind::Legacy;
    }
    if seen.has_optional && crate::es_optional_chain::is_es_optional_chain_module(module) {
        return EsKind::OptionalChain;
    }
    if crate::es_static_blocks::is_es_static_blocks_module(module) {
        return EsKind::StaticBlocks;
    }
    if seen.has_nullish && crate::es_nullish::is_es_nullish_module(module) {
        return EsKind::Nullish;
    }
    if seen.has_value_of && crate::es_to_primitive::is_es_to_primitive_module(module) {
        return EsKind::ToPrimitive;
    }
    if crate::es_coercion::is_es_coercion_module(module) {
        return EsKind::Coercion;
    }
    if seen.ident("Symbol") && crate::es_values::is_es_values_module(module) {
        return EsKind::Values;
    }
    if seen.has_spread_arg && crate::es_call_spread::is_es_call_spread_module(module) {
        return EsKind::CallSpread;
    }
    if seen.has_tagged && crate::es_tagged_template::is_es_tagged_template_module(module) {
        return EsKind::TaggedTemplate;
    }
    if crate::es_param_dstr::is_es_param_dstr_module(module) {
        return EsKind::ParamDstr;
    }
    if seen.has_var && seen.has_for_in_of && crate::es_var_for::is_es_var_for_module(module) {
        return EsKind::VarFor;
    }
    if crate::es_class_expr_name::is_es_class_expr_name_module(module) {
        return EsKind::ClassExprName;
    }
    if crate::es_static_private_methods::is_es_static_private_methods_module(module) {
        return EsKind::StaticPrivateMethods;
    }
    if seen.has_obj_pattern && crate::es_object_destructure::is_es_object_destructure_module(module)
    {
        return EsKind::ObjectDestructure;
    }
    if crate::es_destructure_defaults::is_es_destructure_defaults_module(module) {
        return EsKind::DestructureDefaults;
    }
    if (is_builtins_ident(seen) || seen.has_regexp)
        && crate::es_builtins::is_es_builtins_module(module)
    {
        return EsKind::Builtins;
    }
    if crate::es_console::module_has_console_log(module) {
        if seen.has_function {
            return EsKind::Functions;
        }
        return EsKind::Expr;
    }
    if crate::es_objects::is_es_objects_module(module) {
        return EsKind::Objects;
    }
    if crate::es_classes::walk_es_classes_applies(module) {
        return EsKind::Classes;
    }
    if (seen.has_array || seen.has_arr_pattern) && crate::es_arrays::is_es_arrays_module(module) {
        return EsKind::Arrays;
    }
    if classify_body(module).is_none()
        && (seen.ident("Object") || seen.ident("String"))
        && crate::es_builtins::is_es_builtins_module(module)
    {
        return EsKind::Builtins;
    }
    if seen.has_function {
        return EsKind::Functions;
    }
    EsKind::Expr
}

fn is_builtins_ident(seen: &Seen) -> bool {
    seen.ident("JSON")
        || seen.ident("Date")
        || seen.ident("RegExp")
        || seen.ident("Map")
        || seen.ident("Set")
        || seen.ident("WeakMap")
        || seen.ident("WeakSet")
        || seen.ident("Error")
        || seen.ident("TypeError")
        || seen.ident("parseInt")
        || seen.ident("parseFloat")
        || seen.ident("encodeURI")
        || seen.ident("encodeURIComponent")
        || seen.ident("decodeURI")
        || seen.ident("decodeURIComponent")
        || seen.ident("escape")
        || seen.ident("unescape")
        || seen.ident("ArrayBuffer")
        || seen.ident("DataView")
        || seen.ident("Uint8Array")
        || seen.ident("Int32Array")
        || seen.ident("Float64Array")
        || seen.ident("isNaN")
        || seen.ident("isFinite")
}
