import { loadConfig } from "../config/loadConfig.ts";
import { createJournal, resolveHistory } from "../journal/journal.ts";
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
  const journal = createJournal({
    historyPath: resolveHistory({ cwd: opts.cwd, path: config.history }),
  });
  const lanes = config.lanes.filter(
    (lane) => !config.disable.includes(lane.lane),
  );
  if (lanes.length === 0) {
    journal.record({ kind: "scan", notes: 0, quarantined: 0 });
    return;
  }
  const { notes, quarantines } = scan({ cwd: opts.cwd, config });
  journal.record({
    kind: "scan",
    notes: notes.length,
    quarantined: quarantines.length,
  });
  for (const item of quarantines) {
    journal.record({
      kind: "quarantine",
      path: item.path,
      fault: item.fault,
    });
  }
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
    journal,
  });
  await Promise.all(live.map((run) => run.wait));
}
