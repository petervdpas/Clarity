// A ribbon's open tab (conformance/ribbons.json): the one asked for when it
// is there and visible, else the first visible one.

/**
 * The tab a ribbon shows (from 1) of tabs that are visible or not, when
 * `asked` is wanted: it, when it is there and visible, else the first
 * visible one; null when none is (conformance/ribbons.json).
 */
export function shownTab(visible: boolean[], asked: number): number | null {
  if (asked >= 1 && asked <= visible.length && visible[asked - 1]) return asked;
  const first = visible.indexOf(true);
  return first < 0 ? null : first + 1;
}
