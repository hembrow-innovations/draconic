//! L05.01 / L05.02 / L05.03: in-language `describe` / `it` / `expect` + hooks.

pub fn describe_it_js_polyfill() -> &'static str {
    r#"var __testSuites = [{ beforeEach: [], afterEach: [], after: [] }];
function describe(name, fn) {
  if (typeof fn !== "function") throw new TypeError("describe expects a function");
  __testSuites.push({ beforeEach: [], afterEach: [], after: [] });
  try {
    fn();
  } finally {
    var suite = __testSuites[__testSuites.length - 1];
    var ai;
    for (ai = 0; ai < suite.after.length; ai++) suite.after[ai]();
    __testSuites.pop();
  }
}
function it(name, fn) {
  if (typeof fn !== "function") throw new TypeError("it expects a function");
  var ok = true;
  var s, i, hooks;
  try {
    for (s = 0; s < __testSuites.length; s++) {
      hooks = __testSuites[s].beforeEach;
      for (i = 0; i < hooks.length; i++) hooks[i]();
    }
    fn();
  } catch (e) {
    ok = false;
  }
  try {
    for (s = __testSuites.length - 1; s >= 0; s--) {
      hooks = __testSuites[s].afterEach;
      for (i = 0; i < hooks.length; i++) hooks[i]();
    }
  } catch (e) {
    ok = false;
  }
  if (!ok && typeof process !== "undefined") process.exitCode = 1;
  return ok;
}
function before(fn) {
  if (typeof fn !== "function") throw new TypeError("before expects a function");
  fn();
}
function after(fn) {
  if (typeof fn !== "function") throw new TypeError("after expects a function");
  __testSuites[__testSuites.length - 1].after.push(fn);
}
function beforeEach(fn) {
  if (typeof fn !== "function") throw new TypeError("beforeEach expects a function");
  __testSuites[__testSuites.length - 1].beforeEach.push(fn);
}
function afterEach(fn) {
  if (typeof fn !== "function") throw new TypeError("afterEach expects a function");
  __testSuites[__testSuites.length - 1].afterEach.push(fn);
}
function expectDisplay(v) {
  if (typeof v === "string") return JSON.stringify(v);
  return String(v);
}
function expect(actual) {
  return {
    toBe: function (expected) {
      if (actual === expected) return;
      throw "expected " + expectDisplay(actual) + " to be " + expectDisplay(expected);
    },
    toBeTruthy: function () {
      if (actual) return;
      throw "expected " + expectDisplay(actual) + " to be truthy";
    },
    toBeFalsy: function () {
      if (!actual) return;
      throw "expected " + expectDisplay(actual) + " to be falsy";
    }
  };
}
if (typeof globalThis !== "undefined") {
  globalThis.describe = describe;
  globalThis.it = it;
  globalThis.expect = expect;
  globalThis.before = before;
  globalThis.after = after;
  globalThis.beforeEach = beforeEach;
  globalThis.afterEach = afterEach;
}
"#
}
