#!/usr/bin/env node
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { runOnce } from "./loop/once.ts";
import { runWatch } from "./loop/watch.ts";
import type { SpawnChild } from "./loop/matches.ts";

export type { SpawnChild };

export async function run(opts: {
  argv: readonly string[];
  cwd: string;
  spawnChild?: SpawnChild;
  signal?: AbortSignal;
}): Promise<number> {
  const command = opts.argv[0];
  if (
    command === undefined ||
    command === "-h" ||
    command === "--help" ||
    command === "help"
  ) {
    usage();
    return 0;
  }
  if (command !== "once" && command !== "watch") {
    console.error(`Unknown command: ${command}`);
    return 1;
  }
  try {
    if (command === "watch") {
      const flags = parseWatchFlags(opts.argv.slice(1));
      await runWatch({
        cwd: opts.cwd,
        untilQuiet: flags.untilQuiet,
        untilTarget: flags.untilTarget,
        spawnChild: opts.spawnChild,
        signal: opts.signal,
      });
    } else {
      await runOnce({
        cwd: opts.cwd,
        spawnChild: opts.spawnChild,
      });
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error(message);
    return 1;
  }
  return 0;
}

function parseWatchFlags(argv: readonly string[]): {
  untilQuiet: boolean;
  untilTarget: string | undefined;
} {
  let untilQuiet = false;
  let untilTarget: string | undefined;
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--until-quiet") {
      untilQuiet = true;
      continue;
    }
    if (arg === "--until-target") {
      const path = argv[i + 1];
      if (path === undefined || path === "") {
        throw new Error("--until-target requires a path");
      }
      untilTarget = path;
      i += 1;
      continue;
    }
    throw new Error(`Unknown flag: ${arg}`);
  }
  return { untilQuiet, untilTarget };
}

function usage(): void {
  console.log(`hivemind

Usage:
  node --experimental-strip-types apps/hivemind/src/cli.ts <command>

Commands:
  once    one scan, spawn matches, wait, exit
  watch   resident predicate loop

Options:
  -h, --help              Show help
  --until-quiet           watch: exit after one quiet scan
  --until-target PATH     watch: exit when PATH exists

Events print to stderr. Optional history TSV is set in .hivemind/hivemind.yaml.
`);
}

const entry = process.argv[1];
if (entry !== undefined && fileURLToPath(import.meta.url) === resolve(entry)) {
  const ac = new AbortController();
  const onStop = () => ac.abort();
  process.on("SIGTERM", onStop);
  process.on("SIGINT", onStop);
  const status = await run({
    argv: process.argv.slice(2),
    cwd: process.cwd(),
    signal: ac.signal,
  });
  process.off("SIGTERM", onStop);
  process.off("SIGINT", onStop);
  process.exit(status);
}
