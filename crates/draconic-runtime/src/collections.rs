//! L02.01 / L02.02: collections helpers polyfill (`groupBy` / `chunk` / `Deque`).

pub fn collections_js_polyfill() -> &'static str {
    r#"function groupBy(items, key) {
  if (!Array.isArray(items)) throw new TypeError("groupBy expects an array");
  var mode;
  if (key === undefined) mode = "id";
  else if (typeof key === "function") mode = "fn";
  else if (typeof key === "string") mode = "prop";
  else throw new TypeError("groupBy key must be a function or string");
  var out = {};
  for (var i = 0; i < items.length; i++) {
    var item = items[i];
    var k;
    if (mode === "id") k = item;
    else if (mode === "fn") k = key(item, i);
    else k = item == null ? undefined : item[key];
    k = String(k);
    if (!Object.prototype.hasOwnProperty.call(out, k)) out[k] = [];
    out[k].push(item);
  }
  return out;
}
function chunk(items, size) {
  if (!Array.isArray(items)) throw new TypeError("chunk expects an array");
  if (typeof size !== "number" || size !== size || size === Infinity || size === -Infinity
      || size <= 0 || size !== Math.floor(size)) {
    throw new RangeError("chunk size must be a positive integer");
  }
  var out = [];
  for (var i = 0; i < items.length; i += size) {
    out.push(items.slice(i, i + size));
  }
  return out;
}
function Deque() {
  var items = [];
  var self = this;
  if (!(this instanceof Deque)) {
    self = Object.create(Deque.prototype);
  }
  self.pushBack = function (v) { items.push(v); };
  self.pushFront = function (v) { items.unshift(v); };
  self.popBack = function () {
    return items.length === 0 ? undefined : items.pop();
  };
  self.popFront = function () {
    return items.length === 0 ? undefined : items.shift();
  };
  Object.defineProperty(self, "length", {
    get: function () { return items.length; }
  });
  return self;
}
if (typeof globalThis !== "undefined") {
  globalThis.groupBy = groupBy;
  globalThis.chunk = chunk;
  globalThis.Deque = Deque;
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyfill_defines_groupby_and_chunk() {
        let s = collections_js_polyfill();
        assert!(s.contains("function groupBy("), "{s}");
        assert!(s.contains("function chunk("), "{s}");
        assert!(s.contains("globalThis.groupBy = groupBy"), "{s}");
        assert!(s.contains("globalThis.chunk = chunk"), "{s}");
        assert!(s.contains("groupBy expects an array"), "{s}");
        assert!(s.contains("chunk expects an array"), "{s}");
        assert!(s.contains("positive integer"), "{s}");
        assert!(s.contains("TypeError"), "{s}");
        assert!(s.contains("RangeError"), "{s}");
    }

    #[test]
    fn polyfill_defines_deque() {
        let s = collections_js_polyfill();
        assert!(s.contains("function Deque("), "{s}");
        assert!(s.contains("globalThis.Deque = Deque"), "{s}");
        assert!(s.contains("pushBack"), "{s}");
        assert!(s.contains("pushFront"), "{s}");
        assert!(s.contains("popBack"), "{s}");
        assert!(s.contains("popFront"), "{s}");
        assert!(s.contains("length"), "{s}");
    }
}
