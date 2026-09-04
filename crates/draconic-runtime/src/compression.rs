//! L04: gzip / zlib-deflate JS polyfill (`gzip` / `gunzip` / `deflate` / `inflate`).

pub fn compression_js_polyfill() -> &'static str {
    r#"function gzip(bytes) {
  if (bytes instanceof ArrayBuffer) bytes = new Uint8Array(bytes);
  if (!(bytes instanceof Uint8Array)) throw new TypeError("gzip expects Uint8Array");
  var z = null;
  try { z = require("zlib"); } catch (e) {}
  if (z && typeof z.gzipSync === "function") {
    return new Uint8Array(z.gzipSync(Buffer.from(bytes)));
  }
  throw new TypeError("gzip unavailable");
}
function gunzip(bytes) {
  if (bytes instanceof ArrayBuffer) bytes = new Uint8Array(bytes);
  if (!(bytes instanceof Uint8Array)) throw new TypeError("gunzip expects Uint8Array");
  var z = null;
  try { z = require("zlib"); } catch (e) {}
  if (z && typeof z.gunzipSync === "function") {
    try {
      return new Uint8Array(z.gunzipSync(Buffer.from(bytes)));
    } catch (e) {
      throw new Error("gunzip: invalid or truncated input");
    }
  }
  throw new TypeError("gunzip unavailable");
}
function deflate(bytes) {
  if (bytes instanceof ArrayBuffer) bytes = new Uint8Array(bytes);
  if (!(bytes instanceof Uint8Array)) throw new TypeError("deflate expects Uint8Array");
  var z = null;
  try { z = require("zlib"); } catch (e) {}
  if (z && typeof z.deflateSync === "function") {
    return new Uint8Array(z.deflateSync(Buffer.from(bytes)));
  }
  throw new TypeError("deflate unavailable");
}
function inflate(bytes) {
  if (bytes instanceof ArrayBuffer) bytes = new Uint8Array(bytes);
  if (!(bytes instanceof Uint8Array)) throw new TypeError("inflate expects Uint8Array");
  var z = null;
  try { z = require("zlib"); } catch (e) {}
  if (z && typeof z.inflateSync === "function") {
    try {
      return new Uint8Array(z.inflateSync(Buffer.from(bytes)));
    } catch (e) {
      throw new Error("inflate: invalid or truncated input");
    }
  }
  throw new TypeError("inflate unavailable");
}
if (typeof globalThis !== "undefined") {
  globalThis.gzip = gzip;
  globalThis.gunzip = gunzip;
  globalThis.deflate = deflate;
  globalThis.inflate = inflate;
}
"#
}
