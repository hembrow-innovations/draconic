---
id: "ticket-831-learn-packages-ship"
title: "Learn packages chapter is still not-yet"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T12:50:00Z"
updated_at: "2026-09-12T12:56:00Z"
---

# Learn packages chapter is still not-yet

## Signal

Closed. Website swarm shipped `website/packages.md` as a working Learn chapter: compiling named `export function greet` package-root fence, `draconic check index.drac`, `draconic get github.com/org/pkg@1.0.0` with `--url`, `draconic mod tidy`, and a non-drac module-path import. Hub tests and public-docs status list mark packages shipped. Reference packages stays not-yet. See [[ticket-820-reference-packages-get-tidy]].

## Fit

In scope of `public-site.ia:learn-walkable`. Shipped fences must build. A `from "github.com/org/pkg"` drac fence cannot resolve in the isolated site pipeline, so the consumer import stays a non-drac fence.

## Notes

- Learn page: `website/packages.md`
- Lock: `website/src/tests/learn-pages.test.ts`
- Pipeline: `tests/integration/tests/website_pipeline.rs`
- Consumer example: `examples/pkg-consumer`
- Do not invent a live git remote.
- Do not copy vault package notes onto the site.
- Do not add fences on remaining not-yet pages.

## Parent

[[Public site — Contract]]
