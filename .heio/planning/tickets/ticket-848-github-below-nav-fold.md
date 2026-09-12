---
id: "ticket-848-github-below-nav-fold"
title: "Side-nav GitHub sits below the desktop fold"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T05:58:32Z"
updated_at: "2026-09-12T05:58:32Z"
---

# Side-nav GitHub sits below the desktop fold

## Signal

Website swarm 2026-09-12. Contract chrome includes GitHub in the sticky side nav, but at a 1280 by 720 desktop viewport the GitHub item is off-screen until the aside is scrolled. A visitor can miss the repo link.

## Fit

Unknown until triage. Distinct from closed [[ticket-646-search-below-nav-fold]]: search and theme already sit above the chapter lists. GitHub still follows those lists.

## Notes

- Live: `http://localhost:3000/` and `/cli`
- GitHub href `https://github.com/hembrow-innovations/draconic` is correct
- Bounding box about y 1064, height 44; aside clientHeight 720, scrollHeight about 1140; `inView` false
- Long always-expanded Learn and Reference lists push GitHub out of view

## Parent

[[Public site — Contract]]
