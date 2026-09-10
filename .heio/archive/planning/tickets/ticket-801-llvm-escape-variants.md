---
id: "ticket-801-llvm-escape-variants"
title: "LLVM escape_llvm_string copies differ"
kind: ticket
status: closed
ticket_type: observation
tags: []
blocked_by: []
created_at: "2026-09-10T19:16:04Z"
updated_at: "2026-09-11T19:30:00Z"
---

# LLVM escape_llvm_string copies differ

## Signal

Closed. Six source texts still differ, but every byte 0–255 matches existing `escape_llvm_bytes` in `emitter.rs`. [[task-790-emitter-escape]] wrapped that helper as the one `escape_llvm_string`. No language-behavior change.

## Fit

Unknown until triage. Then one of:

- this slice → promote a decision so [[task-790-emitter-escape]] can keep one helper in `emitter.rs`
- this project, later slice → park
- would rewrite a location destination during a workflow → escalate

## Notes

- **37 copies**: private `fn escape_llvm_string` in llvm adapter files. `emitter.rs` has `escape_llvm_bytes` only.
- **Variant A (10 files)**: printable `(0x20..0x7f)`, `format!` hex. `es_arrays.rs`, `es_call_spread.rs`, `es_classes.rs`, `es_destructure_defaults.rs`, `es_object_destructure.rs`, `es_objects.rs`, `es_tagged_template.rs`, `host_process_async.rs`, `host_signals.rs`, `host_tcp_async.rs`.
- **Variant B (5 files)**: same as A plus `&& c != b'\\'`. `es_class_expr_name.rs`, `es_eval.rs`, `es_nullish.rs`, `host_process.rs`, `host_subprocess.rs`.
- **Variant C (1 file)**: `0x20..=0x7e` plus `write!`. `es_promise.rs`.
- **Variant D (1 file)**: A with `write!` instead of `format!`. `es_static_blocks.rs`.
- **Variant E (1 file)**: explicit `0x07`/`0x08`/`0x09`/`0x0a`/`0x0c`/`0x0d` arms. `es_var_for.rs`.
- **Variant F (19 files)**: `with_capacity` plus `&& c != b'"'`. `host_atomics.rs`, `host_cancel.rs`, `host_channels.rs`, `host_dns.rs`, `host_docs.rs`, `host_fs.rs`, `host_http.rs`, `host_http2.rs`, `host_http_server.rs`, `host_once.rs`, `host_os.rs`, `host_path.rs`, `host_tcp.rs`, `host_time.rs`, `host_timers.rs`, `host_udp.rs`, `host_workers.rs`, `host_ws.rs`, `host_ws_e2e.rs`.

## Parent

[[slice-783-llvm-one-emitter]] [[task-790-emitter-escape]]
