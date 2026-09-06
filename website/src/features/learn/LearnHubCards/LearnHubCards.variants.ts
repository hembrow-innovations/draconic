import { cva } from "class-variance-authority";

/**
 * Chapter cards in the Learn hub article, wrapping without a fixed column count.
 */
export const learnHubCardsVariants = cva(
  "grid gap-6 [grid-template-columns:repeat(auto-fit,minmax(min(100%,12rem),1fr))]",
);

/**
 * One chapter card. Surface and ink come from theme roles.
 */
export const learnHubCardVariants = cva(
  "card border border-line p-6 bg-elevated text-ink",
);

/**
 * Linked chapter title uses the body type role.
 */
export const learnHubCardTitleVariants = cva(
  "font-body text-body text-link no-underline hover:text-ink",
);
