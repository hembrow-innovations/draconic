---
id: "ticket-860-shipped-badge-contrast"
title: "Shipped badge fails contrast in light theme"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T06:56:00Z"
updated_at: "2026-09-12T06:56:00Z"
---

# Shipped badge fails contrast in light theme

## Signal

Playwright audit 2026-09-12. The shipped chip is visible, but light-theme contrast is about 4.4 to 1 at 14px, under AA for normal text. Dark theme passes.

## Fit

Unknown until triage. Distinct from open [[ticket-849-current-page-contrast]] (nav accent green). Promise `public-site.nav:learn-reference-status`.

## Notes

- Live: `/install` light canvas `rgb(245, 248, 252)`
- Badge text `rgb(248, 251, 255)` on `rgb(49, 120, 198)`, ratio about 4.37, font-size 14px
- Dark class: text `rgb(12, 15, 20)` on `rgb(91, 159, 212)`, ratio about 6.72
- Badge is a span with no status role; label is the word shipped

## Parent

[[Public site — Contract]]
