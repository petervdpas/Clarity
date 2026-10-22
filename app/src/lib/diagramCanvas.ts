// What the diagram canvas needs besides Mermaid itself: the IDE theme's
// colors as Mermaid's theme variables, which box a click hit, and how the
// proposals of a written diagram mark its boxes.

/** Mermaid's theme variables from the IDE theme's colors, read by name. */
export function themeVariables(read: (name: string) => string): Record<string, string> {
  const color = (name: string) => read(name).trim();
  return {
    background: color("--bg"),
    mainBkg: color("--panel"),
    primaryColor: color("--panel"),
    primaryTextColor: color("--text"),
    primaryBorderColor: color("--border-strong"),
    secondaryColor: color("--panel-alt"),
    tertiaryColor: color("--panel-alt"),
    lineColor: color("--text-muted"),
    textColor: color("--text"),
    nodeBorder: color("--border-strong"),
    attributeBackgroundColorOdd: color("--panel"),
    attributeBackgroundColorEven: color("--panel-alt"),
    edgeLabelBackground: color("--panel"),
  };
}

/** What a drawn box shows: its element id, its data-id, and its text. */
export type Box = { id: string; dataId: string | null; text: string };

/** The name of the entity, enum or class a box draws; null when it draws none of `names`. */
export function boxName(box: Box, names: string[]): string | null {
  if (box.dataId && names.includes(box.dataId)) return box.dataId;
  const byId = /(?:^|-)(?:entity|classId)-(.+)-\d+$/.exec(box.id)?.[1];
  if (byId && names.includes(byId)) return byId;
  const first = box.text.trim().split(/\s+/)[0] ?? "";
  return names.includes(first) ? first : null;
}

/** How a proposal marks a box. */
export type Mark = "add" | "change" | "remove";

/**
 * The marks of the boxes the proposals are about: a box added whole is
 * "add"; a box with any change inside it is "change"; a box with only
 * removals is "remove".
 */
export function marksOf(proposals: { change: Mark; subject: string }[]): Record<string, Mark> {
  const marks: Record<string, Mark> = {};
  for (const p of proposals) {
    const [box, member] = p.subject.split(".");
    if (!box) continue;
    const mark: Mark = member === undefined ? p.change : p.change === "remove" ? "remove" : "change";
    const had = marks[box];
    marks[box] = had === undefined || had === mark ? mark : had === "add" ? "add" : "change";
  }
  return marks;
}
