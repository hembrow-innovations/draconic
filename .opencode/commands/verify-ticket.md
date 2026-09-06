---
description: Forensic audit of one management ticket
argument-hint: "<ticket-NN-slug>"
---

# Verify ticket

Forensic audit of exactly one tracker ticket under `.heio/planning/tickets`. Goal: surface what is true, false, incomplete, or unproven, so a suspicious ticket cannot survive on vibes. Evidence only. No edits unless 100% sure it is right.

Load **management** if layout is unclear. Load **docs** for committed truth. Do not load **triage** to change state. This prompt never flips status or tags.

**management** owns layout and status. Do not start a second tree.

## Hard rules (anti-hallucination)

1. **Every finding cites evidence.** Acceptable citations:
   - Ticket quote: `ticket §heading. "…"` (short verbatim)
   - Code: `path:line` (or symbol name + path if line unstable)
   - Doc: vault path or `[[wikilink]]` + heading
   - Command output: command + relevant line(s)
2. **No evidence → `UNKNOWN`, not a guess.** If search was incomplete, say what was searched and stop.
3. **Never invent** files, symbols, tests, contracts, or behaviour. If the tree does not contain it, it does not exist for this report.
4. **Disconfirm first.** Prefer searches that would falsify the ticket (already fixed, already rejected, wrong surface, contradicts ADR or purpose).
5. **Read-only by default.** No note edits, no commits, no "helpful" fixes.
6. **One ticket.** Do not expand into a tree-wide cleanup. Related notes are context only. Cite them. Do not re-triage them.

## Arguments

`$ARGUMENTS` (from `/verify-ticket`) or the user message:

- Ticket id (`657`, `ticket-657`), slug fragment, path, or `[[wikilink]]`
- Empty → ask once for the target. Do not pick a ticket yourself

Resolve the file under `.heio/planning/tickets/` and `.heio/archive/planning/tickets/`. Prefer exact id. Fall back to content search.

If zero or many matches, stop and ask. Include `.heio/archive/planning/tickets/` in the search. A "suspicious" ticket may already be closed or dropped.

## Process

Complete each step. A step is done only when its completion criterion holds.

### 1. Snapshot the note

Read the **entire** note (frontmatter + body + Comments + Agent Brief). Pull every `[[wikilink]]` that is a parent, blocker, contract, purpose, or plan. Read those too.

Inventory:

- Identity: `id`, `status`, `tags`, `labels` / category, `severity`
- Claimed problem or opportunity (one sentence, quoted from the note)
- Factual claims list (numbered). Every assertion about code, UX, data, or prior art
- Acceptance criteria / Agent Brief criteria (each as a row)
- Dependencies (`blocked_by`, parent, children)

**Done when:** claims list is exhaustive against the note text (no silent skips).

### 2. Structural / tracker critique

Check the note as a tracker artefact. Cite **management** and **triage** norms only when flagging.

- Frontmatter vs body (status/tag contradictions, missing category, a `ready` task without `## Agent Brief`)
- Terminal status still living under `.heio/planning/tickets` (not moved to `.heio/archive/planning/tickets/`)
- Broken or empty wikilinks
- Agent Brief quality against `AGENT-BRIEF.md` in **triage** if present: behavioural not procedural, testable AC, scope fences, intent/promise ids when behaviour changes, no stale path/line recipes
- AC testability: each criterion independently verifiable or mark **vague**

**Done when:** every structural defect is listed with a citation, or "none".

### 3. Verify each claim (forensic core)

For **each** numbered claim from step 1, assign exactly one result:

- **CONFIRMED.** Evidence supports the claim.
- **REFUTED.** Evidence contradicts the claim.
- **PARTIAL.** True in part. Spell the boundary.
- **UNKNOWN.** Could not prove or disprove after real search.

How to gather evidence (pick what fits; skip irrelevant):

- **Code:** grep/glob by **domain concept** (glossary terms), not only the reporter's wording. Read the actual implementation path end-to-end.
- **Contracts / purpose:** discover under `docs/` via **docs**. Name promise ids if they exist.
- **ADRs / glossary:** discover under `docs/`. Flag term or decision collisions.
- **Out-of-scope / prior rejection:** search `docs/` for rejection notes (see **triage** `OUT-OF-SCOPE.md`).
- **Duplicates / prior art:** search `.heio/planning/tickets/` and `.heio/archive/planning/tickets/` for the same concept.
- **Bug repro:** if steps exist, trace code and run the cheapest honest check (unit test, targeted command, or read-only query). If environment blocks repro, result is **UNKNOWN** with blocker named. Never "probably true".
- **Already implemented:** if behaviour already ships, mark claim **REFUTED** (as a gap) and record where it lives.

**Done when:** every claim has exactly one result + evidence block. Zero claims left implicit.

### 4. Adversarial critique (is this ticket worth doing?)

Answer with evidence, not taste:

1. **Real?** Is there a user-visible or system-visible problem or opportunity?
2. **Right shape?** Bug vs enhancement mis-tagged? One unit or should fan out?
3. **Right layer?** Matches architecture notes under `docs/`, or proposes the wrong surface?
4. **Conflicts?** Purpose out of scope, ADR, contract promise, or project defaults contradicted? Discover those defaults under `docs/` or `AGENTS.md`.
5. **Scope disease?** Extras, multiple independent units smuggled in, untestable "make it nice" criteria?
6. **Agent hazard?** Would an AFK run likely thrash (missing decisions, environment secrets, design judgment, external accounts)?
7. **Cheaper alternative?** One-liner, config, docs-only, or close as duplicate/dropped with pointer?

**Done when:** each question has a one-line answer + citation or UNKNOWN.

### 5. Ship hazards (if implemented as written)

Mandatory even when claims are CONFIRMED. Assume an AFK agent follows the note literally (Agent Brief + AC + "What to build") and stops at the first green interpretation.

List **3 to 8 concrete failure modes**, each with:

- **What the agent likely does wrong** (one line)
- **Why** (citation: other throw site, missing seam, contract lock, package boundary, test gap, or AC that passes while user-visible goal fails)
- **Blast radius** (false green / prod footgun / wrong layer / silent incomplete)

Cover at least, when applicable:

- AC satisfied at the named surface while **other paths** still break the goal
- Over-broad softening (swallow real errors, fake clients, global factory change)
- Contract, auth, or row-level regressions. Discover the project's gates. `check:contracts` is an example, not the only paved path.
- Timeout, retry, or status-signal races with a blocked follow-up ticket
- Surfaces the brief out-scopes but AC still implies (desktop SPA, mobile, SSR vs CSR)

If the ticket is docs-only or close/dropped, write `None. Not an implementation unit.`

**Done when:** every hazard is specific to this codebase (no generic "tests might be weak") or the docs-only line is used.

### 6. Report (chat only)

Emit exactly this structure. Keep prose tight. No preamble.

```markdown
# Ticket verification. <id>. <title>

**Target:** `.heio/planning/tickets/...` or `.heio/archive/planning/tickets/...`
**Status/tags:** …
**Verdict:** SOLID | WEAK | SUSPECT | FALSE | UNKNOWN
<!-- SOLID = claims hold, shape is executable
     WEAK = real but underspecified / brief gaps
     SUSPECT = major claim unproven or contradicted in part
     FALSE = core claim refuted (already done / not real / rejected)
     UNKNOWN = blocked on evidence (env, missing repro, etc.) -->

## Executive read
- 3 to 6 bullets. What a suspicious maintainer needs. No new claims beyond sections below.

## Claim ledger
- **1. <short claim>** CONFIRMED|REFUTED|PARTIAL|UNKNOWN. Evidence: `path:line` or quote
- **2. …**

## Structural defects
- … or "None."

## Already implemented / duplicate / out-of-scope
- … with pointers, or "None found (searched: …)."

## Intent & decision collisions
- purpose / contract promise ids / ADRs / glossary, or "None."

## Repro / runtime checks
- What was run, what happened. Or "Not applicable." / "Not run. <blocker>."

## Adversarial critique
- Numbered answers to the seven questions in step 4.

## If an agent implements as written. Top failure modes
- **1. <mode>.** Why (`path:line` / AC gap). Blast: …
- **2. …**
- Or: `None. Not an implementation unit.`

## If you keep it. Minimum fixes to the note
- Concrete edits to body/brief/AC (suggestions only; do not apply).
- Or: "Close / drop because …" with evidence.

## Evidence index
- Paths, commands, and notes actually read (bullet list). Proves the search happened.
```

**Verdict bar:**

- Any **core** claim **REFUTED** → verdict ≤ **FALSE** (or **SUSPECT** if secondary only)
- Any **core** claim **UNKNOWN** after due search → verdict ≤ **SUSPECT** or **UNKNOWN**
- Brief/AC fail agent-brief bar while claims hold → **WEAK**
- Material ship hazards (false-green AC, wrong layer, silent incomplete fix) while claims hold → verdict ≤ **WEAK** even if the brief looks tidy
- Do not use **SOLID** if the Evidence index is thin or ship-hazards section is missing / generic filler

**Done when:** report is complete, every section filled, ship-hazards section present, no uncited assertions outside quotes of the ticket itself.

## Optional write-back

Only if the user says to attach or save the report: append under `## Comments` with:

```markdown
> *This was generated by AI during ticket verification.*
```

Do not change `status` or `tags` unless the user explicitly orders a triage transition (then hand off to **triage**).
