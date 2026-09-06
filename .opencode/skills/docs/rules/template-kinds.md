---
title: Kind to template to destination
impact: HIGH
tags: [template]
---

# Kind to template to destination

Copy the template. Do not invent a new skeleton.

- **overview**: `templates/overview.md` → `docs/overview/` as `overview-<slug>.md`
- **architecture**: `templates/architecture.md` → `docs/architecture/` as `architecture-<slug>.md`
- **system-design**: `templates/system-design.md` → `docs/architecture/` as `system-design-<slug>.md`
- **adr**: `templates/adr.md` → `docs/decisions/adr/` as `NNNN-<slug>.md`
- **rfc**: `templates/rfc.md` → `docs/decisions/rfc/` as `rfc<N>-<slug>.md`
- **purpose**: `templates/purpose.md` → `docs/specs/<domain>/<area>/purpose.md`
- **contract**: `templates/contract.md` → `docs/specs/<domain>/<area>/contract.md`
- **test**: `templates/test.md` → `docs/specs/<domain>/<area>/test.md`
- **spec**: `templates/spec.md` → `docs/specs/<domain>/<area>/spec-<slug>.md` (optional narrative)
- **api**: `templates/api.md` → `docs/api/` as `api-<slug>.md`
- **schema**: `templates/schema.md` → `docs/api/schema/` as `schema-<slug>.md`
- **non-functional**: `templates/non-functional.md` → `docs/non-functional/` as `<topic>.md`
- **standard**: `templates/standard.md` → `docs/standards/` as `standards-<slug>.md`
- **style**: `templates/style.md` → `docs/style/` as `style-<slug>.md`
- **guide**: `templates/guide.md` → `docs/guides/` as `guides-<slug>.md`

Shared fields: `templates/required-fields.md`. Spec folder procedure: load **spec**. Copy those templates from this skill.

Write an ADR only when the choice is hard to reverse, has real alternatives, or keeps getting re-litigated. Prefer a purpose, contract, standard, or system-design first.

Do not create living `web/{requirements,design,tasks}.md` triad files.
