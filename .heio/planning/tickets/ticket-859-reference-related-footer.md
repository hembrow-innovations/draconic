---
id: "ticket-859-reference-related-footer"
title: "Reference articles have no related-link footer"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T06:56:00Z"
updated_at: "2026-09-12T06:56:00Z"
---

# Reference articles have no related-link footer

## Signal

Playwright audit 2026-09-12. Learn chapters end with a Learn sequence pager. Reference working pages end after the markdown. Purpose and the docs-article-order promise both name a related-link footer on Learn and Reference articles.

## Fit

Unknown until triage. Promise `public-site.chrome:docs-article-order`. Learn already passes a pager into DocsShell children. ReferencePage does not.

## Notes

- Live: `/install` has `nav` Learn sequence with Next from JavaScript and from systems
- Live: `/packages` has Previous host I/O
- Live: `/cli` nav labels are only Primary and On this page; no article footer
- `website/src/features/learn/LearnPage/LearnPage.tsx` renders LearnPager as DocsShell children
- `website/src/features/reference/ReferencePage/ReferencePage.tsx` passes no children

## Parent

[[Public site — Contract]]
