// DevTools' page logic: the plan's file name, which releases start in the
// plan, and which checks hold it.
import { test } from "node:test";
import assert from "node:assert/strict";
import { includedAtFirst, openChecks, planStamp, stateText } from "./devtools.ts";

test("the plan is named after the local time, YYYY-MM-DD-HHMM", () => {
  assert.equal(planStamp(new Date(2026, 10, 5, 9, 7)), "2026-11-05-0907");
});

test("a release with changes since its tag starts in the plan", () => {
  assert.equal(includedAtFirst({ changed: ["runtimes/rust/src/lib.rs"] }), true);
  assert.equal(includedAtFirst({ changed: [] }), false);
});

test("a check holds the plan until its build passed", () => {
  const checks = [{ job: "check:a", title: "A" }, { job: "check:b", title: "B" }, { job: "check:c", title: "C" }];
  const jobs = { "check:a": { state: "passed" }, "check:b": { state: "failed" } };
  assert.deepEqual(openChecks(checks, jobs), ["B", "C"]);
});

test("a build's state reads as words", () => {
  assert.equal(stateText(undefined), "not run");
  assert.equal(stateText({ state: "running" }), "building");
  assert.equal(stateText({ state: "failed" }), "failed");
});
