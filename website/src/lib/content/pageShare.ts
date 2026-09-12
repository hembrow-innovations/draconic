import { unwrapMarkdownLinks } from "./headingId";

const SITE_ORIGIN = "https://hembrow-innovations.github.io/draconic";

/**
 * One document meta tag for title, description, or Open Graph.
 */
export type PageShareMetaTag = {
  title?: string;
  name?: string;
  property?: string;
  content?: string;
};

/**
 * One document link tag, used for the page canonical href.
 */
export type PageShareLink = {
  rel: string;
  href: string;
};

/**
 * Route `head` payload with description, canonical, and Open Graph summary tags.
 */
export type PageShareHead = {
  meta: PageShareMetaTag[];
  links: PageShareLink[];
};

/**
 * Existing page copy and path used to build share and search tags.
 */
export type PageShareInput = {
  title: string;
  path: string;
  description: string;
};

/**
 * First prose paragraph of a teaching page, for meta description without a new slogan.
 *
 * @param body - Markdown body after frontmatter
 * @returns Visible first-paragraph text, or empty when the body has no prose
 */
export function pageDescriptionFromBody(body: string): string {
  for (const line of body.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (
      trimmed === "" ||
      trimmed.startsWith("#") ||
      trimmed.startsWith("```") ||
      trimmed.startsWith("- ") ||
      trimmed.startsWith("* ")
    ) {
      continue;
    }
    return unwrapMarkdownLinks(trimmed);
  }
  return "";
}

/**
 * Page-specific description, canonical URL, and Open Graph tags from existing copy.
 *
 * Locks `public-site.chrome:meta-description`. Does not add Twitter cards, og:image, or theme-color.
 *
 * @param input - Document title, app path, and existing page description
 * @returns TanStack Start `head` meta and links
 */
export function pageShareHead(input: PageShareInput): PageShareHead {
  const url = canonicalUrl(input.path);
  return {
    meta: [
      { title: input.title },
      { name: "description", content: input.description },
      { property: "og:title", content: input.title },
      { property: "og:description", content: input.description },
      { property: "og:url", content: url },
    ],
    links: [{ rel: "canonical", href: url }],
  };
}

/**
 * Absolute canonical href for a public app path on the GitHub Pages origin.
 *
 * @param path - App path beginning with `/`
 * @returns Origin plus path, with a trailing slash only on home
 */
function canonicalUrl(path: string): string {
  if (path === "/") {
    return `${SITE_ORIGIN}/`;
  }
  return `${SITE_ORIGIN}${path}`;
}
