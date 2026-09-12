---
id: "ticket-837-flagship-service-unnamed"
title: "Public site never names flagship-service"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T13:35:00Z"
updated_at: "2026-09-12T13:35:00Z"
---

# Public site never names flagship-service

## Signal

Website swarm 2026-09-12. examples/README.md ships flagship-service as typed HTTP plus fs/config plus a git dep. website/*.md never names it. FizzBuzz, HTTP echo, pkg-lib, pkg-consumer, and Todo have GitHub doorways. Shebang is a CLI heading with a repo path, not a GitHub URL.

## Fit

Unknown until triage. In scope of `public-site.ia:learn-walkable`. Prefer a GitHub link and a heading so title-or-heading search hits it. No playground. No new Learn chapter.

## Notes

- Example: `examples/flagship-service`
- Candidate pages: `website/packages.md`, `website/host-io.md`
- Locks: `website/src/tests/learn-pages.test.ts`, `website/src/tests/search.test.ts`
- Do not widen search to body terms. That is [[ticket-821-search-body-terms]].

## Parent

[[Public site — Contract]]
