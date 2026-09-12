---
id: "ticket-821-search-body-terms"
title: "In-site search misses body terms visitors type"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T10:45:00Z"
updated_at: "2026-09-12T12:50:00Z"
---

# In-site search misses body terms visitors type

## Signal

Website swarm 2026-09-12. Search matches title and heading text only. Most teaching pages have one h1 that repeats the title. CLI verbs such as fmt, run, and repl live in lists, not headings. Type names such as i32 are body-only. A miss renders no copy. Duplicate titles such as packages cannot be told apart between Learn and Reference.

## Fit

Unknown until triage. Current promise `public-site.search:titles-headings` is title or heading only. Widening to body terms needs a contract edit first. Vault phrases must stay out.

## Notes

- Index: `website/src/lib/search/searchIndex.ts`
- Chrome: `website/src/components/SiteSearch/SiteSearch.tsx`
- Lock: `website/src/tests/search.test.ts`
- CLI verbs parse, extract, check, fmt, doc, build, run, repl, test, version, help, and bindgen are headings on `website/content/cli.md`. Search for run, fmt, repl, extract, doc, and bindgen hits `/cli` with section hashes. Body-term indexing still needs a contract edit.

## Parent

[[Public site — Contract]]
