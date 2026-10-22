// The markdown part's Markdown, read into blocks and spans, against the
// cases every renderer shares (conformance/markdown.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { markdownBlocks, type Block } from "./markdownBlocks.ts";

type Cases = { cases: { name: string; markdown: string; blocks: Block[] }[] };
const file: Cases = JSON.parse(readFileSync(new URL("../../../conformance/markdown.json", import.meta.url), "utf8"));

for (const c of file.cases) {
  test(`markdown: ${c.name}`, () => {
    assert.deepEqual(markdownBlocks(c.markdown), c.blocks);
  });
}

