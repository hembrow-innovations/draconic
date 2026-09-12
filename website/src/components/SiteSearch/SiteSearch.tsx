import { useEffect, useId, useState } from "react";
import { Link, useLocation, useNavigate } from "@tanstack/react-router";
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
  siteSearchLiveVariants,
  siteSearchResultsVariants,
  siteSearchVariants,
} from "./SiteSearch.variants";

/**
 * Client finder for Learn and Reference titles and headings.
 *
 * Locks `public-site.search:titles-headings`, `public-site.search:session`,
 * and `public-site.search:keyboard-live`.
 *
 * @param props - Native element attributes
 * @returns Search field and result links
 */
export function SiteSearch({ className, ...props }: SiteSearchProps) {
  const index = useSearchIndex();
  const location = useLocation();
  const navigate = useNavigate();
  const searchId = useId();
  const listId = `${searchId}-results`;
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const results = querySearchIndex(index, query);
  const miss = query.trim() !== "" && results.length === 0;
  const expanded = results.length > 0;
  const clampedIndex =
    results.length === 0 ? 0 : Math.min(activeIndex, results.length - 1);
  const activeHit = results[clampedIndex];
  const activeOptionId =
    results.length > 0 ? `${searchId}-hit-${clampedIndex}` : undefined;

  useEffect(() => {
    setQuery("");
  }, [location.pathname, location.hash]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  return (
    <div className={siteSearchVariants({ className })} {...props}>
      <input
        type="search"
        role="combobox"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            setQuery("");
            return;
          }
          if (event.key === "ArrowDown") {
            if (results.length === 0) {
              return;
            }
            event.preventDefault();
            setActiveIndex((current) =>
              current + 1 < results.length ? current + 1 : current,
            );
            return;
          }
          if (event.key === "ArrowUp") {
            if (results.length === 0) {
              return;
            }
            event.preventDefault();
            setActiveIndex((current) => (current > 0 ? current - 1 : 0));
            return;
          }
          if (event.key === "Enter") {
            const hit = activeHit ?? results[0];
            if (!hit) {
              return;
            }
            event.preventDefault();
            const href = searchHitHref(hit, query);
            const hashAt = href.indexOf("#");
            const to = hashAt === -1 ? href : href.slice(0, hashAt);
            const hash = hashAt === -1 ? undefined : href.slice(hashAt + 1);
            void navigate({ to, hash });
          }
        }}
        aria-label="Search"
        aria-autocomplete="list"
        aria-expanded={expanded}
        aria-controls={results.length > 0 ? listId : undefined}
        aria-activedescendant={activeOptionId}
        className={siteSearchInputVariants()}
      />
      <p className={siteSearchLiveVariants()} aria-live="polite">
        {miss
          ? "No matching pages"
          : results.length > 0
            ? `${results.length} matching pages`
            : ""}
      </p>
      {results.length > 0 ? (
        <ul
          id={listId}
          role="listbox"
          className={siteSearchResultsVariants()}
        >
          {results.map((entry, index) => {
            const href = searchHitHref(entry, query);
            const hashAt = href.indexOf("#");
            const to = hashAt === -1 ? href : href.slice(0, hashAt);
            const hash = hashAt === -1 ? undefined : href.slice(hashAt + 1);
            const selected = index === clampedIndex;
            return (
              <li
                key={entry.href}
                id={`${searchId}-hit-${index}`}
                role="option"
                aria-selected={selected}
              >
                <Link
                  to={to}
                  hash={hash}
                  tabIndex={-1}
                  className={siteSearchLinkVariants({ selected })}
                >
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
