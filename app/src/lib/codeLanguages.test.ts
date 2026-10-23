import { test } from "node:test";
import assert from "node:assert/strict";
import { languageOfFile } from "./codeLanguages.ts";

test("a library file's language comes from its name", () => {
  assert.equal(languageOfFile("Arena.cpp"), "cpp");
  assert.equal(languageOfFile("clarity/Arena.h"), "cpp");
  assert.equal(languageOfFile("glue.c"), "cpp");
  assert.equal(languageOfFile("src/lib.rs"), "rust");
  assert.equal(languageOfFile("arena.go"), "go");
  assert.equal(languageOfFile("go.mod"), "go");
  assert.equal(languageOfFile("Notes.cs"), "csharp");
  assert.equal(languageOfFile("CMakeLists.txt"), "cmake");
  assert.equal(languageOfFile("parts/x.cmake"), "cmake");
  assert.equal(languageOfFile("Cargo.toml"), "toml");
  assert.equal(languageOfFile("README.md"), "plain");
});
