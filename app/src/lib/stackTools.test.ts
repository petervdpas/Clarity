// What the stack fields say about the build tools of the chosen stack (P3).

import { test } from "node:test";
import assert from "node:assert/strict";
import { toolsNote } from "./stackTools.ts";
import type { StackTools } from "@clarity/ui/generated/StackTools.ts";

const all: StackTools[] = [
  { backend: "rust", ui: "clarity", generated: true, ready: true, found: ["cargo 1.96.0"], missing: [], guide: null },
  { backend: "csharp", ui: "clarity", generated: true, ready: false, found: [], missing: ["the .NET 10 SDK, from https://dotnet.microsoft.com/download"], guide: "net-on-linux" },
  { backend: "go", ui: "webview", generated: false, ready: false, found: ["go version go1.25.7"], missing: [], guide: null },
];

test("a ready stack names its tools", () => {
  assert.deepEqual(toolsNote(all, "rust", "clarity"), { level: "ok", text: "Ready to build: cargo 1.96.0", guide: null });
});

test("a stack without its tools says what to install, and that the rest works", () => {
  const note = toolsNote(all, "csharp", "clarity");
  assert.equal(note?.level, "warning");
  assert.match(note!.text, /^Install the \.NET 10 SDK, from https:\/\/dotnet\.microsoft\.com\/download to build and run here\./);
  assert.match(note!.text, /Designing, previewing and generating work without it; the generated project's README\.md says how to build it\./);
  // Where the manual's setup page tells how to install it.
  assert.equal(note?.guide, "net-on-linux");
});

test("a stack Clarity does not generate yet says so", () => {
  assert.deepEqual(toolsNote(all, "go", "webview"), { level: "warning", text: "Clarity does not generate this stack yet.", guide: null });
});

test("nothing is said before the tools are known", () => {
  assert.equal(toolsNote(null, "rust", "clarity"), null);
});
