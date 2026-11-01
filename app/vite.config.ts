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
  },
});
