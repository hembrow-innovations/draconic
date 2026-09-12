---
id: "ticket-835-types-reference-fixed-struct"
title: "types Reference has no compiling fixed struct"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T13:35:00Z"
updated_at: "2026-09-12T14:40:00Z"
---

# types Reference has no compiling fixed struct

## Signal

Closed. Website swarm shipped types Reference as Checker lookup: compiling object-type, union, generic, and fixed-struct fences beside the existing JS-value, i32, and as samples. Search finds Object types, Unions and intersections, Generics, and Fixed structs on `/types`.

## Fit

In scope of `public-site.ia:reference-walkable` and `public-site.fences:shipped-must-build`. Add a compiling fence, not a new page.

## Notes

- Page: `website/content/types.md`
- Learn sample: `website/content/native-types.md`
- Lock: `website/src/tests/reference-hub-pages.test.ts`
- Do not add fences on not-yet pages

## Parent

[[Public site — Contract]]
