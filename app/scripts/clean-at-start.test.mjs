import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { cleanAtStart } from "./clean-at-start.mjs";

/** A repository with these files, each holding one byte. */
function repo(files) {
  const root = mkdtempSync(join(tmpdir(), "clean-at-start-"));
  for (const file of files) {
    mkdirSync(join(root, file, ".."), { recursive: true });
    writeFileSync(join(root, file), "x");
  }
  return root;
}

test("the folders marked for the next start go, and the marks with them", () => {
  const root = repo(["target/debug/clarity.exe", "target/generated/app.bin", "target/release/kept.bin", "src/main.rs"]);
  writeFileSync(join(root, "target/remove-at-start"), "target/debug\ntarget/generated\n");
  const said = [];
  assert.deepEqual(cleanAtStart(root, (l) => said.push(l)), ["target/debug", "target/generated"]);
  assert.ok(!existsSync(join(root, "target/debug")) && !existsSync(join(root, "target/generated")));
  assert.ok(existsSync(join(root, "target/release/kept.bin")) && existsSync(join(root, "src/main.rs")), "nothing else goes");
  assert.ok(!existsSync(join(root, "target/remove-at-start")), "the next start removes nothing");
  assert.equal(said.length, 2);
});

test("nothing outside the repository goes, nor the repository itself", () => {
  const outside = repo(["precious.txt"]);
  const root = repo(["target/debug/a.bin"]);
  writeFileSync(join(root, "target/remove-at-start"), `../${outside.split(/[\\/]/).pop()}\n.\n${outside}\n`);
  const said = [];
  assert.deepEqual(cleanAtStart(root, (l) => said.push(l)), []);
  assert.ok(existsSync(join(outside, "precious.txt")) && existsSync(join(root, "target/debug/a.bin")));
  assert.equal(said.length, 3, said.join("\n"));
});

test("no marks, nothing to do", () => {
  const root = repo(["target/debug/a.bin"]);
  assert.deepEqual(cleanAtStart(root, () => assert.fail("says nothing")), []);
  assert.equal(readFileSync(join(root, "target/debug/a.bin"), "utf8"), "x");
});
