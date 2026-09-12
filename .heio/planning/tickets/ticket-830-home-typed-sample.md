---
id: "ticket-830-home-typed-sample"
title: "Home sample is untyped JavaScript"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T12:50:00Z"
updated_at: "2026-09-12T12:50:00Z"
---

# Home sample is untyped JavaScript

## Signal

Website swarm 2026-09-12. The landing pitch claims TypeScript-inspired types and native systems types. The only Program on `/` is untyped `hello.drac`, locked to match Install. A visitor never sees a type, a `draconic` command, or a doorway to the shipped types page.

## Fit

Unknown until triage. In scope of `public-site.home:landing`. Keep Install hello as the first-run file. A second static sample can reuse the shipped types `greet.drac` fence. No playground and no CodeFence copy control on home.

## Notes

- Sample: `website/src/features/home/HomeSample/HomeSample.tsx`
- Lock: `website/src/tests/home-sample.test.ts`
- Do not change `website/install.md`.
- Do not dump Learn copy onto `/`.

## Parent

[[Public site — Contract]]
