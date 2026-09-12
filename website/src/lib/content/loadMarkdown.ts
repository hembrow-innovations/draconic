/**
 * Teaching page loaded from `website/content/*.md`, not from the agent vault.
 */
export type MarkdownPage = {
  slug: string;
  title: string;
  section: string;
  status: string;
  body: string;
};

const SLUG_PATTERN = /^[a-z0-9-]+$/;

const bundledMarkdown = import.meta.glob("../../../content/*.md", {
  eager: true,
  query: "?raw",
  import: "default",
}) as Record<string, string>;

/**
 * Catalog of public teaching pages bundled from `website/content/*.md`.
 *
 * @returns Parsed pages for each markdown file in the content root
 */
export function listMarkdownPages(): MarkdownPage[] {
  return Object.keys(bundledMarkdown)
    .map((key) => slugFromGlobKey(key))
    .filter((slug) => SLUG_PATTERN.test(slug))
    .map((slug) => loadMarkdownPage(slug))
    .sort((left, right) => left.slug.localeCompare(right.slug));
}

/**
 * Load one teaching page by slug from the bundled catalog.
 *
 * @param slug - File stem of a `website/content/*.md` page
 * @returns Title, section, status, and markdown body
 */
export function loadMarkdownPage(slug: string): MarkdownPage {
  if (!SLUG_PATTERN.test(slug)) {
    throw new Error(`Invalid markdown slug: ${slug}`);
  }
  const source = sourceFromBundle(slug);
  if (source === undefined) {
    throw new Error(`Invalid markdown slug: ${slug}`);
  }
  return { slug, ...parseMarkdownPage(source) };
}

/**
 * Split YAML-like frontmatter from the markdown body.
 *
 * @param source - Raw file text including `title`, `section`, and `status`
 * @returns Frontmatter fields plus the remaining body
 */
export function parseMarkdownPage(source: string): Omit<MarkdownPage, "slug"> {
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

function sourceFromBundle(slug: string): string | undefined {
  const suffix = `/${slug}.md`;
  for (const [key, source] of Object.entries(bundledMarkdown)) {
    if (key === `${slug}.md` || key.endsWith(suffix)) {
      return source;
    }
  }
  return undefined;
}

function slugFromGlobKey(key: string): string {
  const name = key.split("/").pop() ?? "";
  return name.replace(/\.md$/, "");
}

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

function requireField(fields: Map<string, string>, key: string): string {
  const value = fields.get(key);
  if (value === undefined || value === "") {
    throw new Error(`Markdown page is missing frontmatter field: ${key}`);
  }
  return value;
}
