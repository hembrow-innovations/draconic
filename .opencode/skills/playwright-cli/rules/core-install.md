---
title: Install the agent CLI only
impact: HIGH
impactDescription: installing the wrong package pollutes the app
tags: [core, install]
---

## Install the agent CLI only

The agent binary is `@playwright/cli`. It is not a dependency of the React Native app.

```bash
npx --no-install playwright --version
npm install -g @playwright/cli@latest
```

**Incorrect:** Adding `@playwright/test` or `@playwright/cli` to the app `package.json` so an interactive session can start.

**Correct:** Use a local Playwright CLI via `npx playwright cli` when the repo already has Playwright. Otherwise install `@playwright/cli` globally. Leave the app package manager alone.

Notes: After install, re-resolve the binary (`disc-cli-binary`).
