---
id: "ticket-837-flagship-service-unnamed"
title: "Public site never names flagship-service"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T13:35:00Z"
updated_at: "2026-09-12T13:57:00Z"
---

# Public site never names flagship-service

## Signal

Closed. Website swarm shipped `website/packages.md` with a Flagship service heading and GitHub `examples/flagship-service` link, and `website/host-io.md` with the same GitHub link after HTTP echo. Search for Flagship service hits `/packages#flagship-service`. No playground, no new `drac` fence, no new Learn chapter.

## Fit

In scope of `public-site.ia:learn-walkable` and `public-site.search:titles-headings`. GitHub example link plus heading, same pattern as Todo plus HTTP echo.

## Notes

- Example: `examples/flagship-service`
- Pages: `website/packages.md`, `website/host-io.md`
- Locks: `website/src/tests/learn-pages.test.ts`, `website/src/tests/search.test.ts`, `website/src/tests/markdown-render.test.ts`
- Do not widen search to body terms. That is [[ticket-821-search-body-terms]].

## Parent

[[Public site — Contract]]
