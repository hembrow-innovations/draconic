---
name: heio-planning
description: Planning interview for a slice spec and oracles, or for a ticket, under heio-stack.
disable-model-invocation: true
---

# Planning a slice or a ticket

Interview until you share an understanding. Then persist the slice spec and frozen `EXPECT:`, or triage the ticket. Do not build. Do not write `tasks.md`.

Load **heio-stack** before any write under `.heio/`. Load **docs** before any write under `docs/`. Load **domain-modeling** when a term or ADR belongs in the vault.

If `AGENTS.md` or `WORKSPACE.md` already names a tracker, that file wins.

## Rounds

Same frontier format as **heio-wayfinder**. Ask the whole frontier. Recommend an answer. Wait.

A slice you cannot demo or learn from in one sitting is two slices. Split before you write.

## Slice

User names a slice on the current sprint, or this interview produces one.

1. Read intent, roadmap, and sprint `shape.md`. The slice stays inside that bet.
2. Write **Done** in words. Then immediately write oracles. If you cannot write a `CHECK:` / `EXPECT:`, stop and sharpen. The done is still mush.
3. Oracles only for external truth — user-visible, contract, data invariant, “ops can tell.” Internal design stays in TDD.
4. Stop. Summarize spec + `EXPECT:` lines. Wait for confirm.
5. Copy `templates/slice-spec.md` and `templates/slice-oracles.md` into the slice folder. Status `frozen`. `EXPECT:` is frozen. The first `CHECK:` is drafted here; the builder may refine the command later.
6. Leave `tasks.md` unwritten. Tasks exist only when the slice is `active`, via **heio-slice**.

Done when `spec.md` and `oracles.md` exist, every oracle has `CHECK:` and `EXPECT:`, status is `frozen`, and `tasks.md` is absent.

## Ticket

User names a ticket, or an inbound signal that is not yet a ticket.

1. If no file exists, copy `templates/ticket.md` into `.heio/tickets/`.
2. Interview only enough to triage. The solution does not live on the ticket.
3. Same rule every time:
   - Fits the active slice → **TASK**. Status `promoted`. If `tasks.md` exists, append the task line. If it does not, name the line for **heio-tasker**.
   - Fits the project, not this slice → **TICKET**. Status `parked`.
   - Changes the bet → **ESCALATE**. Stop. Hand to **heio-wayfinder**.

Done when the ticket has a status and a verdict.

## Loop

End with `VERDICT: TASK | TICKET | ESCALATE | VERIFY`. Planning a slice is not VERIFY. VERIFY waits for `--reverify` on an active slice.
