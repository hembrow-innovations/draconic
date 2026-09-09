/// JS polyfill for `processArgs()` (H01.01): user program args as string[].
///
/// Node bridge: `process.argv` without the executable; if `argv[1]` looks like a
/// script path (`.js`/`.mjs`/`.cjs`/`.drac`), skip it too (file run). Eval-style
/// (`node -e`) has no script slot — user args start at index 1.
pub fn process_args_js_polyfill() -> &'static str {
    r#"function processArgs() {
  var a = (typeof process !== "undefined" && process && process.argv) ? process.argv : [];
  if (!a || a.length <= 1) return [];
  var first = a[1];
  if (typeof first === "string" && /\.(m?js|cjs|drac)$/i.test(first)) {
    return a.slice(2).map(String);
  }
  return a.slice(1).map(String);
}
if (typeof globalThis !== "undefined") globalThis.processArgs = processArgs;
"#
}

/// JS polyfill for `envGet` / `envSet` / `envDelete` (H01.02).
///
/// Node bridge via `process.env`. Missing key → `undefined`. Values coerced to string.
pub fn process_env_js_polyfill() -> &'static str {
    r#"function envGet(key) {
  if (typeof process === "undefined" || !process || !process.env) return undefined;
  var v = process.env[String(key)];
  if (v === undefined || v === null) return undefined;
  return String(v);
}
function envSet(key, value) {
  if (typeof process === "undefined" || !process) return;
  if (!process.env) process.env = {};
  process.env[String(key)] = String(value);
}
function envDelete(key) {
  if (typeof process === "undefined" || !process || !process.env) return;
  delete process.env[String(key)];
}
if (typeof globalThis !== "undefined") {
  globalThis.envGet = envGet;
  globalThis.envSet = envSet;
  globalThis.envDelete = envDelete;
}
"#
}

/// JS polyfill for `exit` / `exitCode` / `setExitCode` (H01.03).
///
/// Node bridge via `process.exit` and `process.exitCode`. Bare `exit()` uses
/// the deferred code (default 0).
pub fn process_exit_js_polyfill() -> &'static str {
    r#"var __draconic_exitCode = 0;
function exitCode() {
  if (typeof process !== "undefined" && process && process.exitCode != null && process.exitCode !== undefined) {
    return Number(process.exitCode) | 0;
  }
  return __draconic_exitCode | 0;
}
function setExitCode(code) {
  var n = (code === undefined || code === null) ? 0 : (Number(code) | 0);
  __draconic_exitCode = n;
  if (typeof process !== "undefined" && process) process.exitCode = n;
}
function exit(code) {
  var n;
  if (arguments.length === 0 || code === undefined || code === null) {
    n = exitCode();
  } else {
    n = Number(code) | 0;
  }
  if (typeof process !== "undefined" && process && typeof process.exit === "function") {
    process.exit(n);
  }
  throw new Error("exit(" + n + ")");
}
if (typeof globalThis !== "undefined") {
  globalThis.exit = exit;
  globalThis.exitCode = exitCode;
  globalThis.setExitCode = setExitCode;
}
"#
}

/// JS polyfill for `pid` / `ppid` (H01.04).
///
/// Node bridge via `process.pid` and `process.ppid` (read-only numbers).
pub fn process_pid_js_polyfill() -> &'static str {
    r#"function pid() {
  if (typeof process !== "undefined" && process && process.pid != null) {
    return Number(process.pid) | 0;
  }
  return 0;
}
function ppid() {
  if (typeof process !== "undefined" && process && process.ppid != null) {
    return Number(process.ppid) | 0;
  }
  return 0;
}
if (typeof globalThis !== "undefined") {
  globalThis.pid = pid;
  globalThis.ppid = ppid;
}
"#
}

/// JS polyfill for `cwd` / `chdir` (H16.01).
///
/// Node bridge via `process.cwd` / `process.chdir`.
pub fn cwd_chdir_js_polyfill() -> &'static str {
    r#"function cwd() {
  if (typeof process !== "undefined" && process && typeof process.cwd === "function") {
    return process.cwd();
  }
  return "";
}
function chdir(path) {
  if (typeof process !== "undefined" && process && typeof process.chdir === "function") {
    process.chdir(String(path));
    return;
  }
  throw new Error("chdir unavailable");
}
if (typeof globalThis !== "undefined") {
  globalThis.cwd = cwd;
  globalThis.chdir = chdir;
}
"#
}

/// JS polyfill for `hostname` / `osType` / `osArch` (H16.02).
///
/// Node bridge via `os.hostname` / `os.platform` / `os.arch`.
pub fn hostname_os_js_polyfill() -> &'static str {
    r#"function hostname() {
  try {
    var os = require("os");
    if (os && typeof os.hostname === "function") return String(os.hostname());
  } catch (e) {}
  return "";
}
function osType() {
  try {
    var os = require("os");
    if (os && typeof os.platform === "function") return String(os.platform());
  } catch (e) {}
  return "";
}
function osArch() {
  try {
    var os = require("os");
    if (os && typeof os.arch === "function") return String(os.arch());
  } catch (e) {}
  return "";
}
if (typeof globalThis !== "undefined") {
  globalThis.hostname = hostname;
  globalThis.osType = osType;
  globalThis.osArch = osArch;
}
"#
}

/// JS polyfill for `tempDir` / `homeDir` (H16.03).
///
/// Node bridge via `os.tmpdir` / `os.homedir`.
pub fn temp_home_js_polyfill() -> &'static str {
    r#"function tempDir() {
  try {
    var os = require("os");
    if (os && typeof os.tmpdir === "function") return String(os.tmpdir());
  } catch (e) {}
  return "";
}
function homeDir() {
  try {
    var os = require("os");
    if (os && typeof os.homedir === "function") return String(os.homedir());
  } catch (e) {}
  return "";
}
if (typeof globalThis !== "undefined") {
  globalThis.tempDir = tempDir;
  globalThis.homeDir = homeDir;
}
"#
}

/// JS polyfill for `processRun` (H15.01).
///
/// Node bridge via `child_process.spawnSync`. argv[0] is the program; remaining
/// elements are args. Optional cwd (null/undefined → inherit). Optional env
/// object merges onto `process.env` (subset override). Returns exit status;
/// spawn failure → -1; killed by signal → 128.
pub fn process_run_js_polyfill() -> &'static str {
    r#"function processRun(argv, cwd, env) {
  var cp = require("child_process");
  var a = Array.isArray(argv) ? argv.map(function (x) { return String(x); }) : [];
  if (a.length < 1) return -1;
  var opts = { encoding: "utf8", stdio: ["ignore", "ignore", "ignore"] };
  if (cwd != null && cwd !== undefined) opts.cwd = String(cwd);
  if (env != null && env !== undefined && typeof env === "object") {
    var base = (typeof process !== "undefined" && process && process.env) ? process.env : {};
    var merged = {};
    for (var k in base) {
      if (Object.prototype.hasOwnProperty.call(base, k)) merged[k] = base[k];
    }
    for (var ek in env) {
      if (Object.prototype.hasOwnProperty.call(env, ek)) merged[ek] = String(env[ek]);
    }
    opts.env = merged;
  }
  var r = cp.spawnSync(a[0], a.slice(1), opts);
  if (!r || r.error) return -1;
  if (r.status != null && r.status !== undefined) return Number(r.status) | 0;
  if (r.signal) return 128;
  return -1;
}
if (typeof globalThis !== "undefined") {
  globalThis.processRun = processRun;
}
"#
}

/// JS polyfill for H15.02 process spawn + pipes (Node `spawnSync` deferred).
///
/// Handles are deferred until `processWait`: capture uses `spawnSync` with
/// `input`; kill runs a shell wrapper that spawns, SIGTERMs, and waits.
pub fn process_spawn_js_polyfill() -> &'static str {
    r#"(function () {
  var cp = require("child_process");
  var slots = Object.create(null);
  var nextId = 1;
  function mergeEnv(env) {
    var base = (typeof process !== "undefined" && process && process.env) ? process.env : {};
    var merged = {};
    for (var k in base) {
      if (Object.prototype.hasOwnProperty.call(base, k)) merged[k] = base[k];
    }
    if (env != null && env !== undefined && typeof env === "object") {
      for (var ek in env) {
        if (Object.prototype.hasOwnProperty.call(env, ek)) merged[ek] = String(env[ek]);
      }
    }
    return merged;
  }
  function shellQuote(s) {
    return "'" + String(s).replace(/'/g, "'\\''") + "'";
  }
  function processSpawn(argv, cwd, env) {
    var a = Array.isArray(argv) ? argv.map(function (x) { return String(x); }) : [];
    if (a.length < 1) return -1;
    var id = nextId++;
    slots[id] = {
      argv: a,
      cwd: cwd,
      env: env,
      stdin: null,
      stdinSet: false,
      kill: false,
      waited: false,
      exitCode: -1,
      stdout: "",
      stderr: ""
    };
    return id;
  }
  function processStdinWrite(h, text) {
    var s = slots[h | 0];
    if (!s || s.waited || s.stdinSet) return -1;
    s.stdin = text == null || text === undefined ? "" : String(text);
    s.stdinSet = true;
    return 0;
  }
  function processKill(h) {
    var s = slots[h | 0];
    if (!s || s.waited) return -1;
    s.kill = true;
    return 0;
  }
  function processWait(h) {
    var s = slots[h | 0];
    if (!s) return -1;
    if (s.waited) return s.exitCode;
    var opts = { encoding: "utf8", stdio: ["pipe", "pipe", "pipe"] };
    if (s.cwd != null && s.cwd !== undefined) opts.cwd = String(s.cwd);
    if (s.env != null && s.env !== undefined && typeof s.env === "object") {
      opts.env = mergeEnv(s.env);
    }
    var r;
    if (s.kill) {
      var parts = [];
      for (var i = 0; i < s.argv.length; i++) parts.push(shellQuote(s.argv[i]));
      var cmd = parts.join(" ") + " & pid=$!; kill -TERM $pid; wait $pid; exit $?";
      r = cp.spawnSync("/bin/sh", ["-c", cmd], opts);
    } else {
      opts.input = s.stdinSet ? s.stdin : "";
      r = cp.spawnSync(s.argv[0], s.argv.slice(1), opts);
    }
    if (!r || r.error) {
      s.exitCode = -1;
      s.stdout = "";
      s.stderr = "";
    } else {
      s.stdout = r.stdout == null ? "" : String(r.stdout);
      s.stderr = r.stderr == null ? "" : String(r.stderr);
      if (r.status != null && r.status !== undefined) s.exitCode = Number(r.status) | 0;
      else if (r.signal) s.exitCode = 128;
      else s.exitCode = -1;
    }
    s.waited = true;
    return s.exitCode;
  }
  function processStdout(h) {
    var s = slots[h | 0];
    if (!s || !s.waited) return "";
    return s.stdout == null ? "" : String(s.stdout);
  }
  function processStderr(h) {
    var s = slots[h | 0];
    if (!s || !s.waited) return "";
    return s.stderr == null ? "" : String(s.stderr);
  }
  function processClose(h) {
    var s = slots[h | 0];
    if (!s) return -1;
    delete slots[h | 0];
    return 0;
  }
  if (typeof globalThis !== "undefined") {
    globalThis.processSpawn = processSpawn;
    globalThis.processStdinWrite = processStdinWrite;
    globalThis.processWait = processWait;
    globalThis.processStdout = processStdout;
    globalThis.processStderr = processStderr;
    globalThis.processKill = processKill;
    globalThis.processClose = processClose;
  }
})();
"#
}

