---
name: roadmap-audit
description: Roadmap audit of one ROADMAP.md row. Use when the user wants /audit-roadmap, /afk-roadmap, an AFK-style production audit, to verify a done Loop item, continue the language audit, or file tickets for false greens, missing tests, or checker gaps. Not for implementing a Loop item.
---

# Roadmap audit

Findings-only sitting. Exactly **one row** from `ROADMAP.md`, then exit. Tests are truth. Tickets are the output. Do not implement. Do not edit `ROADMAP.md`.

Load **management** before any write under `.heio/`. Load **to-tickets** before filing. Load **draconic-language** for fixture, native-observation, and hard-error rules. Load **docs** only for ADRs the row touches.

This is not **draconic-loop**. Loop implements. This sitting audits.

## Bars

Both bars are Named, Fetchable, and Comparable. Fetch the files. Do not grade from memory.

- **Claim bar** — the row in `ROADMAP.md`. Status holds only if the Tests paths exist and are green on the listed Targets. `native` / `both` means program results (`native.stdout`), not B08 `hello\n`.
- **Production bar** — `CONTEXT.md` plus `.heio/planning/intent.md`. A team can use this feature without leaving Draconic: JS emit where the row is `js`/`both`, native binary where the row is `native`/`both` or a sibling N-row already claims it, Checker depth where the row is a **T** item. The Checker is TypeScript-inspired, not tsc. Untyped JS staying permissive is not a gap.

Do not treat an **E** row listed `js` as a native miss when an **N08** (or other N) sibling covers the same fixtures. Audit that sibling when the cursor reaches it.

## Arguments

From the command or the user message:

- Empty or `continue` — next unaudited row in `ROADMAP.md` document order
- A Roadmap ID (`B01`, `E01.01`, `T07.03`) — that row, even if already in the ledger
- `status` — print ledger progress and the next ID. Do not test. Do not file.

If the ID does not exist, stop and say so.

## Ledger

Campaign log is one round: `.heio/planning/rounds/rounds-<NN>-roadmap-audit.md`. Find it by slug `roadmap-audit` under planning and archive. If missing, copy **management** `templates/round.md`, allocate `<NN>` with:

```
node .opencode/skills/management/scripts/planning-next-id.mjs
```

`sitting_kind: planning`. `status: ready-to-resume` until every Roadmap row has a ledger line, then `published`.

Body (no tables):

```markdown
## Cursor
next: B01

## Ledger
- **B01**: HOLD. `cargo test -p draconic-lexer` ok. no tickets
```

Update `next` to the following unaudited ID after each sitting. Re-audit overwrites that ID's ledger line.

## Hard rules

- One row. Do not start the next.
- Findings, not fixes. No compiler, fixture, or ROADMAP status edits.
- No evidence → `UNKNOWN`, not a guess. Cite `ROADMAP.md` quote, `path:line`, command output, or a ticket wikilink.
- Never stage `.heio/`.
- Do not run `DRACONIC_TEST262_FULL=1` or `cargo test --workspace` for this sitting. Those are Loop / oracle gates, not a row audit.
- Remainder parents **E17.02** and **E18.44** with existing parked tickets are an **honest remainder**. Do not mint a duplicate. Do not mark them done.

## Steps

### 1. Resolve the row

Parse `ROADMAP.md` pipe rows. ID shape: letter + two digits + optional `.digits` groups (`B01`, `N08.01.04.01`). Columns: ID, Status, Targets, Item, Tests.

Read the ledger. Pick the argument ID, or the first row whose ID is not in `## Ledger`.

Read the full Item text and Tests cell. List child IDs that share this prefix plus a further `.`. List sibling N/T rows that name this ID or the same fixture path.

**Done when:** one ID, status, targets, tests paths, and sibling IDs are written down.

### 2. Inventory evidence

Read the Tests paths. For conformance, glob `tests/conformance/fixtures/**` and the matching `tests/conformance/tests/<area>.rs`. Read each fixture `.meta` (`targets`, `native.stdout`, `js.check`, `js.error`). For crate rows, glob that crate's `#[cfg(test)]` modules.

Disconfirm first:

- Fixture or crate test named in Tests is missing
- `done` with no test that can fail
- `targets: native` in meta missing, or `native.stdout` is `hello` / absent on a non-empty Program
- Parent `done` while a child is `todo` and is not an honest remainder with a ticket
- Open or archived ticket already names this ID

**Done when:** every Tests path is found or listed missing, and every `.meta` native claim is noted.

### 3. Run the Tests column

Resolve cargo from the Tests cell. Do not invent a filter that matches nothing. Typical map:

- `crates/draconic-<name>` → `cargo test -p draconic-<name>`
- `tests/conformance` + area `es/expressions` → `cargo test -p draconic-conformance --test expressions`
- One fixture id → the area `--test` plus the `<stem>_runs` filter when that test exists
- `tests/integration` → the integration crate or `draconic-cli` tests that name the item

Run it. Record command, exit code, and the failing assertion line. Timeout → `UNKNOWN` with the command named. Kill a hang; do not wait out the sitting.

If Targets include `js` and a `.drac` fixture exists, also `draconic build --target js` on one representative file. If Targets include `native` or `both`, also `draconic build --target native` on that file. `check` / `compiler` targets use crate tests and `draconic check` when the item is the Checker or `U04`.

**Done when:** every Tests path has a command result, or a named blocker.

### 4. Score the row

Assign one sitting verdict:

- **HOLD** — claim bar holds. Production bar holds or is owned by a named sibling row.
- **GAP** — false green, missing tests, failing command, hello-stub native, or production miss with no sibling.
- **HONEST-TODO** — status is `todo`/`blocked` and a ticket already names the remainder. No new ticket.
- **UNKNOWN** — could not run or could not find evidence.

Spawn one `explore` subagent only to falsify a `done` HOLD (stub-green, missing fixture, duplicate ticket). It is read-only. It does not file. It does not implement.

**Done when:** verdict is set and every gap is a one-line claim plus citation.

### 5. File tickets

Only for **GAP**, and only after a search of `.heio/planning/tickets/` and `.heio/archive/planning/tickets/` for that Roadmap ID and fixture path.

One ticket per independent problem. Two failing asserts in the same test are one ticket. A cluster-wide native miss files once on the cluster parent, not on every child.

Copy **management** `templates/ticket.md`. Re-run `planning-next-id.mjs` immediately before each write. `status: open`. `ticket_type: bug` for false greens and missing tests. `ticket_type: feature-request` for a missing sibling capability. `ticket_type: observation` when the claim holds but honesty is thin.

Headings: `## Signal`, `## Fit`, `## Notes`. Fit is "this project, later slice". Notes cite Roadmap ID, command, and `path:line`. Do not write an Agent Brief. Do not publish tasks. Do not shape a sprint.

**Done when:** each gap is a new ticket, a wikilink to an existing ticket, or an explicit "not filed because duplicate of [[id]]".

### 6. Close the sitting

Append or replace the ledger line. Point `next` at the next unaudited ID. Leave the round `ready-to-resume`.

Chat report, short prose:

- Row ID, status, targets
- Verdict
- What was run and what happened
- Ticket ids filed, or none
- Next ID

End with:

```
VERDICT: TICKET | VERIFY
EVIDENCE: <row id> <HOLD|GAP|HONEST-TODO|UNKNOWN> <ticket ids or none>
```

`VERIFY` when HOLD or HONEST-TODO. `TICKET` when any ticket was filed this sitting.

**Done when:** ledger line exists, chat report names the next ID, and no second row was started.

## Do not

- Load **draconic-loop** or mark a row `in_progress` / `done`
- Load **tdd** to add tests
- Promote tickets into slices or tasks (that is a later planning sitting)
- File "needs tsc" or "drop-in TypeScript migration"
- File "needs native" on an E-row whose N08 sibling already exists on the Roadmap
