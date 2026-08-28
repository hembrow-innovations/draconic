//! L06.01: leveled logger polyfill (`createLogger`).

pub fn create_logger_js_polyfill() -> &'static str {
    r#"function createLogger(level) {
  var ranks = { debug: 0, info: 1, warn: 2, error: 3 };
  function norm(l) {
    if (typeof l !== "string" || ranks[l] === undefined) {
      throw new TypeError("invalid log level");
    }
    return l;
  }
  var current = arguments.length === 0 ? "info" : norm(level);
  var recs = [];
  function emit(lvl, msg) {
    if (ranks[lvl] < ranks[current]) return;
    recs.push({ level: lvl, message: String(msg) });
  }
  return {
    error: function (msg) { emit("error", msg); },
    warn: function (msg) { emit("warn", msg); },
    info: function (msg) { emit("info", msg); },
    debug: function (msg) { emit("debug", msg); },
    setLevel: function (l) { current = norm(l); },
    getLevel: function () { return current; },
    records: function () { return recs.slice(); },
    clear: function () { recs = []; }
  };
}
if (typeof globalThis !== "undefined") globalThis.createLogger = createLogger;
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyfill_defines_create_logger() {
        let s = create_logger_js_polyfill();
        assert!(s.contains("function createLogger("), "{s}");
        assert!(s.contains("globalThis.createLogger = createLogger"), "{s}");
        assert!(s.contains("debug"), "{s}");
        assert!(s.contains("setLevel"), "{s}");
        assert!(s.contains("records"), "{s}");
    }
}
