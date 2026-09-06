---
id: "purpose"
title: "Conformance purpose"
kind: purpose
description: "Product brief: job, scope, and fences for the Conformance harness, Test262 staging, and workspace oracle."
status: active
domain: draconic
area: conformance
tags: [purpose]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Conformance purpose

## Job

Prove language completeness with tests: load fixtures, run them on js and native, and treat a Roadmap item as done only when those tests are green on the applicable targets.

## In scope

- **Harness**: discover `*.drac` fixtures (optional `*.meta`), run on js and native runners ([[CONTEXT]] Conformance suite, E00).
- **Done bar**: a Roadmap item is `done` only when its listed tests pass ([[0006-mega-loop-roadmap-tests]], [[ROADMAP]]).
- **CLI runner**: `draconic test` loads a fixture path or directory.
- **Test262 staged**: curated allowlist on js only; suite optional; skip when missing; failures are report-only until triage ([[0007-test262-staged-roll-in]]).
- **Workspace oracle**: `cargo test --workspace` is the workspace check when that is the promise; default CHECK budget is ten minutes; `[profile.test]` stays line-tables-only ([[0012-oracle-check-timeout]]).

## Out of scope

- **Claiming full Test262 pass**: v1 is harness plus allowlist, not the entire suite on day one, and not native Test262.
- **Inventing Roadmap rows from every Test262 failure**: promote clusters after triage.
- **Language feature meaning**: fixture contents pin ECMA-262 and native types; those promises belong to the language spec slice.
- **Narrowing workspace CHECK to `--lib --bins` to beat a timeout**: that drops conformance and is rejected ([[0012-oracle-check-timeout]]).

## Surfaces

- **`tests/conformance`**: fixture tree and runners.
- **`draconic test <path>`**: developer runner.
- **`tests/test262`**: allowlist, optional vendored suite, baseline report.
- **`cargo test --workspace`**: workspace oracle with ten-minute CHECK budget.

## Authority

- Behaviour: [[Conformance — Contract]]
- Tests: [[Conformance tests]]
- Glossary: [[CONTEXT]]
- Decisions: [[0006-mega-loop-roadmap-tests]], [[0007-test262-staged-roll-in]], [[0012-oracle-check-timeout]]
- Completeness: [[ROADMAP]], [[overview-completeness]]

## Open product questions

- (none)
