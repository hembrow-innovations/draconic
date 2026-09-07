import { cva } from "class-variance-authority";

/**
 * Learn chapter list in the site side nav, not a second handbook chrome.
 */
export const learnNavListVariants = cva("flex flex-col gap-2");

/**
 * Group links use the body type role and muted ink until hover.
 */
export const learnNavLinkVariants = cva(
  "font-body text-body text-muted no-underline hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);
