---
title: Filesystem CLI only
impact: LOW
impactDescription: MCP and the official obsidian CLI need a running app
tags: [layer, mcp]
---

## Filesystem CLI only

This pack is `@andershoffmann/obsidian-axi` on disk. No plugin, no server, no API key, no running app.

**Incorrect:** Installing Obsidian MCP, yakitrak `obsidian-cli`, `notesmd-cli`, or `npx obsidian-axi` (unscoped HermitCountry wrapper).

**Correct:** `npx -y @andershoffmann/obsidian-axi <command>` (`disc-cli-binary`).

Notes: `open` is the one command that talks to the app (`ws-open`). Everything else is files.
