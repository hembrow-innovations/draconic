---
id: "ticket-839-reference-host-io-process"
title: "Host I/O Reference omits process call shapes"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T14:50:00Z"
updated_at: "2026-09-12T15:00:00Z"
---

# Host I/O Reference omits process call shapes

## Signal

Closed. Website swarm shipped `processArgs` headings and a compiling `args.drac` fence on Learn and Reference host I/O, with `envGet`, `envSet`, `envDelete`, `exit`, and `stdinReadLine` lookup. CLI run names leftover args as `processArgs()`. Search for processArgs hits `/host-io#processargs` and `/reference-host-io#processargs`.

## Fit

In scope of `public-site.ia:learn-walkable` and `public-site.ia:reference-walkable`. No contract edit.

## Notes

- Learn: `website/content/host-io.md`
- Reference: `website/content/reference-host-io.md`
- CLI: `website/content/cli.md`
- Locks: `website/src/tests/learn-pages.test.ts`, `website/src/tests/reference-hub-pages.test.ts`, `website/src/tests/search.test.ts`

## Parent

[[Public site — Contract]]
