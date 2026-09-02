import type { Lane } from "../config/loadConfig.ts";

export type InterpolateResult =
  | { kind: "ok"; value: string }
  | { kind: "skip" };

export function interpolate(opts: {
  template: string;
  cwd: string;
  lane: Lane;
  env: NodeJS.ProcessEnv;
}): InterpolateResult {
  let skip = false;
  const value = opts.template.replace(
    /\{\{([^}]*)\}\}/g,
    (_full, name: string) => {
      if (skip) return "";
      const resolved = resolvePlaceholder({
        name,
        cwd: opts.cwd,
        lane: opts.lane,
        env: opts.env,
      });
      if (resolved.kind === "skip") {
        skip = true;
        return "";
      }
      return resolved.value;
    },
  );
  if (skip) return { kind: "skip" };
  if (value.includes("{{")) return { kind: "skip" };
  return { kind: "ok", value };
}

function resolvePlaceholder(opts: {
  name: string;
  cwd: string;
  lane: Lane;
  env: NodeJS.ProcessEnv;
}): InterpolateResult {
  if (opts.name.startsWith("env.")) {
    const key = opts.name.slice("env.".length);
    if (key === "") return { kind: "skip" };
    const value = opts.env[key];
    if (value === undefined || value === "") return { kind: "skip" };
    return { kind: "ok", value };
  }
  if (opts.name === "cwd") return { kind: "ok", value: opts.cwd };
  if (opts.name === "lane") return { kind: "ok", value: opts.lane.lane };
  if (opts.name === "agent") {
    if (opts.lane.agent === undefined || opts.lane.agent === "") {
      return { kind: "skip" };
    }
    return { kind: "ok", value: opts.lane.agent };
  }
  if (opts.name === "prompt") {
    if (opts.lane.prompt === undefined || opts.lane.prompt === "") {
      return { kind: "skip" };
    }
    return { kind: "ok", value: opts.lane.prompt };
  }
  if (opts.name === "exclusive") {
    return { kind: "ok", value: opts.lane.exclusive.join(" ") };
  }
  const named = opts.lane.scalars[opts.name];
  if (named === undefined) return { kind: "skip" };
  return { kind: "ok", value: named };
}
