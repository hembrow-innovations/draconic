---
name: behaviour-contracts
description: How a project locks down what features are meant to do. Contracts under docs/ state plain-language promises. A gate such as check:contracts locks the ones with a test pointer. Use when reading or writing a contract.md, adding or changing a feature's intended behaviour, deciding whether a change is allowed, wiring the check, or backfilling contracts onto existing features.
---

# Behaviour contracts

The durable, readable statement of what each feature must do, plus a gate that stops it silently drifting. Part of the intent system (purpose → contracts → data-flows). Do not resurrect `requirements/design/tasks` triad files.

Contracts live in `docs/`. Load the **docs** skill. Discover the ladder there. Typical notes are named intent-system, behaviour-contracts, data-flows, and a feature `{purpose,contract}` pair. Do not treat one project's guide path as the only law.

If `AGENTS.md` or `WORKSPACE.md` already names a tracker (`.scratch/`, `docs/planning/`, GitHub Issues), that file wins. Then default to this pack.

- The lock. Discover the checker. `check:contracts` is an example, often wired into a broader gate.
- Context pack. Discover the project's pack command before coding. **vault-pack** / `pnpm vault:pack` is an example.

## Before behaviour work

1. Resolve `area` (unit frontmatter or feature folder).
2. Discover and run the project's context pack for that area and task phrase.
3. Read Must-read paths in full. At least purpose + contract(s) for that area, plus the intent-system note if one exists.
4. Name **promise ids** you will keep or change. Ladder empty → stop. File an issue through **management**. Never invent product rules.

## Core rule

You can't lock prose. Prose rots. Lock **behaviour** in tests. Keep one thin `contract.md` per unit stating what/why (not how), wired to those tests so it can't drift silently. Two jobs: **readable** (a human understands the promises) and **locked** (deviating takes a deliberate, visible edit, not a silent code change).

## Anatomy

A `contract.md` has `tags: [contract]` frontmatter and promises grouped by area:

```md
- `feature.section:promise-id`: One plain-language promise a human would state.
  test: {substring of a real it/test/describe title}   ← makes it LOCKED
  test: {another case. one promise, many tests}
- `feature.section:other`: A promise with no test yet.  ← ASSERTED (visible TODO)
```

- **id.** Namespaced `feature.section:name`, globally unique. Stable target for inheritance/overrides (`except records-table:delete`).
- **`test:` pointer present ⇒ locked.** The checker verifies each pointer matches a real `it/test/describe` title. No pointer ⇒ **asserted** (skipped. an honest "promised, not yet proven").
- **`contract_default: locked`** in frontmatter cranks the whole file rigid. Every promise must then be locked or the gate fails. Loose by default, fully enforceable on demand, per promise or per file. Use on **small, high-stakes** files only (sharing, session), not large inventory contracts.

## Wording (anti-drift)

- Falsifiable: subject + obligation + boundary. Ban *should / generally / as needed / flexible / intuitive / helps users*.
- Optional forbids: `feature.section:forbid-…` for agent-bait anti-patterns.
- Lock high-risk promises first. Assert missing product forbids before freestyle.

## Grain (the thing people get wrong)

- Promise = a unit of **intent** (something a human would list). Test = a unit of **verification** (one case). Different axes → **one promise, many tests**.
- Same promise / more cases → more `test:` lines, **not** more promises.
- A promise that reads like a test ("returns 7 when week") is mis-grained. Raise it to intent, push cases into `test:` lines.
- Put a contract at any **coherent unit of functionality**, not one-per-feature. A feature is a folder of finer contracts. Skipping levels breaks nothing.
- Large features: section by concern (`data`, `ui`, …) when natural, not by framework. Small features: one `contract.md` is enough.
- Feature **purpose** (`purpose.md`) is separate: job and non-goals only. Do not put React Query / file layout in contracts or purpose. That is data-flows.

## When to touch a contract

- **Changing behaviour = editing a promise line.** That's the anti-"fix it so it just works" mechanism. You can't quietly break a promise, only openly change it, a visible diff to flag in review. If a change makes a locked promise false, either the test fails (good, the gate caught it) or you must deliberately edit the promise (and say why).
- **New behaviour → write the promise first** (contract-first TDD): promise → red test → green impl → harden with support tests → gate. Support tests need **not** map to a promise. Test-linking is evidence, not law.
- **Never** rename/delete a test that a promise points at without updating the contract. The gate will fail.

## Inheritance (archetypes). Navigation, not computed

An archetype is a contract fragment naming a reusable promise-set. A finer contract inherits by **linking** `[[records-table]]`, then: inherit all (just link), extend (add promises), inherit part (`except records-table:delete`, a visible opt-out), or override. Obsidian backlinks make an archetype show every consumer and an ADR show every promise realising it. **The checker never resolves this.** It only enforces promises physically present in a file. Inheritance is for humans/navigation. No archetype exists until a real repeated pattern earns it. YAGNI.

## Portability

The **kernel** (contract/promise, asserted/locked, scope hierarchy, subtractive archetype inheritance, tree+backlink navigation, dumb checker, paved-path) carries to any project unchanged. The **dialect** (scope-level names, the archetype library) is per-project. Port the grammar, fill in the words. The kernel names no routes/tabs/tables.

## Honest scope

Does **not** make bugs impossible. Guarantees: no *known* promise breaks silently, and every promise is enumerated. Can't invent promises nobody thought of. A bug in *unpromised* behaviour still needs a human to add a promise. Real metric: shrinking *unpromised surface*, not "zero bugs".

## Running the gate

Discover the project's contract checker. `check:contracts` is an example. A broader JS gate may include it.

Green prints `N locked, M asserted`. Failure lists each unlocked/broken promise and exits 1.
