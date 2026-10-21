import { test } from "node:test";
import assert from "node:assert/strict";
import { Pictures, bundled } from "./pictures.ts";

test("a generated application finds a picture next to its renderer", () => {
  const pictures = new Pictures(null, () => {});
  assert.equal(pictures.url("logo.png"), "resources/logo.png");
  assert.equal(bundled("my logo.png"), "resources/my%20logo.png");
});

test("the designer loads a picture once, and shows it when it has come", async () => {
  const asked: string[] = [];
  let changes = 0;
  const pictures = new Pictures(async (name) => {
    asked.push(name);
    return `data:image/png;base64,${name}`;
  }, () => changes++);
  assert.equal(pictures.url("logo.png"), undefined);
  assert.equal(pictures.url("logo.png"), undefined);
  await Promise.resolve();
  await Promise.resolve();
  assert.equal(pictures.url("logo.png"), "data:image/png;base64,logo.png");
  assert.deepEqual(asked, ["logo.png"]);
  assert.equal(changes, 1);
});

test("a picture that cannot be loaded shows as missing", async () => {
  const pictures = new Pictures(async () => {
    throw new Error("gone");
  }, () => {});
  assert.equal(pictures.url("gone.png"), undefined);
  await Promise.resolve();
  await Promise.resolve();
  assert.equal(pictures.url("gone.png"), null);
});

test("a program that carries its pictures in the page gives them from there", () => {
  const page = globalThis as { clarityPictures?: Record<string, string> };
  page.clarityPictures = { "logo.png": "data:image/png;base64,iVBORw0KGgo=" };
  try {
    assert.equal(bundled("logo.png"), "data:image/png;base64,iVBORw0KGgo=");
    assert.equal(bundled("other.png"), "resources/other.png");
  } finally {
    delete page.clarityPictures;
  }
});
