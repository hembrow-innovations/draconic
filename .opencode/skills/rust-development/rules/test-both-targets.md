---
title: Both targets when the row says both
impact: CRITICAL
impactDescription: Shipping one backend calls the feature done
tags: [test, targets, backends]
---

## Both targets when the row says both

When Roadmap **Targets** is `both`, JS and native paths must be tested or explicitly split into `js` / `native` child rows.

**Incorrect:** `targets: js` only, then marking a `both` row `done` because emit_js passed.

**Correct:**

```text
targets: js,native
js.exit: 0
native.exit: 0
```

Or split: `E03.05` js child, `E03.06` native child — complete only the claimed child.

**Notes.** Native-only / JS-only features must diagnostic on the other backend (`err-hard-error`). CLI changes also need `cargo build -p draconic-cli`. Load **draconic-loop**. See `test-conformance`.
