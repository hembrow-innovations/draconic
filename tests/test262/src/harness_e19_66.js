
// E19.66: nans.js
var NaNs = [
  NaN,
  Number.NaN,
  NaN * 0,
  0 / 0,
  Infinity / Infinity,
  -(0 / 0),
  Math.pow(-1, 0.5),
  -Math.pow(-1, 0.5),
  Number("Not-a-Number")
];

// E19.66: resizableArrayBufferUtils.js
// Helper to create subclasses without requiring top-level `class` in the shim.
// Named uniquely so tests that declare `class subClass` (e.g. ERM fixtures) still parse.
function __draconicRabSubClass(type) {
  try {
    return new Function("return class My" + type + " extends " + type + " {}")();
  } catch (e) {}
}

var MyUint8Array = __draconicRabSubClass("Uint8Array");
var MyFloat32Array = __draconicRabSubClass("Float32Array");
var MyBigInt64Array = __draconicRabSubClass("BigInt64Array");

var builtinCtors = [
  Uint8Array,
  Int8Array,
  Uint16Array,
  Int16Array,
  Uint32Array,
  Int32Array,
  Float32Array,
  Float64Array,
  Uint8ClampedArray
];

if (typeof Float16Array !== "undefined") {
  builtinCtors.push(Float16Array);
}
if (typeof BigUint64Array !== "undefined") {
  builtinCtors.push(BigUint64Array);
}
if (typeof BigInt64Array !== "undefined") {
  builtinCtors.push(BigInt64Array);
}

// Install on globalThis (no lexical `var`/`let`) so tests may declare their own
// `let ctors` / `const floatCtors` without duplicate-binding compile errors.
// Bare `ctors` / `floatCtors` resolve via the global object environment record.
var floatCtorsList = [Float32Array, Float64Array, MyFloat32Array];
if (typeof Float16Array !== "undefined") {
  floatCtorsList.push(Float16Array);
}
globalThis.floatCtors = floatCtorsList;

var ctorsList = builtinCtors.concat(MyUint8Array, MyFloat32Array);
if (typeof MyBigInt64Array !== "undefined") {
  ctorsList.push(MyBigInt64Array);
}
globalThis.ctors = ctorsList;
globalThis.MyBigInt64Array = MyBigInt64Array;
globalThis.MyUint8Array = MyUint8Array;
globalThis.MyFloat32Array = MyFloat32Array;

function CreateResizableArrayBuffer(byteLength, maxByteLength) {
  return new ArrayBuffer(byteLength, { maxByteLength: maxByteLength });
}

function Convert(item) {
  if (typeof item == "bigint") {
    return Number(item);
  }
  return item;
}

function ToNumbers(array) {
  var result = [];
  var i = 0;
  while (i < array.length) {
    result.push(Convert(array[i]));
    i = i + 1;
  }
  return result;
}

function MayNeedBigInt(ta, n) {
  assert.sameValue(typeof n, "number");
  if (
    (typeof BigInt64Array !== "undefined" && ta instanceof BigInt64Array) ||
    (typeof BigUint64Array !== "undefined" && ta instanceof BigUint64Array)
  ) {
    return BigInt(n);
  }
  return n;
}

function CreateRabForTest(ctor) {
  var rab = CreateResizableArrayBuffer(4 * ctor.BYTES_PER_ELEMENT, 8 * ctor.BYTES_PER_ELEMENT);
  var taWrite = new ctor(rab);
  var i = 0;
  while (i < 4) {
    taWrite[i] = MayNeedBigInt(taWrite, 2 * i);
    i = i + 1;
  }
  return rab;
}

function CollectValuesAndResize(n, values, rab, resizeAfter, resizeTo) {
  if (typeof n == "bigint") {
    values.push(Number(n));
  } else {
    values.push(n);
  }
  if (values.length == resizeAfter) {
    rab.resize(resizeTo);
  }
  return true;
}

function TestIterationAndResize(iterable, expected, rab, resizeAfter, newByteLength) {
  var values = [];
  var resized = false;
  var arrayValues = false;
  var iter = iterable[Symbol.iterator]();
  var step = iter.next();
  while (!step.done) {
    var value = step.value;
    if (Array.isArray(value)) {
      arrayValues = true;
      values.push([value[0], Number(value[1])]);
    } else {
      values.push(Number(value));
    }
    if (!resized && values.length == resizeAfter) {
      rab.resize(newByteLength);
      resized = true;
    }
    step = iter.next();
  }
  if (!arrayValues) {
    assert.compareArray([].concat(values), expected, "TestIterationAndResize: list of iterated values");
  } else {
    var i = 0;
    while (i < expected.length) {
      assert.compareArray(
        values[i],
        expected[i],
        "TestIterationAndResize: list of iterated lists of values"
      );
      i = i + 1;
    }
  }
  assert(resized, "TestIterationAndResize: resize condition should have been hit");
}
