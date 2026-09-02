import { existsSync, readdirSync, readFileSync, type Dirent } from "node:fs";
import { join, relative, sep } from "node:path";
import { loadConfig, type HivemindConfig } from "../config/loadConfig.ts";
import { quarantineFile } from "../quarantine/write.ts";
import { namedAllowlist } from "../schema/named.ts";
import { parseYaml, type YamlValue } from "../yaml/yaml.ts";

export type ScannedNote = {
  path: string;
  frontMatter: Record<string, YamlValue>;
};

export type QuarantinedNote = {
  path: string;
  fault: string;
};

export type ScanResult = {
  notes: ScannedNote[];
  quarantines: QuarantinedNote[];
};

type FolderSchema =
  | { kind: "named"; name: string }
  | { kind: "inline"; allowedKeys: ReadonlySet<string> };

type FolderEntry = {
  path: string;
  schema: FolderSchema;
  required: readonly string[];
};

export function scan(opts: {
  cwd: string;
  config?: HivemindConfig;
  now?: Date;
}): ScanResult {
  const config = opts.config ?? loadConfig(opts.cwd);
  const now = opts.now ?? new Date();
  const folders = readFolders(config.folders);
  const quarantine = folders.find(isQuarantineFolder);
  if (quarantine === undefined) {
    throw new Error("No quarantine folder configured");
  }

  const notes: ScannedNote[] = [];
  const quarantines: QuarantinedNote[] = [];
  const destDir = join(opts.cwd, quarantine.path);
  const at = now.toISOString();
  const watch = config.watch;
  for (const folder of folders) {
    if (isQuarantineFolder(folder)) continue;
    if (!includeFolder(folder.path, watch)) continue;
    const dir = join(opts.cwd, folder.path);
    if (!existsSync(dir)) continue;
    for (const abs of listMarkdownFiles(dir)) {
      const origin = projectRel(opts.cwd, abs);
      const raw = readFileSync(abs, "utf8");
      if (!raw.startsWith("---")) continue;
      const parsed = readFrontMatter(raw);
      if (parsed.kind === "fault") {
        quarantineFile({ abs, destDir, origin, fault: parsed.fault, at });
        quarantines.push({ path: origin, fault: parsed.fault });
        continue;
      }
      const unknown = unknownKey(parsed.map, folder);
      if (unknown !== undefined) {
        const fault = `unknown-key:${unknown}`;
        quarantineFile({
          abs,
          destDir,
          origin,
          fault,
          at,
        });
        quarantines.push({ path: origin, fault });
        continue;
      }
      const missing = missingKey(parsed.map, folder.required);
      if (missing !== undefined) {
        const fault = `missing-key:${missing}`;
        quarantineFile({
          abs,
          destDir,
          origin,
          fault,
          at,
        });
        quarantines.push({ path: origin, fault });
        continue;
      }
      notes.push({ path: origin, frontMatter: parsed.map });
    }
  }
  return { notes, quarantines };
}

function isQuarantineFolder(folder: FolderEntry): boolean {
  return folder.schema.kind === "named" && folder.schema.name === "quarantine";
}

function readFolders(folders: YamlValue[]): FolderEntry[] {
  const out: FolderEntry[] = [];
  for (const item of folders) {
    if (item === null || typeof item !== "object" || Array.isArray(item)) {
      throw new Error("folders entries must be maps");
    }
    if (typeof item.path !== "string" || item.path === "") {
      throw new Error("folder path is required");
    }
    out.push({
      path: item.path,
      schema: readSchema(item.schema),
      required: readRequired(item.required),
    });
  }
  return out;
}

function readSchema(value: unknown): FolderSchema {
  if (typeof value === "string" && value !== "") {
    return { kind: "named", name: value };
  }
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    return { kind: "inline", allowedKeys: new Set(Object.keys(value)) };
  }
  throw new Error("folder schema is required");
}

function readRequired(value: unknown): string[] {
  if (value === undefined) return [];
  if (!Array.isArray(value)) {
    throw new Error("folder required must be a list of strings");
  }
  const keys: string[] = [];
  for (const item of value) {
    if (typeof item !== "string") {
      throw new Error("folder required must be a list of strings");
    }
    keys.push(item);
  }
  return keys;
}

function missingKey(
  map: Record<string, YamlValue>,
  required: readonly string[],
): string | undefined {
  for (const key of required) {
    if (!(key in map)) return key;
  }
  return undefined;
}

function unknownKey(
  map: Record<string, YamlValue>,
  folder: FolderEntry,
): string | undefined {
  const allowed =
    folder.schema.kind === "inline"
      ? folder.schema.allowedKeys
      : namedAllowlist(folder.schema.name);
  for (const key of Object.keys(map)) {
    if (!allowed.has(key)) return key;
  }
  return undefined;
}

function readFrontMatter(
  raw: string,
):
  | { kind: "ok"; map: Record<string, YamlValue> }
  | { kind: "fault"; fault: string } {
  if (!raw.startsWith("---")) {
    return { kind: "fault", fault: "parse-error" };
  }
  const afterOpen = raw.slice(3);
  const close = afterOpen.match(/\r?\n---(?:\r?\n|$)/);
  if (close === null) {
    return { kind: "fault", fault: "parse-error" };
  }
  const yamlText = afterOpen.slice(0, close.index).replace(/^\r?\n/, "");
  try {
    return { kind: "ok", map: parseYaml(yamlText) };
  } catch {
    return { kind: "fault", fault: "parse-error" };
  }
}

function includeFolder(
  folderPath: string,
  watch: readonly string[] | undefined,
): boolean {
  if (watch === undefined || watch.length === 0) return true;
  const folder = normalizePrefix(folderPath);
  return watch.some((root) => {
    const watchRoot = normalizePrefix(root);
    return (
      folder === watchRoot ||
      folder.startsWith(`${watchRoot}/`) ||
      watchRoot.startsWith(`${folder}/`)
    );
  });
}

function listMarkdownFiles(dir: string): string[] {
  const out: string[] = [];
  const ents = sortDirents(readdirSync(dir, { withFileTypes: true }));
  for (const ent of ents) {
    const abs = join(dir, ent.name);
    if (ent.isDirectory()) {
      out.push(...listMarkdownFiles(abs));
      continue;
    }
    if (ent.isFile() && ent.name.endsWith(".md")) out.push(abs);
  }
  return out;
}

function sortDirents(ents: Dirent[]): Dirent[] {
  return [...ents].sort((a, b) => a.name.localeCompare(b.name));
}

function normalizePrefix(path: string): string {
  return path.replaceAll("\\", "/").replace(/\/+$/, "");
}

function projectRel(cwd: string, abs: string): string {
  return relative(cwd, abs).split(sep).join("/");
}
