---
name: heio-wayfinder
description: Chart fog. Intent, locations, and the way that does not fit one planning sitting.
disable-model-invocation: true
---

# Wayfinder on heio-stack

A loose idea is too big for one sitting, and the way to the destination is not visible yet. Interview the map. Do not generate it. Do not dump the questions. Do not build.

This skill is **fog** and **map**. Slice files and the task-pool belong to a **heio-planning** sitting once the way is clear.

If this sitting surfaces no fog, stop. The work fits **heio-planning**.

Load **heio-stack** before any write under `.heio/`. Load **docs** only when a settled decision should survive a clone. Load **domain-modeling** when a term or ADR belongs in the vault.

If `AGENTS.md` or `WORKSPACE.md` already names a tracker, that file wins.

## Rounds

The frontier is every decision whose prerequisites are already settled. Ask the whole frontier in one round, at most 4 questions. Wait for the user's answers before the next round.

Talk to the user with `ask_user_question`. Do not dump questions in the transcript.

Each question is multiple choice:

- **2–4 options**: short label (1–5 words), plus what choosing it means
- **recommended first**: append `(Recommended)` to that label
- **one call**: do not stack `ask_user_question`

A custom answer is always available. Do not add an "Other" option. A question that depends on another still open in this round belongs later. Finding facts is your job. Decisions are the user's.

## Chart

User invokes with a loose idea that will not fit one sitting.

Interview first. Do not write until the user confirms.

1. Destination first. That round includes nothing that hangs off it. Why this project exists, success looks like X, we will not do Y.
2. Locations. Each bullet is a destination: this is working when. Optional `bet: try X; pivot if Y` under a location. Add bullets; do not rewrite siblings. A location file waits until a bullet needs depth.
3. Fog and out of scope. Ticket what you can phrase as a question. Leave the rest unnamed on the map.
4. Current sprint only as a named grouping, not slice files.

Stop. Summarize. Wait for confirm. Then write.

## Write

Copy templates from **heio-stack**.

- `.heio/planning/intent.md` when intent is new or the user is changing it on purpose
- `.heio/planning/roadmap.md` with the location bullets
- `.heio/planning/locations/<slug>.md` only when a bullet needs depth
- `.heio/planning/sprints/<id>/shape.md` for the current grouping. Status `shaping`. Slice files and the task-pool wait for **heio-planning**.

Done when intent, roadmap, and the fog are on disk, every in-grouping has a one-line why, and this pass wrote no slice files and no task-pool files.

## Loop

End with `VERDICT: TASK | TICKET | ESCALATE | VERIFY` per **heio-stack** `rules/loop.md`. Charting the way is not VERIFY. Newly surfaced work that is not this map is a TICKET. A rewrite of a location destination during a workflow is ESCALATE and waits.
