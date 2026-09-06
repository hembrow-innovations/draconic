---
title: Destination is all of ECMA-262
impact: MEDIUM
impactDescription: Permanent omissions of eval or with are incomplete supersets
tags: [product, ecma]
---

## Destination is all of ECMA-262

The language destination is literally all of ECMA-262, including `eval`, `new Function`, and `with`. Native Embed compiles those strings at run time.

**Incorrect:** "we will never do `with`" as a product rule, or JS-backend-only eval as the end state.

**Correct:** cluster the work under E-rows (E16 eval, E17 with, …). Native Embed grows with N07 fixtures. Test262 is the external bar, staged.

**Notes.** ADR-0004. Split large clusters into child IDs rather than silently subsetting. See `pipe-embed-source` and `test-test262-staged`.
