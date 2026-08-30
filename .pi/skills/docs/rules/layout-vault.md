---
title: Vault layout and naming
impact: CRITICAL
tags: [layout]
---

# Vault layout and naming

`docs/` sits at the project root. Commit it. It is the vault. Kind folders live directly under it.

```text
docs/
├─ overview/         overview-<slug>.md, glossary.md, roadmap-<slug>.md
├─ architecture/     architecture-<slug>.md, system-design-<slug>.md
├─ decisions/
│  ├─ adr/           NNNN-<slug>.md
│  └─ rfc/           rfc<N>-<slug>.md
├─ non-functional/   security.md, performance.md, scalability.md, reliability.md
├─ api/              api-<slug>.md
│  └─ schema/        schema-<slug>.md
├─ specs/            <bucket>/<area>/{purpose,spec-<slug>}.md
├─ standards/        standards-<slug>.md
├─ style/            style-<slug>.md
├─ guides/           guides-<slug>.md
└─ 99_scribble/      IGNORE
```

Create a folder when the first file needs it.

There is no `planning/`, no `log/`, and no `reference/` under `docs/`. Planning and logs belong to `.heio/`.

## Kind vs domain

The folder under `docs/` is the doc kind. The domain the doc concerns rides in frontmatter as `domain:`. Do not invent `docs/<domain>/`.

**Specs exception.** `docs/specs/<bucket>/<area>/` may use path buckets that match the project's package groups. Put the same value in `domain:`. `area:` is the per-area slug. If the project has no package groups, write `docs/specs/<area>/`.

## Numbers

ADR filename is `NNNN-<slug>.md`, zero-padded. Frontmatter `id` is `adr-<N>`. Never renumber. Never delete. Move a retired ADR to `docs/decisions/adr/archive/`.

RFC filename is `rfc<N>-<slug>.md`. Scan live files for the next `N`. Start at `1`.

`slug` is lowercase kebab-case. Keep it short.

## Moves

Edit files under `docs/` directly. After any rename, search `docs/` for the old path and fix leftovers before you commit.
