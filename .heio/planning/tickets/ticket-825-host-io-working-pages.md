---
id: "ticket-825-host-io-working-pages"
title: "Host I/O Learn and Reference never name callable APIs"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T11:22:00Z"
updated_at: "2026-09-12T11:22:00Z"
---

# Host I/O Learn and Reference never name callable APIs

## Signal

Website swarm 2026-09-12. After Dual worlds, modules, and native types, a visitor cannot look up `stdoutWrite`, `readFileText`, `tcpListen`, or `httpParseRequest` on the public site. `website/host-io.md` and `website/reference-host-io.md` are not-yet prose. ROADMAP H00 through H17 is done. Portable conformance and `examples/http-echo` already compile.

## Fit

Unknown until triage. In scope of `public-site.ia:learn-walkable` and `public-site.ia:reference-walkable`. Shipping requires status shipped plus compiling fences. Not-yet pages must stay without fences.

## Notes

- Learn chapter: `website/host-io.md`
- Reference working page: `website/reference-host-io.md`
- Locks: `website/src/tests/learn-pages.test.ts`, `website/src/tests/reference-hub-pages.test.ts`
- Do not publish the vault as the site.
- Verify a listen sample on `--target js` before putting `tcpListen` in a `drac` fence.

## Parent

[[Public site — Contract]]
