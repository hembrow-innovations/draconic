import { cva } from "class-variance-authority";

/**
 * Footer chrome. Classes stay here so the JSX does not own a second type scale.
 */
export const siteFooterVariants = cva(
  "px-8 py-6 bg-canvas text-ink border-t border-line font-body text-muted flex items-center justify-between gap-4",
);

/**
 * Durable GitHub exit shares the body type role with header links.
 */
export const siteFooterLinkVariants = cva(
  "inline-flex items-center min-h-11 font-body text-body text-muted no-underline hover:text-ink",
);
