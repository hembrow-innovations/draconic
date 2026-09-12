---
id: "ticket-836-learn-host-io-listen-fence"
title: "Learn host I/O never compiles a listen Program"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T13:35:00Z"
updated_at: "2026-09-12T13:50:00Z"
---

# Learn host I/O never compiles a listen Program

## Signal

Closed. Website swarm shipped a compiling `tcpListen` fence on `website/host-io.md`, plus `draconic check listen.drac` and native build commands. `website/src/tests/learn-pages.test.ts` locks three `drac` fences. Search for tcpListen hits `/host-io#tcplisten`. Remaining not-yet pages stay prose-only.

## Fit

Unknown until triage. In scope of `public-site.ia:learn-walkable` and `public-site.fences:shipped-must-build`. Prefer a compiling fence and native command fences. No playground.

## Notes

- Learn page: `website/host-io.md`
- Lock: `website/src/tests/learn-pages.test.ts`
- Example: `examples/http-echo`
- Do not add fences on not-yet pages

## Parent

[[Public site — Contract]]
