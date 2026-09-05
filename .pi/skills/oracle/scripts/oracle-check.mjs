#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const USAGE =
  "Usage: oracle-check.mjs [--status | --reverify] [ledger]\nDefault ledger: .heio/oracles.md\n";
const START = /^- \[([ xX])\] (O[1-9][0-9]*): (.+)$/;
const FIELD = /^ {2}(CHECK|EXPECT|EVIDENCE|ABANDON):(.*)$/;
const KNOWN = new Set(["CHECK", "EXPECT", "EVIDENCE", "ABANDON"]);
// 10 minutes. 120s killed healthy `cargo test --workspace` runs that were
// still progressing (match=yes, tens of crates already ok). Override with
// ORACLE_CHECK_TIMEOUT_MS (positive milliseconds).
const DEFAULT_CHECK_TIMEOUT_MS = 600_000;

function die(message, code = 2) {
  process.stderr.write(`error: ${message}\n`);
  process.exit(code);
}

function checkTimeoutMs() {
  const raw = process.env.ORACLE_CHECK_TIMEOUT_MS;
  if (raw == null || raw === "") return DEFAULT_CHECK_TIMEOUT_MS;
  const n = Number(raw);
  if (!Number.isFinite(n) || n <= 0) {
    die(
      `ORACLE_CHECK_TIMEOUT_MS must be a positive number of milliseconds, got ${JSON.stringify(raw)}`,
    );
  }
  return n;
}

function parseArgs(argv) {
  let mode = "run";
  let ledger;
  for (const arg of argv) {
    if (arg === "--help" || arg === "-h") {
      process.stdout.write(USAGE);
      process.exit(0);
    }
    if (arg === "--status" || arg === "--reverify") {
      if (mode !== "run") die("use only one of --status or --reverify");
      mode = arg.slice(2);
      continue;
    }
    if (arg.startsWith("-")) die(`unknown flag: ${arg}`);
    if (ledger) die("too many arguments");
    ledger = arg;
  }
  return { mode, ledger: ledger ?? join(".heio", "oracles.md") };
}

function parseLedger(text) {
  const lines = text.split("\n");
  if (lines.at(-1) === "") lines.pop();
  const oracles = [];
  for (let i = 0; i < lines.length; ) {
    const match = lines[i].match(START);
    if (!match) {
      i += 1;
      continue;
    }
    const oracle = {
      startLine: i,
      id: match[2],
      title: match[3].trim(),
      fields: {},
      fieldLines: {},
    };
    i += 1;
    while (i < lines.length) {
      const field = lines[i].match(FIELD);
      if (!field) {
        const unknown = lines[i].match(/^ {2}([A-Z_]+):/);
        if (unknown && !KNOWN.has(unknown[1])) {
          die(`${oracle.id}: unknown field ${unknown[1]}`);
        }
        break;
      }
      const name = field[1];
      if (name in oracle.fields) die(`${oracle.id}: duplicate field ${name}`);
      oracle.fields[name] = field[2].trim();
      oracle.fieldLines[name] = i;
      i += 1;
    }
    oracle.endLine = i;
    oracles.push(oracle);
  }
  if (oracles.length === 0) die("no oracles");
  const seen = new Set();
  for (const oracle of oracles) {
    if (seen.has(oracle.id)) die(`duplicate id ${oracle.id}`);
    seen.add(oracle.id);
    const abandon = oracle.fields.ABANDON;
    if (abandon !== undefined) {
      if (abandon === "") die(`${oracle.id}: blank ABANDON reason`);
      continue;
    }
    if (!oracle.fields.CHECK) die(`${oracle.id}: missing CHECK`);
    if (!oracle.fields.EXPECT) die(`${oracle.id}: missing EXPECT`);
  }
  return { lines, oracles };
}

function recordedMet(oracle) {
  const evidence = oracle.fields.EVIDENCE ?? "pending";
  return evidence.startsWith("met ") && evidence.includes("match=yes");
}

function stateOf(oracle) {
  if (oracle.fields.ABANDON !== undefined) {
    return { kind: "abandoned", reason: oracle.fields.ABANDON };
  }
  if (recordedMet(oracle)) return { kind: "met" };
  return { kind: "unmet" };
}

function runCheck(oracle) {
  const result = spawnSync(oracle.fields.CHECK, {
    shell: true,
    cwd: process.cwd(),
    encoding: "utf8",
    timeout: checkTimeoutMs(),
    env: process.env,
  });
  const timedOut = result.error?.code === "ETIMEDOUT";
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  const exit = timedOut ? "timeout" : (result.status ?? 1);
  const match = output.includes(oracle.fields.EXPECT);
  const met = exit === 0 && match;
  const sha256 = createHash("sha256").update(output).digest("hex").slice(0, 16);
  const bytes = Buffer.byteLength(output);
  const at = new Date().toISOString();
  const evidence = met
    ? `met exit=0 match=yes sha256=${sha256} bytes=${bytes} at=${at}`
    : `unmet exit=${exit} match=${match ? "yes" : "no"} bytes=${bytes} at=${at}`;
  return { met, evidence, exit, match };
}

function writeLedger(path, lines, oracles) {
  const next = lines.slice();
  const descending = [...oracles].sort((a, b) => b.startLine - a.startLine);
  for (const oracle of descending) {
    const box = recordedMet(oracle) ? "x" : " ";
    next[oracle.startLine] = `- [${box}] ${oracle.id}: ${oracle.title}`;
    const evidenceLine = `  EVIDENCE: ${oracle.fields.EVIDENCE ?? "pending"}`;
    if (oracle.fieldLines.EVIDENCE == null) {
      next.splice(oracle.endLine, 0, evidenceLine);
    } else {
      next[oracle.fieldLines.EVIDENCE] = evidenceLine;
    }
  }
  const tmp = `${path}.${process.pid}.tmp`;
  writeFileSync(tmp, `${next.join("\n")}\n`);
  renameSync(tmp, path);
}

function printReport(oracles) {
  const abandoned = [];
  const unmet = [];
  for (const oracle of oracles) {
    const state = stateOf(oracle);
    if (state.kind === "abandoned") {
      process.stdout.write(`${oracle.id} abandoned ${state.reason}\n`);
      abandoned.push(oracle.id);
      continue;
    }
    if (state.kind === "met") {
      process.stdout.write(`${oracle.id} met\n`);
      continue;
    }
    process.stdout.write(`${oracle.id} unmet\n`);
    unmet.push(oracle.id);
  }
  if (abandoned.length) {
    process.stdout.write(`HANDOFF REQUIRED ${abandoned.join(" ")}\n`);
    process.exit(1);
  }
  if (unmet.length) {
    process.stdout.write(`UNMET ${unmet.join(" ")}\n`);
    process.exit(1);
  }
  process.stdout.write("ALL MET\n");
}

function main() {
  const { mode, ledger } = parseArgs(process.argv.slice(2));
  if (!existsSync(ledger)) die(`missing ledger: ${ledger}`);
  const parsed = parseLedger(readFileSync(ledger, "utf8"));
  if (mode !== "status") {
    for (const oracle of parsed.oracles) {
      if (oracle.fields.ABANDON !== undefined) continue;
      if (mode === "run" && recordedMet(oracle)) continue;
      const result = runCheck(oracle);
      oracle.fields.EVIDENCE = result.evidence;
    }
    writeLedger(ledger, parsed.lines, parsed.oracles);
  }
  printReport(parsed.oracles);
}

main();
