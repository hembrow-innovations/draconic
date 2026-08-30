---
title: Folder layout and naming
impact: CRITICAL
tags: [layout]
---

# Folder layout and naming

`.heio/` sits at the project root. Gitignore the whole directory. Add `.heio/` to `.gitignore` if it is missing.

```text
.heio/
├─ tickets/
│  └─ ticket-01-<slug>.md
└─ planning/
   ├─ intent.md
   ├─ roadmap.md
   └─ sprints/
      └─ <sprint-id>/
         ├─ shape.md
         └─ slices/
            └─ s-<slug>/
               ├─ spec.md
               ├─ oracles.md
               └─ tasks.md
```

Create a folder when the first file needs it.

Parked tickets live in `.heio/tickets/`, never in a slice. `tasks.md` exists only on the active slice.

Reserved at the root, owned by other skills. Leave them in place.

- **TODO.md**: session stub. Call `heio_todo`.
- **decisions.tsv**: long-run decision trail
- **oracles.md**: root ledger owned by the **oracle** skill. Slice ledgers live on the slice.
- **worktrees/**, **sessions/**, **teams/**: runtime

## Names

- **sprint-id**: short folder name (`m3`, `launch`). The id is the folder.
- **slice folder**: `s-<slug>`. Lowercase kebab-case.
- **ticket**: `ticket-<NN>-<slug>.md`. `<NN>` is the next unused integer, zero-padded to two digits. Scan `.heio/tickets/`. Start at `01`. Do not reuse a number.

`slug` is lowercase kebab-case. Keep it short.

## Status

- **intent**: `active` / `superseded`
- **roadmap**: `draft` / `active`
- **sprint**: `shaping` / `active` / `review` / `closed`
- **slice**: `shaping` / `frozen` / `active` / `met` / `abandoned`
- **ticket**: `open` / `parked` / `promoted` / `dropped` / `closed`

## Links

Link notes with `[[id]]`. The `id` is the stem or folder name (`ticket-01-login-flash`, `s-login`, `m3`).
