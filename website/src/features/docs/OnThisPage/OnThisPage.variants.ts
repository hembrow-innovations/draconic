import { cva } from "class-variance-authority";

/**
 * In-article outline wrap. Stays in the article column, not a third shell column.
 */
export const onThisPageVariants = cva("flex flex-col gap-2 py-2");

/**
 * Outline label, letterspaced like other kickers.
 */
export const onThisPageKickerVariants = cva(
  "kicker font-body text-mono uppercase tracking-widest text-muted",
);

/**
 * Heading list in document order.
 */
export const onThisPageListVariants = cva("flex flex-col gap-1");

/**
 * Nested heading indent inside the outline.
 */
export const onThisPageItemVariants = cva("", {
  variants: {
    level: {
      section: "",
      sub: "pl-4",
    },
  },
  defaultVariants: { level: "section" },
});

/**
 * Fragment link to a section heading. The heading in view uses ink; siblings stay muted.
 */
export const onThisPageLinkVariants = cva(
  "inline-flex items-center py-1 font-body text-body text-muted no-underline hover:text-ink aria-[current=true]:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);
