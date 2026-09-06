# Product peer

The counterpart is a **product** peer, not the user. The round file is the notebook. Chat is only the ping.

Load **management** before any write under `.heio/`.

## Seats

- **Planner** owns the tree, the rounds, and **to-slices** / **to-tasks** when the frontier is empty.
- **Product** answers every round, committed `docs/`, and the current tree. `location-<NN>-<slug>.md` is the product owner.

Planner writes the round, then waits for product to answer in that file.

## Notebook

Search `.heio/planning/rounds/` and `.heio/archive/planning/rounds/` for an existing sitting on this topic. Update that file if you find one.

Otherwise copy the management round template to `.heio/planning/rounds/rounds-<NN>-<slug>.md`. `sitting_kind: planning`. Status `awaiting-answers`. Tags include `design-tree`. Keep the template headings. Never rewrite an earlier round.

## Round

Ask the whole frontier. Write the round into the notebook. Product reads `docs/` first. For each question, write the decision under that round. If `docs/` is silent, pick the smallest reversible default, say so in one line, and continue.

Planner records answers that landed only in chat, appends the next frontier, and repeats.

When the frontier is empty, product writes `Confirmed.` under Confirm. Then return to Confirm on the parent skill.

## Defaults

Product direction comes from intent and `docs/`. Irreversible work (force-push, deploy, customer messages, deleting production data) still stops.
