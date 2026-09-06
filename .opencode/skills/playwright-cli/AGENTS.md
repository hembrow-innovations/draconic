# Playwright CLI (React Native Web)

Reference only.

**Prefer `rules/` + `SKILL.md` stack notes.** This file is optional bulk context for humans or when the agent is stuck. Do not load it by default on skill activation.

## Official docs

- Playwright CLI: https://playwright.dev/docs/cli
- Locators: https://playwright.dev/docs/locators
- Web-first assertions (committed suite only): https://playwright.dev/docs/writing-tests
- Expo web: https://docs.expo.dev/workflow/web/
- React Native Web: https://necolas.github.io/react-native-web/docs/
- Expo Router web: https://docs.expo.dev/router/reference/web/

The agent binary is `@playwright/cli` (`playwright-cli` or `npx playwright cli`). It is not `npx playwright test`.

## Discover the repo

Do not assume Expo, port 8081, or a seed user.

1. Resolve the binary (`disc-cli-binary`).
2. Find the web start script (`expo start --web`, `web`, Vite, Next). Read the printed origin.
3. Read package scripts, justfile, README, CI, and any project `playwright-cli` skill.
4. Confirm the target is a browser. iOS and Android stay on Maestro.

If there is no web target, stop.

## Decision tree

```
What failed or what are you adding?
  No web origin / native device only     → Maestro, not this CLI
  Unit or component change               → Jest / RNTL (react-testing)
  Committed Playwright *.spec.ts         → project script / npx playwright test
  Interactive click-through of Expo web  → this CLI
  Selector miss on RN-web                → snapshot ref, then getByTestId
  Pressable not found as button          → testID or getByLabel, not getByRole('button')
  Need phone chrome                      → open --mobile or --device
  Auth leftover from last run            → named session or state-load, then goto
```

## Invocation

```bash
playwright-cli --version
npx --no-install playwright --version
npx playwright cli --help
```

If `playwright-cli` is missing and local Playwright exists, prefix every command with `npx playwright cli`. Otherwise `npm install -g @playwright/cli@latest`.

Global flags: `--raw`, `--json`, `-s=` / `--session=`. Env: `PLAYWRIGHT_CLI_SESSION`, `PLAYWRIGHT_HTML_OPEN=never`.

## Command catalog

**Session / process:** `open`, `attach`, `detach`, `close`, `close-all`, `kill-all`, `list`, `delete-data`

**Navigation:** `goto`, `go-back`, `go-forward`, `reload`

**Interact:** `click`, `dblclick`, `fill`, `type`, `press`, `keydown`, `keyup`, `hover`, `drag`, `drop`, `select`, `upload`, `check`, `uncheck`, `dialog-accept`, `dialog-dismiss`, `resize`

**Mouse raw:** `mousemove`, `mousedown`, `mouseup`, `mousewheel`

**Inspect / decide:** `snapshot`, `find`, `eval`, `generate-locator`, `highlight`, `show`, `console`, `requests`, `request`, `run-code`

**Save / record:** `screenshot`, `pdf`, `tracing-start`, `tracing-stop`, `video-start`, `video-chapter`, `video-stop`, `video-show-actions`, `video-hide-actions`

**Tabs:** `tab-list`, `tab-new`, `tab-close`, `tab-select`

**Network:** `route`, `route-list`, `unroute`

**Storage:** `state-save`, `state-load`, `cookie-list`, `cookie-get`, `cookie-set`, `cookie-delete`, `cookie-clear`, `localstorage-list`, `localstorage-get`, `localstorage-set`, `localstorage-delete`, `localstorage-clear`, `sessionstorage-list`, `sessionstorage-get`, `sessionstorage-set`, `sessionstorage-delete`, `sessionstorage-clear`

**Debug-attach extra:** `resume` (only after attaching to a paused `npx playwright test --debug=cli` session)

Do not invent command names outside this list.

## React Native Web facts

- `testID` becomes `data-testid`. Target with snapshot refs or `getByTestId('…')`.
- `accessibilityLabel` becomes `aria-label`. `accessibilityRole` becomes `role`.
- `View` and `Text` are `div`s. `getByRole('heading')` only works when the app set a header role.
- `Pressable` / `TouchableOpacity` are often role-less divs. Do not assume `getByRole('button')`.
- `TextInput` is `input` or `textarea`. `fill` works. There is no software keyboard.
- Expo Router file routes become URL paths after the origin. Discover the origin. Then `goto` the path.
- `Platform.OS` is `web`. iOS and Android branches do not run.
- `--mobile` / `--device="iPhone 15"` is phone chrome on the web app, not a simulator.

## Artifacts

CLI snapshots land under `.playwright-cli/`. Agent screenshots and PDFs go in a gitignored tmp path the project already uses. Never commit auth state files.

## Sibling skills

- `react-testing` owns Jest and RNTL when a unit test is the right layer.
- `maestro` owns iOS and Android device E2E.
- `tdd` owns what a good test is.
- A project `playwright-cli` skill owns origin, seed, and scratch when installed.
