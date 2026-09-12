---
id: "ticket-903-duplicate-learn-reference-titles"
title: "Learn and Reference host I/O and packages share one document title"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
sprint: "website-chrome-polish"
created_at: "2026-09-12T10:47:00Z"
updated_at: "2026-09-12T10:47:00Z"
---

# Learn and Reference host I/O and packages share one document title

## Signal

Website chrome polish review 2026-09-12 after [[slice-871-per-page-titles]] was marked met. Learn host I/O and Reference host I/O both title as `host I/O · Draconic`. Learn packages and Reference packages both title as `packages · Draconic`. Tabs do not distinguish those destinations. `public-site.chrome:document-title` requires each public page's document title to name the open page so tabs distinguish destinations.

## Fit

Unknown until triage. Follow-up after met [[slice-871-per-page-titles]] / [[task-872-per-page-titles]]. Distinct from promoted [[ticket-851-untitled-pages]] (every tab was only Draconic).

## Notes

- `website/content/host-io.md` and `website/content/reference-host-io.md` both have title host I/O
- `website/content/packages.md` and `website/content/reference-packages.md` both have title packages
- `website/src/routes/host-io.tsx` and `website/src/routes/reference-host-io.tsx` both emit `${loaderData.title} · Draconic` through `pageShareHead`
- Same collision on packages routes and on `og:title`
- `website/src/tests/document-title.test.ts` loops markdown routes and never asserts unique titles
- Visible CONTEXT terms on the pages stay host I/O and packages; this signal is document title and share title, not the side-nav accessible names from [[slice-869-distinct-nav-labels]]

## Parent

[[slice-871-per-page-titles]]
