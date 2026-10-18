// Pictures of records on the web renderer, against the cases every runtime
// shares (conformance/pictures.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { kindOf, pictureFromFile, pictureUrl } from "./recordPictures.ts";

type Cases = {
  kinds: { extension: string; bytes: string; type: string }[];
  notPictures: string[];
  wire: { value: string; bytes?: string; json: unknown }[];
};
const cases: Cases = JSON.parse(readFileSync(new URL("../../../conformance/pictures.json", import.meta.url), "utf8"));

const hex = (text: string) => Uint8Array.from(text.match(/../g) ?? [], (b) => parseInt(b, 16));

test("a picture is told by its first bytes", () => {
  for (const kind of cases.kinds) assert.equal(kindOf(hex(kind.bytes))?.type, kind.type, kind.extension);
  for (const other of cases.notPictures) assert.equal(kindOf(hex(other)), null, other);
});

test("a file chosen becomes a new picture as the runtimes read it", () => {
  const sent = cases.wire.find((w) => w.value === "new")!;
  assert.deepEqual(pictureFromFile(hex(sent.bytes!)), sent.json);
  assert.equal(pictureFromFile(hex("255044462D312E37")), null);
});

test("a field shows its new picture, or the stored one once fetched", () => {
  const sent = cases.wire.find((w) => w.value === "new")!.json as { data: string };
  assert.equal(pictureUrl(sent, undefined), `data:image/png;base64,${sent.data}`);
  assert.equal(pictureUrl({ stored: true }, undefined), null);
  assert.equal(pictureUrl({ stored: true }, "data:image/jpeg;base64,/9j/"), "data:image/jpeg;base64,/9j/");
  assert.equal(pictureUrl(null, "data:image/png;base64,x"), null);
});
