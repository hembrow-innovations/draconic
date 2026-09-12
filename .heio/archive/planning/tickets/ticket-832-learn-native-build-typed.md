---
id: "ticket-832-learn-native-build-typed"
title: "Typed Learn samples never native-build"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T13:10:00Z"
updated_at: "2026-09-12T15:15:00Z"
---

# Typed Learn samples never native-build

## Signal

Closed. Dual worlds ships `draconic build --target native boundary.drac` plus `./boundary`. Native types ships `draconic build --target native width.drac` plus `./width`. Command fences only. No new `drac` fences.

## Fit

In scope of `public-site.ia:learn-walkable`. No contract edit.

## Notes

- Dual worlds: `website/content/dual-worlds.md`
- Native types: `website/content/native-types.md`
- Lock: `website/src/tests/learn-pages.test.ts`
- Same pattern as from systems `--target native`.

## Parent

[[Public site — Contract]]
