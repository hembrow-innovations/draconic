import { createFileRoute } from "@tanstack/react-router";
import { HomeFeatures } from "../features/home/HomeFeatures";
import { HomeHero } from "../features/home/HomeHero";
import { HomeSample } from "../features/home/HomeSample";
import { pageShareHead } from "../lib/content";

const homePitch =
  "JavaScript you already know. Native types when you need them. One language, two backends.";

export const Route = createFileRoute("/")({
  head: () =>
    pageShareHead({
      title: "Draconic",
      path: "/",
      description: homePitch,
    }),
  component: HomeRoute,
});

/**
 * Language homepage at `/`, not Learn copied to index.
 * Locks `public-site.chrome:meta-description`.
 */
function HomeRoute() {
  return (
    <>
      <HomeHero />
      <HomeSample />
      <HomeFeatures />
    </>
  );
}
