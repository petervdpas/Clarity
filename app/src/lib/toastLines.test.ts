import { test } from "node:test";
import assert from "node:assert/strict";
import { cornerLine, iconLine, isToastName, levelLine, lifespanLine, newToastText, textLine } from "./toastLines.ts";

test("each change of a toast is one line, and Clarity's default takes its line away", () => {
  assert.deepEqual(textLine("@savedToast"), { key: "text", line: "text @savedToast" });
  assert.deepEqual(lifespanLine(3000), { key: "lifespan", line: "lifespan 3000" });
  assert.deepEqual(levelLine("success"), { key: "level", line: "level success" });
  assert.deepEqual(levelLine("info"), { key: "level", line: null });
  assert.deepEqual(iconLine("check"), { key: "icon", line: 'icon "check"' });
  assert.deepEqual(iconLine(null), { key: "icon", line: null });
  assert.deepEqual(cornerLine("bottomLeft"), { key: "corner", line: "corner bottomLeft" });
  assert.deepEqual(cornerLine("topRight"), { key: "corner", line: null });
});

test("a lifespan Clarity cannot show is refused with why", () => {
  assert.match(lifespanLine(100) as string, /500 to 120000/);
  assert.match(lifespanLine(2.5) as string, /milliseconds/);
});

test("a new toast has a camelCase name, a text key and Clarity's default lifespan", () => {
  assert.ok(isToastName("saved") && !isToastName("Saved") && !isToastName("not saved"));
  assert.equal(newToastText("saved"), "toast saved\n    text @savedToast\n    lifespan 5000\n");
});
