---
id: "ticket-840-home-native-sample"
title: "Home never shows the native half of the pitch"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T14:50:00Z"
updated_at: "2026-09-12T15:10:00Z"
---

# Home never shows the native half of the pitch

## Signal

Closed. Home shows the shipped native-types `width.drac` listing with `i32`, `i64`, Dual-world `as`, `draconic build --target native`, and a native-types doorway. Static sample only. No playground. No CodeFence on home.

## Fit

In scope of `public-site.home:landing`. No contract edit.

## Notes

- Home sample: `website/src/features/home/`
- Lock: `website/src/tests/home-sample.test.ts`
- Reuse the shipped native-types `width.drac` fence as a static listing.
- Doorway: `/native-types`

## Parent

[[Public site — Contract]]
