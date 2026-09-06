import { cva } from "class-variance-authority";

/**
 * Reference working-page list in the site side nav, not a second handbook chrome.
 */
export const referenceNavListVariants = cva("flex flex-col gap-2");

/**
 * Group links use the body type role and muted ink until hover.
 */
export const referenceNavLinkVariants = cva(
  "font-body text-body text-muted no-underline hover:text-ink",
);
