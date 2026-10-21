import { test } from "node:test";
import assert from "node:assert/strict";
import { keepsPageMenu } from "./pageMenu.ts";

test("the page's own right-click menu stays only in text fields", () => {
  const at = (inField: boolean) => ({ closest: () => (inField ? {} : null) }) as unknown as EventTarget;
  assert.equal(keepsPageMenu(at(true)), true);
  assert.equal(keepsPageMenu(at(false)), false);
  assert.equal(keepsPageMenu(null), false);
});
