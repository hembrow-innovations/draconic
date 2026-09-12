---
id: "ticket-907-share-head-tests-false-green"
title: "Document title and share meta tests do not read the live head"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
sprint: "website-chrome-polish"
created_at: "2026-09-12T10:47:00Z"
updated_at: "2026-09-12T10:47:00Z"
---

# Document title and share meta tests do not read the live head

## Signal

Website chrome polish review 2026-09-12 after [[slice-871-per-page-titles]] and [[slice-890-meta-description]] were marked met. `website/src/tests/document-title.test.ts` never calls `pageShareHead` and can still pass if that helper emits a slogan or drops the page name. `website/src/tests/meta-description.test.ts` builds a correct head inside the test and greps that routes mention `pageShareHead`, so a wrong path or empty description in a route file still passes.

## Fit

Unknown until triage. Follow-up after met [[task-872-per-page-titles]] and [[task-891-meta-description]]. Distinct from [[ticket-903-duplicate-learn-reference-titles]] (colliding host I/O and packages titles) and from promoted [[ticket-851-untitled-pages]] and [[ticket-861-missing-meta-description]].

## Notes

- After share-meta work, the title regex no longer matches `pageShareHead` and falls through to `loaderData.title`
- Meta tests name home, Learn, and from-javascript, then source-check other markdown routes; they never cover 404 or `__root.tsx`
- Fifteen markdown routes duplicate the same `loaderData` ternary around `pageShareHead`
- If `loaderData` is missing, routes emit title Draconic and description `""`
- Tests forbid Twitter cards, `og:image`, and theme-color; the contract does not state those forbids
- Website tests do not use testing-library

## Parent

[[slice-890-meta-description]]
