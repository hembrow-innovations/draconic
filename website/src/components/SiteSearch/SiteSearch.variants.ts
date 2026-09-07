import { cva } from "class-variance-authority";

/**
 * Finder cluster in site chrome.
 */
export const siteSearchVariants = cva("font-body text-body");

/**
 * Query field. Token roles only.
 */
export const siteSearchInputVariants = cva(
  "min-h-11 w-full border border-line bg-canvas px-3 text-ink font-body text-body focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);

/**
 * Result list under the field.
 */
export const siteSearchResultsVariants = cva(
  "mt-1 w-full border border-line bg-canvas",
);

/**
 * Result links use the same body role as primary nav.
 */
export const siteSearchLinkVariants = cva(
  "flex min-h-11 items-center px-3 font-body text-body text-muted no-underline hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);
