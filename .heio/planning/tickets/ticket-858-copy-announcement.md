---
id: "ticket-858-copy-announcement"
title: "Fence Copy controls share one name and are not announced"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T06:56:00Z"
updated_at: "2026-09-12T06:56:00Z"
---

# Fence Copy controls share one name and are not announced

## Signal

Playwright audit 2026-09-12. Copy on a Learn fence puts the sample on the clipboard and the button can read Copied, but every fence uses the same accessible name Copy, and nothing is announced. After a successful copy the label can stay Copied for the rest of the article.

## Fit

Unknown until triage. Promise `public-site.fences:copy` is that a visitor can copy fence text. Confirmation and distinct names are not locked. Distinct from home samples, which have no Copy control.

## Notes

- Live: `/install`, seven Copy buttons, all `aria-label=Copy`
- First fence clipboard text matched the pre text including the trailing newline
- After click, snapshot showed Copied; no `aria-live` on the page
- `website/src/components/CodeFence/CodeFence.tsx` sets copied true and never clears it
- Home `/` has five `pre` samples and zero Copy buttons

## Parent

[[Public site — Contract]]
