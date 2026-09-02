import { loadConfig } from "../config/loadConfig.ts";
import { matchNotes } from "../match/matcher.ts";
import { scan } from "../scan/scan.ts";
import { spawnMatches, type LiveRun, type SpawnChild } from "./matches.ts";

export type { LiveRun, SpawnChild };

export async function runOnce(opts: {
  cwd: string;
  spawnChild?: SpawnChild;
  env?: NodeJS.ProcessEnv;
}): Promise<void> {
  const config = loadConfig(opts.cwd);
  const lanes = config.lanes.filter(
    (lane) => !config.disable.includes(lane.lane),
  );
  if (lanes.length === 0) return;
  const { notes } = scan({ cwd: opts.cwd, config });
  const matches = matchNotes({ lanes, notes, disable: config.disable });
  const env = opts.env ?? process.env;
  const live: LiveRun[] = [];
  spawnMatches({
    cwd: opts.cwd,
    concurrency: config.concurrency,
    matches,
    env,
    spawnChild: opts.spawnChild,
    live,
  });
  await Promise.all(live.map((run) => run.wait));
}
