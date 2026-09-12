---
id: "ticket-827-install-node-prerequisite"
title: "Install teaches draconic run before naming Node"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T11:35:00Z"
updated_at: "2026-09-12T11:42:00Z"
---

# Install teaches draconic run before naming Node

## Signal

Closed. Website swarm shipped `website/install.md` naming `node` on PATH before the first `draconic run` command. `website/src/tests/learn-pages.test.ts` locks that order. Command fences stay command fences. No PowerShell installer and no invented install URL.

## Fit

Unknown until triage. In scope of `public-site.ia:learn-walkable`. Do not invent a PowerShell installer. Command fences are not `drac` fences.

## Notes

- Page: `website/install.md`
- Lock: `website/src/tests/learn-pages.test.ts` already mentions `` `node` on PATH `` but not as a prerequisite
- CLI: `crates/draconic-cli/src/cmd/cmd_run.rs`
- Do not invent an installer URL

## Parent

[[Public site — Contract]]
