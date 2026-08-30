---
title: Kind to template to destination
impact: HIGH
tags: [template]
---

# Kind to template to destination

Copy the template. Do not invent a new skeleton.

| Kind | Template | Destination | Filename |
|---|---|---|---|
| overview | `templates/overview.md` | `docs/overview/` | `overview-<slug>.md` |
| architecture | `templates/architecture.md` | `docs/architecture/` | `architecture-<slug>.md` |
| system-design | `templates/system-design.md` | `docs/architecture/` | `system-design-<slug>.md` |
| adr | `templates/adr.md` | `docs/decisions/adr/` | `NNNN-<slug>.md` |
| rfc | `templates/rfc.md` | `docs/decisions/rfc/` | `rfc<N>-<slug>.md` |
| purpose | `templates/purpose.md` | `docs/specs/<bucket>/<area>/` | `purpose.md` |
| spec | `templates/spec.md` | `docs/specs/<bucket>/<area>/` | `spec-<slug>.md` |
| api | `templates/api.md` | `docs/api/` | `api-<slug>.md` |
| schema | `templates/schema.md` | `docs/api/schema/` | `schema-<slug>.md` |
| non-functional | `templates/non-functional.md` | `docs/non-functional/` | `<topic>.md` |
| standard | `templates/standard.md` | `docs/standards/` | `standards-<slug>.md` |
| style | `templates/style.md` | `docs/style/` | `style-<slug>.md` |
| guide | `templates/guide.md` | `docs/guides/` | `guides-<slug>.md` |

Shared fields: `templates/required-fields.md`.

Write an ADR only when the choice is hard to reverse, has real alternatives, or keeps getting re-litigated. Prefer a purpose, spec, standard, or system-design first.

Do not create living `web/{requirements,design,tasks}.md` triad files.
