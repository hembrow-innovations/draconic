# Agents

This checkout runs **OpenCode** only. Project skills live under `.opencode/skills/`. Do not run Hivemind, Pi, or heio-stack. If another file names those runtimes, this file wins.

## Skills

Load the matching skill before the work it covers.

- **docs**: committed vault under `docs/`
- **domain-modeling**: glossary and ADRs
- **draconic-loop**: one atomic Roadmap item, test-first
- **gauntlet-loop**: bounded implement-then-critic until the bar wins
- **diagnose**: hard bugs and performance regressions
- **behaviour-contracts**: intended behaviour under `docs/`
- **codebase-design**: module seams and interfaces

## Domain docs

Single-context layout (`CONTEXT.md` + `docs/adr/`). See `docs/agents/domain.md`. `AGENTS.md` wins over the docs skill default layout: locked decisions live in `docs/adr/`, not `docs/decisions/adr/`.

- **Glossary**: `CONTEXT.md`
- **Locked decisions**: `docs/adr/`
- **Completeness**: `ROADMAP.md` — Loop source of truth with the Conformance suite

## Draconic language

This repo **is** the Draconic toolchain. Completeness is driven by:

- [`ROADMAP.md`](./ROADMAP.md) — feature checklist (Loop source of truth)
- [`CONTEXT.md`](./CONTEXT.md) — glossary
- [`docs/adr/`](./docs/adr/) — locked decisions
- **draconic-loop** skill — one atomic Roadmap item per Loop (test-first)

Prefer `cargo test --workspace` and the `draconic` CLI over ad-hoc scripts.

Each `rs` file should have a soft limit of 1,000 lines.

## Git

**Commit every work package.** When a Roadmap Loop item (or any discrete unit of work) is marked `done` or otherwise finished, stage its changes and create a git commit before starting the next item. One commit per completed work package; message should name the Roadmap ID(s) and summarize the change. Never stage `.heio/` or `.opencode/node_modules/`.

## Rules

- Markdown: never tables — use `- **{text}**: {text}`
- Do not invent work when ROADMAP has no `todo` and the user did not name a task
- Always keep the `target` directory below 10GB
- Do not edit `.hivemind/hivemind.yaml`, `.pi/` copies, or heio-stack operating notes

## Loop

Language work is one Roadmap atom per sitting. Load **draconic-loop**. For a named implement-and-verify task, load **gauntlet-loop** as well. Default stop is one item; do not start the next unless the user says to continue.
