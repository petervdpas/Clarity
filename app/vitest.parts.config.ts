// The parts of the Clarity UI library on the web, run against the cases
// every renderer shares (conformance/*.json), in a simulated browser page.
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";
import { shared } from "./vite.shared";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    ...shared.resolve,
    conditions: ["browser"],
    // A generated window imports the package by name from outside app/ (DESIGN 7.8).
    alias: [{ find: /^@clarity\/ui$/, replacement: resolve(import.meta.dirname, "../runtimes/web/src/index.ts") }],
  },
  // The window files a test writes with `clarity spec windows` are in target/.
  server: { fs: { allow: [...(shared.server?.fs?.allow ?? []), resolve(import.meta.dirname, "../target")] } },
  test: {
    environment: "jsdom",
    include: ["../runtimes/web/src/**/*.svelte.test.ts", "src/**/*.svelte.test.ts"],
  },
});
