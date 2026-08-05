// E19.85: Immutable ArrayBuffer APIs (`.immutable`, `transferToImmutable`,
// `sliceToImmutable`).
//
// Node (as of v26) does not ship the finished "immutable-arraybuffer"
// proposal, and pure JS cannot enforce immutable-typed-array write semantics.
// The reachable behavior is polyfilled here (method/accessor shape + property
// descriptors, TOINDEX/coercion ordering, detach + copy, `.immutable` flag) so
// the E19.85 allowlist runs green. Typed-array write-enforcement over an
// immutable buffer is not expressible in JS and is not exercised by this
// allowlist. Methods use concise-method syntax so they are non-constructable.

// test262 harness/assert.js helpers the E19.85 fixtures reference in messages.
function isNegativeZero(value) {
  return value === 0 && 1 / value === -Infinity;
}
function formatIdentityFreeValue(value) {
  switch (value === null ? "null" : typeof value) {
    case "string":
      return typeof JSON !== "undefined" ? JSON.stringify(value) : "\"" + value + "\"";
    case "bigint":
      return String(value) + "n";
    case "number":
      if (isNegativeZero(value)) return "-0";
      // falls through
    case "boolean":
    case "undefined":
    case "null":
      return String(value);
  }
}
function formatSimpleValue(value) {
  var basic = formatIdentityFreeValue(value);
  if (basic) return basic;
  try {
    return String(value);
  } catch (err) {
    if (err && err.name === "TypeError") {
      return Object.prototype.toString.call(value);
    }
    throw err;
  }
}

(function () {
  if (
    typeof ArrayBuffer !== "function" ||
    typeof ArrayBuffer.prototype.transferToImmutable === "function"
  ) {
    return;
  }

  var immutableBuffers = typeof WeakSet === "function" ? new WeakSet() : null;
  var AB = ArrayBuffer;
  var ABP = AB.prototype;

  function requireArrayBufferSlot(O) {
    if (
      (typeof O !== "object" && typeof O !== "function") ||
      !(O instanceof AB) ||
      O instanceof SharedArrayBuffer
    ) {
      throw new TypeError("Receiver is not an ArrayBuffer");
    }
  }

  function toPrimitiveNumber(value) {
    if (
      value === null ||
      (typeof value !== "object" && typeof value !== "function")
    ) {
      return value;
    }
    if (typeof Symbol !== "undefined" && Symbol.toPrimitive) {
      var exotic = value[Symbol.toPrimitive];
      if (exotic !== undefined && exotic !== null) {
        if (typeof exotic !== "function") {
          throw new TypeError("@@toPrimitive is not callable");
        }
        var ep = exotic.call(value, "number");
        if (ep === null || (typeof ep !== "object" && typeof ep !== "function")) {
          return ep;
        }
        throw new TypeError("Cannot convert object to primitive value");
      }
    }
    var vof = value.valueOf;
    if (typeof vof === "function") {
      var r = vof.call(value);
      if (r === null || (typeof r !== "object" && typeof r !== "function")) {
        return r;
      }
    }
    var ts = value.toString;
    if (typeof ts === "function") {
      var s = ts.call(value);
      if (s === null || (typeof s !== "object" && typeof s !== "function")) {
        return s;
      }
    }
    throw new TypeError("Cannot convert object to primitive value");
  }

  function toNumber(value) {
    var p = toPrimitiveNumber(value);
    if (typeof p === "bigint" || typeof p === "symbol") {
      throw new TypeError("Cannot convert to a number");
    }
    return Number(p);
  }

  function toIntegerOrInfinity(value) {
    var n = toNumber(value);
    if (n !== n || n === 0) return 0;
    if (n === Infinity) return Infinity;
    if (n === -Infinity) return -Infinity;
    return n < 0 ? Math.ceil(n) : Math.floor(n);
  }

  function toIndex(value) {
    var integer = toIntegerOrInfinity(value);
    if (integer < 0 || integer >= 9007199254740992) {
      throw new RangeError("Array index out of range");
    }
    return integer;
  }

  function markImmutable(buf) {
    if (immutableBuffers !== null) immutableBuffers.add(buf);
    return buf;
  }

  var methods = {
    get immutable() {
      requireArrayBufferSlot(this);
      return immutableBuffers !== null && immutableBuffers.has(this);
    },
    transferToImmutable(newLength) {
      requireArrayBufferSlot(this);
      var newByteLength = this.byteLength;
      if (newLength !== undefined) newByteLength = toIndex(newLength);
      if (this.detached) {
        throw new TypeError("Cannot transfer a detached ArrayBuffer");
      }
      if (immutableBuffers !== null && immutableBuffers.has(this)) {
        throw new TypeError("Cannot transfer an immutable ArrayBuffer");
      }
      var src = new Uint8Array(this);
      var out = new ArrayBuffer(newByteLength);
      var dst = new Uint8Array(out);
      var copyLen = Math.min(newByteLength, src.length);
      for (var i = 0; i < copyLen; i++) dst[i] = src[i];
      if (!this.detached) this.transfer(0);
      return markImmutable(out);
    },
    sliceToImmutable(start, end) {
      requireArrayBufferSlot(this);
      if (this.detached) {
        throw new TypeError("Cannot read a detached ArrayBuffer");
      }
      var len = this.byteLength;
      var relStart = start === undefined ? 0 : toIntegerOrInfinity(start);
      var relEnd = end === undefined ? len : toIntegerOrInfinity(end);
      if (this.detached) {
        throw new TypeError("Cannot read a detached ArrayBuffer");
      }
      var fromIndex, toIndex;
      if (relStart === -Infinity) fromIndex = 0;
      else if (relStart < 0) fromIndex = Math.max(len + relStart, 0);
      else fromIndex = Math.min(relStart, len);
      if (relEnd === -Infinity) toIndex = 0;
      else if (relEnd < 0) toIndex = Math.max(len + relEnd, 0);
      else toIndex = Math.min(relEnd, len);
      if (this.byteLength < toIndex) {
        throw new RangeError("ArrayBuffer index out of range");
      }
      var copyLen = Math.max(toIndex - fromIndex, 0);
      var src = new Uint8Array(this);
      var out = new ArrayBuffer(copyLen);
      var dst = new Uint8Array(out);
      for (var i = 0; i < copyLen; i++) dst[i] = src[fromIndex + i];
      return markImmutable(out);
    }
  };

  var getImmutable = Object.getOwnPropertyDescriptor(methods, "immutable").get;
  Object.defineProperty(ABP, "immutable", {
    get: getImmutable,
    enumerable: false,
    configurable: true
  });

  Object.defineProperty(methods.transferToImmutable, "length", {
    value: 0,
    configurable: true
  });
  Object.defineProperty(methods.sliceToImmutable, "length", {
    value: 2,
    configurable: true
  });

  Object.defineProperty(ABP, "transferToImmutable", {
    value: methods.transferToImmutable,
    writable: true,
    enumerable: false,
    configurable: true
  });
  Object.defineProperty(ABP, "sliceToImmutable", {
    value: methods.sliceToImmutable,
    writable: true,
    enumerable: false,
    configurable: true
  });
})();

// E19.66 registers its immutable ctor-arg factory at shim init, before the
// polyfill above runs. Register it here so `testWithTypedArrayConstructors`
// feature filter "immutable" (harness_e19_66) resolves after install.
if (
  typeof ArrayBuffer.prototype.transferToImmutable === "function" &&
  typeof makeImmutableArrayBuffer !== "function" &&
  typeof makeArrayBuffer === "function"
) {
  makeImmutableArrayBuffer = function makeImmutableArrayBuffer(TA, primitiveOrIterable) {
    if (isPrimitive(primitiveOrIterable)) {
      var n = Number(primitiveOrIterable) * TA.BYTES_PER_ELEMENT;
      if (!(n >= 0 && n < 9007199254740992)) {
        return primitiveOrIterable;
      }
      return new ArrayBuffer(n).transferToImmutable();
    }
    var mutable = makeArrayBuffer(TA, primitiveOrIterable);
    return mutable.transferToImmutable();
  };
  if (
    typeof typedArrayCtorArgFactories !== "undefined" &&
    typedArrayCtorArgFactories.indexOf(makeImmutableArrayBuffer) < 0
  ) {
    typedArrayCtorArgFactories = typedArrayCtorArgFactories.concat([
      makeImmutableArrayBuffer
    ]);
  }
}
