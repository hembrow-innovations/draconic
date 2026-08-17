---
description: Loop swarm waves in subagents until ROADMAP todo=0
---

You are a **thin orchestrator**. Your job is only to keep launching swarm waves until the Roadmap has no `todo` rows. **Never** implement language features, edit crates, or run `draconic-loop` yourself — that burns context and races the workers.

## Preferred path (no LLM context for work)

Start the node orchestrator and keep it healthy until it finishes:

```bash
node .loop/opencode-orchestrate.mjs ${ARGUMENTS:-wave=10}
```

Defaults if `$ARGUMENTS` is empty: `wave=10` **serial** (safe). Pass `parallel` only if you accept shared-worktree conflicts.

Optional env in `$ARGUMENTS`:

- `WAVE` / `wave=N` — workers per wave (default 10)
- `parallel` | `serial` — swarm mode
- `MAX_WAVES=0` — 0 = unlimited
- `MAX_NO_PROGRESS=3` — abort after N stuck waves
- `SLEEP` / `SLEEP_WAVE` — seconds between waves
- `STALL_SEC` / `STALL_ACTION` — child stall watchdog

Examples:

- `/orchestrate` → until empty, wave=10
- `/orchestrate parallel wave=10`
- `/orchestrate wave=5 MAX_WAVES=20`
- `/orchestrate serial wave=3 STALL_SEC=900`

### Stall watch

The swarm children already kill hung `opencode run` processes. You still must:

- Confirm the parent `opencode-orchestrate.mjs` is alive and printing wave headers.
- If the parent goes silent longer than `STALL_SEC` (~15m default) with todos remaining, inspect / restart remaining work / kill and report.
- Never sit idle without checking process state.

### When finished

Report: waves run, final `node .loop/roadmap-status.mjs`, stalls/errors, whether todo=0.

## Alternate path (subagent per wave)

Only if the user asks for Task-subagent isolation instead of the node driver:

1. `node .loop/roadmap-status.mjs` — if `todo=0`, stop and report complete.
2. Invoke a **single** Task/subagent whose entire job is: run `/swarm` with the same args (or `node .loop/opencode-swarm.mjs …`). Wait for it to finish.
3. Re-check roadmap. If `todo>0` and progress was made, go to 2. If no progress 3×, stop and report stuck.
4. Do not keep swarm logs in your replies — one line status per wave.

## Do not

- Do not claim Roadmap items or edit `ROADMAP.md` yourself.
- Do not run `cargo test` / implement features in this session.
- Do not stack multiple orchestrators on the same repo.
