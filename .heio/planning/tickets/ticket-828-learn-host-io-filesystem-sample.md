---
id: "ticket-828-learn-host-io-filesystem-sample"
title: "Learn host I/O names filesystem APIs with no compiling sample"
kind: ticket
status: open
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T11:42:00Z"
updated_at: "2026-09-12T11:42:00Z"
---

# Learn host I/O names filesystem APIs with no compiling sample

## Signal

Website swarm 2026-09-12. `website/host-io.md` ships a compiling `stdoutWrite` fence. Filesystem is a heading that names `readFileText` and `writeFileText` as portable, with no sample. Sockets already point at the GitHub HTTP echo example. A visitor cannot copy a Program that reads or writes a file without leaving the page.

## Fit

Unknown until triage. In scope of `public-site.ia:learn-walkable` and `public-site.fences:shipped-must-build`. A new sample must be a compiling `drac` fence. Command fences are not `drac` fences.

## Notes

- Page: `website/host-io.md`
- Lock: `website/src/tests/learn-pages.test.ts`
- Do not add fences on remaining not-yet pages.

## Parent

[[Public site — Contract]]
