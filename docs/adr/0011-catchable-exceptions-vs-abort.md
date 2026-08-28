# Catchable exceptions vs process abort

Language-level failures a Program can handle with `try`/`catch` are **catchable exceptions**. They do not abort the process. After `catch`, execution continues. Uncaught, they still terminate the Program with a non-zero exit — same class, just not handled.

**Catchable (R04.01):**

- User `throw` of a JS value (number, string, Error object, …)
- ECMA built-in errors constructed and thrown (`throw new TypeError(…)`, `throw new RangeError(…)`, …)
- Language operations that throw TypeError / RangeError / ReferenceError / etc. per ECMA-262

**Not catchable (R04.02):** process abort / panic — resource-budget exhaustion, Runtime internal invariant failure, and similar fail-closed native faults.

Rejected: treating all native failures as abort; treating GC/OOM as catchable JS exceptions in v1.
