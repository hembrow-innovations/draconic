---
id: "slice-661-b03-parse-cli-spawn"
title: "B03 CLI parse spawn in Tests column"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-07T06:25:24Z"
updated_at: "2026-09-07T20:20:00Z"
---

# B03 CLI parse spawn in Tests column

## Why

Roadmap B03 is already `done` for `draconic parse` printing an AST dump. The named Tests column is the CLI crate, and that crate still has no binary spawn of the parse verb. This cut locks the CLI wiring so deleting `cmd_parse` cannot stay green.

## Done

The CLI crate Tests column spawns the `draconic` binary with `parse` on a valid Program file. The process exits 0 and prints an AST dump that starts with `Program`. Promise `toolchain.cli:parse-ast` points at that spawn test. B03 stays `done`.

## Blocked by

None.

## Non-goals

- **Reopening B03** as a new language Loop atom
- **Changing parse CLI behaviour** unless the spawn test proves `cmd_parse` is broken
- **Install-smoke or release-binary PATH coverage** (sibling, not this Tests column)
- **New parse flags, help titles, or error-path encyclopaedia**
- **Website or Learn/Reference copy**

## Oracle checklist

- [x] O1: CLI crate Tests column spawns `draconic parse` on a Program file
  CHECK: cargo test -p draconic-cli --test parse
  EXPECT: test result: ok.
  EVIDENCE: `cargo test -p draconic-cli --test parse` → test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s. B03 remains done.

## Pool

Durable links to task ids. Never drop them.

- `[[task-662-b03-parse-cli-spawn]]`

## See also

[[ticket-588-b03-parse-cli-untested]] ROADMAP.md B03 [[Toolchain purpose]] [[Toolchain — Contract]] [[Toolchain tests]]
