---
id: "test"
title: "Toolchain tests"
kind: test
description: "Which tests cover Toolchain CLI, Frontend, Embed, and LSP promises."
status: active
domain: draconic
area: toolchain
tags: [test]
created_at: "2026-09-06"
updated_at: "2026-09-07"
---

# Toolchain tests

Purpose: [[Toolchain purpose]]. Contract: [[Toolchain — Contract]].

## Coverage

These tests lock `toolchain.cli:parse-ast`, `toolchain.cli:check-no-emit`, `toolchain.cli:build-targets`, `toolchain.cli:run-execute`, `toolchain.cli:repl`, `toolchain.cli:fmt`, `toolchain.frontend:facade`, `toolchain.embed:eval`, and `toolchain.lsp:analysis`. Asserted: `toolchain.cli:forbid-learn-site-content`.

## Tests

- **crates/draconic-cli/src/main.rs** — `parse_sample_program`
  - **How:** Dumps `let x = 1 + 2;` and asserts a `Program` AST with binding `x`.
  - **Why:** Locks `toolchain.cli:parse-ast` at the parse dump used by `draconic parse`.
- **crates/draconic-cli/tests/parse.rs** — `parse_valid_program_exits_zero_dump_starts_with_program`
  - **How:** Spawns the `draconic` binary with `parse` on a tiny valid Program file; exit 0; stdout starts with `Program`.
  - **Why:** Locks `toolchain.cli:parse-ast` through the CLI verb so deleting `cmd_parse` cannot stay green.
- **tests/integration/tests/install_smoke.rs** — `fresh_path_draconic_parse_hello`
  - **How:** After install, a fresh PATH runs `draconic parse` on a hello Program.
  - **Why:** Same promise end-to-end on the shipped binary.
- **crates/draconic-cli/tests/check.rs** — `check_ok_source_exits_zero_no_emit`
  - **How:** Runs `draconic check` on valid source; exit 0; no emit artifact.
  - **Why:** Locks `toolchain.cli:check-no-emit` success path (U04).
- **crates/draconic-cli/tests/check.rs** — `check_type_error_exits_nonzero` / `check_bind_error_exits_nonzero` / `check_parse_error_exits_nonzero`
  - **How:** Each class of check failure exits non-zero.
  - **Why:** Same promise; errors must not look like success.
- **crates/draconic-cli/tests/watch.rs** — `check_watch_reruns_on_source_change`
  - **How:** `check --watch` re-runs after mtime change.
  - **Why:** Watch is a check-mode case, not a separate product.
- **crates/draconic-cli/tests/build.rs** — `build_target_js_writes_runnable_js` / `build_target_native_writes_runnable_binary`
  - **How:** `build --target js` writes JS that runs; `--target native` writes a runnable binary.
  - **Why:** Locks `toolchain.cli:build-targets` (B10).
- **crates/draconic-cli/tests/build.rs** — `build_target_native_rejects_unsupported_js`
  - **How:** Native build of a JS-only Program fails instead of emitting wrong code.
  - **Why:** Native-only / JS-only hard-error rule on the CLI path.
- **tests/integration/tests/cli_build.rs** — `e2e_js_build_artifact_runs` / `e2e_native_build_artifact_runs`
  - **How:** Built artifacts execute with expected output.
  - **Why:** Same promise beyond the CLI crate tests.
- **crates/draconic-cli/src/main.rs** — `parse_build_args_requires_target`
  - **How:** Build args without `--target` error.
  - **Why:** Target is required, not defaulted in parse.
- **crates/draconic-cli/tests/run.rs** — `run_target_js_executes_console_log` / `run_defaults_to_js` / `run_target_native_executes_scalar`
  - **How:** `run` executes js (default) and native Programs.
  - **Why:** Locks `toolchain.cli:run-execute` (U14).
- **crates/draconic-cli/tests/run.rs** — `shebang_bare_path_runs_like_run`
  - **How:** A script-shaped path without a subcommand runs like `run`.
  - **Why:** Shebang-friendly CLI.
- **crates/draconic-cli/tests/repl.rs** — `repl_prints_last_expression_value` / `repl_multiline_function_then_call` / `repl_embed_target_prints_expression`
  - **How:** REPL prints last value, accepts multi-line, embed target evaluates.
  - **Why:** Locks `toolchain.cli:repl` (U08).
- **crates/draconic-cli/tests/fmt.rs** — `fmt_is_idempotent` / `fmt_rewrites_messy_source_in_place`
  - **How:** Format once, format again; messy source is rewritten.
  - **Why:** Locks `toolchain.cli:fmt` (U05).
- **tests/integration/tests/fmt.rs** — `fmt_idempotent_basic_bindings`
  - **How:** Formatter is idempotent on binding fixtures.
  - **Why:** Same promise at integration grain.
- **crates/draconic-frontend/src/lib.rs** — `compile_source_script` / `compile_path_script_skips_link` / `compile_path_module_links_import`
  - **How:** Script compile skips link; Module entry with import links.
  - **Why:** Locks `toolchain.frontend:facade`.
- **tests/packages/tests/k06_03_coexist_relative.rs** — `frontend_compile_mixed_relative_and_module_path`
  - **How:** Frontend compiles a mix of relative and module-path imports.
  - **Why:** Facade still owns Script versus link when packages are in the graph.
- **crates/draconic-embed/src/lib.rs** — `eval_number_literal` / `eval_arithmetic_add` / `eval_direct_eval_expression_cases`
  - **How:** Embed eval returns numbers and arithmetic; direct-eval cases hold.
  - **Why:** Locks `toolchain.embed:eval` (N07.01).
- **tests/integration/tests/lsp_basics.rs** — `lsp_type_diagnostic_on_bad_annotation` / `lsp_hover_and_goto_definition_roundtrip`
  - **How:** Bad annotation is a diagnostic; hover and go-to-definition round-trip.
  - **Why:** Locks `toolchain.lsp:analysis` (U06) end-to-end.
- **crates/draconic-lsp/src/lib.rs** — `diagnostics_on_type_error` / `hover_on_binding_shows_number` / `goto_definition_from_use_to_decl`
  - **How:** Unit analysis of diagnostics, hover, and definition.
  - **Why:** Same promise without the integration harness.

Support tests (not extra promises): `verbose_version_contains_required_fields`, `help_lists_doc_command`, `help_lists_extract_command`, `help_lists_bindgen_command`, `help_lists_test_command`, `help_lists_get`. Those verbs belong to other areas or are unpromised here.

## Gaps

- No test yet for promise `toolchain.cli:forbid-learn-site-content`. Public site generation is covered under `website_pipeline` tests and [[0010-public-docs-draconic-ssg]]; this folder does not point at those titles so the fence stays asserted.
- `draconic parse` has dump, CLI spawn, and install-smoke coverage; there is no `help_lists_parse` title. Usage text in `print_usage` is not a test title.
