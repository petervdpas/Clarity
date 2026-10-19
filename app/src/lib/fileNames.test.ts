// Windows and macOS do not tell file names apart by case, so an import that
// names one file can find another there: `./pageDesigner.svelte` found
// PageDesigner.svelte instead of pageDesigner.svelte.ts. No two files of
// the app may differ only in case, counting `.svelte.ts` the way imports
// name it (without `.ts`).

import { readdirSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

function files(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? files(join(dir, e.name)) : [join(dir, e.name)]));
}

test("no two files differ only in case, as imports name them", () => {
  const src = new URL("..", import.meta.url).pathname;
  const seen = new Map<string, string>();
  const clashes: string[] = [];
  for (const file of files(src)) {
    const imported = (file.endsWith(".svelte.ts") ? file.slice(0, -3) : file).toLowerCase();
    const other = seen.get(imported);
    if (other) clashes.push(`${other} and ${file}`);
    seen.set(imported, file);
  }
  assert.deepEqual(clashes, []);
});
