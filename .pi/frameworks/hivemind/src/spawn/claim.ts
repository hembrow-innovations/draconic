import { mkdirSync, readFileSync, rmdirSync, writeFileSync } from "node:fs";
import { parseYaml, type YamlValue } from "../yaml/yaml.ts";

export type ClaimResult = { kind: "claimed" } | { kind: "skipped" };

export function claim(opts: {
  abs: string;
  triggerStatus: YamlValue;
  claimStatus: string;
  runId: string;
}): ClaimResult {
  const lockPath = `${opts.abs}.claimlock`;
  if (!tryLock(lockPath)) return { kind: "skipped" };
  try {
    const raw = readFileSync(opts.abs, "utf8");
    const parsed = readFrontMatter(raw);
    if (parsed.kind === "fault") return { kind: "skipped" };
    if (!Object.is(parsed.map.status, opts.triggerStatus)) {
      return { kind: "skipped" };
    }
    writeFileSync(opts.abs, applyClaim(raw, opts.claimStatus, opts.runId));
    return { kind: "claimed" };
  } finally {
    rmdirSync(lockPath);
  }
}

function tryLock(lockPath: string): boolean {
  try {
    mkdirSync(lockPath);
    return true;
  } catch (err) {
    if (isAlreadyExists(err)) return false;
    throw err;
  }
}

function isAlreadyExists(err: unknown): boolean {
  return (
    typeof err === "object" &&
    err !== null &&
    "code" in err &&
    err.code === "EEXIST"
  );
}

function applyClaim(raw: string, claimStatus: string, runId: string): string {
  const match = raw.match(/^(---\r?\n)([\s\S]*?)(\r?\n---(?:\r?\n|$))/);
  if (
    match === null ||
    match[1] === undefined ||
    match[2] === undefined ||
    match[3] === undefined
  ) {
    return raw;
  }
  let front = match[2];
  front = upsertKey(front, "status", claimStatus);
  front = upsertKey(front, "claimed-by", runId);
  return `${match[1]}${front}${match[3]}${raw.slice(match[0].length)}`;
}

function upsertKey(front: string, key: string, value: string): string {
  const line = `${key}: ${value}`;
  const pattern = new RegExp(`^${key}:.*$`, "m");
  if (pattern.test(front)) return front.replace(pattern, line);
  if (front === "") return line;
  return `${front}\n${line}`;
}

function readFrontMatter(
  raw: string,
): { kind: "ok"; map: Record<string, YamlValue> } | { kind: "fault" } {
  if (!raw.startsWith("---")) return { kind: "fault" };
  const afterOpen = raw.slice(3);
  const close = afterOpen.match(/\r?\n---(?:\r?\n|$)/);
  if (close === null) return { kind: "fault" };
  const yamlText = afterOpen.slice(0, close.index).replace(/^\r?\n/, "");
  try {
    return { kind: "ok", map: parseYaml(yamlText) };
  } catch {
    return { kind: "fault" };
  }
}
