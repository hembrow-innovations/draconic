---
id: "h17-04-optional-js-node-bridge-for"
title: "H17.04 Optional JS/Node bridge for subset host APIs (after native green)"
kind: task
status: completed
tags: []
created_at: "2026-09-02T22:23:12Z"
updated_at: "2026-09-03T05:16:34Z"
---

# H17.04 Optional JS/Node bridge for subset host APIs (after native green)

## Done

ROADMAP H17.04 is implemented test-first on js: a designed subset of host APIs that previously hard-errored on js now run via an explicit Node bridge, host APIs outside that subset still hard-error (no silent polyfill), `host/policy` fixtures are green, and H17.04 is `done`.

## Context

Roadmap ID **H17.04** (Optional JS/Node bridge for subset host APIs (after native green)). H17.01–H17.03 already land native http-echo and todo serve; H06.06 / H09.03 / H10.07 currently hard-error TCP listen/accept, `dnsLookup`, and HTTP listen helpers on js until this explicit bridge row. This sitting lands the designed js Node-bridge subset (ADR-0008: JS hard-error or host polyfill per row, no silent polyfill). Tests under `tests/conformance` fixtures `host/policy`. Harness `tests/conformance/tests/host_policy.rs`. Mark H17.04 `done` only when those tests are green. Not H17.01, H17.02, H17.03, H17 parent remainder, H00, H06.06, H09.03, H10.07, P04, full Node-shaped `http` / `net` / `dgram` modules, or TLS / HTTP/2 / WebSocket on js.

## Verify

`cargo test -p draconic-conformance --test host_policy` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H17.04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H17.04), `tests/conformance/fixtures/host/policy`, `tests/conformance/tests/host_policy.rs`, `crates/draconic-check/src/host_api.rs`, js host-bridge paths as needed for the designed subset

## Links

[[s-h17-04]] [[ticket-49-h17-04-optional-js-node-bridge-for]]
