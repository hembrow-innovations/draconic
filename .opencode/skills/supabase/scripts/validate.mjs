#!/usr/bin/env node
/**
 * Fail if the supabase router drifts from rules/.
 *
 *   node skills/data/supabase/scripts/validate.mjs
 */

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SKILL = join(ROOT, "SKILL.md");
const RULES = join(ROOT, "rules");
const MAX_SKILL_LINES = 150;
const PREFIXES = [
  "prod",
  "disc",
  "pitfall",
  "key",
  "local",
  "mig",
  "rls",
  "cfg",
  "auth",
  "fn",
  "types",
  "store",
  "test",
  "ci",
];
const ID_RE = new RegExp(`^(?:${PREFIXES.join("|")})-[a-z0-9-]+$`);

const errors = [];
const skill = readFileSync(SKILL, "utf8");
const skillLines = skill.split("\n").length;

if (skillLines > MAX_SKILL_LINES) {
  errors.push(`SKILL.md is ${skillLines} lines (max ${MAX_SKILL_LINES})`);
}
if (!/^---\nname:\s*supabase\n/m.test(skill)) {
  errors.push("SKILL.md frontmatter must start with name: supabase");
}
if (!/^description:\s+\S+/m.test(skill)) {
  errors.push("SKILL.md is missing description");
}
if (!/Do \*\*not\*\* bulk-read `rules\/`/.test(skill)) {
  errors.push("SKILL.md must tell the agent not to bulk-read rules/");
}
if (/\*\*Incorrect:\*\*/.test(skill)) {
  errors.push("SKILL.md is a router; move Incorrect examples into rules/");
}

const ROUTES = [
  { task: "supabase start", ids: ["local-start", "disc-docker-running"] },
  { task: "db push", ids: ["prod-db-push-gated", "prod-opt-in-explicit"] },
  { task: "service_role in browser", ids: ["key-secret-server-only", "pitfall-service-role-browser"] },
  { task: "gen types", ids: ["types-gen-local"] },
  { task: "no supabase on PATH", ids: ["disc-cli-on-path"] },
  { task: "link project", ids: ["prod-link-gated"] },
  { task: "RLS", ids: ["rls-enable-every-table", "rls-is-boundary"] },
];

const indexed = new Set();
for (const m of skill.matchAll(/`([^`]+)`/g)) {
  const id = m[1];
  if (ID_RE.test(id)) indexed.add(id);
}

const onDisk = readdirSync(RULES)
  .filter((f) => f.endsWith(".md") && !f.startsWith("_"))
  .map((f) => f.slice(0, -3))
  .sort();

for (const id of [...indexed].sort()) {
  if (!onDisk.includes(id)) errors.push(`index cites missing rules/${id}.md`);
}
for (const id of onDisk) {
  if (!indexed.has(id)) errors.push(`rules/${id}.md is not in the SKILL.md index`);
  const body = readFileSync(join(RULES, `${id}.md`), "utf8");
  if (!/^---\n(?:.+\n)*title:\s.+\n(?:.+\n)*impact:\s.+\n(?:.+\n)*---/m.test(body)) {
    errors.push(`rules/${id}.md needs title and impact frontmatter`);
  }
  if (!/^## /m.test(body)) errors.push(`rules/${id}.md needs an h2`);
  if (!/\*\*Incorrect:\*\*/.test(body)) errors.push(`rules/${id}.md needs Incorrect`);
  if (!/\*\*Correct:\*\*/.test(body)) errors.push(`rules/${id}.md needs Correct`);
  if (/[\u2013\u2014]/.test(body)) errors.push(`rules/${id}.md has an em or en dash`);
}

if (/[\u2013\u2014]/.test(skill)) errors.push("SKILL.md has an em or en dash");

for (const route of ROUTES) {
  for (const id of route.ids) {
    if (!indexed.has(id)) {
      errors.push(`route "${route.task}" needs index entry ${id}`);
    }
  }
}

if (errors.length) {
  console.error(`supabase skill validate failed (${errors.length}):`);
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}

console.log(
  `ok: SKILL.md ${skillLines} lines, ${onDisk.length} rules, index matches disk`,
);
