---
id: "ticket-824-learn-modules-ship"
title: "Learn modules chapter is a prose stub"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T11:18:00Z"
updated_at: "2026-09-12T11:22:00Z"
---

# Learn modules chapter is a prose stub

## Signal

Closed. Website swarm shipped `website/modules.md` as a working Learn chapter: compiling named `export function greet` fence, `draconic check`, and a relative `./greet.drac` import in prose. Hub tests and public-docs status list mark modules shipped. Remaining not-yet pages stay prose-only.

## Fit

In scope of `public-site.ia:learn-walkable`. Shipped fences must build. Not-yet fence forbid still holds on remaining not-yet chapters.

## Notes

- Modules chapter: `website/modules.md`
- Lock: `website/src/tests/learn-pages.test.ts`
- Pipeline: `tests/integration/tests/website_pipeline.rs`
- Do not add fences on remaining not-yet pages.
- Do not put a dangling relative import in a `drac` fence.

## Parent

[[Public site — Contract]]
