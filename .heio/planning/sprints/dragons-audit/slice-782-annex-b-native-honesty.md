---
id: "slice-782-annex-b-native-honesty"
title: "Annex B native false greens leave the suite"
kind: slice
status: active
sprint: "dragons-audit"
blocked_by: []
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T12:45:00Z"
---

# Annex B native false greens leave the suite

## Why

Three annex-b fixtures declare native and fail with unsupported IR while ROADMAP N08.16.35 / 37 / 38 are done. Completeness cannot stay greener than the suite. Real native lowering is [[slice-784-llvm-no-fingerprint]].

## Done

`cargo test -p draconic-conformance --test annex_b` is green. Those three fixtures are js-only until the walker restores native. N08.16.35, N08.16.37, N08.16.38 are not `done`.

## Blocked by

None.

## Non-goals

- **implementing native lowering for async methods / private methods / static private fields** (that is [[slice-784-llvm-no-fingerprint]])
- **new fingerprint adapters**
- **marking E18 parents done**

## Oracle checklist

- [x] O1: annex_b suite green
  CHECK: cargo test -p draconic-conformance --test annex_b --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 104 passed; 0 failed; finished in 14.46s
- [x] O2: those three ROADMAP rows are not done
  CHECK: python3 -c 'import re,pathlib; t=pathlib.Path("ROADMAP.md").read_text(); ids=["N08.16.35","N08.16.37","N08.16.38"]; print("honest" if all(re.search(r"^\| %s \| (?!done )"%i,t,re.M) for i in ids) else "still-done")'
  EXPECT: honest
  EVIDENCE: honest

## Pool

- [[task-789-annex-b-honesty]]

## See also

[[ticket-775-annex-b-native-false-green]], [[slice-784-llvm-no-fingerprint]], ROADMAP.md, tests/conformance/fixtures/es/annex-b/
