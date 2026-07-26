## Agent skills

### Issue tracker

Vault under `docs/planning/` (issues / plans / tasks) — **no GitHub Issues**, **no `.scratch/` tracker**. See `docs/reference/guides/issue-tracker.md` (pointer: `docs/agents/issue-tracker.md`). Next id: `node scripts/planning-next-id.mjs`.

### Triage labels

Frontmatter `status` + `tags` (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`; category `bug` | `enhancement`). See `docs/reference/guides/triage-labels.md`.

### Domain docs

Single-context layout (`CONTEXT.md` + `docs/adr/`). See `docs/agents/domain.md`.

### Planning workflow

Load **planning-workflow** for issue → execute → review → new issues. Single-unit work = `ready-for-agent` issue + `## Agent Brief` (no task note).

### Draconic language

This repo **is** the Draconic toolchain. Completeness is driven by:

- [`ROADMAP.md`](./ROADMAP.md) — feature checklist (Loop source of truth)
- [`CONTEXT.md`](./CONTEXT.md) — glossary
- [`docs/adr/`](./docs/adr/) — locked decisions
- **draconic-loop** skill — one atomic Roadmap item per Loop (test-first)

Prefer `cargo test --workspace` and the `draconic` CLI over ad-hoc scripts.

Unattended multi-iteration driver: `node .loop/opencode-loop.mjs <n>` (see `.loop/`).

Each `rs` file should have a soft limit of 1,000 lines.

### Git

**Commit every work package.** When a Roadmap Loop item (or any discrete unit of work) is marked `done` or otherwise finished, stage its changes and create a git commit before starting the next item. One commit per completed work package; message should name the Roadmap ID(s) and summarize the change.
