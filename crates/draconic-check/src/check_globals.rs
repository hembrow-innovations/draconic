use super::*;
use crate::bind_globals::find_ident_use;
use draconic_diagnostics::Span;
use draconic_parser::parse;

#[test]
fn check_math_is_object() {
    let program = parse("let t = typeof Math; let a = Math.abs(-3);").unwrap();
    let checked = check(program).unwrap();
    let math = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "Math" && s.span == Span::dummy())
        .expect("Math builtin");
    assert_eq!(checked.type_of_symbol(math.id), Type::Object);
}

#[test]
fn check_types_global_number_nan_infinity() {
    let program =
        parse("let t = typeof Number; let a = Number.isNaN(NaN); let n = NaN; let i = Infinity;")
            .unwrap();
    let checked = check(program).unwrap();
    let number = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "Number" && s.span == Span::dummy())
        .expect("Number builtin");
    assert_eq!(checked.type_of_symbol(number.id), Type::Function);
    let nan = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "NaN" && s.span == Span::dummy())
        .expect("NaN builtin");
    assert_eq!(checked.type_of_symbol(nan.id), Type::Number);
    let inf = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "Infinity" && s.span == Span::dummy())
        .expect("Infinity builtin");
    assert_eq!(checked.type_of_symbol(inf.id), Type::Number);
}

#[test]
fn check_symbol_is_function() {
    let program = parse("let t = typeof Symbol; let s = Symbol();").unwrap();
    let checked = check(program).unwrap();
    let sym = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "Symbol" && s.span == Span::dummy())
        .expect("Symbol builtin");
    assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
}

#[test]
fn check_promise_is_function() {
    let program =
        parse("let t = typeof Promise; let p = new Promise(function (r) { r(1); });").unwrap();
    let checked = check(program).unwrap();
    let sym = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "Promise" && s.span == Span::dummy())
        .expect("Promise builtin");
    assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
}

#[test]
fn check_proxy_is_function() {
    let program = parse("let t = typeof Proxy; let p = new Proxy({}, {});").unwrap();
    let checked = check(program).unwrap();
    let sym = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "Proxy" && s.span == Span::dummy())
        .expect("Proxy builtin");
    assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
}

#[test]
fn check_proxy_of_function_is_callable() {
    let program =
        parse("let t = function (a) { return a; }; let p = new Proxy(t, {}); let r = p(1);")
            .unwrap();
    let checked = check(program).unwrap();
    let p = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "p")
        .expect("p");
    assert_eq!(checked.type_of_symbol(p.id), Type::Function);
}

#[test]
fn check_proxy_of_object_call_typechecks() {
    // E19.13: Object/Proxy callability is a runtime [[Call]] check, not compile reject.
    let program = parse("let p = new Proxy({}, {}); try { p(); } catch (e) {}").unwrap();
    check(program).expect("calling Proxy of object should typecheck");
}

#[test]
fn check_reflect_is_object() {
    let program = parse("let t = typeof Reflect; let g = Reflect.get;").unwrap();
    let checked = check(program).unwrap();
    let sym = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "Reflect" && s.span == Span::dummy())
        .expect("Reflect builtin");
    assert_eq!(checked.type_of_symbol(sym.id), Type::Object);
}

#[test]
fn check_types_undefined_and_global_this() {
    let program = parse("let u = undefined; let g = globalThis;").unwrap();
    let checked = check(program).unwrap();
    let undef = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "undefined" && s.span == Span::dummy())
        .expect("undefined builtin");
    assert_eq!(checked.type_of_symbol(undef.id), Type::Any);
    let gt = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "globalThis" && s.span == Span::dummy())
        .expect("globalThis builtin");
    assert_eq!(checked.type_of_symbol(gt.id), Type::Object);
}

#[test]
fn check_fundamental_constructors_are_functions() {
    let program =
        parse("let a = typeof Object; let b = typeof Function; let c = typeof Array; let d = typeof String; let e = typeof Boolean;")
            .unwrap();
    let checked = check(program).unwrap();
    for name in ["Object", "Function", "Array", "String", "Boolean"] {
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == name && s.span == Span::dummy())
            .unwrap_or_else(|| panic!("{name} builtin"));
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }
}

#[test]
fn check_error_constructors_are_functions() {
    let program =
        parse("let a = typeof Error; let b = typeof TypeError; let c = typeof AggregateError;")
            .unwrap();
    let checked = check(program).unwrap();
    for name in [
        "Error",
        "TypeError",
        "RangeError",
        "ReferenceError",
        "SyntaxError",
        "URIError",
        "EvalError",
        "AggregateError",
    ] {
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == name && s.span == Span::dummy())
            .unwrap_or_else(|| panic!("{name} builtin"));
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }
}

#[test]
fn check_new_error_is_ok() {
    let program = parse(
        "let e = new Error(\"m\"); let t = new TypeError(\"t\"); let a = new AggregateError([], \"a\");",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_global_functions_are_functions() {
    let program = parse(
        "let a = typeof parseInt; let b = typeof parseFloat; let c = typeof isNaN; let d = typeof isFinite;",
    )
    .unwrap();
    let checked = check(program).unwrap();
    for name in ["parseInt", "parseFloat", "isNaN", "isFinite"] {
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == name && s.span == Span::dummy())
            .unwrap_or_else(|| panic!("{name} builtin"));
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }
}

#[test]
fn check_global_function_calls_ok() {
    let program = parse(
        "let a = parseInt(\"42\"); let b = parseFloat(\"3.14\"); let c = isNaN(NaN); let d = isFinite(1);",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_uri_functions_are_functions() {
    let program = parse(
        "let a = typeof encodeURI; let b = typeof decodeURI; let c = typeof encodeURIComponent; let d = typeof decodeURIComponent;",
    )
    .unwrap();
    let checked = check(program).unwrap();
    for name in [
        "encodeURI",
        "decodeURI",
        "encodeURIComponent",
        "decodeURIComponent",
    ] {
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == name && s.span == Span::dummy())
            .unwrap_or_else(|| panic!("{name} builtin"));
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }
}

#[test]
fn check_uri_function_calls_ok() {
    let program = parse(
        "let a = encodeURI(\"a b\"); let b = decodeURI(\"a%20b\"); let c = encodeURIComponent(\"a&b\"); let d = decodeURIComponent(\"a%26b\");",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_escape_unescape_are_functions() {
    let program = parse("let a = typeof escape; let b = typeof unescape;").unwrap();
    let checked = check(program).unwrap();
    for name in ["escape", "unescape"] {
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == name && s.span == Span::dummy())
            .unwrap_or_else(|| panic!("{name} builtin"));
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }
}

#[test]
fn check_escape_unescape_calls_ok() {
    let program = parse(
        "let a = escape(\"a b\"); let b = unescape(\"%20\"); let c = unescape(escape(\"x\"));",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_json_is_object() {
    let program =
        parse("let t = typeof JSON; let p = JSON.parse; let s = JSON.stringify;").unwrap();
    let checked = check(program).unwrap();
    let sym = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "JSON" && s.span == Span::dummy())
        .expect("JSON builtin");
    assert_eq!(checked.type_of_symbol(sym.id), Type::Object);
}

#[test]
fn check_json_parse_stringify_calls_ok() {
    let program = parse(
        "let a = JSON.stringify(1); let b = JSON.parse(\"1\"); let c = JSON.parse(JSON.stringify({ x: 2 }));",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_date_is_function() {
    let program = parse("let t = typeof Date; let n = Date.now;").unwrap();
    let checked = check(program).unwrap();
    let sym = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "Date" && s.span == Span::dummy())
        .expect("Date builtin");
    assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
}

#[test]
fn check_date_now_and_new_ok() {
    let program = parse(
        "let n = Date.now(); let d = new Date(0); let t = d.getTime(); let u = Date.UTC(1970, 0, 1);",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_regexp_is_function() {
    let program = parse("let t = typeof RegExp; let s = RegExp.prototype;").unwrap();
    let checked = check(program).unwrap();
    let sym = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "RegExp" && s.span == Span::dummy())
        .expect("RegExp builtin");
    assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
}

#[test]
fn check_regexp_new_and_methods_ok() {
    let program = parse(
        "let r = new RegExp(\"a+\", \"i\"); let t = r.test(\"AA\"); let m = r.exec(\"xAAy\"); let s = r.source; let f = r.flags;",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_map_and_set_are_functions() {
    let program = parse("let tm = typeof Map; let ts = typeof Set;").unwrap();
    let checked = check(program).unwrap();
    for name in ["Map", "Set"] {
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == name && s.span == Span::dummy())
            .expect(name);
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }
}

#[test]
fn check_map_set_new_and_methods_ok() {
    let program = parse(
        "let m = new Map(); m.set(1, 2); let g = m.get(1); let h = m.has(1); let n = m.size; let s = new Set(); s.add(3); let sh = s.has(3); let sn = s.size;",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_weak_map_and_weak_set_are_functions() {
    let program = parse("let twm = typeof WeakMap; let tws = typeof WeakSet;").unwrap();
    let checked = check(program).unwrap();
    for name in ["WeakMap", "WeakSet"] {
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == name && s.span == Span::dummy())
            .expect(name);
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }
}

#[test]
fn check_weak_map_set_new_and_methods_ok() {
    let program = parse(
        "let k = {}; let wm = new WeakMap(); wm.set(k, 1); let g = wm.get(k); let h = wm.has(k); let d = wm.delete(k); let ws = new WeakSet(); ws.add(k); let sh = ws.has(k); let sd = ws.delete(k);",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_arraybuffer_dataview_typedarrays_are_functions() {
    let program = parse(
        "let tab = typeof ArrayBuffer; let tdv = typeof DataView; let tu8 = typeof Uint8Array;",
    )
    .unwrap();
    let checked = check(program).unwrap();
    for name in [
        "ArrayBuffer",
        "DataView",
        "Uint8Array",
        "Int32Array",
        "Float64Array",
    ] {
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == name && s.span == Span::dummy())
            .expect(name);
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }
}

#[test]
fn check_arraybuffer_typedarrays_new_and_ops_ok() {
    let program = parse(
        "let buf = new ArrayBuffer(8); let bl = buf.byteLength; let u8 = new Uint8Array(buf); u8[0] = 1; let x = u8[0]; let i32 = new Int32Array(2); i32[0] = 42; let f64 = new Float64Array([1.5]); let dv = new DataView(buf); dv.setUint8(0, 1); let g = dv.getUint8(0);",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_eval_is_function() {
    let program = parse("let t = typeof eval;").unwrap();
    let checked = check(program).unwrap();
    let sym = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "eval" && s.span == Span::dummy())
        .expect("eval builtin");
    assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
}

#[test]
fn check_eval_call_ok() {
    let program = parse("let a = eval(\"1 + 2\"); let b = eval(\"typeof undefined\");").unwrap();
    check(program).unwrap();
}

#[test]
fn check_new_function_is_function() {
    let program = parse("let f = new Function(\"return 1\");").unwrap();
    let checked = check(program).unwrap();
    let f = checked
        .bound
        .symbols()
        .iter()
        .find(|s| s.name == "f")
        .expect("f");
    assert_eq!(checked.type_of_symbol(f.id), Type::Function);
}

#[test]
fn check_new_function_call_ok() {
    let program = parse(
        "let f = new Function(\"a\", \"b\", \"return a + b\"); let r = f(1, 2); let g = Function(\"x\", \"return x\"); let s = g(3);",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn check_function_call_construct_ok() {
    let program = parse("let f = Function(\"return 7\"); let r = f();").unwrap();
    check(program).unwrap();
}

// E17.02.09: for-in/of left free IdentifierReference is runtime PutValue, not check error.
#[test]
fn check_free_for_in_of_left_ok() {
    let src = "for (k in {a: 1}) {} for (v of [2]) {}";
    let bound = bind(parse(src).unwrap()).expect("free for-in/of left binds");
    check(parse(src).unwrap()).expect("free for-in/of left checks");
    let k_span = find_ident_use(&bound.program, "k");
    let v_span = find_ident_use(&bound.program, "v");
    assert!(bound.resolve(k_span).is_none());
    assert!(bound.resolve(v_span).is_none());
}

#[test]
fn check_arguments_in_function_ok() {
    let program =
        parse("function f(a, b) { return arguments.length + arguments[0]; } let r = f(1, 2);")
            .unwrap();
    check(program).unwrap();
}
