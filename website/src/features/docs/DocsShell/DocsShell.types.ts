import type { HTMLAttributes, ReactNode } from "react";
import type { VariantProps } from "class-variance-authority";
import type { badgeVariants } from "../../../components/Badge/Badge.variants";
import type { docsShellVariants } from "./DocsShell.variants";

/**
 * Props for handbook chrome. Status is the page frontmatter look from Badge.
 */
export type DocsShellProps = HTMLAttributes<HTMLDivElement> &
  VariantProps<typeof docsShellVariants> & {
    status: NonNullable<VariantProps<typeof badgeVariants>["variant"]>;
    nav?: ReactNode;
    body?: string;
  };
