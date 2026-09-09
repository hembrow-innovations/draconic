//! L03.01 / L03.02 / L10.01 / L10.02: SHA-256 digest, OS CSPRNG bytes,
//! HMAC-SHA256, AES-256-GCM AEAD.

pub fn sha256_js_polyfill() -> &'static str {
    r#"function sha256(bytes) {
  if (bytes instanceof ArrayBuffer) bytes = new Uint8Array(bytes);
  if (!(bytes instanceof Uint8Array)) throw new TypeError("sha256 expects Uint8Array");
  var c = null;
  try { c = require("crypto"); } catch (e) {}
  if (c && typeof c.createHash === "function") {
    var h = c.createHash("sha256");
    h.update(Buffer.from(bytes));
    return new Uint8Array(h.digest());
  }
  throw new TypeError("sha256 unavailable");
}
if (typeof globalThis !== "undefined") globalThis.sha256 = sha256;
"#
}

pub fn random_bytes_js_polyfill() -> &'static str {
    r#"function randomBytes(n) {
  if (typeof n !== "number" || n !== n || n === Infinity || n === -Infinity) {
    throw new TypeError("randomBytes expects a length");
  }
  if (n < 0 || n !== Math.floor(n) || n > 65536) {
    throw new RangeError("randomBytes length must be a non-negative integer");
  }
  var out = new Uint8Array(n);
  if (n === 0) return out;
  var c = null;
  try { c = require("crypto"); } catch (e) {}
  if (c && typeof c.randomFillSync === "function") {
    c.randomFillSync(out);
    return out;
  }
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    crypto.getRandomValues(out);
    return out;
  }
  throw new TypeError("randomBytes unavailable");
}
if (typeof globalThis !== "undefined") globalThis.randomBytes = randomBytes;
"#
}

pub fn hmac_sha256_js_polyfill() -> &'static str {
    r#"function hmacSha256(key, message) {
  if (key instanceof ArrayBuffer) key = new Uint8Array(key);
  if (message instanceof ArrayBuffer) message = new Uint8Array(message);
  if (!(key instanceof Uint8Array) || !(message instanceof Uint8Array)) {
    throw new TypeError("hmacSha256 expects Uint8Array key and message");
  }
  var c = null;
  try { c = require("crypto"); } catch (e) {}
  if (c && typeof c.createHmac === "function") {
    var h = c.createHmac("sha256", Buffer.from(key));
    h.update(Buffer.from(message));
    return new Uint8Array(h.digest());
  }
  throw new TypeError("hmacSha256 unavailable");
}
if (typeof globalThis !== "undefined") globalThis.hmacSha256 = hmacSha256;
"#
}

pub fn aead_js_polyfill() -> &'static str {
    r#"function aeadEncrypt(key, nonce, plaintext) {
  if (key instanceof ArrayBuffer) key = new Uint8Array(key);
  if (nonce instanceof ArrayBuffer) nonce = new Uint8Array(nonce);
  if (plaintext instanceof ArrayBuffer) plaintext = new Uint8Array(plaintext);
  if (!(key instanceof Uint8Array) || !(nonce instanceof Uint8Array) || !(plaintext instanceof Uint8Array)) {
    throw new TypeError("aeadEncrypt expects Uint8Array key, nonce, and plaintext");
  }
  if (key.length !== 32) throw new RangeError("aeadEncrypt key must be 32 bytes");
  if (nonce.length !== 12) throw new RangeError("aeadEncrypt nonce must be 12 bytes");
  var c = null;
  try { c = require("crypto"); } catch (e) {}
  if (c && typeof c.createCipheriv === "function") {
    var cipher = c.createCipheriv("aes-256-gcm", Buffer.from(key), Buffer.from(nonce));
    var ct = Buffer.concat([cipher.update(Buffer.from(plaintext)), cipher.final()]);
    var tag = cipher.getAuthTag();
    return new Uint8Array(Buffer.concat([ct, tag]));
  }
  throw new TypeError("aeadEncrypt unavailable");
}
function aeadDecrypt(key, nonce, ciphertext) {
  if (key instanceof ArrayBuffer) key = new Uint8Array(key);
  if (nonce instanceof ArrayBuffer) nonce = new Uint8Array(nonce);
  if (ciphertext instanceof ArrayBuffer) ciphertext = new Uint8Array(ciphertext);
  if (!(key instanceof Uint8Array) || !(nonce instanceof Uint8Array) || !(ciphertext instanceof Uint8Array)) {
    throw new TypeError("aeadDecrypt expects Uint8Array key, nonce, and ciphertext");
  }
  if (key.length !== 32) throw new RangeError("aeadDecrypt key must be 32 bytes");
  if (nonce.length !== 12) throw new RangeError("aeadDecrypt nonce must be 12 bytes");
  if (ciphertext.length < 16) throw new RangeError("aeadDecrypt ciphertext too short");
  var c = null;
  try { c = require("crypto"); } catch (e) {}
  if (c && typeof c.createDecipheriv === "function") {
    var tag = ciphertext.subarray(ciphertext.length - 16);
    var body = ciphertext.subarray(0, ciphertext.length - 16);
    var decipher = c.createDecipheriv("aes-256-gcm", Buffer.from(key), Buffer.from(nonce));
    decipher.setAuthTag(Buffer.from(tag));
    try {
      var pt = Buffer.concat([decipher.update(Buffer.from(body)), decipher.final()]);
      return new Uint8Array(pt);
    } catch (e) {
      throw new Error("aeadDecrypt: authentication failed");
    }
  }
  throw new TypeError("aeadDecrypt unavailable");
}
if (typeof globalThis !== "undefined") {
  globalThis.aeadEncrypt = aeadEncrypt;
  globalThis.aeadDecrypt = aeadDecrypt;
}
"#
}

pub fn fill_random(buf: &mut [u8]) -> Result<(), ()> {
    if buf.is_empty() {
        return Ok(());
    }
    use std::io::Read;
    let mut f = std::fs::File::open("/dev/urandom").map_err(|_| ())?;
    f.read_exact(buf).map_err(|_| ())
}
