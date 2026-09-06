import { createContext, useContext, type ReactNode } from "react";
import type { SearchEntry } from "./searchIndex";

const SearchIndexContext = createContext<SearchEntry[]>([]);

/**
 * Provide the static title and heading index to site chrome.
 *
 * @param props - Index from the root loader plus children
 * @returns Context provider
 */
export function SearchIndexProvider({
  index,
  children,
}: {
  index: SearchEntry[];
  children: ReactNode;
}) {
  return (
    <SearchIndexContext.Provider value={index}>
      {children}
    </SearchIndexContext.Provider>
  );
}

/**
 * Read the static title and heading index.
 *
 * @returns Finder entries for the public site
 */
export function useSearchIndex(): SearchEntry[] {
  return useContext(SearchIndexContext);
}
