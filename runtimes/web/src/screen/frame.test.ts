// The frame a page shows in the designer, against the cases the spec runs (conformance/frame-shown.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { App } from "../generated/App.ts";
import type { Frame } from "../generated/Frame.ts";
import { frameForPage } from "./frame.ts";

const cases = JSON.parse(readFileSync(new URL("../../../../conformance/frame-shown.json", import.meta.url), "utf8"));

/** A frame with the regions named, each a marker of whose it is. */
function frameOf(regions: string[], who: string): Frame {
  const has = (r: string) => (regions.includes(r) ? { who } : null);
  const sidebars = (["left", "right"] as const).filter((s) => regions.includes(s)).map((side) => ({ who, props: { side } }));
  return { menu: has("menu"), toolbar: has("toolbar"), ribbon: null, sidebars, footer: has("footer") } as unknown as Frame;
}

test("a page shows the frame the cases say", () => {
  for (const c of cases.cases) {
    const app = { frame: frameOf(c.app, "app") } as unknown as App;
    const shown = frameForPage(app, { kind: c.dialog ? "dialog" : "window", frame: frameOf(c.own, "own") });
    const whose = (s: { value: { who: string } } | null) => (s ? s.value.who : null);
    const got = { menu: whose(shown.menu as never), toolbar: whose(shown.toolbar as never), left: whose(shown.left as never), right: whose(shown.right as never), footer: whose(shown.footer as never) };
    assert.deepEqual(got, c.shows, c.name);
  }
});
