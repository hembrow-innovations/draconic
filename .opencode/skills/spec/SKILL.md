---
name: spec
description: How a project states what a feature is for, what it must do, and which tests lock that. Spec folders hold purpose.md, contract.md, and test.md. Use when reading or writing a spec folder, purpose.md, contract.md, or test.md; implementing against a spec; mapping tests to promises; wiring a contract checker; backfilling contracts; or deciding whether a change is in scope.
---

# Spec. Living product intent

A spec is a **folder**, not a file. Purpose, contracts, and a test map live together. Humans walk domain → area → feature. Agents glob stable filenames and read one concern.

You can't lock prose. Lock **behaviour** in tests. `contract.md` states what/why. `test:` pointers lock it.

This skill is the kernel for any project that uses that shape. It does not name this repo's features.

Load **docs** for vault layout, frontmatter, wikilinks, and templates. Copy purpose, contract, test, and spec skeletons from the **docs** skill `templates/`. Load **tdd** when writing the code test. Working tickets live in **management**, not here.

If `AGENTS.md` already names a different spec layout, that file wins. Do not start a second tree.

Per-rule detail lives in `rules/<prefix>-*.md`. Copy-ready skeletons live in the **docs** skill `templates/`.

## Before spec work (always)

1. Discover the layout. `AGENTS.md` wins. Default is `docs/specs/`.
2. Find the area by `purpose.md`, `contract.md`, or `test.md`.
3. Read purpose first. Out of scope is a hard fence.
4. Name promise ids you will keep or change. Empty ladder → stop.

Full steps: `rules/disc-first.md` and `rules/ladder-read.md`.

## When to apply

- Finding, reading, or writing a spec folder
- Adding or updating `purpose.md`, `contract.md`, or `test.md`
- Implementing against a spec, or deciding whether a change is in scope
- Mapping code tests to promises, wiring the checker, or backfilling contracts

## Prefer / careful / do not

### Prefer

- **disc-first** before any read or write
- **ladder-read** before coding
- **purpose-job** when the note is purpose
- **contract-anatomy** when the note is contract.md or a promise will move
- **test-map** when listing or adding coverage
- **docs** for vault path, frontmatter, wikilinks, and templates
- **tdd** for the red test that locks a promise
- **principle-intent-ladder-stop** if it is installed and the ladder is empty

### Careful

- **spec-<slug>.md** is optional how/shape narrative inside the folder. It is not the living product spec.
- **test.md** narrates the suite. `test:` pointers on the contract lock it.

### Do not

- Invent product rules when purpose and contracts do not answer
- Flatten specs into `docs/specs/*.md` or one `spec-<slug>.md` when the folder shape applies
- Put framework, file trees, or React Query in purpose or contracts
- Create living `web/{requirements,design,tasks}.md` triad files
- Treat a working plan or ticket as a spec

## Rule categories by priority

- **1 CRITICAL** - Discover (`disc-`)
- **2 CRITICAL** - Read the ladder (`ladder-`)
- **3 HIGH** - Purpose (`purpose-`)
- **4 HIGH** - Contracts (`contract-`)
- **5 HIGH** - Folder shape (`folder-`)
- **6 HIGH** - Write order (`write-`)
- **7 HIGH** - Test map (`test-`)
- **8 HIGH** - Change (`change-`)
- **9 HIGH** - Portability (`port-`)

## Quick reference

### 1. Discover (CRITICAL)

- `disc-first` AGENTS.md wins. Glob `purpose.md`, `contract.md`, `test.md`

### 2. Read the ladder (CRITICAL)

- `ladder-read` Purpose, then contracts, then test.md. Name promise ids. Empty → stop

### 3. Purpose (HIGH)

- `purpose-job` Job, in/out of scope, surfaces, Authority. Product outcome only

### 4. Contracts (HIGH)

- `contract-anatomy` Ids, locked vs asserted, `contract_default`
- `contract-wording` Falsifiable. Forbids. Lock high-risk first
- `contract-grain` One promise, many tests. Coherent unit
- `contract-inherit` Archetypes are navigation. Checker does not resolve them
- `contract-gate` Discover the checker. Honest unpromised surface

### 5. Folder shape (HIGH)

- `folder-shape` `docs/specs/<domain>/<area>/` and optional `<feature>/`

### 6. Write order (HIGH)

- `write-order` Folder, purpose, contract, test.md, hub

### 7. Test map (HIGH)

- `test-map` test.md is how/why/gaps. `test:` pointers lock

### 8. Change (HIGH)

- `change-promise` Edit the promise first, then the test, then the code

### 9. Portability (HIGH)

- `port-kernel` Grammar travels. Dialect is per-project

## How to use

```
rules/disc-first.md
rules/ladder-read.md
rules/purpose-job.md
rules/contract-anatomy.md
rules/contract-wording.md
rules/contract-grain.md
rules/contract-inherit.md
rules/contract-gate.md
rules/folder-shape.md
rules/write-order.md
rules/test-map.md
rules/change-promise.md
rules/port-kernel.md
```

Read only the rules for the current task. Do not bulk-read `rules/`. Copy templates from **docs**.

Vault kinds other than spec folders: load **docs**.
