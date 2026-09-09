/// JS polyfill for `nowMs()` (H05.01).
///
/// Node/browser wall clock via `Date.now()` (ms since Unix epoch).
pub fn now_ms_js_polyfill() -> &'static str {
    r#"function nowMs() {
  return Date.now();
}
if (typeof globalThis !== "undefined") {
  globalThis.nowMs = nowMs;
}
"#
}

/// JS polyfill for `monotonicMs()` (H05.02).
///
/// Prefer `performance.now()`; else Node `process.hrtime`; last resort wall clock.
pub fn monotonic_ms_js_polyfill() -> &'static str {
    r#"function monotonicMs() {
  if (typeof performance !== "undefined" && performance && typeof performance.now === "function") {
    return performance.now();
  }
  if (typeof process !== "undefined" && process && typeof process.hrtime === "function") {
    var t = process.hrtime();
    return t[0] * 1e3 + t[1] / 1e6;
  }
  return Date.now();
}
if (typeof globalThis !== "undefined") {
  globalThis.monotonicMs = monotonicMs;
}
"#
}

/// JS polyfill for `setTimeout` / `clearTimeout` (H05.03).
///
/// Bridges to the host event loop (Node/browser). Delay coerced with ToNumber;
/// missing/NaN/negative → 0.
pub fn set_timeout_js_polyfill() -> &'static str {
    r#"(function () {
  var _st = globalThis.setTimeout.bind(globalThis);
  var _ct = globalThis.clearTimeout.bind(globalThis);
  function setTimeout(fn, delay) {
    var d = delay == null ? 0 : +delay;
    if (!(d > 0)) d = 0;
    return _st(fn, d);
  }
  function clearTimeout(id) {
    return _ct(id);
  }
  globalThis.setTimeout = setTimeout;
  globalThis.clearTimeout = clearTimeout;
})();
"#
}

/// JS polyfill for `setInterval` / `clearInterval` (H05.04).
///
/// Bridges to the host event loop (Node/browser). Interval coerced with
/// ToNumber; missing/NaN/negative → 0.
pub fn set_interval_js_polyfill() -> &'static str {
    r#"(function () {
  var _si = globalThis.setInterval.bind(globalThis);
  var _ci = globalThis.clearInterval.bind(globalThis);
  function setInterval(fn, delay) {
    var d = delay == null ? 0 : +delay;
    if (!(d > 0)) d = 0;
    return _si(fn, d);
  }
  function clearInterval(id) {
    return _ci(id);
  }
  globalThis.setInterval = setInterval;
  globalThis.clearInterval = clearInterval;
})();
"#
}
