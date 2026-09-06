---
id: "test"
title: "Conformance tests"
kind: test
description: "Which tests cover the harness, CLI test runner, staged Test262, and honest gaps for the workspace oracle."
status: active
domain: draconic
area: conformance
tags: [test]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Conformance tests

Purpose: [[Conformance purpose]]. Contract: [[Conformance — Contract]].

## Coverage

These tests lock `conformance.harness:js-and-native-runners`, `conformance.harness:fixture-meta`, `conformance.cli:test-runner`, and `conformance.test262:staged-js-allowlist`. Asserted: `conformance.loop:done-iff-tests-green`, `conformance.workspace:oracle-timeout-budget`, `conformance.forbid-full-test262-pass`.

## Tests

- **tests/conformance/tests/harness.rs** — `discovers_fixtures_under_fixtures_dir`
  - **How:** Loads `fixtures/`, asserts non-empty, finds smoke empty and let-add ids.
  - **Why:** Locks harness discovery (E00).
- **tests/conformance/tests/harness.rs** — `each_fixture_declares_js_and_or_native`
  - **How:** Every loaded fixture has at least one of js or native.
  - **Why:** Targets are explicit, not implied.
- **tests/conformance/tests/harness.rs** — `run_all_fixtures_green_on_declared_targets`
  - **How:** `run_all` on the smoke slice; at least one js and one native run; no failures.
  - **Why:** Dual runners exist without re-running the entire tree inside this one test (workspace budget).
- **tests/conformance/tests/harness.rs** — `smoke_empty_runs_both_targets` / `smoke_let_add_runs_js_only`
  - **How:** Empty fixture runs js and native; let-add is js-only.
  - **Why:** Per-fixture target lists are honoured.
- **tests/conformance/src/lib.rs** — `parse_meta_smoke` / `parse_meta_native_real_stdout` / `parse_meta_grants`
  - **How:** Meta text parses id, targets, js.check, native.stdout, and `grants:`.
  - **Why:** Locks `conformance.harness:fixture-meta`.
- **crates/draconic-cli/tests/test_cmd.rs** — `test_runs_passing_fixture_dir` / `test_runs_single_fixture_file` / `test_fails_when_js_check_fails` / `help_lists_test_command`
  - **How:** CLI test runner on dir and file; failing js check is non-zero; help lists `test`.
  - **Why:** Locks `conformance.cli:test-runner` (U01).
- **tests/test262/src/lib.rs** — `allowlist_loads_and_has_entries`
  - **How:** Allowlist file loads a non-empty path list.
  - **Why:** Staged roll-in has a curated set (E19.01).
- **tests/test262/src/lib.rs** — `missing_suite_skips_all` / `default_run_does_not_fail_ci_without_suite`
  - **How:** Absent `third_party/test262` skips allowlist entries; default run is not a CI failure.
  - **Why:** Suite is optional; skip when missing.
- **tests/test262/src/lib.rs** — `markdown_report_mentions_totals`
  - **How:** Baseline markdown includes pass/fail/skip totals.
  - **Why:** Failures are a report, not an automatic Roadmap flood.

Per-area conformance binaries (`host_fs`, `expressions`, and the rest) lock language and host fixtures. They are not extra harness promises; they are how those other folders (and the language slice) stay green.

## Gaps

- No test yet for promise `conformance.loop:done-iff-tests-green`. The rule lives on [[ROADMAP]] and [[0006-mega-loop-roadmap-tests]], not a `#[test]` title.
- No test yet for promise `conformance.workspace:oracle-timeout-budget`. Ten minutes and `match=yes` as budget miss are [[0012-oracle-check-timeout]]. `[profile.test]` `debug = "line-tables-only"` and `incremental = false` are in root `Cargo.toml` with no test title.
- No test yet for promise `conformance.forbid-full-test262-pass`. Staging and skip-when-missing are locked; claiming the full suite passes is forbidden in purpose only.
- Test262 remains js-only, allowlist-shaped, and report-only on failure. That is the designed stage, not a hidden full-pass gap.
