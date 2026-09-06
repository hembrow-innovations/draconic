import { defineConfig } from "vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import viteReact from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const pagesBase = (process.env.PAGES_BASE ?? "").replace(/\/$/, "");

const pages = [
  "/",
  "/learn",
  "/install",
  "/from-javascript",
  "/from-systems",
  "/dual-worlds",
  "/modules",
  "/native-types",
  "/host-io",
  "/packages",
  "/reference",
  "/cli",
  "/types",
  "/dual-world-rules",
  "/reference-host-io",
  "/reference-packages",
].map((path) => ({ path }));

export default defineConfig({
  base: pagesBase ? `${pagesBase}/` : "/",
  server: {
    port: 3000,
  },
  plugins: [
    tailwindcss(),
    tanstackStart({
      router: pagesBase ? { basepath: pagesBase } : undefined,
      pages,
      prerender: {
        enabled: true,
        crawlLinks: false,
        autoSubfolderIndex: true,
      },
    }),
    viteReact(),
  ],
});
