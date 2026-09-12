---
id: "ticket-841-use-global-not-globalthis"
title: "Use global instead of globalThis for console bind"
kind: ticket
status: open
ticket_type: feature-request
tags: []
blocked_by: []
created_at: "2026-09-12T05:31:40Z"
updated_at: "2026-09-12T05:31:40Z"
---

# Use global instead of globalThis for console bind

## Signal

Conversation 2026-09-12. Request to change `let console = globalThis.console` to use `global` instead of `globalThis`.

## Fit

Unknown until triage. Does not fit current platform slices [[slice-811-llvm-walk-file-budget]], [[slice-813-host-name-catalog]], [[slice-815-llvm-walker-native-date-stdlib]], [[slice-817-frontend-check-source-script]].

## Notes

- Documented bind is `let console = globalThis.console` because free identifier `console` is unresolved. Same line in README, `examples/shebang/hello.drac`, `examples/fizzbuzz/main.drac`, `docs/guides/guides-toolchain.md`.
- Check installs builtin `globalThis` only (`crates/draconic-check/src/binder.rs`).
- ROADMAP E15.01 locks `globalThis` as the global object.
- Native `globalThis.console.log` lowering already shipped ([[ticket-773-native-console-log]], closed).
