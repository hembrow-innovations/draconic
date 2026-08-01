// E19.77: WellKnownIntrinsicObjects (minimal) + %ThrowTypeError% for caller/arguments tests
// Host wrap also patches Function.prototype.toString / caller / arguments (see wrap_host_api).

var WellKnownIntrinsicObjects = [
  { name: "%AggregateError%", source: "AggregateError" },
  { name: "%Array%", source: "Array" },
  { name: "%ArrayBuffer%", source: "ArrayBuffer" },
  { name: "%ArrayIteratorPrototype%", source: "Object.getPrototypeOf([][Symbol.iterator]())" },
  { name: "%AsyncFromSyncIteratorPrototype%", source: "" },
  { name: "%AsyncFunction%", source: "(async function() {}).constructor" },
  { name: "%AsyncGeneratorFunction%", source: "(async function* () {}).constructor" },
  { name: "%AsyncGeneratorPrototype%", source: "Object.getPrototypeOf(async function* () {}).prototype" },
  { name: "%AsyncIteratorPrototype%", source: "Object.getPrototypeOf(Object.getPrototypeOf(async function* () {}).prototype)" },
  { name: "%Atomics%", source: "Atomics" },
  { name: "%BigInt%", source: "BigInt" },
  { name: "%BigInt64Array%", source: "BigInt64Array" },
  { name: "%BigUint64Array%", source: "BigUint64Array" },
  { name: "%Boolean%", source: "Boolean" },
  { name: "%DataView%", source: "DataView" },
  { name: "%Date%", source: "Date" },
  { name: "%decodeURI%", source: "decodeURI" },
  { name: "%decodeURIComponent%", source: "decodeURIComponent" },
  { name: "%encodeURI%", source: "encodeURI" },
  { name: "%encodeURIComponent%", source: "encodeURIComponent" },
  { name: "%Error%", source: "Error" },
  { name: "%eval%", source: "eval" },
  { name: "%EvalError%", source: "EvalError" },
  { name: "%FinalizationRegistry%", source: "FinalizationRegistry" },
  { name: "%Float16Array%", source: "Float16Array" },
  { name: "%Float32Array%", source: "Float32Array" },
  { name: "%Float64Array%", source: "Float64Array" },
  { name: "%Function%", source: "Function" },
  { name: "%GeneratorFunction%", source: "(function* () {}).constructor" },
  { name: "%GeneratorPrototype%", source: "Object.getPrototypeOf(function* () {}).prototype" },
  { name: "%Int8Array%", source: "Int8Array" },
  { name: "%Int16Array%", source: "Int16Array" },
  { name: "%Int32Array%", source: "Int32Array" },
  { name: "%isFinite%", source: "isFinite" },
  { name: "%isNaN%", source: "isNaN" },
  { name: "%Iterator%", source: "Iterator" },
  { name: "%JSON%", source: "JSON" },
  { name: "%Map%", source: "Map" },
  { name: "%MapIteratorPrototype%", source: "Object.getPrototypeOf(new Map()[Symbol.iterator]())" },
  { name: "%Math%", source: "Math" },
  { name: "%Number%", source: "Number" },
  { name: "%Object%", source: "Object" },
  { name: "%parseFloat%", source: "parseFloat" },
  { name: "%parseInt%", source: "parseInt" },
  { name: "%Promise%", source: "Promise" },
  { name: "%Proxy%", source: "Proxy" },
  { name: "%RangeError%", source: "RangeError" },
  { name: "%ReferenceError%", source: "ReferenceError" },
  { name: "%Reflect%", source: "Reflect" },
  { name: "%RegExp%", source: "RegExp" },
  { name: "%Set%", source: "Set" },
  { name: "%SetIteratorPrototype%", source: "Object.getPrototypeOf(new Set()[Symbol.iterator]())" },
  { name: "%SharedArrayBuffer%", source: "SharedArrayBuffer" },
  { name: "%String%", source: "String" },
  { name: "%StringIteratorPrototype%", source: "Object.getPrototypeOf(''[Symbol.iterator]())" },
  { name: "%Symbol%", source: "Symbol" },
  { name: "%SyntaxError%", source: "SyntaxError" },
  { name: "%ThrowTypeError%", source: "" },
  { name: "%TypedArray%", source: "Object.getPrototypeOf(Uint8Array)" },
  { name: "%TypeError%", source: "TypeError" },
  { name: "%Uint8Array%", source: "Uint8Array" },
  { name: "%Uint8ClampedArray%", source: "Uint8ClampedArray" },
  { name: "%Uint16Array%", source: "Uint16Array" },
  { name: "%Uint32Array%", source: "Uint32Array" },
  { name: "%URIError%", source: "URIError" },
  { name: "%WeakMap%", source: "WeakMap" },
  { name: "%WeakRef%", source: "WeakRef" },
  { name: "%WeakSet%", source: "WeakSet" },
  { name: "%escape%", source: "escape" },
  { name: "%unescape%", source: "unescape" }
];

(function () {
  let i = 0;
  while (i < WellKnownIntrinsicObjects.length) {
    let wkio = WellKnownIntrinsicObjects[i];
    let actual = undefined;
    if (wkio.source) {
      try {
        actual = new Function("return (" + wkio.source + ")")();
      } catch (e) {}
    }
    if (wkio.name === "%ThrowTypeError%" && typeof globalThis.__test262ThrowTypeError === "function") {
      actual = globalThis.__test262ThrowTypeError;
    }
    wkio.value = actual;
    i = i + 1;
  }
})();

// Replaces E19.76 minimal getWellKnownIntrinsicObject; keeps IteratorHelper cases.
function getWellKnownIntrinsicObject(key) {
  let ix = 0;
  while (ix < WellKnownIntrinsicObjects.length) {
    if (WellKnownIntrinsicObjects[ix].name === key) {
      let value = WellKnownIntrinsicObjects[ix].value;
      if (value !== undefined) {
        return value;
      }
      throw new Test262Error("this implementation could not obtain " + key);
    }
    ix = ix + 1;
  }
  if (key === "%IteratorHelperPrototype%") {
    if (typeof Iterator !== "function") {
      throw new Test262Error("this implementation could not obtain " + key);
    }
    return Object.getPrototypeOf(Iterator.from([]).drop(0));
  }
  if (key === "%Iterator%") {
    if (typeof Iterator !== "function") {
      throw new Test262Error("this implementation could not obtain " + key);
    }
    return Iterator;
  }
  throw new Test262Error("unknown well-known intrinsic " + key);
}
