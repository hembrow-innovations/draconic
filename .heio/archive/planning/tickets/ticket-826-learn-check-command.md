---
id: "ticket-826-learn-check-command"
title: "Shipped Learn typed samples never show draconic check"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T11:22:00Z"
updated_at: "2026-09-12T11:58:00Z"
---

# Shipped Learn typed samples never show draconic check

## Signal

Closed. Website swarm shipped `draconic check` on typed Learn samples in from JavaScript, from systems, Dual worlds, and native types. `website/src/tests/learn-pages.test.ts` locks those command fences. Command fences stay non-`drac`. Remaining not-yet pages stay prose-only.

## Fit

Unknown until triage. In scope of `public-site.ia:learn-walkable`. Command fences are not `drac` fences. Do not add a failing `drac` fence for Checker errors.

## Notes

- Pages: `website/from-javascript.md`, `website/from-systems.md`, `website/dual-worlds.md`, `website/native-types.md`
- Lock: `website/src/tests/learn-pages.test.ts`
- Matching Reference lock: `website/src/tests/reference-hub-pages.test.ts`

## Parent

[[Public site — Contract]]
