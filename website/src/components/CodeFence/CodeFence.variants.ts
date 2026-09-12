import { cva } from "class-variance-authority";

/**
 * Fence stack: copy control above the sample, no overlay.
 */
export const codeFenceVariants = cva("flex flex-col gap-2 items-stretch");

/**
 * Copy control for a fence. Same focus ring and touch target as other chrome.
 */
export const codeFenceButtonVariants = cva(
  "inline-flex items-center self-end min-h-11 font-body text-body text-muted hover:text-ink transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);

/**
 * Sample body. Token roles match the article pre/code surface.
 */
export const codeFencePreVariants = cva(
  "overflow-x-auto bg-code p-4 font-mono text-mono",
);
