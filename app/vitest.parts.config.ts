// The parts of the Clarity UI library on the web, run against the cases
// every renderer shares (conformance/*.json), in a simulated browser page.
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { shared } from "./vite.shared";

export default defineConfig({
  plugins: [svelte()],
  resolve: { ...shared.resolve, conditions: ["browser"] },
  server: shared.server,
  test: {
    environment: "jsdom",
    globalSetup: ["./vitest.cli.setup.ts"],
    include: ["../runtimes/web/src/**/*.svelte.test.ts", "src/**/*.svelte.test.ts"],
  },
});
