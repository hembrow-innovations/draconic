import { spawn } from "node:child_process";

export type SpawnHandle = {
  wait: Promise<number>;
  kill: () => void;
};

export function spawnArgv(opts: {
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
  };
}

function noop(): void {}
