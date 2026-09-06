#!/usr/bin/env node
import { existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const PREFIXES = ["ticket-", "task-", "slice-", "location-", "rounds-"];
const ROOTS = [".heio/planning", ".heio/archive"];

function walk(dir, names) {
  if (!existsSync(dir)) return;
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) walk(full, names);
    else names.push(name);
  }
}

const names = [];
for (const root of ROOTS) walk(root, names);

let max = 0;
for (const name of names) {
  for (const prefix of PREFIXES) {
    if (!name.startsWith(prefix)) continue;
    const match = name.slice(prefix.length).match(/^(\d+)/);
    if (match) max = Math.max(max, Number(match[1]));
  }
}

process.stdout.write(`${String(max + 1).padStart(2, "0")}\n`);
