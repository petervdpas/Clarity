import { defineConfig } from "vite";

// Builds the engines beside the screen of C# with Avalonia in a browser
// (DESIGN 7.6) into one ES module, which the C# generator copies into the
// browser program's wwwroot as engine.js.
export default defineConfig({
  build: {
    outDir: "../crates/clarity-gen/ui-engine",
    emptyOutDir: true,
    lib: {
      entry: "src/lib/browserEngines.ts",
      formats: ["es"],
      fileName: () => "engine.js",
    },
  },
});
