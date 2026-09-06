import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { siteHeaderVariants } from "./SiteHeader.variants";

/**
 * Props for site chrome. No extra variants; mobile disclosure waits for a later sitting.
 */
export type SiteHeaderProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof siteHeaderVariants>;
