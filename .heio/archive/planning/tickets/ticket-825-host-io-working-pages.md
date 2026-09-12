---
id: "ticket-825-host-io-working-pages"
title: "Host I/O Learn and Reference never name callable APIs"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T11:22:00Z"
updated_at: "2026-09-12T11:35:00Z"
---

# Host I/O Learn and Reference never name callable APIs

## Signal

Closed. Website swarm shipped `website/host-io.md` and `website/reference-host-io.md`: compiling `stdoutWrite` fence, named `readFileText` / `tcpListen` / `httpParseRequest`, `draconic check`, and a GitHub HTTP echo link. Hub tests and public-docs status list mark both pages shipped. Remaining not-yet pages stay prose-only.

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
