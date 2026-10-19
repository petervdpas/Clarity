// Clarity's manual in its window: where a link on a page leads, and which
// page F1 opens on each tab. Headings' anchors come from markdown.ts; the
// pages are the Markdown files of docs/manual, which the backend embeds.

/** Where a link leads: a topic (or the language reference, "reference"),
 *  a heading on it, or null for anywhere the manual's window does not go. */
export function linkTarget(href: string): { topic: string | null; anchor: string | null } | null {
  const [path, anchor = null] = href.split("#");
  if (path === "") return { topic: null, anchor };
  if (path === "../LANGUAGE.md") return { topic: "reference", anchor };
  const topic = /^([a-z][a-z0-9.-]*)\.md$/.exec(path);
  return topic ? { topic: topic[1], anchor } : null;
}

/** The page F1 opens on a tab (or on the agent panel). */
export function topicForTab(tab: string): string {
  switch (tab) {
    case "project":
      return "project.file";
    case "design":
      return "design.windows";
    case "preview":
      return "project.run-build";
    case "vault":
      return "vault";
    case "agent":
      return "ai.agent";
    default:
      return "start.overview";
  }
}
