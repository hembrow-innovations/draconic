import { cva } from "class-variance-authority";

/**
 * Recovery column in main. Classes stay here so the JSX does not own a second type scale.
 */
export const notFoundVariants = cva(
  "px-8 py-8 bg-canvas text-ink flex flex-col gap-4",
);

/**
 * Heading that names the miss, using the display type role from the theme scale.
 */
export const notFoundTitleVariants = cva("font-display text-display text-ink");

/**
 * In-site way back to a real page, not a new marketing bar.
 */
export const notFoundLinkVariants = cva(
  "inline-flex items-center min-h-11 font-body text-body text-link no-underline hover:text-ink",
);
