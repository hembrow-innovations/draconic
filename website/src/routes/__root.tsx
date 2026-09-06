import type { ReactNode } from "react";
import {
  Outlet,
  createRootRoute,
  HeadContent,
  Scripts,
} from "@tanstack/react-router";
import { SiteFooter } from "../components/SiteFooter";
import { SiteHeader } from "../components/SiteHeader";
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
  }),
  component: RootComponent,
});

function RootComponent() {
  const { searchIndex } = Route.useLoaderData();
  return (
    <RootDocument>
      <SearchIndexProvider index={searchIndex}>
        <SkipLink />
        <SiteHeader />
        <main id="main" tabIndex={-1}>
          <Outlet />
        </main>
        <SiteFooter />
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
