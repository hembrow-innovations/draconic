import { useState } from "react";
import { Link } from "@tanstack/react-router";
import {
  querySearchIndex,
  searchHitHref,
  searchHitLabel,
  useSearchIndex,
} from "../../lib/search";
import type { SiteSearchProps } from "./SiteSearch.types";
import {
  siteSearchEmptyVariants,
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
  const miss = query.trim() !== "" && results.length === 0;

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
          {results.map((entry) => {
            const href = searchHitHref(entry, query);
            const hashAt = href.indexOf("#");
            const to = hashAt === -1 ? href : href.slice(0, hashAt);
            const hash = hashAt === -1 ? undefined : href.slice(hashAt + 1);
            return (
              <li key={entry.href}>
                <Link to={to} hash={hash} className={siteSearchLinkVariants()}>
                  {searchHitLabel(entry, query)}
                </Link>
              </li>
            );
          })}
        </ul>
      ) : miss ? (
        <p className={siteSearchEmptyVariants()}>No matching pages</p>
      ) : null}
    </div>
  );
}
