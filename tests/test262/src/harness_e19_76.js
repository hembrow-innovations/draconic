// E19.76: iteratorZipUtils harness helpers + Iterator.zip / Iterator.zipKeyed polyfill

// --- harness: proxyTrapsHelper.js ---
function allowProxyTraps(overrides, label) {
  var prefix = typeof label === "string" && label.length > 0 ? label + ": " : "";
  function throwTest262Error(msg) {
    return function () {
      Test262Error.thrower(prefix + msg);
    };
  }
  if (!overrides) {
    overrides = {};
  }
  return {
    getPrototypeOf: overrides.getPrototypeOf || throwTest262Error("[[GetPrototypeOf]] trap called"),
    setPrototypeOf: overrides.setPrototypeOf || throwTest262Error("[[SetPrototypeOf]] trap called"),
    isExtensible: overrides.isExtensible || throwTest262Error("[[IsExtensible]] trap called"),
    preventExtensions: overrides.preventExtensions || throwTest262Error("[[PreventExtensions]] trap called"),
    getOwnPropertyDescriptor: overrides.getOwnPropertyDescriptor || throwTest262Error("[[GetOwnProperty]] trap called"),
    has: overrides.has || throwTest262Error("[[HasProperty]] trap called"),
    get: overrides.get || throwTest262Error("[[Get]] trap called"),
    set: overrides.set || throwTest262Error("[[Set]] trap called"),
    deleteProperty: overrides.deleteProperty || throwTest262Error("[[Delete]] trap called"),
    defineProperty: overrides.defineProperty || throwTest262Error("[[DefineOwnProperty]] trap called"),
    enumerate: throwTest262Error("[[Enumerate]] trap called: this trap has been removed"),
    ownKeys: overrides.ownKeys || throwTest262Error("[[OwnPropertyKeys]] trap called"),
    apply: overrides.apply || throwTest262Error("[[Call]] trap called"),
    construct: overrides.construct || throwTest262Error("[[Construct]] trap called")
  };
}

// getWellKnownIntrinsicObject: see harness_e19_77.js (E19.77)

// --- harness: iteratorZipUtils.js ---
function assertIteratorResult(result, value, done, label) {
  assert.sameValue(
    Object.getPrototypeOf(result),
    Object.prototype,
    label + ": [[Prototype]] of iterator result is Object.prototype"
  );
  assert(Object.isExtensible(result), label + ": iterator result is extensible");
  var ownKeys = Reflect.ownKeys(result);
  assert.compareArray(ownKeys, ["value", "done"], label + ": iterator result properties");
  verifyProperty(result, "value", {
    value: value,
    writable: true,
    enumerable: true,
    configurable: true
  });
  verifyProperty(result, "done", {
    value: done,
    writable: true,
    enumerable: true,
    configurable: true
  });
}

function assertIsPackedArray(array, label) {
  assert(Array.isArray(array), label + ": array is an array exotic object");
  assert.sameValue(
    Object.getPrototypeOf(array),
    Array.prototype,
    label + ": [[Prototype]] of array is Array.prototype"
  );
  assert(Object.isExtensible(array), label + ": array is extensible");
  verifyProperty(array, "length", {
    writable: true,
    enumerable: false,
    configurable: false
  });
  var i = 0;
  while (i < array.length) {
    verifyProperty(array, i, {
      writable: true,
      enumerable: true,
      configurable: true
    });
    i = i + 1;
  }
}

function _assertIsNullProtoMutableObject(object, label) {
  assert.sameValue(
    Object.getPrototypeOf(object),
    null,
    label + ": [[Prototype]] of object is null"
  );
  assert(Object.isExtensible(object), label + ": object is extensible");
  var keys = Object.getOwnPropertyNames(object);
  var i = 0;
  while (i < keys.length) {
    verifyProperty(object, keys[i], {
      writable: true,
      enumerable: true,
      configurable: true
    });
    i = i + 1;
  }
}

function assertZipped(zipped, inputs, count, label) {
  var last = null;
  var i = 0;
  while (i < count) {
    var itemLabel = label + ", step " + i;
    var result = zipped.next();
    var value = result.value;
    assertIteratorResult(result, value, false, itemLabel);
    assert.notSameValue(value, last, itemLabel + ": returns a new array");
    last = value;
    var expected = inputs.map(function (array) {
      return array[i];
    });
    assert.compareArray(value, expected, itemLabel + ": values");
    assertIsPackedArray(value, itemLabel);
    i = i + 1;
  }
}

function assertZippedKeyed(zipped, inputs, count, label) {
  var last = null;
  var expectedKeys = Object.keys(inputs);
  var i = 0;
  while (i < count) {
    var itemLabel = label + ", step " + i;
    var result = zipped.next();
    var value = result.value;
    assertIteratorResult(result, value, false, itemLabel);
    assert.notSameValue(value, last, itemLabel + ": returns a new object");
    last = value;
    assert.compareArray(Reflect.ownKeys(value), expectedKeys, itemLabel + ": result object keys");
    var expectedValues = Object.values(inputs).map(function (array) {
      return array[i];
    });
    assert.compareArray(Object.values(value), expectedValues, itemLabel + ": result object values");
    _assertIsNullProtoMutableObject(value, itemLabel);
    i = i + 1;
  }
}

function forEachSequenceCombination(callback) {
  function test(inputs) {
    if (inputs.length === 0) {
      callback(inputs, "inputs = []", 0, 0);
      return;
    }
    var lengths = inputs.map(function (array) {
      return array.length;
    });
    var min = Math.min.apply(null, lengths);
    var max = Math.max.apply(null, lengths);
    var inputsLabel = "inputs = " + JSON.stringify(inputs);
    callback(inputs, inputsLabel, min, max);
  }

  function prefixes(s) {
    var out = [];
    var i = 0;
    while (i <= s.length) {
      out.push(s.slice(0, i));
      i = i + 1;
    }
    return out;
  }

  test([]);

  var p1 = prefixes("abcd");
  var a = 0;
  while (a < p1.length) {
    test([p1[a].split("")]);
    a = a + 1;
  }

  a = 0;
  while (a < p1.length) {
    var p2 = prefixes("efgh");
    var b = 0;
    while (b < p2.length) {
      test([p1[a].split(""), p2[b].split("")]);
      b = b + 1;
    }
    a = a + 1;
  }

  a = 0;
  while (a < p1.length) {
    p2 = prefixes("efgh");
    b = 0;
    while (b < p2.length) {
      var p3 = prefixes("ijkl");
      var c = 0;
      while (c < p3.length) {
        test([p1[a].split(""), p2[b].split(""), p3[c].split("")]);
        c = c + 1;
      }
      b = b + 1;
    }
    a = a + 1;
  }
}

function forEachSequenceCombinationKeyed(callback) {
  return forEachSequenceCombination(function (inputs, inputsLabel, min, max) {
    var object = {};
    var i = 0;
    while (i < inputs.length) {
      object["prop_" + i] = inputs[i];
      i = i + 1;
    }
    inputsLabel = "inputs = " + JSON.stringify(object);
    callback(object, inputsLabel, min, max);
  });
}

// --- Iterator.zip / Iterator.zipKeyed polyfill ---
(function () {
  if (typeof Iterator !== "function") {
    return;
  }
  if (typeof Iterator.zip === "function" && typeof Iterator.zipKeyed === "function") {
    return;
  }

  var DONE = {};

  function __draconicIsObject(v) {
    return (typeof v === "object" && v !== null) || typeof v === "function";
  }

  function __draconicGetOptionsObject(options) {
    if (options === undefined) {
      return Object.create(null);
    }
    if (__draconicIsObject(options)) {
      return options;
    }
    throw new TypeError("GetOptionsObject: options must be undefined or an object");
  }

  function __draconicGetIteratorDirect(obj) {
    var nextMethod = obj.next;
    return { iterator: obj, nextMethod: nextMethod };
  }

  function __draconicGetIterator(obj) {
    var method = obj[Symbol.iterator];
    if (method === undefined || method === null) {
      throw new TypeError("GetIterator: object is not iterable");
    }
    if (typeof method !== "function") {
      throw new TypeError("GetIterator: @@iterator is not callable");
    }
    var iter = method.call(obj);
    if (!__draconicIsObject(iter)) {
      throw new TypeError("GetIterator: @@iterator returned non-object");
    }
    return __draconicGetIteratorDirect(iter);
  }

  function __draconicGetIteratorFlattenable(obj) {
    if (!__draconicIsObject(obj)) {
      throw new TypeError("GetIteratorFlattenable: primitive not allowed");
    }
    var method = obj[Symbol.iterator];
    if (method !== undefined && method !== null) {
      if (typeof method !== "function") {
        throw new TypeError("GetIteratorFlattenable: @@iterator is not callable");
      }
      var iter = method.call(obj);
      if (!__draconicIsObject(iter)) {
        throw new TypeError("GetIteratorFlattenable: @@iterator returned non-object");
      }
      return __draconicGetIteratorDirect(iter);
    }
    return __draconicGetIteratorDirect(obj);
  }

  function __draconicIteratorNext(rec) {
    var result = rec.nextMethod.call(rec.iterator);
    if (!__draconicIsObject(result)) {
      throw new TypeError("IteratorNext: result is not an object");
    }
    return result;
  }

  function __draconicIteratorStep(rec) {
    var result = __draconicIteratorNext(rec);
    if (result.done) {
      return DONE;
    }
    return result;
  }

  function __draconicIteratorStepValue(rec) {
    var result = __draconicIteratorStep(rec);
    if (result === DONE) {
      return DONE;
    }
    return result.value;
  }

  function __draconicIteratorClose(rec, pendingError) {
    var iterator = rec.iterator;
    var returnMethod;
    try {
      returnMethod = iterator["return"];
    } catch (e) {
      if (pendingError) {
        throw pendingError;
      }
      throw e;
    }
    if (returnMethod === undefined || returnMethod === null) {
      if (pendingError) {
        throw pendingError;
      }
      return;
    }
    if (typeof returnMethod !== "function") {
      if (pendingError) {
        throw pendingError;
      }
      throw new TypeError("IteratorClose: return is not callable");
    }
    var innerResult;
    var threw = false;
    var thrownValue;
    try {
      innerResult = returnMethod.call(iterator);
    } catch (e) {
      threw = true;
      thrownValue = e;
    }
    if (pendingError) {
      throw pendingError;
    }
    if (threw) {
      throw thrownValue;
    }
    if (!__draconicIsObject(innerResult)) {
      throw new TypeError("IteratorClose: return returned non-object");
    }
  }

  function __draconicIteratorCloseAll(openIters, pendingError) {
    var list = openIters.slice();
    var i = list.length - 1;
    while (i >= 0) {
      try {
        __draconicIteratorClose(list[i], pendingError);
        pendingError = null;
      } catch (e) {
        pendingError = e;
      }
      i = i - 1;
    }
    if (pendingError) {
      throw pendingError;
    }
  }

  function __draconicRemoveFromOpen(openIters, rec) {
    var i = 0;
    while (i < openIters.length) {
      if (openIters[i] === rec) {
        openIters.splice(i, 1);
        return;
      }
      i = i + 1;
    }
  }

  function __draconicCreateIteratorHelper(openIters, pullNext) {
    var helperProto = Object.getPrototypeOf(Iterator.from([]).drop(0));
    var state = "suspended-start";
    var obj = Object.create(helperProto);

    function next() {
      if (state === "executing") {
        throw new TypeError("Iterator helper next called while executing");
      }
      if (state === "completed") {
        return { value: undefined, done: true };
      }
      state = "executing";
      try {
        var result = pullNext();
        if (result.done) {
          state = "completed";
          return { value: undefined, done: true };
        }
        state = "suspended-yield";
        return { value: result.value, done: false };
      } catch (e) {
        state = "completed";
        throw e;
      }
    }

    function ret() {
      if (state === "executing") {
        throw new TypeError("Iterator helper return called while executing");
      }
      if (state === "completed") {
        return { value: undefined, done: true };
      }
      if (state === "suspended-start") {
        // Spec: set completed first, then IteratorCloseAll (nested return is no-op).
        state = "completed";
        __draconicIteratorCloseAll(openIters, null);
        return { value: undefined, done: true };
      }
      // suspended-yield: set executing so nested next/return throw TypeError during close.
      state = "executing";
      try {
        __draconicIteratorCloseAll(openIters, null);
      } finally {
        state = "completed";
      }
      return { value: undefined, done: true };
    }

    Object.defineProperty(obj, "next", {
      value: next,
      writable: true,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(obj, "return", {
      value: ret,
      writable: true,
      enumerable: false,
      configurable: true
    });
    return obj;
  }

  function __draconicIteratorZip(iters, mode, padding, finishResults) {
    var iterCount = iters.length;
    var openIters = iters.slice();

    function pullNext() {
      if (iterCount === 0) {
        return { done: true, value: undefined };
      }
      var results = [];
      var i = 0;
      while (i < iterCount) {
        var iter = iters[i];
        var result;
        if (iter === null) {
          result = padding[i];
        } else {
          try {
            result = __draconicIteratorStepValue(iter);
          } catch (e) {
            __draconicRemoveFromOpen(openIters, iter);
            __draconicIteratorCloseAll(openIters, e);
            throw e;
          }
          if (result === DONE) {
            __draconicRemoveFromOpen(openIters, iter);
            if (mode === "shortest") {
              __draconicIteratorCloseAll(openIters, null);
              return { done: true, value: undefined };
            } else if (mode === "strict") {
              if (i !== 0) {
                var errEarly = new TypeError("Iterator.zip strict: early exhaustion");
                __draconicIteratorCloseAll(openIters, errEarly);
                throw errEarly;
              }
              var k = 1;
              while (k < iterCount) {
                var other = iters[k];
                var open;
                try {
                  open = __draconicIteratorStep(other);
                } catch (e2) {
                  __draconicRemoveFromOpen(openIters, other);
                  __draconicIteratorCloseAll(openIters, e2);
                  throw e2;
                }
                if (open === DONE) {
                  __draconicRemoveFromOpen(openIters, other);
                } else {
                  var errUnequal = new TypeError("Iterator.zip strict: unequal lengths");
                  __draconicIteratorCloseAll(openIters, errUnequal);
                  throw errUnequal;
                }
                k = k + 1;
              }
              return { done: true, value: undefined };
            } else {
              if (openIters.length === 0) {
                return { done: true, value: undefined };
              }
              iters[i] = null;
              result = padding[i];
            }
          }
        }
        results.push(result);
        i = i + 1;
      }
      return { done: false, value: finishResults(results) };
    }

    return __draconicCreateIteratorHelper(openIters, pullNext);
  }

  function __draconicReadZipModeAndPadding(options) {
    var mode = options.mode;
    if (mode === undefined) {
      mode = "shortest";
    }
    if (mode !== "shortest" && mode !== "longest" && mode !== "strict") {
      throw new TypeError("Iterator.zip: invalid mode");
    }
    var paddingOption = undefined;
    if (mode === "longest") {
      paddingOption = options.padding;
      if (paddingOption !== undefined && !__draconicIsObject(paddingOption)) {
        throw new TypeError("Iterator.zip: padding must be an object");
      }
    }
    return { mode: mode, paddingOption: paddingOption };
  }

  function __draconicBuildPaddingFromIterable(paddingOption, iterCount, openIters) {
    var padding = [];
    var j;
    if (paddingOption === undefined) {
      j = 0;
      while (j < iterCount) {
        padding.push(undefined);
        j = j + 1;
      }
      return padding;
    }
    var paddingIter;
    try {
      paddingIter = __draconicGetIterator(paddingOption);
    } catch (e) {
      __draconicIteratorCloseAll(openIters, e);
      throw e;
    }
    var usingIterator = true;
    j = 0;
    while (j < iterCount) {
      if (usingIterator) {
        var next;
        try {
          next = __draconicIteratorStepValue(paddingIter);
        } catch (e2) {
          __draconicIteratorCloseAll(openIters, e2);
          throw e2;
        }
        if (next === DONE) {
          usingIterator = false;
          padding.push(undefined);
        } else {
          padding.push(next);
        }
      } else {
        padding.push(undefined);
      }
      j = j + 1;
    }
    if (usingIterator) {
      try {
        __draconicIteratorClose(paddingIter, null);
      } catch (e3) {
        __draconicIteratorCloseAll(openIters, e3);
        throw e3;
      }
    }
    return padding;
  }

  if (typeof Iterator.zip !== "function") {
    var zipHolder = {
      zip(iterables, options) {
        if (!__draconicIsObject(iterables)) {
          throw new TypeError("Iterator.zip: iterables is not an object");
        }
        options = __draconicGetOptionsObject(options);
        var mp = __draconicReadZipModeAndPadding(options);
        var mode = mp.mode;
        var paddingOption = mp.paddingOption;
        var iters = [];
        var padding = [];
        var inputIter = __draconicGetIterator(iterables);
        var doneInput = false;
        while (!doneInput) {
          var next;
          try {
            next = __draconicIteratorStepValue(inputIter);
          } catch (e) {
            __draconicIteratorCloseAll(iters, e);
            throw e;
          }
          if (next === DONE) {
            doneInput = true;
          } else {
            var iter;
            try {
              iter = __draconicGetIteratorFlattenable(next);
            } catch (e2) {
              var closeList = [inputIter].concat(iters);
              __draconicIteratorCloseAll(closeList, e2);
              throw e2;
            }
            iters.push(iter);
          }
        }
        var iterCount = iters.length;
        if (mode === "longest") {
          padding = __draconicBuildPaddingFromIterable(paddingOption, iterCount, iters);
        }
        function finishResults(results) {
          return results.slice();
        }
        return __draconicIteratorZip(iters, mode, padding, finishResults);
      }
    };
    var zip = zipHolder.zip;
    Object.defineProperty(zip, "length", {
      value: 1,
      writable: false,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(zip, "name", {
      value: "zip",
      writable: false,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(Iterator, "zip", {
      value: zip,
      writable: true,
      enumerable: false,
      configurable: true
    });
  }

  if (typeof Iterator.zipKeyed !== "function") {
    var zipKeyedHolder = {
      zipKeyed(iterables, options) {
        if (!__draconicIsObject(iterables)) {
          throw new TypeError("Iterator.zipKeyed: iterables is not an object");
        }
        options = __draconicGetOptionsObject(options);
        var mp = __draconicReadZipModeAndPadding(options);
        var mode = mp.mode;
        var paddingOption = mp.paddingOption;
        var iters = [];
        var padding = [];
        var keys = [];
        var allKeys = Reflect.ownKeys(iterables);
        var ki = 0;
        while (ki < allKeys.length) {
          var key = allKeys[ki];
          var desc;
          try {
            desc = Object.getOwnPropertyDescriptor(iterables, key);
          } catch (e) {
            __draconicIteratorCloseAll(iters, e);
            throw e;
          }
          if (desc !== undefined && desc.enumerable === true) {
            var value;
            try {
              value = iterables[key];
            } catch (e2) {
              __draconicIteratorCloseAll(iters, e2);
              throw e2;
            }
            if (value !== undefined) {
              keys.push(key);
              var iter;
              try {
                iter = __draconicGetIteratorFlattenable(value);
              } catch (e3) {
                __draconicIteratorCloseAll(iters, e3);
                throw e3;
              }
              iters.push(iter);
            }
          }
          ki = ki + 1;
        }
        var iterCount = iters.length;
        var j;
        if (mode === "longest") {
          if (paddingOption === undefined) {
            j = 0;
            while (j < iterCount) {
              padding.push(undefined);
              j = j + 1;
            }
          } else {
            j = 0;
            while (j < keys.length) {
              var padVal;
              try {
                padVal = paddingOption[keys[j]];
              } catch (e4) {
                __draconicIteratorCloseAll(iters, e4);
                throw e4;
              }
              padding.push(padVal);
              j = j + 1;
            }
          }
        }
        function finishResults(results) {
          var obj = Object.create(null);
          var i = 0;
          while (i < iterCount) {
            obj[keys[i]] = results[i];
            i = i + 1;
          }
          return obj;
        }
        return __draconicIteratorZip(iters, mode, padding, finishResults);
      }
    };
    var zipKeyed = zipKeyedHolder.zipKeyed;
    Object.defineProperty(zipKeyed, "length", {
      value: 1,
      writable: false,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(zipKeyed, "name", {
      value: "zipKeyed",
      writable: false,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(Iterator, "zipKeyed", {
      value: zipKeyed,
      writable: true,
      enumerable: false,
      configurable: true
    });
  }
})();
