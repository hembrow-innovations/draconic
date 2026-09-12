---
id: "ticket-904-404-status-untested"
title: "Unknown URL recovery does not lock HTTP 404"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
sprint: "website-chrome-polish"
created_at: "2026-09-12T10:47:00Z"
updated_at: "2026-09-12T10:47:00Z"
---

# Unknown URL recovery does not lock HTTP 404

## Signal

Website chrome polish review 2026-09-12 after [[slice-880-empty-404]] was marked met. `public-site.chrome:not-found` promises the response is 404, the document title names the miss, and recovery chrome stays. `website/src/tests/not-found.test.ts` greps that root source does not say `statusCode: 200` and never requests an unknown URL. GitHub Pages prerender lists only real pages and has no 404.html.

## Fit

Unknown until triage. Follow-up after met [[slice-880-empty-404]] / [[task-881-empty-404]]. Distinct from promoted [[ticket-856-empty-404]] (generic empty Not Found paragraph). Share tags on the miss also sit next to [[slice-890-meta-description]].

## Notes

- `website/src/tests/not-found.test.ts` is source-only; vitest passed without fetching a miss
- `website/src/components/NotFound/NotFound.tsx` puts `Not found · Draconic` in a body `<title>`; other pages use `pageShareHead`
- `website/src/routes/__root.tsx` still has default title Draconic; unmatched URLs still take that root title
- Recovery view has no page-specific description, canonical, or Open Graph
- `website/vite.config.ts` prerender `crawlLinks: false` and no 404.html
- No splat `routes/$.tsx`; local Start SSR may still 404 while public Pages serves something else

## Parent

[[slice-880-empty-404]]
