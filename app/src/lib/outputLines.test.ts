import { test } from "node:test";
import assert from "node:assert/strict";
import { shownLine } from "./outputLines.ts";

test("a carriage return writes over the line, as in a terminal", () => {
  assert.equal(shownLine("    Building [====> ] 176/179: clarity-runtime\r   Compiling clarity-ui v0.19.0 (/x)"), "   Compiling clarity-ui v0.19.0 (/x)");
  assert.equal(shownLine("   Compiling clarity-runtime v0.54.0"), "   Compiling clarity-runtime v0.54.0");
  assert.equal(shownLine("text\r\r   "), "text");
});
