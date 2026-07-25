## Agent skills

### Issue tracker

Issues live as local markdown under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default roles: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout (`CONTEXT.md` + `docs/adr/`). See `docs/agents/domain.md`.

### Draconic language

This repo **is** the Draconic toolchain. Completeness is driven by:

- [`ROADMAP.md`](./ROADMAP.md) — feature checklist
- [`CONTEXT.md`](./CONTEXT.md) — glossary
- [`docs/adr/`](./docs/adr/) — locked decisions
- **draconic-loop** skill — one atomic Roadmap item per Loop (test-first)

Prefer `cargo test --workspace` and the `draconic` CLI over ad-hoc scripts.

Unattended multi-iteration driver: `node .loop/opencode-loop.mjs <n>` (see `.loop/`).

Each `rs` file should have a soft limit of 1,000 lines.

### Git

**Commit every work package.** When a Roadmap Loop item (or any discrete unit of work) is marked `done` or otherwise finished, stage its changes and create a git commit before starting the next item. One commit per completed work package; message should name the Roadmap ID(s) and summarize the change.
