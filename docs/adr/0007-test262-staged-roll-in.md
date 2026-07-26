# Test262 staged roll-in

Official Test262 is the external ECMA-262 bar, rolled in by stages—not day-one full suite. v1 is a harness + curated allowlist on the **js** target only; the suite is vendored at a pinned commit into gitignored `third_party/test262/` via `scripts/fetch-test262.mjs`. Failures produce a baseline report only; promote clusters to Roadmap rows after triage. Full-suite-from-day-one, native-in-v1, and auto-spawning Roadmap rows were rejected as too noisy before the harness and process exist.
