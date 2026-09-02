## Agent skills

### Domain docs

Single-context layout (`CONTEXT.md` + `docs/adr/`). See `docs/agents/domain.md`.

### Draconic language

This repo **is** the Draconic toolchain. Completeness is driven by:

- [`ROADMAP.md`](./ROADMAP.md) — feature checklist (Loop source of truth)
- [`CONTEXT.md`](./CONTEXT.md) — glossary
- [`docs/adr/`](./docs/adr/) — locked decisions
- **draconic-loop** skill — one atomic Roadmap item per Loop (test-first)

Prefer `cargo test --workspace` and the `draconic` CLI over ad-hoc scripts.

Each `rs` file should have a soft limit of 1,000 lines.

### Git

**Commit every work package.** When a Roadmap Loop item (or any discrete unit of work) is marked `done` or otherwise finished, stage its changes and create a git commit before starting the next item. One commit per completed work package; message should name the Roadmap ID(s) and summarize the change. Never stage `.heio/`.

## Tracker

This checkout runs **heio-stack**. Live operating notes live under `.heio/planning/` and `.heio/tickets/`. Git ignores `.heio/`. `docs/` (including historical `docs/planning/`) stays the committed vault.

Language completeness remains [`ROADMAP.md`](./ROADMAP.md) — that is the **draconic-loop** source of truth. Do not replace ROADMAP.md with `.heio/planning/roadmap.md`. Heio-stack is the agent operating loop (verdicts, slices, tickets); ROADMAP.md is the language feature checklist.

Chart intent and sprints with **heio-wayfinder**. Plan a slice or ticket with **heio-planning**. Execute a frozen slice with **heio-slice**. Every heio output ends `VERDICT: TASK | TICKET | ESCALATE | VERIFY`.

Inbound product work that is not a Roadmap atom is a ticket under `.heio/tickets/`. Do not file ECMA-262 Loop atoms only as vault issues.

## Hivemind statuses (this dest)

This dest runs **without a human in the lane**. Do not interview. Do not wait for confirm. Do not spawn children. Do the unit named in the prompt, write the disk, die.

In-session heio-stack docs say `frozen` / `met` and ticket `open` / `parked`. **This dest overlays Hivemind nouns.** `AGENTS.md` wins.

Tickets:

- **ready-for-agent**: Plan may take it
- **active**: Plan claimed it this run
- **promoted**: Plan finished; slice exists
- **dropped** / **closed**: dead

Slices (one file `s-<slug>.md`, `kind: slice`):

- **ready**: sealed Done + EXPECT; Tasker may take it (replaces `frozen`)
- **active**: Tasker claimed, or Build is looping tasks
- **released**: no incomplete task-pool links; Review may take it
- **reviewing**: Review claimed
- **met**: oracles ALL MET
- **failed**: Review miss; slice stays sealed

Pump (`.heio/planning/pump.md`, `kind: pump`):

- **idle**: Pump may mint the next ROADMAP `todo` as one ticket
- **active**: Pump claimed this run
- **exhausted**: no ROADMAP `todo` left; watch `--until-quiet` can exit

Do not write `frozen` on a slice. Tasker only matches `ready`. Review misses mint `ready-for-agent`, not `ready-for-human`.

## Rules

- Markdown: never tables — use `- **{text}**: {text}`
- Front matter on `.heio/planning/**/*.md` may only use: `id`, `title`, `kind`, `status`, `sprint`, `tags`, `created_at`, `updated_at`, `claimed-by`, `blocked-by`. Extra keys quarantine the file.
- Front matter on tickets may only use the ticket schema keys. Extra keys quarantine.
- Do not edit `hivemind.yaml` unless changing lanes on purpose.
- Do not edit `.pi/` copies.
- Do not write intent, roadmap, or sprint destination sentences from Build or Review.
- Do not invent work when no matching ticket or slice exists and ROADMAP has no `todo`.

## Loop

- **Pump** (`heio-triage`): idle pump + empty in-flight queue → one ROADMAP `todo` ticket at `ready-for-agent`, or `exhausted`.
- **Plan** (`heio-planner`): one `ready-for-agent` ticket → one slice file with oracles, `status: ready`. Ticket → `promoted`.
- **Tasker** (`heio-tasker`): claimed `ready` slice (now `active`) → task-pool files + slice `[[id]]` links.
- **Build** (`heio-builder`): one incomplete task, **draconic-loop** + TDD, commit. If none remain, set slice `status: released`.
- **Review** (`heio-verifier`): `--reverify`. ALL MET → `met` and ROADMAP `done`. Miss → slice `failed` plus a new ticket at `ready-for-agent` with `caused-by`.

## Pi

Install / refresh the dest pack from a local agentic-core checkout:

```
node scripts/install-heio.mjs
```

Override the source with `AGENTIC_CORE=/path/to/agentic-core`. Dest is `.pi/`. Profile is `draconic`. Run `pi` in this directory and trust the folder.

Unattended loop:

```
node scripts/run-hivemind.mjs watch --until-quiet
node scripts/run-hivemind.mjs once
node scripts/hivemind-status.mjs
```
