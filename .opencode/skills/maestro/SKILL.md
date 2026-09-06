---
name: maestro
description: Drive React Native UI tests with Maestro CLI. Use when writing, running, or debugging Maestro flows, YAML under .maestro/ or e2e/mobile, testID selectors, emulator or simulator verification, or flaky mobile E2E. Triggers on maestro test, launchApp, tapOn, inputText, runFlow, hideKeyboard.
metadata:
  version: "1.0.0"
---

# Maestro (React Native)

Progressive rules for Maestro device E2E on React Native and Expo. Discover the repo first. Read only rule files that match the task.

## Discover first

1. Confirm the CLI: `which maestro` then `maestro --version`. If missing, tell the user how to install and stop.
2. Find existing flows: `.maestro/`, `maestro/`, `e2e/maestro/`, `tests/e2e/mobile/`, or YAML with `appId:`, `tapOn:`, `launchApp`.
3. Read README, package scripts, justfile, and CI for how flows run.
4. Copy existing naming, folder layout, `appId`, and launch args.

If none exist, propose `.maestro/` and wait before creating files.

## Stack caveats

**Assumes:** React Native or Expo. Maestro reads the native accessibility tree. No npm Maestro package inside the app.

**Prefer:** the project's E2E script; `testID` as Maestro `id:`; a release APK or IPA; `assertVisible` as the wait; one user-visible journey per flow; `runFlow` for shared login when the suite already does that.

**Careful:** Expo Go needs `openLink`, not a custom `appId` launch. `hideKeyboard` sends Android back. Debug builds hang `inputText`. Device loopback is not the host. iOS nested views need `accessible` flipped.

**Do not introduce:** Detox, Appium, Cypress, or freestyle `adb` or `expo` boot as the primary path. Do not commit process screenshots. Unit tests stay on Jest and RNTL (`react-testing`).

A project-local **maestro** skill owns `appId`, seed users, and oneshot scripts when present.

## When to apply

Writing or debugging Maestro YAML. Device, emulator, or sim verification. "Run on device." Flaky mobile E2E. Adding `testID`s so a flow can land.

## Priority bands

- **1 Discover (CRITICAL)**: `disc-`
- **2 Run (CRITICAL)**: `run-`
- **3 Pitfalls (CRITICAL)**: `pitfall-`
- **4 Selectors (CRITICAL)**: `sel-`
- **5 Input and keyboard (HIGH)**: `input-`
- **6 Sync (HIGH)**: `sync-`
- **7 React Native (HIGH)**: `rn-`
- **8 Flows (HIGH)**: `flow-`
- **9 Debug and artifacts (HIGH)**: `artifact-` `debug-` `flake-`
- **10 Env (MEDIUM-HIGH)**: `env-`
- **11 Assert (MEDIUM)**: `assert-`
- **12 Platform (MEDIUM)**: `plat-`
- **13 Layer (LOW)**: `layer-`

## Quick reference

**disc-:** `disc-cli-on-path` install then stop · `disc-project-layout` find flows and appId · `disc-match-conventions` copy the suite

**run-:** `run-prefer-project-script` just or npm e2e · `run-canonical-cli` maestro test · `run-single-flow-first` one YAML · `run-studio-optional` authoring only

**pitfall-:** `pitfall-no-detox-appium` no second stack · `pitfall-no-freestyle-boot` no ad-hoc adb or expo

**sel-:** `sel-testid-over-text` id from testID · `sel-add-testid-in-app` fix the app · `sel-index-for-collisions` duplicate copy · `sel-regex-dynamic-copy` greetings · `sel-point-last-resort` coordinates last · `sel-nested-ios-accessible` RN iOS nesting

**input-:** `input-no-hidekeyboard` back on Android · `input-presskey-enter` submit · `input-debug-deadline` release build

**sync-:** `sync-assert-not-sleep` assertVisible waits · `sync-scroll-until-visible` below the fold

**rn-:** `rn-release-build` not Metro debug · `rn-expo-go-openlink` Expo Go · `rn-launch-clear-state` isolation

**flow-:** `flow-one-behavior` one journey · `flow-runflow-setup` shared login · `flow-appid-from-existing` never invent · `flow-tags-platform` android and ios tags

**artifact, debug, flake:** `artifact-gitignored` tmp only · `debug-reproduce-minimize` smallest red · `flake-harden-not-sleep` selectors and sync

**env-:** `env-device-not-host` 10.0.2.2 or LAN

**assert-:** `assert-observable-ui` user-visible outcomes

**plat-:** `plat-ios-when-named` not default · `plat-dual-both-installed` both apps first

**layer-:** `layer-unit-not-maestro` Jest and RNTL · `layer-journeys-only` not variants

## How to use

1. Discover the repo (CLI, flows, scripts).
2. Pick 1 to N rule ids (higher priority first).
3. `Read` only `rules/<id>.md` (relative to this skill directory).
4. Do **not** bulk-read `rules/` or load all of `AGENTS.md` unless stuck or asked.
5. Reviewing or refactoring: walk categories top-down until covered.

## Full reference

Upstream Maestro docs and compiled notes: `AGENTS.md` (reference only; prefer `rules/` + this router).
