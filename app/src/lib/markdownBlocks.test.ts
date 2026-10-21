// The markdown part's Markdown, read into blocks and spans, against the
// cases every renderer shares (conformance/markdown.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { markdownBlocks, markdownIn, type Block } from "./markdownBlocks.ts";

type Cases = { cases: { name: string; markdown: string; blocks: Block[] }[] };
const file: Cases = JSON.parse(readFileSync(new URL("../../../conformance/markdown.json", import.meta.url), "utf8"));

for (const c of file.cases) {
  test(`markdown: ${c.name}`, () => {
    assert.deepEqual(markdownBlocks(c.markdown), c.blocks);
  });
}

test("a Markdown file shows in the language picked, else the default one's", () => {
  const markdown = { "rules.md": { en: "# Rules", nl: "# Regels" } };
  assert.equal(markdownIn(markdown, "rules.md", "nl", "en"), "# Regels");
  assert.equal(markdownIn(markdown, "rules.md", "de", "en"), "# Rules");
  assert.equal(markdownIn(markdown, "missing.md", "nl", "en"), "");
  assert.equal(markdownIn(undefined, "rules.md", "nl", "en"), "");
});
