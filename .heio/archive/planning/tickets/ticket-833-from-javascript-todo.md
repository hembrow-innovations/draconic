---
id: "ticket-833-from-javascript-todo"
title: "from JavaScript never shows the browser todo"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T13:10:00Z"
updated_at: "2026-09-12T13:35:00Z"
---

# from JavaScript never shows the browser todo

## Signal

Closed. Website swarm shipped `website/from-javascript.md` with a Todo heading, `document` and `localStorage` in prose, and a GitHub `examples/todo` link. Search for Todo hits `/from-javascript#todo`. No playground, no new `drac` fence, no new Learn chapter.

## Fit

In scope of `public-site.ia:learn-walkable` and `public-site.search:titles-headings`. GitHub example link plus heading, same pattern as FizzBuzz plus Zed.

## Notes

- Learn page: `website/from-javascript.md`
- Locks: `website/src/tests/learn-pages.test.ts`, `website/src/tests/search.test.ts`, `website/src/tests/markdown-render.test.ts`
- Example: `examples/todo`
- Do not add a playground or a new Learn chapter

## Parent

[[Public site — Contract]]
