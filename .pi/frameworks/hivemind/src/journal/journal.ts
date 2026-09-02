import { appendFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { dirname, isAbsolute, join } from "node:path";

export type HistoryEvent =
  | { kind: "scan"; notes: number; quarantined: number }
  | { kind: "quarantine"; path: string; fault: string }
  | { kind: "skip"; lane: string; path: string; reason: string }
  | { kind: "claim"; lane: string; path: string; runId: string }
  | { kind: "spawn"; lane: string; path: string; runId: string }
  | { kind: "exit"; lane: string; path: string; runId: string; status: number };

export type Journal = {
  record(event: HistoryEvent): void;
};

const HEADER = "ts\taction\tlane\tpath\trun_id\tdetail\n";

export function createJournal(opts: {
  historyPath?: string;
  writeLine?: (line: string) => void;
  now?: () => Date;
}): Journal {
  const writeLine =
    opts.writeLine ??
    ((line: string) => {
      process.stderr.write(`${line}\n`);
    });
  const now = opts.now ?? (() => new Date());
  return {
    record(event) {
      const ts = now().toISOString();
      writeLine(formatLine(event));
      if (opts.historyPath === undefined || opts.historyPath === "") return;
      writeRow({ path: opts.historyPath, ts, event });
    },
  };
}

export function resolveHistory(opts: {
  cwd: string;
  path: string | undefined;
}): string | undefined {
  if (opts.path === undefined || opts.path === "") return undefined;
  return isAbsolute(opts.path) ? opts.path : join(opts.cwd, opts.path);
}

function formatLine(event: HistoryEvent): string {
  switch (event.kind) {
    case "scan":
      return `hivemind scan notes=${event.notes} quarantined=${event.quarantined}`;
    case "quarantine":
      return `hivemind quarantine ${event.path} ${event.fault}`;
    case "skip":
      return `hivemind skip ${event.lane} ${event.path} ${event.reason}`;
    case "claim":
      return `hivemind claim ${event.lane} ${event.path}`;
    case "spawn":
      return `hivemind spawn ${event.lane} ${event.path}`;
    case "exit":
      return `hivemind exit ${event.lane} ${event.path} status=${event.status}`;
    default: {
      const _exhaustive: never = event;
      return _exhaustive;
    }
  }
}

function writeRow(opts: {
  path: string;
  ts: string;
  event: HistoryEvent;
}): void {
  mkdirSync(dirname(opts.path), { recursive: true });
  if (!hasHeader(opts.path)) {
    appendFileSync(opts.path, HEADER);
  }
  const row = tsvRow(opts.ts, opts.event);
  appendFileSync(opts.path, `${row}\n`);
}

function hasHeader(path: string): boolean {
  if (!existsSync(path)) return false;
  return statSync(path).size > 0;
}

function tsvRow(ts: string, event: HistoryEvent): string {
  const fields = [ts, event.kind, "", "", "", ""];
  switch (event.kind) {
    case "scan":
      fields[5] = `notes=${event.notes} quarantined=${event.quarantined}`;
      break;
    case "quarantine":
      fields[3] = event.path;
      fields[5] = event.fault;
      break;
    case "skip":
      fields[2] = event.lane;
      fields[3] = event.path;
      fields[5] = event.reason;
      break;
    case "claim":
    case "spawn":
      fields[2] = event.lane;
      fields[3] = event.path;
      fields[4] = event.runId;
      break;
    case "exit":
      fields[2] = event.lane;
      fields[3] = event.path;
      fields[4] = event.runId;
      fields[5] = `status=${event.status}`;
      break;
    default: {
      const _exhaustive: never = event;
      return _exhaustive;
    }
  }
  return fields.map(tsvField).join("\t");
}

function tsvField(value: string): string {
  return value
    .replaceAll("\t", " ")
    .replaceAll("\r", " ")
    .replaceAll("\n", " ");
}
