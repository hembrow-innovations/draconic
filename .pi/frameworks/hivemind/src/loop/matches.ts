import { randomUUID } from "node:crypto";
import { join } from "node:path";
import type { Lane, SpawnSpec } from "../config/loadConfig.ts";
import type { Journal } from "../journal/journal.ts";
import type { Match } from "../match/matcher.ts";
import { claim } from "../note/io.ts";
import { renderArgv, startSpawn, type SpawnChild } from "../spawn/spawner.ts";

export type { SpawnChild };

export type LiveRun = {
  exclusive: readonly string[];
  wait: Promise<number>;
  kill: () => void;
  done: boolean;
  path: string;
  lane: string;
  runId: string;
};

export function spawnMatches(opts: {
  cwd: string;
  matches: readonly Match[];
  env: NodeJS.ProcessEnv;
  spawnChild?: SpawnChild;
  live: LiveRun[];
  journal?: Journal;
  lastFinished?: Map<string, number>;
  now?: () => number;
}): number {
  let spawned = 0;
  const now = opts.now ?? Date.now;
  for (const match of opts.matches) {
    const current = opts.live.filter((run) => !run.done);
    const laneLive = current.filter((run) => run.lane === match.lane.lane);
    if (laneLive.length >= match.lane.concurrency) {
      opts.journal?.record({
        kind: "skip",
        lane: match.lane.lane,
        path: match.note.path,
        reason: "concurrency",
      });
      continue;
    }
    if (current.some((run) => run.path === match.note.path)) {
      opts.journal?.record({
        kind: "skip",
        lane: match.lane.lane,
        path: match.note.path,
        reason: "live",
      });
      continue;
    }
    if (
      current.some((run) =>
        exclusiveSetsOverlap(match.lane.exclusive, run.exclusive),
      )
    ) {
      opts.journal?.record({
        kind: "skip",
        lane: match.lane.lane,
        path: match.note.path,
        reason: "exclusive",
      });
      continue;
    }
    if (match.lane.cooldownMs > 0 && opts.lastFinished !== undefined) {
      const last = opts.lastFinished.get(match.lane.lane);
      if (last !== undefined && now() - last < match.lane.cooldownMs) {
        opts.journal?.record({
          kind: "skip",
          lane: match.lane.lane,
          path: match.note.path,
          reason: "cooldown",
        });
        continue;
      }
    }
    const runId = randomUUID();
    const rendered = renderArgv({
      specs: spawnSpecs(match.lane),
      lane: match.lane.lane,
      cwd: opts.cwd,
      env: opts.env,
      runId,
    });
    if (rendered.kind === "skip") {
      opts.journal?.record({
        kind: "skip",
        lane: match.lane.lane,
        path: match.note.path,
        reason: rendered.reason,
      });
      continue;
    }
    const taken = claim({
      abs: join(opts.cwd, match.note.path),
      triggerStatus: match.lane.trigger.status,
      claimStatus: match.lane.claimStatus,
      runId,
    });
    if (taken.kind === "skipped") {
      opts.journal?.record({
        kind: "skip",
        lane: match.lane.lane,
        path: match.note.path,
        reason: "claim-race",
      });
      continue;
    }
    opts.journal?.record({
      kind: "claim",
      lane: match.lane.lane,
      path: match.note.path,
      runId,
    });
    const handle = startSpawn({
      argvList: rendered.argvList,
      cwd: opts.cwd,
      env: opts.env,
      spawnChild: opts.spawnChild,
      journal: opts.journal,
      lane: match.lane.lane,
      path: match.note.path,
      runId,
      stages: spawnStages(match.lane),
    });
    opts.live.push(
      track({
        exclusive: match.lane.exclusive,
        wait: handle.wait,
        kill: handle.kill,
        path: match.note.path,
        lane: match.lane.lane,
        runId,
        lastFinished: opts.lastFinished,
      }),
    );
    spawned += 1;
  }
  return spawned;
}

function exclusiveSetsOverlap(
  left: readonly string[],
  right: readonly string[],
): boolean {
  for (const a of left) {
    for (const b of right) {
      if (pathsOverlap(a, b)) return true;
    }
  }
  return false;
}

function pathsOverlap(left: string, right: string): boolean {
  const a = normalizePrefix(left);
  const b = normalizePrefix(right);
  return a === b || a.startsWith(`${b}/`) || b.startsWith(`${a}/`);
}

function normalizePrefix(path: string): string {
  return path.replaceAll("\\", "/").replace(/\/+$/, "");
}

function spawnSpecs(lane: Lane): SpawnSpec[] {
  if (lane.type === "single") return [lane];
  return [...lane.stages];
}

function spawnStages(lane: Lane): readonly string[] | undefined {
  if (lane.type === "single") return undefined;
  return lane.stages.map((stage) => stage.stage);
}

function track(
  run: Omit<LiveRun, "done"> & {
    lastFinished?: Map<string, number>;
  },
): LiveRun {
  const live: LiveRun = {
    exclusive: run.exclusive,
    wait: run.wait,
    kill: run.kill,
    done: false,
    path: run.path,
    lane: run.lane,
    runId: run.runId,
  };
  void live.wait.then(
    () => {
      live.done = true;
      run.lastFinished?.set(live.lane, Date.now());
    },
    () => {
      live.done = true;
      run.lastFinished?.set(live.lane, Date.now());
    },
  );
  return live;
}
