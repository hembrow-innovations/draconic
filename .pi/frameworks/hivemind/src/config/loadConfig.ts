import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parseYaml, type YamlValue } from "../yaml/yaml.ts";

const CONFIG_KEYS = new Set([
  "concurrency",
  "watch",
  "folders",
  "lanes",
  "disable",
  "history",
]);

export type Lane = {
  lane: string;
  cmd: string | readonly string[];
  trigger: Record<string, YamlValue>;
  claimStatus: string;
  need: Record<string, YamlValue> | undefined;
  agent: string | undefined;
  prompt: string | undefined;
  exclusive: readonly string[];
  scalars: Record<string, string>;
  backoffMs: number;
};

export type HivemindConfig = {
  folders: YamlValue[];
  lanes: Lane[];
  concurrency: number;
  disable: readonly string[];
  watch: readonly string[] | undefined;
  history: string | undefined;
};

export function loadConfig(cwd: string): HivemindConfig {
  const file = join(cwd, "hivemind.yaml");
  if (!existsSync(file)) {
    throw new Error("Missing hivemind.yaml");
  }
  const raw = parseYaml(readFileSync(file, "utf8"));
  for (const key of Object.keys(raw)) {
    if (!CONFIG_KEYS.has(key)) {
      throw new Error(`Unknown key "${key}"`);
    }
  }
  return {
    folders: asList(raw.folders, "folders"),
    lanes: parseLanes(asList(raw.lanes, "lanes")),
    concurrency: parseConcurrency(raw.concurrency),
    disable: parseStringList(raw.disable, "disable"),
    watch: parseWatch(raw.watch),
    history: parseHistory(raw.history),
  };
}

function parseHistory(value: unknown): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "string" || value === "") {
    throw new Error("history must be a path");
  }
  return value;
}

function parseWatch(value: unknown): string[] | undefined {
  if (value === undefined) return undefined;
  const list = parseStringList(value, "watch");
  if (list.length === 0) return undefined;
  return list.map(normalizeWatchRoot);
}

function normalizeWatchRoot(path: string): string {
  return path.replace(/\/\*\*(\/\*\.md)?$/, "").replace(/\/+$/, "");
}

function asList(value: unknown, key: string): YamlValue[] {
  if (value === undefined) {
    throw new Error(`"${key}" is required`);
  }
  if (!Array.isArray(value)) {
    throw new Error(`"${key}" must be a list`);
  }
  return value;
}

function parseConcurrency(value: unknown): number {
  if (value === undefined) return 1;
  if (typeof value !== "number" || !Number.isInteger(value) || value < 1) {
    throw new Error("concurrency must be a positive integer");
  }
  return value;
}

function parseStringList(value: unknown, key: string): string[] {
  if (value === undefined) return [];
  if (!Array.isArray(value)) {
    throw new Error(`"${key}" must be a list of strings`);
  }
  const out: string[] = [];
  for (const item of value) {
    if (typeof item !== "string") {
      throw new Error(`"${key}" must be a list of strings`);
    }
    out.push(item);
  }
  return out;
}

function parseLanes(items: YamlValue[]): Lane[] {
  const lanes: Lane[] = [];
  const ids = new Set<string>();
  for (const item of items) {
    const lane = parseLane(item);
    if (ids.has(lane.lane)) {
      throw new Error(`Duplicate lane "${lane.lane}"`);
    }
    ids.add(lane.lane);
    lanes.push(lane);
  }
  return lanes;
}

function parseLane(item: YamlValue): Lane {
  if (item === null || typeof item !== "object" || Array.isArray(item)) {
    throw new Error("lanes entries must be maps");
  }
  if (typeof item.lane !== "string" || item.lane === "") {
    throw new Error("lane id is required");
  }
  const cmd = parseCmd(item.cmd);
  const trigger = parseMap(item.trigger, "trigger");
  if (typeof item["claim-status"] !== "string" || item["claim-status"] === "") {
    throw new Error(`lane "${item.lane}" is missing claim-status`);
  }
  const scope = parseStringList(item.scope, "scope");
  const exclusive = parseStringList(item.exclusive, "exclusive");
  if (item.scope !== undefined && item.exclusive !== undefined) {
    if (!sameStrings(scope, exclusive)) {
      throw new Error(`lane "${item.lane}" exclusive and scope must be equal`);
    }
  }
  const scalars: Record<string, string> = {};
  for (const [key, value] of Object.entries(item)) {
    if (typeof value === "string") scalars[key] = value;
  }
  return {
    lane: item.lane,
    cmd,
    trigger,
    claimStatus: item["claim-status"],
    need: item.need === undefined ? undefined : parseMap(item.need, "need"),
    agent: optionalString(item.agent),
    prompt: optionalString(item.prompt),
    exclusive: exclusive.length > 0 ? exclusive : scope,
    scalars,
    backoffMs: parseBackoff(item.backoff, item.lane),
  };
}

function parseBackoff(value: unknown, laneId: string): number {
  if (value === undefined || value === 0 || value === "0") return 0;
  if (typeof value === "string") {
    const match = value.match(/^(\d+)s$/);
    if (match !== null && match[1] !== undefined) {
      return Number(match[1]) * 1000;
    }
  }
  throw new Error(`lane "${laneId}" backoff is invalid`);
}

function parseCmd(value: unknown): string | readonly string[] {
  if (typeof value === "string" && value !== "") return value;
  if (Array.isArray(value) && value.length > 0) {
    const parts: string[] = [];
    for (const item of value) {
      if (typeof item !== "string") {
        throw new Error("cmd list items must be strings");
      }
      parts.push(item);
    }
    return parts;
  }
  throw new Error("cmd is required");
}

function parseMap(
  value: YamlValue | undefined,
  key: string,
): Record<string, YamlValue> {
  if (
    value === null ||
    value === undefined ||
    typeof value !== "object" ||
    Array.isArray(value)
  ) {
    throw new Error(`"${key}" must be a map`);
  }
  return value;
}

function optionalString(value: unknown): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "string") {
    throw new Error("expected a string");
  }
  return value;
}

function sameStrings(a: readonly string[], b: readonly string[]): boolean {
  if (a.length !== b.length) return false;
  return a.every((item, i) => item === b[i]);
}
