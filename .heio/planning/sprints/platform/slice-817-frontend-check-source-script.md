---
id: "slice-817-frontend-check-source-script"
title: "Frontend check_source stays Script"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-12T00:15:00Z"
updated_at: "2026-09-12T16:40:00Z"
---

# Frontend check_source stays Script

## Why

`parse_source` retries Module after Script. `check_source` and `compile_source` stay Script-only, which [[architecture-frontend]] and [[architecture-pipeline]] already require. The split looked accidental in audit. This cut locks it with tests so Embed and single-buffer check do not silently become Module.

## Done

`check_source` and `compile_source` parse and check as Script. `check_source_module` and `compile_source_module` remain the Module string APIs. `parse_source` may still retry Module for fmt and dump tools. Frontend crate tests green.

## Blocked by

None.

## Non-goals

- **retry Module on check_source / compile_source**
- **changing path load_program Script-then-Module policy**
- **Linker / compile_path**
- **E17.02 / E18.44**: do not mark done

## Oracle checklist

- [x] O1: frontend tests lock Script-only string check
  CHECK: cargo test -p draconic-frontend --offline
  EXPECT: test result: ok.
  EVIDENCE: cargo test -p draconic-frontend --offline → test result: ok. 17 passed; 0 failed. Script string APIs reject export and TLA; Module string APIs accept TLA; parse_source still retries Module.

## Pool

- [[task-818-frontend-check-source-script]]

## See also

[[ticket-808-frontend-check-source-script-only]], [[architecture-frontend]], [[architecture-pipeline]], [[ticket-780-frontend-cli-bypass]]
