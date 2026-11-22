// The lines of custom parts and libraries the IDE writes, as the spec reads them.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { LIBRARY_KINDS, PART_KINDS, isDeclName, kindsFor, librarySourceLine, messageLine, missingOf, newLibraryText, newPartText, paramsOf, partLibraryLine, propertyLine, sliceOf, startLibrarySource, startPartLibrary } from "./customDecl.ts";

test("a new part starts in its own folder, in the stack's own kind", () => {
  assert.deepEqual(kindsFor(PART_KINDS, "rust").map((k) => k.kind), ["crate"]);
  assert.deepEqual(kindsFor(PART_KINDS, "csharp").map((k) => k.kind), ["nuget", "dll"]);
  assert.deepEqual(kindsFor(LIBRARY_KINDS, "cpp").map((k) => k.kind), ["cmake"]);
  assert.equal(kindsFor(PART_KINDS, null).length, 5);
  assert.equal(newPartText("Arena", startPartLibrary("crate", "Arena")), 'part Arena\n    crate "parts/Arena"\n    size 6 x 4\n');
  assert.equal(partLibraryLine(startPartLibrary("go", "Arena")), 'go "parts/arena"');
  assert.equal(partLibraryLine({ kind: "nuget", package: "Markdig", version: "0.40.0" }), 'nuget "Markdig" 0.40.0');
  assert.ok(isDeclName("Arena"));
  assert.ok(!isDeclName("arena"));
});

test("a library's package line per stack", () => {
  assert.equal(librarySourceLine({ kind: "nuget", package: "Humanizer.Core", version: "2.14.1" }), 'nuget "Humanizer.Core" 2.14.1');
  assert.equal(librarySourceLine({ kind: "crate", name: "humanize", version: "0.1" }), 'crate "humanize" "0.1"');
  assert.equal(librarySourceLine({ kind: "go", module: "github.com/dustin/go-humanize", version: "v1.0.1" }), 'go "github.com/dustin/go-humanize" v1.0.1');
  assert.equal(librarySourceLine(startLibrarySource("cmake", "Words")), 'cmake "Words"');
  assert.equal(newLibraryText("Words", { kind: "dll", file: "libs/Words.dll" }), 'library Words\n    dll "libs/Words.dll"\n');
  assert.equal(missingOf(startLibrarySource("nuget", "Words")), "the package");
  assert.equal(missingOf({ kind: "crate", name: "humanize", version: "" }), "the version");
  assert.equal(missingOf(startPartLibrary("crate", "Arena")), null);
});

test("each kind is written for the stacks the project's checks say (clarity_project PART_KIND_BACKENDS, LIBRARY_KIND_BACKENDS)", () => {
  const rust = readFileSync(new URL("../../../crates/clarity-project/src/lib.rs", import.meta.url), "utf8");
  const table = (name: string) => {
    const listed = rust.match(new RegExp(`pub const ${name}: [^=]*=\\s*\\[([\\s\\S]*?)\\];`))?.[1] ?? "";
    return [...listed.matchAll(/\("([a-z]+)", &\[([^\]]*)\]\)/g)].map((m) => ({ kind: m[1], backends: [...m[2].matchAll(/Backend::(\w+)/g)].map((b) => b[1].toLowerCase()) }));
  };
  assert.deepEqual(PART_KINDS.map((k) => ({ kind: k.kind, backends: k.backends })), table("PART_KIND_BACKENDS"));
  assert.deepEqual(LIBRARY_KINDS.map((k) => ({ kind: k.kind, backends: k.backends })), table("LIBRARY_KIND_BACKENDS"));
});
