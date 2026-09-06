import { cva } from "class-variance-authority";

/**
 * Working-page cards in the Reference hub article, wrapping without a fixed column count.
 */
export const referenceHubCardsVariants = cva(
  "grid gap-6 [grid-template-columns:repeat(auto-fit,minmax(min(100%,12rem),1fr))]",
);

/**
 * One working-page card. Surface and ink come from theme roles.
 */
export const referenceHubCardVariants = cva(
  "card border border-line p-6 bg-elevated text-ink",
);

/**
 * Linked working-page title uses the body type role.
 */
export const referenceHubCardTitleVariants = cva(
  "font-body text-body text-link no-underline hover:text-ink",
);
