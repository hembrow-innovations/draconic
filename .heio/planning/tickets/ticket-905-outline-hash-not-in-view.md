---
id: "ticket-905-outline-hash-not-in-view"
title: "On this page marks the hash, not the heading in view"
kind: ticket
status: open
ticket_type: bug
tags: [website, public-site]
blocked_by: []
sprint: "website-chrome-polish"
created_at: "2026-09-12T10:47:00Z"
updated_at: "2026-09-12T10:47:00Z"
---

# On this page marks the hash, not the heading in view

## Signal

Website chrome polish review 2026-09-12 after [[slice-892-outline-current-section]] was marked met. `public-site.chrome:on-page-toc` requires the outline to mark the heading in view with `aria-current` that is not page. `OnThisPage` marks by `location.hash` only. A load with no hash marks nothing. Scroll does not update the outline.

## Fit

Unknown until triage. Follow-up after met [[slice-892-outline-current-section]] / [[task-893-outline-current-section]]. Distinct from promoted [[ticket-862-outline-current-section]] (outline never marked current at all).

## Notes

- `website/src/features/docs/OnThisPage/OnThisPage.tsx` current id is `location.hash`; `aria-current` is `"true"`, not `"page"`
- Outline items are native `#` anchors, not router hash navigation, so `useLocation` may not update on in-page clicks
- `website/src/tests/docs-shell.test.ts` greps `location.hash` and `"aria-current": "true"` and never renders an outline
- Coverage map claims `#zed-editor` cannot leave every outline link unmarked; the test only `toContain`s extracted ids
- Reference related-link footer from [[slice-886-reference-related-footer]] is out of this signal

## Parent

[[slice-892-outline-current-section]]
