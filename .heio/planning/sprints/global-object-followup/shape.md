---
id: "global-object-followup"
title: "Global object follow-up"
kind: sprint
status: active
tags: [wayfinder]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Global object follow-up

## Grouping

HITL answers after [[rounds-842-global-object-identifier]]. Host aliases and Test262 honesty first; ADR and location drafts last. Language ROADMAP.md loop is out. Do not rewrite [[location-218-conformance]] destination sentences in this grouping.

## Slices in

- [[slice-896-browser-window-self]]: record whether Programs see window and/or self. blocked_by: none
- [[slice-898-test262-absent-globalthis]]: record honest-fail versus harness-rewrite for absent globalThis. blocked_by: none
- [[slice-900-adr-0004-global-wording]]: draft proposed ADR-0004 and location-218 sentences for a later map sitting. blocked_by: [[slice-896-browser-window-self]] [[slice-898-test262-absent-globalthis]]

## Slices out

- implementing window, self, or frames
- patching ADR-0004 or location-218 in this grouping
- Test262 harness or allowlist edits
- website chrome
- language ROADMAP atoms

## Drain

HITL only. Do not `/afk-task` these. First answers are [[task-897-browser-window-self]] and [[task-899-test262-absent-globalthis]]. [[task-901-adr-0004-draft-wording]] waits on both.
