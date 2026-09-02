import { mkdirSync, renameSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

export function quarantineFile(opts: {
  abs: string;
  destDir: string;
  origin: string;
  fault: string;
  at: string;
}): void {
  mkdirSync(opts.destDir, { recursive: true });
  const dest = join(opts.destDir, basename(opts.abs));
  renameSync(opts.abs, dest);
  writeFileSync(
    dest,
    `---\norigin-location: ${opts.origin}\nquarantined-at: ${opts.at}\nfault: ${opts.fault}\n---\n`,
  );
}
