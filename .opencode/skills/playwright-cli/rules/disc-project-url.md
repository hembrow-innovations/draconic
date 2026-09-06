---
title: Use the printed web origin
impact: CRITICAL
impactDescription: invented host or port opens the wrong app
tags: [disc, expo, url]
---

## Use the printed web origin

Expo, Metro, and Vite print the origin when they start. Ports move. Do not hardcode 8081, 19006, or 3000.

**Incorrect:** `playwright-cli open http://localhost:8081` because that is a common Expo port.

**Correct:** Read the project's start script, run it if needed, and open the origin it printed. A project `playwright-cli` skill wins when it names the origin.

Notes: No web script and no running origin means stop (`stack-web-not-native`).
