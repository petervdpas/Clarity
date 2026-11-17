import { test } from "node:test";
import assert from "node:assert/strict";
import { startProject } from "./startProject.ts";

/** What a start did: the project it opened, whether it forgot the remembered one, what it said. */
function starting(given: string | null, remembered: string | null, fails: boolean) {
  const did = { opened: [] as string[], forgot: false, said: null as string | null, warned: null as string | null, chose: false };
  const run = startProject(given, remembered, {
    open: async (path) => {
      did.opened.push(path);
      if (fails) throw `cannot open ${path}: The system cannot find the path specified. (os error 3)`;
    },
    forget: () => {
      did.forgot = true;
    },
    say: (text) => (did.said = text),
    warn: (text) => (did.warned = text),
    choose: () => {
      did.chose = true;
    },
  });
  return run.then(() => did);
}

test("the project given at start opens before the one remembered", async () => {
  const did = await starting("C:/given.clarity", "C:/last.clarity", false);
  assert.deepEqual(did.opened, ["C:/given.clarity"]);
  assert.equal(did.forgot, false);
});

test("the project remembered from last time opens when none is given", async () => {
  const did = await starting(null, "C:/last.clarity", false);
  assert.deepEqual(did.opened, ["C:/last.clarity"]);
  assert.equal(did.said, null);
});

test("a missing remembered project opens nothing, is forgotten, and gets a warning, not an error", async () => {
  const did = await starting(null, "C:/gone/gone.clarity", true);
  assert.equal(did.forgot, true, "the next start does not try it again");
  assert.equal(did.warned, "Project missing: C:/gone/gone.clarity");
  assert.equal(did.said, null);
  assert.equal(did.chose, true, "the project browser opens, to pick another");
});

test("a project given at start that cannot be opened is said, and the remembered one kept", async () => {
  const did = await starting("C:/typo.clarity", "C:/last.clarity", true);
  assert.equal(did.forgot, false);
  assert.match(did.said ?? "", /os error 3/);
  assert.equal(did.chose, false);
});

test("nothing given and nothing remembered opens nothing", async () => {
  const did = await starting(null, null, false);
  assert.deepEqual(did.opened, []);
});
