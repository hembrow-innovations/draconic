---
id: "ticket-853-github-mobile-hit-area"
title: "Mobile GitHub control stretches to a tall hit area"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T05:58:32Z"
updated_at: "2026-09-12T05:58:32Z"
---

# Mobile GitHub control stretches to a tall hit area

## Signal

Website swarm 2026-09-12. On a small viewport the primary cluster wraps Learn and Reference into columns, and GitHub sits in a third column with stretch alignment, so the link becomes about 51 by 460 CSS pixels instead of a compact control.

## Fit

Unknown until triage. Distinct from [[ticket-848-github-below-nav-fold]] (desktop below-fold). Promise `public-site.a11y:keyboard-small`. Purpose already says the side nav stacks and wraps.

## Notes

- Live home at 360 by 732 (Pixel 10 Chrome)
- GitHub box about x 281, y 170, w 51, h 460
- Cluster computed flex-direction row, flex-wrap wrap
- Keyboard Tab landed on GitHub with a 2-pixel focus ring around that whole strip
- Theme Dark stayed 34 by 44 beside search

## Parent

[[Public site — Contract]]
