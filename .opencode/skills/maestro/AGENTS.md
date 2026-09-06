# Maestro (React Native). Reference only

**Prefer `rules/` + `SKILL.md` stack notes.** This file is optional bulk context for humans or when the agent is stuck. Do not load it by default on skill activation.

## Official docs

- React Native: https://docs.maestro.dev/get-started/supported-platform/react-native
- Flows: https://docs.maestro.dev/maestro-flows/
- Selectors: https://docs.maestro.dev/reference/selectors
- Commands: https://docs.maestro.dev/reference/commands-available
- Wait commands: https://docs.maestro.dev/maestro-flows/flow-control-and-logic/wait-commands
- Nested flows: https://docs.maestro.dev/maestro-flows/flow-control-and-logic/nested-flows
- Artifacts: https://docs.maestro.dev/maestro-flows/workspace-management/test-reports-and-artifacts
- CLI install: https://docs.maestro.dev/maestro-cli/how-to-install-maestro-cli
- Known issues: https://docs.maestro.dev/extra-materials/troubleshooting/known-issues

Install: https://maestro.mobile.dev (or the CLI install page above).

## Discover the repo

Do not assume `.maestro/`, Expo, or a oneshot script.

1. `which maestro` and `maestro --version`. Stop if missing.
2. Search `.maestro/`, `maestro/`, `e2e/maestro/`, `tests/e2e/mobile/`, and YAML that contains `appId:`, `tapOn:`, or `launchApp`.
3. Read package scripts, justfile, README, and CI.
4. Copy `appId`, folder layout, tags, and seed users from what you find.

If nothing exists, propose `.maestro/` and wait.

## Decision tree

```
What failed or what are you adding?
  CLI missing                         → tell the user how to install; stop
  Project has an e2e script           → run that (run-prefer-project-script)
  No script, flows exist              → maestro test <path>
  Selector miss or colliding copy     → testID, then index, then regex
  inputText DEADLINE_EXCEEDED         → release build, not a longer wait
  Keyboard covers Submit              → pressKey enter; never hideKeyboard
  Device cannot reach host API        → 10.0.2.2 (Android emu) or LAN IP
  Unit or component change            → Jest or RNTL, not a new flow
```

## Canonical commands

Use the project's script when it exists.

```bash
which maestro || echo "Maestro CLI not on PATH"
maestro --version
maestro test path/to/flow.yaml
maestro test .maestro/
maestro studio
```

Default artifact dir is `~/.maestro/tests`. Honor `testOutputDir` or `--test-output-dir` when the repo sets one. Agent dumps go in a gitignored tmp path, never under `docs/` or `apps/`.

## React Native facts

- `testID` is Maestro `id:`. Visible `Text` or `title` is `text:` (regex).
- Zero app instrumentation required. Maestro drives the shipped binary.
- Expo Go: `openLink` with the exp URL. Standalone or EAS: `launchApp` plus `appId`.
- iOS nested `TouchableOpacity` can swallow taps. Set the wrapper `accessible={false}` and the inner `accessible={true}`.
- Android `hideKeyboard` is a back press. That often pops the screen.
- Debug or Metro bins hang `inputText` (DEADLINE). Prefer a release APK or IPA.

## Maintain this pack

```bash
node skills/ui/maestro/scripts/validate.mjs
```

The script fails if `SKILL.md` grows past 150 lines, if the index cites a missing rule, or if a rule file is not in the index.

## Sibling skills

- `react-testing` owns Jest and RNTL when E2E is the wrong layer
- `tdd` owns what a good test is
- project `maestro` owns appId, seed, and oneshot when installed
