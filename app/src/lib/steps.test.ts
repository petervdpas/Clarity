// A wizard walked through as every renderer does (conformance/steps.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { goBack, goNext, shownStep, stepButtons } from "./steps.ts";

const steps = JSON.parse(readFileSync(new URL("../../../conformance/steps.json", import.meta.url), "utf8"));

test("every walk through a wizard is as on every renderer", async () => {
  assert.ok(steps.walk.length > 0);
  for (const c of steps.walk) {
    let step: number | undefined = c.start ?? 1;
    const place = { get: () => step, set: (s: number) => (step = s) };
    const ran: string[] = [];
    const handler = c.handler as { refuses?: number[]; jumps?: { from: number; to: number } } | null;
    // The handler of `on next`, as the backend runs it: a refusal changes nothing.
    const run = handler
      ? async () => {
          const on = step!;
          ran.push(`next ${on}`);
          if (handler.refuses?.includes(on)) return false;
          if (handler.jumps?.from === on) step = handler.jumps.to;
          return true;
        }
      : undefined;
    for (const press of c.do) {
      const shown = shownStep(step, c.steps);
      assert.ok(stepButtons(shown, c.steps).includes(press), `${c.name}: ${press} is not shown`);
      if (press === "next") await goNext(place, c.steps, run);
      else if (press === "back") goBack(place, c.steps);
      else ran.push(`finish ${shown}`);
    }
    const shown = shownStep(step, c.steps);
    assert.equal(shown, c.expect.shown, c.name);
    assert.deepEqual(stepButtons(shown, c.steps), c.expect.buttons, c.name);
    assert.deepEqual(ran, c.expect.ran, c.name);
  }
});
