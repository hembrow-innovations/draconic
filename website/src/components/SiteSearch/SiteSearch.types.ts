import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { SearchEntry } from "../../lib/search";
import type { siteSearchVariants } from "./SiteSearch.variants";

/**
 * Props for the in-chrome title, heading, and teaching-body finder.
 */
export type SiteSearchProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof siteSearchVariants>;

export type { SearchEntry };
