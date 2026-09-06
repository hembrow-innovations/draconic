---
title: Start Expo web from the project script
impact: HIGH
impactDescription: the wrong bundler serves an empty or native-only shell
tags: [rn, expo]
---

## Start Expo web from the project script

Expo web is a real browser origin. The start command lives in `package.json` or the justfile.

**Incorrect:** `npx expo start` without `--web`, then pointing Playwright at Metro's native bundle URL.

**Correct:** Run the project's web script (`expo start --web`, `web`, or whatever the repo names). Wait until it prints an `http` origin. Then `open` that origin.

Notes: Webpack vs Metro web is the project's choice. Do not switch bundlers to make the CLI happier.
