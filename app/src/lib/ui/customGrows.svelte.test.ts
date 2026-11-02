// A custom part grows with its window when its declaration says `grows`
// (DESIGN 5.19): the project marks each placed part, and the designer's
// preview lays it out by that, as every Clarity.UI does.

import { expect, test } from "vitest";
import type { Preview } from "@clarity/ui/generated/Preview.ts";
import type { Widget } from "@clarity/ui/generated/Widget.ts";
import { widgetGrows } from "@clarity/ui/screen/pages.ts";

const custom = (grows: boolean) => ({ props: { kind: "custom", part: "Board", grows }, children: [], pages: [], span: { start: 0, end: 0, line: 0, col: 0 }, menu: null }) as unknown as Widget;

test("the designer's preview lets a custom part grow as its declaration says", () => {
  const preview = {} as Preview;
  expect(widgetGrows(preview, custom(true))).toBe(true);
  expect(widgetGrows(preview, custom(false))).toBe(false);
});
