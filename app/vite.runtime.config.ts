import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { readFileSync } from "node:fs";
import { relative, resolve } from "node:path";

/**
 * Records the source files the bundle is built from and a hash of them
 * (runtime.sources.json), so crates/clarity-gen/tests/web_bundle.rs fails
 * when the screen changed and the bundle was not built again. The hash is
 * FNV-1a, 64 bits, over each file's path, a newline, its bytes, a newline.
 */
function sourcesRecorded(): Plugin {
  const here = resolve(".");
  return {
    name: "clarity-runtime-sources",
    generateBundle() {
      const files = [
        ...new Set(
          [...this.getModuleIds()]
            .filter((id) => !id.startsWith("\0"))
            .map((id) => id.split("?")[0])
            .filter((id) => id.startsWith(here) && !id.includes("node_modules"))
            .map((id) => relative(here, id).split("\\").join("/")),
        ),
      ].sort();
      let hash = 0xcbf29ce484222325n;
      const add = (bytes: Uint8Array) => {
        for (const b of bytes) hash = ((hash ^ BigInt(b)) * 0x100000001b3n) & 0xffffffffffffffffn;
      };
      const newline = new Uint8Array([10]);
      for (const file of files) {
        add(new TextEncoder().encode(file));
        add(newline);
        add(readFileSync(resolve(here, file)));
        add(newline);
      }
      this.emitFile({
        type: "asset",
        fileName: "runtime.sources.json",
        source: JSON.stringify({ files, fnv1a64: hash.toString(16).padStart(16, "0") }, null, 1) + "\n",
      });
    },
  };
}

// Builds the frontend of generated applications into fixed file names, which
// the Rust generator copies into every application's `ui/` folder.
export default defineConfig({
  plugins: [svelte(), sourcesRecorded()],
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
