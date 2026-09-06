import { cva } from "class-variance-authority";

/**
 * Article-footer pager so a chapter can be walked without the aside.
 */
export const learnPagerVariants = cva(
  "flex justify-between gap-4 border-t border-line pt-4 mt-4",
);

/**
 * One direction of the Learn path, previous or next.
 */
export const learnPagerGroupVariants = cva("flex flex-col gap-2");

/**
 * Quiet Previous or Next word above the chapter links.
 */
export const learnPagerLabelVariants = cva("font-body text-muted");

/**
 * Chapter links share the aside body role and muted ink until hover.
 */
export const learnPagerLinkVariants = cva(
  "inline-flex items-center min-h-11 font-body text-body text-link no-underline hover:text-ink",
);
