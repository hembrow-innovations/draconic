---
name: dragons-audit
description: Combined harsh maintainability review and architecture deepening audit. Self-contained; do not load other skills.
disable-model-invocation: true
---

# Dragons Audit

One audit, two lenses: **code judo** on implementation quality, and **deepening** on module shape. Aim for a simpler implementation behind a smaller interface — testable and AI-navigable.

**Self-contained.** Do not load other skills. Do not invoke thermo-review, improve-codebase-architecture, codebase-design, grilling, domain-modeling, management, or docs. Read only this folder plus the target repo.

## Posture (always)

Be **ambitious** about structure. Search for code-judo moves: restructurings that preserve behavior while making the implementation dramatically simpler. Behavior-correct is not enough to approve.

Load first: `rules/tone-code-judo.md`, `rules/output-approval-bar.md`, `references/vocabulary.md`.

Use these terms exactly: **module**, **interface**, **implementation**, **depth**, **deep**, **shallow**, **seam**, **adapter**, **leverage**, **locality**.

## Process

### 1. Scope

YAGNI. Deepening pays off where change is happening.

- If the user named a module, subsystem, or pain point, take it.
- Otherwise walk `git log --oneline` for hot spots; if scattered, widen.

Read `AGENTS.md`, then whichever glossary exists (`CONTEXT.md` or `docs/overview/glossary.md`), and ADRs in the area (`docs/adr/` or `docs/decisions/adr/`). Repo-documented standards override Fowler flags.

### 2. Explore

Walk the scoped code. A read-only subagent is fine: give it this vocabulary and the questions below, and tell it not to load skills.

Quality lens — flag giant files, spaghetti-condition growth, hacky abstractions, missed code-judo moves, and Fowler smells that match the code.

Architecture lens — note friction:

- Understanding one concept requires bouncing between many small modules
- Modules **shallow** — interface nearly as complex as the implementation
- Pure functions extracted for testability while bugs hide in how they are called (no **locality**)
- Tightly-coupled modules leak across their **seams**
- Untested parts, or hard to test through the current interface

Apply the **deletion test** to anything suspected shallow: would deleting it concentrate complexity, or just move it? "Yes, concentrates" is the signal.

Read only relevant `rules/<id>.md`. Do not bulk-read `rules/`. Prefer CRITICAL/HIGH first. Dependency categories live in `references/deepening.md`.

### 3. Report

Write HTML and Markdown per `rules/output-artifacts.md` and `references/html-report.md`. Open the HTML. Tell the user the absolute path.

Do not propose interfaces yet. After the file is written, ask which finding or candidate to explore.

### 4. Grill

When they pick one, load `references/grilling.md` and walk that loop. Still do not load other skills.

If they want alternative interfaces for a deepened module, load `references/design-it-twice.md`.

## When to apply

- Combined quality plus architecture audit
- Harsh maintainability review of a branch or PR
- Abstraction quality, giant files, spaghetti-condition growth
- Shallow modules, bad seams, testability, AI-navigability

## Priority bands

- **1 CRITICAL.** `tone-code-judo`, `output-approval-bar`, `std-file-size`, `std-no-spaghetti`. Shallow modules that fail the deletion test.
- **2 HIGH.** Remaining `std-*`, `smell-baseline`, `remedy-preferred`, `output-priority`. Deepening with real friction.
- **3 MEDIUM.** Fowler `smell-*`, tone, artifacts. Speculative deepenings.
- **4 LOW.** Rare smells (e.g. `smell-refused-bequest`)

Prefer higher bands when time is short.

## Project caveats

Discover limits in `AGENTS.md`. Examples, not the only paved path:

- File target ≤1000 LOC (hard 1250). 1k crossing is a common default smell
- Package ownership such as `packages/<group>/<domain>/<platform>/` for features/ui/core
- Repo-documented standards override Fowler baseline flags (`smell-baseline`)

## Quick reference

### Posture and approval (CRITICAL)

- `tone-code-judo`. Ambitious simplification; delete complexity
- `output-approval-bar`. Approval criteria and presumptive blockers
- `std-file-size`. Do not cross 1k lines without strong reason
- `std-no-spaghetti`. No ad-hoc branches in unrelated flows

### Standards (HIGH)

- `std-clean-design`. Cleaner structure over "it works"
- `std-direct-boring`. Direct code over magic and thin wrappers
- `std-types-boundaries`. Casts, any, unknown, silent fallbacks
- `std-canonical-layer`. Right package; reuse canonical helpers
- `std-orchestration`. Parallelize independent work; atomic updates
- `std-review-questions`. Checklist for every meaningful change
- `std-flag-aggressively`. Patterns to escalate

### Smells (Fowler)

- `smell-baseline`. Repo overrides; judgement call only
- `smell-mysterious-name`. Name doesn't reveal role
- `smell-duplicated-code`. Same logic shape twice
- `smell-feature-envy`. Method uses another's data more
- `smell-data-clumps`. Fields that travel together
- `smell-primitive-obsession`. Primitive standing in for domain type
- `smell-repeated-switches`. Same cascade in multiple places
- `smell-shotgun-surgery`. One change scatters across files
- `smell-divergent-change`. One module edited for many reasons
- `smell-speculative-generality`. Abstraction without a real need
- `smell-message-chains`. Long a.b().c().d() walks
- `smell-middle-man`. Mostly delegates onward
- `smell-refused-bequest`. Ignores most of inheritance

### Remedies, tone, output

- `remedy-preferred`. Structural fixes to suggest
- `tone-delivery`. Direct tone and sample phrases
- `output-priority`. Finding order; high-conviction over nits
- `output-artifacts`. HTML + MD in the OS temp directory

### Architecture (this folder)

- `references/vocabulary.md`. Deep-module terms
- `references/deepening.md`. Dependency categories and seam discipline
- `references/html-report.md`. Combined report scaffold
- `references/grilling.md`. Decision tree after the user picks
- `references/design-it-twice.md`. Parallel interface alternatives

## How to use

1. Read this router. Load the three posture files named above.
2. Pick 1-N rule ids for the code in scope. `Read` only those `rules/<id>.md` files.
3. Do not bulk-read `rules/`.
4. Prefer CRITICAL/HIGH bands first.
5. Write findings per `output-priority` and `output-artifacts`; gate approve on `output-approval-bar`.
6. Architecture candidates use `references/deepening.md`. Report via `references/html-report.md`.
