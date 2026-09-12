/**
 * One stop on the public Learn path from `website/content/learn.md`.
 */
export type LearnStop = {
  href: LearnHref;
  label: string;
};

/**
 * Prev and next for one chapter. Landings are siblings, not a forced order.
 */
export type LearnNeighbors = {
  prev: readonly LearnStop[];
  next: readonly LearnStop[];
};

/**
 * Chapter hrefs on the Learn path. Hub `/learn` is not a chapter.
 */
export type LearnHref =
  | "/install"
  | "/from-javascript"
  | "/from-systems"
  | "/dual-worlds"
  | "/modules"
  | "/native-types"
  | "/host-io"
  | "/packages";

const INSTALL: LearnStop = { href: "/install", label: "Install" };
const FROM_JAVASCRIPT: LearnStop = {
  href: "/from-javascript",
  label: "from JavaScript",
};
const FROM_SYSTEMS: LearnStop = {
  href: "/from-systems",
  label: "from systems",
};
const DUAL_WORLDS: LearnStop = { href: "/dual-worlds", label: "Dual worlds" };
const MODULES: LearnStop = { href: "/modules", label: "modules" };
const NATIVE_TYPES: LearnStop = {
  href: "/native-types",
  label: "native types",
};
const HOST_IO: LearnStop = { href: "/host-io", label: "host I/O" };
const PACKAGES: LearnStop = { href: "/packages", label: "packages" };

const EMPTY: LearnNeighbors = { prev: [], next: [] };

/**
 * Neighbors for a Learn chapter slug.
 *
 * Install leads into both landings. from-javascript and from-systems both
 * continue to Dual worlds. After the join the path is one sequence.
 *
 * @param slug - File stem of a Learn chapter
 * @returns Prev and next stops, empty when the slug is not a chapter
 */
export function learnNeighbors(slug: string): LearnNeighbors {
  switch (slug) {
    case "install":
      return { prev: [], next: [FROM_JAVASCRIPT, FROM_SYSTEMS] };
    case "from-javascript":
    case "from-systems":
      return { prev: [INSTALL], next: [DUAL_WORLDS] };
    case "dual-worlds":
      return { prev: [FROM_JAVASCRIPT, FROM_SYSTEMS], next: [MODULES] };
    case "modules":
      return { prev: [DUAL_WORLDS], next: [NATIVE_TYPES] };
    case "native-types":
      return { prev: [MODULES], next: [HOST_IO] };
    case "host-io":
      return { prev: [NATIVE_TYPES], next: [PACKAGES] };
    case "packages":
      return { prev: [HOST_IO], next: [] };
    default:
      return EMPTY;
  }
}
