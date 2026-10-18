// A call that finds the vault locked asks for the password, then goes on.

import { test } from "node:test";
import assert from "node:assert/strict";

import { LOCKED, Unlocker } from "./unlock.ts";

test("a locked vault asks for the password and the call goes on", async () => {
  let locked = true;
  let asked = 0;
  const unlocker = new Unlocker(async () => {
    asked++;
    locked = false;
    return true;
  });
  const value = await unlocker.run(async () => {
    if (locked) throw LOCKED;
    return 42;
  });
  assert.equal(value, 42);
  assert.equal(asked, 1);
});

test("giving up on the password gives the locked error back", async () => {
  const unlocker = new Unlocker(async () => false);
  await assert.rejects(
    unlocker.run(async () => {
      throw LOCKED;
    }),
    (err) => err === LOCKED,
  );
});

test("other errors pass through without asking", async () => {
  let asked = 0;
  const unlocker = new Unlocker(async () => {
    asked++;
    return true;
  });
  await assert.rejects(unlocker.run(async () => { throw "no connection anthropic"; }));
  assert.equal(asked, 0);
});

test("calls that find it locked at once share one question", async () => {
  let locked = true;
  let asked = 0;
  const unlocker = new Unlocker(async () => {
    asked++;
    await new Promise((r) => setTimeout(r, 5));
    locked = false;
    return true;
  });
  const call = async () => {
    if (locked) throw LOCKED;
    return "ok";
  };
  assert.deepEqual(await Promise.all([unlocker.run(call), unlocker.run(call)]), ["ok", "ok"]);
  assert.equal(asked, 1);
});
