import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Teaching page loaded from `website/*.md`, not from the agent vault.
 */
export type MarkdownPage = {
  slug: string;
  title: string;
  section: string;
  status: string;
  body: string;
};

const SLUG_PATTERN = /^[a-z0-9-]+$/;
const DEFAULT_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../..",
);

/**
 * Catalog of public teaching pages, derived from markdown files in the content root.
 *
 * @param rootDir - Directory of `*.md` sources; defaults to `website/`
 * @returns Parsed pages for each markdown file in that directory
 */
export function listMarkdownPages(rootDir: string = DEFAULT_ROOT): MarkdownPage[] {
  const root = resolve(rootDir);
  return readdirSync(root)
    .filter((name) => name.endsWith(".md"))
    .map((name) => loadMarkdownPage(name.slice(0, -3), root))
    .sort((left, right) => left.slug.localeCompare(right.slug));
}

/**
 * Load one teaching page by slug from the content root.
 *
 * @param slug - File stem of a `website/*.md` page
 * @param rootDir - Directory of `*.md` sources; defaults to `website/`
 * @returns Title, section, status, and markdown body
 */
export function loadMarkdownPage(
  slug: string,
  rootDir: string = DEFAULT_ROOT,
): MarkdownPage {
  if (!SLUG_PATTERN.test(slug)) {
    throw new Error(`Invalid markdown slug: ${slug}`);
  }
  const root = resolve(rootDir);
  const filePath = join(root, `${slug}.md`);
  if (!filePath.startsWith(root)) {
    throw new Error(`Invalid markdown slug: ${slug}`);
  }
  const source = readFileSync(filePath, "utf8");
  const parsed = parseMarkdownPage(source);
  return { slug, ...parsed };
}

/**
 * Split YAML-like frontmatter from the markdown body.
 *
 * @param source - Raw file text including `title`, `section`, and `status`
 * @returns Frontmatter fields plus the remaining body
 */
function parseMarkdownPage(source: string): Omit<MarkdownPage, "slug"> {
  if (!source.startsWith("---\n") && !source.startsWith("---\r\n")) {
    throw new Error("Markdown page is missing frontmatter");
  }
  const rest = source.slice(source.indexOf("\n") + 1);
  const close = rest.search(/\r?\n---\r?\n/);
  if (close === -1) {
    throw new Error("Markdown page is missing frontmatter");
  }
  const frontmatter = rest.slice(0, close);
  const body = rest.slice(close).replace(/^\r?\n---\r?\n/, "");
  const fields = parseFrontmatter(frontmatter);
  return {
    title: requireField(fields, "title"),
    section: requireField(fields, "section"),
    status: requireField(fields, "status"),
    body,
  };
}

/**
 * Read `key: value` pairs from a frontmatter block.
 *
 * @param block - Text between the opening and closing `---` fences
 * @returns Map of frontmatter keys to string values
 */
function parseFrontmatter(block: string): Map<string, string> {
  const fields = new Map<string, string>();
  for (const line of block.split(/\r?\n/)) {
    if (line.trim() === "") {
      continue;
    }
    const sep = line.indexOf(":");
    if (sep === -1) {
      throw new Error(`Invalid frontmatter line: ${line}`);
    }
    fields.set(line.slice(0, sep).trim(), line.slice(sep + 1).trim());
  }
  return fields;
}

/**
 * Require a named frontmatter field.
 *
 * @param fields - Parsed frontmatter map
 * @param key - Field name that must be present
 * @returns The field value
 */
function requireField(fields: Map<string, string>, key: string): string {
  const value = fields.get(key);
  if (value === undefined || value === "") {
    throw new Error(`Markdown page is missing frontmatter field: ${key}`);
  }
  return value;
}
