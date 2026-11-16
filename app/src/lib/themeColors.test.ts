import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

// Every color the IDE's pages use comes from its themes (app.css): a name no
// theme defines falls back to whatever the page wrote beside it (white on a
// dark theme), and a theme that leaves a color out shows the light one.
const src = join(import.meta.dirname, "..");
const css = readFileSync(join(src, "app.css"), "utf8");

function block(selector: string): Set<string> {
  const start = css.indexOf(selector);
  assert.ok(start >= 0, selector);
  const body = css.slice(css.indexOf("{", start) + 1, css.indexOf("}", start));
  return new Set([...body.matchAll(/(--[a-zA-Z0-9-]+)\s*:/g)].map((m) => m[1]));
}

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return files(path);
    return /\.(svelte|css)$/.test(name) ? [path] : [];
  });
}

const light = block(":root,\n[data-scheme=\"light\"]");
const themes = { dark: block("[data-scheme=\"dark\"] {"), "cream-green": block(":root[data-ide-theme=\"cream-green\"]") };
// Not colors: the same in every theme.
const sameInAll = new Set(["--radius", "--mono", "--brand"]);

test("every color a page uses is one the themes define", () => {
  const missing: string[] = [];
  for (const file of files(src)) {
    const text = readFileSync(file, "utf8");
    // A name the page sets itself (style:--edge={...}, style="--edge: ...") is its own.
    const own = new Set([...text.matchAll(/(?:style:|[;"\s])(--[a-zA-Z0-9-]+)\s*[=:]/g)].map((m) => m[1]));
    for (const [, name] of text.matchAll(/var\((--[a-zA-Z0-9-]+)/g)) {
      if (!light.has(name) && !own.has(name)) missing.push(`${file.slice(src.length + 1)}: ${name}`);
    }
  }
  assert.deepEqual([...new Set(missing)], []);
});

test("every theme gives each color its own value", () => {
  for (const [theme, names] of Object.entries(themes)) {
    const left = [...light].filter((n) => !sameInAll.has(n) && !names.has(n));
    assert.deepEqual(left, [], `${theme} leaves these to the light theme`);
  }
});
