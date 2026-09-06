import type { HTMLAttributes, ReactNode } from "react";
import type { VariantProps } from "class-variance-authority";
import type { badgeVariants } from "../../../components/Badge/Badge.variants";
import type { docsShellVariants } from "./DocsShell.variants";

/**
 * Props for handbook article chrome. Status is the page frontmatter look from Badge.
 * Kicker is the teaching section label above the article heading.
 * `nav` is accepted from callers; section lists render in the site side nav.
 */
export type DocsShellProps = HTMLAttributes<HTMLDivElement> &
  VariantProps<typeof docsShellVariants> & {
    kicker: string;
    status: NonNullable<VariantProps<typeof badgeVariants>["variant"]>;
    nav?: ReactNode;
    body?: string;
  };
