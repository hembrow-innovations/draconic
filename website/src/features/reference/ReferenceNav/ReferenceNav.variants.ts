import { cva } from "class-variance-authority";

/**
 * Reference working-page list in the site side nav, not a second handbook chrome.
 */
export const referenceNavListVariants = cva("flex flex-col gap-2");

/**
 * Group links use the body type role and muted ink until hover.
 */
export const referenceNavLinkVariants = cva(
  "inline-flex items-center min-h-11 font-body text-body text-muted no-underline hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);
