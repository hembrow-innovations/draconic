---
id: "dragons-audit"
title: "Dragons audit follow-up"
kind: sprint
status: closed
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T06:57:16Z"
---
# Dragons audit follow-up

## Grouping

Location: none. 2026-09-10 dragons-audit of the toolchain. Workspace tests were red. LLVM fingerprint adapters survived slice-756. Not a ROADMAP Loop mill. Not the platform sprint.

## Slices in

- [[slice-781-for-head-destructure]]: `for ([u] of …)` must compile on js. blocked_by: none
- [[slice-782-annex-b-native-honesty]]: three annex-b native false greens leave the suite. blocked_by: none
- [[slice-783-llvm-one-emitter]]: one Emitter, one SlotTy, one escape. Prefactor for 784. blocked_by: none
- [[slice-784-llvm-no-fingerprint]]: one IR walker, no `try_folded_walks`. blocked_by: [[slice-783-llvm-one-emitter]]
- [[slice-785-js-host-polyfill]]: JS emit uses `host_js_polyfill`. blocked_by: none
- [[slice-786-type-world-predicates]]: dual-world tests live on Type. blocked_by: none
- [[slice-787-frontend-cli-seam]]: parse, extract, REPL, test262 use Frontend. blocked_by: none

## Slices out

- ROADMAP language atoms except honesty on N08.16.35 / 37 / 38
- file-budget splits that only relocate classify/ok/eval
- inkwell / llvm-sys
- cargo test --workspace as this sprint's oracle
