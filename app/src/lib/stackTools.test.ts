// What the stack fields say about the build tools of the chosen stack (P3).

import { test } from "node:test";
import assert from "node:assert/strict";
import { toolsNote } from "./stackTools.ts";
import type { StackTools } from "./generated/StackTools";

const all: StackTools[] = [
  { backend: "rust", ui: "web", generated: true, ready: true, found: ["cargo 1.96.0"], missing: [], guide: null, web_found: [], web_missing: [], web_installable: false, web_elevated: false },
  { backend: "csharp", ui: "avalonia", generated: true, ready: false, found: [], missing: ["the .NET 10 SDK, from https://dotnet.microsoft.com/download"], guide: "net-on-linux", web_found: [], web_missing: [], web_installable: false, web_elevated: false },
  { backend: "go", ui: "web", generated: false, ready: false, found: ["go version go1.25.7"], missing: [], guide: null, web_found: [], web_missing: [], web_installable: false, web_elevated: false },
];

const WASM = ".NET's WebAssembly tools, for the web target: dotnet workload install wasm-tools";
const sdk: StackTools = { backend: "csharp", ui: "avalonia", generated: true, ready: true, found: [".NET SDK 10.0.402"], missing: [], guide: null, web_found: [], web_missing: [WASM], web_installable: true, web_elevated: true };

test("a ready stack names its tools", () => {
  assert.deepEqual(toolsNote(all, "rust", "web", ["desktop"]), { level: "ok", text: "Ready to build: cargo 1.96.0", guide: null, install: null });
});

test("a stack without its tools says what to install, and that the rest works", () => {
  const note = toolsNote(all, "csharp", "avalonia", ["desktop"]);
  assert.equal(note?.level, "warning");
  assert.match(note!.text, /^Install the \.NET 10 SDK, from https:\/\/dotnet\.microsoft\.com\/download to build and run here\./);
  assert.match(note!.text, /Designing, previewing and generating work without it; the generated project's README\.md says how to build it\./);
  // Where the manual's setup page tells how to install it.
  assert.equal(note?.guide, "net-on-linux");
});

test("a stack Clarity does not generate yet says so", () => {
  assert.deepEqual(toolsNote(all, "go", "web", ["desktop"]), { level: "warning", text: "Clarity does not generate this stack yet.", guide: null, install: null });
});

test("nothing is said before the tools are known", () => {
  assert.equal(toolsNote(null, "rust", "web", ["desktop"]), null);
});

test("the web target of C# says what it needs more, and that Clarity installs it", () => {
  // A desktop project is ready without it.
  assert.equal(toolsNote([sdk], "csharp", "avalonia", ["desktop"])?.level, "ok");
  const note = toolsNote([sdk], "csharp", "avalonia", ["web"]);
  assert.equal(note?.level, "warning");
  assert.match(note!.text, /^Install \.NET's WebAssembly tools, for the web target: dotnet workload install wasm-tools to build and run here\./);
  assert.deepEqual(note?.install, { elevated: true });
  // Installed, the web target is ready too, and says so.
  const installed = { ...sdk, web_missing: [], web_found: ["wasm-tools 10.0.114/10.0.100"], web_installable: false };
  assert.deepEqual(toolsNote([installed], "csharp", "avalonia", ["web"]), { level: "ok", text: "Ready to build: .NET SDK 10.0.402, wasm-tools 10.0.114/10.0.100", guide: null, install: null });
});
