import { useEffect, useState } from "react";
import type { CodeFenceProps } from "./CodeFence.types";
import {
  codeFenceButtonVariants,
  codeFenceLiveVariants,
  codeFencePreVariants,
  codeFenceVariants,
} from "./CodeFence.variants";

/**
 * Copyable code fence for Learn and Reference articles.
 *
 * Locks `public-site.fences:copy` and `public-site.fences:copy-announce`.
 *
 * @param props - Fence source text, distinct copy label, plus native wrapper attributes
 * @returns Copy control and sample
 */
export function CodeFence({
  className,
  code,
  label,
  ...props
}: CodeFenceProps) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) {
      return;
    }
    const timer = setTimeout(() => {
      setCopied(false);
    }, 2000);
    return () => {
      clearTimeout(timer);
    };
  }, [copied]);

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
        aria-label={copied ? `Copied ${label}` : `Copy ${label}`}
      >
        {copied ? "Copied" : "Copy"}
      </button>
      <p className={codeFenceLiveVariants()} aria-live="polite">
        {copied ? `Copied ${label}` : ""}
      </p>
      <pre className={codeFencePreVariants()}>
        <code>{code}</code>
      </pre>
    </div>
  );
}
