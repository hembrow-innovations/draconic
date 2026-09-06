import { cva } from "class-variance-authority";

/**
 * Learn chapter list inside the docs aside, not a second handbook chrome.
 */
export const learnNavListVariants = cva("flex flex-col gap-2");

/**
 * Aside links use the body type role and muted ink until hover.
 */
export const learnNavLinkVariants = cva(
  "font-body text-body text-muted no-underline hover:text-ink",
);
