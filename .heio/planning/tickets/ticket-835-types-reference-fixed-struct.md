---
id: "ticket-835-types-reference-fixed-struct"
title: "types Reference has no compiling fixed struct"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T13:35:00Z"
updated_at: "2026-09-12T13:35:00Z"
---

# types Reference has no compiling fixed struct

## Signal

Website swarm 2026-09-12. types.md names a fixed struct and says initialize with an object literal, then ships only JS-value, i32, and as fences. Learn native types already has compiling `type Point = { x: i32; y: i32 }`. Write-time lookup cannot copy a struct without leaving Reference.

## Fit

Unknown until triage. In scope of `public-site.ia:reference-walkable` and `public-site.fences:shipped-must-build`. Add a compiling fence, not a new page.

## Notes

- Page: `website/types.md`
- Learn sample: `website/native-types.md`
- Lock: `website/src/tests/reference-hub-pages.test.ts`
- Do not add fences on not-yet pages

## Parent

[[Public site — Contract]]
