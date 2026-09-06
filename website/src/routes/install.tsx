import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/install")({
  loader: () => loadMarkdownPage("install"),
  component: InstallRoute,
});

/**
 * Install chapter. Locks `public-site.ia:learn-walkable`.
 */
function InstallRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
