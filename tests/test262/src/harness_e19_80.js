// E19.80: Math.sumPrecise polyfill (Shewchuk exact sum; TC39 proposal / core-js)
(function () {
  if (typeof Math !== "object" || Math === null) {
    return;
  }
  if (typeof Math.sumPrecise === "function") {
    return;
  }

  var DONE = {};
  var NOT_A_NUMBER = {};
  var MINUS_INFINITY = {};
  var PLUS_INFINITY = {};
  var MINUS_ZERO = {};
  var FINITE = {};

  var POW_2_1023 = Math.pow(2, 1023);
  var MAX_SAFE_INTEGER = 9007199254740991;
  var MAX_DOUBLE = Number.MAX_VALUE;
  var MAX_ULP = Math.pow(2, 971);
  var INF = Infinity;
  var NEG_INF = -Infinity;

  function __draconicIsObject(v) {
    return (typeof v === "object" && v !== null) || typeof v === "function";
  }

  function __draconicRequireObjectCoercible(v) {
    if (v === undefined || v === null) {
      throw new TypeError("Math.sumPrecise: argument is null or undefined");
    }
    return v;
  }

  function __draconicGetIteratorDirect(obj) {
    return { iterator: obj, nextMethod: obj.next };
  }

  function __draconicGetIterator(obj) {
    var method = obj[Symbol.iterator];
    if (method === undefined || method === null) {
      throw new TypeError("Math.sumPrecise: argument is not iterable");
    }
    if (typeof method !== "function") {
      throw new TypeError("Math.sumPrecise: @@iterator is not callable");
    }
    var iter = method.call(obj);
    if (!__draconicIsObject(iter)) {
      throw new TypeError("Math.sumPrecise: @@iterator returned non-object");
    }
    return __draconicGetIteratorDirect(iter);
  }

  function __draconicIteratorNext(rec) {
    var result = rec.nextMethod.call(rec.iterator);
    if (!__draconicIsObject(result)) {
      throw new TypeError("IteratorNext: result is not an object");
    }
    return result;
  }

  function __draconicIteratorStepValue(rec) {
    var result = __draconicIteratorNext(rec);
    if (result.done) {
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
    var threw = false;
    var thrownValue;
    var innerResult;
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

  // prerequisite: Math.abs(x) >= Math.abs(y)
  function __draconicTwoSum(x, y) {
    var hi = x + y;
    var lo = y - (hi - x);
    return { hi: hi, lo: lo };
  }

  function __draconicExactSum(numbers) {
    var partials = [];
    var overflow = 0;
    var i;
    var j;
    var x;
    var y;
    var sum;
    var hi;
    var lo;
    var tmp;
    var actuallyUsedPartials;
    var sign;
    var n;
    var next;

    for (i = 0; i < numbers.length; i++) {
      x = numbers[i];
      actuallyUsedPartials = 0;
      for (j = 0; j < partials.length; j++) {
        y = partials[j];
        if (Math.abs(x) < Math.abs(y)) {
          tmp = x;
          x = y;
          y = tmp;
        }
        sum = __draconicTwoSum(x, y);
        hi = sum.hi;
        lo = sum.lo;
        if (Math.abs(hi) === INF) {
          sign = hi === INF ? 1 : -1;
          overflow += sign;
          x = x - sign * POW_2_1023 - sign * POW_2_1023;
          if (Math.abs(x) < Math.abs(y)) {
            tmp = x;
            x = y;
            y = tmp;
          }
          sum = __draconicTwoSum(x, y);
          hi = sum.hi;
          lo = sum.lo;
        }
        if (lo !== 0) {
          partials[actuallyUsedPartials++] = lo;
        }
        x = hi;
      }
      partials.length = actuallyUsedPartials;
      if (x !== 0) {
        partials.push(x);
      }
    }

    n = partials.length - 1;
    hi = 0;
    lo = 0;

    if (overflow !== 0) {
      next = n >= 0 ? partials[n] : 0;
      n--;
      if (
        Math.abs(overflow) > 1 ||
        (overflow > 0 && next > 0) ||
        (overflow < 0 && next < 0)
      ) {
        return overflow > 0 ? INF : NEG_INF;
      }
      sum = __draconicTwoSum(overflow * POW_2_1023, next / 2);
      hi = sum.hi;
      lo = sum.lo;
      lo *= 2;
      if (Math.abs(2 * hi) === INF) {
        if (hi > 0) {
          return hi === POW_2_1023 &&
            lo === -(MAX_ULP / 2) &&
            n >= 0 &&
            partials[n] < 0
            ? MAX_DOUBLE
            : INF;
        }
        return hi === -POW_2_1023 &&
          lo === MAX_ULP / 2 &&
          n >= 0 &&
          partials[n] > 0
          ? -MAX_DOUBLE
          : NEG_INF;
      }
      if (lo !== 0) {
        n++;
        partials[n] = lo;
        lo = 0;
      }
      hi *= 2;
    }

    while (n >= 0) {
      sum = __draconicTwoSum(hi, partials[n]);
      n--;
      hi = sum.hi;
      lo = sum.lo;
      if (lo !== 0) {
        break;
      }
    }

    if (
      n >= 0 &&
      ((lo < 0 && partials[n] < 0) || (lo > 0 && partials[n] > 0))
    ) {
      y = lo * 2;
      x = hi + y;
      if (y === x - hi) {
        hi = x;
      }
    }

    return hi;
  }

  var holder = {
    sumPrecise(items) {
      __draconicRequireObjectCoercible(items);
      var rec = __draconicGetIterator(items);
      var numbers = [];
      var count = 0;
      var state = MINUS_ZERO;
      var next;
      var n;

      for (;;) {
        next = __draconicIteratorStepValue(rec);
        if (next === DONE) {
          break;
        }
        count++;
        if (count > MAX_SAFE_INTEGER) {
          __draconicIteratorClose(
            rec,
            new RangeError("Math.sumPrecise: maximum allowed index exceeded")
          );
        }
        if (typeof next !== "number") {
          __draconicIteratorClose(
            rec,
            new TypeError("Math.sumPrecise: value is not a number")
          );
        }
        if (state !== NOT_A_NUMBER) {
          if (next !== next) {
            state = NOT_A_NUMBER;
          } else if (next === INF) {
            state = state === MINUS_INFINITY ? NOT_A_NUMBER : PLUS_INFINITY;
          } else if (next === NEG_INF) {
            state = state === PLUS_INFINITY ? NOT_A_NUMBER : MINUS_INFINITY;
          } else if (
            (next !== 0 || 1 / next === INF) &&
            (state === MINUS_ZERO || state === FINITE)
          ) {
            state = FINITE;
            numbers.push(next);
          }
        }
      }

      if (state === NOT_A_NUMBER) {
        return NaN;
      }
      if (state === MINUS_INFINITY) {
        return NEG_INF;
      }
      if (state === PLUS_INFINITY) {
        return INF;
      }
      if (state === MINUS_ZERO) {
        return -0;
      }
      return __draconicExactSum(numbers);
    }
  };

  var fn = holder.sumPrecise;
  Object.defineProperty(fn, "length", {
    value: 1,
    writable: false,
    enumerable: false,
    configurable: true
  });
  Object.defineProperty(fn, "name", {
    value: "sumPrecise",
    writable: false,
    enumerable: false,
    configurable: true
  });
  Object.defineProperty(Math, "sumPrecise", {
    value: fn,
    writable: true,
    enumerable: false,
    configurable: true
  });
})();
