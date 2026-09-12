/**
 * One stop on the public Reference path from `website/content/reference.md`.
 */
export type ReferenceStop = {
  href: ReferenceHref;
  label: string;
};

/**
 * Prev and next for one working page. The sequence is one line, not a fork.
 */
export type ReferenceNeighbors = {
  prev: readonly ReferenceStop[];
  next: readonly ReferenceStop[];
};

/**
 * Working-page hrefs on the Reference path. Hub `/reference` is not a stop.
 */
export type ReferenceHref =
  | "/cli"
  | "/types"
  | "/dual-world-rules"
  | "/reference-host-io"
  | "/reference-packages";

const CLI: ReferenceStop = { href: "/cli", label: "CLI" };
const TYPES: ReferenceStop = { href: "/types", label: "types" };
const DUAL_WORLD_RULES: ReferenceStop = {
  href: "/dual-world-rules",
  label: "Dual-world rules",
};
const HOST_IO: ReferenceStop = {
  href: "/reference-host-io",
  label: "host I/O",
};
const PACKAGES: ReferenceStop = {
  href: "/reference-packages",
  label: "packages",
};

const EMPTY: ReferenceNeighbors = { prev: [], next: [] };

/**
 * Neighbors for a Reference working-page slug.
 *
 * Order matches `public-site.ia:reference-walkable`: CLI, types,
 * Dual-world rules, host I/O, packages. First has no previous. Last has no next.
 *
 * @param slug - File stem of a Reference working page
 * @returns Prev and next stops, empty when the slug is not a working page
 */
export function referenceNeighbors(slug: string): ReferenceNeighbors {
  switch (slug) {
    case "cli":
      return { prev: [], next: [TYPES] };
    case "types":
      return { prev: [CLI], next: [DUAL_WORLD_RULES] };
    case "dual-world-rules":
      return { prev: [TYPES], next: [HOST_IO] };
    case "reference-host-io":
      return { prev: [DUAL_WORLD_RULES], next: [PACKAGES] };
    case "reference-packages":
      return { prev: [HOST_IO], next: [] };
    default:
      return EMPTY;
  }
}
