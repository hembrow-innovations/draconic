---
id: "ticket-820-reference-packages-get-tidy"
title: "Packages Reference does not show get or tidy usage"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T10:45:00Z"
updated_at: "2026-09-12T13:10:00Z"
---

# Packages Reference does not show get or tidy usage

## Signal

Closed. Website swarm shipped `website/reference-packages.md` as a working lookup: compiling named `export function greet` package-root fence, `draconic check index.drac`, `draconic get github.com/org/pkg@1.0.0` with `--url`, `draconic mod tidy`, and a non-drac module-path import. Hub tests, search heading hits, the Reference hub row, and the public-docs status list mark packages shipped. Remaining not-yet pages: none.

## Fit

In scope of `public-site.ia:reference-walkable`. Shipped fences must build. A `from "github.com/org/pkg"` drac fence cannot resolve in the isolated site pipeline, so the consumer import stays a non-drac fence.

## Notes

- CLI page: `website/cli.md`
- Packages working page: `website/reference-packages.md`
- Lock: `website/src/tests/reference-hub-pages.test.ts`
- Search: `website/src/tests/search.test.ts`
- Do not publish the vault API note as the site.

## Parent

[[Public site — Contract]]
