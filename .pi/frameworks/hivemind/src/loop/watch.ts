import { existsSync, watch, type FSWatcher } from "node:fs";
import { dirname, isAbsolute, join } from "node:path";
import { loadConfig, type Lane } from "../config/loadConfig.ts";
import { createJournal, resolveHistory } from "../journal/journal.ts";
import type { LiveRun, SpawnChild } from "./matches.ts";
import { runTick } from "./tick.ts";

export async function runWatch(opts: {
  cwd: string;
  untilQuiet?: boolean;
  untilTarget?: string;
  spawnChild?: SpawnChild;
  env?: NodeJS.ProcessEnv;
  signal?: AbortSignal;
}): Promise<void> {
  const config = loadConfig(opts.cwd);
  const journal = createJournal({
    historyPath: resolveHistory({ cwd: opts.cwd, path: config.history }),
  });
  const lanes = config.lanes.filter(
    (lane) => !config.disable.includes(lane.lane),
  );
  const target = resolveTarget(opts.cwd, opts.untilTarget);
  const env = opts.env ?? process.env;
  const live: LiveRun[] = [];
  const lastFinished = new Map<string, number>();
  try {
    while (opts.signal?.aborted !== true) {
      if (target !== undefined && existsSync(target)) return;
      const spawned = runTick({
        cwd: opts.cwd,
        config,
        lanes,
        env,
        spawnChild: opts.spawnChild,
        live,
        journal,
        lastFinished,
      });
      await Promise.resolve();
      const remaining = live.filter((run) => !run.done);
      if (opts.untilQuiet === true && spawned === 0 && remaining.length === 0) {
        return;
      }
      if (target !== undefined && existsSync(target)) return;
      await waitForWake({
        cwd: opts.cwd,
        target,
        live: remaining,
        backoffMs: maxBackoffMs(lanes),
        quiet: spawned === 0 && remaining.length === 0,
        signal: opts.signal,
      });
    }
  } finally {
    for (const run of live) {
      if (!run.done) run.kill();
    }
  }
}

function maxBackoffMs(lanes: readonly Lane[]): number {
  let max = 0;
  for (const lane of lanes) {
    if (lane.backoffMs > max) max = lane.backoffMs;
  }
  return max;
}

function resolveTarget(
  cwd: string,
  path: string | undefined,
): string | undefined {
  if (path === undefined || path === "") return undefined;
  return isAbsolute(path) ? path : join(cwd, path);
}

function waitForWake(opts: {
  cwd: string;
  target: string | undefined;
  live: readonly LiveRun[];
  backoffMs: number;
  quiet: boolean;
  signal?: AbortSignal;
}): Promise<void> {
  if (opts.signal?.aborted === true) return Promise.resolve();
  return new Promise((resolve) => {
    let done = false;
    const watchers: FSWatcher[] = [];
    let timer: ReturnType<typeof setTimeout> | undefined;
    const finish = () => {
      if (done) return;
      done = true;
      if (timer !== undefined) clearTimeout(timer);
      for (const watcher of watchers) watcher.close();
      opts.signal?.removeEventListener("abort", finish);
      resolve();
    };
    const useBackoff = opts.quiet && opts.backoffMs > 0;
    if (useBackoff) {
      timer = setTimeout(finish, opts.backoffMs);
      watchTarget(opts.target, watchers, finish);
    } else {
      const paths = new Set<string>([opts.cwd]);
      if (opts.target !== undefined) paths.add(dirname(opts.target));
      for (const path of paths) {
        if (!existsSync(path)) continue;
        watchers.push(watch(path, finish));
      }
      if (opts.target !== undefined && existsSync(opts.target)) finish();
    }
    for (const run of opts.live) {
      void run.wait.then(finish, finish);
    }
    opts.signal?.addEventListener("abort", finish, { once: true });
  });
}

function watchTarget(
  target: string | undefined,
  watchers: FSWatcher[],
  finish: () => void,
): void {
  if (target === undefined) return;
  const dir = dirname(target);
  if (existsSync(dir)) {
    watchers.push(
      watch(dir, () => {
        if (existsSync(target)) finish();
      }),
    );
  }
  if (existsSync(target)) finish();
}
