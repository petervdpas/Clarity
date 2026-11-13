// What the design canvas and preview say in their place when Clarity.UI's
// browser files are missing or behind, instead of a raw error.
import { test } from "node:test";
import assert from "node:assert/strict";
import { browserFilesNotice } from "./browserFiles.ts";

test("fine files say nothing", () => {
  assert.equal(browserFilesNotice({ state: "fine", repository: true }), null);
  assert.equal(browserFilesNotice({ state: "fine", repository: false }), null);
});

test("missing files offer Rebuild in a Clarity run from its repository", () => {
  const notice = browserFilesNotice({ state: "missing", repository: true });
  assert.equal(notice?.rebuild, true);
  assert.match(notice?.text ?? "", /missing/);
  assert.match(notice?.text ?? "", /build\.sh/);
});

test("missing files in an installed Clarity say to install it again, with no Rebuild", () => {
  const notice = browserFilesNotice({ state: "missing", repository: false });
  assert.equal(notice?.rebuild, false);
  assert.match(notice?.text ?? "", /[Ii]nstall Clarity again/);
});

test("stale files offer Rebuild; an installed Clarity is never stale", () => {
  const notice = browserFilesNotice({ state: "stale", repository: true });
  assert.equal(notice?.rebuild, true);
  assert.match(notice?.text ?? "", /older than its source/);
  assert.equal(browserFilesNotice({ state: "stale", repository: false }), null);
});

test("no text has an em-dash", () => {
  for (const state of ["missing", "stale"] as const) {
    for (const repository of [true, false]) {
      assert.doesNotMatch(browserFilesNotice({ state, repository })?.text ?? "", /\u2014|\u2013/);
    }
  }
});
