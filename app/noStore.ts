// The IDE's webview keeps no copy of the dev server's answers. Vite answers
// modules with "no-cache" and an ETag, which the webview stored on disk and,
// after a restart, ran old compiled components from: their styles, asked of
// a Vite that never compiled them, came back empty ("failed to load virtual
// css module") and the IDE showed without its layout. So every answer says
// "no-store", has no ETag, and a question with an old ETag gets a full answer.
// Its own module, so its test calls it without a server (src/lib/devServer.test.ts).

import type { IncomingMessage, ServerResponse } from "node:http";
import type { Plugin } from "vite";

/** What the middleware does to one question and its answer, before Vite answers it. */
export function keepNothing(req: IncomingMessage, res: ServerResponse): void {
  delete req.headers["if-none-match"];
  const setHeader = res.setHeader.bind(res);
  res.setHeader = (name: string, value: number | string | readonly string[]) => {
    const lower = name.toLowerCase();
    if (lower === "etag") return res;
    return setHeader(name, lower === "cache-control" ? "no-store" : value);
  };
  res.setHeader("Cache-Control", "no-store");
}

/** The plugin: the middleware first, before every answer Vite gives. */
export const noStore: Plugin = {
  name: "clarity:no-store",
  configureServer(server) {
    server.middlewares.use((req, res, next) => {
      keepNothing(req, res);
      next();
    });
  },
};
