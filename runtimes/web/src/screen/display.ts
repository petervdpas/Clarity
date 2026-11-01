// The display part: a number in seven-segment digits, as on a calculator.
// The text a value becomes and the cells it fills follow the cases every
// renderer shares (conformance/display.json); ui/SegmentDisplay.svelte draws them.

/** The digits a display has without `digits n` (clarity_spec::widgets::DEFAULT_DIGITS). */
export const DEFAULT_DIGITS = 12;

/** The lit segments of each character: a top, b top right, c bottom right,
 *  d bottom, e bottom left, f top left, g middle. */
export const SEGMENTS: Record<string, string> = {
  "0": "abcdef",
  "1": "bc",
  "2": "abdeg",
  "3": "abcdg",
  "4": "bcfg",
  "5": "acdfg",
  "6": "acdefg",
  "7": "abc",
  "8": "abcdefg",
  "9": "abcdfg",
  "-": "g",
  E: "adefg",
  " ": "",
};

/** `digits` plus one, as a decimal string: "0999" gives "1000". */
function plusOne(digits: string): string {
  const out = digits.split("");
  for (let i = out.length - 1; i >= 0; i--) {
    if (out[i] !== "9") {
      out[i] = String(Number(out[i]) + 1);
      return out.join("");
    }
    out[i] = "0";
  }
  return "1" + out.join("");
}

/** The text `value` (a number as text) shows in `digits` digits: rounded
 *  half away from zero to fit, without trailing zeros; E when it cannot fit. */
export function displayText(value: string, digits: number): string {
  let text = value.trim();
  if (!/^-?\d+(\.\d+)?$/.test(text)) return "E";
  const negative = text.startsWith("-");
  if (negative) text = text.slice(1);
  const [whole = "0", fraction = ""] = text.split(".");
  const integer = whole.replace(/^0+/, "") || "0";
  if (integer.length > digits) return "E";
  const places = digits - integer.length;
  let kept = integer + fraction.slice(0, places).padEnd(places, "0");
  if (fraction.length > places && fraction[places] >= "5") kept = plusOne(kept);
  const newWhole = kept.slice(0, kept.length - places).replace(/^0+/, "") || "0";
  if (newWhole.length > digits) return "E";
  const newFraction = kept.slice(kept.length - places).replace(/0+$/, "");
  const shown = newFraction ? `${newWhole}.${newFraction}` : newWhole;
  return negative && /[1-9]/.test(shown) ? `-${shown}` : shown;
}

export type Cell = { char: string; point: boolean };

/** What each of the display's `digits` + 1 cells shows, from the left; the
 *  extra cell is for the minus sign. */
export function cells(text: string, digits: number): Cell[] {
  const lit: Cell[] = [];
  for (const char of text) {
    const last = lit.at(-1);
    if (char === "." && last) last.point = true;
    else lit.push({ char, point: false });
  }
  const dark = Array.from({ length: Math.max(digits + 1 - lit.length, 0) }, () => ({ char: " ", point: false }));
  return [...dark, ...lit];
}

/** A cell is 13 units wide and 20 high; its digit sits in 2..10 across and
 *  2..18 down, its point at the bottom right. Each segment is a bar with
 *  pointed ends, as the points of its outline. Avalonia draws the same. */
export const CELL_WIDTH = 13;
export const CELL_HEIGHT = 20;
const T = 1; // half the thickness of a bar
const G = 0.6; // the gap between bars

function across(x1: number, x2: number, y: number): [number, number][] {
  return [[x1, y], [x1 + T, y - T], [x2 - T, y - T], [x2, y], [x2 - T, y + T], [x1 + T, y + T]];
}

function down(x: number, y1: number, y2: number): [number, number][] {
  return [[x, y1], [x + T, y1 + T], [x + T, y2 - T], [x, y2], [x - T, y2 - T], [x - T, y1 + T]];
}

export const SHAPES: Record<string, [number, number][]> = {
  a: across(2 + G, 10 - G, 2),
  b: down(10, 2 + G, 10 - G),
  c: down(10, 10 + G, 18 - G),
  d: across(2 + G, 10 - G, 18),
  e: down(2, 10 + G, 18 - G),
  f: down(2, 2 + G, 10 - G),
  g: across(2 + G, 10 - G, 10),
};

/** The decimal point of a cell: a dot at its bottom right. */
export const POINT = { x: 11.5, y: 18, r: 1 };
