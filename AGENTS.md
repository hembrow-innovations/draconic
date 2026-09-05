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

Chart intent and sprints with **heio-wayfinder**. Plan a slice or ticket with **heio-planning**. File-backed sittings use **heio-rounds** (stop at `awaiting-confirm`; do not publish). Execute a `ready` slice with **heio-slice**. Nouns for this dest are the Hivemind card below. Every heio output ends `VERDICT: TASK | TICKET | ESCALATE | VERIFY`.

Inbound product work that is not a Roadmap atom is a ticket under `.heio/tickets/`. Do not file ECMA-262 Loop atoms only as vault issues.

## Hivemind statuses (this dest)

This dest runs **without a human in the lane**. Do not interview. Do not wait for confirm. Do not spawn children. Do the unit named in the prompt, write the disk, die.

**This dest overlays Hivemind nouns.** `AGENTS.md` wins. In-session heio-stack SKILL.md may still say `frozen` / `open`; do not copy those nouns onto this dest, and do not edit `.pi/` skills.

Tickets. Never write `open`, `parked`, or `ready-for-human` on this dest.

- **ready-for-agent**: Planner may take it
- **active**: Planner claimed it this run
- **promoted**: Planner finished; slice exists
- **dropped** / **closed**: dead

Slices (one file `s-<slug>.md`, `kind: slice`). Never write `frozen`. Planner writes `active` with task-pool files in one sitting.

- **ready**: sealed Done + EXPECT (legacy; Planner now publishes `active`)
- **active**: Builder may gauntlet an incomplete task
- **released**: no incomplete task-pool links; Reviewer may take it
- **reviewing**: Reviewer claimed
- **met**: oracles ALL MET
- **failed**: Reviewer miss or Builder plateau with no remaining work; slice stays sealed. Failed is **not** in-flight.

Task-pool: `draft` → `ready` → `claimed` → `implemented` → `completed`. **blocked** is a gauntlet plateau. Builder skips `blocked` and `completed`.

Pump (`.heio/planning/pump.md`, `kind: pump`) is the Planner lock, not its own lane:

- **idle**: engine may claim Planner
- **active**: Planner claimed this run
- **held**: at WIP cap, or Planner had nothing to mint/plan
- **exhausted**: no ROADMAP `todo`; watch `--until-quiet` can exit

Occupancy: WIP cap is **3**. In-flight is tickets `ready-for-agent`/`active` plus slices `ready`/`active`/`released`/`reviewing`. Planner may mint/plan while Builder and Reviewer run, while in-flight is under cap. At cap, pump is **held**. Empty board + held pump becomes **idle** so Planner can mint.

Reviewer and Builder misses mint `ready-for-agent`, not `ready-for-human`.

Interactive Pi is the control plane (status/housekeep scripts). Language Loop atoms run in Hivemind `--print --no-session`. Do not wrap Loop atoms in Swarm Pi.

- **status**: `node scripts/hivemind-status.mjs`
- **housekeep**: `node scripts/heio-housekeep.mjs --dry-run` (apply is `--apply`)

## Rules

- Markdown: never tables — use `- **{text}**: {text}`
- Front matter on `.heio/planning/**/*.md` may only use: `id`, `title`, `kind`, `status`, `sprint`, `tags`, `created_at`, `updated_at`, `claimed-by`, `blocked-by`, `mode`, `sitting-kind`. Extra keys quarantine the file.
- Front matter on tickets may only use the ticket schema keys. Extra keys quarantine.
- Do not edit `.hivemind/hivemind.yaml` unless changing lanes on purpose.
- Do not edit `.pi/` copies.
- Do not write intent, roadmap, or sprint destination sentences from Builder or Reviewer.
- Do not invent work when no matching ticket or slice exists and ROADMAP has no `todo`.
- Always keep the `target` directory below 10GB.

## Loop

Three lanes. They match different notes, so they run in parallel. Each sitting is one unit, then die.

- **Planner** (`planner`): idle pump lock. One `ready-for-agent` ticket → one slice with oracles and task-pool files, `status: active`, ticket `promoted`. Else mint one ROADMAP `todo` ticket at `ready-for-agent` when WIP is under cap. Else `held` or `exhausted`. No product code.
- **Builder** (`builder`): one incomplete task on an `active` slice. **gauntlet-loop** + **draconic-loop** + TDD. Critic hat is `CHECK:`/`EXPECT:`. Max 3 rounds. Same gap twice or budget hit → task `blocked`, mint a fix ticket, stop. Win → `completed` and commit. If none remain, set slice `status: released`. If only `blocked` remain, set `failed`.
- **Reviewer** (`reviewer`): `--reverify`. ALL MET → `met` and ROADMAP `done`. Miss → slice `failed` plus a new ticket at `ready-for-agent` with `caused-by` so Planner can feed a fix slice to Builder.

## Pi

Install / refresh the dest pack from a local agentic-core checkout:

```
node scripts/install-heio.mjs
```

Override the source with `AGENTIC_CORE=/path/to/agentic-core`. Dest is `.pi/`. Profile is `draconic`. Run `pi` in this directory and trust the folder. Reinstall does not seed Hivemind. Copy `apps/hivemind` from `../hivemind` into `.pi/frameworks/hivemind` when the engine is missing or stale. Runtime config is `.hivemind/hivemind.yaml` only.

Unattended loop:

```
node scripts/run-hivemind.mjs watch --until-quiet
node scripts/run-hivemind.mjs once
node scripts/hivemind-status.mjs
node scripts/heio-housekeep.mjs --dry-run
```

`--approve` on every lane trusts this folder. No trust prompt. No TUI.
