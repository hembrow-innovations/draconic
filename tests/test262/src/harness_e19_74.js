
// E19.74: Promise.allKeyed / Promise.allSettledKeyed (await-dictionary)
(function () {
  if (typeof Promise !== "function") {
    return;
  }
  if (typeof Promise.allKeyed === "function" && typeof Promise.allSettledKeyed === "function") {
    return;
  }

  function __draconicIsObject(v) {
    return (typeof v === "object" && v !== null) || typeof v === "function";
  }

  function __draconicNewPromiseCapability(C) {
    if (typeof C !== "function") {
      throw new TypeError("NewPromiseCapability: constructor is not a function");
    }
    var resolve = undefined;
    var reject = undefined;
    var executorHolder = {
      executor(res, rej) {
        if (resolve !== undefined || reject !== undefined) {
          throw new TypeError("NewPromiseCapability: executor already called");
        }
        resolve = res;
        reject = rej;
      }
    };
    var executor = executorHolder.executor;
    Object.defineProperty(executor, "length", {
      value: 2,
      writable: false,
      enumerable: false,
      configurable: true
    });
    var promise = new C(executor);
    if (typeof resolve !== "function" || typeof reject !== "function") {
      throw new TypeError("NewPromiseCapability: resolve or reject not callable");
    }
    return { promise: promise, resolve: resolve, reject: reject };
  }

  function __draconicGetPromiseResolve(C) {
    var promiseResolve = C.resolve;
    if (typeof promiseResolve !== "function") {
      throw new TypeError("GetPromiseResolve: resolve is not callable");
    }
    return promiseResolve;
  }

  function __draconicCreateKeyedResult(keys, values) {
    var obj = Object.create(null);
    var i = 0;
    while (i < keys.length) {
      obj[keys[i]] = values[i];
      i = i + 1;
    }
    return obj;
  }

  function __draconicMakeElementFn(steps) {
    var holder = {
      fn(x) {
        return steps(x);
      }
    };
    var fn = holder.fn;
    Object.defineProperty(fn, "length", {
      value: 1,
      writable: false,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(fn, "name", {
      value: "",
      writable: false,
      enumerable: false,
      configurable: true
    });
    return fn;
  }

  function __draconicPerformPromiseAllKeyed(variant, promises, C, capability, promiseResolve) {
    var allKeys = Reflect.ownKeys(promises);
    var keys = [];
    var values = [];
    var remaining = { value: 1 };
    var resultResolved = { value: false };

    function tryResolveResult() {
      if (resultResolved.value) {
        return;
      }
      if (remaining.value !== 0) {
        return;
      }
      resultResolved.value = true;
      var result = __draconicCreateKeyedResult(keys, values);
      return capability.resolve(result);
    }

    function attachKeyHandlers(slot, nextPromise) {
      var alreadyCalled = { value: false };
      var onFulfilled = __draconicMakeElementFn(function (x) {
        if (alreadyCalled.value) {
          return;
        }
        alreadyCalled.value = true;
        if (variant === "all-settled") {
          values[slot] = { status: "fulfilled", value: x };
        } else {
          values[slot] = x;
        }
        remaining.value = remaining.value - 1;
        tryResolveResult();
      });
      var onRejected;
      if (variant === "all-settled") {
        onRejected = __draconicMakeElementFn(function (r) {
          if (alreadyCalled.value) {
            return;
          }
          alreadyCalled.value = true;
          values[slot] = { status: "rejected", reason: r };
          remaining.value = remaining.value - 1;
          tryResolveResult();
        });
      } else {
        onRejected = capability.reject;
      }
      nextPromise.then(onFulfilled, onRejected);
    }

    var keyIndex = 0;
    while (keyIndex < allKeys.length) {
      var key = allKeys[keyIndex];
      keyIndex = keyIndex + 1;
      var desc = Object.getOwnPropertyDescriptor(promises, key);
      if (desc === undefined || desc.enumerable !== true) {
        continue;
      }
      var index = keys.length;
      keys[index] = key;
      values[index] = undefined;
      remaining.value = remaining.value + 1;

      var nextValue = promises[key];
      var nextPromise = promiseResolve.call(C, nextValue);
      attachKeyHandlers(index, nextPromise);
    }

    remaining.value = remaining.value - 1;
    tryResolveResult();
  }

  function __draconicPromiseAllKeyedGeneric(variant, promises) {
    var C = this;
    var capability = __draconicNewPromiseCapability(C);
    try {
      var promiseResolve = __draconicGetPromiseResolve(C);
      if (!__draconicIsObject(promises)) {
        capability.reject(new TypeError("Promise.allKeyed: argument is not an object"));
        return capability.promise;
      }
      __draconicPerformPromiseAllKeyed(variant, promises, C, capability, promiseResolve);
    } catch (e) {
      try {
        capability.reject(e);
      } catch (_e2) {}
    }
    return capability.promise;
  }

  if (typeof Promise.allKeyed !== "function") {
    var allKeyedHolder = {
      allKeyed(promises) {
        return __draconicPromiseAllKeyedGeneric.call(this, "all", promises);
      }
    };
    var allKeyed = allKeyedHolder.allKeyed;
    Object.defineProperty(allKeyed, "length", {
      value: 1,
      writable: false,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(allKeyed, "name", {
      value: "allKeyed",
      writable: false,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(Promise, "allKeyed", {
      value: allKeyed,
      writable: true,
      enumerable: false,
      configurable: true
    });
  }

  if (typeof Promise.allSettledKeyed !== "function") {
    var allSettledKeyedHolder = {
      allSettledKeyed(promises) {
        return __draconicPromiseAllKeyedGeneric.call(this, "all-settled", promises);
      }
    };
    var allSettledKeyed = allSettledKeyedHolder.allSettledKeyed;
    Object.defineProperty(allSettledKeyed, "length", {
      value: 1,
      writable: false,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(allSettledKeyed, "name", {
      value: "allSettledKeyed",
      writable: false,
      enumerable: false,
      configurable: true
    });
    Object.defineProperty(Promise, "allSettledKeyed", {
      value: allSettledKeyed,
      writable: true,
      enumerable: false,
      configurable: true
    });
  }
})();
