export type AgentName = string & { readonly __brand: "AgentName" };

export type AgentDefinition = {
  readonly name: AgentName;
  readonly body: string;
  readonly skills?: readonly string[];
  readonly tools?: readonly string[];
  readonly model?: string;
};

export type AgentDefinitionErrorCode =
  | "bad_frontmatter"
  | "bad_name"
  | "empty_body"
  | "empty_name"
  | "missing_name"
  | "unknown_keys";

const NAME_RE = /^[a-z][a-z0-9-]{0,63}$/;
const ALLOWED_KEYS = new Set(["name", "skills", "tools", "model"]);

export class AgentDefinitionError extends Error {
  readonly code: AgentDefinitionErrorCode;
  readonly keys?: readonly string[];

  constructor(args: {
    code: AgentDefinitionErrorCode;
    message: string;
    keys?: readonly string[];
  }) {
    super(args.message);
    this.name = "AgentDefinitionError";
    this.code = args.code;
    this.keys = args.keys;
  }
}

export function parseAgentDefinition(text: string): AgentDefinition {
  const { fields, body } = splitAgentMarkdown(text);
  const seen: string[] = [];
  let nameRaw: string | null = null;
  let skills: readonly string[] | undefined;
  let tools: readonly string[] | undefined;
  let model: string | undefined;

  for (const [key, raw] of fields) {
    seen.push(key);
    if (key === "name") nameRaw = raw;
    else if (key === "skills") skills = parseStringList(raw);
    else if (key === "tools") tools = parseStringList(raw);
    else if (key === "model") {
      const value = stripWrappingQuotes(raw.trim()).trim();
      model = value.length > 0 ? value : undefined;
    }
  }

  const unknown = seen.filter((key) => !ALLOWED_KEYS.has(key));
  if (unknown.length > 0) {
    throw new AgentDefinitionError({
      code: "unknown_keys",
      message: `unknown frontmatter keys: ${unknown.join(", ")}`,
      keys: unknown,
    });
  }
  if (nameRaw === null) {
    throw new AgentDefinitionError({
      code: "missing_name",
      message: "missing name",
    });
  }
  const name = parseAgentName(nameRaw);
  if (!body) {
    throw new AgentDefinitionError({
      code: "empty_body",
      message: "empty body",
    });
  }
  return { name, body, skills, tools, model };
}

function splitAgentMarkdown(text: string): {
  fields: ReadonlyArray<readonly [string, string]>;
  body: string;
} {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  if (lines[0] !== "---") {
    throw new AgentDefinitionError({
      code: "bad_frontmatter",
      message: "missing frontmatter",
    });
  }
  const fmLines: string[] = [];
  let close = -1;
  for (let i = 1; i < lines.length; i++) {
    if (lines[i] === "---") {
      close = i;
      break;
    }
    fmLines.push(lines[i] ?? "");
  }
  if (close === -1) {
    throw new AgentDefinitionError({
      code: "bad_frontmatter",
      message: "missing frontmatter close",
    });
  }
  const fields: Array<readonly [string, string]> = [];
  for (const line of fmLines) {
    if (line.trim() === "") continue;
    const m = line.match(/^([^:]+?)\s*:\s*(.*)$/);
    if (!m) {
      throw new AgentDefinitionError({
        code: "bad_frontmatter",
        message: "invalid frontmatter line",
      });
    }
    fields.push([(m[1] ?? "").trim(), m[2] ?? ""]);
  }
  return {
    fields,
    body: lines
      .slice(close + 1)
      .join("\n")
      .trim(),
  };
}

function parseAgentName(raw: string): AgentName {
  const text = stripWrappingQuotes(raw.trim()).trim();
  if (!text) {
    throw new AgentDefinitionError({
      code: "empty_name",
      message: "name is empty",
    });
  }
  if (!NAME_RE.test(text)) {
    throw new AgentDefinitionError({
      code: "bad_name",
      message: `invalid name: ${text}`,
    });
  }
  return text as AgentName;
}

function parseStringList(raw: string): readonly string[] {
  let text = raw.trim();
  if (text.startsWith("[") && text.endsWith("]")) {
    text = text.slice(1, -1).trim();
  }
  if (!text) return [];
  return text
    .split(",")
    .map((item) => stripWrappingQuotes(item.trim()).trim())
    .filter((item) => item.length > 0);
}

function stripWrappingQuotes(text: string): string {
  if (text.length >= 2) {
    const a = text[0];
    const b = text.at(-1);
    if ((a === '"' && b === '"') || (a === "'" && b === "'")) {
      return text.slice(1, -1);
    }
  }
  return text;
}

export default function () {}
