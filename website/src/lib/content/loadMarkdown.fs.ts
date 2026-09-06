import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseMarkdownPage, type MarkdownPage } from "./loadMarkdown";

const SLUG_PATTERN = /^[a-z0-9-]+$/;

/**
 * Catalog of teaching pages from an explicit directory. Node tests only.
 *
 * @param rootDir - Directory of `*.md` sources
 * @returns Parsed pages for each markdown file in that directory
 */
export function listMarkdownPagesFromDir(rootDir: string): MarkdownPage[] {
  const root = resolve(rootDir);
  return readdirSync(root)
    .filter((name) => name.endsWith(".md"))
    .map((name) => loadMarkdownPageFromDir(name.slice(0, -3), root))
    .sort((left, right) => left.slug.localeCompare(right.slug));
}

/**
 * Load one teaching page by slug from an explicit directory. Node tests only.
 *
 * @param slug - File stem of a markdown page
 * @param rootDir - Directory of `*.md` sources
 * @returns Title, section, status, and markdown body
 */
export function loadMarkdownPageFromDir(
  slug: string,
  rootDir: string,
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
  return { slug, ...parseMarkdownPage(source) };
}
