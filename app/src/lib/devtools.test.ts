// DevTools' page logic: how the release checks stand as one outcome, and
// what a release's changes come to.
import { test } from "node:test";
import assert from "node:assert/strict";
import { DISK_TAB, RELEASE_TAB, bytesText, changeText, checkState, cycle, stateText, tabOrder, verdict } from "./devtools.ts";

const check = (title: string, last: boolean | null = null) => ({ job: `check:${title}`, title, last: last === null ? null : { passed: last } });

test("a check's build in this run wins over how it last ended", () => {
  assert.equal(checkState(check("a", false), { state: "passed" }), "passed");
  assert.equal(checkState(check("a", true), undefined), "passed", "kept from an earlier run of Clarity");
  assert.equal(checkState(check("a"), undefined), "notRun");
  assert.equal(checkState(check("a", true), { state: "failed" }), "failed");
});

test("the checks read as one outcome", () => {
  const checks = [check("A"), check("B"), check("C")];
  assert.equal(verdict(checks, {}).text, "Not run yet: 3 checks");
  assert.equal(verdict(checks, { "check:A": { state: "passed" }, "check:B": { state: "running" } }).text, "Running 2 of 3: B");
  const some = verdict(checks, { "check:A": { state: "passed" }, "check:B": { state: "failed" } });
  assert.equal(some.text, "1 passed, 1 failed, 1 not run");
  assert.deepEqual(some.failed.map((f) => f.title), ["B"]);
  assert.equal(verdict(checks, { "check:A": { state: "passed" }, "check:B": { state: "failed" }, "check:C": { state: "passed" } }).text, "1 of 3 failed");
  const all = verdict([check("A", true), check("B", true)], {});
  assert.equal(all.text, "All 2 checks passed");
  assert.equal(all.allPassed, true);
});

test("a release's changes say code first and name the rest", () => {
  const none = { code: [], manifest: [], tests: [], docs: [] };
  assert.equal(changeText({ ...none, code: ["a.rs", "b.rs"], manifest: ["Cargo.toml"] }), "2 code files");
  assert.equal(changeText({ ...none, manifest: ["Cargo.toml"] }), "manifest only");
  assert.equal(changeText({ ...none, manifest: ["Cargo.toml"], tests: ["t.rs"] }), "manifest and tests, no code");
  assert.equal(changeText(none), "no changes");
});

test("a build's state reads as words", () => {
  assert.equal(stateText(undefined), "not run");
  assert.equal(stateText({ state: "running" }), "building");
  assert.equal(stateText({ state: "failed" }), "failed");
});

test("a size reads as people read it", () => {
  assert.equal(bytesText(512), "512 bytes");
  assert.equal(bytesText(1536), "1.5 KB");
  assert.equal(bytesText(448 * 1024 * 1024), "448 MB");
  assert.equal(bytesText(39.2 * 1024 ** 3), "39 GB");
  assert.equal(bytesText(9.4 * 1024 ** 3), "9.4 GB");
});

test("Disk and Release planning come first, then the pieces with a signal", () => {
  const pieces = [{ key: "clarityUi", signal: null }, { key: "clarityUiBrowser", signal: "behind" }, { key: "rustCrates", signal: null }, { key: "clarityAudio", signal: "behind" }];
  assert.deepEqual(tabOrder(pieces), [DISK_TAB, RELEASE_TAB, "clarityUiBrowser", "clarityAudio", "clarityUi", "rustCrates"]);
});

test("the arrows cycle through the tabs, round at both ends", () => {
  const order = ["a", "b", "c"];
  assert.equal(cycle(order, "b", 1), "c");
  assert.equal(cycle(order, "c", 1), "a");
  assert.equal(cycle(order, "a", -1), "c");
  assert.equal(cycle(order, "gone", 1), "b", "a tab that went: from the first");
});
