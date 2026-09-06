import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { querySearchIndex, useSearchIndex } from "../../lib/search";
import type { SiteSearchProps } from "./SiteSearch.types";
import {
  siteSearchInputVariants,
  siteSearchLinkVariants,
  siteSearchResultsVariants,
  siteSearchVariants,
} from "./SiteSearch.variants";

/**
 * Client finder for Learn and Reference titles and headings.
 *
 * Locks `public-site.search:titles-headings`.
 *
 * @param props - Native element attributes
 * @returns Search field and result links
 */
export function SiteSearch({ className, ...props }: SiteSearchProps) {
  const index = useSearchIndex();
  const [query, setQuery] = useState("");
  const results = querySearchIndex(index, query);

  return (
    <div className={siteSearchVariants({ className })} {...props}>
      <input
        type="search"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        aria-label="Search"
        className={siteSearchInputVariants()}
      />
      {results.length > 0 ? (
        <ul className={siteSearchResultsVariants()}>
          {results.map((entry) => (
            <li key={entry.href}>
              <Link to={entry.href} className={siteSearchLinkVariants()}>
                {entry.title}
              </Link>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
