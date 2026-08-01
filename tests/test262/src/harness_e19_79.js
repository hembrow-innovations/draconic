// E19.79: nativeErrors.js + verifyCallableProperty / verifyAccessorProperty
// (+ primordial aliases) for Error.prototype.stack prop-desc tests.

var nativeErrors = [
  Error,
  EvalError,
  RangeError,
  ReferenceError,
  SyntaxError,
  TypeError,
  URIError
];

var allErrorConstructors = nativeErrors.slice();
if (typeof AggregateError !== "undefined") {
  allErrorConstructors.push(AggregateError);
}
if (typeof SuppressedError !== "undefined") {
  allErrorConstructors.push(SuppressedError);
}

function makeNativeError(Ctor, useNew) {
  if (typeof AggregateError !== "undefined" && Ctor === AggregateError) {
    return useNew
      ? new AggregateError([new Error("inner")], "msg")
      : AggregateError([new Error("inner")], "msg");
  }
  if (typeof SuppressedError !== "undefined" && Ctor === SuppressedError) {
    return useNew
      ? new SuppressedError(new Error("inner"), new Error("suppressed"), "msg")
      : SuppressedError(new Error("inner"), new Error("suppressed"), "msg");
  }
  return useNew ? new Ctor("msg") : Ctor("msg");
}

function verifyCallableProperty(obj, name, functionName, functionLength, desc, options) {
  let label = (options && options.label) || String(name);
  let propertyVerifier = (options && options.verifyProperty) || verifyProperty;
  let value = obj && obj[name];
  assert.sameValue(typeof value, "function", label + " should be a function");
  if (desc === undefined) {
    desc = {
      writable: true,
      enumerable: false,
      configurable: true,
      value: value
    };
  } else if (!__phHasOwnProperty(desc, "value") && !__phHasOwnProperty(desc, "get")) {
    desc.value = value;
  }
  propertyVerifier(obj, name, desc, options);
  if (functionName === undefined) {
    if (typeof name === "symbol") {
      functionName = "[" + name.description + "]";
    } else {
      functionName = name;
    }
  }
  propertyVerifier(
    value,
    "name",
    {
      value: functionName,
      writable: false,
      enumerable: false,
      configurable: desc.configurable
    },
    { label: label + " name", restore: options && options.restore }
  );
  propertyVerifier(
    value,
    "length",
    {
      value: functionLength,
      writable: false,
      enumerable: false,
      configurable: desc.configurable
    },
    { label: label + " length", restore: options && options.restore }
  );
}

function verifyAccessorProperty(obj, name, desc, options) {
  let checkGet = __phHasOwnProperty(desc, "get");
  let checkSet = __phHasOwnProperty(desc, "set");
  assert(checkGet || checkSet, 'verifyAccessorProperty requires at least one of "get" and "set"');
  let label = (options && options.label) || String(name);
  let propertyVerifier = (options && options.verifyProperty) || verifyProperty;
  let callabilityVerifier =
    (options && options.verifyCallableProperty) || verifyCallableProperty;
  let originalDesc = __phGetOwnPropertyDescriptor(obj, name);
  if (checkGet) {
    let expectGetter = desc.get;
    let getterLabel = label + " getter";
    if (expectGetter === undefined || typeof expectGetter === "function") {
      assert.sameValue(originalDesc.get, expectGetter, getterLabel);
    } else {
      let getterName = expectGetter.name;
      if (getterName === undefined) {
        getterName =
          "get " + (typeof name === "symbol" ? "[" + name.description + "]" : name);
      }
      let getterLength = expectGetter.length !== undefined ? expectGetter.length : 0;
      callabilityVerifier(originalDesc, "get", getterName, getterLength, {}, {
        label: getterLabel
      });
    }
  }
  if (checkSet) {
    let expectSetter = desc.set;
    let setterLabel = label + " setter";
    if (expectSetter === undefined || typeof expectSetter === "function") {
      assert.sameValue(originalDesc.set, expectSetter, setterLabel);
    } else {
      let setterName = expectSetter.name;
      if (setterName === undefined) {
        setterName =
          "set " + (typeof name === "symbol" ? "[" + name.description + "]" : name);
      }
      let setterLength = expectSetter.length !== undefined ? expectSetter.length : 1;
      callabilityVerifier(originalDesc, "set", setterName, setterLength, {}, {
        label: setterLabel
      });
    }
  }
  let resolvedDesc = { get: originalDesc.get, set: originalDesc.set };
  if (!__phHasOwnProperty(desc, "enumerable")) {
    resolvedDesc.enumerable = false;
  } else if (desc.enumerable !== undefined) {
    resolvedDesc.enumerable = desc.enumerable;
  }
  if (!__phHasOwnProperty(desc, "configurable")) {
    resolvedDesc.configurable = true;
  } else if (desc.configurable !== undefined) {
    resolvedDesc.configurable = desc.configurable;
  }
  propertyVerifier(obj, name, resolvedDesc, options);
}

var verifyPrimordialProperty = verifyProperty;
function verifyPrimordialCallableProperty(obj, name, functionName, functionLength, desc, options) {
  let resolvedOptions = {
    verifyProperty:
      options && options.verifyProperty !== undefined
        ? options.verifyProperty
        : verifyPrimordialProperty
  };
  if (options && options.label !== undefined) resolvedOptions.label = options.label;
  if (options && options.restore !== undefined) resolvedOptions.restore = options.restore;
  return verifyCallableProperty(
    obj,
    name,
    functionName,
    functionLength,
    desc,
    resolvedOptions
  );
}
function verifyPrimordialAccessorProperty(obj, name, desc, options) {
  let resolvedOptions = {
    verifyProperty:
      options && options.verifyProperty !== undefined
        ? options.verifyProperty
        : verifyPrimordialProperty,
    verifyCallableProperty:
      options && options.verifyCallableProperty !== undefined
        ? options.verifyCallableProperty
        : verifyPrimordialCallableProperty
  };
  if (options && options.label !== undefined) resolvedOptions.label = options.label;
  if (options && options.restore !== undefined) resolvedOptions.restore = options.restore;
  return verifyAccessorProperty(obj, name, desc, resolvedOptions);
}
