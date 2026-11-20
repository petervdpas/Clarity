// The IDE's dev server (vite.config.ts) as the IDE's webview meets it, tested
// without a server listening or a network, so it answers the same on every
// machine: no answer may be kept (noStore.ts), or after a restart the webview
// runs old compiled components whose styles the new server never compiled
// ("failed to load virtual css module"); and a component's styles asked
// before the component are its CSS (vite.config.ts, stylesAfterComponent).
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import type { IncomingMessage, ServerResponse } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { keepNothing, noStore } from "../../noStore.ts";

const app = fileURLToPath(new URL("../..", import.meta.url));

/** A question and its answer as the middleware meets them: the headers the answer was given. */
function exchange(headers: Record<string, string>) {
  const given: Record<string, unknown> = {};
  const req = { headers: { ...headers } } as unknown as IncomingMessage;
  const res = {
    setHeader(name: string, value: unknown) {
      given[name.toLowerCase()] = value;
      return res;
    },
  } as unknown as ServerResponse;
  return { req, res, given };
}

test("the dev server lets the webview keep nothing: no-store, no ETag, an old ETag forgotten", () => {
  const { req, res, given } = exchange({ "if-none-match": 'W/"kept-from-before"' });
  keepNothing(req, res);
  assert.equal(given["cache-control"], "no-store", "every answer starts as no-store");
  assert.equal(req.headers["if-none-match"], undefined, "a question with an old ETag gets the whole answer");
  // What Vite then sets on any answer: a module's no-cache, a prepared package's year, its ETag.
  res.setHeader("Cache-Control", "max-age=31536000,immutable");
  assert.equal(given["cache-control"], "no-store", "a prepared package may not be kept either");
  res.setHeader("ETag", 'W/"123"');
  assert.equal(given.etag, undefined, "no ETag to ask again with");
  res.setHeader("Content-Type", "text/javascript");
  assert.equal(given["content-type"], "text/javascript", "the other headers as Vite gives them");
});

test("the no-store middleware stands before every answer Vite gives", () => {
  const used: ((req: IncomingMessage, res: ServerResponse, next: () => void) => void)[] = [];
  const server = { middlewares: { use: (f: (typeof used)[number]) => used.push(f) } };
  (noStore.configureServer as (s: unknown) => void)(server);
  assert.equal(used.length, 1, "one middleware, added as the server is made, before Vite's own");
  const { req, res, given } = exchange({});
  let passed = false;
  used[0](req, res, () => (passed = true));
  assert.ok(passed && given["cache-control"] === "no-store", "it marks the answer and lets Vite answer");
});

test("a component's styles asked before the component are its CSS", async () => {
  // WebKit fetches the files a page loaded last time ahead, at the next start: the styles before their
  // component. Vite transforms them as it would answer them, with nothing listening and no packages to prepare.
  const cacheDir = mkdtempSync(join(tmpdir(), "clarity-vite-test-"));
  const server = await createServer({
    root: app,
    configFile: `${app}vite.config.ts`,
    cacheDir,
    logLevel: "silent",
    optimizeDeps: { noDiscovery: true, include: [] },
    server: { middlewareMode: true, watch: null, hmr: false },
  });
  try {
    for (const component of ["/src/App.svelte", "/src/lib/ReleasePlanning.svelte", "/src/lib/BuildLines.svelte"]) {
      const styles = await server.environments.client.transformRequest(`${component}?svelte&type=style&lang.css`);
      assert.ok(styles, component);
      assert.ok(!styles.code.includes("<script"), `${component}: no component source as styles`);
      assert.ok(styles.code.trim().length > 0, `${component}: its styles, not empty`);
    }
    const shell = await server.environments.client.transformRequest("/src/App.svelte?svelte&type=style&lang.css");
    assert.ok(shell?.code.includes(".app") && shell.code.includes("grid"), "App's styles hold its layout");
  } finally {
    await server.close();
    rmSync(cacheDir, { recursive: true, force: true });
  }
});
