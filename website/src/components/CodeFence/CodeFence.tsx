import { useState } from "react";
import type { CodeFenceProps } from "./CodeFence.types";
import {
  codeFenceButtonVariants,
  codeFencePreVariants,
  codeFenceVariants,
} from "./CodeFence.variants";

/**
 * Copyable code fence for Learn and Reference articles.
 *
 * Locks `public-site.fences:copy`.
 *
 * @param props - Fence source text plus native wrapper attributes
 * @returns Copy control and sample
 */
export function CodeFence({
  className,
  code,
  ...props
}: CodeFenceProps) {
  const [copied, setCopied] = useState(false);

  async function onCopy() {
    await navigator.clipboard.writeText(code);
    setCopied(true);
  }

  return (
    <div className={codeFenceVariants({ className })} {...props}>
      <button
        type="button"
        className={codeFenceButtonVariants()}
        onClick={() => {
          void onCopy();
        }}
        aria-label={copied ? "Copied" : "Copy"}
      >
        {copied ? "Copied" : "Copy"}
      </button>
      <pre className={codeFencePreVariants()}>
        <code>{code}</code>
      </pre>
    </div>
  );
}
