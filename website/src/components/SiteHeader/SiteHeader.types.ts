import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { siteHeaderVariants } from "./SiteHeader.variants";

/**
 * Props for sticky side nav chrome. Small-viewport wrap is CSS, not disclosure state.
 */
export type SiteHeaderProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof siteHeaderVariants>;
