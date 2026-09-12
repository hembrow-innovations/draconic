---
id: "ticket-839-reference-host-io-process"
title: "Host I/O Reference omits process call shapes"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T14:50:00Z"
updated_at: "2026-09-12T14:50:00Z"
---

# Host I/O Reference omits process call shapes

## Signal

Website swarm 2026-09-12. Reference hub promises process. The Names heading only lists `stdoutWrite` and `stderrWrite`. Checker ships `processArgs()`, `envGet(key)`, `exit(code)`, and `stdinReadLine()` on both backends. Search cannot hit `processArgs` because it is not a heading. A writer with Reference open cannot look up argv, env, stdin, or exit.

## Fit

Unknown until triage. In scope of `public-site.ia:reference-walkable`. Add headings and one compiling `processArgs` fence. Do not dump TLS, WebSocket, or HTTP/2.

## Notes

- Reference: `website/content/reference-host-io.md`
- Registry: `crates/draconic-check/src/host_api/host_api_process.rs`
- Lock: `website/src/tests/reference-hub-pages.test.ts`
- Do not add fences on not-yet pages.

## Parent

[[Public site — Contract]]
