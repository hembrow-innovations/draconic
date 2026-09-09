/// JS polyfill for `spawnWorker` (C01.01), `joinWorker` (C01.02),
/// and `terminateWorker` (C01.03). Optional second arg is a channel handle
/// (C02.04): the worker fn is called with that handle; queued values move
/// into the isolate and worker `channelSend` drains back on join.
///
/// Node `worker_threads`: eval bootstrap + SharedArrayBuffer handshake.
/// `unref` so the parent can exit without join. Join waits via `Atomics.wait`
/// and returns 0 on success or a negative code on invalid/already-joined handle.
/// Worker throw is stored as status 2 and surfaced as join result 1.
/// Terminate force-stops the worker thread; slot is not shared with the parent heap.
pub fn spawn_worker_js_polyfill() -> &'static str {
    r#"(function () {
  var nextId = 1;
  var slots = Object.create(null);
  function spawnWorker(entry, ch) {
    var fnSrc;
    if (typeof entry === "function") {
      fnSrc = "(" + Function.prototype.toString.call(entry) + ")";
    } else if (typeof entry === "string" && entry.length > 0) {
      fnSrc = "(function(){})";
    } else {
      return -1;
    }
    var sab = new SharedArrayBuffer(4);
    var ia = new Int32Array(sab);
    var channelId = 0;
    var chanInit = [];
    var port2 = null;
    var drainPort = null;
    if (arguments.length >= 2 && typeof ch === "number" && ch > 0) {
      var chans = globalThis.__draconicChannels;
      if (!chans || !chans.slots[ch]) return -1;
      try {
        var wt0 = require("worker_threads");
        var mc = new wt0.MessageChannel();
        drainPort = mc.port1;
        port2 = mc.port2;
      } catch (e0) {
        return -1;
      }
      channelId = ch;
      chanInit = chans.slots[ch].slice();
      chans.slots[ch].length = 0;
    }
    var chanBoot = "";
    if (channelId) {
      chanBoot =
        "var __q = (workerData.initial || []).slice();\n" +
        "var __port = workerData.port;\n" +
        "var __chid = workerData.channelId;\n" +
        "function channelSend(c, v) { if (c !== __chid) return -1; if (__port && typeof __port.postMessage === 'function') __port.postMessage(v); return 0; }\n" +
        "function channelRecv(c) { if (c !== __chid) return undefined; if (!__q.length) return undefined; return __q.shift(); }\n";
    }
    var callSrc = channelId ? (fnSrc + "(workerData.channelId);\n") : (fnSrc + "();\n");
    var bootstrap =
      "const { workerData } = require('worker_threads');\n" +
      chanBoot +
      "try {\n" +
      callSrc +
      "  Atomics.store(workerData.ia, 0, 1);\n" +
      "} catch (e) {\n" +
      "  Atomics.store(workerData.ia, 0, 2);\n" +
      "}\n" +
      "Atomics.notify(workerData.ia, 0, 1);\n";
    try {
      var wt = require("worker_threads");
      var opts = { eval: true, workerData: { ia: ia, channelId: channelId, initial: chanInit, port: port2 } };
      if (port2) opts.transferList = [port2];
      var w = new wt.Worker(bootstrap, opts);
      if (typeof w.unref === "function") w.unref();
      var id = nextId++;
      slots[id] = { ia: ia, worker: w, joined: false, drainPort: drainPort, channelId: channelId };
      return id;
    } catch (e) {
      return -1;
    }
  }
  function joinWorker(h) {
    var rec = slots[h];
    if (!rec || rec.joined) return -1;
    rec.joined = true;
    if (rec.worker && typeof rec.worker.ref === "function") rec.worker.ref();
    Atomics.wait(rec.ia, 0, 0);
    var st = Atomics.load(rec.ia, 0);
    if (rec.drainPort) {
      try {
        var rmp = require("worker_threads").receiveMessageOnPort;
        var chans2 = globalThis.__draconicChannels;
        var msg;
        while (typeof rmp === "function") {
          msg = rmp(rec.drainPort);
          if (!msg) break;
          if (chans2 && rec.channelId && chans2.slots[rec.channelId]) {
            chans2.slots[rec.channelId].push(msg.message);
          }
        }
      } catch (e1) {}
    }
    if (rec.worker && typeof rec.worker.unref === "function") rec.worker.unref();
    delete slots[h];
    if (st === 2) return 1;
    return 0;
  }
  function terminateWorker(h) {
    var rec = slots[h];
    if (!rec || rec.joined) return -1;
    rec.joined = true;
    try {
      if (rec.worker && typeof rec.worker.terminate === "function") rec.worker.terminate();
    } catch (e) {}
    delete slots[h];
    return 0;
  }
  if (typeof globalThis !== "undefined") {
    globalThis.spawnWorker = spawnWorker;
    globalThis.joinWorker = joinWorker;
    globalThis.terminateWorker = terminateWorker;
  }
})();
"#
}

/// JS polyfill for `makeChannel` / `channelSend` / `channelRecv` (C02.01–C02.03).
///
/// Same-isolate FIFO of numbers, strings, bools, and structured-cloned
/// plain objects. `makeChannel()` / `makeChannel(n<=0)` is unbounded;
/// `makeChannel(n)` with n > 0 bounds the buffer. Send on a full bounded
/// channel returns -2 without enqueueing. Shared object refs (cycles /
/// diamonds) and non-plain values are rejected. Send returns 0 on success
/// or -1 on invalid handle / reject.
pub fn channel_js_polyfill() -> &'static str {
    r#"(function () {
  var nextId = 1;
  var slots = Object.create(null);
  var caps = Object.create(null);
  var FAIL = {};
  function clonePlain(v, seen) {
    var t = typeof v;
    if (t === "number" || t === "string" || t === "boolean") return v;
    if (v === null || t !== "object") return FAIL;
    if (typeof Array.isArray === "function" && Array.isArray(v)) return FAIL;
    var i;
    for (i = 0; i < seen.length; i++) {
      if (seen[i] === v) return FAIL;
    }
    seen.push(v);
    var out = {};
    var keys = Object.keys(v);
    for (i = 0; i < keys.length; i++) {
      var c = clonePlain(v[keys[i]], seen);
      if (c === FAIL) return FAIL;
      out[keys[i]] = c;
    }
    return out;
  }
  function makeChannel(cap) {
    var id = nextId++;
    slots[id] = [];
    caps[id] = (typeof cap === "number" && cap > 0) ? cap : 0;
    return id;
  }
  function channelSend(ch, v) {
    var q = slots[ch];
    if (!q) return -1;
    var cap = caps[ch];
    if (cap > 0 && q.length >= cap) return -2;
    var t = typeof v;
    if (t === "number" || t === "string" || t === "boolean") {
      q.push(v);
      return 0;
    }
    var cloned = clonePlain(v, []);
    if (cloned === FAIL) return -1;
    q.push(cloned);
    return 0;
  }
  function channelRecv(ch) {
    var q = slots[ch];
    if (!q || q.length === 0) return undefined;
    return q.shift();
  }
  if (typeof globalThis !== "undefined") {
    globalThis.makeChannel = makeChannel;
    globalThis.channelSend = channelSend;
    globalThis.channelRecv = channelRecv;
    globalThis.__draconicChannels = { slots: slots, caps: caps };
  }
})();
"#
}

