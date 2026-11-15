// What the design canvas and preview say in their place when Clarity.UI's
// browser files are missing, instead of a raw error. Clarity never builds
// them: a cDdev's own build is swapped in, and signals there when behind.
import { test } from "node:test";
import assert from "node:assert/strict";
import { browserFilesNotice } from "./browserFiles.ts";

test("fine files say nothing", () => {
  assert.equal(browserFilesNotice({ state: "fine", repository: true }), null);
  assert.equal(browserFilesNotice({ state: "fine", repository: false }), null);
});

test("missing files in a Clarity run from its repository point at the release and DevTools", () => {
  const notice = browserFilesNotice({ state: "missing", repository: true });
  assert.match(notice?.text ?? "", /release/);
  assert.match(notice?.text ?? "", /Clarity settings > DevTools/);
  assert.doesNotMatch(notice?.text ?? "", /[Rr]ebuild/);
});

test("missing files in an installed Clarity say to install it again", () => {
  const notice = browserFilesNotice({ state: "missing", repository: false });
  assert.match(notice?.text ?? "", /[Ii]nstall Clarity again/);
});

test("behind its source is never said here: DevTools signals it", () => {
  assert.equal(browserFilesNotice({ state: "stale", repository: true }), null);
  assert.equal(browserFilesNotice({ state: "stale", repository: false }), null);
});

test("no text has an em-dash", () => {
  for (const repository of [true, false]) {
    assert.doesNotMatch(browserFilesNotice({ state: "missing", repository })?.text ?? "", /—|–/);
  }
});
