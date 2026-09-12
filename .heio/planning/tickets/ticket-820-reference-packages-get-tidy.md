---
id: "ticket-820-reference-packages-get-tidy"
title: "Packages Reference does not show get or tidy usage"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T10:45:00Z"
updated_at: "2026-09-12T10:45:00Z"
---

# Packages Reference does not show get or tidy usage

## Signal

Website swarm 2026-09-12. `website/cli.md` sends package command names to packages. `website/reference-packages.md` sends them back to CLI and only names `draconic get` and `draconic mod tidy` with no argv. `website/reference.md` promises packages as git identity, manifest, lockfile, get and tidy. A visitor writing a Program has no public-site place to look up how to add a git dependency.

## Fit

Unknown until triage. In scope of `public-site.ia:reference-walkable`. The packages Reference page is not-yet, so fences stay off.

## Notes

- CLI page: `website/cli.md`
- Packages working page: `website/reference-packages.md`
- Do not publish the vault API note as the site.
- Do not add fences on a not-yet page.

## Parent

[[Public site — Contract]]
