---
id: "ticket-01-roadmap-honesty-pass"
title: "Roadmap honesty pass"
kind: ticket
status: closed
tags: []
created_at: "2026-07-26T00:00:00Z"
updated_at: "2026-07-26T00:00:00Z"
---

# Roadmap honesty pass

Archived from `docs/planning/issues/closed/issues-1-roadmap-honesty-pass.md`.

# Roadmap honesty pass

## Description

Audit clusters marked `done` on [[ROADMAP]] for real gaps (thin fixtures, native stubs, half-implemented builtins, missing edge cases). Reopen or split Roadmap rows where coverage is dishonest.

“Done” today means green fixtures for the Tests column, not full semantics. Blindly starting Phase 2 on a false complete baseline will hide debt.

## Affected

- `ROADMAP.md` (E / T / N / U clusters)
- `tests/conformance` fixtures
- Native backends / Runtime where stubs remain

## Observed

Mega-loop marked ~214 items done with ~199 fixtures. Parent clusters may over-claim vs item text.

## Impact

False completeness; Phase 2 work built on gaps that look closed.

## Proposed Fix

1. Walk each E/T/N/U parent cluster; note gaps vs item text.
2. For each material gap: add a child `todo` row or file a linked issue with evidence.
3. Short report: clusters trusted vs reopened.
4. No silent deletions of ECMA-262 obligations (split or `blocked` with reason only).

## Agent Brief

### Goal

Produce an honesty audit of the Roadmap: every parent cluster (and material child rows) either trusted as genuinely covered by its Tests column, or corrected via new/split `todo` rows (or linked vault issues) with evidence. Do not invent a new phase structure here — only correct the baseline.

### Contract

- **Done means tests green for the item text**, not “a fixture exists nearby.” Compare each Roadmap item’s **Item** column to what its **Tests** paths actually assert (js and/or native as listed under **Targets**).
- Material gap = item claims behavior that has no fixture observation, only a stub/fallback path, or coverage that is thinner than the item wording (e.g. parent `done` while child obligations missing).
- Prefer **split into child rows** with status `todo` over flipping a whole cluster back to `todo` when only part is weak.
- Never delete ECMA-262 obligations; use `blocked` only with an explicit reason.
- Preserve historical B/E/T/N/U IDs; new rows get new IDs in the existing numbering style (e.g. `E01.04.10`, `N07.05`).
- Leave a short audit report in-repo (planning note or section under Comments on this issue) listing: trusted clusters, reopened/split IDs, and deferred-with-reason items.

### Acceptance criteria

1. Every E/T/N/U **parent** cluster has an explicit audit outcome: `trusted` | `split` | `reopened` | `blocked-with-reason`.
2. At least one material gap (if any exist) is reflected as a new or status-changed Roadmap row with `todo` (or a linked open issue if the gap is multi-unit).
3. `rg '\| todo \|' ROADMAP.md` is non-empty **or** the audit report states with evidence that no material gaps were found (unlikely; stubs and thin fixtures should be checked).
4. No ECMA-262 obligation rows deleted; `cargo test --workspace` still green if any code/fixtures changed (prefer Roadmap-only edits unless a fixture rename is required for honesty).
5. Report is linkable from this issue’s Comments.

### Out of scope

- Defining Phase 2 section names (see [[issues-7-new-roadmap-phase]]).
- Wiring Test262 ([[issues-2-test262-deeper-conformance]]).
- Implementing missing language features beyond filing/splitting Roadmap rows.
- Large refactors ([[issues-6-architecture-cleanup]]).

## Audit report (2026-07-26)

Evidence sources: `ROADMAP.md`; `tests/conformance/fixtures/**/*.meta` (`native.stdout`); `crates/draconic-backend-llvm/src/lib.rs` (`emit_hello_stub` fallback for non-native/Promise/eval shapes).

Fixture census: **184** entry metas; **29** real native stdout (async 9 + eval 3 + native ints/floats/layout 17); **~150** native paths assert only B08 `hello\n`; **5** js-only (N04 policy).

### Legend change

Added **Native observations** rule: `Targets: native`/`both` requires program-result assertions on native, not hello-stub-only.

### Parent cluster outcomes

| Parent | Outcome | Notes |
|--------|---------|--------|
| **B01–B10** | `trusted` | B08 honestly “stub + hello”; B09 GC hello in runtime tests; B10 CLI e2e. |
| **E00** | `trusted` | Harness runs js + native runners. |
| **E01–E11** | `split` | JS fixtures match item text; `Targets` narrowed `both`→`js` (hello-stub native was not real). Native debt → **N08.01–N08.11**. |
| **E12** | `trusted` | Real `native.stdout` on all async fixtures; backed by **N06**. |
| **E13–E15** | `split` | Same as E01–E11; native → **N08.12–N08.14**. |
| **E16** | `trusted` | Real native on eval fixtures; backed by **N07**. |
| **E17** | `split` | Only `with` fixtures existed under broad “non-strict legacy” wording. **E17.01** done (`with`); **E17.02** todo (other legacy). Native → **N08.15**. |
| **E18** | `split` | 43 tracked children keep `done` on **js**; parent wording no longer claims “full 262 gaps done.” **E18.44** todo = untracked remainder. Native → **N08.16**. |
| **T01–T05** | `trusted` | Erase/happy-path fixtures match “annotations / structural / unions / generics / native types” as stated (thin but not over-claiming full TS). |
| **T06** | `split` | JS boundary checks real; native was hello-only → `Targets`→`js`; native → **N08.17**. |
| **T07** | `todo` (new) | Negative typechecking (reject ill-typed) missing entirely. |
| **N01–N05** | `trusted` | Real native stdout / js-policy error paths. |
| **N06–N07** | `trusted` | Real native for Promise/async and eval/Function. |
| **N08** | `todo` (new) | Cluster + children for real native ES observations off hello stub. |
| **U01–U03** | `trusted` | Tooling crate tests. |

### Roadmap deltas (no IDs deleted)

| Kind | IDs |
|------|-----|
| `Targets` `both`→`js` | E01–E11 (+children), E13–E15 (+children), E17–E18 (+children), T06 |
| New `done` | E17.01 |
| New `todo` | E17.02, E18.44, T07, N08, N08.01–N08.17 |
| Wording | E17, E18 parent; legend native-observations bullet |

Post-edit counts (approx): **215** `done`, **21** `todo`.

### Deferred (not reopened here)

| Item | Reason |
|------|--------|
| Full Test262 surface | [[issues-2-test262-deeper-conformance]] |
| Phase 2 section structure | [[issues-7-new-roadmap-phase]] |
| Depth of individual JS fixtures (edge cases within a child row) | Prefer Loop splits when a specific gap is found; E18.44 is the catch-all obligation |
| Architecture / LLVM multipath | [[issues-6-architecture-cleanup]] children |

### Acceptance checklist

1. Every parent has outcome above — **yes**
2. Material gaps as `todo` rows — **yes** (N08*, E17.02, E18.44, T07)
3. `rg '\| todo \|' ROADMAP.md` non-empty — **yes** (~21)
4. No ECMA-262 rows deleted; Roadmap-only edits — **yes**
5. Report on this issue — **this section**

## Comments

> *This was generated by AI during triage.*
>
> **2026-07-26 triage:** Confirmed not already implemented — Roadmap is 214/214 `done`, 0 `todo`; LLVM still has hello-stub fallback for unsupported shapes; internal fixtures only (no Test262). Category `enhancement`. Moved to `ready-for-agent` with Agent Brief. No prior out-of-scope match.

> **2026-07-26 implement:** Honesty pass complete. See **Audit report** above. `ROADMAP.md` updated (targets narrowed, N08/E17/E18.44/T07 todos). No code/fixture changes.
