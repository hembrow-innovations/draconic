# Wayfinder

The tree will not fit in one sitting. Chart fog on a **round** file with `sitting_kind: wayfinder`. Tickets capture decisions that still need a sitting. Do not write slices or tasks from wayfinder.

Load **management** before any write under `.heio/`. Load **docs** only when a settled decision should survive a clone. Placement, numbers, status, and close-moves live in **management**.

This branch plans. Each ticket resolves a decision. The map is done when nothing is left to decide.

## Refer by name

Every note has a `title`. In narration, refer to it by that title wrapped around `[[id]]`.

## The notebook is a round

One round at `.heio/planning/rounds/rounds-<NN>-<slug>.md`. `sitting_kind: wayfinder`. Status `awaiting-answers` while charting, `awaiting-confirm` when the frontier is empty, `published` when a later sitting wrote the map artifacts.

Copy the management round template. Keep these extra sections under Confirm:

```markdown
## Objectives

<the destination. one or two lines.>

## Decisions so far

- [[ticket-01-slug|title]]. <one-line gist of the answer>

## Not yet specified

<in-scope fog you cannot ticket yet>

## Out of scope

<work ruled beyond this destination>
```

The round is an index. A decision lives in its ticket. The round gists and links.

Find open wayfinder tickets by scanning `.heio/planning/tickets/` for `wayfinder` tags that `references` this round.

## Tickets

Each ticket is `.heio/planning/tickets/ticket-<NN>-<slug>.md`. Copy the management ticket template.

- `tags` include `wayfinder`
- `references` lists the round id
- `blocked_by` lists blocking ticket ids
- Put the question in Signal. Leave the solution off the ticket

A ticket is unblocked when every id in `blocked_by` is `closed` or `dropped`. The frontier is open, unblocked children of this round. First by number wins.

Create tickets first, then wire `blocked_by`.

## Ticket types

Every ticket is HITL (worked with the human) or AFK (agent alone). A HITL ticket only resolves through that live exchange.

- **research** (AFK). Load **research** if installed. Write findings on the ticket under `## Notes`.
- **prototype** (HITL). Load **prototype** if installed. Link the artifact from the ticket.
- **planning** (HITL). Return to the parent skill. Default type.

Do not mint execution tasks from wayfinder. That is a later planning sitting.

## Fog of war

Do not chart what you cannot yet see. **Not yet specified** is in-scope fog that is not sharp enough to ticket. **Out of scope** is past the destination.

Ticket when you can state the question precisely, even if it is blocked. Leave it in fog when you cannot phrase it that sharply.

## Chart the map

1. Name the destination. Run the parent skill's frontier once, and **domain-modeling**.
2. Map the frontier, breadth-first. If this surfaces no fog, you do not need a map. Stop and ask how they want to proceed.
3. Create the round. Destination in Objectives. Fog in Not yet specified. Decisions so far empty.
4. Create the tickets you can specify now. Wire `blocked_by` in a second pass.
5. Fire research tickets in parallel.
6. Stop. Charting hand-resolves nothing.

## Work through the map

Never resolve more than one ticket per session, except research tickets.

1. Load the round. Not every ticket body.
2. If the user named a ticket, use it. Otherwise take the first frontier ticket.
3. Resolve it. Load the skills the round names.
4. Write the answer on the ticket. Set `closed`. Append one gist line to Decisions so far.
5. Add newly surfaced tickets. Graduate fog that is now specifiable. If a ticket sits past the destination, set `dropped`, move it to `.heio/archive/planning/tickets/`, and add one line to Out of scope.
6. If the decision invalidates other tickets, update or close them.

Move a terminal ticket with **obsidian-axi** `mv` the day it closes. Keep the filename.

## When the way is clear

The map is done when no open wayfinder tickets remain and Not yet specified is empty.

Return to Confirm on the parent skill. If someone should now build, that is a planning sitting that publishes slices and tasks. Do not reuse this round as the execution pool.
