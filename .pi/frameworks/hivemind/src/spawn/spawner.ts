import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import type { SpawnSpec } from "../config/loadConfig.ts";
import type { Journal } from "../journal/journal.ts";
import { interpolate } from "./interpolator.ts";
import { tokenize } from "./tokenizer.ts";

export type SpawnHandle = {
  wait: Promise<number>;
  kill: () => void;
  pid?: number;
};

export type SpawnChild = (opts: {
  argv: readonly string[];
  cwd: string;
  env?: NodeJS.ProcessEnv;
}) => SpawnHandle;

export type RenderArgvResult =
  | { kind: "ok"; argvList: string[][] }
  | { kind: "skip"; reason: "missing-prompt" | "cmd-skip" };

export function renderArgv(opts: {
  specs: readonly SpawnSpec[];
  cwd: string;
  lane: string;
  env: NodeJS.ProcessEnv;
  runId?: string;
}): RenderArgvResult {
  const argvList: string[][] = [];
  for (const spec of opts.specs) {
    const argv = cmdArgv({
      spec,
      lane: opts.lane,
      cwd: opts.cwd,
      env: opts.env,
      runId: opts.runId,
    });
    if (argv.kind === "skip") return argv;
    argvList.push(argv.argv);
  }
  return { kind: "ok", argvList };
}

export function startSpawn(opts: {
  argvList: readonly (readonly string[])[];
  cwd: string;
  env: NodeJS.ProcessEnv;
  spawnChild?: SpawnChild;
  journal?: Journal;
  lane: string;
  path: string;
  runId: string;
  stages?: readonly (string | undefined)[];
}): SpawnHandle {
  const spawnChild = opts.spawnChild ?? spawnArgv;
  let currentKill = noop;
  let cancelled = false;
  const wait = (async () => {
    let index = 0;
    for (const argv of opts.argvList) {
      if (cancelled) return 1;
      const handle = spawnChild({
        argv,
        cwd: opts.cwd,
        env: {
          ...opts.env,
          HIVEMIND_RUN_ID: opts.runId,
          HIVEMIND_LANE: opts.lane,
          HIVEMIND_PATH: opts.path,
        },
      });
      currentKill = handle.kill;
      const identity = spawnExitIdentity({
        stage: opts.stages?.[index],
        pid: handle.pid,
      });
      opts.journal?.record({
        kind: "spawn",
        lane: opts.lane,
        path: opts.path,
        runId: opts.runId,
        ...identity,
      });
      let status: number;
      try {
        status = await handle.wait;
      } catch {
        status = 1;
      }
      opts.journal?.record({
        kind: "exit",
        lane: opts.lane,
        path: opts.path,
        runId: opts.runId,
        status,
        ...identity,
      });
      index += 1;
      if (status !== 0) return status;
    }
    return 0;
  })();
  return {
    wait,
    kill: () => {
      cancelled = true;
      currentKill();
    },
  };
}

function spawnArgv(opts: {
  argv: readonly string[];
  cwd: string;
  env?: NodeJS.ProcessEnv;
}): SpawnHandle {
  const command = opts.argv[0];
  if (command === undefined) {
    return { wait: Promise.resolve(0), kill: noop };
  }
  const args = opts.argv.slice(1);
  const child = spawn(command, args, {
    cwd: opts.cwd,
    env: opts.env,
    shell: false,
    stdio: "inherit",
  });
  const wait = new Promise<number>((resolve, reject) => {
    child.on("error", reject);
    child.on("exit", (code) => {
      resolve(code ?? 1);
    });
  });
  return {
    wait,
    kill: () => {
      if (child.exitCode === null && child.signalCode === null) {
        child.kill("SIGTERM");
      }
    },
    pid: child.pid,
  };
}

function spawnExitIdentity(opts: {
  stage: string | undefined;
  pid: number | undefined;
}): { stage?: string; pid?: number } {
  const identity: { stage?: string; pid?: number } = {};
  if (opts.stage !== undefined && opts.stage !== "") {
    identity.stage = opts.stage;
  }
  if (opts.pid !== undefined) identity.pid = opts.pid;
  return identity;
}

function cmdArgv(opts: {
  spec: SpawnSpec;
  lane: string;
  cwd: string;
  env: NodeJS.ProcessEnv;
  runId?: string;
}):
  | { kind: "ok"; argv: string[] }
  | { kind: "skip"; reason: "missing-prompt" | "cmd-skip" } {
  if (opts.spec.prompt !== undefined && opts.spec.prompt !== "") {
    if (!existsSync(join(opts.cwd, opts.spec.prompt))) {
      return { kind: "skip", reason: "missing-prompt" };
    }
  }
  if (typeof opts.spec.cmd !== "string") {
    const argv: string[] = [];
    for (const part of opts.spec.cmd) {
      const rendered = interpolate({
        template: part,
        cwd: opts.cwd,
        lane: opts.lane,
        spec: opts.spec,
        env: opts.env,
        runId: opts.runId,
      });
      if (rendered.kind === "skip") {
        return { kind: "skip", reason: "cmd-skip" };
      }
      argv.push(rendered.value);
    }
    return { kind: "ok", argv };
  }
  const rendered = interpolate({
    template: opts.spec.cmd,
    cwd: opts.cwd,
    lane: opts.lane,
    spec: opts.spec,
    env: opts.env,
    runId: opts.runId,
  });
  if (rendered.kind === "skip") return { kind: "skip", reason: "cmd-skip" };
  const tokens = tokenize(rendered.value);
  if (tokens.kind === "fail") return { kind: "skip", reason: "cmd-skip" };
  return { kind: "ok", argv: tokens.argv };
}

function noop(): void {}
