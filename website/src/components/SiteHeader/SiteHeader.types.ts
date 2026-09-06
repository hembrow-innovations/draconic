import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { siteHeaderVariants } from "./SiteHeader.variants";

/**
 * Props for sticky side nav chrome. Small-viewport disclosure is internal state.
 */
export type SiteHeaderProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof siteHeaderVariants>;
