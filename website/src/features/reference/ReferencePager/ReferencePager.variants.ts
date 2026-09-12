import { cva } from "class-variance-authority";

/**
 * Article-footer pager so a working page can be walked without the aside.
 */
export const referencePagerVariants = cva(
  "flex justify-between gap-4 border-t border-line pt-4 mt-4",
);

/**
 * One direction of the Reference path, previous or next.
 */
export const referencePagerGroupVariants = cva("flex flex-col gap-2");

/**
 * Quiet Previous or Next word above the working-page links.
 */
export const referencePagerLabelVariants = cva("font-body text-muted");

/**
 * Working-page links share the aside body role and muted ink until hover.
 */
export const referencePagerLinkVariants = cva(
  "inline-flex items-center min-h-11 font-body text-body text-link no-underline hover:text-ink",
);
