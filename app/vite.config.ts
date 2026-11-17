import { defineConfig, type Plugin, type ViteDevServer } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { shared } from "./vite.shared";

// The IDE's webview keeps no copy of the dev server's answers. Vite answers
// modules with "no-cache" and an ETag, which the webview stored on disk and,
// after a restart, ran old compiled components from: their styles, asked of
// a Vite that never compiled them, came back empty ("failed to load virtual
// css module") and the IDE showed without its layout. So every answer says
// "no-store", has no ETag, and a question with an old ETag gets a full answer.
const noStore: Plugin = {
  name: "clarity:no-store",
  configureServer(server) {
    server.middlewares.use((req, res, next) => {
      delete req.headers["if-none-match"];
      const setHeader = res.setHeader.bind(res);
      res.setHeader = (name: string, value: number | string | readonly string[]) => {
        const lower = name.toLowerCase();
        if (lower === "etag") return res;
        return setHeader(name, lower === "cache-control" ? "no-store" : value);
      };
      res.setHeader("Cache-Control", "no-store");
      next();
    });
  },
};

// A component's styles asked before the component compiles it first. WebKit
// fetches what a page loaded last time ahead at the next start, the styles
// before their component; Svelte's plugin has a component's styles only once
// it compiled the component, and answered them with its source instead
// ("failed to load virtual css module"): the IDE showed without its layout.
let devServer: ViteDevServer | undefined;
const stylesAfterComponent: Plugin = {
  name: "clarity:styles-after-component",
  enforce: "pre",
  apply: "serve",
  configureServer(server) {
    devServer = server;
  },
  async load(id) {
    const at = id.indexOf("?svelte&type=style");
    if (at < 0 || !devServer) return;
    const file = id.slice(0, at);
    const root = devServer.config.root;
    // A file outside the root goes by /@fs/: on Linux and macOS its path starts
    // with "/", on Windows with its drive ("E:/"), which needs the slash added.
    const url = file.startsWith(`${root}/`) ? file.slice(root.length) : `/@fs/${file.replace(/^\//, "")}`;
    await devServer.environments.client.transformRequest(url);
  },
};

// Settings follow the Tauri guidance: fixed port, no screen clearing,
// and the Rust side excluded from the file watcher.
export default defineConfig({
  plugins: [noStore, stylesAfterComponent, svelte()],
  // Every package the IDE's page uses, prepared before the page is served,
  // so Vite does not prepare them again while the page loads.
  optimizeDeps: {
    include: [
      "svelte",
      "@tauri-apps/api/core",
      "@tauri-apps/api/event",
      "@tauri-apps/api/window",
      "codemirror",
      "@codemirror/commands",
      "@codemirror/lang-cpp",
      "@codemirror/lang-go",
      "@codemirror/lang-rust",
      "@codemirror/language",
      "@codemirror/legacy-modes/mode/clike",
      "@codemirror/legacy-modes/mode/cmake",
      "@codemirror/legacy-modes/mode/toml",
      "@codemirror/lint",
      "@codemirror/search",
      "@codemirror/state",
      "@codemirror/view",
      "@lezer/highlight",
      "mermaid",
    ],
  },
  resolve: shared.resolve,
  clearScreen: false,
  server: {
    ...shared.server,
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
