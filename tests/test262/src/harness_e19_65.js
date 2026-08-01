
// E19.65: fnGlobalObject.js
var __globalObject = globalThis;
function fnGlobalObject() {
  return __globalObject;
}

// E19.65: decimalToHexString.js
function decimalToHexString(n) {
  var hex = "0123456789ABCDEF";
  n = n >>> 0;
  var s = "";
  while (n) {
    s = hex[n & 0xf] + s;
    n = n >>> 4;
  }
  while (s.length < 4) {
    s = "0" + s;
  }
  return s;
}
function decimalToPercentHexString(n) {
  var hex = "0123456789ABCDEF";
  return "%" + hex[(n >> 4) & 0xf] + hex[n & 0xf];
}

// E19.65: promiseHelper.js
function checkSequence(arr, message) {
  var i = 0;
  while (i < arr.length) {
    if (arr[i] !== i + 1) {
      throw new Test262Error(
        (message ? message : "Steps in unexpected sequence:") + " '" + arr.join(",") + "'"
      );
    }
    i = i + 1;
  }
  return true;
}
function checkSettledPromises(settleds, expected, message) {
  var prefix = message ? message + ": " : "";
  assert.sameValue(Array.isArray(settleds), true, prefix + "Settled values is an array");
  assert.sameValue(
    settleds.length,
    expected.length,
    prefix + "The settled values has a different length than expected"
  );
  var i = 0;
  while (i < settleds.length) {
    var settled = settleds[i];
    assert.sameValue(
      Object.prototype.hasOwnProperty.call(settled, "status"),
      true,
      prefix + "The settled value has a property status"
    );
    assert.sameValue(settled.status, expected[i].status, prefix + "status for item " + i);
    if (settled.status === "fulfilled") {
      assert.sameValue(
        Object.prototype.hasOwnProperty.call(settled, "value"),
        true,
        prefix + "The fulfilled promise has a property named value"
      );
      assert.sameValue(
        Object.prototype.hasOwnProperty.call(settled, "reason"),
        false,
        prefix + "The fulfilled promise has no property named reason"
      );
      assert.sameValue(settled.value, expected[i].value, prefix + "value for item " + i);
    } else {
      assert.sameValue(
        settled.status,
        "rejected",
        prefix + "Valid statuses are only fulfilled or rejected"
      );
      assert.sameValue(
        Object.prototype.hasOwnProperty.call(settled, "value"),
        false,
        prefix + "The fulfilled promise has no property named value"
      );
      assert.sameValue(
        Object.prototype.hasOwnProperty.call(settled, "reason"),
        true,
        prefix + "The fulfilled promise has a property named reason"
      );
      assert.sameValue(settled.reason, expected[i].reason, prefix + "Reason value for item " + i);
    }
    i = i + 1;
  }
}

// E19.65: sm/assertThrowsValue.js
function assertThrowsValue(f, val, msg) {
  try {
    f();
  } catch (exc) {
    assert.sameValue(exc, val, msg);
    return;
  }
  var fullmsg = "Assertion failed: expected exception, no exception thrown";
  if (msg !== void 0) {
    fullmsg = fullmsg + " - " + msg;
  }
  throw new Test262Error(fullmsg);
}

// E19.65: tcoHelper.js
var $MAX_ITERATIONS = 100000;

// E19.65: nativeFunctionMatcher.js (simplified NativeFunction grammar check)
function validateNativeFunctionSource(source) {
  var isNewline = function (c) {
    return c === "\n" || c === "\r" || c === "\u2028" || c === "\u2029";
  };
  var isWhitespace = function (c) {
    return (
      c === "\t" ||
      c === "\u000b" ||
      c === "\f" ||
      c === " " ||
      c === "\u00a0" ||
      c === "\ufeff"
    );
  };
  var isIdStart = function (c) {
    if (c === undefined || c === null || c === "") {
      return false;
    }
    var ch = c.charCodeAt(0);
    if (c === "_" || c === "$") {
      return true;
    }
    if (ch >= 65 && ch <= 90) {
      return true;
    }
    if (ch >= 97 && ch <= 122) {
      return true;
    }
    return false;
  };
  var isIdContinue = function (c) {
    if (isIdStart(c)) {
      return true;
    }
    if (c === undefined || c === null || c === "") {
      return false;
    }
    var ch = c.charCodeAt(0);
    return ch >= 48 && ch <= 57;
  };
  var pos = 0;
  var eatWhitespace = function () {
    while (pos < source.length) {
      var c = source[pos];
      if (isWhitespace(c) || isNewline(c)) {
        pos = pos + 1;
        continue;
      }
      if (c === "/") {
        if (source[pos + 1] === "/") {
          while (pos < source.length) {
            if (isNewline(source[pos])) {
              break;
            }
            pos = pos + 1;
          }
          continue;
        }
        if (source[pos + 1] === "*") {
          var end = source.indexOf("*/", pos);
          if (end === -1) {
            throw new SyntaxError();
          }
          pos = end + 2;
          continue;
        }
      }
      break;
    }
  };
  var getIdentifier = function () {
    eatWhitespace();
    var start = pos;
    var end = pos;
    if (source[end] === "_" || source[end] === "$") {
      end = end + 1;
    } else if (isIdStart(source[end])) {
      end = end + 1;
    } else {
      return null;
    }
    while (end < source.length) {
      var c = source[end];
      if (c === "_" || c === "$" || isIdContinue(c)) {
        end = end + 1;
      } else {
        return source.slice(start, end);
      }
    }
    return source.slice(start, end);
  };
  var testTok = function (s) {
    eatWhitespace();
    if (/^\w/.test(s)) {
      return getIdentifier() === s;
    }
    return source.slice(pos, pos + s.length) === s;
  };
  var eat = function (s) {
    if (testTok(s)) {
      pos = pos + s.length;
      return true;
    }
    return false;
  };
  var eatIdentifier = function () {
    var n = getIdentifier();
    if (n !== null) {
      pos = pos + n.length;
      return true;
    }
    return false;
  };
  var expect = function (s) {
    if (!eat(s)) {
      throw new SyntaxError();
    }
  };
  var eatString = function () {
    if (source[pos] === "'" || source[pos] === '"') {
      var match = source[pos];
      pos = pos + 1;
      while (pos < source.length) {
        if (source[pos] === match && source[pos - 1] !== "\\") {
          return;
        }
        if (isNewline(source[pos])) {
          throw new SyntaxError();
        }
        pos = pos + 1;
      }
      throw new SyntaxError();
    }
  };
  var stumbleUntil = function (c) {
    var open = c === "]" ? "[" : "(";
    var nesting = 1;
    while (pos < source.length) {
      eatWhitespace();
      eatString();
      if (source[pos] === open) {
        nesting = nesting + 1;
      } else if (source[pos] === c) {
        nesting = nesting - 1;
      }
      pos = pos + 1;
      if (nesting === 0) {
        return;
      }
    }
    throw new SyntaxError();
  };
  expect("function");
  if (!eat("get")) {
    eat("set");
  }
  if (!eatIdentifier() && eat("[")) {
    stumbleUntil("]");
  }
  expect("(");
  stumbleUntil(")");
  expect("{");
  expect("[");
  expect("native");
  expect("code");
  expect("]");
  expect("}");
  eatWhitespace();
  if (pos !== source.length) {
    throw new SyntaxError();
  }
}
function assertToStringOrNativeFunction(fn, expected) {
  var actual = "" + fn;
  try {
    assert.sameValue(actual, expected);
  } catch (unused) {
    assertNativeFunction(fn, expected);
  }
}
function assertNativeFunction(fn, special) {
  var actual = "" + fn;
  try {
    validateNativeFunctionSource(actual);
  } catch (unused) {
    throw new Test262Error(
      "Conforms to NativeFunction Syntax: " +
        JSON.stringify(actual) +
        (special ? " (" + special + ")" : "")
    );
  }
}

// E19.65: sm/non262-Math-shell.js assertNear (+ epsilons)
var ONE_PLUS_EPSILON = 1 + Math.pow(2, -52);
var ONE_MINUS_EPSILON = 1 - Math.pow(2, -53);
var __assertNearEndian = 0;
var __assertNearF = new Float64Array([0, 0]);
var __assertNearU = new Uint32Array(__assertNearF.buffer);
function __assertNearDiff(a, b) {
  __assertNearF[0] = a;
  __assertNearF[1] = b;
  var hiA = __assertNearU[1 - __assertNearEndian];
  var loA = __assertNearU[0 + __assertNearEndian];
  var hiB = __assertNearU[3 - __assertNearEndian];
  var loB = __assertNearU[2 + __assertNearEndian];
  return Math.abs((hiB - hiA) * 0x100000000 + loB - loA);
}
(function () {
  __assertNearEndian = 0;
  if (__assertNearDiff(2, 4) === 0x100000) {
    __assertNearEndian = 1;
  }
})();
function assertNear(a, b, tolerance) {
  if (tolerance === undefined) {
    tolerance = 1;
  }
  if (!Number.isFinite(b)) {
    throw new Error("second argument to assertNear (expected value) must be a finite number");
  } else if (Number.isNaN(a)) {
    throw new Error("got NaN, expected a number near " + b);
  } else if (!Number.isFinite(a)) {
    if (b * Math.sign(a) < Number.MAX_VALUE) {
      throw new Error("got " + a + ", expected a number near " + b);
    }
  } else {
    var target = b === 0 ? a * 0 : b;
    var err = __assertNearDiff(a, target);
    if (err > tolerance) {
      throw new Error(
        "got " + a + ", expected a number near " + b + " (relative error: " + err + ")"
      );
    }
  }
}
