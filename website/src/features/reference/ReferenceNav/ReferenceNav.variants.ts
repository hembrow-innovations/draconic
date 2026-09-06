import { cva } from "class-variance-authority";

/**
 * Reference working-page list inside the docs aside, not a second handbook chrome.
 */
export const referenceNavListVariants = cva("flex flex-col gap-2");

/**
 * Aside links use the body type role and muted ink until hover.
 */
export const referenceNavLinkVariants = cva(
  "font-body text-body text-muted no-underline hover:text-ink",
);
