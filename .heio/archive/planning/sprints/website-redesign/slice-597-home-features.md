---
id: "slice-597-home-features"
title: "Home features"
kind: slice
status: met
sprint: "website-redesign"
blocked_by: ["slice-596-home-hero-cta"]
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T13:46:20Z"
---

# Home features

## Why

After the pitch, the homepage must name the three things the language actually is, using CONTEXT.md terms, not marketing synonyms. JS backend is IR to ECMAScript. LLVM backend is IR through LLVM to a native binary. Dual worlds is JS values and native types in one Program at explicit boundaries. Invented phrases (transpiler, native backend as the product name, FFI-only) teach the wrong model. This grid is the value props under the hero, still not a markdown dump.

## Done

Home feature grid shows those three props: JS backend, LLVM native, Dual worlds at explicit boundaries. No invented glossary terms.

## Blocked by

`[[slice-596-home-hero-cta]]`: features sit under the hero landing, not instead of it.

## Non-goals

- **[[slice-596-home-hero-cta]]**: pitch and CTAs already owned
- **[[slice-598-markdown-loader]]**: teaching-page loader
- Playground
- Next.js
- Extra feature cards beyond the three glossary props
- `cargo test --workspace` as this slice's oracle

## Oracle checklist

- [x] O1: home feature grid names JS backend, LLVM native, and Dual worlds and does not invent terms
  CHECK: pnpm --dir website exec vitest run -t "home features"
  EXPECT: Test Files  1 passed
  EVIDENCE: `pnpm --dir website exec vitest run -t "home features"` → Test Files  1 passed | 6 skipped (7)

## Pool

Durable links to task ids. Never drop them.

- `[[task-616-home-features]]`

## See also

- [[location-589-public-site]]
- [[ticket-628-public-site-redesign]]
- [[0013-public-site-tanstack-start]]
- docs/specs/draconic/public-site/
- website/
- CONTEXT.md
