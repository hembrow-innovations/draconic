---
name: playwright-cli
description: Drive Expo web and React Native Web with the playwright-cli agent browser (open, click, snapshot, screenshot). Use for interactive debugging, form filling, screenshots, and exploratory RN-web checks. Not Jest/RNTL, not Maestro device E2E, not the committed Playwright suite. Triggers on playwright-cli, npx playwright cli, expo start --web, getByTestId, data-testid.
metadata:
  version: "1.0.0"
---

# Playwright CLI (React Native Web)

Progressive rules for the agent browser CLI against Expo web and React Native Web. Discover the repo first. Read only rule files that match the task.

## Discover first

1. Resolve the binary: `playwright-cli` or `npx playwright cli` (`disc-cli-binary`).
2. Find how this repo starts web. Read the printed origin. Do not invent host or port.
3. Read README, justfile, CI, and any project `playwright-cli` skill for seed users and scratch dirs.
4. Confirm the target is a browser. Native device work is Maestro.

If there is no web target, stop. Do not invent a Playwright suite.

## Stack caveats

**Assumes:** A running web origin. Expo web or React Native Web in Chromium, Firefox, or WebKit. This CLI is `playwright-cli` / `npx playwright cli`, not `npx playwright test`.

**Prefer:** snapshot then act by ref; `testID` as `data-testid` / `getByTestId`; `--mobile` for phone chrome; gitignored scratch for screenshots; the project's start script.

**Careful:** Refs expire after navigation. Pressable often has no button role. `Platform.OS` is `web`. Expo Go native is not this CLI.

**Do not introduce:** Detox, Appium, Cypress, a committed Playwright suite the repo does not have, or Playwright against iOS or Android. Unit tests stay on Jest and RNTL (`react-testing`). Device E2E stays on Maestro.

A project-local **playwright-cli** skill owns origin, seed login, and scratch paths when present.

## When to apply

Click through Expo web. Fill a form. Capture a screenshot for a human. Inspect console or network on a live RN-web page. Not for unit tests, Maestro YAML, or committed `*.spec.ts`.

## Priority bands

- **1 Discover (CRITICAL)**: `disc-`
- **2 Stack (CRITICAL)**: `stack-`
- **3 Core loop (CRITICAL)**: `core-`
- **4 Snapshot (CRITICAL)**: `snap-`
- **5 Selectors (CRITICAL)**: `sel-`
- **6 React Native Web (HIGH)**: `rn-`
- **7 Pitfalls (HIGH)**: `pitfall-`
- **8 Nav and sessions (HIGH)**: `nav-` `session-`
- **9 Artifacts (HIGH)**: `artifact-`
- **10 Network and storage (MEDIUM)**: `net-` `storage-`
- **11 Inspect (MEDIUM)**: `inspect-`
- **12 Examples (MEDIUM)**: `example-`
- **13 Layer (LOW)**: `layer-`

## Quick reference

**disc-:** `disc-cli-binary` global vs npx · `disc-project-url` real origin · `disc-match-conventions` scratch and seed

**stack-:** `stack-web-not-native` browser only

**core-:** `core-install` @playwright/cli · `core-quickstart` open snapshot act close · `core-interact` click fill type · `core-keyboard-mouse` press wheel · `core-find` search snapshot · `core-eval` attributes · `core-raw-output` pipe

**snap-:** `snap-after-each-act` refs expire · `snap-target-refs` e15 CSS locator

**sel-:** `sel-role-testid` getByRole then getByTestId

**rn-:** `rn-expo-web-start` expo --web · `rn-testid-web` data-testid · `rn-pressable` no button role · `rn-textinput` fill · `rn-router` Expo Router paths · `rn-mobile-profile` --mobile

**pitfall-:** `pitfall-no-invent-url` · `pitfall-no-native-device` · `pitfall-no-commit-auth`

**nav- / session-:** `nav-history` · `nav-open-options` · `nav-windows-ampersand` · `session-tabs` · `session-named`

**artifact-:** `artifact-gitignored` · `artifact-snapshot-over-screenshot`

**net- / storage-:** `net-route-mock` · `storage-state`

**inspect-:** `inspect-console-requests` · `inspect-run-code` · `inspect-trace-video`

**example-:** `example-form` · `example-debug`

**layer-:** `layer-unit-not-cli` Jest RNTL · `layer-not-committed-suite` not playwright test

## How to use

1. Discover the repo (binary, origin, scripts).
2. Pick **1–N** rule ids (higher priority first).
3. `Read` only `rules/<id>.md` (relative to this skill directory).
4. Do **not** bulk-read `rules/` or load all of `AGENTS.md` unless stuck or asked.
5. Reviewing or refactoring: walk categories top-down until covered.

## Full reference

Command catalog and official docs: `AGENTS.md` (reference only; prefer `rules/` + this router).
