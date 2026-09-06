---
id: "contract"
title: "Host I/O — Contract"
kind: contract
description: "Durable promises for host process, stdio, fs, sockets-first HTTP, default permissive policy, and opt-in grants."
status: active
domain: draconic
area: host-io
tags: [contract]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Host I/O — Contract

A promise with a `test:` pointer is locked. One without is asserted. Purpose: [[Host I/O purpose]]. Coverage map: [[Host I/O tests]].

## Behaviour

- `host-io.process:args-env-exit`: A Program can read process args, get/set/delete env, and exit with a status on js and native.
  test: process_args_runs_js_and_native
  test: process_env_runs_js_and_native
  test: process_exit_runs_js_and_native
  test: host_process_argv_user_args
- `host-io.stdio:read-write`: stdout and stderr write strings or bytes; stdin reads a line or bounded bytes on js and native.
  test: stdout_write_string_runs_js_and_native
  test: stderr_write_string_runs_js_and_native
  test: stdin_read_line_runs_js_and_native
  test: host_stdout_write_bytes
- `host-io.fs:read-write-dirs`: A Program can read and write files, stat/exists, and create/list/remove directories on the targets that expose fs.
  test: read_file_text_runs
  test: write_file_text_runs
  test: exists_basic_runs
  test: mkdir_basic_runs
  test: host_fs_read_text_and_bytes
- `host-io.path:string-ops`: Path helpers join, normalize, dirname, basename, extname, isAbsolute, and resolve relative to cwd without doing I/O.
  test: path_join_runs_js_and_native
  test: path_normalize_runs_js_and_native
  test: path_dirname_runs_js_and_native
  test: path_resolve_runs_js_and_native
- `host-io.net:tcp-sockets-first`: Native TCP listen (including port 0), accept, connect, read/write, and loopback echo exist before HTTP helpers.
  test: tcp_listen_ephemeral_runs_native
  test: tcp_loopback_echo_runs_native
  test: host_tcp_listen_ephemeral_local_port_close
  test: emit_tcp_listen_ephemeral
- `host-io.http:thin-helpers`: Plaintext HTTP/1.1 parse/write helpers and a one-shot native server sit on those sockets; `examples/http-echo` is a pure Draconic native server.
  test: parse_get_runs_native
  test: server_oneshot_runs_native
  test: client_e2e_runs_native
  test: host_http_parse_request_line_headers_body
  test: http_echo_start_request_assert_shutdown
- `host-io.policy:js-unsupported`: JS either hard-errors a native-only host API or uses an explicit bridge row; it does not silently emit wrong code.
  test: unsupported_diagnostic_on_js_for_native_only
  test: h00_no_js_only_host_api_and_native_only_hard_errors
  test: tcp_listen_js_bridge_on_js
  test: registry_lists_h17_04_bridge_subset_both
- `host-io.permissions:default-permissive`: With no explicit grant subset, filesystem and TCP host ops already exposed succeed on those targets.
  test: default_fs_no_explicit_grant_subset
  test: default_fs_runs_both_targets
  test: default_net_no_explicit_grant_subset
  test: default_net_runs_both_targets
- `host-io.permissions:opt-in-grants`: `--allow-fs-read`, `--allow-fs-write`, `--allow-net-listen`, and `--allow-net-connect` on `draconic run` install a grant subset; listed grants succeed; a missing grant denies with a clear diagnostic.
  test: parse_run_args_allow_grant_flags
  test: help_lists_allow_grant_flags
  test: run_allow_fs_subset_honoured_js
  test: grant_fs_explicit_grant_subset
  test: deny_fs_clear_diagnostic_js
  test: allow_fs_names_cli_flags
- `host-io.forbid-deno-locked-default`: v1 does not treat a Deno-style deny-by-default as the designed default while host surfaces already succeed without grants.
  test: default_fs_no_explicit_grant_subset
- `host-io.forbid-full-browser`: Host I/O does not ship a browser engine, DOM, or page runtime.
