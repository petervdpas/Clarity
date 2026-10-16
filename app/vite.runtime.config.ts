import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Builds the frontend of generated applications into fixed file names, which
// the Rust generator copies into every application's `ui/` folder.
export default defineConfig({
  plugins: [svelte()],
  base: "./",
  build: {
    outDir: "../crates/clarity-gen/ui",
    emptyOutDir: true,
    rollupOptions: {
      input: "runtime.html",
      output: {
        entryFileNames: "runtime.js",
        chunkFileNames: "runtime-[name].js",
        assetFileNames: "runtime.[ext]",
      },
    },
  },
});
