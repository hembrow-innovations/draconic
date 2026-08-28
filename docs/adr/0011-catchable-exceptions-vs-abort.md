# Catchable exceptions vs process abort

Language-level failures a Program can handle with `try`/`catch` are **catchable exceptions**. They do not abort the process. After `catch`, execution continues. Uncaught, they still terminate the Program with a non-zero exit — same class, just not handled.

**Catchable (R04.01):**

- User `throw` of a JS value (number, string, Error object, …)
- ECMA built-in errors constructed and thrown (`throw new TypeError(…)`, `throw new RangeError(…)`, …)
- Language operations that throw TypeError / RangeError / ReferenceError / etc. per ECMA-262

**Not catchable (R04.02):** process abort / panic. These never become JS values. `try`/`catch` does not run. After abort, the process is dead.

- `draconic_rt_abort`: canonical Runtime abort entry (stderr `draconic_rt: abort`, then libc `abort()`)
- Runtime internal invariant failure (GC root stack underflow / grow failure, helper `malloc` failure, and similar `abort()` sites in the Runtime)
- Resource-budget exhaustion (R01.02 / R01.03): fail-closed at the C ABI (`NULL` / exceeded flag), not a JS exception. If the Runtime cannot continue, abort.

Rejected: treating all native failures as abort; treating GC/OOM as catchable JS exceptions in v1.
