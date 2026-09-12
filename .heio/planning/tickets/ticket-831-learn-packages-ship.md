---
id: "ticket-831-learn-packages-ship"
title: "Learn packages chapter is still not-yet"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T12:50:00Z"
updated_at: "2026-09-12T12:50:00Z"
---

# Learn packages chapter is still not-yet

## Signal

Website swarm 2026-09-12. Learn walks Install through host I/O, then stops on packages. The page names `draconic get` and `draconic mod tidy` with no argv. Roadmap K01 through K10 is done. Git-backed packages are the last advertised Learn skill.

## Fit

Unknown until triage. In scope of `public-site.ia:learn-walkable`. Flip to shipped only with compiling fences. A `from "github.com/org/pkg"` drac fence cannot resolve in the isolated site pipeline. Shell fences for get and tidy, plus a package-root export, are allowed. Leave Reference packages not-yet unless that page ships separately. See [[ticket-820-reference-packages-get-tidy]].

## Notes

- Learn page: `website/packages.md`
- Lock: `website/src/tests/learn-pages.test.ts`
- Consumer example: `examples/pkg-consumer`
- Do not invent a live git remote.
- Do not copy vault package notes onto the site.

## Parent

[[Public site — Contract]]
