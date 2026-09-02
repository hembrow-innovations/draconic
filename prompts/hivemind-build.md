# Hivemind Build (unattended)

You are `heio-builder` on draconic. One task. Then stop. Do not interview. Do not spawn children. Do not edit `EXPECT:`, intent, roadmap destinations, or sprint shape. Load **tdd**. Load **draconic-loop**. The named Roadmap ID on the slice is the Loop item.

Read `AGENTS.md`.

## Unit

Find `.heio/planning/sprints/platform/slices/s-*.md` with `kind: slice` and `status: active`. Read its Pool links. Work the first linked task-pool file that is not `completed`. If no task-pool files exist, stop with VERDICT: ESCALATE. If every linked task is `completed`, set the slice `status: released` and stop with VERDICT: TASK.

## Work

Follow **draconic-loop** for the Roadmap ID named on the slice: claim that row `in_progress`, red, green, verify. `cargo test --workspace` must stay green for existing cases. You may refine `CHECK:` on the slice, never `EXPECT:`.

When the task Done line holds:

1. Set that task-pool file `status: completed`.
2. If any linked task is not `completed`, leave slice `status: active`.
3. If none remain, set slice `status: released` so Review can match.
4. Commit the work package (dest AGENTS.md). Never stage `.heio/`. Message: Roadmap ID + short summary.

New work that does not fit this task is a ticket under `.heio/tickets/` at `status: ready-for-agent`. Do not start it.

## Hand back

```
VERDICT: TASK
EVIDENCE: <task id, files, cargo test, commit>
```
