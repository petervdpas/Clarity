import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { shared } from "./vite.shared";

// Settings follow the Tauri guidance: fixed port, no screen clearing,
// and the Rust side excluded from the file watcher.
export default defineConfig({
  plugins: [svelte()],
  resolve: shared.resolve,
  clearScreen: false,
  server: {
    ...shared.server,
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
    // The IDE's webview keeps no copy of the dev server's modules: a cached
    // page would ask for files that were moved or removed since.
    headers: { "Cache-Control": "no-store" },
  },
});
