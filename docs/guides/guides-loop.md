---
id: guides-loop
title: Language Loop
kind: guide
description: One atomic Roadmap item per sitting, test-first, verified with workspace tests and the draconic CLI.
domain: draconic
area: guides
tags: [guide, loop, roadmap]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Language Loop

## Overview

A Loop is one atomic Roadmap item: claim, write failing tests, implement, verify, mark `done` only when tests pass. Completeness is [[ROADMAP]] plus the Conformance suite, not model judgment and not `.heio/` occupancy ([[0006-mega-loop-roadmap-tests]], [[overview-completeness]]). Default stop is one item.

## Prerequisites

- **Read**: [[CONTEXT]], relevant `docs/adr/`, and the claimed row on [[ROADMAP]].
- **Skills**: **draconic-loop** for the atom; **gauntlet-loop** when the brief is a named implement-and-verify task.
- **Tools**: `cargo test --workspace` and the `draconic` CLI ([[guides-toolchain]], [[api-cli]]). Prefer those over ad-hoc scripts.
- **Empty-board rule**: if there is no named item and no `todo` row, stop. Do not invent work ([[AGENTS]]).

## Steps

1. **Claim**. If the brief already names a Roadmap ID, that is the item. Otherwise take the first `todo` whose blockers are satisfied. Set it `in_progress`. Exactly one `in_progress` at a time.

2. **Orient**. Read existing tests and code for that item’s Tests paths. Skim ADRs if the item touches IR, Runtime, Embed, or Dual worlds.

3. **Red**. Add or extend tests that fail for the missing behavior. Prefer crate unit tests for compiler pieces; `tests/conformance/**` for language semantics; both backends when Targets is `both`.

4. **Green**. Implement the minimum to pass. No silent subsetting of ECMA-262. If the row is too large, split into child IDs and complete only the claimed child.

5. **Verify**. Run `cargo test` for touched crates, then `cargo test --workspace`. That workspace command is the oracle surface ([[0012-oracle-check-timeout]], [[performance]], [[reliability]]). Use the `draconic` CLI when the item is CLI-facing. Do not narrow the workspace run to `--lib --bins` to dodge the timeout.

6. **Close**. Set the item `done` only if verify is green. Never `done` with failing or missing tests. If blocked on tools (for example system LLVM), set `blocked` with a reason.

7. **Stop**. One item per sitting unless the user says to continue. Do not start the next Roadmap atom.

Do not copy management `.heio/` tickets, slices, or rounds into the vault as language completeness. Promote durable outcomes into `docs/` per [[standards-docs-vault]]. `.heio/` is day-to-day occupancy, not [[ROADMAP]].

## Examples

Correct Loop:

- **Claim** `E03.05` (or whatever the next unblocked `todo` is)
- **Add** a Conformance fixture that fails
- **Implement** until `cargo test --workspace` is green on applicable targets
- **Mark** the row `done`
- **Stop**

Incorrect:

- **Marking done from reading code** with no new or existing green tests
- **Shipping one backend** and calling a `both` feature done
- **Silent wrong JS emit** for a native-only feature instead of a hard-error ([[reliability]])
- **Treating a Heio slice as the Roadmap**
- **Narrowing CHECK** from `cargo test --workspace` to `--lib --bins` because the oracle budget was missed ([[0012-oracle-check-timeout]])

## Reference

- **Decision**: [[0006-mega-loop-roadmap-tests]]
- **Checklist**: [[ROADMAP]]
- **Glossary**: [[CONTEXT]] (Loop, Roadmap, Conformance suite)
- **Oracle budget**: [[0012-oracle-check-timeout]]
- **CLI**: [[api-cli]], [[architecture-cli]]
- **Vault vs tracker**: [[standards-docs-vault]], [[overview-completeness]]
