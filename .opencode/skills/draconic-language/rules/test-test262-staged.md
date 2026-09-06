---
title: Test262 is staged js
impact: HIGH
impactDescription: Full-suite CI or native Test262 drowns the Loop
tags: [test, test262]
---

## Test262 is staged js

Official Test262 is the external ECMA-262 bar, rolled in by allowlist on the js target only. Missing suite skips; CI stays green.

**Incorrect:** failing CI on the full suite; running Test262 on native; auto-spawning Roadmap rows from every failure.

**Correct:** `tests/test262/allowlist.txt`. Vendor via `scripts/fetch-test262.mjs` into gitignored `third_party/test262/`. Failures are report-only until triage promotes a Roadmap row. Probe paths before expanding.

**Notes.** ADR-0007 / E19. Full allowlist run is opt-in (`DRACONIC_TEST262_FULL=1`). See `prod-full-ecma`.
