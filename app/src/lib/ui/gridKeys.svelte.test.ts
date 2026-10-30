// The designer keeps the parts of a grid by a key of their own: a form's
// label column (`labels 2`) makes a label for each input, at the input's
// place in the file, so the key tells the made label from its input.

import { expect, test } from "vitest";
import type { Widget } from "../generated/Widget";
import { partKey } from "../pages";

const part = (start: number, made: boolean): Widget =>
  ({ props: { made }, span: { start, end: start + 10, line: 1, col: 1 }, children: [], pages: [], menu: null }) as unknown as Widget;

test("a made label and its input have keys of their own", () => {
  const keys = [part(3269, true), part(3300, true), part(3269, false), part(3300, false)].map(partKey);
  expect(new Set(keys).size).toBe(4);
});
