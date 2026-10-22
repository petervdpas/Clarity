// The resources of a project as the IDE offers them: the files a part or a
// key can point at, by the kind it shows, and the rows of the project tree.
// A key points each language at a file (`gameRules resource "rules-nl.md"`).

import { test } from "node:test";
import assert from "node:assert/strict";
import { keyResource, resourceChoices, resourceKind, resourceUses } from "./resources.ts";

test("a key gives the resource of the language shown, else the default's", () => {
  const keys = { en: { gameRules: "rules-en.md" }, nl: { gameRules: "rules-nl.md" }, de: {} };
  assert.equal(keyResource(keys, "@gameRules", "nl", "en"), "rules-nl.md");
  assert.equal(keyResource(keys, "@gameRules", "de", "en"), "rules-en.md");
  assert.equal(keyResource(keys, "@saveCaption", "nl", "en"), undefined);
  assert.equal(keyResource(keys, "not a key", "nl", "en"), undefined);
  assert.equal(keyResource(undefined, "@gameRules", "nl", "en"), undefined);
});

test("a resource is Markdown, a picture or another file, by its name", () => {
  assert.equal(resourceKind("rules-nl.md"), "markdown");
  assert.equal(resourceKind("Logo.PNG"), "picture");
  assert.equal(resourceKind("data.csv"), "file");
});

test("a part or a key is offered the files of the kind it shows", () => {
  const files = [{ name: "banner.png" }, { name: "rules-en.md" }, { name: "data.csv" }, { name: "logo.jpg" }];
  assert.deepEqual(resourceChoices(files, "picture"), ["banner.png", "logo.jpg"]);
  assert.deepEqual(resourceChoices(files, "markdown"), ["rules-en.md"]);
  assert.deepEqual(resourceChoices(undefined, "markdown"), []);
});

test("a resource says which keys point at it, in which languages", () => {
  const keys = { en: { gameRules: "rules-en.md" }, nl: { gameRules: "rules-nl.md", bannerPicture: "rules-en.md" } };
  assert.deepEqual(resourceUses(keys, "rules-en.md"), ["@gameRules in en", "@bannerPicture in nl"]);
  assert.deepEqual(resourceUses(keys, "other.md"), []);
});
