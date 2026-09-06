---
name: obsidian-axi
description: Obsidian vault CLI (`npx -y @andershoffmann/obsidian-axi`). Use when searching a vault, following wikilinks or backlinks, listing tags, or patching a heading or frontmatter field in the `docs/` vault.
metadata:
  version: "1.0.0"
---

# Obsidian AXI

Filesystem CLI for `docs/` Obsidian vault. TOON output. No plugin, no server, no running app. Per-rule detail lives in `rules/<prefix>-*.md`. Command catalog lives in [AGENTS.md](AGENTS.md) (reference only).

## Discover first

1. Resolve the binary (`disc-cli-binary`). Prefer `npx -y @andershoffmann/obsidian-axi`. A global `obsidian-axi` is fine if `--help` works.
2. Resolve the vault (`disc-vault-resolve`). This checkout is `docs/`. Pass `--vault docs` after the command. There is no `docs/.obsidian/`.
3. Run the no-args dashboard (`disc-dashboard-first`). Then search. Then read.

If the path is not a vault of markdown notes, stop.

## Stack caveats

**Assumes:** Node 20+. Vault on disk. This repo's vault is `docs/`.

**Prefer:**

- **search-** and **graph-** for vault find and links
- **patch-*** for one heading, block, or frontmatter field
- **docs** for kind, template, layout, and required frontmatter on a new durable note

**Careful:**

- Agent `Edit` when the file is already in session and the change is whole-body prose
- `write` only after **docs** has placed the file
- `active` / `open` need a saved Obsidian workspace; this checkout may not have one
- Unsaved notes in the Obsidian app can overwrite CLI writes (`mut-sync-overwrite`)

**Do not introduce:**

- Obsidian MCP, community plugins, or a local HTTP API
- The unscoped `obsidian-axi` / HermitCountry wrapper (needs the official `obsidian` CLI and a running app)
- `notesmd-cli` or yakitrak `obsidian-cli`
- Dataview, Templater, or semantic search through this CLI (`UNSUPPORTED`)
- Daily notes or vault tasks as the project tracker

## When to apply

- Search, read, or list notes in an Obsidian vault
- Backlinks, broken links, orphans, tags
- Patch a heading, block, or frontmatter field
- Append, rename, or trash a note
- Dashboard, recent notes, vault info

## Rule categories by priority

- **1 CRITICAL** - Discover (`disc-`)
- **2 CRITICAL** - AXI contract (`axi-`)
- **3 HIGH** - Read (`read-`)
- **4 HIGH** - Search (`search-`)
- **5 HIGH** - Mutate (`mut-` `patch-` `rm-` `append-`)
- **6 HIGH** - Graph (`graph-`)
- **7 MEDIUM** - List (`list-`)
- **8 MEDIUM** - Workspace (`ws-`)
- **9 LOW** - Layer (`layer-`)

## Quick reference

### 1. Discover (CRITICAL)

- `disc-cli-binary` `npx -y @andershoffmann/obsidian-axi`
- `disc-vault-resolve` `--vault` after the command; this repo is `docs/`
- `disc-dashboard-first` no-args first

### 2. AXI contract (CRITICAL)

- `axi-flags-after` flags after the command, never before
- `axi-toon-read` TOON + `count:`; do not re-parse as JSON
- `axi-help-follow` run the `help[]` follow-up
- `axi-exit-codes` 0 includes no-ops; errors on stdout

### 3. Read (HIGH)

- `read-truncate-full` bodies cap at 1000 chars; `--full` to escape
- `read-metadata` frontmatter and counts without the body
- `read-multi` several paths in one call

### 4. Search (HIGH)

- `search-narrow` `--tag` `--path` `--frontmatter` `--modified-since`
- `search-context` `--context N` for surrounding lines
- `search-regex-path` `--regex` plus `--path` glob

### 5. Mutate (HIGH)

- `mut-prefer-patch` patch over write
- `mut-idempotent` retry is safe
- `mut-content-file` newlines via `--content-file` or stdin
- `mut-sync-overwrite` unsaved app buffers win
- `patch-heading-scope` own section; `::` for a child
- `patch-frontmatter-target` value lives in `--target`
- `patch-heading-delete` delete removes the subtree
- `rm-trash` `.trash` unless `--permanent`
- `append-end` append vs heading patch

### 6. Graph (HIGH)

- `graph-links` `links`, `--broken`, `--orphans`
- `graph-link-resolve` path, relative, filename, alias
- `graph-tags` list, `tags files`, nested rollup

### 7 to 9. List, workspace, layer

- `list-ls` `ls -r --sort modified`
- `list-recent` `recent --days`
- `list-fields` `--fields path`
- `ws-active` last saved layout, not the screen
- `ws-open` `open` asks the app
- `ws-daily-not-tracker` daily notes are not tickets
- `layer-docs-create` new durable notes go through **docs**
- `layer-heio-boundary` this CLI does not own tracker meaning
- `layer-no-mcp-plugin` filesystem CLI only
- `layer-unsupported` Dataview / Templater / semantic search

## How to use

1. Discover (binary, vault, dashboard).
2. Pick 1–N rule ids (higher priority first).
3. `Read` only `rules/<id>.md` (relative to this skill directory).
4. Do **not** bulk-read `rules/` or load all of `AGENTS.md` unless stuck or asked.

```text
rules/search-narrow.md
rules/patch-heading-scope.md
```

Each rule: why → incorrect → correct → notes.

## Full reference

Upstream command catalog and AXI notes: `AGENTS.md` (reference only; prefer `rules/` + this router).
