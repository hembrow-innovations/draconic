import type { ReactNode } from "react";
import {
  Outlet,
  createRootRoute,
  HeadContent,
  Scripts,
} from "@tanstack/react-router";
import { NotFound } from "../components/NotFound";
import { SiteFooter } from "../components/SiteFooter";
import {
  SiteHeader,
  siteMainVariants,
  siteShellVariants,
} from "../components/SiteHeader";
import { SkipLink } from "../components/SkipLink";
import {
  buildSearchIndex,
  SearchIndexProvider,
} from "../lib/search";
import "../styles/theme.css";

export const Route = createRootRoute({
  loader: () => ({ searchIndex: buildSearchIndex() }),
  head: () => ({
    meta: [
      {
        charSet: "utf-8",
      },
      {
        name: "viewport",
        content: "width=device-width, initial-scale=1",
      },
      {
        title: "Draconic",
      },
    ],
    links: [
      {
        rel: "icon",
        href: "/favicon.ico",
      },
    ],
  }),
  component: RootComponent,
  notFoundComponent: NotFound,
});

/**
 * Skip link, sticky side nav, and main column on every route.
 *
 * Locks `public-site.chrome:odm-shell`, `public-site.chrome:primary-nav`,
 * `public-site.chrome:favicon`, and `public-site.chrome:not-found`.
 */
function RootComponent() {
  const { searchIndex } = Route.useLoaderData();
  return (
    <RootDocument>
      <SearchIndexProvider index={searchIndex}>
        <SkipLink />
        <div className={siteShellVariants()}>
          <SiteHeader />
          <main id="main" className={siteMainVariants()} tabIndex={-1}>
            <Outlet />
            <SiteFooter />
          </main>
        </div>
      </SearchIndexProvider>
    </RootDocument>
  );
}

function RootDocument({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <HeadContent />
        <script
          dangerouslySetInnerHTML={{
            __html:
              'try{var r=JSON.parse(localStorage.getItem("theme:v1"));if(r&&r.theme==="dark")document.documentElement.classList.add("dark")}catch(e){}',
          }}
        />
      </head>
      <body className="bg-canvas text-ink min-h-dvh">
        {children}
        <Scripts />
      </body>
    </html>
  );
}
