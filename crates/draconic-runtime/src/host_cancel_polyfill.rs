/// JS polyfill for `makeCancelToken` / `cancelTokenAbort` / `cancelTokenAborted` /
/// `cancelTokenLink` (C05.01) and `withTimeout` / `clearWithTimeout` (C05.02).
///
/// Abort is sticky and idempotent. `cancelTokenLink(child, parent)` makes a
/// parent abort propagate to the child (immediately if the parent is already
/// aborted). Invalid handles return -1.
///
/// `withTimeout(ms)` returns a token that auto-aborts after ms (H05 timer).
/// `clearWithTimeout(token)` cancels the pending timer (work won; settle
/// cleanly). Invalid handles return -1.
pub fn cancel_token_js_polyfill() -> &'static str {
    r#"(function () {
  var nextId = 1;
  var slots = Object.create(null);
  function makeCancelToken() {
    var id = nextId++;
    slots[id] = { aborted: 0, links: [], timer: null };
    return id;
  }
  function cancelTokenAbort(t) {
    var s = slots[t];
    if (!s) return -1;
    if (s.aborted) return 0;
    s.aborted = 1;
    if (s.timer != null) {
      clearTimeout(s.timer);
      s.timer = null;
    }
    var kids = s.links;
    var i;
    for (i = 0; i < kids.length; i++) {
      cancelTokenAbort(kids[i]);
    }
    return 0;
  }
  function cancelTokenAborted(t) {
    var s = slots[t];
    if (!s) return -1;
    return s.aborted ? 1 : 0;
  }
  function cancelTokenLink(child, parent) {
    var c = slots[child];
    var p = slots[parent];
    if (!c || !p) return -1;
    if (p.aborted) {
      cancelTokenAbort(child);
      return 0;
    }
    p.links.push(child);
    return 0;
  }
  function withTimeout(ms) {
    var tok = makeCancelToken();
    var s = slots[tok];
    s.timer = setTimeout(function () {
      s.timer = null;
      cancelTokenAbort(tok);
    }, ms);
    return tok;
  }
  function clearWithTimeout(t) {
    var s = slots[t];
    if (!s) return -1;
    if (s.timer != null) {
      clearTimeout(s.timer);
      s.timer = null;
    }
    return 0;
  }
  if (typeof globalThis !== "undefined") {
    globalThis.makeCancelToken = makeCancelToken;
    globalThis.cancelTokenAbort = cancelTokenAbort;
    globalThis.cancelTokenAborted = cancelTokenAborted;
    globalThis.cancelTokenLink = cancelTokenLink;
    globalThis.withTimeout = withTimeout;
    globalThis.clearWithTimeout = clearWithTimeout;
  }
})();
"#
}
