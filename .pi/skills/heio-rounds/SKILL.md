---
name: heio-rounds
description: File-backed planning or wayfinder sitting. Start writes the next numbered file. Resume advances unanswered, next round, or confirm. Does not publish.
disable-model-invocation: true
---

# File-backed sitting

A second door next to **heio-planning** and **heio-wayfinder**. The sitting file holds the grill. Prompts: **heio-rounds-start**, **heio-rounds-resume**.

Load **heio-stack** before any write under `.heio/`. Load **docs** only when a later sitting publishes. This skill does not publish.

If `AGENTS.md` or `WORKSPACE.md` already names a tracker, that file wins.

First argument `planning` or `wayfinder` → Start. Otherwise → Resume.

## Status

`awaiting-answers` → `ready-to-resume` → `awaiting-confirm` → `published`. `parked` is a side door.

Stop at `awaiting-confirm`. Do not publish: no frozen slices, no task-pool write, no intent / roadmap / sprint-shape write. The sitting is not `published`.

## Questions

Write questions into the sitting file. Do not call `ask_user_question`. A short pointer to the path is enough in the transcript.

The frontier is every decision whose prerequisites are already settled. Ask the whole frontier in one round, at most 4 questions.

Each question is multiple choice:

- **2–4 options**: short label (1–5 words), plus what choosing it means
- **recommended first**: append `(Recommended)` to that label

Finding facts is your job. Decisions are the user's. A question that depends on another still open in this round belongs later.

## Start

Prompt **heio-rounds-start**. Argument: `planning` or `wayfinder`, then an optional slug. That first token is `sitting-kind`.

1. Scan `.heio/planning/rounds/` for the next unused integer, zero-padded two digits. Start at `01`. Do not reuse a number.
2. Slug is the argument slug, or a short kebab from the sitting title. Lowercase kebab-case. No `round-` prefix.
3. Copy `templates/round.md` from **heio-stack** to `.heio/planning/rounds/<NN>-<slug>.md`.
4. Set `id` to the file stem, `kind: round`, `sitting-kind` to `planning` or `wayfinder`, status `awaiting-answers`.
5. Write Round 1 questions for that sitting-kind. Stop.

Planning: destination first when intent or locations are missing. Otherwise this sprint's tracer bullets (slice Done, oracles, `blocked-by`, AFK or HITL).

Wayfinder: destination first. That round includes nothing that hangs off it.

Done when `.heio/planning/rounds/<NN>-<slug>.md` exists with `sitting-kind`, status `awaiting-answers`, and Round 1 questions.

## Resume

Prompt **heio-rounds-resume**. Argument: file name `01-slug` or `01-slug.md`. Resolve to `.heio/planning/rounds/<stem>.md`. Missing file → stop.

Read the file. The current round is the last `## Round N`. Answers are present when status is `ready-to-resume` or that round's Answers list has user content (not blank, not only template placeholders).

### Unanswered

Answers are not present. Leave status `awaiting-answers`. Do not append a round. Point at the file.

Done when status is `awaiting-answers` and no new `## Round N` was added.

### Next round

Answers are present and the frontier is not empty. Append `## Round N` with Questions and Answers. Set status `awaiting-answers`.

Planning: destination, then tracer bullets, fog last.

Wayfinder: destination, then locations, fog, current sprint as a named grouping.

Done when the previous answers remain, a new Round heading with questions is in the file, and status is `awaiting-answers`.

### Confirm

Answers are present and the frontier is empty. Fill **Confirm** with:

- Destination
- In-slices: each Done + `EXPECT:` (wayfinder: none)
- Tracer-bullet list: title, slice, blocked-by, AFK or HITL

Set status `awaiting-confirm`. Stop.

Done when status is `awaiting-confirm` and the Confirm block has destination / in-slices / tracer-bullet. Do not publish: no frozen slices, no task-pool write.

## Loop

End with `VERDICT: TASK | TICKET | ESCALATE | VERIFY` per **heio-stack**. Start and resume are TASK. This sitting is not VERIFY.
