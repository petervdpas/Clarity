import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Settings follow the Tauri guidance: fixed port, no screen clearing,
// and the Rust side excluded from the file watcher.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
