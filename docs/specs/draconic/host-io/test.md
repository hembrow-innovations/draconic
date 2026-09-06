---
id: "test"
title: "Host I/O tests"
kind: test
description: "Which tests cover host process, stdio, fs, sockets, HTTP, and permission promises."
status: active
domain: draconic
area: host-io
tags: [test]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Host I/O tests

Purpose: [[Host I/O purpose]]. Contract: [[Host I/O — Contract]].

## Coverage

These tests lock `host-io.process:args-env-exit`, `host-io.stdio:read-write`, `host-io.fs:read-write-dirs`, `host-io.path:string-ops`, `host-io.net:tcp-sockets-first`, `host-io.http:thin-helpers`, `host-io.policy:js-unsupported`, `host-io.permissions:default-permissive`, `host-io.permissions:opt-in-grants`, and `host-io.forbid-deno-locked-default`. Asserted: `host-io.forbid-full-browser`.

## Tests

- **tests/conformance/tests/host_process.rs** — `process_args_runs_js_and_native` / `process_env_runs_js_and_native` / `process_exit_runs_js_and_native`
  - **How:** Fixtures under `host/process` run on js and native with expected argv, env, and exit.
  - **Why:** Locks `host-io.process:args-env-exit` (H01).
- **crates/draconic-runtime/src/host_abi_tests.rs** — `host_process_argv_user_args`
  - **How:** Runtime ABI exposes user argv.
  - **Why:** Same promise at the native Runtime boundary.
- **tests/conformance/tests/host_stdio.rs** — `stdout_write_string_runs_js_and_native` / `stderr_write_string_runs_js_and_native` / `stdin_read_line_runs_js_and_native`
  - **How:** Stdio fixtures write and read on both targets.
  - **Why:** Locks `host-io.stdio:read-write` (H02).
- **tests/conformance/tests/host_fs.rs** — `read_file_text_runs` / `write_file_text_runs` / `exists_basic_runs` / `mkdir_basic_runs`
  - **How:** Fixtures read/write text, exists, and mkdir.
  - **Why:** Locks `host-io.fs:read-write-dirs` (H04).
- **tests/conformance/tests/host_path.rs** — `path_join_runs_js_and_native` / `path_resolve_runs_js_and_native`
  - **How:** Path string fixtures run on both targets.
  - **Why:** Locks `host-io.path:string-ops` (H03).
- **tests/conformance/tests/host_tcp.rs** — `tcp_listen_ephemeral_runs_native` / `tcp_loopback_echo_runs_native`
  - **How:** Native TCP listen on port 0 and loopback echo.
  - **Why:** Locks `host-io.net:tcp-sockets-first` (H06).
- **crates/draconic-runtime/src/host_abi_tests.rs** — `host_tcp_listen_ephemeral_local_port_close`
  - **How:** Runtime bind, query local port, close.
  - **Why:** Same socket promise at ABI grain.
- **crates/draconic-backend-llvm/src/host_tcp.rs** — `emit_tcp_listen_ephemeral`
  - **How:** LLVM emit classifies TCP listen.
  - **Why:** Backend lowering matches the native host symbols.
- **tests/conformance/tests/host_http.rs** — `parse_get_runs_native` / `server_oneshot_runs_native` / `client_e2e_runs_native`
  - **How:** HTTP/1.1 parse, one-shot server, client e2e on native.
  - **Why:** Locks `host-io.http:thin-helpers` (H10).
- **tests/integration/tests/http_echo.rs** — `http_echo_start_request_assert_shutdown`
  - **How:** Build and start `examples/http-echo`, GET, assert status/body, shutdown.
  - **Why:** Success Program is pure Draconic native HTTP (H17.02).
- **crates/draconic-check/src/host_api.rs** — `unsupported_diagnostic_on_js_for_native_only` / `h00_no_js_only_host_api_and_native_only_hard_errors` / `registry_lists_h17_04_bridge_subset_both`
  - **How:** Checker registry marks native-only APIs and the H17.04 bridge subset; JS gets a hard diagnostic for unsupported host use.
  - **Why:** Locks `host-io.policy:js-unsupported` (H00).
- **tests/conformance/tests/host_policy.rs** — `tcp_listen_js_bridge_on_js`
  - **How:** JS fixture for TCP listen uses the explicit bridge rather than silent wrong emit.
  - **Why:** Same policy promise on the conformance runner (H06.06 / H17.04).
- **tests/conformance/tests/permissions.rs** — `default_fs_no_explicit_grant_subset` / `default_fs_runs_both_targets` / `default_net_no_explicit_grant_subset`
  - **How:** Fixtures with empty grants still perform fs and net on both targets.
  - **Why:** Locks `host-io.permissions:default-permissive` and `host-io.forbid-deno-locked-default` (R02.04).
- **crates/draconic-cli/src/main.rs** — `parse_run_args_allow_grant_flags`
  - **How:** The four `--allow-*` flags parse into `fs-read`, `fs-write`, `net-listen`, `net-connect`.
  - **Why:** Locks CLI grant installation (R02.03).
- **crates/draconic-cli/tests/permissions.rs** — `run_allow_fs_subset_honoured_js` / `help_lists_allow_grant_flags`
  - **How:** `draconic run` with allow flags honours the subset; help lists the flags.
  - **Why:** Same opt-in grant promise on the CLI.
- **tests/conformance/tests/permissions.rs** — `grant_fs_explicit_grant_subset` / `deny_fs_clear_diagnostic_js` / `allow_fs_names_cli_flags`
  - **How:** Explicit grants succeed; missing grant is a clear diagnostic; fixtures name the CLI flags.
  - **Why:** Locks `host-io.permissions:opt-in-grants` (R02.01–R02.03).

Support tests (not extra promises): TLS wrap, HTTP/2 preface, WebSocket frames, UDP, DNS, subprocess, signals, and JS Node polyfills in `host_js_bridge.rs`. Those exist on native (or as bridges) after the sockets-then-HTTP job; they do not expand this folder into a browser.

## Gaps

- No test yet for promise `host-io.forbid-full-browser`. The fence is ADR and purpose only.
- Permission grants are locked for the four named flags. There is no Deno-compatible permission UX test, and none is promised.
- Default policy is permissive; deny-by-default is not a missing v1 feature, it is a rejected default.
