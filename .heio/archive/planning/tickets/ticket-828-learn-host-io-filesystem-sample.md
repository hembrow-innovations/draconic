---
id: "ticket-828-learn-host-io-filesystem-sample"
title: "Learn host I/O names filesystem APIs with no compiling sample"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T11:42:00Z"
updated_at: "2026-09-12T11:53:00Z"
---

# Learn host I/O names filesystem APIs with no compiling sample

## Signal

Closed. Website swarm shipped a compiling write-then-read `writeFileText`/`readFileText` fence on `website/host-io.md`, plus `draconic check note.drac`. `website/src/tests/learn-pages.test.ts` locks two `drac` fences. Remaining not-yet pages stay prose-only.

## Fit

Unknown until triage. In scope of `public-site.ia:learn-walkable` and `public-site.fences:shipped-must-build`. A new sample must be a compiling `drac` fence. Command fences are not `drac` fences.

## Notes

- Page: `website/host-io.md`
- Lock: `website/src/tests/learn-pages.test.ts`
- Do not add fences on remaining not-yet pages.

## Parent

[[Public site — Contract]]
