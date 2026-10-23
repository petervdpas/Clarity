// A sound's melodies read alike on every stack (conformance/melody.json),
// and the renderer keeps what each sound part was told apart from custom parts.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { melody } from "./sound.ts";

type Case = { notes: string; tempo: number; plays: { hertz: number | null; seconds: number }[]; skips: string[] };
const cases: Case[] = JSON.parse(readFileSync(new URL("../../../conformance/melody.json", import.meta.url), "utf8")).cases;

test("melodies read as conformance/melody.json says", () => {
  for (const c of cases) {
    const { plays, skips } = melody(c.notes, c.tempo);
    const rounded = plays.map((n) => ({ hertz: n.hertz === null ? null : Math.round(n.hertz * 100) / 100, seconds: Math.round(n.seconds * 10000) / 10000 }));
    assert.deepEqual(rounded, c.plays, c.notes);
    assert.deepEqual(skips, c.skips, c.notes);
  }
});
