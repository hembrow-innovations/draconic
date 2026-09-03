---
id: "h09-dns"
title: "H09 DNS"
kind: task
status: completed
tags: []
created_at: "2026-09-02T22:18:20Z"
updated_at: "2026-09-03T05:16:34Z"
---

# H09 DNS

## Done

ROADMAP H09 is implemented test-first on native: a Program can look up a hostname to addresses (typed failure errors) and connect-by-name (H09.01 + H06.03); `host/net/dns` fixtures are green and H09 is `done`.

## Context

Roadmap ID **H09** (DNS). H09.01–H09.03 already land hostname lookup with failure errors, connect-by-name on top of H06.03, and js `dnsLookup` hard-error; this sitting unifies them as one honest DNS surface on native. Tests under `tests/conformance/host/net/dns`. Harness `tests/conformance/tests/host_dns.rs`. Mark H09 `done` only when those tests are green. Not H09.01, H09.02, H09.03, H06, H08, H10, or H17.04.

## Verify

`cargo test -p draconic-conformance --test host_dns` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H09 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H09), `tests/conformance/fixtures/host/net/dns`, `tests/conformance/tests/host_dns.rs`, `crates/draconic-backend-llvm/src/host_dns.rs`, `crates/draconic-runtime`, js/native DNS paths as needed for the parent surface

## Links

[[s-h09]] [[ticket-40-h09-dns]]
