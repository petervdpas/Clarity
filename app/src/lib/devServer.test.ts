// The IDE's dev server (vite.config.ts) as the IDE's webview meets it: no
// answer may be kept, or after a restart the webview runs old compiled
// components whose styles the new server never compiled ("failed to load
// virtual css module") and the IDE shows without its layout.
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const app = fileURLToPath(new URL("../..", import.meta.url));

/** Asks `path` as the webview does, twice: fresh, and again with an old ETag and date. */
async function keepsNothing(base: string, path: string) {
  const first = await fetch(base + path);
  assert.equal(first.status, 200, path);
  assert.equal(first.headers.get("cache-control"), "no-store", `${path} may not be kept`);
  assert.equal(first.headers.get("etag"), null, `${path} has no ETag to ask again with`);
  const body = await first.text();
  const again = await fetch(base + path, { headers: { "If-None-Match": 'W/"kept-from-before"', "If-Modified-Since": new Date().toUTCString() } });
  assert.equal(again.status, 200, `${path}: a question as if it had a copy gets the whole answer`);
  assert.equal(again.headers.get("cache-control"), "no-store", `${path} asked again may not be kept`);
  await again.arrayBuffer();
  return body;
}

/** The IDE's dev server on a free port, with a cache of its own: the IDE's (node_modules/.vite) is left as it is. */
async function start() {
  const cacheDir = mkdtempSync(join(tmpdir(), "clarity-vite-test-"));
  const server = await createServer({ root: app, configFile: `${app}vite.config.ts`, cacheDir, server: { port: 0, strictPort: false }, logLevel: "silent" });
  await server.listen();
  const address = server.httpServer?.address();
  assert.ok(address && typeof address === "object");
  return { server, base: `http://localhost:${address.port}`, done: async () => { await server.close(); rmSync(cacheDir, { recursive: true, force: true }); } };
}

test("a component's styles asked before the component are its CSS", async () => {
  // WebKit fetches the files a page loaded last time ahead, at the next start: the styles before their component.
  const { base, done } = await start();
  try {
    for (const component of ["/src/App.svelte", "/src/lib/ReleasePlanning.svelte", "/src/lib/BuildLines.svelte"]) {
      const styles = await fetch(`${base}${component}?svelte&type=style&lang.css`);
      assert.equal(styles.status, 200, component);
      const text = await styles.text();
      assert.ok(!text.includes("<script"), `${component}: no component source as styles`);
      assert.ok(text.trim().length > 0, `${component}: its styles, not empty`);
    }
  } finally {
    await done();
  }
});

test("the dev server lets the webview keep nothing, of every kind it serves", async () => {
  const { base: started, done } = await start();
  try {
    const base = started;
    // The page, the manual's window, the IDE's code, a component and its styles.
    for (const path of ["/", "/?window=manual", "/src/app.css"]) {
      await keepsNothing(base, path);
    }
    // A component, then its styles: real CSS with its rules, not empty and not the component's source.
    const component = await keepsNothing(base, "/src/App.svelte");
    assert.ok(component.includes("App.svelte?svelte&type=style&lang.css"), "App.svelte asks for its styles");
    const styles = await keepsNothing(base, "/src/App.svelte?svelte&type=style&lang.css");
    assert.ok(styles.includes(".app") && styles.includes("grid"), "App's styles hold its layout");
    assert.ok(!styles.includes("<script"), "the styles are no component source");
    // A static file of public/ (the splash screen), served by another part of Vite.
    await keepsNothing(base, "/splash.html");
    // The IDE's code and Clarity.UI's web screen from outside app/ (/@fs/), and a prepared package, which Vite marks to keep for a year.
    const main = await keepsNothing(base, "/src/main.ts");
    const outside = main.match(/from "(\/@fs\/[^"]+)"/)?.[1];
    assert.ok(outside, "main.ts imports from runtimes/web through /@fs/");
    await keepsNothing(base, outside);
    const prepared = main.match(/from "([^"]*\/deps\/[^"]+)"/)?.[1];
    assert.ok(prepared, "main.ts imports a prepared package");
    await keepsNothing(base, prepared);
  } finally {
    await done();
  }
});
