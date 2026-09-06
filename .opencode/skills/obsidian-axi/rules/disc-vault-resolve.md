---
title: Pass --vault after the command
impact: CRITICAL
impactDescription: this checkout has no docs/.obsidian so walk-up misses
tags: [disc, vault]
---

## Pass --vault after the command

Vault pick is first match: `--vault`, then `OBSIDIAN_VAULT`, then `.obsidian/` in cwd or parents, then config default, then the app's open vault, then most recently opened.

This checkout's vault is `docs/`. There is no `docs/.obsidian/`. Walk-up will miss it.

**Incorrect:** `obsidian-axi --vault docs search heio`

**Correct:** `obsidian-axi search heio --vault docs`

Notes: `vault info` prints which rule applied. `vault list` shows known vaults. Put `--vault` on every call in this repo unless you just confirmed the dashboard resolved `docs/`.
