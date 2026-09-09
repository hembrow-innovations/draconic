/// JS polyfill for host file APIs (H04.01–H04.05).
///
/// Node `fs` bridge. Missing path → throw `Error` with `.code === "ENOENT"`
/// and `.name === "HostError"`. `exists` returns boolean (no throw).
pub fn fs_read_js_polyfill() -> &'static str {
    r#"function __draconic_host_fs_err(code, path, cause) {
  var msg = code + ": " + (cause && cause.message ? cause.message : "file error");
  if (path != null) msg += ", open '" + String(path) + "'";
  var e = new Error(msg);
  e.name = "HostError";
  e.code = code;
  if (cause && cause.code) e.code = String(cause.code);
  throw e;
}
function __draconic_host_fs_catch(p, err) {
  if (err && (err.code === "ENOENT" || err.code === "ENOTDIR")) {
    __draconic_host_fs_err("ENOENT", p, err);
  }
  if (err && err.code === "EEXIST") {
    __draconic_host_fs_err("EEXIST", p, err);
  }
  if (err && (err.code === "EACCES" || err.code === "EPERM")) {
    __draconic_host_fs_err("EPERM", p, err);
  }
  __draconic_host_fs_err("EIO", p, err);
}
function __draconic_permissions_allows(need) {
  var raw;
  try {
    if (typeof process === "undefined" || !process.env) return true;
    raw = process.env.DRACONIC_PERMISSIONS;
  } catch (e) {
    return true;
  }
  if (raw == null || raw === "") return true;
  var parts = String(raw).split(",");
  for (var i = 0; i < parts.length; i++) {
    if (parts[i].trim() === need) return true;
  }
  return false;
}
function readFileText(path) {
  var p = String(path);
  if (!__draconic_permissions_allows("fs-read")) {
    __draconic_host_fs_err("EPERM", p, { message: "permission denied (fs-read)" });
  }
  var fs = require("fs");
  try {
    return fs.readFileSync(p, "utf8");
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function readFileBytes(path) {
  var p = String(path);
  if (!__draconic_permissions_allows("fs-read")) {
    __draconic_host_fs_err("EPERM", p, { message: "permission denied (fs-read)" });
  }
  var fs = require("fs");
  try {
    var buf = fs.readFileSync(p);
    return new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function writeFileText(path, text) {
  var p = String(path);
  if (!__draconic_permissions_allows("fs-write")) {
    __draconic_host_fs_err("EPERM", p, { message: "permission denied (fs-write)" });
  }
  var fs = require("fs");
  try {
    fs.writeFileSync(p, text == null ? "" : String(text), "utf8");
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function appendFileText(path, text) {
  var p = String(path);
  if (!__draconic_permissions_allows("fs-write")) {
    __draconic_host_fs_err("EPERM", p, { message: "permission denied (fs-write)" });
  }
  var fs = require("fs");
  try {
    fs.appendFileSync(p, text == null ? "" : String(text), "utf8");
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function writeFileBytes(path, data) {
  var p = String(path);
  if (!__draconic_permissions_allows("fs-write")) {
    __draconic_host_fs_err("EPERM", p, { message: "permission denied (fs-write)" });
  }
  var fs = require("fs");
  try {
    var buf = Buffer.from(data == null ? [] : data);
    fs.writeFileSync(p, buf);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function appendFileBytes(path, data) {
  var p = String(path);
  if (!__draconic_permissions_allows("fs-write")) {
    __draconic_host_fs_err("EPERM", p, { message: "permission denied (fs-write)" });
  }
  var fs = require("fs");
  try {
    var buf = Buffer.from(data == null ? [] : data);
    fs.appendFileSync(p, buf);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function exists(path) {
  var p = String(path);
  if (!p) return false;
  var fs = require("fs");
  try {
    fs.accessSync(p);
    return true;
  } catch (err) {
    return false;
  }
}
function stat(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    var st = fs.statSync(p);
    return {
      size: st.size,
      isFile: st.isFile(),
      isDir: st.isDirectory(),
      mtime: st.mtimeMs != null ? st.mtimeMs : (+st.mtime)
    };
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function mkdir(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    fs.mkdirSync(p);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function mkdirAll(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    fs.mkdirSync(p, { recursive: true });
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function readdir(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    return fs.readdirSync(p);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function rmdir(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    fs.rmdirSync(p);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function removeFile(path) {
  var p = String(path);
  var fs = require("fs");
  try {
    fs.unlinkSync(p);
  } catch (err) {
    __draconic_host_fs_catch(p, err);
  }
}
function renameFile(from, to) {
  var a = String(from);
  var b = String(to);
  var fs = require("fs");
  try {
    fs.renameSync(a, b);
  } catch (err) {
    __draconic_host_fs_catch(a, err);
  }
}
function copyFile(from, to) {
  var a = String(from);
  var b = String(to);
  var fs = require("fs");
  try {
    fs.copyFileSync(a, b);
  } catch (err) {
    __draconic_host_fs_catch(a, err);
  }
}
if (typeof globalThis !== "undefined") {
  globalThis.readFileText = readFileText;
  globalThis.readFileBytes = readFileBytes;
  globalThis.writeFileText = writeFileText;
  globalThis.appendFileText = appendFileText;
  globalThis.writeFileBytes = writeFileBytes;
  globalThis.appendFileBytes = appendFileBytes;
  globalThis.exists = exists;
  globalThis.stat = stat;
  globalThis.mkdir = mkdir;
  globalThis.mkdirAll = mkdirAll;
  globalThis.readdir = readdir;
  globalThis.rmdir = rmdir;
  globalThis.removeFile = removeFile;
  globalThis.renameFile = renameFile;
  globalThis.copyFile = copyFile;
}
"#
}
