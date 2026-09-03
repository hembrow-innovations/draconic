//! H17.04 JS/Node bridge polyfills: HTTP/1.1 helpers, `dnsLookup`, sync TCP.
//!
//! HTTP helpers are portable (no Node). DNS/TCP use Node `dns` / `net` with a
//! `worker_threads` + `Atomics.wait` handshake so the Program API stays blocking.

pub fn http_js_polyfill() -> &'static str {
    r#"function __draconic_host_err(code, msg) {
  var e = new Error(msg ? (code + ": " + msg) : code);
  e.name = "HostError";
  e.code = code;
  throw e;
}
function __draconic_to_bin(x) {
  if (x == null) return "";
  if (typeof x === "string") return x;
  if (typeof Uint8Array !== "undefined" && x instanceof Uint8Array) {
    var s = "";
    for (var i = 0; i < x.length; i++) s += String.fromCharCode(x[i] & 255);
    return s;
  }
  if (typeof Buffer !== "undefined" && Buffer.isBuffer && Buffer.isBuffer(x)) {
    return x.toString("latin1");
  }
  return String(x);
}
function __http_default_reason(status) {
  switch (status) {
    case 200: return "OK";
    case 201: return "Created";
    case 204: return "No Content";
    case 301: return "Moved Permanently";
    case 302: return "Found";
    case 304: return "Not Modified";
    case 400: return "Bad Request";
    case 401: return "Unauthorized";
    case 403: return "Forbidden";
    case 404: return "Not Found";
    case 405: return "Method Not Allowed";
    case 500: return "Internal Server Error";
    case 502: return "Bad Gateway";
    case 503: return "Service Unavailable";
    default: return "";
  }
}
function __http_ieq(a, b) {
  return String(a).toLowerCase() === String(b).toLowerCase();
}
function __http_header_value(hdrs, name) {
  var src = hdrs == null ? "" : String(hdrs);
  var i = 0;
  while (i < src.length) {
    var lineEnd = src.indexOf("\r\n", i);
    if (lineEnd < 0) lineEnd = src.length;
    if (lineEnd === i) { i = lineEnd + 2; continue; }
    var line = src.slice(i, lineEnd);
    var colon = line.indexOf(":");
    if (colon >= 0) {
      var n = line.slice(0, colon).replace(/[ \t]+$/, "");
      if (__http_ieq(n, name)) {
        var v = line.slice(colon + 1).replace(/^[ \t]+/, "").replace(/[ \t]+$/, "");
        return v;
      }
    }
    i = lineEnd + 2;
  }
  return null;
}
function __http_headers_have_chunked_te(hdrs) {
  var te = __http_header_value(hdrs, "Transfer-Encoding");
  if (te == null) return false;
  return /(^|,)[ \t]*chunked[ \t]*(,|$)/i.test(te);
}
function __http_headers_have_content_length(hdrs) {
  return __http_header_value(hdrs, "Content-Length") != null;
}
function __http_ensure_crlf(hdrs) {
  var h = hdrs == null ? "" : String(hdrs);
  if (h.length === 0) return "";
  if (h.length >= 2 && h.slice(-2) === "\r\n") return h;
  return h + "\r\n";
}
function __http_chunked_body(body) {
  var b = body == null ? "" : String(body);
  var hex = b.length.toString(16);
  return hex + "\r\n" + b + "\r\n0\r\n\r\n";
}
function __http_decode_chunked(data) {
  var off = 0;
  var out = "";
  while (off < data.length) {
    var lineEnd = data.indexOf("\r\n", off);
    if (lineEnd < 0) return null;
    var sizeLine = data.slice(off, lineEnd);
    var semi = sizeLine.indexOf(";");
    if (semi >= 0) sizeLine = sizeLine.slice(0, semi);
    var chunkSize = parseInt(sizeLine, 16);
    if (!(chunkSize >= 0)) return null;
    var dataStart = lineEnd + 2;
    if (chunkSize === 0) return out;
    if (dataStart + chunkSize + 2 > data.length) return null;
    if (data.slice(dataStart + chunkSize, dataStart + chunkSize + 2) !== "\r\n") return null;
    out += data.slice(dataStart, dataStart + chunkSize);
    off = dataStart + chunkSize + 2;
  }
  return null;
}
function __http_resolve_body(raw, hdrs, bodyOff) {
  var bodyData = raw.slice(bodyOff);
  if (__http_headers_have_chunked_te(hdrs)) {
    var decoded = __http_decode_chunked(bodyData);
    if (decoded == null) __draconic_host_err("EINVAL", "malformed chunked body");
    return decoded;
  }
  var cl = __http_header_value(hdrs, "Content-Length");
  if (cl != null) {
    if (!/^\d+$/.test(cl)) __draconic_host_err("EINVAL", "bad Content-Length");
    var n = parseInt(cl, 10);
    if (n > bodyData.length) n = bodyData.length;
    return bodyData.slice(0, n);
  }
  return "";
}
function __http_lookup_header(raw, name) {
  if (raw == null) __draconic_host_err("EINVAL", "missing message");
  var s = __draconic_to_bin(raw);
  var idx = s.indexOf("\r\n\r\n");
  if (idx < 0) __draconic_host_err("EINVAL", "missing header terminator");
  var lineEnd = s.indexOf("\r\n");
  if (lineEnd < 0 || lineEnd > idx) __draconic_host_err("EINVAL", "bad start line");
  var hdrs = s.slice(lineEnd + 2, idx + 2);
  var v = __http_header_value(hdrs, name);
  return v == null ? "" : v;
}
function httpParseRequest(raw) {
  var s = __draconic_to_bin(raw);
  var idx = s.indexOf("\r\n\r\n");
  if (idx < 0) __draconic_host_err("EINVAL", "missing header terminator");
  var lineEnd = s.indexOf("\r\n");
  if (lineEnd < 0 || lineEnd > idx) __draconic_host_err("EINVAL", "bad request-line");
  var start = s.slice(0, lineEnd);
  var sp1 = start.indexOf(" ");
  var sp2 = sp1 < 0 ? -1 : start.indexOf(" ", sp1 + 1);
  if (sp1 < 1 || sp2 < 0 || sp2 + 1 >= start.length) {
    __draconic_host_err("EINVAL", "bad request-line");
  }
  var method = start.slice(0, sp1);
  var path = start.slice(sp1 + 1, sp2);
  var version = start.slice(sp2 + 1);
  if (!method || !path || !version) __draconic_host_err("EINVAL", "bad request-line");
  var hdrs = s.slice(lineEnd + 2, idx + 2);
  var body = __http_resolve_body(s, hdrs, idx + 4);
  return { method: method, path: path, version: version, body: body, __raw: s };
}
function httpRequestHeader(req, name) {
  if (req == null || typeof req !== "object") __draconic_host_err("EINVAL", "missing request");
  return __http_lookup_header(req.__raw, name);
}
function httpWriteResponse(status, reason, headers, body) {
  var st = status | 0;
  if (st < 100 || st > 599) __draconic_host_err("EINVAL", "bad status");
  var r = reason == null ? "" : String(reason);
  if (r.length === 0) r = __http_default_reason(st);
  var hdrs = __http_ensure_crlf(headers);
  var b = body == null ? "" : __draconic_to_bin(body);
  var useChunked = __http_headers_have_chunked_te(hdrs);
  var needCl = !useChunked && !__http_headers_have_content_length(hdrs);
  var msg = "HTTP/1.1 " + st + " " + r + "\r\n" + hdrs;
  if (needCl) msg += "Content-Length: " + b.length + "\r\n";
  msg += "\r\n";
  if (useChunked) msg += __http_chunked_body(b);
  else msg += b;
  return msg;
}
function httpWriteRequest(method, path, headers, body) {
  var m = method == null ? "" : String(method);
  var p = path == null ? "" : String(path);
  if (!m || !p) __draconic_host_err("EINVAL", "method/path required");
  if (/[ \t\r\n]/.test(m) || /[ \t\r\n]/.test(p)) __draconic_host_err("EINVAL", "bad method/path");
  var hdrs = __http_ensure_crlf(headers);
  var b = body == null ? "" : __draconic_to_bin(body);
  var useChunked = __http_headers_have_chunked_te(hdrs);
  var needCl = !useChunked && !__http_headers_have_content_length(hdrs);
  var msg = m + " " + p + " HTTP/1.1\r\n" + hdrs;
  if (needCl) msg += "Content-Length: " + b.length + "\r\n";
  msg += "\r\n";
  if (useChunked) msg += __http_chunked_body(b);
  else msg += b;
  return msg;
}
function httpParseResponse(raw) {
  var s = __draconic_to_bin(raw);
  var idx = s.indexOf("\r\n\r\n");
  if (idx < 0) __draconic_host_err("EINVAL", "missing header terminator");
  var lineEnd = s.indexOf("\r\n");
  if (lineEnd < 0 || lineEnd > idx) __draconic_host_err("EINVAL", "bad status-line");
  var start = s.slice(0, lineEnd);
  var sp1 = start.indexOf(" ");
  if (sp1 < 1) __draconic_host_err("EINVAL", "bad status-line");
  var rest = start.slice(sp1 + 1);
  var sp2 = rest.indexOf(" ");
  var version = start.slice(0, sp1);
  var statusStr = sp2 < 0 ? rest : rest.slice(0, sp2);
  var reason = sp2 < 0 ? "" : rest.slice(sp2 + 1);
  var status = parseInt(statusStr, 10);
  if (!(status >= 100 && status <= 599)) __draconic_host_err("EINVAL", "bad status");
  var hdrs = s.slice(lineEnd + 2, idx + 2);
  var body = __http_resolve_body(s, hdrs, idx + 4);
  return { version: version, status: status, reason: reason, body: body, __raw: s };
}
function httpResponseHeader(res, name) {
  if (res == null || typeof res !== "object") __draconic_host_err("EINVAL", "missing response");
  return __http_lookup_header(res.__raw, name);
}
if (typeof globalThis !== "undefined") {
  globalThis.httpParseRequest = httpParseRequest;
  globalThis.httpRequestHeader = httpRequestHeader;
  globalThis.httpWriteResponse = httpWriteResponse;
  globalThis.httpWriteRequest = httpWriteRequest;
  globalThis.httpParseResponse = httpParseResponse;
  globalThis.httpResponseHeader = httpResponseHeader;
}
"#
}

pub fn dns_js_polyfill() -> &'static str {
    r#"function __draconic_dns_err(code, msg) {
  var e = new Error(msg ? (code + ": " + msg) : code);
  e.name = "HostError";
  e.code = code;
  throw e;
}
function dnsLookup(host) {
  var h = host == null ? "" : String(host);
  if (!h) __draconic_dns_err("EINVAL", "empty host");
  if (/^\d{1,3}(?:\.\d{1,3}){3}$/.test(h)) return [h];
  var wt;
  try { wt = require("worker_threads"); } catch (e) {
    __draconic_dns_err("EADDR", "dns unavailable");
  }
  var sab = new SharedArrayBuffer(8 + 4096);
  var i32 = new Int32Array(sab);
  var u8 = new Uint8Array(sab, 8);
  var boot =
    "const { workerData } = require('worker_threads');\n" +
    "const dns = require('dns');\n" +
    "const i32 = new Int32Array(workerData.sab);\n" +
    "const u8 = new Uint8Array(workerData.sab, 8);\n" +
    "dns.lookup(workerData.host, { all: true, family: 4 }, function(err, addrs) {\n" +
    "  if (err || !addrs || !addrs.length) { Atomics.store(i32, 0, 2); Atomics.notify(i32, 0, 1); return; }\n" +
    "  var seen = Object.create(null);\n" +
    "  var out = [];\n" +
    "  for (var i = 0; i < addrs.length; i++) {\n" +
    "    var a = addrs[i] && addrs[i].address;\n" +
    "    if (!a || seen[a]) continue;\n" +
    "    seen[a] = 1;\n" +
    "    out.push(a);\n" +
    "  }\n" +
    "  if (!out.length) { Atomics.store(i32, 0, 2); Atomics.notify(i32, 0, 1); return; }\n" +
    "  var payload = JSON.stringify(out);\n" +
    "  for (var j = 0; j < payload.length && j < u8.length; j++) u8[j] = payload.charCodeAt(j) & 255;\n" +
    "  i32[1] = Math.min(payload.length, u8.length);\n" +
    "  Atomics.store(i32, 0, 1);\n" +
    "  Atomics.notify(i32, 0, 1);\n" +
    "});\n";
  var w = new wt.Worker(boot, { eval: true, workerData: { sab: sab, host: h } });
  Atomics.wait(i32, 0, 0);
  try { if (w && typeof w.terminate === "function") w.terminate(); } catch (e2) {}
  var st = Atomics.load(i32, 0);
  if (st !== 1) __draconic_dns_err("EADDR", "lookup failed");
  var n = i32[1] | 0;
  var s = "";
  for (var i = 0; i < n; i++) s += String.fromCharCode(u8[i]);
  var parsed = JSON.parse(s);
  if (!Array.isArray(parsed) || parsed.length === 0) __draconic_dns_err("EADDR", "lookup failed");
  return parsed;
}
if (typeof globalThis !== "undefined") globalThis.dnsLookup = dnsLookup;
"#
}

pub fn tcp_js_polyfill() -> &'static str {
    r#"(function () {
  function hostErr(code, msg) {
    var e = new Error(msg ? (code + ": " + msg) : code);
    e.name = "HostError";
    e.code = code;
    throw e;
  }
  var HEADER = 24;
  var PAYLOAD = 65536 + 2048;
  var sab = null;
  var i32 = null;
  var u8 = null;
  var worker = null;
  var CMD = {
    LISTEN: 1, LOCAL_PORT: 2, ACCEPT: 3, CONNECT: 4, PEER_ADDR: 5,
    PEER_PORT: 6, READ: 7, WRITE: 8, SHUTDOWN: 9, CLOSE: 10
  };
  function ensureWorker() {
    if (worker) return;
    var wt;
    try { wt = require("worker_threads"); } catch (e) {
      hostErr("ENOSYS", "worker_threads unavailable");
    }
    sab = new SharedArrayBuffer(HEADER + PAYLOAD);
    i32 = new Int32Array(sab);
    u8 = new Uint8Array(sab, HEADER);
    var boot =
      "const { workerData } = require('worker_threads');\n" +
      "const net = require('net');\n" +
      "const i32 = new Int32Array(workerData.sab);\n" +
      "const u8 = new Uint8Array(workerData.sab, 24);\n" +
      "var nextId = 1;\n" +
      "var slots = Object.create(null);\n" +
      "function writeStr(s) {\n" +
      "  s = String(s || '');\n" +
      "  var n = Math.min(s.length, u8.length);\n" +
      "  for (var i = 0; i < n; i++) u8[i] = s.charCodeAt(i) & 255;\n" +
      "  i32[4] = n;\n" +
      "}\n" +
      "function readPayload() {\n" +
      "  var n = i32[4] | 0;\n" +
      "  if (n < 0) n = 0;\n" +
      "  if (n > u8.length) n = u8.length;\n" +
      "  var buf = Buffer.alloc(n);\n" +
      "  for (var i = 0; i < n; i++) buf[i] = u8[i];\n" +
      "  return buf;\n" +
      "}\n" +
      "function finish(ok, result) {\n" +
      "  i32[5] = result | 0;\n" +
      "  Atomics.store(i32, 0, ok ? 1 : 2);\n" +
      "  Atomics.notify(i32, 0, 1);\n" +
      "}\n" +
      "function mapConnErr(err) {\n" +
      "  var c = err && err.code;\n" +
      "  if (c === 'ENOTFOUND' || c === 'EAI_AGAIN' || c === 'EAI_FAIL' || c === 'EAI_NODATA') return 'EADDR';\n" +
      "  return 'ECONN';\n" +
      "}\n" +
      "function allocListen(server, port) {\n" +
      "  var id = nextId++;\n" +
      "  slots[id] = { kind: 'listen', server: server, port: port, queue: [], wait: null };\n" +
      "  server.on('connection', function(sock) {\n" +
      "    var rec = slots[id];\n" +
      "    if (!rec) { try { sock.destroy(); } catch (e) {} return; }\n" +
      "    if (rec.wait) { var w = rec.wait; rec.wait = null; w(sock); }\n" +
      "    else rec.queue.push(sock);\n" +
      "  });\n" +
      "  return id;\n" +
      "}\n" +
      "function allocConn(sock) {\n" +
      "  var id = nextId++;\n" +
      "  var rec = { kind: 'conn', sock: sock, buf: Buffer.alloc(0), eof: false, readWait: null };\n" +
      "  sock.on('data', function(chunk) {\n" +
      "    rec.buf = Buffer.concat([rec.buf, chunk]);\n" +
      "    if (rec.readWait) { var w = rec.readWait; rec.readWait = null; w(); }\n" +
      "  });\n" +
      "  sock.on('end', function() {\n" +
      "    rec.eof = true;\n" +
      "    if (rec.readWait) { var w = rec.readWait; rec.readWait = null; w(); }\n" +
      "  });\n" +
      "  sock.on('error', function() {\n" +
      "    rec.eof = true;\n" +
      "    if (rec.readWait) { var w = rec.readWait; rec.readWait = null; w(); }\n" +
      "  });\n" +
      "  slots[id] = rec;\n" +
      "  return id;\n" +
      "}\n" +
      "function handle(cmd, done) {\n" +
      "  var a = i32[2] | 0;\n" +
      "  var b = i32[3] | 0;\n" +
      "  if (cmd === 1) {\n" +
      "    if (a < 0 || a > 65535) { writeStr('EINVAL'); done(false, 0); return; }\n" +
      "    var backlog = b > 0 ? b : 128;\n" +
      "    var server = net.createServer();\n" +
      "    var finished = false;\n" +
      "    server.once('error', function() {\n" +
      "      if (finished) return;\n" +
      "      finished = true;\n" +
      "      writeStr('EADDR');\n" +
      "      done(false, 0);\n" +
      "    });\n" +
      "    server.listen({ port: a, host: '127.0.0.1', backlog: backlog }, function() {\n" +
      "      if (finished) return;\n" +
      "      finished = true;\n" +
      "      var addr = server.address();\n" +
      "      var port = addr && addr.port ? addr.port : a;\n" +
      "      var id = allocListen(server, port);\n" +
      "      done(true, id);\n" +
      "    });\n" +
      "    return;\n" +
      "  }\n" +
      "  if (cmd === 2) {\n" +
      "    var rec = slots[a];\n" +
      "    if (!rec || rec.kind !== 'listen') { writeStr('EBADF'); done(false, 0); return; }\n" +
      "    done(true, rec.port | 0);\n" +
      "    return;\n" +
      "  }\n" +
      "  if (cmd === 3) {\n" +
      "    var recA = slots[a];\n" +
      "    if (!recA || recA.kind !== 'listen') { writeStr('EBADF'); done(false, 0); return; }\n" +
      "    if (recA.queue.length) { done(true, allocConn(recA.queue.shift())); return; }\n" +
      "    recA.wait = function(sock) { done(true, allocConn(sock)); };\n" +
      "    return;\n" +
      "  }\n" +
      "  if (cmd === 4) {\n" +
      "    if (a < 1 || a > 65535) { writeStr('EINVAL'); done(false, 0); return; }\n" +
      "    var host = readPayload().toString('utf8');\n" +
      "    if (!host) { writeStr('EINVAL'); done(false, 0); return; }\n" +
      "    var finishedC = false;\n" +
      "    var sock = net.connect({ host: host, port: a, family: 4 }, function() {\n" +
      "      if (finishedC) return;\n" +
      "      finishedC = true;\n" +
      "      done(true, allocConn(sock));\n" +
      "    });\n" +
      "    sock.once('error', function(err) {\n" +
      "      if (finishedC) return;\n" +
      "      finishedC = true;\n" +
      "      writeStr(mapConnErr(err));\n" +
      "      try { sock.destroy(); } catch (e) {}\n" +
      "      done(false, 0);\n" +
      "    });\n" +
      "    return;\n" +
      "  }\n" +
      "  if (cmd === 5) {\n" +
      "    var recP = slots[a];\n" +
      "    if (!recP || recP.kind !== 'conn') { writeStr('EBADF'); done(false, 0); return; }\n" +
      "    var addr = recP.sock && recP.sock.remoteAddress ? String(recP.sock.remoteAddress) : '';\n" +
      "    if (addr.indexOf('::ffff:') === 0) addr = addr.slice(7);\n" +
      "    writeStr(addr);\n" +
      "    done(true, 0);\n" +
      "    return;\n" +
      "  }\n" +
      "  if (cmd === 6) {\n" +
      "    var recPP = slots[a];\n" +
      "    if (!recPP || recPP.kind !== 'conn') { writeStr('EBADF'); done(false, 0); return; }\n" +
      "    done(true, (recPP.sock && recPP.sock.remotePort) | 0);\n" +
      "    return;\n" +
      "  }\n" +
      "  if (cmd === 7) {\n" +
      "    var recR = slots[a];\n" +
      "    if (!recR || recR.kind !== 'conn') { writeStr('EBADF'); done(false, 0); return; }\n" +
      "    var maxLen = b | 0;\n" +
      "    if (maxLen < 0) maxLen = 0;\n" +
      "    function take() {\n" +
      "      if (recR.buf.length > 0) {\n" +
      "        var n = Math.min(maxLen, recR.buf.length, u8.length);\n" +
      "        for (var i = 0; i < n; i++) u8[i] = recR.buf[i];\n" +
      "        recR.buf = recR.buf.slice(n);\n" +
      "        i32[4] = n;\n" +
      "        done(true, n);\n" +
      "        return true;\n" +
      "      }\n" +
      "      if (recR.eof) { i32[4] = 0; done(true, 0); return true; }\n" +
      "      return false;\n" +
      "    }\n" +
      "    if (!take()) recR.readWait = function() { take(); };\n" +
      "    return;\n" +
      "  }\n" +
      "  if (cmd === 8) {\n" +
      "    var recW = slots[a];\n" +
      "    if (!recW || recW.kind !== 'conn') { writeStr('EBADF'); done(false, 0); return; }\n" +
      "    var buf = readPayload();\n" +
      "    recW.sock.write(buf, function(err) {\n" +
      "      if (err) { writeStr('ECONN'); done(false, 0); return; }\n" +
      "      done(true, buf.length);\n" +
      "    });\n" +
      "    return;\n" +
      "  }\n" +
      "  if (cmd === 9) {\n" +
      "    var recS = slots[a];\n" +
      "    if (!recS || recS.kind !== 'conn') { writeStr('EBADF'); done(false, 0); return; }\n" +
      "    try {\n" +
      "      if (b === 0) recS.sock.pause();\n" +
      "      else recS.sock.end();\n" +
      "    } catch (eS) {}\n" +
      "    done(true, 0);\n" +
      "    return;\n" +
      "  }\n" +
      "  if (cmd === 10) {\n" +
      "    var recC = slots[a];\n" +
      "    if (!recC) { writeStr('EBADF'); done(false, 0); return; }\n" +
      "    try {\n" +
      "      if (recC.kind === 'listen') recC.server.close();\n" +
      "      else if (recC.sock) recC.sock.destroy();\n" +
      "    } catch (eC) {}\n" +
      "    delete slots[a];\n" +
      "    done(true, 0);\n" +
      "    return;\n" +
      "  }\n" +
      "  writeStr('EINVAL');\n" +
      "  done(false, 0);\n" +
      "}\n" +
      "function pump() {\n" +
      "  var st = Atomics.load(i32, 0);\n" +
      "  if (st === 3) {\n" +
      "    handle(i32[1] | 0, function(ok, result) {\n" +
      "      finish(ok, result);\n" +
      "      setImmediate(pump);\n" +
      "    });\n" +
      "    return;\n" +
      "  }\n" +
      "  Atomics.wait(i32, 0, st, 5);\n" +
      "  setImmediate(pump);\n" +
      "}\n" +
      "pump();\n";
    worker = new wt.Worker(boot, { eval: true, workerData: { sab: sab } });
    if (typeof worker.unref === "function") worker.unref();
  }
  function writeStr(s) {
    s = String(s || "");
    var n = Math.min(s.length, u8.length);
    for (var i = 0; i < n; i++) u8[i] = s.charCodeAt(i) & 255;
    i32[4] = n;
  }
  function readStr() {
    var n = i32[4] | 0;
    if (n < 0) n = 0;
    if (n > u8.length) n = u8.length;
    var s = "";
    for (var i = 0; i < n; i++) s += String.fromCharCode(u8[i]);
    return s;
  }
  function rpc(cmd, a, b, payload) {
    ensureWorker();
    i32[1] = cmd | 0;
    i32[2] = a | 0;
    i32[3] = b | 0;
    if (payload != null) {
      var buf;
      if (typeof payload === "string") {
        writeStr(payload);
      } else {
        buf = payload;
        var n = Math.min(buf.length, u8.length);
        for (var i = 0; i < n; i++) u8[i] = buf[i] & 255;
        i32[4] = n;
      }
    } else {
      i32[4] = 0;
    }
    Atomics.store(i32, 0, 3);
    Atomics.notify(i32, 0, 1);
    while (true) {
      var st = Atomics.load(i32, 0);
      if (st === 1 || st === 2) break;
      Atomics.wait(i32, 0, st);
    }
    var done = Atomics.load(i32, 0);
    var result = i32[5] | 0;
    var errCode = done === 2 ? readStr() : "";
    var outBytes = null;
    if (done === 1 && (cmd === CMD.READ || cmd === CMD.PEER_ADDR)) {
      var n2 = i32[4] | 0;
      outBytes = new Uint8Array(n2);
      for (var j = 0; j < n2; j++) outBytes[j] = u8[j];
    }
    Atomics.store(i32, 0, 0);
    Atomics.notify(i32, 0, 1);
    if (done === 2) hostErr(errCode || "EIO", "tcp");
    return { result: result, bytes: outBytes };
  }
  function tcpListen(port, backlog) {
    var r = rpc(CMD.LISTEN, port | 0, backlog == null ? 0 : (backlog | 0), null);
    return r.result;
  }
  function tcpLocalPort(h) {
    return rpc(CMD.LOCAL_PORT, h | 0, 0, null).result;
  }
  function tcpAccept(h) {
    return rpc(CMD.ACCEPT, h | 0, 0, null).result;
  }
  function tcpConnect(host, port) {
    return rpc(CMD.CONNECT, port | 0, 0, host == null ? "" : String(host)).result;
  }
  function tcpPeerAddress(h) {
    var r = rpc(CMD.PEER_ADDR, h | 0, 0, null);
    var b = r.bytes || new Uint8Array(0);
    var s = "";
    for (var i = 0; i < b.length; i++) s += String.fromCharCode(b[i]);
    return s;
  }
  function tcpPeerPort(h) {
    return rpc(CMD.PEER_PORT, h | 0, 0, null).result;
  }
  function tcpRead(h, maxLen) {
    return rpc(CMD.READ, h | 0, maxLen | 0, null).bytes || new Uint8Array(0);
  }
  function tcpWrite(h, data) {
    var bytes;
    if (data == null) bytes = new Uint8Array(0);
    else if (typeof data === "string") {
      bytes = new Uint8Array(data.length);
      for (var i = 0; i < data.length; i++) bytes[i] = data.charCodeAt(i) & 255;
    } else if (typeof Uint8Array !== "undefined" && data instanceof Uint8Array) {
      bytes = data;
    } else {
      var s = String(data);
      bytes = new Uint8Array(s.length);
      for (var j = 0; j < s.length; j++) bytes[j] = s.charCodeAt(j) & 255;
    }
    return rpc(CMD.WRITE, h | 0, 0, bytes).result;
  }
  function tcpShutdown(h, how) {
    rpc(CMD.SHUTDOWN, h | 0, how == null ? 1 : (how | 0), null);
  }
  function closeTcp(h) {
    rpc(CMD.CLOSE, h | 0, 0, null);
  }
  if (typeof globalThis !== "undefined") {
    globalThis.tcpListen = tcpListen;
    globalThis.tcpLocalPort = tcpLocalPort;
    globalThis.tcpAccept = tcpAccept;
    globalThis.tcpConnect = tcpConnect;
    globalThis.tcpPeerAddress = tcpPeerAddress;
    globalThis.tcpPeerPort = tcpPeerPort;
    globalThis.tcpRead = tcpRead;
    globalThis.tcpWrite = tcpWrite;
    globalThis.tcpShutdown = tcpShutdown;
    globalThis.closeTcp = closeTcp;
  }
})();
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_polyfill_defines_helpers() {
        let s = http_js_polyfill();
        assert!(s.contains("function httpParseRequest"));
        assert!(s.contains("function httpWriteResponse"));
        assert!(s.contains("function httpWriteRequest"));
        assert!(s.contains("function httpParseResponse"));
    }

    #[test]
    fn dns_polyfill_defines_lookup() {
        let s = dns_js_polyfill();
        assert!(s.contains("function dnsLookup"));
        assert!(s.contains("worker_threads"));
    }

    #[test]
    fn tcp_polyfill_defines_listen_accept() {
        let s = tcp_js_polyfill();
        assert!(s.contains("function tcpListen"));
        assert!(s.contains("function tcpAccept"));
        assert!(s.contains("function tcpConnect"));
        assert!(
            s.contains("require(\"worker_threads\")") || s.contains("require('worker_threads')")
        );
    }
}
