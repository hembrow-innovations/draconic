import { cva } from "class-variance-authority";

/**
 * Learn chapter list in the site side nav, not a second handbook chrome.
 */
export const learnNavListVariants = cva("flex flex-col gap-2");

/**
 * Group links use the body type role and muted ink until hover or the current page.
 */
export const learnNavLinkVariants = cva(
  "inline-flex items-center min-h-11 font-body text-body text-muted no-underline hover:text-ink aria-[current=page]:text-accent-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);
