---
id: "ticket-836-learn-host-io-listen-fence"
title: "Learn host I/O never compiles a listen Program"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T13:35:00Z"
updated_at: "2026-09-12T13:35:00Z"
---

# Learn host I/O never compiles a listen Program

## Signal

Website swarm 2026-09-12. host-io.md ships stdout and filesystem fences with js-default run, then names `tcpListen` and `httpParseRequest` and links HTTP echo. A visitor can finish the chapter without compiling or native-building a listen Program. examples/http-echo already ships that loop.

## Fit

Unknown until triage. In scope of `public-site.ia:learn-walkable` and `public-site.fences:shipped-must-build`. Prefer a compiling fence and native command fences. No playground.

## Notes

- Learn page: `website/host-io.md`
- Lock: `website/src/tests/learn-pages.test.ts`
- Example: `examples/http-echo`
- Do not add fences on not-yet pages

## Parent

[[Public site — Contract]]
