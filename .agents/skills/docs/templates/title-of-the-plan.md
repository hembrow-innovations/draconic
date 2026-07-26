---
id: architecture-17
type: doc
kind: architecture
title: "Title of the plan"
domain: <domain>   # domain the doc concerns, e.g. system
created_at: 2026-06-29
updated_at: 2026-06-29
---
### Plan Model
**Location**: `planning/<domain>/plans/`
**Filename**: `plans-<N>-<slug>.md`

#### Frontmatter
_Required frontmatter fields always included_
```yaml
...Required frontmatter fields
title: "plan title"
description: "one sentence description"
status: "draft" | "ready" | "active" | "complete" | "abandoned"
tags: ["list of tags"]
```

#### Body / Content Sections

##### Title
> `#` H1 Title of the plan.
> `>` blockquotes section describing the plan

```markdown

# Title of the plan

> Description of the plan

```


##### Objectives
> `##` H2 Objectives section
> `-` List of objectives

```markdown
## Objectives
- Goal 1
- Goal 2
- ...
```

##### Phases
>`##` H2 Phases of the plan Section
>`###` H3 Phase definition for each phase
> `>` blockquotes section with a short description of the phase (optional)
> `-` `[ ]` `[[link to task]]`: task description

```markdown
## Phases

### Phase {num}: {phase title}
> Phase Description (Optional)

- [ ] [[{Link to Task}]]: {Task Title} 
- [ ] ...etc

```

##### Requirements / Specifications


```markdown


## Approach
The chosen strategy and reasoning behind it.

## Steps
Ordered breakdown of work — each step should be completable independently.

## Dependencies
Third-party libraries, local packages, services, or other plans this depends on.

## Constraints
Hard limits — things that must not change or be violated.

## Risks
What could go wrong and any known mitigations.

## Acceptance
- list of acceptance criteria
  
  
## Success Metrics - 80% of test users can extract insights in under 5 minutes - Support at least 3 different paper formats - Average satisfaction score ≥ 4.5/5

## Risks & Blockers - Access to latest research papers (legal constraints) - LLM cost management at scale

## Definition of Done (DoD)
- [ ] Code written and reviewed
- [ ] Unit & integration tests added and passing
- [ ] Documentation updated
- [ ] QA / stakeholder sign-off
- [ ] Deployed to production (if applicable)

## Additional Information
- Links to designs, Figma, API docs, etc.
- Screenshots / references

```

The h1 `Title` string must match the string of the `title` frontmatter field.
