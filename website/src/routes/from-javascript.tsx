import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/from-javascript")({
  loader: () => loadMarkdownPage("from-javascript"),
  component: FromJavascriptRoute,
});

/**
 * from JavaScript landing. Locks `public-site.ia:learn-walkable`.
 */
function FromJavascriptRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
