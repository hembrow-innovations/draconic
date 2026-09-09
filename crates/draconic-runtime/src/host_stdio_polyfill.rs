/// JS polyfill for `stdoutWrite` (H02.01).
///
/// Node bridge via `process.stdout.write`. Accepts string (UTF-8) or `Uint8Array`
/// (raw bytes). No automatic newline — include `\n` in the string when needed.
pub fn stdout_write_js_polyfill() -> &'static str {
    r#"function stdoutWrite(data) {
  if (typeof process === "undefined" || !process || !process.stdout || typeof process.stdout.write !== "function") {
    return;
  }
  if (data == null) return;
  if (typeof data === "string") {
    process.stdout.write(data);
    return;
  }
  if (typeof Uint8Array !== "undefined" && data instanceof Uint8Array) {
    process.stdout.write(Buffer.from(data.buffer, data.byteOffset, data.byteLength));
    return;
  }
  if (typeof Buffer !== "undefined" && Buffer.isBuffer && Buffer.isBuffer(data)) {
    process.stdout.write(data);
    return;
  }
  process.stdout.write(String(data));
}
if (typeof globalThis !== "undefined") globalThis.stdoutWrite = stdoutWrite;
"#
}

/// JS polyfill for `stderrWrite` (H02.02).
///
/// Node bridge via `process.stderr.write`. Accepts string (UTF-8) or `Uint8Array`
/// (raw bytes). No automatic newline — include `\n` in the string when needed.
pub fn stderr_write_js_polyfill() -> &'static str {
    r#"function stderrWrite(data) {
  if (typeof process === "undefined" || !process || !process.stderr || typeof process.stderr.write !== "function") {
    return;
  }
  if (data == null) return;
  if (typeof data === "string") {
    process.stderr.write(data);
    return;
  }
  if (typeof Uint8Array !== "undefined" && data instanceof Uint8Array) {
    process.stderr.write(Buffer.from(data.buffer, data.byteOffset, data.byteLength));
    return;
  }
  if (typeof Buffer !== "undefined" && Buffer.isBuffer && Buffer.isBuffer(data)) {
    process.stderr.write(data);
    return;
  }
  process.stderr.write(String(data));
}
if (typeof globalThis !== "undefined") globalThis.stderrWrite = stderrWrite;
"#
}

/// JS polyfill for `stdinReadLine` / `stdinReadBytes` (H02.03).
///
/// Node bridge via `fs.readSync(0, …)` (blocking). Line strips trailing `\n` /
/// `\r\n`; EOF with no data → `null`. Bytes return a `Uint8Array` of actual
/// length (empty at EOF).
pub fn stdin_read_js_polyfill() -> &'static str {
    r#"function stdinReadLine() {
  var fs = require("fs");
  var chunks = [];
  var buf = Buffer.alloc(1);
  for (;;) {
    var n;
    try {
      n = fs.readSync(0, buf, 0, 1, null);
    } catch (e) {
      if (e && (e.code === "EOF" || e.code === "EAGAIN")) n = 0;
      else throw e;
    }
    if (n === 0) {
      if (chunks.length === 0) return null;
      break;
    }
    var c = buf[0];
    if (c === 10) break;
    chunks.push(c);
  }
  if (chunks.length > 0 && chunks[chunks.length - 1] === 13) chunks.pop();
  return Buffer.from(chunks).toString("utf8");
}
function stdinReadBytes(max) {
  var fs = require("fs");
  var m = Number(max);
  if (!(m > 0) || !isFinite(m)) return new Uint8Array(0);
  m = m >>> 0;
  if (m === 0) return new Uint8Array(0);
  var buf = Buffer.alloc(m);
  var n;
  try {
    n = fs.readSync(0, buf, 0, m, null);
  } catch (e) {
    if (e && (e.code === "EOF" || e.code === "EAGAIN")) n = 0;
    else throw e;
  }
  if (!n) return new Uint8Array(0);
  return new Uint8Array(buf.buffer, buf.byteOffset, n);
}
if (typeof globalThis !== "undefined") {
  globalThis.stdinReadLine = stdinReadLine;
  globalThis.stdinReadBytes = stdinReadBytes;
}
"#
}
