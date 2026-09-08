---
id: "contract"
title: "Toolchain — Contract"
kind: contract
description: "Durable promises for CLI parse/check/build/run/repl/fmt, Frontend, Embed eval, and LSP."
status: active
domain: draconic
area: toolchain
tags: [contract]
created_at: "2026-09-06"
updated_at: "2026-09-08"
---

# Toolchain — Contract

A promise with a `test:` pointer is locked. One without is asserted. Purpose: [[Toolchain purpose]]. Coverage map: [[Toolchain tests]].

## Behaviour

- `toolchain.cli:parse-ast`: `draconic parse` accepts a Program file and prints an AST dump for a valid Program.
  test: parse_sample_program
  test: parse_valid_program_exits_zero_dump_starts_with_program
  test: fresh_path_draconic_parse_hello
- `toolchain.cli:check-no-emit`: `draconic check` typechecks and binds a Program with no emit, exits zero on success, and exits non-zero on parse, bind, or type errors.
  test: help_lists_check_command
  test: check_ok_source_exits_zero_no_emit
  test: check_parse_error_exits_nonzero
  test: check_type_error_exits_nonzero
  test: check_bind_error_exits_nonzero
  test: check_watch_reruns_on_source_change
- `toolchain.cli:build-targets`: `draconic build --target js|native` compiles a Program to runnable JS or a native binary; `--target` is required; native rejects JS-only Programs instead of emitting silent wrong code.
  test: parse_build_args_requires_target
  test: build_target_js_writes_runnable_js
  test: build_target_native_writes_runnable_binary
  test: build_target_native_rejects_unsupported_js
  test: e2e_js_build_artifact_runs
  test: e2e_native_build_artifact_runs
  test: build_watch_rebuilds_on_source_change
- `toolchain.cli:build-scratch-name`: When `-o` is omitted, `draconic build` writes `{stem}.out.js` for js and `{stem}.out` for native beside the input.
  test: default_output_paths
  test: build_js_default_output_next_to_source
  test: build_native_default_output_next_to_source
- `toolchain.cli:run-execute`: `draconic run` builds and executes a Program; default target is js; `--target native` runs a native binary; remaining tokens are program argv; a shebang-shaped path invokes run.
  test: help_lists_run
  test: parse_run_args_defaults_js_and_forwards
  test: run_target_js_executes_console_log
  test: run_defaults_to_js
  test: run_target_native_executes_scalar
  test: shebang_bare_path_runs_like_run
  test: e2e_run_js_console_log
- `toolchain.cli:repl`: `draconic repl` reads, evaluates, and prints; multi-line input completes before eval; last expression value prints; `--target embed` evaluates through Embed.
  test: help_lists_repl
  test: repl_prints_last_expression_value
  test: repl_multiline_function_then_call
  test: repl_embed_target_prints_expression
- `toolchain.cli:fmt`: `draconic fmt` rewrites a Program in place to a stable style and is idempotent; `--check` fails when the file would change.
  test: help_lists_fmt_command
  test: fmt_rewrites_messy_source_in_place
  test: fmt_is_idempotent
  test: fmt_check_fails_when_unformatted
  test: fmt_idempotent_basic_bindings
- `toolchain.frontend:facade`: Callers compile through Frontend. A Script path does not link. A Module entry with import/export links the graph. Callers do not assemble parser, Checker, and IR stages by hand.
  test: compile_source_script
  test: compile_path_script_skips_link
  test: compile_path_module_links_import
  test: frontend_compile_mixed_relative_and_module_path
- `toolchain.embed:eval`: Embed compiles and evaluates simple expression strings (literals, arithmetic, unary, grouping, `typeof` on primitives) on the native eval path.
  test: eval_number_literal
  test: eval_arithmetic_add
  test: eval_direct_eval_expression_cases
- `toolchain.lsp:analysis`: LSP analysis reports type diagnostics, hover types on bindings, and go-to-definition from use to declaration.
  test: lsp_type_diagnostic_on_bad_annotation
  test: lsp_hover_and_goto_definition_roundtrip
  test: diagnostics_on_type_error
  test: hover_on_binding_shows_number
  test: goto_definition_from_use_to_decl
- `toolchain.cli:forbid-learn-site-content`: The Toolchain CLI and this vault do not author the public Learn site; that site is authored from `website/` ([[specs/draconic/public-site/purpose]]), not this vault or the CLI.
