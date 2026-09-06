# Round contracts

Parent and **round-orchestrator** only. Do not inject this file into panelists.

## Paths

Notebook: `.heio/planning/rounds/rounds-<NN>-<slug>.md`

Append each frontier as `## Round N` in that file. Never rewrite an earlier round.

## Round heading

```markdown
## Round <N>

### Questions

### Question <Q> - title
question and any related context.
```

`N` is the round number. `Q` is the question number in that round, starting at 1. Omit the parent's recommended answer.

After the judge runs, each question also has:

```markdown
#### Selected
- **winner**: <panel_member_name>
- **answer**: <text>
- **reasoning**: <text>
- **reason**: <why this winner>
```

## Answers

Each panelist appends under the same round:

```markdown
### <panel_member_name>

#### Question <Q> - title
original question text

#### Answer
Answer to the question.

#### Reasoning
reasoning to the answer.
```

Copy every question. Answer all of them. Write only that round in the notebook.

## Launch

Stable keys: `round-orchestrator`, `answer-architect`, `answer-product`, `answer-coder`, `judge`. Phase labels: `Round <N> orchestrate`, `Round <N> answer`, `Round <N> judge`. Fresh subagent on every child.

Spawn OpenCode subagents. No Pi plugins. No peer answers. Include the round file path, settled selected answers from earlier rounds, and evidence.

Judge waits until every panelist has written. Append `#### Selected` under each question. Leave question text and answer blocks untouched.

If spawn is missing, the parent writes the questions, three answers, and the selected blocks, and marks `skip: no spawn runtime` on the round.

## Return

Round-orchestrator returns the round file path and each question's winner, answer, and reason.
