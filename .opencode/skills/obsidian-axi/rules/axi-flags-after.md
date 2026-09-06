---
title: Flags after the command
impact: CRITICAL
impactDescription: flags before the command are rejected
tags: [axi, flags]
---

## Flags after the command

Every flag belongs after the subcommand. A leading `--vault` or `--help` mixed in front is a usage error.

**Incorrect:** `obsidian-axi --vault docs search heio`

**Correct:** `obsidian-axi search heio --vault docs`

Notes: `-h` does nothing. Use `--help`. Per-command help: `obsidian-axi search --help`. Top-level `--help` must be the only argument.
