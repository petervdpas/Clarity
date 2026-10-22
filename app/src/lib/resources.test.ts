// The resources of a project as the IDE offers them: the files a part or a
// key can point at, by the kind it shows, and the rows of the project tree.
// A key points each language at a file (`gameRules resource "rules-nl.md"`).

import { test } from "node:test";
import assert from "node:assert/strict";
import { keyResource, resourceChoices, resourceKind, resourceUses } from "./resources.ts";
import type { ResourceKindInfo } from "./generated/ResourceKindInfo";

test("a key gives the resource of the language shown, else the default's", () => {
  const keys = { en: { gameRules: "rules-en.md" }, nl: { gameRules: "rules-nl.md" }, de: {} };
  assert.equal(keyResource(keys, "@gameRules", "nl", "en"), "rules-nl.md");
  assert.equal(keyResource(keys, "@gameRules", "de", "en"), "rules-en.md");
  assert.equal(keyResource(keys, "@saveCaption", "nl", "en"), undefined);
  assert.equal(keyResource(keys, "not a key", "nl", "en"), undefined);
  assert.equal(keyResource(undefined, "@gameRules", "nl", "en"), undefined);
});

/** The types as the backend gives them (clarity_spec::resources::kinds). */
const KINDS: ResourceKindInfo[] = [
  { kind: "markdown", name: "Markdown", extensions: ["md"] },
  { kind: "picture", name: "Picture", extensions: ["png", "jpg", "jpeg", "gif", "bmp", "webp"] },
  { kind: "pdf", name: "PDF", extensions: ["pdf"] },
  { kind: "drawing", name: "Drawing", extensions: ["svg", "dxf"] },
  { kind: "file", name: "Other file", extensions: [] },
];

test("a resource's type is by its extension, as the backend lists the types", () => {
  assert.equal(resourceKind("rules-nl.md", KINDS), "markdown");
  assert.equal(resourceKind("Logo.PNG", KINDS), "picture");
  assert.equal(resourceKind("plan.svg", KINDS), "drawing");
  assert.equal(resourceKind("manual.pdf", KINDS), "pdf");
  assert.equal(resourceKind("data.csv", KINDS), "file");
  assert.equal(resourceKind("rules.md", []), "file");
});

test("a part or a key is offered the files of the type it shows", () => {
  const files = [{ name: "banner.png" }, { name: "rules-en.md" }, { name: "data.csv" }, { name: "logo.jpg" }];
  assert.deepEqual(resourceChoices(files, "picture", KINDS), ["banner.png", "logo.jpg"]);
  assert.deepEqual(resourceChoices(files, "markdown", KINDS), ["rules-en.md"]);
  assert.deepEqual(resourceChoices(undefined, "markdown", KINDS), []);
});

test("a resource says which keys point at it, in which languages", () => {
  const keys = { en: { gameRules: "rules-en.md" }, nl: { gameRules: "rules-nl.md", bannerPicture: "rules-en.md" } };
  assert.deepEqual(resourceUses(keys, "rules-en.md"), ["@gameRules in en", "@bannerPicture in nl"]);
  assert.deepEqual(resourceUses(keys, "other.md"), []);
});
