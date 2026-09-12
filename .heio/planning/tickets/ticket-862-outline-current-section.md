---
id: "ticket-862-outline-current-section"
title: "On this page never marks the visible heading"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T06:56:00Z"
updated_at: "2026-09-12T06:56:00Z"
---

# On this page never marks the visible heading

## Signal

Playwright audit 2026-09-12. Learn and Reference articles list section headings as On this page links, and those hashes work, but the outline does not mark the heading in view. After opening a heading permalink, every outline link still looks like a sibling.

## Fit

Unknown until triage. Promise `public-site.chrome:on-page-toc` requires the list of heading links, not a current-section state. Side nav already uses `aria-current=page` for the open article.

## Notes

- Live: `/install#zed-editor` scrolled the Zed editor heading to the top of the viewport
- Outline links From source, Zed editor, Reproducibility all had `aria-current` null and the same muted class
- Outline is in-flow under the title, not a sticky column
- Heading permalinks themselves resolve (`#from-source`, `#zed-editor`, `#reproducibility`)

## Parent

[[Public site — Contract]]
