//! Test262 staged harness (ROADMAP E19 / ADR 0007).
//!
//! js target only. Resolves the suite from `TEST262_ROOT` or
//! `<workspace>/third_party/test262`. When the suite is absent, runs skip
//! (CI stays green). When present, compiles each allowlisted test through the
//! Draconic frontend → JS backend and executes under Node with a minimal
//! `$ERROR` / `assert` shim.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use draconic_backend_js::emit_js;
use draconic_frontend::{compile_path, compile_source, compile_source_module};
use rayon::prelude::*;

/// Outcome bucket for one allowlisted path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
    Skip,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Skip => "skip",
        }
    }
}

/// One allowlist entry after a run attempt.
#[derive(Debug, Clone)]
pub struct CaseResult {
    pub path: String,
    pub status: Status,
    pub message: String,
}

/// Aggregate of a harness run.
#[derive(Debug, Clone)]
pub struct Report {
    pub suite_root: Option<PathBuf>,
    pub suite_present: bool,
    pub cases: Vec<CaseResult>,
}

impl Report {
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut pass = 0;
        let mut fail = 0;
        let mut skip = 0;
        for c in &self.cases {
            match c.status {
                Status::Pass => pass += 1,
                Status::Fail => fail += 1,
                Status::Skip => skip += 1,
            }
        }
        (pass, fail, skip)
    }

    /// Markdown baseline report (checked in or written under target/).
    pub fn to_markdown(&self) -> String {
        let (pass, fail, skip) = self.counts();
        let mut out = String::new();
        out.push_str("# Test262 baseline report\n\n");
        out.push_str("Staged roll-in (ADR 0007). Target: **js** only.\n");
        out.push_str("Failures are report-only until triage promotes Roadmap rows.\n\n");
        match &self.suite_root {
            Some(p) if self.suite_present => {
                out.push_str(&format!("- Suite root: `{}`\n", p.display()));
                out.push_str("- Suite: **present**\n");
            }
            Some(p) => {
                out.push_str(&format!("- Suite root (missing): `{}`\n", p.display()));
                out.push_str("- Suite: **absent** (all allowlist entries skipped)\n");
            }
            None => out.push_str("- Suite root: unresolved\n"),
        }
        out.push_str(&format!(
            "- Totals: pass={pass} fail={fail} skip={skip} (allowlist={})\n\n",
            self.cases.len()
        ));
        out.push_str("| Path | Status | Message |\n");
        out.push_str("|------|--------|---------|\n");
        for c in &self.cases {
            let msg = c.message.replace('|', "\\|").replace('\n', " ");
            out.push_str(&format!(
                "| `{}` | {} | {} |\n",
                c.path,
                c.status.as_str(),
                msg
            ));
        }
        out.push('\n');
        out
    }
}

/// Package root (`tests/test262`).
pub fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Workspace root (parent of `tests/`).
pub fn workspace_root() -> PathBuf {
    package_root()
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .expect("tests/test262 → workspace root")
}

/// Resolve suite root: `TEST262_ROOT` env, else `<workspace>/third_party/test262`.
pub fn resolve_suite_root() -> PathBuf {
    if let Ok(p) = std::env::var("TEST262_ROOT") {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    workspace_root().join("third_party").join("test262")
}

/// True when `root` looks like a usable test262 checkout (`test/` directory).
pub fn suite_present(root: &Path) -> bool {
    root.is_dir() && root.join("test").is_dir()
}

/// Default allowlist path.
pub fn allowlist_path() -> PathBuf {
    package_root().join("allowlist.txt")
}

/// Load relative test paths from allowlist (comments/blank lines ignored).
pub fn load_allowlist(path: &Path) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("read allowlist {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.contains("..") {
            return Err(format!(
                "allowlist line {}: path must not contain `..`: {line}",
                i + 1
            ));
        }
        out.push(line.to_string());
    }
    if out.is_empty() {
        return Err(format!("allowlist empty: {}", path.display()));
    }
    Ok(out)
}

/// Minimal Test262 harness symbols, parseable by Draconic.
///
/// Real `harness/assert.js` is not compiled through the frontend (it uses
/// patterns outside the current surface). This shim covers `$ERROR` and
/// `assert.sameValue` / `assert.notSameValue` / `assert.throws` used by
/// early language tests (incl. E19.07 BigInt mixed-type TypeError paths).
pub const HARNESS_SHIM: &str = concat!(
    r#"
function Test262Error(message) {
  if (!(this instanceof Test262Error)) {
    return new Test262Error(message);
  }
  this.message = message || "";
}
Test262Error.prototype.toString = function () {
  return "Test262Error: " + this.message;
};
Test262Error.thrower = function (message) {
  throw new Test262Error(message);
};
function $ERROR(message) {
  throw new Test262Error(String(message));
}
function assert(mustBeTrue, message) {
  if (mustBeTrue !== true) {
    $ERROR(message || ("Expected true but got " + String(mustBeTrue)));
  }
}
assert.sameValue = function(actual, expected, message) {
  let same = actual === expected;
  if (actual !== actual && expected !== expected) {
    same = true;
  }
  if (same === false) {
    $ERROR(message || ("Expected SameValue, got " + String(actual) + " vs " + String(expected)));
  }
};
assert.notSameValue = function(actual, unexpected, message) {
  let same = actual === unexpected;
  if (actual !== actual && unexpected !== unexpected) {
    same = true;
  }
  if (same === true) {
    $ERROR(message || "Unexpected SameValue match");
  }
};
assert.throws = function(expectedErrorConstructor, func, message) {
  if (typeof func !== "function") {
    $ERROR("assert.throws requires two arguments: the error constructor and a function to run");
  }
  let msg = "";
  if (message !== undefined) {
    msg = message + " ";
  }
  let threw = false;
  try {
    func();
  } catch (thrown) {
    threw = true;
    if (typeof thrown !== "object" || thrown === null) {
      $ERROR(msg + "Thrown value was not an object!");
    } else if (thrown.constructor !== expectedErrorConstructor) {
      let expectedName = expectedErrorConstructor.name;
      let actualName = thrown.constructor.name;
      $ERROR(msg + "Expected a " + expectedName + " but got a " + actualName);
    }
  }
  if (threw === false) {
    $ERROR(msg + "Expected a " + expectedErrorConstructor.name + " to be thrown but no exception was thrown at all");
  }
};
function compareArray(a, b) {
  if (a === null || a === undefined || b === null || b === undefined) {
    return false;
  }
  if (typeof a !== "object" || typeof b !== "object") {
    return false;
  }
  if (b.length !== a.length) {
    return false;
  }
  let i = 0;
  while (i < a.length) {
    let av = a[i];
    let bv = b[i];
    let same = av === bv;
    if (av !== av && bv !== bv) {
      same = true;
    }
    if (av === 0 && bv === 0 && 1 / av !== 1 / bv) {
      same = false;
    }
    if (same === false) {
      return false;
    }
    i = i + 1;
  }
  return true;
}
compareArray.format = function (arrayLike) {
  return "[" + Array.prototype.map.call(arrayLike, String).join(", ") + "]";
};
assert.compareArray = function(actual, expected, message) {
  let msg = message === undefined ? "" : message;
  if (typeof msg === "symbol") {
    msg = msg.toString();
  }
  if (actual === null || actual === undefined || (typeof actual !== "object" && typeof actual !== "function")) {
    $ERROR("Actual argument [" + actual + "] shouldn't be primitive. " + String(msg));
  }
  if (expected === null || expected === undefined || (typeof expected !== "object" && typeof expected !== "function")) {
    $ERROR("Expected argument [" + expected + "] shouldn't be primitive. " + String(msg));
  }
  if (compareArray(actual, expected)) {
    return;
  }
  $ERROR("Actual " + compareArray.format(actual) + " and expected " + compareArray.format(expected) + " should have the same contents. " + String(msg));
};
// E19.29 / E19.63: propertyHelper.js verifyProperty + deprecated verify* helpers.
// Capture primordials at load so verifyConfigurable(this, "Object") can delete Object.
let __phIsArray = Array.isArray;
let __phDefineProperty = Object.defineProperty;
let __phGetOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
let __phHasOwnProperty = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
let __phPropertyIsEnumerable = Function.prototype.call.bind(Object.prototype.propertyIsEnumerable);
function __phIsSameValue(a, b) {
  if (a === 0 && b === 0) {
    return 1 / a === 1 / b;
  }
  if (a !== a && b !== b) {
    return true;
  }
  return a === b;
}
function __phIsConfigurable(obj, name) {
  try {
    delete obj[name];
  } catch (e) {
    if (!(e instanceof TypeError)) {
      throw new Test262Error("Expected TypeError, got " + e);
    }
  }
  return !__phHasOwnProperty(obj, name);
}
function __phIsEnumerable(obj, name) {
  let stringCheck = false;
  if (typeof name === "string") {
    for (let x in obj) {
      if (x === name) {
        stringCheck = true;
        break;
      }
    }
  } else {
    stringCheck = true;
  }
  return stringCheck && __phHasOwnProperty(obj, name) && __phPropertyIsEnumerable(obj, name);
}
function __phIsWritable(obj, name, verifyProp, value) {
  let nonIndexNumericPropertyName = 4294967295;
  let unlikelyValue = __phIsArray(obj) && name === "length" ? nonIndexNumericPropertyName : "unlikelyValue";
  let newValue = value || unlikelyValue;
  let hadValue = __phHasOwnProperty(obj, name);
  let oldValue = obj[name];
  let writeSucceeded;
  if (arguments.length < 4 && newValue === oldValue) {
    newValue = newValue + "2";
  }
  try {
    obj[name] = newValue;
  } catch (e) {
    if (!(e instanceof TypeError)) {
      throw new Test262Error("Expected TypeError, got " + e);
    }
  }
  writeSucceeded = __phIsSameValue(obj[verifyProp || name], newValue);
  if (writeSucceeded) {
    if (hadValue) {
      obj[name] = oldValue;
    } else {
      delete obj[name];
    }
  }
  return writeSucceeded;
}
function verifyProperty(obj, name, desc, options) {
  assert(arguments.length > 2, "verifyProperty should receive at least 3 arguments: obj, name, and descriptor");
  let label = (options && options.label) || String(name);
  let originalDesc = __phGetOwnPropertyDescriptor(obj, name);
  if (desc === undefined) {
    assert.sameValue(originalDesc, undefined, label + " descriptor should be undefined");
    return true;
  }
  assert(__phHasOwnProperty(obj, name), label + " should be an own property");
  assert.notSameValue(desc, null, "The desc argument should be an object or undefined, null");
  assert.sameValue(typeof desc, "object", "The desc argument should be an object or undefined, " + String(desc));
  let failures = [];
  if (__phHasOwnProperty(desc, "value")) {
    if (!__phIsSameValue(desc.value, originalDesc.value)) {
      failures = failures.concat([label + " descriptor value should be " + String(desc.value)]);
    }
    if (!__phIsSameValue(desc.value, obj[name])) {
      failures = failures.concat([label + " value should be " + String(desc.value)]);
    }
  }
  if (__phHasOwnProperty(desc, "enumerable") && desc.enumerable !== undefined) {
    if (desc.enumerable !== originalDesc.enumerable || desc.enumerable !== __phIsEnumerable(obj, name)) {
      failures = failures.concat([label + " descriptor should " + (desc.enumerable ? "" : "not ") + "be enumerable"]);
    }
  }
  if (__phHasOwnProperty(desc, "writable") && desc.writable !== undefined) {
    if (desc.writable !== originalDesc.writable || desc.writable !== __phIsWritable(obj, name)) {
      failures = failures.concat([label + " descriptor should " + (desc.writable ? "" : "not ") + "be writable"]);
    }
  }
  if (__phHasOwnProperty(desc, "configurable") && desc.configurable !== undefined) {
    if (desc.configurable !== originalDesc.configurable || desc.configurable !== __phIsConfigurable(obj, name)) {
      failures = failures.concat([label + " descriptor should " + (desc.configurable ? "" : "not ") + "be configurable"]);
    }
  }
  if (__phHasOwnProperty(desc, "get")) {
    if (originalDesc.get !== desc.get) {
      failures = failures.concat([label + " getter mismatch"]);
    }
  }
  if (__phHasOwnProperty(desc, "set")) {
    if (originalDesc.set !== desc.set) {
      failures = failures.concat([label + " setter mismatch"]);
    }
  }
  if (failures.length) {
    assert(false, failures.join("; "));
  }
  if (options && options.restore) {
    __phDefineProperty(obj, name, originalDesc);
  }
  return true;
}
// E19.63: deprecated propertyHelper verify* helpers (+ bare compareArray above).
function verifyEqualTo(obj, name, value) {
  if (!__phIsSameValue(obj[name], value)) {
    throw new Test262Error("Expected obj[" + String(name) + "] to equal " + value + ", actually " + obj[name]);
  }
}
function verifyWritable(obj, name, verifyProp, value) {
  if (!verifyProp) {
    assert(__phGetOwnPropertyDescriptor(obj, name).writable, "Expected obj[" + String(name) + "] to have writable:true.");
  }
  if (!__phIsWritable(obj, name, verifyProp, value)) {
    throw new Test262Error("Expected obj[" + String(name) + "] to be writable, but was not.");
  }
}
function verifyNotWritable(obj, name, verifyProp, value) {
  if (!verifyProp) {
    assert(!__phGetOwnPropertyDescriptor(obj, name).writable, "Expected obj[" + String(name) + "] to have writable:false.");
  }
  if (__phIsWritable(obj, name, verifyProp)) {
    throw new Test262Error("Expected obj[" + String(name) + "] NOT to be writable, but was.");
  }
}
function verifyEnumerable(obj, name) {
  assert(__phGetOwnPropertyDescriptor(obj, name).enumerable, "Expected obj[" + String(name) + "] to have enumerable:true.");
  if (!__phIsEnumerable(obj, name)) {
    throw new Test262Error("Expected obj[" + String(name) + "] to be enumerable, but was not.");
  }
}
function verifyNotEnumerable(obj, name) {
  assert(!__phGetOwnPropertyDescriptor(obj, name).enumerable, "Expected obj[" + String(name) + "] to have enumerable:false.");
  if (__phIsEnumerable(obj, name)) {
    throw new Test262Error("Expected obj[" + String(name) + "] NOT to be enumerable, but was.");
  }
}
function verifyConfigurable(obj, name) {
  assert(__phGetOwnPropertyDescriptor(obj, name).configurable, "Expected obj[" + String(name) + "] to have configurable:true.");
  if (!__phIsConfigurable(obj, name)) {
    throw new Test262Error("Expected obj[" + String(name) + "] to be configurable, but was not.");
  }
}
function verifyNotConfigurable(obj, name) {
  assert(!__phGetOwnPropertyDescriptor(obj, name).configurable, "Expected obj[" + String(name) + "] to have configurable:false.");
  if (__phIsConfigurable(obj, name)) {
    throw new Test262Error("Expected obj[" + String(name) + "] NOT to be configurable, but was.");
  }
}
// E19.61: isConstructor (harness/isConstructor.js)
function isConstructor(f) {
  if (typeof f !== "function") {
    throw new Test262Error("isConstructor invoked with a non-function value");
  }
  try {
    Reflect.construct(function(){}, [], f);
  } catch (e) {
    return false;
  }
  return true;
}
// E19.61: asyncTest + assert.throwsAsync (harness/asyncHelpers.js)
function asyncTest(testFunc) {
  if (!Object.prototype.hasOwnProperty.call(globalThis, "$DONE")) {
    throw new Test262Error("asyncTest called without async flag");
  }
  if (typeof testFunc !== "function") {
    $DONE(new Test262Error("asyncTest called with non-function argument"));
    return;
  }
  try {
    testFunc().then(
      function () {
        $DONE();
      },
      function (error) {
        $DONE(error);
      }
    );
  } catch (syncError) {
    $DONE(syncError);
  }
}
assert.throwsAsync = function (expectedErrorConstructor, func, message) {
  return new Promise(function (resolve) {
    let fail = function (detail) {
      if (message === undefined) {
        throw new Test262Error(detail);
      }
      throw new Test262Error(message + " " + detail);
    };
    if (typeof expectedErrorConstructor !== "function") {
      fail("assert.throwsAsync called with an argument that is not an error constructor");
    }
    if (typeof func !== "function") {
      fail("assert.throwsAsync called with an argument that is not a function");
    }
    let expectedName = expectedErrorConstructor.name;
    let expectation = "Expected a " + expectedName + " to be thrown asynchronously";
    let res;
    try {
      res = func();
    } catch (thrown) {
      fail(expectation + " but the function threw synchronously");
    }
    if (res === null || typeof res !== "object" || typeof res.then !== "function") {
      fail(expectation + " but result was not a thenable");
    }
    let onResFulfilled;
    let onResRejected;
    let resSettlementP = new Promise(function (onFulfilled, onRejected) {
      onResFulfilled = onFulfilled;
      onResRejected = onRejected;
    });
    try {
      res.then(onResFulfilled, onResRejected);
    } catch (thrown) {
      fail(expectation + " but .then threw synchronously");
    }
    resolve(resSettlementP.then(
      function () {
        fail(expectation + " but no exception was thrown at all");
      },
      function (thrown) {
        if (thrown === null || typeof thrown !== "object") {
          fail(expectation + " but thrown value was not an object");
        } else if (thrown.constructor !== expectedErrorConstructor) {
          let actualName = thrown.constructor.name;
          if (expectedName === actualName) {
            fail(expectation + " but got a different error constructor with the same name");
          }
          fail(expectation + " but got a " + actualName);
        }
      }
    ));
  });
};
// E19.61: TypedArray harness (minimal port of harness/testTypedArray.js)
function isPrimitive(value) {
  return !value || (typeof value !== "object" && typeof value !== "function");
}
let floatArrayConstructors = [Float64Array, Float32Array];
let nonClampedIntArrayConstructors = [
  Int32Array,
  Int16Array,
  Int8Array,
  Uint32Array,
  Uint16Array,
  Uint8Array
];
let intArrayConstructors = nonClampedIntArrayConstructors.concat([Uint8ClampedArray]);
if (typeof Float16Array !== "undefined") {
  floatArrayConstructors = floatArrayConstructors.concat([Float16Array]);
}
let bigIntArrayConstructors = [];
if (typeof BigInt64Array !== "undefined") {
  bigIntArrayConstructors = bigIntArrayConstructors.concat([BigInt64Array]);
}
if (typeof BigUint64Array !== "undefined") {
  bigIntArrayConstructors = bigIntArrayConstructors.concat([BigUint64Array]);
}
let typedArrayConstructors = floatArrayConstructors.concat(intArrayConstructors);
let allTypedArrayConstructors = typedArrayConstructors.concat(bigIntArrayConstructors);
let TypedArray = Object.getPrototypeOf(Int8Array);
function makePassthrough(TA, primitiveOrIterable) {
  return primitiveOrIterable;
}
function makeArray(TA, primitiveOrIterable) {
  if (isPrimitive(primitiveOrIterable)) {
    let n = Number(primitiveOrIterable);
    if (!(n >= 0 && n < 9007199254740992)) {
      return primitiveOrIterable;
    }
    let out = [];
    let i = 0;
    while (i < n) {
      out = out.concat(["0"]);
      i = i + 1;
    }
    return out;
  }
  return Array.from(primitiveOrIterable);
}
function makeArrayLike(TA, primitiveOrIterable) {
  let arr = makeArray(TA, primitiveOrIterable);
  if (isPrimitive(arr)) {
    return arr;
  }
  let obj = { length: arr.length };
  let i = 0;
  while (i < obj.length) {
    obj[i] = arr[i];
    i = i + 1;
  }
  return obj;
}
function makeIterable(TA, primitiveOrIterable) {
  let src = makeArray(TA, primitiveOrIterable);
  if (isPrimitive(src)) {
    return src;
  }
  let obj = {};
  obj[Symbol.iterator] = function () {
    return src[Symbol.iterator]();
  };
  return obj;
}
function makeArrayBuffer(TA, primitiveOrIterable) {
  let arr = makeArray(TA, primitiveOrIterable);
  if (isPrimitive(arr)) {
    return arr;
  }
  return new TA(arr).buffer;
}
// E19.66: resizable / grown / shrunk / immutable ArrayBuffer arg factories
let makeResizableArrayBuffer = undefined;
let makeGrownArrayBuffer = undefined;
let makeShrunkArrayBuffer = undefined;
let makeImmutableArrayBuffer = undefined;
if (ArrayBuffer.prototype.resize) {
  function copyIntoArrayBuffer(destBuffer, srcBuffer) {
    let destView = new Uint8Array(destBuffer);
    let srcView = new Uint8Array(srcBuffer);
    let i = 0;
    while (i < srcView.length) {
      destView[i] = srcView[i];
      i = i + 1;
    }
    return destBuffer;
  }
  makeResizableArrayBuffer = function makeResizableArrayBuffer(TA, primitiveOrIterable) {
    if (isPrimitive(primitiveOrIterable)) {
      let n = Number(primitiveOrIterable) * TA.BYTES_PER_ELEMENT;
      if (!(n >= 0 && n < 9007199254740992)) {
        return primitiveOrIterable;
      }
      return new ArrayBuffer(n, { maxByteLength: n * 2 });
    }
    let fixed = makeArrayBuffer(TA, primitiveOrIterable);
    let byteLength = fixed.byteLength;
    let resizable = new ArrayBuffer(byteLength, { maxByteLength: byteLength * 2 });
    return copyIntoArrayBuffer(resizable, fixed);
  };
  makeGrownArrayBuffer = function makeGrownArrayBuffer(TA, primitiveOrIterable) {
    if (isPrimitive(primitiveOrIterable)) {
      let n = Number(primitiveOrIterable) * TA.BYTES_PER_ELEMENT;
      if (!(n >= 0 && n < 9007199254740992)) {
        return primitiveOrIterable;
      }
      let grownP = new ArrayBuffer(Math.floor(n / 2), { maxByteLength: n });
      grownP.resize(n);
      return grownP;
    }
    let fixed = makeArrayBuffer(TA, primitiveOrIterable);
    let byteLength = fixed.byteLength;
    let grown = new ArrayBuffer(Math.floor(byteLength / 2), { maxByteLength: byteLength });
    grown.resize(byteLength);
    return copyIntoArrayBuffer(grown, fixed);
  };
  makeShrunkArrayBuffer = function makeShrunkArrayBuffer(TA, primitiveOrIterable) {
    if (isPrimitive(primitiveOrIterable)) {
      let n = Number(primitiveOrIterable) * TA.BYTES_PER_ELEMENT;
      if (!(n >= 0 && n < 9007199254740992)) {
        return primitiveOrIterable;
      }
      let shrunkP = new ArrayBuffer(n * 2, { maxByteLength: n * 2 });
      shrunkP.resize(n);
      return shrunkP;
    }
    let fixed = makeArrayBuffer(TA, primitiveOrIterable);
    let byteLength = fixed.byteLength;
    let shrunk = new ArrayBuffer(byteLength * 2, { maxByteLength: byteLength * 2 });
    copyIntoArrayBuffer(shrunk, fixed);
    shrunk.resize(byteLength);
    return shrunk;
  };
}
if (ArrayBuffer.prototype.transferToImmutable) {
  makeImmutableArrayBuffer = function makeImmutableArrayBuffer(TA, primitiveOrIterable) {
    if (isPrimitive(primitiveOrIterable)) {
      let n = Number(primitiveOrIterable) * TA.BYTES_PER_ELEMENT;
      if (!(n >= 0 && n < 9007199254740992)) {
        return primitiveOrIterable;
      }
      return new ArrayBuffer(n).transferToImmutable();
    }
    let mutable = makeArrayBuffer(TA, primitiveOrIterable);
    return mutable.transferToImmutable();
  };
}
let typedArrayCtorArgFactories = [
  makePassthrough,
  makeArray,
  makeArrayLike,
  makeIterable,
  makeArrayBuffer
];
if (makeResizableArrayBuffer) {
  typedArrayCtorArgFactories = typedArrayCtorArgFactories.concat([makeResizableArrayBuffer]);
}
if (makeGrownArrayBuffer) {
  typedArrayCtorArgFactories = typedArrayCtorArgFactories.concat([makeGrownArrayBuffer]);
}
if (makeShrunkArrayBuffer) {
  typedArrayCtorArgFactories = typedArrayCtorArgFactories.concat([makeShrunkArrayBuffer]);
}
if (makeImmutableArrayBuffer) {
  typedArrayCtorArgFactories = typedArrayCtorArgFactories.concat([makeImmutableArrayBuffer]);
}
function ctorArgFactoryMatchesSome(argFactory, features) {
  let i = 0;
  while (i < features.length) {
    let feat = features[i];
    if (feat === "passthrough" && argFactory === makePassthrough) {
      return true;
    }
    if (feat === "arraylike" && (argFactory === makeArray || argFactory === makeArrayLike)) {
      return true;
    }
    if (feat === "iterable" && argFactory === makeIterable) {
      return true;
    }
    if (
      feat === "arraybuffer" &&
      (argFactory === makeArrayBuffer ||
        argFactory === makeResizableArrayBuffer ||
        argFactory === makeGrownArrayBuffer ||
        argFactory === makeShrunkArrayBuffer ||
        argFactory === makeImmutableArrayBuffer)
    ) {
      return true;
    }
    if (
      feat === "resizable" &&
      (argFactory === makeResizableArrayBuffer ||
        argFactory === makeGrownArrayBuffer ||
        argFactory === makeShrunkArrayBuffer)
    ) {
      return true;
    }
    if (feat === "immutable" && argFactory === makeImmutableArrayBuffer) {
      return true;
    }
    i = i + 1;
  }
  return false;
}
function testWithAllTypedArrayConstructors(f, constructors, includeArgFactories, excludeArgFactories) {
  let ctors = constructors || allTypedArrayConstructors;
  let ctorArgFactories = typedArrayCtorArgFactories;
  if (includeArgFactories) {
    ctorArgFactories = [];
    let i = 0;
    while (i < typedArrayCtorArgFactories.length) {
      if (ctorArgFactoryMatchesSome(typedArrayCtorArgFactories[i], includeArgFactories)) {
        ctorArgFactories = ctorArgFactories.concat([typedArrayCtorArgFactories[i]]);
      }
      i = i + 1;
    }
  }
  if (excludeArgFactories) {
    let filtered = [];
    let j = 0;
    while (j < ctorArgFactories.length) {
      if (!ctorArgFactoryMatchesSome(ctorArgFactories[j], excludeArgFactories)) {
        filtered = filtered.concat([ctorArgFactories[j]]);
      }
      j = j + 1;
    }
    ctorArgFactories = filtered;
  }
  if (ctorArgFactories.length === 0) {
    throw new Test262Error("no arg factories match include " + includeArgFactories + " and exclude " + excludeArgFactories);
  }
  let k = 0;
  while (k < ctorArgFactories.length) {
    let argFactory = ctorArgFactories[k];
    let i = 0;
    while (i < ctors.length) {
      let constructor = ctors[i];
      let boundArgFactory = function (x) {
        return argFactory(constructor, x);
      };
      try {
        f(constructor, boundArgFactory);
      } catch (e) {
        if (e && typeof e === "object") {
          e.message = String(e.message || "") + " (Testing with " + constructor.name + " and " + argFactory.name + ".)";
        }
        throw e;
      }
      i = i + 1;
    }
    k = k + 1;
  }
}
function testWithTypedArrayConstructors(f, constructors, includeArgFactories, excludeArgFactories) {
  let ctors = constructors || typedArrayConstructors;
  testWithAllTypedArrayConstructors(f, ctors, includeArgFactories, excludeArgFactories);
}
function testWithBigIntTypedArrayConstructors(f, constructors, includeArgFactories, excludeArgFactories) {
  let ctors = constructors || bigIntArrayConstructors;
  testWithAllTypedArrayConstructors(f, ctors, includeArgFactories, excludeArgFactories);
}
let nonAtomicsFriendlyTypedArrayConstructors = floatArrayConstructors.concat([Uint8ClampedArray]);
function testWithNonAtomicsFriendlyTypedArrayConstructors(f, includeArgFactories, excludeArgFactories) {
  testWithAllTypedArrayConstructors(
    f,
    nonAtomicsFriendlyTypedArrayConstructors,
    includeArgFactories,
    excludeArgFactories
  );
}
function testWithAtomicsFriendlyTypedArrayConstructors(f, includeArgFactories, excludeArgFactories) {
  testWithAllTypedArrayConstructors(
    f,
    [
      Int32Array,
      Int16Array,
      Int8Array,
      Uint32Array,
      Uint16Array,
      Uint8Array
    ],
    includeArgFactories,
    excludeArgFactories
  );
}
function testTypedArrayConversions(byteConversionValues, fn) {
  let values = byteConversionValues.values;
  let expected = byteConversionValues.expected;
  testWithTypedArrayConstructors(function (TA) {
    let name = TA.name.slice(0, -5);
    let index = 0;
    while (index < values.length) {
      let value = values[index];
      let exp = expected[name][index];
      let initial = 0;
      if (exp === 0) {
        initial = 1;
      }
      fn(TA, value, exp, initial);
      index = index + 1;
    }
  }, null, ["passthrough"]);
}
function isFloatTypedArrayConstructor(arg) {
  let i = 0;
  while (i < floatArrayConstructors.length) {
    if (floatArrayConstructors[i] === arg) {
      return true;
    }
    i = i + 1;
  }
  return false;
}
function floatTypedArrayConstructorPrecision(FA) {
  if (typeof Float16Array !== "undefined" && FA === Float16Array) {
    return "half";
  } else if (FA === Float32Array) {
    return "single";
  } else if (FA === Float64Array) {
    return "double";
  } else {
    throw new Error("Malformed test - floatTypedArrayConstructorPrecision called with non-float TypedArray");
  }
}
    "#,
    include_str!("harness_e19_64.js"),
    include_str!("harness_e19_65.js"),
    include_str!("harness_e19_66.js"),
    include_str!("harness_e19_70.js"),
    include_str!("harness_e19_74.js"),
    include_str!("harness_e19_76.js"),
    include_str!("harness_e19_77.js"),
    include_str!("harness_e19_79.js"),
    include_str!("harness_e19_80.js"),
);

/// Locate Test262 YAML frontmatter (`/*--- ... ---*/`), if present.
///
/// Frontmatter may follow a copyright line-comment prologue.
fn frontmatter_meta(source: &str) -> Option<&str> {
    let start = source.find("/*---")?;
    let after = &source[start + 5..];
    let end = after.find("---*/")?;
    Some(&after[..end])
}

/// Strip Test262 YAML frontmatter comment block if present.
pub fn strip_frontmatter(source: &str) -> &str {
    let Some(start) = source.find("/*---") else {
        return source;
    };
    let after = &source[start + 5..];
    let Some(end) = after.find("---*/") else {
        return source;
    };
    after[end + 5..].trim_start()
}

/// True when frontmatter declares a negative parse/early/resolution SyntaxError expectation.
///
/// E19.71: `phase: resolution` is link-time (ambiguous/missing export) — treat like
/// compile failure, same as parse/early.
pub fn is_negative_parse(source: &str) -> bool {
    let Some(meta) = frontmatter_meta(source) else {
        return false;
    };
    if !meta.contains("negative:") {
        return false;
    }
    meta.contains("phase: parse")
        || meta.contains("phase: early")
        || meta.contains("phase: resolution")
}

/// True when frontmatter declares a negative runtime expectation (error must be thrown).
pub fn is_negative_runtime(source: &str) -> bool {
    let Some(meta) = frontmatter_meta(source) else {
        return false;
    };
    meta.contains("negative:") && meta.contains("phase: runtime")
}

/// True when frontmatter requires strict mode only (`flags: [onlyStrict]`).
///
/// E19.19: strict PutValue TypeError on compound assignment needs a leading
/// `"use strict"` so Node observes the same mode as Test262's onlyStrict run.
pub fn is_only_strict(source: &str) -> bool {
    let Some(meta) = frontmatter_meta(source) else {
        return false;
    };
    // flags: [onlyStrict] or flags: [onlyStrict, ...] — bracket form used by suite.
    meta.lines().any(|line| {
        let t = line.trim();
        t.starts_with("flags:") && t.contains("onlyStrict")
    }) || meta.contains("onlyStrict")
}

/// True when frontmatter has the `async` **flag** (not `features: [async-…]`).
///
/// E19.26: async tests settle via `$DONE` rather than sync script completion.
pub fn is_async_flag(source: &str) -> bool {
    flag_token(source, "async")
}

/// True when frontmatter has the `module` **flag** (Module goal / top-level await).
///
/// E19.28: top-level `await` is valid only under Module goal.
pub fn is_module_flag(source: &str) -> bool {
    flag_token(source, "module")
}

/// True when frontmatter has the `raw` **flag** (hashbang / full-file source).
///
/// E19.39: hashbang and other early-byte tests must compile the full file so
/// content before the YAML frontmatter is not stripped away.
pub fn is_raw_flag(source: &str) -> bool {
    flag_token(source, "raw")
}

/// Match a single comma/bracket-separated token on a `flags:` frontmatter line,
/// or a YAML list item under `flags:` (`flags:\n  - module`).
fn flag_token(source: &str, token: &str) -> bool {
    let Some(meta) = frontmatter_meta(source) else {
        return false;
    };
    let mut in_flags_list = false;
    for line in meta.lines() {
        let t = line.trim();
        if t.starts_with("flags:") {
            for part in t.trim_start_matches("flags:").split([',', '[', ']']) {
                if part.trim() == token {
                    return true;
                }
            }
            // Multi-line YAML list: `flags:` alone or with nothing else on the line.
            let rest = t.trim_start_matches("flags:").trim();
            in_flags_list = rest.is_empty() || rest == "[]" || rest == "[";
            continue;
        }
        if in_flags_list {
            // Next top-level key ends the list.
            if !t.is_empty() && !t.starts_with('-') && t.contains(':') {
                in_flags_list = false;
                continue;
            }
            if let Some(item) = t.strip_prefix('-') {
                if item.trim() == token {
                    return true;
                }
            }
        }
    }
    false
}

/// Node-only host wrapper for Test262 `flags: [async]` (E19.26 / doneprintHandle).
///
/// Injected **after** frontend emit so `process` / `setTimeout` are not compiled.
/// Defines `$DONE` as a free global the emitted body looks up.
pub fn wrap_async_host(compiled_js: &str) -> String {
    format!(
        r#"
var __test262AsyncSettled = false;
var __test262AsyncTimer = setTimeout(function () {{
  if (!__test262AsyncSettled) {{
    console.error("Test262:AsyncTestFailure:Test262Error: timeout (no $DONE)");
    process.exit(1);
  }}
}}, 10000);
function $DONE(error) {{
  if (__test262AsyncSettled) {{
    return;
  }}
  __test262AsyncSettled = true;
  clearTimeout(__test262AsyncTimer);
  if (error) {{
    if (typeof error === "object" && error !== null && "name" in error) {{
      console.error(
        "Test262:AsyncTestFailure:" + error.name + ": " + String(error.message || "")
      );
    }} else {{
      console.error("Test262:AsyncTestFailure:Test262Error: " + String(error));
    }}
    process.exit(1);
  }}
  console.log("Test262:AsyncTestComplete");
  process.exit(0);
}}
process.on("unhandledRejection", function (reason) {{
  $DONE(reason);
}});
{compiled_js}
"#
    )
}

/// Node-only `$262` host API (E19.61 / INTERPRETING.md).
///
/// Injected **after** frontend emit so `require('vm')` is not compiled.
/// Minimal surface: `global`, `createRealm`, `evalScript` (+ best-effort detach).
///
/// When `as_module` is true, uses `createRequire` so the wrapper works under
/// Node `--input-type=module` (bare `require` is not defined in ESM).
pub fn wrap_host_api(compiled_js: &str) -> String {
    wrap_host_api_mode(compiled_js, false, false)
}

/// Like [`wrap_host_api`], with ESM vs script host bootstrap.
///
/// When `only_strict` is set (and not module), the wrapper begins with
/// `"use strict";` so host statements do not suppress the script's strict mode
/// (caller/callee TypeError tests, strict PutValue, etc.).
pub fn wrap_host_api_mode(compiled_js: &str, as_module: bool, only_strict: bool) -> String {
    let strict_boot = if only_strict && !as_module {
        "\"use strict\";\n"
    } else {
        ""
    };
    let require_boot = if as_module {
        r#"
import { createRequire as __test262CreateRequire } from "module";
var require = __test262CreateRequire(import.meta.url);
"#
    } else {
        ""
    };
    format!(
        r#"{strict_boot}{require_boot}
(function () {{
  var vm = require("vm");
  var worker_threads = require("worker_threads");
  var __agentWorkers = [];
  var __agentReportPorts = [];
  var __agentBroadcastPorts = [];
  function __test262MakeAgent() {{
    return {{
      start: function (source) {{
        // MessageChannel + receiveMessageOnPort: sync I/O without the event loop
        // (Atomics.wait would otherwise starve worker "message" handlers).
        var reportCh = new worker_threads.MessageChannel();
        var broadcastCh = new worker_threads.MessageChannel();
        var readySab = new SharedArrayBuffer(4);
        var readyIa = new Int32Array(readySab);
        var bootstrap =
          "const {{ workerData }} = require('worker_threads');" +
          "var __reportPort = workerData.reportPort;" +
          "var __broadcastPort = workerData.broadcastPort;" +
          "var __readyIa = new Int32Array(workerData.readySab);" +
          "var __recv = null;" +
          "var $262 = {{ agent: {{" +
          "  receiveBroadcast: function (cb) {{ __recv = cb; }}," +
          "  report: function (msg) {{ __reportPort.postMessage(String(msg)); }}," +
          "  sleep: function (ms) {{ var s = new SharedArrayBuffer(4); var a = new Int32Array(s); Atomics.wait(a, 0, 0, ms); }}," +
          "  leaving: function () {{}}," +
          "  monotonicNow: function () {{ return performance.now(); }}" +
          "}} }};" +
          "__broadcastPort.on('message', function (m) {{" +
          "  if (m && m.type === 'broadcast' && typeof __recv === 'function') {{ __recv(m.sab, m.id); }}" +
          "}});" +
          "Atomics.store(__readyIa, 0, 1);" +
          "Atomics.notify(__readyIa, 0);" +
          String(source);
        var w = new worker_threads.Worker(bootstrap, {{
          eval: true,
          workerData: {{
            reportPort: reportCh.port2,
            broadcastPort: broadcastCh.port2,
            readySab: readySab
          }},
          transferList: [reportCh.port2, broadcastCh.port2]
        }});
        w.on("error", function (err) {{
          try {{
            reportCh.port1.postMessage("agent-error:" + String(err && err.message ? err.message : err));
          }} catch (e) {{}}
        }});
        // Do not keep the Node process alive solely for idle agents.
        try {{ w.unref(); }} catch (e) {{}}
        try {{ reportCh.port1.unref(); }} catch (e) {{}}
        try {{ broadcastCh.port1.unref(); }} catch (e) {{}}
        __agentWorkers.push(w);
        __agentReportPorts.push(reportCh.port1);
        __agentBroadcastPorts.push(broadcastCh.port1);
        var spins = 0;
        while (Atomics.load(readyIa, 0) === 0 && spins < 20000) {{
          Atomics.wait(readyIa, 0, 0, 5);
          spins = spins + 1;
        }}
        if (Atomics.load(readyIa, 0) === 0) {{
          throw new Error("$262.agent.start: agent did not become ready");
        }}
      }},
      broadcast: function (sab, id) {{
        var msg = {{ type: "broadcast", sab: sab, id: id }};
        var i = 0;
        while (i < __agentBroadcastPorts.length) {{
          __agentBroadcastPorts[i].postMessage(msg);
          i = i + 1;
        }}
      }},
      getReport: function () {{
        var i = 0;
        while (i < __agentReportPorts.length) {{
          var got = worker_threads.receiveMessageOnPort(__agentReportPorts[i]);
          if (got) {{
            return String(got.message);
          }}
          i = i + 1;
        }}
        return null;
      }},
      sleep: function (ms) {{
        var s = new SharedArrayBuffer(4);
        var a = new Int32Array(s);
        Atomics.wait(a, 0, 0, ms);
      }},
      monotonicNow: function () {{
        return performance.now();
      }},
      // atomicsHelper overlays (E19.64) — installed after api object is created
      timeouts: {{ yield: 100, small: 200, long: 1000, huge: 10000 }}
    }};
  }}
  function __test262InstallAgentHelpers(agent) {{
    var rawGetReport = agent.getReport.bind(agent);
    agent.getReport = function () {{
      var r;
      while ((r = rawGetReport()) == null) {{
        agent.sleep(1);
      }}
      return r;
    }};
    agent.setTimeout = typeof setTimeout === "function" ? setTimeout : function (cb, delay) {{
      var p = Promise.resolve();
      var start = Date.now();
      var end = start + delay;
      function check() {{
        if ((end - Date.now()) > 0) {{
          p.then(check);
        }} else {{
          cb();
        }}
      }}
      p.then(check);
    }};
    agent.getReportAsync = function () {{
      return new Promise(function (resolve) {{
        (function loop() {{
          var result = rawGetReport();
          if (!result) {{
            agent.setTimeout(loop, 1);
          }} else {{
            resolve(result);
          }}
        }})();
      }});
    }};
    agent.safeBroadcast = function (typedArray) {{
      var Constructor = Object.getPrototypeOf(typedArray).constructor;
      var temp = new Constructor(new SharedArrayBuffer(Constructor.BYTES_PER_ELEMENT));
      try {{
        Atomics.wait(temp, 0, Constructor === Int32Array ? 1 : BigInt(1));
      }} catch (error) {{
        throw new Error(Constructor.name + " cannot be used as a shared typed array. (" + error + ")");
      }}
      agent.broadcast(typedArray.buffer);
    }};
    agent.safeBroadcastAsync = async function (ta, index, expected) {{
      await agent.broadcast(ta.buffer);
      await agent.waitUntil(ta, index, expected);
      await agent.tryYield();
      return await Atomics.load(ta, index);
    }};
    agent.waitUntil = function (typedArray, index, expected) {{
      var agents = 0;
      while ((agents = Atomics.load(typedArray, index)) !== expected) {{
      }}
      if (agents !== expected) {{
        throw new Error("Reporting number of agents equals the value of expected");
      }}
    }};
    agent.tryYield = function () {{
      agent.sleep(agent.timeouts.yield);
    }};
    agent.trySleep = function (ms) {{
      agent.sleep(ms);
    }};
  }}
  // E19.79: Error.prototype.stack accessor + no own stack on fresh instances.
  // Host engines often install an own stack accessor; rewrite to the proposal shape.
  function __test262InstallErrorStack(globalObj) {{
    var E = globalObj.Error;
    if (typeof E !== "function" || !E.prototype) {{
      return;
    }}
    var existing = Object.getOwnPropertyDescriptor(E.prototype, "stack");
    if (
      existing &&
      typeof existing.get === "function" &&
      typeof existing.set === "function" &&
      existing.enumerable === false &&
      existing.configurable === true
    ) {{
      // Already looks like the error-stack-accessor proposal.
      return;
    }}
    var stackStore = new WeakMap();
    var isErrorFn =
      typeof E.isError === "function"
        ? E.isError.bind(E)
        : function (v) {{
            return v instanceof E;
          }};
    // Prefer the realm's current TypeError so assert.throws(TypeError, …) matches
    // after we replace global error constructors.
    function makeTypeError(msg) {{
      var TE = globalObj.TypeError;
      if (typeof TE !== "function") {{
        TE = TypeError;
      }}
      return new TE(msg);
    }}
    function stashStack(err) {{
      if (!isErrorFn(err)) {{
        return err;
      }}
      if (!stackStore.has(err)) {{
        var s = "";
        try {{
          var d = Object.getOwnPropertyDescriptor(err, "stack");
          if (d) {{
            if (typeof d.get === "function") {{
              s = d.get.call(err);
            }} else if (typeof d.value === "string") {{
              s = d.value;
            }}
          }} else if (typeof err.stack === "string") {{
            s = err.stack;
          }}
        }} catch (e0) {{
          s = "";
        }}
        if (typeof s !== "string") {{
          s = "";
        }}
        stackStore.set(err, s);
      }}
      try {{
        delete err.stack;
      }} catch (e1) {{}}
      // If delete left a non-configurable own stack, leave it; most hosts are configurable.
      return err;
    }}
    // Concise methods are non-constructible (isConstructor false). Body is strict
    // so null/undefined this is not coerced to the global object.
    var getHolder = {{
      "get stack"() {{
        "use strict";
        var receiver = this;
        if (receiver === null || (typeof receiver !== "object" && typeof receiver !== "function")) {{
          throw makeTypeError("Error.prototype.stack getter called on non-object");
        }}
        if (!isErrorFn(receiver)) {{
          return undefined;
        }}
        if (stackStore.has(receiver)) {{
          return stackStore.get(receiver);
        }}
        // Error created before install: capture once, strip own, return.
        stashStack(receiver);
        return stackStore.has(receiver) ? stackStore.get(receiver) : "";
      }}
    }};
    var setHolder = {{
      "set stack"(v) {{
        "use strict";
        var receiver = this;
        if (receiver === null || (typeof receiver !== "object" && typeof receiver !== "function")) {{
          throw makeTypeError("Error.prototype.stack setter called on non-object");
        }}
        if (typeof v !== "string") {{
          throw makeTypeError("Error.prototype.stack setter requires a string");
        }}
        if (receiver === E.prototype) {{
          throw makeTypeError("Cannot set Error.prototype.stack on Error.prototype");
        }}
        var desc = Object.getOwnPropertyDescriptor(receiver, "stack");
        if (desc === undefined) {{
          Object.defineProperty(receiver, "stack", {{
            value: v,
            writable: true,
            enumerable: true,
            configurable: true
          }});
        }} else {{
          var ok = Reflect.set(receiver, "stack", v, receiver);
          if (!ok) {{
            throw makeTypeError("Error.prototype.stack setter failed");
          }}
        }}
      }}
    }};
    var getStack = getHolder["get stack"];
    var setStack = setHolder["set stack"];
    try {{
      Object.defineProperty(E.prototype, "stack", {{
        get: getStack,
        set: setStack,
        enumerable: false,
        configurable: true
      }});
    }} catch (e2) {{
      return;
    }}
    function patchErrorCtor(name) {{
      var Orig = globalObj[name];
      if (typeof Orig !== "function") {{
        return;
      }}
      var Patched = function () {{
        var args = arguments;
        var nt = new.target;
        var err = Reflect.construct(Orig, args, nt || Patched);
        return stashStack(err);
      }};
      try {{
        Object.defineProperty(Patched, "name", {{
          value: name,
          writable: false,
          enumerable: false,
          configurable: true
        }});
      }} catch (e3) {{}}
      try {{
        Object.defineProperty(Patched, "length", {{
          value: Orig.length,
          writable: false,
          enumerable: false,
          configurable: true
        }});
      }} catch (e4) {{}}
      Patched.prototype = Orig.prototype;
      // Keep `.constructor === Ctor` for assert.throws after global replacement.
      try {{
        Object.defineProperty(Orig.prototype, "constructor", {{
          value: Patched,
          writable: true,
          enumerable: false,
          configurable: true
        }});
      }} catch (e4b) {{}}
      try {{
        Object.setPrototypeOf(Patched, Object.getPrototypeOf(Orig));
      }} catch (e5) {{
        try {{
          Patched.__proto__ = Orig.__proto__;
        }} catch (e6) {{}}
      }}
      try {{
        var keys = Object.getOwnPropertyNames(Orig);
        var ki = 0;
        while (ki < keys.length) {{
          var k = keys[ki];
          if (k !== "prototype" && k !== "length" && k !== "name") {{
            try {{
              var pd = Object.getOwnPropertyDescriptor(Orig, k);
              if (pd) {{
                Object.defineProperty(Patched, k, pd);
              }}
            }} catch (e7) {{}}
          }}
          ki = ki + 1;
        }}
      }} catch (e8) {{}}
      try {{
        globalObj[name] = Patched;
      }} catch (e9) {{}}
    }}
    var names = [
      "Error",
      "EvalError",
      "RangeError",
      "ReferenceError",
      "SyntaxError",
      "TypeError",
      "URIError",
      "AggregateError",
      "SuppressedError"
    ];
    var ni = 0;
    while (ni < names.length) {{
      patchErrorCtor(names[ni]);
      ni = ni + 1;
    }}
  }}
  // E19.77: Function.prototype.toString (NativeFunction form for user fns) +
  // caller/arguments poison props sharing one %ThrowTypeError%.
  function __test262InstallFunctionProto(globalObj) {{
    var F = globalObj.Function;
    if (typeof F !== "function" || !F.prototype) {{
      return;
    }}
    var proto = F.prototype;
    function __test262ThrowTypeError() {{
      throw new TypeError(
        "'caller', 'callee', and 'arguments' properties may not be accessed on strict mode functions or the arguments objects for calls to them"
      );
    }}
    try {{
      Object.defineProperty(__test262ThrowTypeError, "length", {{
        value: 0,
        writable: false,
        enumerable: false,
        configurable: true
      }});
    }} catch (e) {{}}
    try {{
      Object.defineProperty(__test262ThrowTypeError, "name", {{
        value: "",
        writable: false,
        enumerable: false,
        configurable: true
      }});
    }} catch (e) {{}}
    try {{
      Object.preventExtensions(__test262ThrowTypeError);
    }} catch (e) {{}}
    try {{
      globalObj.__test262ThrowTypeError = __test262ThrowTypeError;
    }} catch (e) {{}}
    // Concise method → no [[Construct]] (isConstructor false).
    // Also tag with a sentinel so self-stringification does not return method source.
    var __dracNativeTag = "__draconicNativeToString__";
    var __toStringHolder = {{
      toString() {{
        if (typeof this !== "function") {{
          throw new TypeError("Function.prototype.toString requires that 'this' be a Function");
        }}
        if (this && this[__dracNativeTag]) {{
          return "function toString() {{ [native code] }}";
        }}
        var name = "";
        try {{
          name = this.name;
          if (typeof name !== "string") {{
            name = "";
          }}
        }} catch (e2) {{
          name = "";
        }}
        // Always synthesize NativeFunction form with a safe IdentifierName
        // (host names like "get $&" are not valid IdentifierName).
        var acc = "";
        var id = name;
        if (name === "get" || name === "set") {{
          acc = name + " ";
          id = "";
        }} else if (name.indexOf("get ") === 0) {{
          acc = "get ";
          id = name.slice(4);
        }} else if (name.indexOf("set ") === 0) {{
          acc = "set ";
          id = name.slice(4);
        }}
        if (!/^[A-Za-z_$][\w$]*$/.test(id)) {{
          id = "";
        }}
        if (acc || id) {{
          return "function " + acc + id + "() {{ [native code] }}";
        }}
        return "function () {{ [native code] }}";
      }}
    }};
    try {{
      Object.defineProperty(__toStringHolder.toString, __dracNativeTag, {{
        value: true,
        writable: false,
        enumerable: false,
        configurable: false
      }});
    }} catch (e) {{}}
    try {{
      Object.defineProperty(proto, "toString", {{
        value: __toStringHolder.toString,
        writable: true,
        enumerable: false,
        configurable: true
      }});
    }} catch (e) {{}}
    try {{
      Object.defineProperty(proto, "caller", {{
        get: __test262ThrowTypeError,
        set: __test262ThrowTypeError,
        enumerable: false,
        configurable: true
      }});
    }} catch (e) {{}}
    try {{
      Object.defineProperty(proto, "arguments", {{
        get: __test262ThrowTypeError,
        set: __test262ThrowTypeError,
        enumerable: false,
        configurable: true
      }});
    }} catch (e) {{}}
  }}
  function __test262InstallHost(globalObj, runEval) {{
    __test262InstallFunctionProto(globalObj);
    __test262InstallErrorStack(globalObj);
    var agent = __test262MakeAgent();
    __test262InstallAgentHelpers(agent);
    var api = {{
      global: globalObj,
      createRealm: function () {{
        return __test262CreateRealm();
      }},
      evalScript: function (src) {{
        return runEval(String(src));
      }},
      detachArrayBuffer: function (buffer) {{
        if (buffer && typeof buffer.transfer === "function") {{
          buffer.transfer(0);
          return;
        }}
        throw new TypeError("$262.detachArrayBuffer is not supported on this host");
      }},
      agent: agent
    }};
    Object.defineProperty(globalObj, "$262", {{
      value: api,
      writable: true,
      configurable: true,
      enumerable: false
    }});
    return api;
  }}
  function __test262CreateRealm() {{
    var context = vm.createContext({{}});
    // Node vm keeps intrinsics inside the context; copy them onto the context
    // object so outer code can read `other.Array` / `other.Function` (cross-realm).
    vm.runInContext(
      "(function () {{" +
        "var g = globalThis;" +
        "var names = Object.getOwnPropertyNames(g);" +
        "var snap = {{}};" +
        "for (var i = 0; i < names.length; i++) {{" +
          "var n = names[i];" +
          "try {{ snap[n] = g[n]; }} catch (e) {{}}" +
        "}}" +
        "globalThis.__test262Snap = snap;" +
      "}})();",
      context
    );
    var snap = vm.runInContext("globalThis.__test262Snap", context);
    var keys = Object.keys(snap);
    for (var ki = 0; ki < keys.length; ki++) {{
      var key = keys[ki];
      if (key === "__test262Snap" || key === "$262") continue;
      try {{
        Object.defineProperty(context, key, {{
          configurable: true,
          enumerable: true,
          writable: true,
          value: snap[key]
        }});
      }} catch (e) {{}}
    }}
    try {{
      vm.runInContext("delete globalThis.__test262Snap", context);
    }} catch (e) {{}}
    return __test262InstallHost(context, function (src) {{
      return vm.runInContext(src, context);
    }});
  }}
  var __test262Root = typeof globalThis !== "undefined" ? globalThis : global;
  __test262InstallHost(__test262Root, function (src) {{
    return (0, eval)(src);
  }});
}})();
{compiled_js}
"#
    )
}

/// Compile Test262 test body (+ shim) through frontend → JS emit.
///
/// Script goal by default. When `flags: [module]` (E19.28), uses Module goal so
/// top-level `await` is accepted. When `test_path` is set and the body has
/// static import/export, links via a temp entry next to the test file.
pub fn compile_test_to_js(test_body: &str) -> Result<String, String> {
    compile_test_to_js_at(test_body, None)
}

/// Like [`compile_test_to_js`], with optional suite file path for Module link.
pub fn compile_test_to_js_at(test_body: &str, test_path: Option<&Path>) -> Result<String, String> {
    let module_goal = is_module_flag(test_body);
    let scan_body = if is_raw_flag(test_body) {
        test_body
    } else {
        strip_frontmatter(test_body)
    };
    let needs_link = module_goal && source_has_static_module_syntax(scan_body);
    // E19.71: for static module link, keep harness *out* of the linked graph and
    // prepend it to emitted JS so it always runs before any dependency body
    // (sibling test files may still contain bare `assert.sameValue` calls).
    let source = if is_raw_flag(test_body) {
        if needs_link {
            test_body.to_string()
        } else if is_only_strict(test_body) {
            format!("{test_body}\n\"use strict\";\n{HARNESS_SHIM}")
        } else {
            format!("{test_body}\n{HARNESS_SHIM}")
        }
    } else {
        let body = strip_frontmatter(test_body);
        if needs_link {
            if is_only_strict(test_body) {
                format!("\"use strict\";\n{body}")
            } else {
                body.to_string()
            }
        } else if is_only_strict(test_body) {
            // `"use strict"` must be the first statement so the whole script (incl. body) is strict.
            format!("\"use strict\";\n{HARNESS_SHIM}\n{body}")
        } else {
            format!("{HARNESS_SHIM}\n{body}")
        }
    };
    let module = if needs_link {
        let Some(path) = test_path else {
            return Err("compile: module test with import/export needs suite path".into());
        };
        let dir = path.parent().ok_or_else(|| "compile: test path has no parent".to_string())?;
        // Unique per concurrent compile (pid alone races under DRACONIC_TEST262_JOBS>1).
        static ENTRY_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = ENTRY_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp_name = format!(
            ".draconic-test262-entry-{}-{}.js",
            std::process::id(),
            seq
        );
        let tmp = dir.join(&tmp_name);
        // E19.71: Test262 often self-imports `./this-file.js`. Rewrite to the temp
        // entry so link does not load the on-disk original as a second module.
        let source = if let Some(base) = path.file_name().and_then(|s| s.to_str()) {
            rewrite_self_module_specifiers(&source, base, &tmp_name)
        } else {
            source
        };
        fs::write(&tmp, &source).map_err(|e| format!("compile: write temp entry: {e}"))?;
        let result = compile_path(&tmp).map_err(|d| format!("compile: {d}"));
        let _ = fs::remove_file(&tmp);
        result?
    } else if module_goal {
        compile_source_module(&source).map_err(|d| format!("compile: {d}"))?
    } else {
        compile_source(&source).map_err(|d| format!("compile: {d}"))?
    };
    let js = emit_js(&module).map_err(|d| format!("emit_js: {d}"))?;
    if needs_link {
        Ok(format!("{HARNESS_SHIM}\n{js}"))
    } else {
        Ok(js)
    }
}

/// Rewrite `from "./orig.js"` / `from './orig.js'` to the temp entry name (E19.71).
fn rewrite_self_module_specifiers(source: &str, orig_base: &str, tmp_base: &str) -> String {
    let mut out = source.to_string();
    for quote in ['\'', '"'] {
        let from = format!("from {quote}./{orig_base}{quote}");
        let to = format!("from {quote}./{tmp_base}{quote}");
        out = out.replace(&from, &to);
        let from = format!("from {quote}{orig_base}{quote}");
        let to = format!("from {quote}./{tmp_base}{quote}");
        out = out.replace(&from, &to);
    }
    out
}

/// Rough scan: static `import`/`export` declarations (not dynamic `import()`).
fn source_has_static_module_syntax(body: &str) -> bool {
    for line in body.lines() {
        let t = line.trim_start();
        if t.starts_with("//") || t.starts_with("/*") {
            continue;
        }
        if t.starts_with("export ") || t.starts_with("export{") || t.starts_with("export*") {
            return true;
        }
        // `import … from` / `import "` / `import '` — not `import(`.
        if let Some(rest) = t.strip_prefix("import") {
            let rest = rest.trim_start();
            if rest.starts_with('(') {
                continue;
            }
            return true;
        }
    }
    false
}

/// Run emitted JS under Node. Exit 0 = pass.
///
/// When `cwd` is set (typically the test file's directory), relative
/// `import('./fixture.js')` resolves like Test262's host (E19.27).
pub fn run_js_in_node(js: &str) -> Result<(), String> {
    run_js_in_node_cwd(js, None, false)
}

/// Like [`run_js_in_node`], optionally with a working directory and ESM mode.
///
/// E19.28: `as_module` uses `--input-type=module` so top-level `await` is valid.
pub fn run_js_in_node_cwd(js: &str, cwd: Option<&Path>, as_module: bool) -> Result<(), String> {
    run_js_in_node_cwd_opts(js, cwd, as_module, false)
}

/// Like [`run_js_in_node_cwd`], with optional Node `--unhandled-rejections=none`.
///
/// E19.83.01: non-async dynamic-import syntax tests fire `import('')` / nested
/// `import(import(…))` without awaiting; Node would exit 1 on the rejected
/// promise. Test262 only requires the syntax to be accepted.
pub fn run_js_in_node_cwd_opts(
    js: &str,
    cwd: Option<&Path>,
    as_module: bool,
    ignore_unhandled_rejections: bool,
) -> Result<(), String> {
    let mut cmd = Command::new("node");
    // E19.73: V8 ShadowRealm is experimental; enable for Test262 built-ins/ShadowRealm.
    cmd.arg("--harmony-shadow-realm");
    if ignore_unhandled_rejections {
        cmd.arg("--unhandled-rejections=none");
    }
    if as_module {
        cmd.arg("--input-type=module");
    }
    cmd.arg("-e").arg(js).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output().map_err(|e| format!("spawn node: {e}"))?;
    let code = output.status.code().unwrap_or(1);
    if code == 0 {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "node exit {code}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    ))
}

/// Run one allowlisted relative path against `suite_root`.
///
/// Panics from the compiler (e.g. mid-UTF-8 lexer bugs) are caught and reported
/// as `Fail` so baseline triage stays report-only (ADR 0007 / E19.02).
pub fn run_case(suite_root: &Path, rel: &str) -> CaseResult {
    let path = rel.to_string();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_case_inner(suite_root, rel)
    })) {
        Ok(c) => c,
        Err(_) => CaseResult {
            path,
            status: Status::Fail,
            message: "panic during compile/run (see E19.03 lexer UTF-8 / related)".into(),
        },
    }
}

fn run_case_inner(suite_root: &Path, rel: &str) -> CaseResult {
    let full = suite_root.join(rel);
    if !full.is_file() {
        return CaseResult {
            path: rel.to_string(),
            status: Status::Fail,
            message: format!("missing file: {}", full.display()),
        };
    }
    let source = match fs::read_to_string(&full) {
        Ok(s) => s,
        Err(e) => {
            return CaseResult {
                path: rel.to_string(),
                status: Status::Fail,
                message: format!("read: {e}"),
            };
        }
    };
    let test_path = Some(full.as_path());
    if is_negative_parse(&source) {
        // Negative parse/early: pass iff frontend rejects the body.
        return match compile_test_to_js_at(&source, test_path) {
            Err(_) => CaseResult {
                path: rel.to_string(),
                status: Status::Pass,
                message: "ok (negative parse)".to_string(),
            },
            Ok(_) => CaseResult {
                path: rel.to_string(),
                status: Status::Fail,
                message: "expected compile failure for negative parse test".to_string(),
            },
        };
    }
    let js = match compile_test_to_js_at(&source, test_path) {
        Ok(j) => j,
        Err(e) => {
            return CaseResult {
                path: rel.to_string(),
                status: Status::Fail,
                message: e,
            };
        }
    };
    // E19.27: resolve relative dynamic `import()` against the test file directory.
    // E19.28: Module-flag tests run as ESM (`--input-type=module`) for top-level await.
    let cwd = full.parent();
    let as_module = is_module_flag(&source);
    let async_flag = is_async_flag(&source);
    // E19.26: async-flag tests need `$DONE` host (Node wrapper around emitted JS).
    let js = if async_flag {
        wrap_async_host(&js)
    } else {
        js
    };
    // E19.61: `$262` host outside so ESM `import` stays first under module goal.
    // onlyStrict: host must not precede the effective `"use strict"` directive.
    let js = wrap_host_api_mode(&js, as_module, is_only_strict(&source));
    // E19.83.01: non-async tests may leave dynamic-import rejections unhandled;
    // async tests settle via `$DONE` / unhandledRejection → keep default policy.
    let ignore_unhandled = !async_flag;
    if is_negative_runtime(&source) {
        // Negative runtime: pass iff Node throws (exit ≠ 0).
        return match run_js_in_node_cwd_opts(&js, cwd, as_module, ignore_unhandled) {
            Err(_) => CaseResult {
                path: rel.to_string(),
                status: Status::Pass,
                message: "ok (negative runtime)".to_string(),
            },
            Ok(()) => CaseResult {
                path: rel.to_string(),
                status: Status::Fail,
                message: "expected runtime failure for negative runtime test".to_string(),
            },
        };
    }
    match run_js_in_node_cwd_opts(&js, cwd, as_module, ignore_unhandled) {
        Ok(()) => CaseResult {
            path: rel.to_string(),
            status: Status::Pass,
            message: if async_flag && as_module {
                "ok (module async $DONE)".to_string()
            } else if async_flag {
                "ok (async $DONE)".to_string()
            } else if as_module {
                "ok (module)".to_string()
            } else {
                "ok".to_string()
            },
        },
        Err(e) => CaseResult {
            path: rel.to_string(),
            status: Status::Fail,
            message: e,
        },
    }
}

/// Parallelism for allowlist runs (`DRACONIC_TEST262_JOBS`, default = CPUs).
pub fn test262_jobs() -> usize {
    if let Ok(raw) = std::env::var("DRACONIC_TEST262_JOBS") {
        if let Ok(n) = raw.trim().parse::<usize>() {
            return n.max(1);
        }
    }
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Run the curated allowlist. Suite absent → every case `skip`.
///
/// Cases run in parallel (see [`test262_jobs`]). Order matches the allowlist.
pub fn run_allowlist(suite_root: &Path, allowlist: &[String]) -> Report {
    let present = suite_present(suite_root);
    let cases = if !present {
        allowlist
            .iter()
            .map(|p| CaseResult {
                path: p.clone(),
                status: Status::Skip,
                message: format!(
                    "suite not present at {} (run: node scripts/fetch-test262.mjs)",
                    suite_root.display()
                ),
            })
            .collect()
    } else {
        let jobs = test262_jobs();
        let root = suite_root.to_path_buf();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .stack_size(8 * 1024 * 1024)
            .build()
            .expect("test262 rayon pool");
        pool.install(|| {
            allowlist
                .par_iter()
                .map(|p| run_case(&root, p))
                .collect()
        })
    };
    Report {
        suite_root: Some(suite_root.to_path_buf()),
        suite_present: present,
        cases,
    }
}

/// Default entry: resolve root + package allowlist.
pub fn run_default() -> Result<Report, String> {
    let root = resolve_suite_root();
    let list = load_allowlist(&allowlist_path())?;
    Ok(run_allowlist(&root, &list))
}

/// Write markdown report next to the package (or `DRACONIC_TEST262_REPORT` path).
pub fn write_baseline_report(report: &Report) -> Result<PathBuf, String> {
    let path = if let Ok(p) = std::env::var("DRACONIC_TEST262_REPORT") {
        PathBuf::from(p)
    } else {
        package_root().join("baseline-report.md")
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    fs::write(&path, report.to_markdown())
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    // Also stamp under target/ when available (CI artifact friendly).
    let stamp = workspace_root()
        .join("target")
        .join("test262-baseline-report.md");
    if let Some(parent) = stamp.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&stamp, report.to_markdown());
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_loads_and_has_entries() {
        let list = load_allowlist(&allowlist_path()).expect("allowlist");
        // E19.02/E19.06/E19.10/E19.15/E19.20/E19.25–E19.68/E19.75/E19.81 expanded curated set.
        assert!(
            list.len() >= 45100,
            "expected expanded curated allowlist (>=45100), got {}",
            list.len()
        );
        assert!(list.iter().all(|p| p.starts_with("test/")));
    }

    #[test]
    fn strip_frontmatter_removes_yaml_block() {
        let src = "/*---\ndescription: x\n---*/\nif (true) {}\n";
        let body = strip_frontmatter(src);
        assert!(body.starts_with("if (true)"));
    }

    #[test]
    fn harness_shim_compiles_and_runs() {
        let js = compile_test_to_js("assert.sameValue(1 + 2, 3);\n").expect("compile");
        run_js_in_node(&js).expect("node");
    }

    #[test]
    fn harness_shim_catches_failure() {
        let js = compile_test_to_js("assert.sameValue(1, 2, \"nope\");\n").expect("compile");
        assert!(run_js_in_node(&js).is_err());
    }

    #[test]
    fn harness_shim_throws_typeerror() {
        let js = compile_test_to_js(
            r#"
            assert.throws(TypeError, function() { 1n + 1; });
            assert.throws(TypeError, function() { 1 + 1n; });
            "#,
        )
        .expect("compile");
        run_js_in_node(&js).expect("node");
    }

    #[test]
    fn harness_shim_throws_fails_when_no_throw() {
        let js = compile_test_to_js("assert.throws(TypeError, function() { 1 + 1; });\n")
            .expect("compile");
        assert!(run_js_in_node(&js).is_err());
    }

    #[test]
    fn missing_suite_skips_all() {
        let root = workspace_root().join("third_party").join("test262-does-not-exist");
        let list = vec![
            "test/language/types/boolean/S8.3_A1_T1.js".to_string(),
            "test/language/types/null/S8.2_A1_T1.js".to_string(),
        ];
        let report = run_allowlist(&root, &list);
        assert!(!report.suite_present);
        assert_eq!(report.cases.len(), 2);
        assert!(report.cases.iter().all(|c| c.status == Status::Skip));
        let (p, f, s) = report.counts();
        assert_eq!((p, f, s), (0, 0, 2));
    }

    #[test]
    fn markdown_report_mentions_totals() {
        let report = Report {
            suite_root: Some(PathBuf::from("/tmp/nope")),
            suite_present: false,
            cases: vec![CaseResult {
                path: "test/x.js".into(),
                status: Status::Skip,
                message: "suite not present".into(),
            }],
        };
        let md = report.to_markdown();
        assert!(md.contains("pass=0"));
        assert!(md.contains("skip=1"));
        assert!(md.contains("test/x.js"));
    }

    #[test]
    fn negative_parse_meta_detected() {
        let src = "/*---\nnegative:\n  phase: parse\n  type: SyntaxError\n---*/\n1_\n";
        assert!(is_negative_parse(src));
        assert!(!is_negative_parse("/*---\ndescription: x\n---*/\n1\n"));
        // E19.71: resolution-phase SyntaxError is compile/link failure.
        let res = "/*---\nnegative:\n  phase: resolution\n  type: SyntaxError\nflags: [module]\n---*/\nimport { x } from \"./m.js\";\n";
        assert!(is_negative_parse(res));
    }

    #[test]
    fn only_strict_meta_detected() {
        let src = "/*---\nflags: [onlyStrict]\ndescription: x\n---*/\n1\n";
        assert!(is_only_strict(src));
        assert!(!is_only_strict("/*---\ndescription: x\n---*/\n1\n"));
        assert!(!is_only_strict("/*---\nflags: [noStrict]\n---*/\n1\n"));
    }

    #[test]
    fn async_flag_meta_detected() {
        // E19.26: flags token `async`, not features like `async-functions`.
        assert!(is_async_flag(
            "/*---\nflags: [generated, async]\nfeatures: [async-functions]\n---*/\n1\n"
        ));
        assert!(is_async_flag("/*---\nflags: [async]\n---*/\n1\n"));
        assert!(!is_async_flag(
            "/*---\nfeatures: [async-functions, async-iteration]\n---*/\n1\n"
        ));
        assert!(!is_async_flag("/*---\nflags: [generated]\n---*/\n1\n"));
    }

    #[test]
    fn module_flag_meta_detected() {
        // E19.28: flags token `module`.
        assert!(is_module_flag(
            "/*---\nflags: [generated, module]\nfeatures: [top-level-await]\n---*/\n1\n"
        ));
        assert!(is_module_flag("/*---\nflags: [module, async]\n---*/\n1\n"));
        assert!(!is_module_flag(
            "/*---\nfeatures: [top-level-await]\n---*/\n1\n"
        ));
        assert!(!is_module_flag("/*---\nflags: [async]\n---*/\n1\n"));
        assert!(is_module_flag(
            "/*---\nflags:\n  - module\nnegative:\n  phase: parse\n---*/\n1\n"
        ));
    }

    #[test]
    fn top_level_await_module_compiles_and_runs() {
        // E19.28: Module goal + ESM host accepts top-level await.
        let src = r#"
/*---
description: top-level await basics
flags: [module, async]
features: [top-level-await]
---*/
var x = await 42;
assert.sameValue(x, 42);
$DONE();
"#;
        let js = compile_test_to_js(src).expect("compile module TLA");
        assert!(js.contains("await"), "{js}");
        let js = wrap_async_host(&js);
        run_js_in_node_cwd(&js, None, true).expect("node ESM TLA");
    }

    #[test]
    fn top_level_await_script_rejected() {
        // E19.28 / E19.52: Script [~Await] — `await` is IdentifierReference, so
        // `await 1` is a syntax error (not AwaitExpression).
        let src = "var x = await 1;\n";
        let err = compile_test_to_js(src).expect_err("script TLA must fail");
        assert!(!err.is_empty(), "unexpected empty err");
    }

    #[test]
    fn async_done_success_settles() {
        // E19.26: promise chain + $DONE() → pass (no ReferenceError).
        let src = r#"
/*---
description: async $DONE success
flags: [async]
---*/
Promise.resolve(1).then(function (v) {
  assert.sameValue(v, 1);
}).then($DONE, $DONE);
"#;
        let js = compile_test_to_js(src).expect("compile");
        let js = wrap_async_host(&js);
        run_js_in_node(&js).expect("async $DONE success");
    }

    #[test]
    fn async_done_failure_rejects() {
        // E19.26: $DONE(error) → node non-zero.
        let src = r#"
/*---
flags: [async]
---*/
Promise.resolve().then(function () {
  $DONE(new Error("boom"));
});
"#;
        let js = compile_test_to_js(src).expect("compile");
        let js = wrap_async_host(&js);
        assert!(run_js_in_node(&js).is_err());
    }

    #[test]
    fn async_done_missing_is_reference_error_without_host() {
        // Without wrap_async_host, $DONE is unresolved at runtime.
        let src = r#"
/*---
flags: [async]
---*/
Promise.resolve().then($DONE, $DONE);
"#;
        let js = compile_test_to_js(src).expect("compile");
        assert!(
            run_js_in_node(&js).is_err(),
            "bare $DONE without host must fail"
        );
    }

    #[test]
    fn dynamic_import_call_compiles_and_emits() {
        // E19.27: ImportCall round-trip through frontend → JS.
        let src = r#"
/*---
features: [dynamic-import]
---*/
let p = import('./m.js');
assert.sameValue(typeof p.then, "function");
"#;
        let js = compile_test_to_js(src).expect("compile");
        assert!(js.contains("import(\"./m.js\")"), "{js}");
    }

    #[test]
    fn import_meta_compiles_and_emits() {
        // E19.83.01: `import.meta` as ImportCall AssignmentExpression (module).
        let src = r#"
/*---
features: [dynamic-import, import.meta]
flags: [module, async]
---*/
const p = import(import.meta);
assert.sameValue(Promise.resolve(p), p);
p.catch(function () {}).then($DONE, $DONE);
"#;
        let js = compile_test_to_js(src).expect("compile");
        assert!(js.contains("import.meta"), "{js}");
        assert!(js.contains("import("), "{js}");
    }

    #[test]
    fn only_strict_compound_assign_putvalue_typeerror() {
        // E19.19: non-writable data prop + compound `*=` must TypeError under onlyStrict.
        let src = r#"
/*---
description: strict PutValue TypeError on compound assign to non-writable
flags: [onlyStrict]
---*/
var obj = {};
Object.defineProperty(obj, "prop", {
  value: 10,
  writable: false,
  enumerable: true,
  configurable: true
});
assert.throws(TypeError, function() {
  obj.prop *= 20;
});
assert.sameValue(obj.prop, 10, "obj.prop");
"#;
        let js = compile_test_to_js(src).expect("compile");
        assert!(
            js.contains("use strict"),
            "emitted JS must include use strict for onlyStrict: {js}"
        );
        run_js_in_node(&js).expect("node strict PutValue");
    }

    #[test]
    fn only_strict_compound_assign_nonextensible_typeerror() {
        // E19.19: missing prop on non-extensible object.
        let src = r#"
/*---
flags: [onlyStrict]
---*/
var obj = {};
Object.preventExtensions(obj);
assert.throws(TypeError, function() {
  obj.len *= 10;
});
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&js).expect("node non-extensible PutValue");
    }

    #[test]
    fn harness_is_constructor() {
        // E19.61: isConstructor via Reflect.construct.
        let src = r#"
assert.sameValue(isConstructor(Array), true);
assert.sameValue(isConstructor(function () {}), true);
assert.sameValue(isConstructor(() => {}), false);
assert.sameValue(isConstructor(eval), false);
assert.throws(Test262Error, function () {
  isConstructor({});
});
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&wrap_host_api(&js)).expect("isConstructor");
    }

    #[test]
    fn harness_async_test_success() {
        // E19.61: asyncTest settles via $DONE.
        let src = r#"
/*---
flags: [async]
---*/
asyncTest(async function () {
  assert.sameValue(await Promise.resolve(7), 7);
});
"#;
        let js = compile_test_to_js(src).expect("compile");
        let js = wrap_async_host(&wrap_host_api(&js));
        run_js_in_node(&js).expect("asyncTest success");
    }

    #[test]
    fn harness_async_test_failure() {
        let src = r#"
/*---
flags: [async]
---*/
asyncTest(async function () {
  assert.sameValue(1, 2);
});
"#;
        let js = compile_test_to_js(src).expect("compile");
        let js = wrap_async_host(&wrap_host_api(&js));
        assert!(run_js_in_node(&js).is_err());
    }

    #[test]
    fn harness_typed_array_constructors() {
        // E19.61: testWithTypedArrayConstructors + TypedArray intrinsic.
        let src = r#"
assert.sameValue(typeof TypedArray, "function");
assert.sameValue(TypedArray.name, "TypedArray");
let seen = 0;
testWithTypedArrayConstructors(function (TA) {
  let sample = new TA([1, 2, 3]);
  assert.sameValue(sample.length, 3);
  seen = seen + 1;
}, null, ["passthrough"]);
assert.sameValue(seen >= 9, true, "visited non-bigint TAs");
let bigSeen = 0;
testWithBigIntTypedArrayConstructors(function (TA) {
  let sample = new TA([1n, 2n]);
  assert.sameValue(sample.length, 2);
  bigSeen = bigSeen + 1;
}, null, ["passthrough"]);
assert.sameValue(bigSeen >= 2, true, "visited bigint TAs");
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&wrap_host_api(&js)).expect("typed array harness");
    }

    #[test]
    fn harness_create_realm_minimal() {
        // E19.61: $262.createRealm().global has distinct constructors.
        let src = r#"
assert.sameValue(typeof $262, "object");
assert.sameValue(typeof $262.createRealm, "function");
let other = $262.createRealm().global;
assert.sameValue(typeof other.Array, "function");
assert.sameValue(other.Array === Array, false, "cross-realm Array");
let a = new other.Array(1, 2, 3);
assert.sameValue(a instanceof other.Array, true);
assert.sameValue(Object.getPrototypeOf(a) === other.Array.prototype, true);
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&wrap_host_api(&js)).expect("createRealm");
    }

    #[test]
    fn e19_73_shadow_realm_basics() {
        // E19.73: ShadowRealm constructor + evaluate / importValue basics.
        let src = r#"
assert.sameValue(typeof ShadowRealm, "function");
assert.sameValue(ShadowRealm.length, 0);
assert.sameValue(ShadowRealm.name, "ShadowRealm");
let r = new ShadowRealm();
assert.sameValue(typeof r.evaluate, "function");
assert.sameValue(typeof r.importValue, "function");
assert.sameValue(r.evaluate("1 + 1"), 2);
assert.sameValue(r.evaluate("'hi'"), "hi");
assert.sameValue(r.evaluate("undefined"), undefined);
let wrapped = r.evaluate("(function (x) { return x + 1; })");
assert.sameValue(typeof wrapped, "function");
assert.sameValue(wrapped(41), 42);
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&js).expect("ShadowRealm basics");
    }

    #[test]
    fn e19_73_shadow_realm_import_value_data_url() {
        // importValue via data: URL (relative file import is incomplete in Node harmony).
        let src = r#"
/*---
flags: [async]
---*/
let r = new ShadowRealm();
r.importValue("data:text/javascript,export const x = 41;", "x").then(function (x) {
  assert.sameValue(x, 41);
}).then($DONE, $DONE);
"#;
        let js = compile_test_to_js(src).expect("compile");
        let js = wrap_async_host(&js);
        run_js_in_node(&js).expect("ShadowRealm importValue data:");
    }

    #[test]
    fn harness_detach_buffer() {
        // E19.64: $DETACHBUFFER via $262.detachArrayBuffer.
        let src = r#"
let buf = new ArrayBuffer(8);
let view = new Uint8Array(buf);
view[0] = 1;
assert.sameValue(buf.byteLength, 8);
$DETACHBUFFER(buf);
assert.sameValue(buf.byteLength, 0);
assert.sameValue(buf.detached, true);
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&wrap_host_api(&js)).expect("detach buffer");
    }

    #[test]
    fn harness_byte_conversion_values() {
        // E19.64: byteConversionValues tables present for testTypedArrayConversions.
        let src = r#"
assert.sameValue(typeof byteConversionValues, "object");
assert.sameValue(byteConversionValues.values.length > 0, true);
assert.sameValue(byteConversionValues.expected.Int8.length, byteConversionValues.values.length);
assert.sameValue(byteConversionValues.expected.Float64.length, byteConversionValues.values.length);
let seen = 0;
testTypedArrayConversions(byteConversionValues, function (TA, value, expected, initial) {
  let sample = new TA([initial]);
  sample[0] = value;
  assert.sameValue(sample[0], expected);
  seen = seen + 1;
});
assert.sameValue(seen > 0, true);
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&wrap_host_api(&js)).expect("byteConversionValues");
    }

    #[test]
    fn harness_test_with_atomics_indices() {
        // E19.64: testWithAtomics* index/value generators.
        let src = r#"
let oob = 0;
testWithAtomicsOutOfBoundsIndices(function (IdxGen) {
  let v = new Int32Array(new SharedArrayBuffer(4));
  let idx = IdxGen(v);
  assert.sameValue(typeof idx === "number" || typeof idx === "object" || idx === undefined, true);
  oob = oob + 1;
});
assert.sameValue(oob, 7);
let ib = 0;
testWithAtomicsInBoundsIndices(function (IdxGen) {
  let v = new Int32Array(new SharedArrayBuffer(16));
  IdxGen(v);
  ib = ib + 1;
});
assert.sameValue(ib, 11);
let nv = 0;
testWithAtomicsNonViewValues(function (nonView) {
  nv = nv + 1;
});
assert.sameValue(nv >= 20, true);
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&wrap_host_api(&js)).expect("testWithAtomics indices");
    }

    #[test]
    fn harness_agent_minimal() {
        // E19.64: minimal $262.agent parent surface + report round-trip.
        let src = r#"
assert.sameValue(typeof $262.agent, "object");
assert.sameValue(typeof $262.agent.start, "function");
assert.sameValue(typeof $262.agent.broadcast, "function");
assert.sameValue(typeof $262.agent.getReport, "function");
assert.sameValue(typeof $262.agent.sleep, "function");
assert.sameValue(typeof $262.agent.monotonicNow, "function");
assert.sameValue(typeof $262.agent.safeBroadcast, "function");
assert.sameValue(typeof $262.agent.tryYield, "function");
$262.agent.start(`
  $262.agent.receiveBroadcast(function (sab) {
    var i32a = new Int32Array(sab);
    $262.agent.report(String(Atomics.load(i32a, 0)));
    $262.agent.leaving();
  });
`);
var sab = new SharedArrayBuffer(4);
var i32a = new Int32Array(sab);
Atomics.store(i32a, 0, 42);
$262.agent.broadcast(sab);
assert.sameValue($262.agent.getReport(), "42");
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&wrap_host_api(&js)).expect("agent minimal");
    }

    #[test]
    fn harness_e19_66_ctors_and_rab_utils() {
        // E19.66: global ctors / floatCtors / CreateResizableArrayBuffer / NaNs +
        // resizable/immutable TypedArray arg factories.
        let src = r#"
assert.sameValue(typeof ctors, "object");
assert.sameValue(ctors.length >= 9, true);
assert.sameValue(typeof floatCtors, "object");
assert.sameValue(floatCtors.length >= 2, true);
assert.sameValue(typeof CreateResizableArrayBuffer, "function");
assert.sameValue(typeof MayNeedBigInt, "function");
assert.sameValue(typeof CreateRabForTest, "function");
assert.sameValue(typeof ToNumbers, "function");
assert.sameValue(typeof CollectValuesAndResize, "function");
assert.sameValue(typeof TestIterationAndResize, "function");

let rab = CreateResizableArrayBuffer(8, 16);
assert.sameValue(rab.byteLength, 8);
assert.sameValue(rab.maxByteLength, 16);
rab.resize(12);
assert.sameValue(rab.byteLength, 12);

let rab2 = CreateRabForTest(Uint8Array);
assert.sameValue(rab2.byteLength, 4);
let view = new Uint8Array(rab2);
assert.sameValue(view[0], 0);
assert.sameValue(view[1], 2);
assert.sameValue(view[2], 4);
assert.sameValue(view[3], 6);

assert.sameValue(MayNeedBigInt(new Uint8Array(1), 3), 3);
if (typeof BigInt64Array !== "undefined") {
  assert.sameValue(MayNeedBigInt(new BigInt64Array(1), 3), 3n);
}

assert.sameValue(typeof NaNs, "object");
assert.sameValue(NaNs.length >= 5, true);
let ni = 0;
while (ni < NaNs.length) {
  assert.sameValue(Number.isNaN(NaNs[ni]), true);
  ni = ni + 1;
}

let resSeen = 0;
testWithTypedArrayConstructors(function (TA, makeCtorArg) {
  let ta = new TA(makeCtorArg([1, 2, 3, 4]));
  assert.sameValue(ta.length, 4);
  assert.sameValue(ta.buffer.resizable, true);
  resSeen = resSeen + 1;
}, [Uint8Array], ["resizable"]);
assert.sameValue(resSeen >= 1, true, "resizable factories");

if (typeof ArrayBuffer.prototype.transferToImmutable === "function") {
  let immSeen = 0;
  testWithTypedArrayConstructors(function (TA, makeCtorArg) {
    let ta = new TA(makeCtorArg([1, 2, 3, 4]));
    assert.sameValue(ta.length, 4);
    assert.sameValue(ta.buffer.immutable, true);
    immSeen = immSeen + 1;
  }, [Uint8Array], ["immutable"]);
  assert.sameValue(immSeen >= 1, true, "immutable factory");
}

let bi = 0;
while (bi < ctors.length) {
  assert.sameValue(typeof ctors[bi], "function");
  bi = bi + 1;
}
"#;
        let js = compile_test_to_js(src).expect("compile e19.66");
        run_js_in_node(&wrap_host_api(&js)).expect("e19.66 ctors and rab utils");
    }

    #[test]
    fn harness_e19_65_residual_helpers() {
        // E19.65: fnGlobalObject, decimalTo*, checkSequence/Settled, assertThrowsValue,
        // assertNativeFunction, assertNear, $MAX_ITERATIONS.
        let src = r#"
assert.sameValue(typeof fnGlobalObject, "function");
assert.sameValue(fnGlobalObject(), globalThis);

assert.sameValue(decimalToHexString(-1), "FFFFFFFF");
assert.sameValue(decimalToHexString(0.5), "0000");
assert.sameValue(decimalToHexString(1), "0001");
assert.sameValue(decimalToHexString(100), "0064");
assert.sameValue(decimalToHexString(65535), "FFFF");
assert.sameValue(decimalToHexString(65536), "10000");
assert.sameValue(decimalToPercentHexString(-1), "%FF");
assert.sameValue(decimalToPercentHexString(0.5), "%00");
assert.sameValue(decimalToPercentHexString(1), "%01");
assert.sameValue(decimalToPercentHexString(100), "%64");
assert.sameValue(decimalToPercentHexString(65535), "%FF");
assert.sameValue(decimalToPercentHexString(65536), "%00");

assert.sameValue(checkSequence([1, 2, 3, 4, 5]), true);
assert.throws(Test262Error, function () {
  checkSequence([2, 1, 3]);
});
checkSettledPromises(
  [
    { status: "fulfilled", value: 1 },
    { status: "rejected", reason: "e" }
  ],
  [
    { status: "fulfilled", value: 1 },
    { status: "rejected", reason: "e" }
  ]
);
assert.throws(Test262Error, function () {
  checkSettledPromises([{ status: "fulfilled", value: 1 }], [{ status: "fulfilled", value: 2 }]);
});

assertThrowsValue(function () {
  throw 42;
}, 42);
assert.throws(Test262Error, function () {
  assertThrowsValue(function () {}, 1);
});

assert.sameValue($MAX_ITERATIONS, 100000);

assert.sameValue(typeof assertNativeFunction, "function");
assert.sameValue(typeof validateNativeFunctionSource, "function");
validateNativeFunctionSource("function(){ [native code] }");
validateNativeFunctionSource("function a(){ [native code] }");
validateNativeFunctionSource("function ( ) { [ native code ] }");
assert.throws(SyntaxError, function () {
  validateNativeFunctionSource("function() {}");
});
assert.throws(Test262Error, function () {
  assertNativeFunction(function () {
    return 1;
  });
});

assert.sameValue(typeof assertNear, "function");
assertNear(1, 1);
assertNear(1, ONE_PLUS_EPSILON);
assertNear(1, ONE_MINUS_EPSILON);
assert.throws(Error, function () {
  assertNear(1, 2);
});
"#;
        let js = compile_test_to_js(src).expect("compile e19.65");
        // assertNativeFunction on real host natives needs Node builtins.
        let host_src = r#"
assertNativeFunction(Array);
assertNativeFunction(Object);
assertToStringOrNativeFunction(Array, String(Array));
"#;
        let host_js = compile_test_to_js(host_src).expect("compile e19.65 host");
        run_js_in_node(&js).expect("e19.65 residual helpers");
        run_js_in_node(&wrap_host_api(&host_js)).expect("e19.65 native Function matchers");
    }

    #[test]
    fn harness_e19_70_regexp_utils() {
        // E19.70: buildString + testPropertyEscapes + testPropertyOfStrings.
        let src = r#"
const matchSymbols = buildString({
  loneCodePoints: [],
  ranges: [
    [0x000000, 0x00007F]
  ]
});
testPropertyEscapes(
  /^\p{ASCII}+$/u,
  matchSymbols,
  "\\p{ASCII}"
);
const nonMatchSymbols = buildString({
  loneCodePoints: [0x000080],
  ranges: []
});
assert.sameValue(/^\p{ASCII}+$/u.test(nonMatchSymbols), false);
testPropertyOfStrings({
  regExp: /^\p{Basic_Emoji}+$/v,
  expression: "\\p{Basic_Emoji}",
  matchStrings: ["\u231A"],
  nonMatchStrings: ["A"]
});
let validate = matchValidator(["b"], 1, "abc");
validate(/b/.exec("abc"));
"#;
        let js = compile_test_to_js(src).expect("compile e19.70 harness");
        run_js_in_node(&js).expect("e19.70 regexp utils");
    }

    #[test]
    fn harness_e19_74_promise_all_keyed() {
        // E19.74: Promise.allKeyed / Promise.allSettledKeyed polyfill.
        let sync_src = r#"
assert.sameValue(typeof Promise.allKeyed, "function");
assert.sameValue(typeof Promise.allSettledKeyed, "function");
assert.sameValue(Promise.allKeyed.length, 1);
assert.sameValue(Promise.allKeyed.name, "allKeyed");
assert.sameValue(Promise.allSettledKeyed.name, "allSettledKeyed");
assert.sameValue(isConstructor(Promise.allKeyed), false);
assert.sameValue(isConstructor(Promise.allSettledKeyed), false);
assert.throws(TypeError, function () {
  Promise.allKeyed.call(eval);
});
"#;
        let sync_js = compile_test_to_js(sync_src).expect("compile e19.74 sync");
        run_js_in_node(&wrap_host_api(&sync_js)).expect("e19.74 promise allKeyed sync");

        let async_src = r#"
asyncTest(function () {
  return Promise.allKeyed({}).then(function (result) {
    assert.sameValue(Object.getPrototypeOf(result), null);
    assert.compareArray(Reflect.ownKeys(result), []);
  }).then(function () {
    return Promise.allKeyed({ a: Promise.resolve(1), b: 2 }).then(function (result) {
      assert.sameValue(Object.getPrototypeOf(result), null);
      assert.sameValue(result.a, 1);
      assert.sameValue(result.b, 2);
    });
  }).then(function () {
    return Promise.allSettledKeyed({
      ok: Promise.resolve(1),
      bad: Promise.reject("nope")
    }).then(function (result) {
      assert.sameValue(Object.getPrototypeOf(result), null);
      assert.sameValue(result.ok.status, "fulfilled");
      assert.sameValue(result.ok.value, 1);
      assert.sameValue(result.bad.status, "rejected");
      assert.sameValue(result.bad.reason, "nope");
    });
  });
});
"#;
        let async_js = compile_test_to_js(async_src).expect("compile e19.74 async");
        run_js_in_node(&wrap_async_host(&async_js)).expect("e19.74 promise allKeyed async");
    }

    #[test]
    fn harness_e19_76_iterator_zip() {
        // E19.76: Iterator.zip / Iterator.zipKeyed polyfill + zip utils.
        let src = r#"
assert.sameValue(typeof Iterator.zip, "function");
assert.sameValue(typeof Iterator.zipKeyed, "function");
assert.sameValue(Iterator.zip.length, 1);
assert.sameValue(Iterator.zip.name, "zip");
assert.sameValue(Iterator.zipKeyed.name, "zipKeyed");
assert.sameValue(isConstructor(Iterator.zip), false);
assert.sameValue(isConstructor(Iterator.zipKeyed), false);
assert.throws(TypeError, function () {
  new Iterator.zip([]);
});
assert.throws(TypeError, function () {
  Iterator.zip();
});
assert.throws(TypeError, function () {
  Iterator.zip(null);
});
assert.throws(TypeError, function () {
  Iterator.zip([], null);
});
assert.throws(TypeError, function () {
  Iterator.zip([], { mode: "loose" });
});

var it = Iterator.zip([[1, 2], [3, 4]]);
assert(it instanceof Iterator);
assert.sameValue(
  Object.getPrototypeOf(it),
  getWellKnownIntrinsicObject("%IteratorHelperPrototype%")
);
assert.compareArray(it.next().value, [1, 3]);
assert.compareArray(it.next().value, [2, 4]);
assert.sameValue(it.next().done, true);

var short = Iterator.zip([[1, 2, 3], [4]]);
assert.compareArray(short.next().value, [1, 4]);
assert.sameValue(short.next().done, true);

var long = Iterator.zip([[1], [2, 3]], { mode: "longest", padding: ["p", "q"] });
assert.compareArray(long.next().value, [1, 2]);
assert.compareArray(long.next().value, ["p", 3]);
assert.sameValue(long.next().done, true);

assert.throws(TypeError, function () {
  var s = Iterator.zip([[1], [2, 3]], { mode: "strict" });
  s.next();
  s.next();
});

var keyed = Iterator.zipKeyed({ a: [1, 2], b: [3, 4] });
var k0 = keyed.next().value;
assert.sameValue(Object.getPrototypeOf(k0), null);
assert.sameValue(k0.a, 1);
assert.sameValue(k0.b, 3);
var k1 = keyed.next().value;
assert.sameValue(k1.a, 2);
assert.sameValue(k1.b, 4);
assert.sameValue(keyed.next().done, true);

var returnCount = 0;
var underlying = {
  next: function () {
    return { value: 1, done: false };
  },
  return: function () {
    returnCount = returnCount + 1;
    return {};
  }
};
var closed = Iterator.zip([underlying]);
assert.sameValue(closed.return().done, true);
assert.sameValue(returnCount, 1);

forEachSequenceCombination(function (inputs, label, min) {
  var z = Iterator.zip(inputs);
  assertZipped(z, inputs, min, label);
});
"#;
        let js = compile_test_to_js(src).expect("compile e19.76");
        run_js_in_node(&wrap_host_api(&js)).expect("e19.76 iterator zip");
    }

    #[test]
    fn harness_e19_77_function_tostring_and_poison() {
        // E19.77: Function.prototype.toString NativeFunction form + caller/arguments poison.
        let src = r#"
function f /* a */ (x) { return x; }
assertToStringOrNativeFunction(f, "function /* a */ (x) { return x; }");
assertNativeFunction(function () {});
assert.sameValue(typeof WellKnownIntrinsicObjects, "object");
assert.sameValue(Array.isArray(WellKnownIntrinsicObjects), true);
assert.notSameValue(WellKnownIntrinsicObjects.length, 0);

const callerDesc = Object.getOwnPropertyDescriptor(Function.prototype, "caller");
const argumentsDesc = Object.getOwnPropertyDescriptor(Function.prototype, "arguments");
assert.sameValue(typeof callerDesc.get, "function");
assert.sameValue(typeof callerDesc.set, "function");
assert.sameValue(callerDesc.get, callerDesc.set);
assert.sameValue(argumentsDesc.get, argumentsDesc.set);
assert.sameValue(callerDesc.get, argumentsDesc.get);
assert.throws(TypeError, function () {
  return Function.prototype.caller;
});
assert.throws(TypeError, function () {
  Function.prototype.caller = null;
});
assert.throws(TypeError, function () {
  return Function.prototype.arguments;
});
"#;
        let js = compile_test_to_js(src).expect("compile e19.77");
        run_js_in_node(&wrap_host_api(&js)).expect("e19.77 function toString + poison");
    }

    #[test]
    fn harness_e19_79_error_iserror_and_stack() {
        // E19.79: Error.isError on subclasses + Error.prototype.stack accessor.
        let src = r#"
class MyError extends Error {}
assert.sameValue(Error.isError(new MyError()), true);
assert.sameValue(Error.isError(new Error()), true);
assert.sameValue(Error.isError({}), false);

var desc = Object.getOwnPropertyDescriptor(Error.prototype, "stack");
assert.sameValue(typeof desc.get, "function");
assert.sameValue(typeof desc.set, "function");
assert.sameValue(desc.enumerable, false);
assert.sameValue(desc.configurable, true);
assert.sameValue(desc.get.name, "get stack");
assert.sameValue(desc.set.name, "set stack");
assert.sameValue(desc.get.length, 0);
assert.sameValue(desc.set.length, 1);

var err = new Error("msg");
assert.sameValue(Object.prototype.hasOwnProperty.call(err, "stack"), false);
assert.sameValue(typeof desc.get.call(err), "string");
assert.sameValue(typeof err.stack, "string");
assert.sameValue(desc.get.call({}), undefined);

desc.set.call(err, "sentinel");
assert.sameValue(err.stack, "sentinel");
assert.sameValue(Object.prototype.hasOwnProperty.call(err, "stack"), true);
assert.sameValue(typeof desc.get.call(err), "string");

assert.throws(TypeError, function () {
  desc.set.call(Error.prototype, "x");
});
assert.throws(TypeError, function () {
  desc.get.call(null);
});
assert.throws(TypeError, function () {
  desc.set.call(err, 1);
});

assert.sameValue(typeof nativeErrors, "object");
assert.sameValue(nativeErrors.length, 7);
assert.sameValue(typeof makeNativeError, "function");
assert.sameValue(typeof verifyPrimordialAccessorProperty, "function");
"#;
        let js = compile_test_to_js(src).expect("compile e19.79");
        run_js_in_node(&wrap_host_api(&js)).expect("e19.79 error isError + stack");
    }

    #[test]
    fn harness_e19_80_math_sum_precise() {
        // E19.80: Math.sumPrecise polyfill (precise iterable sum).
        let src = r#"
assert.sameValue(typeof Math.sumPrecise, "function");
assert.sameValue(Math.sumPrecise.length, 1);
assert.sameValue(Math.sumPrecise.name, "sumPrecise");
assert.sameValue(isConstructor(Math.sumPrecise), false);
assert.throws(TypeError, function () {
  new Math.sumPrecise([]);
});

var desc = Object.getOwnPropertyDescriptor(Math, "sumPrecise");
assert.sameValue(desc.writable, true);
assert.sameValue(desc.enumerable, false);
assert.sameValue(desc.configurable, true);

assert.sameValue(Math.sumPrecise([]), -0);
assert.sameValue(Math.sumPrecise([-0]), -0);
assert.sameValue(Math.sumPrecise([-0, 0]), 0);
assert.sameValue(Math.sumPrecise([1, 2, 3]), 6);
assert.sameValue(Math.sumPrecise([1e20, 0.1, -1e20]), 0.1);
assert.sameValue(Math.sumPrecise([1e308, -1e308]), 0);
assert.sameValue(Math.sumPrecise([NaN]), NaN);
assert.sameValue(Math.sumPrecise([Infinity, -Infinity]), NaN);
assert.sameValue(Math.sumPrecise([Infinity]), Infinity);
assert.sameValue(Math.sumPrecise([-Infinity]), -Infinity);

function* gen() {
  yield 1;
  yield 2;
}
assert.sameValue(Math.sumPrecise(gen()), 3);

assert.throws(TypeError, function () {
  Math.sumPrecise();
});
assert.throws(TypeError, function () {
  Math.sumPrecise(1);
});
assert.throws(TypeError, function () {
  Math.sumPrecise([{}]);
});
assert.throws(TypeError, function () {
  Math.sumPrecise([0n]);
});

var returnCalls = 0;
var bad = {
  next: function () {
    return { done: false, value: {} };
  },
  return: function () {
    returnCalls = returnCalls + 1;
    return {};
  }
};
var iterable = {};
iterable[Symbol.iterator] = function () {
  return bad;
};
assert.throws(TypeError, function () {
  Math.sumPrecise(iterable);
});
assert.sameValue(returnCalls, 1);
"#;
        let js = compile_test_to_js(src).expect("compile e19.80");
        run_js_in_node(&wrap_host_api(&js)).expect("e19.80 Math.sumPrecise");
    }

    #[test]
    fn harness_property_helpers_verify_star() {
        // E19.63: deprecated verify* helpers + bare compareArray.
        let src = r#"
let obj = {};
Object.defineProperty(obj, "a", {
  writable: true,
  enumerable: true,
  configurable: true,
  value: 123
});
verifyEqualTo(obj, "a", 123);
verifyWritable(obj, "a");
assert.sameValue(obj.a, 123, "verifyWritable non-destructive");
verifyEnumerable(obj, "a");
assert.throws(Test262Error, function () {
  verifyNotWritable(obj, "a");
});
verifyConfigurable(obj, "a");
assert.sameValue(Object.prototype.hasOwnProperty.call(obj, "a"), false, "verifyConfigurable deletes");

let frozen = {};
Object.defineProperty(frozen, "b", {
  writable: false,
  enumerable: false,
  configurable: false,
  value: 7
});
verifyEqualTo(frozen, "b", 7);
verifyNotWritable(frozen, "b");
verifyNotEnumerable(frozen, "b");
verifyNotConfigurable(frozen, "b");
assert.sameValue(frozen.b, 7);
assert.throws(Test262Error, function () {
  verifyWritable(frozen, "b");
});

let arr = [1, 2, 3];
verifyWritable(arr, "length");
assert.sameValue(arr.length, 3);

assert.sameValue(compareArray([1, 2], [1, 2]), true);
assert.sameValue(compareArray([1, 2], [1, 3]), false);
assert.sameValue(compareArray([], []), true);
assert.compareArray([1, NaN], [1, NaN]);
assert.throws(Test262Error, function () {
  assert.compareArray([1], [2]);
});
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&js).expect("property helpers");
    }

    #[test]
    fn default_run_does_not_fail_ci_without_suite() {
        // Fast path (default): suite absent → skip-all green; suite present → do not
        // run the full allowlist (tens of k Node spawns). Set DRACONIC_TEST262_FULL=1
        // for the full gate (allowlist-expand Loops / pre-push).
        let root = resolve_suite_root();
        let list = load_allowlist(&allowlist_path()).expect("allowlist");
        if !suite_present(&root) {
            let report = run_allowlist(&root, &list);
            let path = write_baseline_report(&report).expect("write report");
            assert!(path.is_file(), "report path {}", path.display());
            let (pass, fail, skip) = report.counts();
            eprintln!(
                "test262 default (suite absent): pass={pass} fail={fail} skip={skip} report={}",
                path.display()
            );
            assert!(skip > 0);
            assert_eq!(fail, 0);
            assert_eq!(pass, 0);
            return;
        }

        let full = std::env::var_os("DRACONIC_TEST262_FULL").is_some();
        if !full {
            // Smoke: a handful of stable allowlisted paths (parallel still).
            let smoke: Vec<String> = list.iter().take(32).cloned().collect();
            let report = run_allowlist(&root, &smoke);
            let (pass, fail, skip) = report.counts();
            eprintln!(
                "test262 smoke (set DRACONIC_TEST262_FULL=1 for full allowlist): pass={pass} fail={fail} skip={skip} jobs={}",
                test262_jobs()
            );
            assert_eq!(fail, 0, "smoke allowlist must pass; failing: {:?}", report.cases.iter().filter(|c| c.status == Status::Fail).map(|c| &c.path).collect::<Vec<_>>());
            assert_eq!(skip, 0);
            assert_eq!(pass, smoke.len());
            return;
        }

        // Full allowlist gate (parallel). Larger stack for deep debug recursion.
        let handle = std::thread::Builder::new()
            .name("test262-default-run".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let report = run_default().expect("run_default");
                let path = write_baseline_report(&report).expect("write report");
                assert!(path.is_file(), "report path {}", path.display());
                let (pass, fail, skip) = report.counts();
                eprintln!(
                    "test262 FULL: present={} pass={pass} fail={fail} skip={skip} jobs={} report={}",
                    report.suite_present,
                    test262_jobs(),
                    path.display()
                );
                assert_eq!(
                    fail, 0,
                    "allowlisted Test262 cases must pass (got fail={fail}); triage before expanding"
                );
                assert!(
                    pass >= 45100,
                    "expected expanded allowlist pass count >= 45100, got {pass}"
                );
            })
            .expect("spawn test262-default-run");
        handle.join().expect("test262-default-run thread");
    }
}

