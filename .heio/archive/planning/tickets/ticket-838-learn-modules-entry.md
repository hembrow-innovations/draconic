---
id: "ticket-838-learn-modules-entry"
title: "Learn modules never runs a two-file graph"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T14:48:00Z"
updated_at: "2026-09-12T14:50:00Z"
---

# Learn modules never runs a two-file graph

## Signal

Closed. Website swarm shipped `website/content/modules.md` with an Entry heading, a complete non-drac `main.drac` that imports `./greet.drac` and logs `greet("from the entry")`, plus `draconic check main.drac` and `draconic run main.drac`. The compiling greet fence stays one `drac` fence. Search for Entry hits `/modules#entry`.

## Fit

In scope of `public-site.ia:learn-walkable`. Keep the importer out of a `drac` fence so `public-site.fences:shipped-must-build` still holds. No contract edit.

## Notes

- Modules chapter: `website/content/modules.md`
- Lock: `website/src/tests/learn-pages.test.ts`
- Search: `website/src/tests/search.test.ts`
- Do not put a dangling relative import in a `drac` fence.

## Parent

[[Public site — Contract]]
