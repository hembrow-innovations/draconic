import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { siteFooterVariants } from "./SiteFooter.variants";

/**
 * Props for quiet site chrome. Chapter lists stay out of this footer.
 */
export type SiteFooterProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof siteFooterVariants>;
