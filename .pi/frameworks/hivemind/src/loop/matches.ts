import { randomUUID } from "node:crypto";
import { existsSync } from "node:fs";
import { join } from "node:path";
import type { Lane } from "../config/loadConfig.ts";
import type { Journal } from "../journal/journal.ts";
import { exclusiveSetsOverlap, type Match } from "../match/matcher.ts";
import { claim } from "../spawn/claim.ts";
import { interpolate } from "../spawn/interpolator.ts";
import { spawnArgv } from "../spawn/spawner.ts";
import { tokenize } from "../spawn/tokenizer.ts";

export type SpawnChild = (argv: readonly string[]) => unknown;

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
  concurrency: number;
  matches: readonly Match[];
  env: NodeJS.ProcessEnv;
  spawnChild?: SpawnChild;
  live: LiveRun[];
  journal?: Journal;
}): number {
  let spawned = 0;
  for (const match of opts.matches) {
    const current = opts.live.filter((run) => !run.done);
    if (current.length >= opts.concurrency) {
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
    const argv = cmdArgv({ lane: match.lane, cwd: opts.cwd, env: opts.env });
    if (argv.kind === "skip") {
      opts.journal?.record({
        kind: "skip",
        lane: match.lane.lane,
        path: match.note.path,
        reason: argv.reason,
      });
      continue;
    }
    const runId = randomUUID();
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
    opts.journal?.record({
      kind: "spawn",
      lane: match.lane.lane,
      path: match.note.path,
      runId,
    });
    if (opts.spawnChild !== undefined) {
      opts.spawnChild(argv.argv);
      opts.live.push(
        track({
          exclusive: match.lane.exclusive,
          wait: Promise.resolve(0),
          kill: noop,
          path: match.note.path,
          lane: match.lane.lane,
          runId,
          journal: opts.journal,
        }),
      );
      spawned += 1;
      continue;
    }
    const handle = spawnArgv({
      argv: argv.argv,
      cwd: opts.cwd,
      env: opts.env,
    });
    opts.live.push(
      track({
        exclusive: match.lane.exclusive,
        wait: handle.wait,
        kill: handle.kill,
        path: match.note.path,
        lane: match.lane.lane,
        runId,
        journal: opts.journal,
      }),
    );
    spawned += 1;
  }
  return spawned;
}

function track(run: Omit<LiveRun, "done"> & { journal?: Journal }): LiveRun {
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
    (status) => {
      live.done = true;
      run.journal?.record({
        kind: "exit",
        lane: live.lane,
        path: live.path,
        runId: live.runId,
        status,
      });
    },
    () => {
      live.done = true;
      run.journal?.record({
        kind: "exit",
        lane: live.lane,
        path: live.path,
        runId: live.runId,
        status: 1,
      });
    },
  );
  return live;
}

function noop(): void {}

function cmdArgv(opts: {
  lane: Lane;
  cwd: string;
  env: NodeJS.ProcessEnv;
}):
  | { kind: "ok"; argv: string[] }
  | { kind: "skip"; reason: "missing-prompt" | "cmd-skip" } {
  if (opts.lane.prompt !== undefined && opts.lane.prompt !== "") {
    if (!existsSync(join(opts.cwd, opts.lane.prompt))) {
      return { kind: "skip", reason: "missing-prompt" };
    }
  }
  if (typeof opts.lane.cmd !== "string") {
    const argv: string[] = [];
    for (const part of opts.lane.cmd) {
      const rendered = interpolate({
        template: part,
        cwd: opts.cwd,
        lane: opts.lane,
        env: opts.env,
      });
      if (rendered.kind === "skip") {
        return { kind: "skip", reason: "cmd-skip" };
      }
      argv.push(rendered.value);
    }
    return { kind: "ok", argv };
  }
  const rendered = interpolate({
    template: opts.lane.cmd,
    cwd: opts.cwd,
    lane: opts.lane,
    env: opts.env,
  });
  if (rendered.kind === "skip") return { kind: "skip", reason: "cmd-skip" };
  const tokens = tokenize(rendered.value);
  if (tokens.kind === "fail") return { kind: "skip", reason: "cmd-skip" };
  return { kind: "ok", argv: tokens.argv };
}
