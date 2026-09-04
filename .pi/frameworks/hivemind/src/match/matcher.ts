import type { Lane } from "../config/loadConfig.ts";
import type { ScannedNote } from "../scan/scan.ts";
import type { YamlValue } from "../yaml/yaml.ts";

export type Match = {
  lane: Lane;
  note: ScannedNote;
};

export function matchNotes(opts: {
  lanes: readonly Lane[];
  notes: readonly ScannedNote[];
  disable?: readonly string[];
}): Match[] {
  const disabled = new Set(opts.disable ?? []);
  const matches: Match[] = [];
  for (const lane of opts.lanes) {
    if (disabled.has(lane.lane)) continue;
    for (const note of opts.notes) {
      if (!matchesPredicates(note.frontMatter, lane.trigger)) continue;
      if (
        lane.need !== undefined &&
        !matchesPredicates(note.frontMatter, lane.need)
      ) {
        continue;
      }
      matches.push({ lane, note });
    }
  }
  return matches;
}

function matchesPredicates(
  frontMatter: Record<string, YamlValue>,
  predicates: Record<string, YamlValue>,
): boolean {
  for (const [key, expected] of Object.entries(predicates)) {
    if (!yamlEqual(frontMatter[key], expected)) return false;
  }
  return true;
}

function yamlEqual(left: YamlValue | undefined, right: YamlValue): boolean {
  if (Object.is(left, right)) return true;
  if (left === undefined || left === null || right === null) return false;
  if (Array.isArray(left) || Array.isArray(right)) return false;
  if (typeof left === "object" || typeof right === "object") return false;
  return false;
}
