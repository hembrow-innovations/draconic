import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/modules")({
  loader: () => loadMarkdownPage("modules"),
  component: ModulesRoute,
});

/**
 * modules chapter. Locks `public-site.ia:learn-walkable`.
 */
function ModulesRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
