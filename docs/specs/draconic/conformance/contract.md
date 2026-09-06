---
id: "contract"
title: "Conformance — Contract"
kind: contract
description: "Durable promises for the js+native harness, staged Test262, CLI test runner, and workspace oracle."
status: active
domain: draconic
area: conformance
tags: [contract]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Conformance — Contract

A promise with a `test:` pointer is locked. One without is asserted. Purpose: [[Conformance purpose]]. Coverage map: [[Conformance tests]].

## Behaviour

- `conformance.harness:js-and-native-runners`: The harness loads fixtures under `tests/conformance/fixtures`, each fixture declares js and/or native, and declared targets run green for the smoke slice that exercises both runners.
  test: discovers_fixtures_under_fixtures_dir
  test: each_fixture_declares_js_and_or_native
  test: run_all_fixtures_green_on_declared_targets
  test: smoke_empty_runs_both_targets
  test: smoke_let_add_runs_js_only
- `conformance.harness:fixture-meta`: Fixture sidecars parse id, targets, js/native expectations, and optional grant subsets.
  test: parse_meta_smoke
  test: parse_meta_native_real_stdout
  test: parse_meta_grants
- `conformance.cli:test-runner`: `draconic test` runs a fixture directory or a single `.drac` file, exits non-zero when a js check fails, and is listed in help.
  test: help_lists_test_command
  test: test_runs_passing_fixture_dir
  test: test_runs_single_fixture_file
  test: test_fails_when_js_check_fails
- `conformance.test262:staged-js-allowlist`: Test262 runs a curated allowlist on js only; when the suite is absent, allowlist entries skip and CI stays green; the baseline report records totals.
  test: allowlist_loads_and_has_entries
  test: missing_suite_skips_all
  test: default_run_does_not_fail_ci_without_suite
  test: markdown_report_mentions_totals
- `conformance.loop:done-iff-tests-green`: A Roadmap item is done only when its tests are green on applicable targets. The Loop does not mark done on LLM judgment alone.
- `conformance.workspace:oracle-timeout-budget`: Workspace completeness uses `cargo test --workspace` as the oracle when that is the promise. Default CHECK timeout is ten minutes. `exit=timeout` with `match=yes` is a budget miss, not a hang. Do not drop conformance by rewriting CHECK to `--lib --bins`.
- `conformance.forbid-full-test262-pass`: This area does not claim the entire official Test262 suite passes.
