---
id: standards-markdown
title: Markdown in the vault
kind: standard
description: Frontmatter, headings, wikilinks, and list form required of committed docs notes.
domain: draconic
area: standards
tags: [standard, markdown]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Markdown in the vault

## Statement

Every committed vault note starts with YAML frontmatter, uses an h1 that equals `title`, links other notes with `[[wikilinks]]` only, and never uses markdown tables. Labelled facts use a dash-star list: `- **label**: text`. [[AGENTS]] states the table rule for the whole checkout.

## Rationale

Tables break TTS, diffs, and Obsidian moves. Relative `.md` paths break when a note is renamed. An h1 that disagrees with `title` splits search and Dataview. Required frontmatter keeps kind, domain, and area queryable without inventing a second index.

## Rules

- **No tables**: never pipe tables. Use `- **label**: text` (or a short prose sentence).
- **Wikilinks only**: `[[note-name]]` or `[[note-name|alias]]`. Do not use relative `.md` paths between notes.
- **h1 equals title**: the first heading string is exactly the `title` frontmatter value.
- **id is the filename stem**: unless an ADR rule says otherwise. Do not change `id` when you retitle.
- **Required frontmatter**:
  - **id**: filename stem
  - **title**: same string as the h1
  - **kind**: `overview`, `architecture`, `system-design`, `adr`, `rfc`, `purpose`, `contract`, `test`, `spec`, `api`, `schema`, `non-functional`, `standard`, `style`, or `guide`
  - **domain**: subject domain (`draconic` for this toolchain)
  - **area**: area slug (`standards`, `guides`, `non-functional`, `api`, and so on)
  - **tags**: YAML list
  - **created_at**: ISO-8601 or `YYYY-MM-DD`
  - **updated_at**: ISO-8601 or `YYYY-MM-DD`
- **Optional frontmatter**: `description`, `status`, `source` when the kind template names them.
- **Kind templates**: copy the docs-skill skeleton (`standard`, `guide`, `non-functional`, `api`, and the rest). Keep the template section headings.
- **Prose terms**: use [[CONTEXT]] vocabulary (Program, Toolchain, Frontend, IR, Loop). Do not invent synonyms.
- **Place notes** per [[standards-docs-vault]].

## Examples

Correct:

- **List fact**: `- **Glossary**: [[CONTEXT]]`
- **Wikilink**: `[[architecture-cli]]`, `[[0010-public-docs-draconic-ssg]]`
- **Frontmatter id**: file `docs/guides/guides-loop.md` has `id: guides-loop` and h1 `Language Loop`

Incorrect:

- **Table**: a pipe grid of CLI flags (document flags as a labelled list in [[api-cli]] instead)
- **Relative link**: `[cli](../api/api-cli.md)`
- **Mismatched heading**: `title: Markdown in the vault` with h1 `# Vault markdown`

## Exceptions

- **Leftover notes** under `docs/reference/guides/` ([[issue-tracker]], [[triage-labels]]) may predate this standard. Do not “fix” them in passing. New notes must follow these rules.
- **Repo-root [[CONTEXT]] and [[ROADMAP]]** are not vault-kind notes. They do not carry this frontmatter block.
- **Public site sources** in `website/` use a smaller markdown subset and status frontmatter ([[0010-public-docs-draconic-ssg]], [[guides-public-docs]]). They are not vault notes.
- **Code fences** may contain source that is not vault markdown. Do not put a table in the surrounding prose.
