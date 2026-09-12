---
id: "ticket-840-home-native-sample"
title: "Home never shows the native half of the pitch"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T14:50:00Z"
updated_at: "2026-09-12T14:50:00Z"
---

# Home never shows the native half of the pitch

## Signal

Website swarm 2026-09-12. HomeHero promises native types and LLVM. HomeSample only shows untyped `hello.drac` and TypeScript-style `greet.drac`. There is no `i32`, no Dual-world `as`, and no `--target native`. HomeFeatures claims LLVM as dead text with no doorway. Ticket-830 added greet and closed.

## Fit

Unknown until triage. In scope of `public-site.home:landing`. Keep the sample static. No playground. No CodeFence on home.

## Notes

- Home sample: `website/src/features/home/`
- Lock: `website/src/tests/home-sample.test.ts`
- Reuse the shipped native-types `width.drac` fence as a static listing.
- Doorway: `/native-types`

## Parent

[[Public site — Contract]]
