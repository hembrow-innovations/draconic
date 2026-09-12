---
id: "ticket-929-package-tests-false-green"
title: "Package tests lock one-file consume, not authoring a library"
kind: ticket
status: open
ticket_type: observation
tags: [packages, tests]
blocked_by: []
created_at: "2026-09-13T12:00:00Z"
updated_at: "2026-09-13T12:00:00Z"
---

# Package tests lock one-file consume, not authoring a library

## Signal

Roadmap K and K10 read done. The tests that back them check a one-file greet lib and, for the in-repo examples, mostly file layout. They do not lock authoring a multi-file library, subpath fetch e2e, or the asserted no-registry promise.

## Fit

Unknown until triage. Spec Gaps already name `packages.forbid-central-registry`. Not a missing test for intentional non-goals (init, publish command, diamond versions) unless those promises are asserted.

## Notes

- `docs/specs/draconic/packages/test.md` Gaps: `packages.forbid-central-registry` has no test. Contract has no `test:` pointer.
- `tests/packages/tests/k10_01_pkg_lib.rs` and `k10_02_pkg_consumer.rs` layout tests only check files and README substrings. Importable / documented-build tests copy toml plus one `index.drac` into temp git, then `compile_path` plus Node. They never run `draconic get` or `draconic build` on `examples/pkg-consumer`. `tests/packages/Cargo.toml` does not depend on the CLI crate.
- k09 / `build_packages.rs` tag a single `index.drac`.
- `packages.import:module-path-via-linker` says plus subpath. Listed tests import the package root. `resolve_package_subpath_file` is a seeded-cache unit test. Nothing fetches a tagged multi-file tree and compiles `from "github.com/org/pkg/util"`.
- `k06_03` `package_relative_internals_plus_consumer_relative` asserts an AST dump contains `add` or `scale` or `__m`. No Node run. No `draconic build`.
- Conformance has no package fixtures. Supply-chain integration is consume-and-tamper on a one-file `answer = 42` lib.
- Related false-green of K11 classify-all-false is [[ticket-927-k11-cli-false-green]].

## Parent

[[location-220-packages]]

## What to build

K10 and the packages spec promises fail when authoring a real library (multi-file, subpath, example CLI path, no-registry) is not locked.

## Blocked by

none
