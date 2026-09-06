---
id: "adr-13"
title: "ADR-0013: Public site is a TanStack Start app"
kind: adr
description: "Public Learn and Reference is a TanStack Start app rooted at website/, not a Draconic Program HTML dump."
status: accepted
domain: draconic
area: decisions
tags: [adr, public-site]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# ADR-0013: Public site is a TanStack Start app

## Context

The current public site is `website/` markdown fed through `website/generate.drac` ([[0010-public-docs-draconic-ssg]]). It is a thin HTML dump. The product owner wants a complete redesign at TypeScript.org quality: a language homepage, handbook-style Learn, and working Reference.

## Decision

Public Learn and Reference is a TanStack Start and TanStack Router app (pnpm, TypeScript only) rooted at `website/`. Markdown sources with `title`, `section`, and `status` stay the teaching source. GitHub Pages still hosts the built site. Keep shipped and not-yet. Copy-paste fences exist only on shipped pages and they must build. Playground and in-page runners stay later. Agent and toolchain notes stay in `docs/`.

This supersedes [[0010-public-docs-draconic-ssg]] on presentation only (Draconic Program SSG as the HTML renderer). It does not supersede the vault-versus-site split, the Learn and Reference IA, fence compile, or playground-later.

## Alternatives considered

- **mdBook**: A linear book, not a language homepage plus Learn and Reference.
- **Starlight**: A Node docs theme, not the chosen app.
- **VitePress**: A Node docs generator as the HTML renderer.
- **Next.js**: A different app stack; not chosen.
- **Serving the vault**: Agent notes in `docs/` are not the public site.
- **Documenting samples that do not compile**: Shipped copy-paste fences must build.
- **A playground in this sprint**: Playground and in-page runners stay later.

## Consequences

- **`website/generate.drac`**: Retired as the site renderer. `scripts/generate-website.sh` wraps the Start static build.
- **Fence extraction tests**: Cases in `tests/integration/tests/website_pipeline.rs` stay until retargeted.
- **App code**: Lives under `website/` nested `src/`.
- **Colour**: No hardcoded hex in components; Tailwind v4 `@theme` tokens.

## Relationships

- **Kept split**: [[0010-public-docs-draconic-ssg]]
- **Guide**: [[guides-public-docs]]
- **Glossary**: [[CONTEXT]]
- **Product ladder**: [[Public site purpose]], [[Public site — Contract]]
- **See also**: `.heio` sprint `website-redesign` (working plan, not a vault note)
