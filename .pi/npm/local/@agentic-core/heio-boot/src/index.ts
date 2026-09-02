import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import type {
  ExtensionAPI,
  ExtensionContext,
} from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { parseAgentDefinition, type AgentDefinition } from "./definition.ts";

function agentFile(cwd: string, name: string): string {
  return join(cwd, ".pi", "agents", `${name}.md`);
}

function loadAgent(cwd: string, name: string): AgentDefinition | undefined {
  const path = agentFile(cwd, name);
  if (!existsSync(path)) return undefined;
  try {
    return parseAgentDefinition(readFileSync(path, "utf8"));
  } catch {
    return undefined;
  }
}

function flagString(pi: ExtensionAPI, name: string): string | undefined {
  const value = pi.getFlag(name);
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function paint(ctx: Pick<ExtensionContext, "ui">, name: string): void {
  try {
    ctx.ui.setStatus("agent", name);
  } catch {
    // print mode
  }
}

function notify(ctx: Pick<ExtensionContext, "ui">, message: string): void {
  try {
    ctx.ui.notify(message, "info");
  } catch {
    // print mode
  }
}

const DEST_LOADER_NAME = "dest_activate_tools";
const LENS_LOADER_NAME = "pi_lens_activate_tools";
const FIRST_PARTY_NAMES = new Set([DEST_LOADER_NAME]);
const FIRST_PARTY_PREFIXES = ["coms_", "team_", "task_"] as const;
const PARKED_NAMES = new Set([
  "web_search",
  "fetch_content",
  "source_check",
  "get_search_content",
  "subagent",
  "subagent_wait",
  "lens_diagnostics",
  "lsp_diagnostics",
  "module_report",
  "read_symbol",
  "read_enclosing",
  "symbol_search",
  "project_report",
  "lens_health",
  "lens_project_scan",
  "ast_grep_search",
  "ast_grep_replace",
  "ast_grep_outline",
  "ast_grep_dump",
  "lsp_navigation",
  "lens_diagnostic_mark",
]);
const PARKED_PATH_MARKERS = [
  "pi-web-access",
  "pi-subagents",
  "pi-lens",
] as const;

type ToolRecord = {
  readonly name: string;
  readonly sourceInfo: {
    readonly source: string;
    readonly path?: string;
  };
};

function isFirstPartyName(name: string): boolean {
  if (FIRST_PARTY_NAMES.has(name)) return true;
  return FIRST_PARTY_PREFIXES.some((prefix) => name.startsWith(prefix));
}

function isParkedTool(tool: ToolRecord): boolean {
  if (tool.sourceInfo.source === "builtin") return false;
  if (isFirstPartyName(tool.name)) return false;
  if (tool.name === LENS_LOADER_NAME) return false;
  if (PARKED_NAMES.has(tool.name)) return true;
  const path = tool.sourceInfo.path ?? "";
  return PARKED_PATH_MARKERS.some((marker) => path.includes(marker));
}

function requestedToolNames(params: { tools?: unknown }): string[] {
  if (!Array.isArray(params.tools)) return [];
  return params.tools.filter(
    (name): name is string => typeof name === "string",
  );
}

function parkThirdPartyTools(
  pi: Pick<ExtensionAPI, "getActiveTools" | "getAllTools" | "setActiveTools">,
): Set<string> {
  const all = pi.getAllTools();
  const parked = new Set(
    all.filter((tool) => isParkedTool(tool)).map((tool) => tool.name),
  );
  const next = pi.getActiveTools().filter((name) => !parked.has(name));
  if (
    all.some((tool) => tool.name === DEST_LOADER_NAME) &&
    !next.includes(DEST_LOADER_NAME)
  ) {
    next.push(DEST_LOADER_NAME);
  }
  pi.setActiveTools(next);
  return parked;
}

function liveSetWithoutParked(args: {
  names: readonly string[];
  parked: ReadonlySet<string>;
  keep: readonly string[];
}): string[] {
  return [
    ...new Set([
      ...args.names.filter((name) => !args.parked.has(name)),
      ...args.keep,
    ]),
  ];
}

function destLoaderKeep(
  all: ReadonlyArray<{ name: string }>,
): readonly string[] {
  return all.some((tool) => tool.name === DEST_LOADER_NAME)
    ? [DEST_LOADER_NAME]
    : [];
}

function bindActiveTools(
  pi: Pick<ExtensionAPI, "getActiveTools" | "getAllTools" | "setActiveTools">,
  definition: AgentDefinition | undefined,
  snapshot: string[] | null,
  parked: ReadonlySet<string>,
): string[] | null {
  const all = pi.getAllTools();
  const keep = destLoaderKeep(all);
  if (!definition || definition.tools === undefined) {
    if (snapshot) {
      pi.setActiveTools(
        liveSetWithoutParked({ names: snapshot, parked, keep }),
      );
    }
    return snapshot;
  }
  const nextSnapshot =
    snapshot ??
    liveSetWithoutParked({
      names: pi.getActiveTools(),
      parked,
      keep,
    });
  const active = new Set(pi.getActiveTools());
  const builtin = new Set(
    all
      .filter((tool) => tool.sourceInfo.source === "builtin")
      .map((tool) => tool.name),
  );
  const valid = definition.tools.filter((name) => builtin.has(name));
  if (valid.length === 0) return nextSnapshot;
  const extensions = all
    .filter(
      (tool) =>
        tool.sourceInfo.source !== "builtin" &&
        active.has(tool.name) &&
        !parked.has(tool.name),
    )
    .map((tool) => tool.name);
  const firstParty = all
    .filter((tool) => isFirstPartyName(tool.name))
    .map((tool) => tool.name);
  pi.setActiveTools(
    liveSetWithoutParked({
      names: [...valid, ...extensions, ...firstParty],
      parked,
      keep,
    }),
  );
  return nextSnapshot;
}

export default function (pi: ExtensionAPI) {
  pi.registerFlag("agent", {
    description: "Dest .pi/agents stem for this process",
    type: "string",
    default: undefined,
  });

  let selected: string | null = null;
  let toolsSnapshot: string[] | null = null;
  let parkedNames = new Set<string>();

  function currentStem(): string | null {
    return selected;
  }

  function loadCurrent(cwd: string): AgentDefinition | undefined {
    const stem = currentStem();
    return stem ? loadAgent(cwd, stem) : undefined;
  }

  function applyDefinition(
    ctx: ExtensionContext,
    def: AgentDefinition | undefined,
  ): void {
    toolsSnapshot = bindActiveTools(pi, def, toolsSnapshot, parkedNames);
    paint(ctx, def?.name ?? "off");
  }

  function selectNone(ctx: ExtensionContext): void {
    selected = null;
    applyDefinition(ctx, undefined);
    notify(ctx, "agent off");
  }

  pi.on("session_start", (_event, ctx) => {
    parkedNames = parkThirdPartyTools(pi);
    const flagged = flagString(pi, "agent");
    if (flagged && loadAgent(ctx.cwd, flagged)) selected = flagged;
    applyDefinition(ctx, loadCurrent(ctx.cwd));
  });

  pi.on("before_agent_start", (event, ctx) => {
    const def = loadCurrent(ctx.cwd);
    applyDefinition(ctx, def);
    if (!def) return;
    return {
      systemPrompt: `${event.systemPrompt}\n\n${def.body}`,
    };
  });

  pi.registerCommand("agent", {
    description:
      "Select a dest .pi/agents file for this process. off clears it",
    async handler(args, ctx) {
      const raw = args.trim();
      if (raw === "" || raw === "default" || raw === "off") {
        selectNone(ctx);
        return;
      }
      const def = loadAgent(ctx.cwd, raw);
      if (!def) {
        notify(ctx, `unknown agent: ${raw}`);
        return;
      }
      selected = raw;
      applyDefinition(ctx, def);
      notify(ctx, `agent ${def.name}`);
    },
  });

  pi.registerTool({
    name: DEST_LOADER_NAME,
    label: "Activate dest tools",
    description:
      "Activate one or more registered tools that stay inactive until requested. Call this once with the tool names you need. They become callable on the next model request after this result. Continue; do not wait for the user.",
    promptSnippet:
      "Activate parked third-party tools by name before using them",
    parameters: Type.Object({
      tools: Type.Array(Type.String(), {
        minItems: 1,
        description: "Names of registered tools to activate.",
      }),
    }),
    async execute(_toolCallId, params) {
      const registered = new Set(pi.getAllTools().map((tool) => tool.name));
      const valid = requestedToolNames(params).filter((name) =>
        registered.has(name),
      );
      for (const name of valid) parkedNames.delete(name);
      const active = pi.getActiveTools();
      pi.setActiveTools([...new Set([...active, ...valid])]);
      return {
        content: [
          {
            type: "text" as const,
            text:
              valid.length > 0
                ? `Activated: ${valid.join(", ")}. Callable on the next model request. Continue.`
                : "No valid tool names given.",
          },
        ],
        details: { matches: valid },
      };
    },
  });
}
