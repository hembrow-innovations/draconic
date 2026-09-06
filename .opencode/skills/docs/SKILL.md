---
name: docs
description: How to find, read, and write committed project documentation under `docs/`. Obsidian vault and the source of truth. ADRs, RFCs, architecture, APIs, standards, guides, and spec templates. Not the day-to-day tracker. Use whenever you look for docs, add a durable note, or update existing docs. Spec folder procedure: load **spec**.
---

# Docs. The source of truth

`docs/` is an Obsidian vault and the committed store for project knowledge that should survive a clone. Architecture, decisions, specs, APIs, standards, and guides live here.

Day-to-day work does not. Tickets, slices, tasks, and rounds live under `.heio/`. Load the **management** skill for that tree.

When you need lasting context, look here first. When you produce durable knowledge, write it here.

Per-rule detail lives in `rules/<prefix>-*.md`. Copy-ready skeletons live in `templates/`.

If `AGENTS.md` already names a different docs layout, that file wins. Do not start a second vault.

## Before writing (always)

1. Search `docs/` first. Update in place over near-dupes.
2. Pick the kind. Copy `templates/<kind>.md`. Spec folder procedure (ladder, write order): load **spec**.
3. Place and name per `layout-vault` and the template convention.
4. Ignore `docs/99_scribble/`.

Full steps: `rules/write-before.md`.

## When to apply

- Finding or reading project docs under `docs/`
- Adding or updating an ADR, RFC, architecture note, API, standard, style, or guide
- Moving or renaming notes
- Choosing vault path, frontmatter, or wikilink style
- Deciding whether a note belongs in `docs/` or `.heio/`

## Prefer / careful / do not

### Prefer

- **write-before** before any new note
- **layout-vault** for path and naming
- **obsidian-axi** for vault search, backlinks, tags, heading patch
- **obsidian-standards** for frontmatter, tags, wikilinks, Tasks, Dataview
- **spec** for spec-folder procedure. Templates for purpose, contract, test, and spec live in this skill
- **template-kinds** plus `templates/<kind>.md` for the skeleton
- **link-wikilinks** for note-to-note links
- **mgmt-boundary** when the note is still in-flight work

### Careful

- **mgmt-boundary.** A working slice is not a spec. Promote the durable outcome, then close the working file.

### Do not

- Put tickets, slices, tasks, rounds, or working reports in `docs/`
- Create living `web/{requirements,design,tasks}.md` triad files under specs
- Treat `docs/99_scribble/` as source of truth
- Use relative `.md` paths between notes (use `[[wikilinks]]`)
- Put a domain folder under `docs/` except `docs/specs/<domain>/` (path equals frontmatter `domain:`)
- Flatten specs; **spec** owns the folder procedure, this skill owns the templates
- Open a GitHub Issue for knowledge this vault already holds

## Rule categories by priority

- **1 CRITICAL** - Before writing (`write-`)
- **2 CRITICAL** - Vault layout (`layout-`)
- **3 CRITICAL** - Obsidian standards (`obsidian-`)
- **4 HIGH** - Spec folders (`spec-`)
- **5 HIGH** - Templates (`template-`)
- **6 HIGH** - Links (`link-`)
- **7 HIGH** - Management boundary (`mgmt-`)

## Quick reference

### 1. Before writing (CRITICAL)

- `write-before` Search first, pick kind, place, ignore scribble

### 2. Vault layout (CRITICAL)

- `layout-vault` Tree, naming, domain vs kind path

### 3. Obsidian standards (CRITICAL)

- `obsidian-standards` Frontmatter, wikilinks, tags, Tasks, Dataview

### 4. Spec folders (HIGH)

- `spec-folder` Path reminder. Copy templates here. Load **spec** for the ladder

### 5. Templates (HIGH)

- `template-kinds` Kind to template file to destination. See `templates/`

### 6. Links (HIGH)

- `link-wikilinks` `[[note-name]]` only. Working tracker is the management skill

### 7. Management boundary (HIGH)

- `mgmt-boundary` `docs/` is truth. `.heio/` is day-to-day

## How to use

```
rules/write-before.md
rules/layout-vault.md
rules/obsidian-standards.md
rules/spec-folder.md
rules/template-kinds.md
rules/link-wikilinks.md
rules/mgmt-boundary.md
templates/required-fields.md
templates/<kind>.md
```

Read only the rules for the current task. Do not bulk-read `rules/` or every template.

Working lifecycle (ticket to slice to task to close). Load the **management** skill.
