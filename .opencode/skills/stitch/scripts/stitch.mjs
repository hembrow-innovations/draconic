#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const SKILL_ROOT = join(SCRIPT_DIR, "..");
const USAGE = `Usage: stitch.mjs <command> [flags]

Commands:
  doctor
  projects
  create        --title <name>
  ensure        --title <name>
  screens       --project <id>
  generate      --project <id> --prompt <text>|--prompt-file <path>
                [--device MOBILE|DESKTOP|TABLET|AGNOSTIC]
                [--model GEMINI_3_FLASH|GEMINI_3_PRO|GEMINI_3_1_PRO]
                [--pull] [--slug <slug>] [--out <dir>]
  edit          --project <id> --screen <id> --prompt <text>|--prompt-file <path>
                [--device ...] [--model ...] [--pull] [--slug <slug>] [--out <dir>]
  variants      --project <id> --screen <id> --prompt <text>|--prompt-file <path>
                [--count 1-5] [--range REFINE|EXPLORE|REIMAGINE]
                [--aspects LAYOUT,COLOR_SCHEME,IMAGES,TEXT_FONT,TEXT_CONTENT]
                [--device ...] [--model ...] [--pull] [--out <dir>]
  get           --project <id> --screen <id>
  pull          --project <id> --screen <id> [--slug <slug>] [--out <dir>]
  pull          --project <id> --all [--out <dir>]
  upload        --project <id> --file <path> [--title <name>] [--pull] [--out <dir>]
  upload-md     --project <id> --file <path>
  tools
  call          --tool <name> --args <json>

Env: STITCH_API_KEY (required). Optional: STITCH_HOST, STITCH_ACCESS_TOKEN, GOOGLE_CLOUD_PROJECT.
`;

const DEVICES = new Set(["MOBILE", "DESKTOP", "TABLET", "AGNOSTIC"]);
const MODELS = new Set(["GEMINI_3_FLASH", "GEMINI_3_PRO", "GEMINI_3_1_PRO"]);
const RANGES = new Set(["REFINE", "EXPLORE", "REIMAGINE"]);
const ASPECTS = new Set([
  "LAYOUT",
  "COLOR_SCHEME",
  "IMAGES",
  "TEXT_FONT",
  "TEXT_CONTENT",
]);
const DEFAULT_OUT = join(".stitch", "designs");

function emit(value, code = 0) {
  process.stdout.write(`${JSON.stringify(value)}\n`);
  process.exit(code);
}

function fail(code, message, extra = {}) {
  emit({ ok: false, error: { code, message, ...extra } }, 1);
}

function parseArgs(argv) {
  const flags = {};
  const positionals = [];
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--") {
      positionals.push(...argv.slice(i + 1));
      break;
    }
    if (arg === "-h" || arg === "--help") {
      flags.help = true;
      continue;
    }
    if (arg.startsWith("--")) {
      const eq = arg.indexOf("=");
      if (eq !== -1) {
        flags[arg.slice(2, eq)] = arg.slice(eq + 1);
        continue;
      }
      const key = arg.slice(2);
      const next = argv[i + 1];
      if (next == null || next.startsWith("-")) {
        flags[key] = true;
        continue;
      }
      flags[key] = next;
      i += 1;
      continue;
    }
    if (arg.startsWith("-") && arg !== "-") {
      fail("USAGE", `unknown flag: ${arg}`);
    }
    positionals.push(arg);
  }
  return { flags, positionals };
}

function loadDotEnv(file) {
  if (!existsSync(file)) return;
  const text = readFileSync(file, "utf8");
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq === -1) continue;
    const key = trimmed.slice(0, eq).trim();
    let value = trimmed.slice(eq + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (process.env[key] == null || process.env[key] === "") {
      process.env[key] = value;
    }
  }
}

function flagString(flags, name) {
  const value = flags[name];
  if (value == null || value === true) return undefined;
  return String(value);
}

function requireFlag(flags, name) {
  const value = flagString(flags, name);
  if (!value) fail("USAGE", `missing --${name}`);
  return value;
}

function optionalEnum(flags, name, allowed, fallback) {
  const raw = flagString(flags, name);
  if (raw == null) return fallback;
  const value = raw.toUpperCase();
  if (!allowed.has(value)) {
    fail("USAGE", `invalid --${name}: ${raw}`);
  }
  return value;
}

function readPrompt(flags) {
  const file = flagString(flags, "prompt-file");
  if (file) return readFileSync(resolve(file), "utf8");
  const prompt = flags.prompt;
  if (prompt === "-" || prompt === true) return readFileSync(0, "utf8");
  if (typeof prompt === "string") return prompt;
  if (!process.stdin.isTTY) return readFileSync(0, "utf8");
  fail("USAGE", "missing --prompt or --prompt-file");
}

function slugify(text) {
  const slug = String(text)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 60);
  return slug || "screen";
}

function metadataPath(cwd) {
  return join(cwd, ".stitch", "metadata.json");
}

function readMetadata(cwd) {
  const path = metadataPath(cwd);
  if (!existsSync(path)) {
    return { projectId: null, projectTitle: null, screens: [] };
  }
  try {
    const parsed = JSON.parse(readFileSync(path, "utf8"));
    return {
      projectId: parsed.projectId ?? null,
      projectTitle: parsed.projectTitle ?? null,
      screens: Array.isArray(parsed.screens) ? parsed.screens : [],
    };
  } catch {
    return { projectId: null, projectTitle: null, screens: [] };
  }
}

function writeMetadata(cwd, data) {
  const dir = join(cwd, ".stitch");
  mkdirSync(dir, { recursive: true });
  writeFileSync(metadataPath(cwd), `${JSON.stringify(data, null, 2)}\n`);
}

function recordProject(cwd, project, title) {
  const current = readMetadata(cwd);
  current.projectId = project.id ?? project.projectId;
  current.projectTitle = title ?? projectTitle(project) ?? current.projectTitle;
  writeMetadata(cwd, current);
  return current;
}

function recordScreen(cwd, entry) {
  const current = readMetadata(cwd);
  current.projectId = entry.projectId ?? current.projectId;
  const screens = current.screens.filter((item) => item.id !== entry.id);
  screens.push(entry);
  current.screens = screens;
  writeMetadata(cwd, current);
  return current;
}

function projectTitle(project) {
  const data = project.data ?? {};
  return data.title ?? data.displayName ?? data.name ?? null;
}

function screenTitle(screen) {
  const data = screen.data ?? {};
  return data.title ?? data.displayName ?? data.name ?? null;
}

function summarizeProject(project) {
  return {
    id: project.id ?? project.projectId,
    title: projectTitle(project),
  };
}

function summarizeScreen(screen) {
  const data = screen.data ?? {};
  return {
    id: screen.id ?? screen.screenId,
    projectId: screen.projectId,
    title: screenTitle(screen),
    deviceType: data.deviceType ?? null,
  };
}

async function withUrls(screen) {
  return {
    ...summarizeScreen(screen),
    htmlUrl: await screen.getHtml(),
    imageUrl: await screen.getImage(),
  };
}

function resolveSdk() {
  const entry = join(
    SKILL_ROOT,
    "node_modules",
    "@google",
    "stitch-sdk",
    "dist",
    "src",
    "index.js",
  );
  return existsSync(entry) ? entry : null;
}

function hasApiKey() {
  return Boolean(
    process.env.STITCH_API_KEY ||
      (process.env.STITCH_ACCESS_TOKEN && process.env.GOOGLE_CLOUD_PROJECT),
  );
}

async function loadSdk() {
  const resolved = resolveSdk();
  if (!resolved) {
    fail("SETUP", `SDK missing. Run: npm install --prefix ${SKILL_ROOT}`);
  }
  return import(pathToFileURL(resolved).href);
}

function stitchError(error) {
  const code = error?.code ?? "UNKNOWN_ERROR";
  const message = error?.message ?? String(error);
  const recoverable = Boolean(error?.recoverable);
  fail(code, message, { recoverable });
}

async function withStitch(fn) {
  const sdk = await loadSdk();
  const { stitch } = sdk;
  try {
    return await fn(sdk);
  } catch (error) {
    stitchError(error);
  } finally {
    try {
      await stitch.close();
    } catch {
      // ignore
    }
  }
}

function outDir(flags) {
  return flagString(flags, "out") ?? DEFAULT_OUT;
}

function extForImage(url, contentType) {
  if (contentType?.includes("jpeg")) return "jpg";
  if (contentType?.includes("webp")) return "webp";
  if (contentType?.includes("png")) return "png";
  const path = url.split("?")[0].toLowerCase();
  if (path.endsWith(".jpg") || path.endsWith(".jpeg")) return "jpg";
  if (path.endsWith(".webp")) return "webp";
  return "png";
}

async function fetchToFile(url, dest, asText) {
  const response = await fetch(url);
  if (!response.ok) {
    fail("NETWORK_ERROR", `download failed ${response.status} for ${dest}`, {
      recoverable: true,
    });
  }
  mkdirSync(dirname(dest), { recursive: true });
  if (asText) {
    writeFileSync(dest, await response.text());
    return { path: dest, contentType: response.headers.get("content-type") };
  }
  writeFileSync(dest, Buffer.from(await response.arrayBuffer()));
  return { path: dest, contentType: response.headers.get("content-type") };
}

async function pullScreen(screen, { cwd, slug, destRoot }) {
  const id = screen.id ?? screen.screenId;
  const usedSlug = slugify(slug ?? screenTitle(screen) ?? id);
  const dir = join(cwd, destRoot, usedSlug);
  const htmlUrl = await screen.getHtml();
  const imageUrl = await screen.getImage();
  const files = {};
  if (htmlUrl) {
    const html = await fetchToFile(htmlUrl, join(dir, "code.html"), true);
    files.html = html.path;
  }
  if (imageUrl) {
    const tmp = await fetch(imageUrl);
    if (!tmp.ok) {
      fail("NETWORK_ERROR", `screenshot download failed ${tmp.status}`, {
        recoverable: true,
      });
    }
    const ext = extForImage(imageUrl, tmp.headers.get("content-type"));
    const imagePath = join(dir, `screen.${ext}`);
    mkdirSync(dir, { recursive: true });
    writeFileSync(imagePath, Buffer.from(await tmp.arrayBuffer()));
    files.image = imagePath;
  }
  const entry = {
    id,
    projectId: screen.projectId,
    slug: usedSlug,
    title: screenTitle(screen),
    ...files,
  };
  recordScreen(cwd, entry);
  return { ...summarizeScreen(screen), slug: usedSlug, files };
}

function parseAspects(flags) {
  const raw = flagString(flags, "aspects");
  if (!raw) return undefined;
  const aspects = raw.split(",").map((part) => part.trim().toUpperCase());
  for (const aspect of aspects) {
    if (!ASPECTS.has(aspect))
      fail("USAGE", `invalid --aspects value: ${aspect}`);
  }
  return aspects;
}

function parseCount(flags) {
  const raw = flagString(flags, "count");
  if (raw == null) return 3;
  const count = Number(raw);
  if (!Number.isInteger(count) || count < 1 || count > 5) {
    fail("USAGE", "invalid --count (1-5)");
  }
  return count;
}

function parseArgsJson(flags) {
  const raw = flagString(flags, "args") ?? "{}";
  try {
    const parsed = JSON.parse(raw);
    if (parsed == null || typeof parsed !== "object" || Array.isArray(parsed)) {
      fail("USAGE", "--args must be a JSON object");
    }
    return parsed;
  } catch {
    fail("USAGE", "--args must be JSON");
  }
}

async function maybePull(flags, cwd, screen, slug) {
  if (flags.pull !== true) return undefined;
  return pullScreen(screen, { cwd, slug, destRoot: outDir(flags) });
}

async function main() {
  loadDotEnv(join(process.cwd(), ".env"));
  const { flags, positionals } = parseArgs(process.argv.slice(2));
  if (flags.help || positionals[0] === "help") {
    process.stdout.write(USAGE);
    process.exit(0);
  }
  const command = positionals[0];
  if (!command) fail("USAGE", USAGE.trim());
  if (positionals.length > 1)
    fail("USAGE", `unexpected argument: ${positionals[1]}`);
  const cwd = process.cwd();

  if (command === "doctor") {
    const sdk = resolveSdk();
    const apiKey = hasApiKey();
    const result = {
      ok: Boolean(sdk) && apiKey,
      sdk: Boolean(sdk),
      apiKey,
      skillRoot: SKILL_ROOT,
    };
    if (!sdk) {
      result.hint = `npm install --prefix ${SKILL_ROOT}`;
    } else if (!apiKey) {
      result.hint =
        "Set STITCH_API_KEY from https://stitch.withgoogle.com/settings";
    }
    emit(result, result.ok ? 0 : 1);
  }

  if (!hasApiKey()) {
    fail(
      "AUTH_FAILED",
      "Set STITCH_API_KEY from https://stitch.withgoogle.com/settings",
    );
  }

  await withStitch(async ({ stitch }) => {
    if (command === "projects") {
      const projects = await stitch.projects();
      emit({ ok: true, projects: projects.map(summarizeProject) });
    }

    if (command === "create") {
      const title = requireFlag(flags, "title");
      const project = await stitch.createProject(title);
      recordProject(cwd, project, title);
      emit({ ok: true, created: true, ...summarizeProject(project) });
    }

    if (command === "ensure") {
      const title = requireFlag(flags, "title");
      const meta = readMetadata(cwd);
      if (meta.projectId) {
        emit({
          ok: true,
          created: false,
          reused: "metadata",
          id: meta.projectId,
          title: meta.projectTitle ?? title,
        });
      }
      const projects = await stitch.projects();
      const match = projects.find((project) => projectTitle(project) === title);
      if (match) {
        recordProject(cwd, match, title);
        emit({
          ok: true,
          created: false,
          reused: "title",
          ...summarizeProject(match),
        });
      }
      const project = await stitch.createProject(title);
      recordProject(cwd, project, title);
      emit({ ok: true, created: true, ...summarizeProject(project) });
    }

    if (command === "screens") {
      const projectId = requireFlag(flags, "project");
      const screens = await stitch.project(projectId).screens();
      emit({
        ok: true,
        projectId,
        screens: screens.map(summarizeScreen),
      });
    }

    if (command === "generate") {
      const projectId = requireFlag(flags, "project");
      const prompt = readPrompt(flags);
      const device = optionalEnum(flags, "device", DEVICES, "DESKTOP");
      const model = optionalEnum(flags, "model", MODELS);
      const screen = await stitch
        .project(projectId)
        .generate(prompt, device, model);
      const pulled = await maybePull(
        flags,
        cwd,
        screen,
        flagString(flags, "slug"),
      );
      emit({
        ok: true,
        ...(await withUrls(screen)),
        deviceType: device,
        pulled,
      });
    }

    if (command === "edit") {
      const projectId = requireFlag(flags, "project");
      const screenId = requireFlag(flags, "screen");
      const prompt = readPrompt(flags);
      const device = optionalEnum(flags, "device", DEVICES);
      const model = optionalEnum(flags, "model", MODELS);
      const edited = await stitch
        .project(projectId)
        .screen(screenId)
        .edit(prompt, device, model);
      const pulled = await maybePull(
        flags,
        cwd,
        edited,
        flagString(flags, "slug"),
      );
      emit({ ok: true, ...(await withUrls(edited)), pulled });
    }

    if (command === "variants") {
      const projectId = requireFlag(flags, "project");
      const screenId = requireFlag(flags, "screen");
      const prompt = readPrompt(flags);
      const device = optionalEnum(flags, "device", DEVICES);
      const model = optionalEnum(flags, "model", MODELS);
      const variantOptions = {
        variantCount: parseCount(flags),
        creativeRange: optionalEnum(flags, "range", RANGES, "EXPLORE"),
      };
      const aspects = parseAspects(flags);
      if (aspects) variantOptions.aspects = aspects;
      const variants = await stitch
        .project(projectId)
        .screen(screenId)
        .variants(prompt, variantOptions, device, model);
      const pulled = [];
      if (flags.pull === true) {
        for (const [index, variant] of variants.entries()) {
          pulled.push(
            await pullScreen(variant, {
              cwd,
              slug: `${flagString(flags, "slug") ?? slugify(prompt)}-v${index + 1}`,
              destRoot: outDir(flags),
            }),
          );
        }
      }
      emit({
        ok: true,
        projectId,
        sourceScreenId: screenId,
        variants: await Promise.all(variants.map(withUrls)),
        pulled: flags.pull === true ? pulled : undefined,
      });
    }

    if (command === "get") {
      const projectId = requireFlag(flags, "project");
      const screenId = requireFlag(flags, "screen");
      const screen = await stitch.project(projectId).getScreen(screenId);
      const htmlUrl = await screen.getHtml();
      const imageUrl = await screen.getImage();
      emit({
        ok: true,
        ...summarizeScreen(screen),
        htmlUrl,
        imageUrl,
      });
    }

    if (command === "pull") {
      const projectId = requireFlag(flags, "project");
      const destRoot = outDir(flags);
      if (flags.all === true) {
        const result = await stitch
          .project(projectId)
          .downloadAssets(resolve(cwd, destRoot));
        emit({ ok: true, projectId, dest: destRoot, ...result });
      }
      const screenId = requireFlag(flags, "screen");
      const screen = await stitch.project(projectId).getScreen(screenId);
      const pulled = await pullScreen(screen, {
        cwd,
        slug: flagString(flags, "slug"),
        destRoot,
      });
      emit({ ok: true, ...pulled });
    }

    if (command === "upload") {
      const projectId = requireFlag(flags, "project");
      const file = resolve(requireFlag(flags, "file"));
      if (!existsSync(file)) fail("NOT_FOUND", `missing file: ${file}`);
      const project = stitch.project(projectId);
      const title = flagString(flags, "title");
      let screens;
      if (typeof project.upload === "function") {
        screens = await project.upload(file, title ? { title } : undefined);
      } else if (typeof project.uploadImage === "function") {
        screens = await project.uploadImage(
          file,
          title ? { title } : undefined,
        );
      } else {
        fail("UNKNOWN_ERROR", "SDK has no upload method");
      }
      const pulled = [];
      if (flags.pull === true) {
        for (const screen of screens) {
          pulled.push(
            await pullScreen(screen, {
              cwd,
              slug: flagString(flags, "slug") ?? slugify(title ?? file),
              destRoot: outDir(flags),
            }),
          );
        }
      }
      emit({
        ok: true,
        projectId,
        screens: screens.map(summarizeScreen),
        pulled: flags.pull === true ? pulled : undefined,
      });
    }

    if (command === "upload-md") {
      const projectId = requireFlag(flags, "project");
      const file = resolve(requireFlag(flags, "file"));
      if (!existsSync(file)) fail("NOT_FOUND", `missing file: ${file}`);
      const designMdBase64 = readFileSync(file).toString("base64");
      const result = await stitch
        .project(projectId)
        .uploadDesignMd(designMdBase64);
      emit({ ok: true, projectId, result });
    }

    if (command === "tools") {
      const { tools } = await stitch.listTools();
      emit({
        ok: true,
        tools: tools.map((tool) => ({
          name: tool.name,
          description: tool.description,
        })),
      });
    }

    if (command === "call") {
      const tool = requireFlag(flags, "tool");
      const args = parseArgsJson(flags);
      const result = await stitch.callTool(tool, args);
      emit({ ok: true, tool, result });
    }

    fail("USAGE", `unknown command: ${command}`);
  });
}

main();
