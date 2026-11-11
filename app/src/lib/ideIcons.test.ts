// Every icon the IDE names is in Clarity's icon set, so none draws empty.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { IDE_ICONS, idePaths } from "./ideIcons.ts";

const set = JSON.parse(readFileSync(new URL("../../../crates/clarity-spec/icons/lucide.json", import.meta.url), "utf8")) as Record<string, string>;

test("every IDE icon is a Lucide icon of Clarity's set", () => {
  for (const [word, lucide] of Object.entries(IDE_ICONS)) assert.ok(set[lucide], `${word}: no Lucide icon '${lucide}'`);
  for (const [word, d] of Object.entries(idePaths(set))) assert.ok(d.length > 0, `${word} draws nothing`);
});

test("every icon a ribbon button or the file browser names is an IDE icon", () => {
  const sources = ["../App.svelte", "./FileBrowser.svelte"].map((f) => readFileSync(new URL(f, import.meta.url), "utf8"));
  for (const source of sources) {
    for (const match of source.matchAll(/<(?:RibbonButton|IdeIcon)\b[^>]*\b(?:icon|name)="([a-z-]+)"/g)) {
      assert.ok(match[1] in IDE_ICONS, `'${match[1]}' is no IDE icon`);
    }
  }
});
