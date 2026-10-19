// Clarity's manual in its window: where a link leads, the anchor of a
// heading, and which page F1 opens on each tab.

import { test } from "node:test";
import assert from "node:assert/strict";

import { anchor } from "./markdown.ts";
import { linkTarget, topicForTab } from "./manual.ts";

test("a link leads to a topic, the reference, or a heading on the page", () => {
  assert.deepEqual(linkTarget("code.queries.md"), { topic: "code.queries", anchor: null });
  assert.deepEqual(linkTarget("code.queries.md#files"), { topic: "code.queries", anchor: "files" });
  assert.deepEqual(linkTarget("../LANGUAGE.md#queries"), { topic: "reference", anchor: "queries" });
  assert.deepEqual(linkTarget("../LANGUAGE.md"), { topic: "reference", anchor: null });
  assert.deepEqual(linkTarget("#formats"), { topic: null, anchor: "formats" });
  // The reference links to manual pages from the folder above them.
  assert.deepEqual(linkTarget("manual/code.engines.md#writing-an-engine"), { topic: "code.engines", anchor: "writing-an-engine" });
  // Anywhere else is no place the manual's window goes.
  assert.equal(linkTarget("https://example.com"), null);
  assert.equal(linkTarget("../DESIGN.md"), null);
});

test("a heading's anchor is written as the reference's links write it", () => {
  assert.equal(anchor("9. Functions and actions"), "9-functions-and-actions");
  assert.equal(anchor("Text with slots"), "text-with-slots");
  assert.equal(anchor("Lines, names and values"), "lines-names-and-values");
  assert.equal(anchor("`find` and **queries**"), "find-and-queries");
});

test("F1 opens the page about the tab you are on", () => {
  assert.equal(topicForTab("project"), "project.file");
  assert.equal(topicForTab("design"), "design.windows");
  assert.equal(topicForTab("preview"), "project.run-build");
  assert.equal(topicForTab("vault"), "vault");
  assert.equal(topicForTab("agent"), "ai.agent");
  assert.equal(topicForTab("anything else"), "start.overview");
});
