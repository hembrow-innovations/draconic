/// JS polyfill for path helpers (H03.01–H03.03).
///
/// Pure string ops except `pathResolve` (uses cwd). POSIX-style `/` output;
/// input accepts `/` and `\`. Empty normalize/join → `"."`. Matches Node
/// `path.posix` for `/` inputs; `pathResolve` matches Node `path.resolve`.
pub fn path_js_polyfill() -> &'static str {
    r#"function pathNormalize(path) {
  var src = path == null ? "" : String(path);
  if (src.length === 0) return ".";
  function isSep(c) { return c === "/" || c === "\\"; }
  var isAbs = isSep(src.charAt(0));
  var trailing = isSep(src.charAt(src.length - 1));
  var segs = [];
  var i = 0;
  while (i < src.length) {
    while (i < src.length && isSep(src.charAt(i))) i++;
    if (i >= src.length) break;
    var start = i;
    while (i < src.length && !isSep(src.charAt(i))) i++;
    var seg = src.slice(start, i);
    if (seg === ".") continue;
    if (seg === "..") {
      if (segs.length > 0 && segs[segs.length - 1] !== "..") {
        segs.pop();
        continue;
      }
      if (!isAbs) segs.push("..");
      continue;
    }
    segs.push(seg);
  }
  var out = "";
  if (isAbs) out = "/";
  if (segs.length === 0) {
    if (!isAbs) out = ".";
  } else {
    out += segs.join("/");
    if (trailing) out += "/";
  }
  return out;
}
function pathJoin() {
  var parts = [];
  for (var i = 0; i < arguments.length; i++) {
    var p = arguments[i] == null ? "" : String(arguments[i]);
    if (p.length > 0) parts.push(p);
  }
  if (parts.length === 0) return ".";
  return pathNormalize(parts.join("/"));
}
function pathDirname(path) {
  var src = path == null ? "" : String(path);
  function isSep(c) { return c === "/" || c === "\\"; }
  if (src.length === 0) return ".";
  var end = src.length;
  while (end > 0 && isSep(src.charAt(end - 1))) end--;
  if (end === 0) return "/";
  var i = end;
  while (i > 0 && !isSep(src.charAt(i - 1))) i--;
  if (i === 0) return ".";
  var dend = i;
  while (dend > 0 && isSep(src.charAt(dend - 1))) dend--;
  if (dend === 0) return "/";
  return src.slice(0, dend).replace(/\\/g, "/");
}
function pathBasename(path) {
  var src = path == null ? "" : String(path);
  function isSep(c) { return c === "/" || c === "\\"; }
  if (src.length === 0) return "";
  var end = src.length;
  while (end > 0 && isSep(src.charAt(end - 1))) end--;
  if (end === 0) return "";
  var i = end;
  while (i > 0 && !isSep(src.charAt(i - 1))) i--;
  return src.slice(i, end).replace(/\\/g, "/");
}
function pathExtname(path) {
  var src = path == null ? "" : String(path);
  function isSep(c) { return c === "/" || c === "\\"; }
  if (src.length === 0) return "";
  var startDot = -1;
  var startPart = 0;
  var end = -1;
  var matchedSlash = true;
  var preDotState = 0;
  for (var i = src.length - 1; i >= 0; --i) {
    var c = src.charAt(i);
    if (isSep(c)) {
      if (!matchedSlash) {
        startPart = i + 1;
        break;
      }
      continue;
    }
    if (end === -1) {
      matchedSlash = false;
      end = i + 1;
    }
    if (c === ".") {
      if (startDot === -1) startDot = i;
      else if (preDotState !== 1) preDotState = 1;
    } else if (startDot !== -1) {
      preDotState = -1;
    }
  }
  if (startDot === -1 || end === -1 ||
      preDotState === 0 ||
      (preDotState === 1 && startDot === end - 1 && startDot === startPart + 1)) {
    return "";
  }
  return src.slice(startDot, end);
}
function pathIsAbsolute(path) {
  var src = path == null ? "" : String(path);
  if (src.length === 0) return false;
  var c = src.charAt(0);
  return c === "/" || c === "\\";
}
function pathResolve() {
  var resolvedPath = "";
  var resolvedAbsolute = false;
  for (var i = arguments.length - 1; i >= -1 && !resolvedAbsolute; i--) {
    var path;
    if (i >= 0) {
      path = arguments[i] == null ? "" : String(arguments[i]);
    } else if (typeof process !== "undefined" && process && typeof process.cwd === "function") {
      path = process.cwd();
    } else if (typeof cwd === "function") {
      path = String(cwd());
    } else {
      path = "/";
    }
    if (path.length === 0) continue;
    resolvedPath = path + "/" + resolvedPath;
    resolvedAbsolute = path.charAt(0) === "/" || path.charAt(0) === "\\";
  }
  resolvedPath = pathNormalize(resolvedPath);
  if (resolvedAbsolute) {
    if (resolvedPath.length > 1 && resolvedPath.charAt(resolvedPath.length - 1) === "/") {
      resolvedPath = resolvedPath.slice(0, -1);
    }
    if (resolvedPath.length === 0) return "/";
    if (resolvedPath.charAt(0) !== "/" && resolvedPath.charAt(0) !== "\\") {
      resolvedPath = "/" + resolvedPath;
    }
    return resolvedPath.replace(/\\/g, "/");
  }
  return resolvedPath.length > 0 ? resolvedPath : ".";
}
if (typeof globalThis !== "undefined") {
  globalThis.pathJoin = pathJoin;
  globalThis.pathNormalize = pathNormalize;
  globalThis.pathDirname = pathDirname;
  globalThis.pathBasename = pathBasename;
  globalThis.pathExtname = pathExtname;
  globalThis.pathIsAbsolute = pathIsAbsolute;
  globalThis.pathResolve = pathResolve;
}
"#
}
