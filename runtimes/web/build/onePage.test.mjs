// The one page of the C and C++ webview carries everything it shows (DESIGN 7.8).
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { onePageHtml } from "./onePage.mjs";

function publicFolder() {
  const dir = mkdtempSync(join(tmpdir(), "clarity-one-page-"));
  mkdirSync(join(dir, "themes"));
  mkdirSync(join(dir, "resources"));
  writeFileSync(join(dir, "themes", "shop.css"), ".theme-shop { --accent: red; }");
  writeFileSync(join(dir, "resources", "logo.svg"), '<svg xmlns="http://www.w3.org/2000/svg"/>');
  writeFileSync(join(dir, "resources", "roll.wav"), "RIFF");
  writeFileSync(join(dir, "resources", "notes.md"), "# Notes");
  return dir;
}

const BUILT = '<html><head><script type="module" crossorigin src="./runtime.js"></script>\n<link rel="stylesheet" crossorigin href="./runtime.css">\n<link rel="stylesheet" href="./themes/shop.css">\n</head><body></body></html>';

test("the page carries its script, styles and themes, their closing tags made safe", () => {
  const dir = publicFolder();
  const page = onePageHtml(BUILT, 'show("</script>")', ".a{}", dir);
  rmSync(dir, { recursive: true });
  assert.ok(page.includes('<script type="module">show("<\\/script>")</script>'), page);
  assert.ok(page.includes("<style>.a{}</style>") && page.includes("<style>.theme-shop { --accent: red; }</style>"), page);
  assert.ok(!page.includes("runtime.js") && !page.includes("./themes/"), page);
});

test("the page carries its pictures and sounds, an SVG as an SVG", () => {
  const dir = publicFolder();
  const page = onePageHtml(BUILT, "", "", dir);
  rmSync(dir, { recursive: true });
  assert.ok(page.includes("window.clarityResources = {"), page);
  assert.ok(page.includes('"logo.svg":"data:image/svg+xml;base64,'), page);
  assert.ok(page.includes('"roll.wav":"data:audio/wav;base64,'), page);
  assert.ok(!page.includes("notes.md"), "Markdown travels in the model, not as a data URL");
});
