---
name: heio-planning
description: Planning interview for a one-file slice (oracles on the slice) or a ticket, under heio-stack.
disable-model-invocation: true
---

# Planning a slice or a ticket

Interview until you share an understanding. Then persist the slice file with frozen `EXPECT:`, or triage the ticket. Do not build. Do not write task-pool files. Execution, **heio-slice**, and **heio-tasker** do that.

Load **heio-stack** before any write under `.heio/`. Load **docs** before any write under `docs/`. Load **domain-modeling** when a term or ADR belongs in the vault.

If `AGENTS.md` or `WORKSPACE.md` already names a tracker, that file wins.

## Rounds

Same frontier format as **heio-wayfinder**. Ask the whole frontier. Recommend an answer. Wait.

A slice you cannot demo or learn from in one sitting is two slices. Split before you write.

## Slice

User names a slice on a current sprint, or this interview produces one.

1. Read intent, roadmap, any location file, and sprint `shape.md`. The slice stays inside that grouping.
2. Write **Done** in words on the slice file. Then immediately write oracles. If you cannot write a `CHECK:` / `EXPECT:`, stop and sharpen. The done is still mush.
3. Name `blocked-by` when this slice must wait. Unblocked slices may run in parallel.
4. Oracles only for external truth — user-visible, contract, data invariant, “ops can tell.” Internal design stays in TDD.
5. Carry enough ADRs, specs, and paths that a stranger does not hunt.
6. Stop. Summarize Done + `EXPECT:` lines. Wait for confirm.
7. Copy `templates/slice.md` to `.heio/planning/sprints/<id>/slices/s-<slug>.md`. Status `frozen`. `EXPECT:` is frozen. The first `CHECK:` is drafted here; the builder may refine the command later.
8. Do not write task-pool files. Tasks exist when the slice is `frozen` or `active`, via **heio-slice** / **heio-tasker**.

Done when the slice file exists, every oracle has `CHECK:` and `EXPECT:`, status is `frozen`, and no new task-pool files were written by this pass.

## Ticket

User names a ticket, or an inbound signal that is not yet a ticket.

1. If no file exists, copy `templates/ticket.md` into `.heio/tickets/`.
2. Interview only enough to triage. The solution does not live on the ticket.
3. Same rule every time:
   - Fits an unblocked active slice → **TASK**. Status `promoted`. Name the task for **heio-tasker** (task-pool file + slice `[[id]]` link), or say the line to add. Do not write the task-pool file.
   - Fits the project, not this slice → **TICKET**. Status `parked`.
   - Would rewrite a location destination during a workflow → **ESCALATE**. Stop. Hand to **heio-wayfinder**.

Done when the ticket has a status and a verdict.

## Loop

End with `VERDICT: TASK | TICKET | ESCALATE | VERIFY`. Planning a slice is not VERIFY. VERIFY checks oracles on the slice file.
