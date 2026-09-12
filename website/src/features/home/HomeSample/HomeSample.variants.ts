import { cva } from "class-variance-authority";

/**
 * Home sample band. Same width and canvas as the pitch so `/` stays one landing.
 */
export const homeSampleVariants = cva(
  "px-8 py-12 bg-canvas text-ink flex flex-col gap-6 max-w-3/4",
);

/**
 * Section label that names the sample as a Program, not a Learn chapter.
 */
export const homeSampleKickerVariants = cva(
  "kicker font-body text-mono uppercase tracking-widest text-muted",
);

/**
 * Caption under the kicker. Body type, not a second pitch.
 */
export const homeSampleLeadVariants = cva("font-body text-body text-ink");

/**
 * Static hello source. Token surface matches article fences, without a copy control.
 */
export const homeSamplePreVariants = cva(
  "overflow-x-auto bg-code p-4 font-mono text-mono",
);

/**
 * Install link reuses the path-link role so the sample points at the existing CTA.
 */
export const homeSampleLinkVariants = cva(
  "font-body text-body text-link no-underline hover:text-ink",
);
