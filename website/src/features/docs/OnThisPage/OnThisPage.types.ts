import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { onThisPageVariants } from "./OnThisPage.variants";

/**
 * Props for the in-article outline of section headings.
 * `body` is the teaching markdown used to build heading links.
 */
export type OnThisPageProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof onThisPageVariants> & {
    body: string;
  };
